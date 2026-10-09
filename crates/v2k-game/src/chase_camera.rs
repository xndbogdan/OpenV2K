//! Retail gameplay chase-camera target construction and spring state.
//!
//! `FUN_0040ED10` builds two signed-8.8 targets around the tracked entity:
//! an eye on the fixed -Z side and a focus point 0xFA raw units along the
//! entity's stored forward basis. `FUN_0040F3A0` then advances independent
//! eye/focus springs with `FUN_0040F5E0`/`FUN_0040F800`. Keeping the state in
//! raw words preserves the original torus wrapping and integer response.
//!
//! Active gameplay/Intro2's `FUN_0044FFA0` caller supplies terrain context. That path
//! adjusts the eye anchor around terrain/sea level, then walks a three-cell-wide
//! clearance probe behind the craft before handing the target to the same
//! spring. Null-context callers retain the simpler craft-Y eye policy.

use v2k_formats::anim_frames::TerrainObjectTable;
use v2k_formats::models::CollisionModelPool;
use v2k_formats::terrain::TerrainGrid;
use v2k_render::Camera;

use crate::common_mover::target_correction::normalize_retail_q31;
use crate::hover::RETAIL_FRAME_DELTA_MAX_US;
use crate::native_model_frame::NativeWorldViewport;

const FIXED_SCALE: f32 = 256.0;
const BASE_DISTANCE_RAW: i32 = 0x800;
const ACTIVE_CAMERA_DISTANCE_STEP_RAW: i32 = 0x60;
const ACTIVE_CAMERA_MAX: u8 = 10;
const FOCUS_LOOK_AHEAD_RAW: i32 = 0xFA;
const MIN_FORWARD_SEPARATION_RAW: i16 = 0x100;
const TERRAIN_RAY_REFERENCE_DISTANCE_RAW: i32 = 0xC00;
const TERRAIN_RAY_STEP_RAW: i32 = 0x80;
const TERRAIN_RAY_CLEARANCE_Q31: i32 = 0x2580_0000;
const TERRAIN_OBJECT_KIND_NO_CLEARANCE: [u32; 2] = [0x0B, 0x1C];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChaseBodyBasis {
    /// Consecutive signed-Q31 lateral/up/forward vectors cached by
    /// `FUN_0040ED10` from entity `+0x0C`.
    pub lateral: [i32; 3],
    pub up: [i32; 3],
    pub forward: [i32; 3],
}

impl ChaseBodyBasis {
    /// `FUN_00413F70`'s identity result uses the positive sine-table endpoint,
    /// not the mathematically exact (unrepresentable) signed-Q31 value +1.
    pub const RETAIL_IDENTITY: Self = Self {
        lateral: [0x7FFE_0000, 0, 0],
        up: [0, 0x7FFE_0000, 0],
        forward: [0, 0, 0x7FFE_0000],
    };

    pub const fn from_vectors(vectors: [[i32; 3]; 3]) -> Self {
        Self {
            lateral: vectors[0],
            up: vectors[1],
            forward: vectors[2],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ChaseCameraTarget {
    /// Tracked entity origin in retail's wrapping signed-8.8 domain.
    pub position_raw: [i16; 3],
    /// Exact body vectors retained in their authored/runtime signed-Q31 form.
    pub body_basis: ChaseBodyBasis,
    pub active_camera: u8,
    pub parameters: ChaseCameraParameters,
}

/// `FUN_0040ED10` parameters 3 and 4. In play the console's in-play binding
/// set feeds them from the analog pad's right stick, through FGDK channels
/// that hold each write unfiltered (`0x004F71C0`, `0x004F71C8`). The PC
/// never has that device, so on the PC both stay zero.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChaseCameraParameters {
    /// Parameter 3: the eye moves sideways by `distance * swing >> 11`.
    pub swing: i32,
    /// Parameter 4: added to the chase distance.
    pub distance: i32,
}

/// Authored world data consulted only by active gameplay's terrain-aware eye
/// target. Section 9 selects a static object's model; model header `+0x0A`
/// supplies the clearance radius used by `FUN_00436C30`.
#[derive(Clone, Copy)]
pub struct ChaseTerrainContext<'a> {
    pub terrain: &'a TerrainGrid,
    pub terrain_objects: &'a TerrainObjectTable,
    pub model_pool: &'a dyn CollisionModelPool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChaseCameraPose {
    pub eye: [f32; 3],
    pub focus: [f32; 3],
    pub forward: [f32; 3],
    pub distance: f32,
}

impl ChaseCameraPose {
    /// Install the recovered look-at pose while preserving gameplay's
    /// left-handed +X-screen-right presentation.
    pub fn apply_to(self, camera: &mut Camera) {
        camera.position = self.eye;
        camera.yaw = self.forward[0].atan2(-self.forward[2]);
        camera.pitch = self.forward[1]
            .atan2((self.forward[0] * self.forward[0] + self.forward[2] * self.forward[2]).sqrt());
        camera.left_handed = true;
    }
}

#[derive(Debug, Default, Clone)]
pub struct ChaseCameraState {
    focus: SpringPoint,
    eye: SpringPoint,
    initialized: bool,
}

impl ChaseCameraState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Current signed-8.8 position of retail's first camera spring
    /// (`DAT_004DAF18`). Static-program opcode 6 measures against this focus
    /// owner before the later gameplay camera update, not against the eye or
    /// the floating-point render pose.
    pub const fn focus_position_raw(&self) -> [i16; 3] {
        self.focus.position
    }

    /// F3A0 -> F350 preserves the spring eye and the three separately
    /// normalized/crossed Q31 vectors. Model callbacks consume these words;
    /// reconstructing them from the floating-point presentation pose loses
    /// the two integer transform stages used by 46D610 and 199B0.
    pub fn native_viewport(&self) -> Option<NativeWorldViewport> {
        self.initialized
            .then(|| native_viewport_from_spring_points(self.eye.position, self.focus.position))
    }

    pub fn update(
        &mut self,
        target: ChaseCameraTarget,
        terrain: Option<ChaseTerrainContext<'_>>,
        elapsed_micros: u32,
    ) -> ChaseCameraPose {
        let distance_raw = chase_distance_raw(target.body_basis, target.active_camera)
            .wrapping_add(target.parameters.distance);
        let targets = camera_targets(
            target.position_raw,
            target.body_basis.forward,
            distance_raw,
            target.parameters.swing,
            terrain,
        );

        if !self.initialized {
            self.focus.position = targets.focus;
            self.eye.position = targets.eye;
            self.initialized = true;
        }

        // FUN_0040F3A0 receives microseconds shifted into its signed-Q31 time
        // domain. Retail's caller caps the frame delta before this call.
        let elapsed_micros = elapsed_micros.min(RETAIL_FRAME_DELTA_MAX_US);
        let dt_q31 = (elapsed_micros as i32) << 11;
        self.eye
            .step(targets.eye, EYE_TUNING, dt_q31, [true, false, true]);
        self.focus
            .step(targets.focus, FOCUS_TUNING, dt_q31, [true; 3]);

        let forward = view_forward(self.eye.position, self.focus.position);
        ChaseCameraPose {
            eye: raw_point_to_world(self.eye.position),
            focus: raw_point_to_world(self.focus.position),
            forward,
            distance: distance_raw as f32 / FIXED_SCALE,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct CameraTargets {
    eye: [i16; 3],
    focus: [i16; 3],
}

fn camera_targets(
    position: [i16; 3],
    body_forward: [i32; 3],
    distance_raw: i32,
    swing: i32,
    terrain: Option<ChaseTerrainContext<'_>>,
) -> CameraTargets {
    let mut eye = terrain.map_or_else(
        || no_ray_eye_target(position, distance_raw),
        |context| terrain_eye_target(position, distance_raw, context),
    );
    // The clearance probe stays on the entity's X; only the finished eye
    // moves sideways, by a 32-bit product shifted right 11.
    eye[0] = eye[0].wrapping_sub((distance_raw.wrapping_mul(swing) >> 11) as i16);
    targets_with_eye(position, body_forward, eye)
}

#[cfg(test)]
fn no_ray_targets(position: [i16; 3], body_forward: [i32; 3], distance_raw: i32) -> CameraTargets {
    targets_with_eye(
        position,
        body_forward,
        no_ray_eye_target(position, distance_raw),
    )
}

fn no_ray_eye_target(position: [i16; 3], distance_raw: i32) -> [i16; 3] {
    // Frontend callers pass null terrain context: the eye shares the
    // entity's Y and stays on FUN_0040ED10's fixed -Z compass bearing.
    [
        position[0],
        position[1],
        position[2].wrapping_sub(distance_raw as i16),
    ]
}

fn targets_with_eye(position: [i16; 3], body_forward: [i32; 3], eye: [i16; 3]) -> CameraTargets {
    let mut focus = std::array::from_fn(|axis| {
        position[axis].wrapping_add(q31_mul(body_forward[axis], FOCUS_LOOK_AHEAD_RAW) as i16)
    });

    // The retail view builder requires at least one cell of forward (+Z)
    // separation before handing the targets to the independent springs.
    if focus[2].wrapping_sub(eye[2]) < MIN_FORWARD_SEPARATION_RAW {
        focus[2] = eye[2].wrapping_add(MIN_FORWARD_SEPARATION_RAW);
    }

    CameraTargets { eye, focus }
}

/// Active `FUN_0040ED10` eye target. All intermediate values stay in retail's
/// signed 8.8 raw domain; only the completed sprung pose is converted to port
/// world units.
fn terrain_eye_target(
    position: [i16; 3],
    distance_raw: i32,
    context: ChaseTerrainContext<'_>,
) -> [i16; 3] {
    let terrain_y = i32::from(terrain_height_raw(
        context.terrain,
        position[0],
        position[2],
    ));
    let sea_y = i32::from((context.terrain.header[0] >> 8) as i16);
    let entity_y = i32::from(position[1]);
    let (anchor_y, floor_y) = terrain_eye_anchor_raw(entity_y, terrain_y, sea_y);

    // The initial 12-cell reference slope points from the adjusted anchor back
    // toward its terrain/sea floor. Each half-cell probe may replace it with a
    // steeper obstruction slope.
    let mut ray_horizontal = TERRAIN_RAY_REFERENCE_DISTANCE_RAW;
    let mut ray_vertical = anchor_y - floor_y;
    let mut sample_distance = TERRAIN_RAY_STEP_RAW;
    while sample_distance < distance_raw {
        let sample_z = position[2].wrapping_sub(sample_distance as i16);
        let clearance_y = terrain_clearance_height_raw(context, position[0], sample_z);
        let candidate_vertical = q31_mul(TERRAIN_RAY_CLEARANCE_Q31, sample_distance)
            .wrapping_add(clearance_y)
            .wrapping_sub(anchor_y);

        let current_cross = ray_vertical.wrapping_mul(sample_distance);
        let candidate_cross = ray_horizontal.wrapping_mul(candidate_vertical);
        if current_cross < candidate_cross {
            ray_horizontal = sample_distance;
            ray_vertical = candidate_vertical;
        }
        sample_distance += TERRAIN_RAY_STEP_RAW;
    }

    let ray_length = integer_sqrt(
        ray_horizontal
            .wrapping_mul(ray_horizontal)
            .wrapping_add(ray_vertical.wrapping_mul(ray_vertical)),
    );
    let horizontal_q31 = normalized_component_q31(ray_horizontal, ray_length);
    let vertical_q31 = normalized_component_q31(ray_vertical, ray_length);

    [
        position[0],
        (anchor_y as i16).wrapping_add(q31_mul(vertical_q31, distance_raw) as i16),
        position[2].wrapping_sub(q31_mul(horizontal_q31, distance_raw) as i16),
    ]
}

/// Terrain/sea anchor setup in the first half of `FUN_0040ED10`.
fn terrain_eye_anchor_raw(entity_y: i32, terrain_y: i32, sea_y: i32) -> (i32, i32) {
    let mut anchor_y = entity_y;
    let mut floor_y = terrain_y.max(sea_y);

    if entity_y < floor_y {
        let adjusted_entity_y = entity_y.max(terrain_y + 0x200);
        let below_sea_y = sea_y - 0x200;
        let deep_water_margin = sea_y - adjusted_entity_y - 0x400;
        if below_sea_y < adjusted_entity_y {
            anchor_y = below_sea_y;
            floor_y = below_sea_y;
        } else {
            floor_y = adjusted_entity_y;
            if deep_water_margin > 0 {
                floor_y = adjusted_entity_y + deep_water_margin / 2;
            }
        }
    } else {
        let high_altitude_excess = entity_y - floor_y - 0xA00;
        if high_altitude_excess > 0 {
            floor_y += high_altitude_excess * 2;
            if high_altitude_excess > 0x9FF {
                floor_y = entity_y;
            }
        }
        floor_y = floor_y.max(sea_y + 0x200);
    }

    anchor_y = anchor_y.max(floor_y);
    if floor_y + 0x600 < anchor_y {
        anchor_y -= 0x200;
    } else {
        // The executable's 0x55555555 multiply sequence is signed division by
        // three for this non-negative difference.
        anchor_y -= (anchor_y - floor_y) / 3;
    }

    (anchor_y, floor_y)
}

/// Exact signed-word bilinear height used at the tracked entity position.
pub(crate) fn terrain_height_raw(terrain: &TerrainGrid, x_raw: i16, z_raw: i16) -> i16 {
    let x_word = x_raw as u16;
    let z_word = z_raw as u16;
    let x0 = usize::from(x_word >> 8);
    let z0 = usize::from(z_word >> 8);
    let x1 = (x0 + 1) & 0xFF;
    let z1 = (z0 + 1) & 0xFF;
    let x_fraction = i32::from(x_word & 0xFF);
    let z_fraction = i32::from(z_word & 0xFF);
    let height = |x: usize, z: usize| {
        i32::from(terrain.cell(x, z).expect("complete 256x256 terrain").height as i8) << 5
    };

    let h00 = height(x0, z0);
    let h10 = height(x1, z0);
    let h01 = height(x0, z1);
    let h11 = height(x1, z1);
    let along_x0 = (((h10 - h00) * x_fraction) >> 8) + h00;
    let along_x1 = (((h11 - h01) * x_fraction) >> 8) + h01;
    ((((along_x1 - along_x0) * z_fraction) >> 8) + along_x0) as i16
}

/// `FUN_00436C30`: maximum of three cell-centre terrain heights across the
/// camera's lateral footprint, plus the selected static model radius unless
/// the descriptor belongs to one of the two explicitly excluded object kinds.
fn terrain_clearance_height_raw(context: ChaseTerrainContext<'_>, x_raw: i16, z_raw: i16) -> i32 {
    let center_x = i32::from((x_raw as u16) >> 8);
    let z = usize::from((z_raw as u16) >> 8);
    let next_z = (z + 1) & 0xFF;
    let mut maximum = i32::MIN;

    for x_offset in -1..=1 {
        let x = (center_x + x_offset).rem_euclid(256) as usize;
        let next_x = (x + 1) & 0xFF;
        let corners = [(x, z), (next_x, z), (x, next_z), (next_x, next_z)];
        let height_sum = corners
            .into_iter()
            .map(|(corner_x, corner_z)| {
                i32::from(
                    context
                        .terrain
                        .cell(corner_x, corner_z)
                        .expect("complete 256x256 terrain")
                        .height as i8,
                ) << 5
            })
            .sum::<i32>();
        // Retail corrects the arithmetic shift so negative sums truncate
        // toward zero, exactly matching Rust's signed division.
        let mut height = height_sum / 4;

        let cell = context.terrain.cell(x, z).expect("wrapped terrain cell");
        if cell.attribute != 0 {
            if let Some(descriptor) = context
                .terrain_objects
                .records
                .get(usize::from(cell.attribute))
            {
                if !TERRAIN_OBJECT_KIND_NO_CLEARANCE.contains(&descriptor.kind_index) {
                    let model_id = descriptor.model_id_for(cell.terrain_type);
                    if let Some(model) = context.model_pool.collision_model(usize::from(model_id)) {
                        height += i32::from(model.collision_radius_raw);
                    }
                }
            }
        }

        maximum = maximum.max(height);
    }

    maximum
}

fn integer_sqrt(value: i32) -> i32 {
    if value <= 0 {
        return 0;
    }
    let mut remainder = value as u32;
    let mut root = 0u32;
    let mut bit = 1u32 << 30;
    while bit > remainder {
        bit >>= 2;
    }
    while bit != 0 {
        if remainder >= root + bit {
            remainder -= root + bit;
            root = (root >> 1) + bit;
        } else {
            root >>= 1;
        }
        bit >>= 2;
    }
    root as i32
}

fn normalized_component_q31(component: i32, length: i32) -> i32 {
    if component.abs() < length {
        ((i64::from(component) << 31) / i64::from(length)) as i32
    } else {
        // Literal 0x40F084/0x40F0AD saturation. For a positive length this
        // produces 0x7FFF_FFFF for a positive component and -1 for a negative
        // component; the latter is deliberately near zero, not -1.0 Q31.
        (component ^ length) | i32::MAX
    }
}

fn chase_distance_raw(body: ChaseBodyBasis, active_camera: u8) -> i32 {
    let active_offset =
        i32::from(active_camera.min(ACTIVE_CAMERA_MAX)) * ACTIVE_CAMERA_DISTANCE_STEP_RAW;
    let mut lateral_offset = -q31_mul(body.lateral[0], active_offset);
    // FUN_00456E00 returns the 0x2300 absolute camera ceiling. The executable
    // clamps only this positive contribution to `ceiling - 0x800`.
    lateral_offset = lateral_offset.min(0x1B00);
    if body.up[1] <= 0 {
        lateral_offset = -lateral_offset;
    }
    BASE_DISTANCE_RAW + active_offset + lateral_offset
}

#[derive(Debug, Default, Clone, Copy)]
struct SpringPoint {
    position: [i16; 3],
    velocity: [i16; 3],
}

impl SpringPoint {
    fn step(
        &mut self,
        target: [i16; 3],
        tuning: [SpringAxis; 3],
        dt_q31: i32,
        leash_axis: [bool; 3],
    ) {
        for axis in 0..3 {
            let error = i32::from(target[axis].wrapping_sub(self.position[axis]));
            self.velocity[axis] =
                spring_velocity(i32::from(self.velocity[axis]), error, tuning[axis], dt_q31) as i16;
        }

        for axis in 0..3 {
            let displacement = q31_mul(i32::from(self.velocity[axis]), dt_q31) as i16;
            self.position[axis] = self.position[axis].wrapping_add(displacement);
        }

        for axis in 0..3 {
            if leash_axis[axis] {
                leash_to_target(
                    &mut self.position[axis],
                    target[axis],
                    tuning[axis].max_error,
                );
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct SpringAxis {
    velocity_limit: i32,
    acceleration: i32,
    dead_zone: i32,
    max_error: i32,
}

// First state block in FUN_0040F3A0, advanced by FUN_0040F800 toward focus.
const FOCUS_TUNING: [SpringAxis; 3] = [
    SpringAxis {
        velocity_limit: 0x1000,
        acceleration: 0x4000,
        dead_zone: 0x30,
        max_error: 0x100,
    },
    SpringAxis {
        velocity_limit: 0x1000,
        acceleration: 0x3000,
        dead_zone: 0x30,
        max_error: 0x100,
    },
    SpringAxis {
        velocity_limit: 0x1000,
        acceleration: 0x3000,
        dead_zone: 0x30,
        max_error: 0x100,
    },
];

// Second state block in FUN_0040F3A0, advanced by FUN_0040F5E0 toward eye.
const EYE_TUNING: [SpringAxis; 3] = [
    SpringAxis {
        velocity_limit: 0x800,
        acceleration: 0x800,
        dead_zone: 0x20,
        max_error: 0x200,
    },
    SpringAxis {
        velocity_limit: 0x800,
        acceleration: 0x3000,
        dead_zone: 0x60,
        max_error: 0x100,
    },
    SpringAxis {
        velocity_limit: 0x1000,
        acceleration: 0x2000,
        dead_zone: 0x20,
        max_error: 0x200,
    },
];

fn spring_velocity(current_velocity: i32, error: i32, tuning: SpringAxis, dt_q31: i32) -> i32 {
    // FUN_0040F740: proportional acceleration outside the dead zone. Reversal
    // doubles acceleration until velocity points toward the target again.
    let mut drive = if error > tuning.dead_zone {
        (error - tuning.dead_zone) * tuning.acceleration / (tuning.max_error - tuning.dead_zone)
    } else if error < -tuning.dead_zone {
        (error + tuning.dead_zone) * tuning.acceleration / (tuning.max_error - tuning.dead_zone)
    } else {
        0
    };
    if (drive > 0 && current_velocity < 0) || (drive < 0 && current_velocity > 0) {
        drive *= 2;
    }

    let velocity_cap = tuning.velocity_limit * 4;
    let accelerated =
        (current_velocity + q31_mul(dt_q31, drive)).clamp(-velocity_cap, velocity_cap);
    let damping = tuning.acceleration * accelerated / tuning.velocity_limit;
    let damped = accelerated - q31_mul(dt_q31, damping);

    if accelerated > 0 && damped < 0 || accelerated < 0 && damped > 0 {
        0
    } else {
        damped
    }
}

fn leash_to_target(position: &mut i16, target: i16, max_error: i32) {
    let error = i32::from(target.wrapping_sub(*position));
    if error > max_error {
        *position = target.wrapping_sub(max_error as i16);
    } else if error < -max_error {
        *position = target.wrapping_add(max_error as i16);
    }
}

/// Source-backed F3A0/F350 viewport from signed-short camera spring points.
/// Diagnostic callers may supply controlled points; this does not assert that
/// a free-camera GL view is a captured retail camera.
pub fn native_viewport_from_spring_points(eye: [i16; 3], focus: [i16; 3]) -> NativeWorldViewport {
    // 0F3A0 subtracts short spring coordinates, caps only positive DY, then
    // 57960 normalizes with its integer sqrt + 1 divisor and scaling loop.
    let forward = normalize_retail_q31([
        i32::from(focus[0].wrapping_sub(eye[0])),
        i32::from(focus[1].wrapping_sub(eye[1]).min(500)),
        i32::from(focus[2].wrapping_sub(eye[2])),
    ]);
    let right = normalize_retail_q31([forward[2], 0, forward[0].wrapping_neg()]);
    let up = [
        q31_mul(forward[1], right[2]).wrapping_sub(q31_mul(forward[2], right[1])),
        q31_mul(forward[2], right[0]).wrapping_sub(q31_mul(forward[0], right[2])),
        q31_mul(forward[0], right[1]).wrapping_sub(q31_mul(forward[1], right[0])),
    ];
    NativeWorldViewport {
        origin_raw: eye.map(i32::from),
        axes_q31: [right, up, forward],
        // F350 always enables the inverse-viewport products, even at a
        // cardinal look-at. Its identity field is a policy, not a float test.
        identity: false,
    }
}

fn view_forward(eye: [i16; 3], focus: [i16; 3]) -> [f32; 3] {
    let x = f32::from(focus[0].wrapping_sub(eye[0]));
    // FUN_0040F3A0 caps only an excessive positive vertical delta at 500 raw.
    let y = f32::from(focus[1].wrapping_sub(eye[1]).min(500));
    let z = f32::from(focus[2].wrapping_sub(eye[2]));
    let length = (x * x + y * y + z * z).sqrt();
    if length <= f32::EPSILON {
        [0.0, 0.0, 1.0]
    } else {
        [x / length, y / length, z / length]
    }
}

fn raw_point_to_world(raw: [i16; 3]) -> [f32; 3] {
    [
        f32::from(raw[0] as u16) / FIXED_SCALE,
        f32::from(raw[1]) / FIXED_SCALE,
        f32::from(raw[2] as u16) / FIXED_SCALE,
    ]
}

fn q31_mul(lhs: i32, rhs: i32) -> i32 {
    ((i64::from(lhs) * i64::from(rhs)) >> 31) as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use v2k_formats::anim_frames::{ModelSlotPattern, TerrainObjectDescriptor};
    use v2k_formats::models::{Billboard, ModelEntry, ModelInstance};
    use v2k_formats::terrain::{TerrainCell, GRID_SIZE};

    struct TestModelPool(Vec<ModelEntry>);

    impl CollisionModelPool for TestModelPool {
        fn collision_model(&self, global_id: usize) -> Option<&ModelEntry> {
            self.0.get(global_id)
        }
    }

    fn collision_model(collision_radius_raw: u16) -> ModelEntry {
        ModelEntry {
            index: 0,
            cmd_word_count: 0,
            extra_count: 0,
            flags: 0x40,
            slot_count: 0,
            face_val: 2,
            radius: 0,
            collision_radius_raw,
            collision_program: Vec::new(),
            records: Vec::new(),
            normal_pool: Vec::new(),
            cmd_words: Vec::new(),
            has_view_commands: false,
            vertices: Vec::new(),
            vertex_type_flags: Vec::new(),
            vertex_projection: Vec::new(),
            vertex_clip: Vec::new(),
            vertex_surface_origin: Vec::new(),
            triangles: Vec::new(),
            face_vertices: Vec::new(),
            normals: Vec::new(),
            face_cull: Vec::new(),
            face_materials: Vec::new(),
            face_uvs: Vec::new(),
            face_corner_normals: Vec::new(),
            face_shading: Vec::new(),
            shadow_triangles: Vec::new(),
            edges: Vec::new(),
            billboards: Vec::<Billboard>::new(),
            instances: Vec::<ModelInstance>::new(),
            painter_program: Vec::new(),
            name: None,
        }
    }

    fn flat_terrain(height: i8, sea_y_raw: i16) -> TerrainGrid {
        TerrainGrid {
            header: [i32::from(sea_y_raw) << 8, 0, 0, 0, 0],
            cells: vec![
                TerrainCell {
                    height: height as u8,
                    attribute: 0,
                    terrain_type: 0,
                };
                GRID_SIZE * GRID_SIZE
            ],
        }
    }

    fn terrain_objects(attribute: u8, model_id: u16, kind_index: u32) -> TerrainObjectTable {
        let empty = TerrainObjectDescriptor {
            model_ids: [0; 4],
            kind_index: 0,
            pattern: ModelSlotPattern::Static,
        };
        let mut records = vec![empty; 256];
        records[usize::from(attribute)] = TerrainObjectDescriptor {
            model_ids: [model_id; 4],
            kind_index,
            pattern: ModelSlotPattern::Static,
        };
        TerrainObjectTable { records }
    }

    fn context<'a>(
        terrain: &'a TerrainGrid,
        objects: &'a TerrainObjectTable,
        models: &'a TestModelPool,
    ) -> ChaseTerrainContext<'a> {
        ChaseTerrainContext {
            terrain,
            terrain_objects: objects,
            model_pool: models,
        }
    }

    fn raw_position(position: [f32; 3]) -> [i16; 3] {
        position.map(|value| ((value * FIXED_SCALE).round() as i32) as i16)
    }

    fn identity_target(position: [f32; 3], active_camera: u8) -> ChaseCameraTarget {
        ChaseCameraTarget {
            position_raw: raw_position(position),
            body_basis: ChaseBodyBasis::RETAIL_IDENTITY,
            active_camera,
            parameters: ChaseCameraParameters::default(),
        }
    }

    #[test]
    fn parameters_swing_the_eye_and_change_the_distance() {
        // A full right stick on the console pad: (0xFF - 0x80) * -8 = -1016
        // swings the eye towards +X by about half the distance; a full push
        // forward, (0 - 0x80) * 8 = -1024, halves the base distance.
        let mut camera = ChaseCameraState::default();
        let pose = camera.update(
            ChaseCameraTarget {
                parameters: ChaseCameraParameters {
                    swing: -1016,
                    distance: -1024,
                },
                ..identity_target([77.0, 0.0, 58.0], 0)
            },
            None,
            20_000,
        );
        let distance = 0x800 - 1024;
        assert_eq!(pose.distance, distance as f32 / 256.0);
        let swing = (distance * -1016) >> 11;
        assert_eq!(swing, -508);
        assert_eq!(pose.eye[0], 77.0 + 508.0 / 256.0);
        assert_eq!(pose.eye[2], 58.0 - distance as f32 / 256.0);

        // Released, the targets return to the plain chase position.
        let mut plain = ChaseCameraState::default();
        let rest = plain.update(identity_target([77.0, 0.0, 58.0], 0), None, 20_000);
        assert_eq!(rest.eye, [77.0, 0.0, 58.0 - 8.0]);
    }

    #[test]
    fn native_viewport_matches_retained_retail_camera_samples() {
        // Ledgered20260719-012555-intro2-camera-spring.jsonl, actual
        // duplicate-stable eye/focus/output words; no float reconstruction.
        let samples = [
            // Captureline2, tick1374.
            (
                [0, 0, -2048],
                [0, 0, 249],
                [0, 0, -2048],
                [[2147352519, 0, 0], [0, 2146418074, 0], [0, 0, 2146549146]],
            ),
            // Captureline438, tick6.
            (
                [0, 512, -6400],
                [0, 512, -4103],
                [0, 512, -6400],
                [[2147352519, 0, 0], [0, 2146418074, 0], [0, 0, 2146549146]],
            ),
            // Captureline1925, tick377.
            (
                [-29184, 704, 31066],
                [-29184, -333, -32519],
                [-29184, 704, 31066],
                [
                    [2147335177, 0, 0],
                    [0, 1895679152, 1007595737],
                    [0, -1007665404, 1895810224],
                ],
            ),
            // Captureline3413, tick749.
            (
                [-29184, -116, 30745],
                [-29184, -341, -32519],
                [-29184, -116, 30745],
                [
                    [2147351884, 0, 0],
                    [0, 2136069824, 211538605],
                    [0, -211551585, 2136200896],
                ],
            ),
            // Captureline3414, tick750.
            (
                [-29184, -116, 30745],
                [-29184, -341, -32519],
                [-29184, -116, 30745],
                [
                    [2147351884, 0, 0],
                    [0, 2136069824, 211538605],
                    [0, -211551585, 2136200896],
                ],
            ),
            // Captureline4215, tick950.
            (
                [-17347, -143, 29196],
                [-17534, -364, 31598],
                [-17347, -143, 29196],
                [
                    [2140901120, 0, 166669586],
                    [-15220653, 2137856075, 195512045],
                    [-165941918, -196113176, 2131510629],
                ],
            ),
            // Captureline5013, tick1149.
            (
                [-17619, -245, 28703],
                [-17627, -460, 31011],
                [-17619, -245, 28703],
                [
                    [2147351957, 0, 7374690],
                    [-683726, 2137191596, 199086102],
                    [-7408309, -199098311, 2137297222],
                ],
            ),
            // Captureline5014, tick1150.
            (
                [-17620, -245, 28702],
                [-17628, -460, 31010],
                [-17620, -245, 28702],
                [
                    [2147351957, 0, 7374690],
                    [-683726, 2137191596, 199086102],
                    [-7408309, -199098311, 2137297222],
                ],
            ),
            // Captureline5473, tick1265.
            (
                [-16104, 551, 1588],
                [-16117, 299, 3852],
                [-16104, 551, 1588],
                [
                    [2147351714, 0, 12269827],
                    [-1356734, 2133288191, 237443015],
                    [-12249797, -237457603, 2133349266],
                ],
            ),
            // Captureline5913, tick1375.
            (
                [-16253, 551, 1476],
                [-16102, 299, 3766],
                [-16253, 551, 1476],
                [
                    [2142736370, 0, -141363369],
                    [15428137, 2134349135, 233854294],
                    [140437432, -234372403, 2129812712],
                ],
            ),
            // Captureline5914, tick1375.
            (
                [-16253, 551, 1476],
                [-16102, 299, 3766],
                [-16253, 551, 1476],
                [
                    [2142736370, 0, -141363369],
                    [15428137, 2134349135, 233854294],
                    [140437432, -234372403, 2129812712],
                ],
            ),
            // Captureline5958, tick1386.
            (
                [-17145, 564, 1483],
                [-17401, 359, 3984],
                [-17145, 564, 1483],
                [
                    [2136302947, 0, 218615571],
                    [-17763057, 2139857018, 173579907],
                    [-217897667, -174488366, 2128758067],
                ],
            ),
            // Captureline6001, tick1397.
            (
                [-18852, 1394, 2762],
                [-19108, 360, 4592],
                [-18852, 1394, 2762],
                [
                    [2126750980, 0, 297468701],
                    [-145223153, 1873515713, 1038272197],
                    [-259563651, -1048393811, 1855474540],
                ],
            ),
            // Captureline3920, tick876.
            (
                [-18508, 1530, 30012],
                [-18253, -454, 31791],
                [-18508, 1530, 30012],
                [
                    [2125713993, 0, -304775159],
                    [225877443, 1441674583, 1575426434],
                    [204560452, -1591560537, 1427109977],
                ],
            ),
            // Captureline4055, tick910.
            (
                [-17579, -383, 29618],
                [-17762, -311, 31755],
                [-17579, -383, 29618],
                [
                    [2139611159, 0, 183166565],
                    [6142521, 2145257814, -71752213],
                    [-183041223, 72016219, 2137481395],
                ],
            ),
            // Captureline441, tick7.
            (
                [-29184, 590, 30834],
                [-29184, 0, -32519],
                [-29184, 590, 30834],
                [
                    [2147347834, 0, 0],
                    [0, 2072352042, 560095147],
                    [0, -560130571, 2072483113],
                ],
            ),
            // Captureline402, tick1474.
            (
                [0, 512, -6400],
                [0, 512, -4103],
                [0, 512, -6400],
                [[2147352519, 0, 0], [0, 2146418074, 0], [0, 0, 2146549146]],
            ),
            // Captureline3488, tick768.
            (
                [-26668, 791, 31178],
                [-26412, 76, -32748],
                [-26668, 791, 31178],
                [
                    [2120706329, 0, -337365571],
                    [135438732, 1965582693, 851378453],
                    [308678166, -862128471, 1941296279],
                ],
            ),
        ];
        for (eye, focus, origin_raw, axes_q31) in samples {
            assert_eq!(
                native_viewport_from_spring_points(eye, focus),
                NativeWorldViewport {
                    origin_raw,
                    axes_q31,
                    identity: false,
                },
            );
        }
    }

    #[test]
    fn reset_camera_has_no_native_viewport_until_authentic_initialization() {
        let mut camera = ChaseCameraState::default();
        assert_eq!(camera.native_viewport(), None);
        camera.update(identity_target([0.0, 0.0, 0.0], 0), None, 16_667);
        assert!(camera.native_viewport().is_some());
        camera.reset();
        assert_eq!(camera.native_viewport(), None);
    }

    #[test]
    fn source_positive_focus_dy_cap_is_preserved_without_clamping_negative_dy() {
        let capped = native_viewport_from_spring_points([0, 0, 0], [1000, 500, 2000]);
        assert_eq!(
            capped,
            native_viewport_from_spring_points([0, 0, 0], [1000, 900, 2000])
        );
        assert_ne!(
            capped,
            native_viewport_from_spring_points([0, 0, 0], [1000, -900, 2000])
        );
    }
    #[test]
    fn active_camera_uses_raw_lateral_and_up_vectors() {
        assert_eq!(
            chase_distance_raw(ChaseBodyBasis::RETAIL_IDENTITY, 0),
            0x800
        );
        assert_eq!(
            chase_distance_raw(ChaseBodyBasis::RETAIL_IDENTITY, 6),
            0x801
        );
        let lateral_zero = ChaseBodyBasis {
            lateral: [0, 0, 0],
            ..ChaseBodyBasis::RETAIL_IDENTITY
        };
        assert_eq!(chase_distance_raw(lateral_zero, 6), 0xA40);
        let lateral_negative = ChaseBodyBasis {
            lateral: [i32::MIN, 0, 0],
            ..ChaseBodyBasis::RETAIL_IDENTITY
        };
        assert_eq!(chase_distance_raw(lateral_negative, 6), 0xC80);
        assert_eq!(chase_distance_raw(lateral_negative, 0), 0x800);

        let inverted_up = ChaseBodyBasis {
            up: [0, -0x7FFE_0000, 0],
            ..lateral_negative
        };
        assert_eq!(chase_distance_raw(inverted_up, 6), 0x800);
    }

    #[test]
    fn focus_spring_matches_the_intro2_tick_1149_golden_transition() {
        // 20260719-012555-intro2-camera-spring.jsonl samples 5007..5009.
        // This pins the raw FUN_0040F800 arithmetic independently from the
        // moving subject and proves the remaining discrepancy is upstream.
        let mut focus = SpringPoint {
            position: [-17625, -460, 31013],
            velocity: [-55, 0, -21],
        };
        focus.step(
            [-17634, -508, 30986],
            FOCUS_TUNING,
            (8_000_i32) << 11,
            [true; 3],
        );
        assert_eq!(focus.position, [-17626, -460, 31012]);
        assert_eq!(focus.velocity, [-53, 0, -20]);
    }

    #[test]
    fn first_pose_uses_fixed_eye_bearing_and_forward_focus() {
        let mut camera = ChaseCameraState::default();
        assert_eq!(camera.focus_position_raw(), [0; 3]);
        let pose = camera.update(identity_target([77.0, -2.109_375, 58.0], 6), None, 20_000);

        assert_eq!(pose.eye, [77.0, -2.109_375, 58.0 - 2049.0 / 256.0]);
        assert_eq!(pose.focus[0], 77.0);
        assert_eq!(pose.focus[1], -2.109_375);
        // Positive Q31 cannot represent +1 exactly, matching the executable's
        // 249-raw result for a 0xFA look-ahead at the positive endpoint.
        assert_eq!(pose.focus[2], 58.0 + 249.0 / 256.0);
        assert_eq!(camera.focus_position_raw(), [19_712, -540, 15_097]);
        assert_eq!(pose.forward, [0.0, 0.0, 1.0]);
        assert_eq!(pose.distance, 2049.0 / 256.0);
    }

    #[test]
    fn focus_minimum_forward_separation_is_one_cell() {
        let targets = no_ray_targets([0, 0, 0], [0, 0, i32::MIN], 0);
        assert_eq!(targets.eye[2], 0);
        assert_eq!(targets.focus[2], 0x100);
    }

    #[test]
    fn flat_terrain_eye_target_matches_retail_raw_construction() {
        let terrain = flat_terrain(0, 0);
        let objects = terrain_objects(0, 0, 0);
        let models = TestModelPool(Vec::new());
        let eye = terrain_eye_target(
            [10 * 256, 4 * 256, 20 * 256],
            0x800,
            context(&terrain, &objects, &models),
        );

        assert_eq!(eye, [10 * 256, 1080, 20 * 256 - 2036]);
    }

    #[test]
    fn clearance_probe_adds_static_model_radius_except_authored_kinds() {
        let mut terrain = flat_terrain(0, 0);
        let cell = terrain
            .cells
            .get_mut(10 * GRID_SIZE + 20)
            .expect("test cell");
        cell.attribute = 1;
        let models = TestModelPool(vec![collision_model(0), collision_model(300)]);

        let ordinary = terrain_objects(1, 1, 0);
        assert_eq!(
            terrain_clearance_height_raw(context(&terrain, &ordinary, &models), 10 * 256, 20 * 256,),
            300
        );

        for excluded_kind in TERRAIN_OBJECT_KIND_NO_CLEARANCE {
            let excluded = terrain_objects(1, 1, excluded_kind);
            assert_eq!(
                terrain_clearance_height_raw(
                    context(&terrain, &excluded, &models),
                    10 * 256,
                    20 * 256,
                ),
                0
            );
        }
    }

    #[test]
    fn clearance_probe_raises_and_shortens_eye_behind_a_ridge() {
        let mut terrain = flat_terrain(0, 0);
        for x in 9..=12 {
            for z in [19, 20] {
                terrain.cells[x * GRID_SIZE + z].height = 40;
            }
        }
        let objects = terrain_objects(0, 0, 0);
        let models = TestModelPool(Vec::new());
        let eye = terrain_eye_target(
            [10 * 256, 4 * 256, 20 * 256],
            0x800,
            context(&terrain, &objects, &models),
        );

        assert!(eye[1] > 4 * 256);
        assert!(eye[2] > 20 * 256 - 0x800);
    }

    #[test]
    fn normalized_component_uses_retail_negative_saturation() {
        assert_eq!(normalized_component_q31(10, 10), i32::MAX);
        assert_eq!(normalized_component_q31(-10, 10), -1);
    }

    #[test]
    fn spring_step_matches_fun_0040f740_integer_math() {
        let tuning = FOCUS_TUNING[0];
        let dt_q31 = 20_000 << 11;
        assert_eq!(spring_velocity(0, 0x100, tuning, dt_q31), 289);

        let mut point = SpringPoint::default();
        point.step([0x100, 0, 0], FOCUS_TUNING, dt_q31, [true; 3]);
        assert_eq!(point.velocity[0], 289);
        assert_eq!(point.position[0], 5);
    }

    #[test]
    fn signed_word_state_takes_short_path_across_world_seam() {
        let mut camera = ChaseCameraState::default();
        camera.update(identity_target([255.5, 0.0, 20.0], 0), None, 20_000);
        let pose = camera.update(identity_target([0.5, 0.0, 20.0], 0), None, 20_000);

        assert!((0.0..256.0).contains(&pose.eye[0]));
        assert_eq!(
            raw_position([0.5, 0.0, 0.0])[0].wrapping_sub(raw_position([255.5, 0.0, 0.0])[0]),
            0x100
        );
        assert!(pose.eye[0] > 250.0 || pose.eye[0] < 5.0);
    }

    #[test]
    fn applying_pose_looks_at_sprung_focus_without_moving_eye_bearing() {
        let mut state = ChaseCameraState::default();
        let target = ChaseCameraTarget {
            position_raw: raw_position([10.0, 4.0, 20.0]),
            body_basis: ChaseBodyBasis {
                lateral: [0, 0, i32::MIN],
                up: [0, 0x7FFE_0000, 0],
                forward: [0x7FFE_0000, 0, 0],
            },
            active_camera: 0,
            parameters: ChaseCameraParameters::default(),
        };
        let pose = state.update(target, None, 20_000);
        assert_eq!(pose.eye, [10.0, 4.0, 12.0]);
        assert!(pose.forward[0] > 0.0);

        let mut camera = Camera::new(4.0 / 3.0);
        pose.apply_to(&mut camera);
        assert_eq!(camera.position, pose.eye);
        assert!(camera.forward()[0] > 0.0);
        assert!(camera.left_handed);
    }
}
