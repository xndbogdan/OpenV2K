//! Software backend: run model-tree painter programs into the primitive
//! queue the way the original executes a model's command stream.
//!
//! Retail interprets a model tree as one command stream: primitives are
//! queued as they are reached, `0x06`/`0x26`/`0x46`/`0x66`/`0x86`/`0xA6`/
//! `0xC6` open a FIFO group and `0x15` a sorted one (each keyed by a VIEW
//! depth), `0xE6` closes the innermost, and an instance command expands its
//! child in place, inside whatever groups are open. The port submits a tree
//! one node at a time (parent body, its billboards, then each child, then
//! [`crate::Renderer::end_model_node`]), so each node becomes a frame whose
//! program runs up to its next instance and resumes when the next child,
//! or the node's end, arrives.

use v2k_formats::models::{ModelPainterOp, ModelPainterSorting};

use crate::software::{PrimitiveQueue, QueueError};
use crate::sw_model::{PreparedBillboards, PreparedNode};

/// One submitted tree node.
struct Frame {
    /// `None` when the node lies beyond the far plane: none of its
    /// commands run, so its program is empty too.
    node: Option<PreparedNode>,
    program: Vec<ModelPainterOp>,
    cursor: usize,
    billboards: Option<PreparedBillboards>,
    /// Billboards arrive in their own draw right after the body.
    awaiting_billboards: bool,
}

/// How far to run the top frame.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Until {
    /// Up to the next instance, which a later child may expand.
    NextInstance,
    /// Through the given instance; earlier ones had no child submitted.
    Instance(usize),
    /// To the end, skipping instances whose child never arrived.
    End,
}

#[derive(Default)]
pub(crate) struct PainterStack {
    frames: Vec<Frame>,
    /// Groups the running tree opened: `true` when queued, `false` when
    /// its key was unresolved and the group was skipped.
    groups: Vec<bool>,
}

impl PainterStack {
    pub(crate) fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Whether the top frame still waits for its billboard draw.
    pub(crate) fn awaiting_billboards(&self) -> bool {
        self.frames
            .last()
            .is_some_and(|frame| frame.awaiting_billboards)
    }

    /// A tree node's body arrived. A child first runs its parent up to its
    /// instance; a root first finishes any tree left open.
    pub(crate) fn begin_node(
        &mut self,
        queue: &mut PrimitiveQueue,
        node: Option<PreparedNode>,
        program: &[ModelPainterOp],
        parent_instance: Option<usize>,
    ) -> Result<(), QueueError> {
        match parent_instance {
            Some(index) if !self.frames.is_empty() => {
                self.settle_billboards(queue)?;
                self.run(queue, Until::Instance(index))?;
            }
            _ => self.finish_all(queue)?,
        }
        let program = if node.is_some() {
            program.to_vec()
        } else {
            Vec::new()
        };
        self.frames.push(Frame {
            node,
            program,
            cursor: 0,
            billboards: None,
            awaiting_billboards: true,
        });
        Ok(())
    }

    /// The top node's billboards arrived; run it up to its first instance.
    pub(crate) fn attach_billboards(
        &mut self,
        queue: &mut PrimitiveQueue,
        billboards: Option<PreparedBillboards>,
    ) -> Result<(), QueueError> {
        if let Some(frame) = self.frames.last_mut() {
            frame.billboards = billboards;
            frame.awaiting_billboards = false;
        }
        self.run(queue, Until::NextInstance)
    }

    /// The top node's children are done: finish and drop it, and close the
    /// tree's leftover groups once its root ends.
    pub(crate) fn end_node(&mut self, queue: &mut PrimitiveQueue) -> Result<(), QueueError> {
        if self.frames.is_empty() {
            return Ok(());
        }
        self.settle_billboards(queue)?;
        self.run(queue, Until::End)?;
        self.frames.pop();
        if self.frames.is_empty() {
            self.close_groups(queue)?;
        }
        Ok(())
    }

    /// Finish every open frame, innermost first: another producer is about
    /// to queue, or the queue is about to drain.
    pub(crate) fn finish_all(&mut self, queue: &mut PrimitiveQueue) -> Result<(), QueueError> {
        while !self.frames.is_empty() {
            self.end_node(queue)?;
        }
        Ok(())
    }

    /// A top frame whose billboard draw never came runs without them.
    fn settle_billboards(&mut self, queue: &mut PrimitiveQueue) -> Result<(), QueueError> {
        if self.awaiting_billboards() {
            self.attach_billboards(queue, None)?;
        }
        Ok(())
    }

    fn close_groups(&mut self, queue: &mut PrimitiveQueue) -> Result<(), QueueError> {
        while let Some(queued) = self.groups.pop() {
            if queued {
                queue.end_group()?;
            }
        }
        Ok(())
    }

    fn run(&mut self, queue: &mut PrimitiveQueue, until: Until) -> Result<(), QueueError> {
        let Self { frames, groups } = self;
        let Some(frame) = frames.last_mut() else {
            return Ok(());
        };
        let Some(node) = frame.node.as_ref() else {
            return Ok(());
        };
        if frame.program.is_empty() {
            // A body without a program queues in mesh order once.
            if frame.cursor == 0 {
                frame.cursor = 1;
                node.emit_all(queue)?;
                if let Some(billboards) = &frame.billboards {
                    billboards.emit_all(queue)?;
                }
            }
            return Ok(());
        }
        while let Some(op) = frame.program.get(frame.cursor) {
            match op {
                ModelPainterOp::Instance { instance_index } => match until {
                    Until::NextInstance => return Ok(()),
                    Until::Instance(target) if *instance_index == target => {
                        frame.cursor += 1;
                        return Ok(());
                    }
                    _ => {}
                },
                ModelPainterOp::Face { triangle_range, .. } => {
                    node.emit_face(queue, triangle_range.start)?;
                }
                ModelPainterOp::Edge { edge_index, .. } => node.emit_edge(queue, *edge_index)?,
                ModelPainterOp::Billboard {
                    billboard_index, ..
                } => {
                    if let Some(billboards) = &frame.billboards {
                        billboards.emit(queue, *billboard_index)?;
                    }
                }
                ModelPainterOp::BeginGroup {
                    depth_key, sorting, ..
                } => match node.group_key(depth_key) {
                    Some(key) => {
                        match sorting {
                            ModelPainterSorting::Sorted => queue.begin_sorted_group(key)?,
                            ModelPainterSorting::Unsorted => queue.begin_fifo_group(key)?,
                        }
                        groups.push(true);
                    }
                    None => groups.push(false),
                },
                ModelPainterOp::EndGroup => {
                    if groups.pop() == Some(true) {
                        queue.end_group()?;
                    }
                }
            }
            frame.cursor += 1;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::software::{FillSlot, WORLD_ARENA_BYTES};
    use v2k_formats::models::ModelPainterDepthKey;

    fn face(first: usize) -> ModelPainterOp {
        ModelPainterOp::Face {
            triangle_range: first..first + 1,
            depth_key: ModelPainterDepthKey::Fixed(0),
        }
    }

    fn group(depth_key: ModelPainterDepthKey, sorting: ModelPainterSorting) -> ModelPainterOp {
        ModelPainterOp::BeginGroup {
            depth_key,
            sorting,
            referenced_slots: Vec::new(),
        }
    }

    fn fifo(key: i32) -> ModelPainterOp {
        group(
            ModelPainterDepthKey::Fixed(key),
            ModelPainterSorting::Unsorted,
        )
    }

    fn instance(instance_index: usize) -> ModelPainterOp {
        ModelPainterOp::Instance { instance_index }
    }

    /// The colours of the flat triangles the queue drains, in order.
    fn drain(queue: &mut PrimitiveQueue) -> Vec<u32> {
        let mut colours = Vec::new();
        queue.flush(&mut |slot, payload| {
            assert_eq!(slot, FillSlot::FlatTriangle);
            colours.push(u32::from_le_bytes(payload[12..16].try_into().unwrap()));
            0
        });
        colours
    }

    fn run_tree(
        parent: (&[(usize, u32, i32)], &[ModelPainterOp]),
        children: &[(usize, &[(usize, u32, i32)], &[ModelPainterOp])],
    ) -> Vec<u32> {
        let mut queue = PrimitiveQueue::new(WORLD_ARENA_BYTES).unwrap();
        let mut stack = PainterStack::default();
        let node = PreparedNode::test_triangles(parent.0);
        stack
            .begin_node(&mut queue, Some(node), parent.1, None)
            .unwrap();
        stack.attach_billboards(&mut queue, None).unwrap();
        for &(index, faces, program) in children {
            let node = PreparedNode::test_triangles(faces);
            stack
                .begin_node(&mut queue, Some(node), program, Some(index))
                .unwrap();
            stack.attach_billboards(&mut queue, None).unwrap();
            stack.end_node(&mut queue).unwrap();
        }
        stack.end_node(&mut queue).unwrap();
        assert!(stack.is_empty());
        drain(&mut queue)
    }

    #[test]
    fn a_group_spanning_an_instance_holds_the_child_in_command_order() {
        // Root scope sorts by depth: D (500), the group (100), A (50). The
        // FIFO group keeps B, the child's E, then C, whatever their depths.
        let order = run_tree(
            (
                &[(0, 0xA, 50), (1, 0xB, 900), (2, 0xC, 10), (3, 0xD, 500)],
                &[
                    face(0),
                    fifo(100),
                    face(1),
                    instance(0),
                    face(2),
                    ModelPainterOp::EndGroup,
                    face(3),
                ],
            ),
            &[(0, &[(0, 0xE, 1000)], &[face(0)])],
        );
        assert_eq!(order, [0xD, 0xB, 0xE, 0xC, 0xA]);
    }

    #[test]
    fn missing_children_are_skipped_in_place() {
        let order = run_tree(
            (
                &[(0, 0xA, 1), (1, 0xB, 2), (2, 0xC, 3)],
                &[
                    fifo(0),
                    face(0),
                    instance(0),
                    face(1),
                    instance(1),
                    face(2),
                    ModelPainterOp::EndGroup,
                ],
            ),
            &[(1, &[(0, 0xE, 4)], &[face(0)])],
        );
        assert_eq!(order, [0xA, 0xB, 0xE, 0xC]);
    }

    #[test]
    fn unresolved_groups_leave_their_primitives_in_the_enclosing_scope() {
        let order = run_tree(
            (
                &[(0, 0xA, 10), (1, 0xB, 30), (2, 0xC, 20)],
                &[
                    group(
                        ModelPainterDepthKey::Unresolved,
                        ModelPainterSorting::Unsorted,
                    ),
                    face(0),
                    face(1),
                    ModelPainterOp::EndGroup,
                    face(2),
                ],
            ),
            &[],
        );
        assert_eq!(order, [0xB, 0xC, 0xA]);
    }

    #[test]
    fn a_sorted_group_orders_its_children_by_depth_inside_the_group() {
        let order = run_tree(
            (
                &[(0, 0xA, 10), (1, 0xB, 30), (2, 0xC, 20), (3, 0xD, 40)],
                &[
                    group(ModelPainterDepthKey::Fixed(25), ModelPainterSorting::Sorted),
                    face(0),
                    face(1),
                    face(2),
                    ModelPainterOp::EndGroup,
                    face(3),
                ],
            ),
            &[],
        );
        assert_eq!(order, [0xD, 0xB, 0xC, 0xA]);
    }

    #[test]
    fn a_new_root_finishes_a_tree_left_open() {
        let mut queue = PrimitiveQueue::new(WORLD_ARENA_BYTES).unwrap();
        let mut stack = PainterStack::default();
        let program = [
            fifo(0),
            face(0),
            instance(0),
            face(1),
            ModelPainterOp::EndGroup,
        ];
        let node = PreparedNode::test_triangles(&[(0, 0xA, 1), (1, 0xB, 2)]);
        stack
            .begin_node(&mut queue, Some(node), &program, None)
            .unwrap();
        stack.attach_billboards(&mut queue, None).unwrap();
        let other = PreparedNode::test_triangles(&[(0, 0xC, 3)]);
        stack
            .begin_node(&mut queue, Some(other), &[face(0)], None)
            .unwrap();
        stack.finish_all(&mut queue).unwrap();
        assert!(!queue.group_open());
        assert_eq!(drain(&mut queue), [0xC, 0xA, 0xB]);
    }
}
