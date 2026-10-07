//! Polygon scan conversion: `FUN_00472B20` and its helpers.
//!
//! The converter walks the polygon's two vertex chains from the topmost
//! vertex, clips them against the Graph2D rectangle and hands each scanline
//! to the selected span filler with the two edge records ordered by x. Its
//! top/side clipping rebuilds the chains from vertices pushed onto the clip
//! boundary, so clipped spans keep interpolating along the true edges. A quad
//! whose chain is not monotonic in y is split into two triangles
//! (`FUN_004734E0`).

use super::attr::AttributeClass;
use super::span::{Span, SpanContext, SpanTarget};
use super::state::{
    vertex_ref, Edge, RasterState, ScanFrame, VertexRef, LIST_CHAIN_A, LIST_CHAIN_B, LIST_CLIP_A,
    LIST_CLIP_B, LIST_WORDS, VX, VY,
};

/// Fill-side flags `FUN_00472880` writes for each split chain segment.
const LEFT_OF_CLIP: u32 = 1;
const INSIDE_CLIP: u32 = 2;
const RIGHT_OF_CLIP: u32 = 4;

/// Scan-convert the first `count` pool vertices with the current row.
pub(crate) fn scan_polygon(
    state: &mut RasterState,
    count: usize,
    target: &mut SpanTarget<'_>,
    context: &SpanContext<'_>,
    frame: ScanFrame,
) {
    let clip = state.clip;
    let class = state.row.class();

    // Bounds and the topmost vertex (first strict minimum).
    let mut max_x = state.pool[0][VX];
    let mut min_x = max_x;
    let mut min_y = state.pool[0][VY];
    let mut max_y = min_y;
    let mut top = 0usize;
    for (index, vertex) in state.pool.iter().enumerate().take(count).skip(1) {
        let x = vertex[VX];
        if x < min_x {
            min_x = x;
        } else if x > max_x {
            max_x = x;
        }
        let y = vertex[VY];
        if y < min_y {
            min_y = y;
            top = index;
        } else if y > max_y {
            max_y = y;
        }
    }
    if !(min_y < max_y
        && i32::from(clip.y0) < max_y
        && min_y < i32::from(clip.y1)
        && min_x < max_x
        && i32::from(clip.x0) << 16 < max_x
        && min_x < i32::from(clip.x1) << 16)
    {
        return;
    }

    state.pool_count = count as i32;
    // Chain A walks backward from the top vertex to the first bottom vertex.
    let mut monotonic_break = 0;
    let mut count_a = 0usize;
    let mut index = top;
    let mut previous_y = state.pool[top][VY];
    loop {
        let slot = LIST_CHAIN_A + 3 * count_a;
        state.lists[slot] = vertex_ref(index);
        index = if index == 0 { count } else { index } - 1;
        state.lists[slot + 1] = vertex_ref(index);
        let y = state.pool[index][VY];
        if y < previous_y {
            monotonic_break = 1;
        }
        count_a += 1;
        previous_y = y;
        if y >= max_y {
            break;
        }
    }
    // Chain B walks forward.
    let mut count_b = 0usize;
    let mut index = top;
    let mut previous_y = state.pool[top][VY];
    loop {
        let slot = LIST_CHAIN_B + 3 * count_b;
        let next = if index + 1 >= count { 0 } else { index + 1 };
        state.lists[slot] = vertex_ref(index);
        state.lists[slot + 1] = vertex_ref(next);
        let y = state.pool[next][VY];
        if y < previous_y {
            monotonic_break = 2;
        }
        count_b += 1;
        index = next;
        previous_y = y;
        if y >= max_y {
            break;
        }
    }

    if monotonic_break != 0 {
        split_quad(state, monotonic_break, target, context, frame);
        return;
    }

    let mut edge_a: Edge = [0; 18];
    let mut edge_b: Edge = [0; 18];
    if !(i32::from(clip.x0) << 16 <= min_x
        && max_x < i32::from(clip.x1) << 16
        && i32::from(clip.y0) <= min_y)
    {
        match clip_chains(state, class, count_a, count_b, &mut edge_a, &mut edge_b) {
            Some((a, b)) => {
                count_a = a;
                count_b = b;
            }
            None => return,
        }
    }
    fill_chains(
        state,
        class,
        count_a,
        count_b,
        &mut edge_a,
        &mut edge_b,
        target,
        context,
        frame,
    );
}

/// The span loop at `00473347`: both chains are inside the clip rectangle.
#[allow(clippy::too_many_arguments)]
fn fill_chains(
    state: &mut RasterState,
    class: AttributeClass,
    count_a: usize,
    count_b: usize,
    edge_a: &mut Edge,
    edge_b: &mut Edge,
    target: &mut SpanTarget<'_>,
    context: &SpanContext<'_>,
    frame: ScanFrame,
) {
    let filler = state.row.filler();
    let mut a = 0usize;
    let mut b = 0usize;
    let from = *state.vertex(state.lists[LIST_CHAIN_A]);
    let to = *state.vertex(state.lists[LIST_CHAIN_A + 1]);
    class.init_edge(edge_a, &from, &to);
    let from = *state.vertex(state.lists[LIST_CHAIN_B]);
    let to = *state.vertex(state.lists[LIST_CHAIN_B + 1]);
    class.init_edge(edge_b, &from, &to);
    let mut y = state.vertex(state.lists[LIST_CHAIN_A])[VY];
    let bottom = i32::from(state.clip.y1);
    loop {
        let a_end = state.vertex(state.lists[LIST_CHAIN_A + 3 * a + 1])[VY];
        let b_end = state.vertex(state.lists[LIST_CHAIN_B + 3 * b + 1])[VY];
        let mut end = b_end;
        if a_end < end {
            end = a_end;
        }
        if end > bottom {
            end = bottom;
        }
        // FUN_00473620: one span per line, the smaller-x edge first.
        if y < end {
            for line in y..end {
                let (left, right, left_address) = if edge_a[0] < edge_b[0] {
                    (&mut *edge_a, &mut *edge_b, frame.chain_a_edge())
                } else {
                    (&mut *edge_b, &mut *edge_a, frame.chain_b_edge())
                };
                filler.fill(Span {
                    state,
                    target,
                    context,
                    line,
                    left,
                    right,
                    left_address,
                });
            }
            y = end;
        }
        if end >= bottom {
            return;
        }
        if a_end <= end {
            a += 1;
            if a >= count_a {
                return;
            }
            let from = *state.vertex(state.lists[LIST_CHAIN_A + 3 * a]);
            let to = *state.vertex(state.lists[LIST_CHAIN_A + 3 * a + 1]);
            class.init_edge(edge_a, &from, &to);
        }
        let b_end = state.vertex(state.lists[LIST_CHAIN_B + 3 * b + 1])[VY];
        if b_end <= end {
            b += 1;
            if b >= count_b {
                return;
            }
            let from = *state.vertex(state.lists[LIST_CHAIN_B + 3 * b]);
            let to = *state.vertex(state.lists[LIST_CHAIN_B + 3 * b + 1]);
            class.init_edge(edge_b, &from, &to);
        }
    }
}

/// Top/side clipping (`00472D1E..00473343`): trim both chains to the clip
/// top, split them at the clip sides, then rebuild chains A and B from
/// vertices clamped onto the clip rectangle. `None` when no visible span
/// remains.
fn clip_chains(
    state: &mut RasterState,
    class: AttributeClass,
    count_a: usize,
    count_b: usize,
    edge_a: &mut Edge,
    edge_b: &mut Edge,
) -> Option<(usize, usize)> {
    let trimmed_a = clip_chain_top(state, LIST_CHAIN_A, count_a);
    let trimmed_b = clip_chain_top(state, LIST_CHAIN_B, count_b);
    let split_a = split_chain_sides(state, class, LIST_CLIP_A, LIST_CHAIN_A, trimmed_a);
    let split_b = split_chain_sides(state, class, LIST_CLIP_B, LIST_CHAIN_B, trimmed_b);

    // Skip segment pairs that lie wholly beyond the same clip side.
    let mut a = 0usize;
    let mut b = 0usize;
    let mut y = state.vertex(state.lists[LIST_CLIP_A])[VY];
    loop {
        let flags = state.lists[LIST_CLIP_A + 3 * a + 2] | state.lists[LIST_CLIP_B + 3 * b + 2];
        if flags != LEFT_OF_CLIP && flags != RIGHT_OF_CLIP {
            break;
        }
        let a_end = state.vertex(state.lists[LIST_CLIP_A + 3 * a + 1])[VY];
        let b_end = state.vertex(state.lists[LIST_CLIP_B + 3 * b + 1])[VY];
        y = a_end.min(b_end);
        if a_end <= y {
            a += 1;
            if a >= split_a {
                return None;
            }
        }
        if b_end <= y {
            b += 1;
            // Retail compares with `jg`: one stale entry past the end is read.
            if b > split_b {
                return None;
            }
        }
    }

    // Start both rebuilt chains at the current scanline.
    let a_from = state.lists[LIST_CLIP_A + 3 * a];
    let a_to = state.lists[LIST_CLIP_A + 3 * a + 1];
    let from = *state.vertex(a_from);
    let to = *state.vertex(a_to);
    class.init_edge(edge_a, &from, &to);
    let b_from = state.lists[LIST_CLIP_B + 3 * b];
    let b_to = state.lists[LIST_CLIP_B + 3 * b + 1];
    let from_b = *state.vertex(b_from);
    let to_b = *state.vertex(b_to);
    class.init_edge(edge_b, &from_b, &to_b);
    if from[VY] < y {
        class.step_edge(edge_a, y - from[VY]);
    }
    if from_b[VY] < y {
        class.step_edge(edge_b, y - from_b[VY]);
    }

    let mut new_a = 0usize;
    let mut new_b = 0usize;
    let start_a = edge_vertex(state, class, edge_a, y, edge_b);
    state.lists[LIST_CHAIN_A] = start_a;
    let start_b = edge_vertex(state, class, edge_b, y, edge_a);
    state.lists[LIST_CHAIN_B] = start_b;

    // Merge both split lists down to the bottom, emitting chain vertices.
    loop {
        let a_end = state.vertex(state.lists[LIST_CLIP_A + 3 * a + 1])[VY];
        let b_end = state.vertex(state.lists[LIST_CLIP_B + 3 * b + 1])[VY];
        let (mut end, advance_a, advance_b) = if a_end < b_end {
            (a_end, true, b_end <= a_end)
        } else {
            (b_end, a_end <= b_end, true)
        };
        if end > i32::from(state.clip.y1) {
            end = i32::from(state.clip.y1);
        }
        class.step_edge(edge_a, end - y);
        class.step_edge(edge_b, end - y);
        if end >= i32::from(state.clip.y1) {
            break_into_bottom(state, class, new_a, new_b, edge_a, edge_b, end);
            return Some((new_a + 1, new_b + 1));
        }
        y = end;
        if advance_a {
            a += 1;
            if a >= split_a {
                break_into_bottom(state, class, new_a, new_b, edge_a, edge_b, end);
                return Some((new_a + 1, new_b + 1));
            }
        }
        if advance_b {
            b += 1;
            if b >= split_b {
                break_into_bottom(state, class, new_a, new_b, edge_a, edge_b, end);
                return Some((new_a + 1, new_b + 1));
            }
        }
        let flags = state.lists[LIST_CLIP_A + 3 * a + 2] | state.lists[LIST_CLIP_B + 3 * b + 2];
        if flags == LEFT_OF_CLIP || flags == RIGHT_OF_CLIP {
            break_into_bottom(state, class, new_a, new_b, edge_a, edge_b, end);
            return Some((new_a + 1, new_b + 1));
        }
        if advance_a {
            // The previous segment's end becomes a chain vertex.
            let joint = state.lists[LIST_CLIP_A + 3 * a - 2];
            let vertex = state.allocate();
            state.lists[LIST_CHAIN_A + 3 * new_a + 1] = vertex;
            if state.vertex(joint)[VY] <= end {
                *state.vertex_mut(vertex) = *state.vertex(joint);
            } else {
                class.copy_edge(state.vertex_mut(vertex), edge_a);
                state.vertex_mut(vertex)[VY] = end;
            }
            clamp_to_clip(state, class, vertex, edge_b);
            new_a += 1;
        }
        if advance_b {
            let joint = state.lists[LIST_CLIP_B + 3 * b - 2];
            let vertex = state.allocate();
            state.lists[LIST_CHAIN_B + 3 * new_b + 1] = vertex;
            if state.vertex(joint)[VY] <= end {
                *state.vertex_mut(vertex) = *state.vertex(joint);
            } else {
                class.copy_edge(state.vertex_mut(vertex), edge_b);
                state.vertex_mut(vertex)[VY] = end;
            }
            clamp_to_clip(state, class, vertex, edge_a);
            new_b += 1;
        }
        if advance_a {
            let from = *state.vertex(state.lists[LIST_CLIP_A + 3 * a]);
            let to = *state.vertex(state.lists[LIST_CLIP_A + 3 * a + 1]);
            class.init_edge(edge_a, &from, &to);
        }
        if advance_b {
            let from = *state.vertex(state.lists[LIST_CLIP_B + 3 * b]);
            let to = *state.vertex(state.lists[LIST_CLIP_B + 3 * b + 1]);
            class.init_edge(edge_b, &from, &to);
        }
        if advance_a {
            let slot = LIST_CHAIN_A + 3 * new_a;
            if state.lists[LIST_CLIP_A + 3 * a] == state.lists[LIST_CLIP_A + 3 * a - 2] {
                state.lists[slot] = state.lists[slot - 2];
            } else {
                let vertex = state.allocate();
                state.lists[slot] = vertex;
                class.copy_edge(state.vertex_mut(vertex), edge_a);
                state.vertex_mut(vertex)[VY] = end;
                clamp_to_clip(state, class, vertex, edge_b);
            }
        }
        if advance_b {
            let slot = LIST_CHAIN_B + 3 * new_b;
            if state.lists[LIST_CLIP_B + 3 * b] == state.lists[LIST_CLIP_B + 3 * b - 2] {
                state.lists[slot] = state.lists[slot - 2];
            } else {
                let vertex = state.allocate();
                state.lists[slot] = vertex;
                class.copy_edge(state.vertex_mut(vertex), edge_b);
                state.vertex_mut(vertex)[VY] = end;
                // Retail clamps this one toward its own edge (00473281).
                clamp_to_clip(state, class, vertex, edge_b);
            }
        }
    }
}

/// `00473291`: close both rebuilt chains with the edges' current points.
fn break_into_bottom(
    state: &mut RasterState,
    class: AttributeClass,
    new_a: usize,
    new_b: usize,
    edge_a: &Edge,
    edge_b: &Edge,
    y: i32,
) {
    let vertex = state.allocate();
    state.lists[LIST_CHAIN_A + 3 * new_a + 1] = vertex;
    class.copy_edge(state.vertex_mut(vertex), edge_a);
    state.vertex_mut(vertex)[VY] = y;
    clamp_to_clip(state, class, vertex, edge_b);
    let vertex = state.allocate();
    state.lists[LIST_CHAIN_B + 3 * new_b + 1] = vertex;
    class.copy_edge(state.vertex_mut(vertex), edge_b);
    state.vertex_mut(vertex)[VY] = y;
    clamp_to_clip(state, class, vertex, edge_a);
}

/// A new pool vertex at the edge's current point on scanline `y`, clamped
/// toward the other edge.
fn edge_vertex(
    state: &mut RasterState,
    class: AttributeClass,
    edge: &Edge,
    y: i32,
    other: &Edge,
) -> VertexRef {
    let vertex = state.allocate();
    class.copy_edge(state.vertex_mut(vertex), edge);
    state.vertex_mut(vertex)[VY] = y;
    clamp_to_clip(state, class, vertex, other);
    vertex
}

/// `FUN_004734A0`: push a vertex outside the clip sides onto the nearer
/// side, interpolating toward `edge`'s point on the same scanline.
fn clamp_to_clip(state: &mut RasterState, class: AttributeClass, vertex: VertexRef, edge: &Edge) {
    let left = i32::from(state.clip.x0) << 16;
    let right = i32::from(state.clip.x1) << 16;
    let x = state.vertex(vertex)[VX];
    if x < left {
        class.clamp_vertex(state.vertex_mut(vertex), edge, left);
    } else if x > right {
        class.clamp_vertex(state.vertex_mut(vertex), edge, right);
    }
}

/// `FUN_00472680`: drop chain segments above the clip top and start the
/// first crossing segment on the top scanline. Returns the new length.
fn clip_chain_top(state: &mut RasterState, list: usize, count: usize) -> usize {
    let mut kept = 0usize;
    let first = state.vertex(state.lists[list])[VY];
    let mut start = first.max(i32::from(state.clip.y0));
    for entry in 0..count {
        let from_ref = state.lists[list + 3 * entry];
        let to_ref = state.lists[list + 3 * entry + 1];
        let from = *state.vertex(from_ref);
        let to = *state.vertex(to_ref);
        if from[VY] < start {
            if to[VY] > start {
                let t = super::fixed::top_clip_ratio(start - from[VY], to[VY] - from[VY]);
                let vertex = state.allocate();
                let out = state.vertex_mut(vertex);
                out[VY] = start;
                for field in [VX, 6, 7, 3, 0, 1, 2, 8] {
                    out[field] = super::fixed::mul_q31(to[field].wrapping_sub(from[field]), t)
                        .wrapping_add(from[field]);
                }
                state.lists[list + 3 * kept] = vertex;
                state.lists[list + 3 * kept + 1] = to_ref;
                kept += 1;
            }
        } else if to[VY] > start {
            let flags = state.lists[list + 3 * entry + 2];
            state.lists[list + 3 * kept] = from_ref;
            state.lists[list + 3 * kept + 1] = to_ref;
            state.lists[list + 3 * kept + 2] = flags;
            kept += 1;
            start = to[VY];
        }
    }
    kept
}

/// `FUN_00472880`: split each chain segment where it crosses a clip side,
/// tagging every piece with the side it lies on.
fn split_chain_sides(
    state: &mut RasterState,
    class: AttributeClass,
    out_list: usize,
    in_list: usize,
    count: usize,
) -> usize {
    let left = i32::from(state.clip.x0) << 16;
    let right = i32::from(state.clip.x1) << 16;
    let mut out = out_list;
    let mut written = 0usize;
    for entry in 0..count {
        let from_ref = state.lists[in_list + 3 * entry];
        let to_ref = state.lists[in_list + 3 * entry + 1];
        state.lists[out] = from_ref;
        let from_x = state.vertex(from_ref)[VX];
        let to_x = state.vertex(to_ref)[VX];
        if from_x < left {
            state.lists[out + 2] = LEFT_OF_CLIP;
            if to_x > left {
                let crossing = split_vertex(state, class, out, from_ref, to_ref, left);
                written += 1;
                out += 3;
                state.lists[out] = crossing;
                state.lists[out + 2] = INSIDE_CLIP;
                if to_x > right {
                    let crossing = split_vertex(state, class, out, from_ref, to_ref, right);
                    written += 1;
                    out += 3;
                    state.lists[out] = crossing;
                    state.lists[out + 2] = RIGHT_OF_CLIP;
                }
            }
        } else if from_x > right {
            state.lists[out + 2] = RIGHT_OF_CLIP;
            if to_x < right {
                let crossing = split_vertex(state, class, out, from_ref, to_ref, right);
                written += 1;
                out += 3;
                state.lists[out] = crossing;
                state.lists[out + 2] = INSIDE_CLIP;
                if to_x < left {
                    let crossing = split_vertex(state, class, out, from_ref, to_ref, left);
                    written += 1;
                    out += 3;
                    state.lists[out] = crossing;
                    state.lists[out + 2] = LEFT_OF_CLIP;
                }
            }
        } else {
            state.lists[out + 2] = INSIDE_CLIP;
            if to_x < left {
                let crossing = split_vertex(state, class, out, from_ref, to_ref, left);
                written += 1;
                out += 3;
                state.lists[out] = crossing;
                state.lists[out + 2] = LEFT_OF_CLIP;
            } else if to_x > right {
                let crossing = split_vertex(state, class, out, from_ref, to_ref, right);
                written += 1;
                out += 3;
                state.lists[out] = crossing;
                state.lists[out + 2] = RIGHT_OF_CLIP;
            }
        }
        state.lists[out + 1] = to_ref;
        written += 1;
        out += 3;
        debug_assert!(out <= out_list + LIST_WORDS);
    }
    written
}

/// Allocate the crossing vertex of `from`→`to` at `clip_x` and end the
/// current piece there.
fn split_vertex(
    state: &mut RasterState,
    class: AttributeClass,
    slot: usize,
    from: VertexRef,
    to: VertexRef,
    clip_x: i32,
) -> VertexRef {
    let crossing = state.allocate();
    state.lists[slot + 1] = crossing;
    let (from, to) = (*state.vertex(from), *state.vertex(to));
    class.clip_vertex(state.vertex_mut(crossing), &from, &to, clip_x);
    crossing
}

/// `FUN_004734E0`: draw a quad whose chain turns upward as two triangles
/// sharing the chain's second and fourth vertices.
fn split_quad(
    state: &mut RasterState,
    chain: u32,
    target: &mut SpanTarget<'_>,
    context: &SpanContext<'_>,
    frame: ScanFrame,
) {
    let list = if chain == 1 {
        LIST_CHAIN_A
    } else {
        LIST_CHAIN_B
    };
    let corners = [
        *state.vertex(state.lists[list]),
        *state.vertex(state.lists[list + 1]),
        *state.vertex(state.lists[list + 4]),
        *state.vertex(state.lists[list + 7]),
    ];
    let nested = frame.nested();
    state.pool[0] = corners[0];
    state.pool[1] = corners[1];
    state.pool[2] = corners[3];
    scan_polygon(state, 3, target, context, nested);
    state.pool[0] = corners[2];
    state.pool[1] = corners[1];
    state.pool[2] = corners[3];
    scan_polygon(state, 3, target, context, nested);
}
