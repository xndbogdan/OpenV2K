//! The integer VIEW frame used by 4138F0/465870, 46D610 and 199B0.
//!
//! A marker is resolved after projection into the current child frame. Keep
//! every separately shifted Q31 product and the VIEW-space generator ordering.

use crate::ordinary_type47_shot_math::normalize_retail_vector_q31;
use std::collections::{HashMap, HashSet};
use v2k_formats::fixed_math::retail_sine_q15;
use v2k_formats::models::{
    AnimVars, ModelEntry, ModelFaceCullPlane, ModelInstance, ModelNativeInstanceFrame,
    ModelNativeMountCommand, ModelNativeSpatialGenerator, ModelViewSelection,
};
use v2k_formats::terrain::TerrainGrid;

/// The retained viewport supplied by 40F350, with axes in consecutive native
/// +14/+20/+2C order. Normal world VIEW uses dot products with these rows;
/// 199B0's inverse conversion consumes their columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeWorldViewport {
    pub origin_raw: [i32; 3],
    pub axes_q31: [[i32; 3]; 3],
    pub identity: bool,
}

impl NativeWorldViewport {
    /// The words the software producers transform with.
    pub fn words(self) -> v2k_render::projection::NativeViewportWords {
        v2k_render::projection::NativeViewportWords {
            origin_raw: self.origin_raw,
            axes_q31: self.axes_q31,
        }
    }

    pub fn world_vector_to_view(self, vector: [i32; 3]) -> [i32; 3] {
        if self.identity {
            vector
        } else {
            self.axes_q31.map(|axis| dot_q31(vector, axis))
        }
    }

    pub fn view_point_to_world(self, point: [i32; 3]) -> [i32; 3] {
        let displacement = if self.identity {
            point
        } else {
            std::array::from_fn(|axis| {
                dot_q31(point, std::array::from_fn(|row| self.axes_q31[row][axis]))
            })
        };
        std::array::from_fn(|axis| self.origin_raw[axis].wrapping_add(displacement[axis]))
    }

    /// 4138F0 forms each wrapped signed-short delta before 465870 subtracts
    /// the viewport origin. Preserve the same selected toroidal world image.
    pub fn actor_world_image(self, origin_raw: [i16; 3]) -> [i32; 3] {
        std::array::from_fn(|axis| {
            self.origin_raw[axis].wrapping_add(i32::from(
                origin_raw[axis].wrapping_sub(self.origin_raw[axis] as i16),
            ))
        })
    }
}

/// The active source slot table: world tf12 projects onto terrain, while
/// intrinsic/collision tf12 only aliases its source XYZ. A world scope with
/// unavailable terrain cannot resolve that callback.
#[derive(Clone, Copy)]
pub enum NativeSlotSurface<'a> {
    Intrinsic,
    World {
        viewport: NativeWorldViewport,
        terrain: Option<&'a TerrainGrid>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeModelFrame {
    pub origin_view_raw: [i32; 3],
    /// Consecutive local-axis vectors in VIEW space, native ctx+18..38.
    pub axes_view_q31: [[i32; 3]; 3],
    pub identity: bool,
    linked_view_slots: Vec<[Option<[i32; 3]>; 2]>,
}

impl NativeModelFrame {
    pub fn from_actor(
        viewport: NativeWorldViewport,
        origin_raw: [i16; 3],
        body_axes_q31: [[i32; 3]; 3],
    ) -> Self {
        let world_image = viewport.actor_world_image(origin_raw);
        let delta =
            std::array::from_fn(|axis| world_image[axis].wrapping_sub(viewport.origin_raw[axis]));
        Self {
            origin_view_raw: viewport.world_vector_to_view(delta),
            axes_view_q31: body_axes_q31.map(|axis| viewport.world_vector_to_view(axis)),
            // 4138F0 sets the submitted actor identity word to zero even when
            // its retained body basis happens to resemble identity.
            identity: false,
            linked_view_slots: Vec::new(),
        }
    }

    pub fn view_selection(&self) -> ModelViewSelection {
        ModelViewSelection::Retail {
            local_origin_from_camera_raw: if self.identity {
                self.origin_view_raw
            } else {
                self.axes_view_q31
                    .map(|axis| dot_q31(self.origin_view_raw, axis))
            },
        }
    }

    /// 46D3F0 uses raw anchor words, wrapping IMUL/ADD and a strict negative
    /// result. Generated or projected anchor coordinates are not its input.
    pub fn face_plane_visible(&self, plane: ModelFaceCullPlane) -> Option<bool> {
        let anchor = plane.native_anchor_raw?;
        let ModelViewSelection::Retail {
            local_origin_from_camera_raw: origin,
        } = self.view_selection()
        else {
            return None;
        };
        Some(
            (0..3).fold(0_i32, |sum, axis| {
                sum.wrapping_add(
                    origin[axis]
                        .wrapping_add(anchor[axis])
                        .wrapping_mul(plane.normal_raw[axis]),
                )
            }) < 0,
        )
    }

    /// D360/41E310 consumes the current node's already resolved type-0 VIEW
    /// cache. Odd X is mirrored after each Q31 product has been shifted.
    pub(crate) fn resolve_type0_slot_view_raw(
        &self,
        records: &[[i16; 4]],
        slot: u16,
    ) -> Option<[i32; 3]> {
        let [kind, a, b, c] = *records.get(usize::from(slot >> 1))?;
        (kind == 0).then(|| self.plain_slot_view_raw(slot, kind, [a, b, c]))
    }

    pub(crate) fn plain_slot_view_raw(
        &self,
        slot: u16,
        kind: i16,
        coordinates: [i16; 3],
    ) -> [i32; 3] {
        let mut local = coordinates.map(i32::from);
        if kind == 4 && slot & 1 != 0 {
            local[0] = 0;
        }
        std::array::from_fn(|axis| {
            let terms: [i32; 3] = if self.identity {
                std::array::from_fn(|row| if row == axis { local[row] } else { 0 })
            } else {
                std::array::from_fn(|row| mul_q31(local[row], self.axes_view_q31[row][axis]))
            };
            let xterm = if kind == 0 && slot & 1 != 0 {
                terms[0].wrapping_neg()
            } else {
                terms[0]
            };
            self.origin_view_raw[axis]
                .wrapping_add(xterm)
                .wrapping_add(terms[1])
                .wrapping_add(terms[2])
        })
    }

    /// Evaluate the canonical spatial operation after the interpreter has
    /// resolved every dependency in the current node's native VIEW cache.
    pub fn resolve_spatial_view_point(&self, generator: ModelNativeSpatialGenerator) -> [i32; 3] {
        generator.evaluate(self.origin_view_raw)
    }

    pub fn resolve_model_slot(
        &self,
        model: &ModelEntry,
        vars: &AnimVars,
        slot: u16,
    ) -> Option<[i32; 3]> {
        self.resolve_model_slot_with_surface(model, vars, slot, NativeSlotSurface::Intrinsic)
    }

    pub fn resolve_model_slot_with_surface(
        &self,
        model: &ModelEntry,
        vars: &AnimVars,
        slot: u16,
        surface: NativeSlotSurface<'_>,
    ) -> Option<[i32; 3]> {
        NativeSlots::new(self, &model.records, vars, surface).resolve(slot)
    }

    /// 67410 consumes the parent's already resolved VIEW attachment, then
    /// 676C0 installs a signed-permuted parent VIEW basis or the current mount.
    pub fn child(
        &self,
        model: &ModelEntry,
        instance: &ModelInstance,
        vars: &AnimVars,
    ) -> Option<Self> {
        self.child_with_surface(model, instance, vars, NativeSlotSurface::Intrinsic)
    }

    pub fn child_with_surface(
        &self,
        model: &ModelEntry,
        instance: &ModelInstance,
        vars: &AnimVars,
        surface: NativeSlotSurface<'_>,
    ) -> Option<Self> {
        let mut snapshot = vars.clone();
        snapshot.registers = instance.registers;
        let mut resolver = NativeSlots::new(self, &model.records, &snapshot, surface);
        let origin_view_raw = resolver.resolve(instance.attach_slot)?;
        let axes_view_q31 = match instance.native_frame.as_ref()? {
            ModelNativeInstanceFrame::Parent(code) => permute_axes(self.axes_view_q31, *code),
            ModelNativeInstanceFrame::Mount(commands) => {
                let mut mount = None;
                for command in commands {
                    match command {
                        ModelNativeMountCommand::Identity => {
                            mount = Some([[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]])
                        }
                        ModelNativeMountCommand::Parent(code) => {
                            mount = Some(permute_axes(self.axes_view_q31, *code))
                        }
                        ModelNativeMountCommand::Vertices { slots, registers } => {
                            let mut mount_vars = vars.clone();
                            mount_vars.registers = *registers;
                            let mut slots_resolver =
                                NativeSlots::new(self, &model.records, &mount_vars, surface);
                            let a = slots_resolver.resolve(slots[0])?;
                            let b = slots_resolver.resolve(slots[1])?;
                            let c = slots_resolver.resolve(slots[2])?;
                            let e = normalize_retail_vector_q31(sub(b, a));
                            let u = normalize_retail_vector_q31(cross_q31(sub(c, a), e));
                            mount = Some([u, cross_q31(e, u), e]);
                        }
                        ModelNativeMountCommand::Rotate { axis, angle } => {
                            rotate_mount(mount.as_mut()?, *axis, *angle)
                        }
                    }
                }
                mount?
            }
        };
        let linked_view_slots = instance
            .linked_slots
            .iter()
            .map(|slot| [resolver.resolve(*slot), resolver.resolve(*slot ^ 1)])
            .collect();
        Some(Self {
            origin_view_raw,
            axes_view_q31,
            identity: false,
            linked_view_slots,
        })
    }
}

struct NativeSlots<'a> {
    frame: &'a NativeModelFrame,
    surface: NativeSlotSurface<'a>,
    records: &'a [[i16; 4]],
    vars: &'a AnimVars,
    cache: HashMap<u16, Option<[i32; 3]>>,
    busy: HashSet<u16>,
}

impl<'a> NativeSlots<'a> {
    fn new(
        frame: &'a NativeModelFrame,
        records: &'a [[i16; 4]],
        vars: &'a AnimVars,
        surface: NativeSlotSurface<'a>,
    ) -> Self {
        Self {
            frame,
            surface,
            records,
            vars,
            cache: HashMap::new(),
            busy: HashSet::new(),
        }
    }

    fn resolve(&mut self, slot: u16) -> Option<[i32; 3]> {
        if let Some(point) = self.cache.get(&slot) {
            return *point;
        }
        let record = *self.records.get(usize::from(slot >> 1))?;
        if !self.busy.insert(slot) {
            return None;
        }
        let point = self.compute(slot, record);
        self.busy.remove(&slot);
        self.cache.insert(slot, point);
        point
    }

    // 70700 returns six-bit immediate/coarse values or an unsigned low word.
    fn phase(&self, word: i16) -> i32 {
        let index = usize::from(word as u16 & 0x3F);
        match word & 0xC0 {
            0x00 => i32::from(word & 0x3F),
            0x40 => i32::from(word & 0x3F) << 10,
            0x80 => self.vars.dynamic[index] & 0xFFFF,
            _ => i32::from(self.vars.registers[index]),
        }
    }

    fn compute(&mut self, slot: u16, [kind, a, b, c]: [i16; 4]) -> Option<[i32; 3]> {
        let odd = slot & 1;
        match kind {
            0 | 4 => Some(self.frame.plain_slot_view_raw(slot, kind, [a, b, c])),
            2 => {
                let source = self.resolve(c as u16 ^ odd)?;
                Some(
                    self.frame.resolve_spatial_view_point(
                        ModelNativeSpatialGenerator::Reflection { source },
                    ),
                )
            }
            5 => {
                let first = self.resolve(b as u16 ^ odd)?;
                let second = self.resolve(c as u16 ^ odd)?;
                Some(
                    self.frame
                        .resolve_spatial_view_point(ModelNativeSpatialGenerator::Midpoint {
                            first,
                            second,
                        }),
                )
            }
            6 => {
                // 46F1B0 resolves B, C, A in that order before arithmetic.
                let point_b = self.resolve(b as u16 ^ odd)?;
                let point_c = self.resolve(c as u16 ^ odd)?;
                let point_a = self.resolve(a as u16 ^ odd)?;
                Some(self.frame.resolve_spatial_view_point(
                    ModelNativeSpatialGenerator::Parallelogram {
                        a: point_a,
                        b: point_b,
                        c: point_c,
                    },
                ))
            }
            7 => {
                let first = self.resolve(b as u16 ^ odd)?;
                let second = self.resolve(c as u16 ^ odd)?;
                Some(self.frame.resolve_spatial_view_point(
                    ModelNativeSpatialGenerator::VectorSum { first, second },
                ))
            }
            8 => {
                // 46F7D0 consumes saved VIEW dependencies before evaluating a.
                let first = self.resolve(b as u16 ^ odd)?;
                let second = self.resolve(c as u16 ^ odd)?;
                Some(
                    self.frame
                        .resolve_spatial_view_point(ModelNativeSpatialGenerator::Lerp {
                            first,
                            second,
                            phase: self.phase(a),
                        }),
                )
            }
            9 => {
                // 46FB80 dispatches B, C, C+2, B+2, then evaluates current a.
                // Missing control points have no authenticated native cache.
                let points = [
                    self.resolve(b as u16 ^ odd)?,
                    self.resolve(c as u16 ^ odd)?,
                    self.resolve((c as u16).wrapping_add(2) ^ odd)?,
                    self.resolve((b as u16).wrapping_add(2) ^ odd)?,
                ];
                Some(
                    self.frame
                        .resolve_spatial_view_point(ModelNativeSpatialGenerator::Bezier {
                            points,
                            phase: self.phase(a),
                        }),
                )
            }
            11 => *self
                .frame
                .linked_view_slots
                .get(usize::try_from(a).ok()?)?
                .get(usize::from(odd))?,
            12 => {
                let source_view = self.resolve(a as u16 ^ odd)?;
                match self.surface {
                    NativeSlotSurface::Intrinsic => Some(source_view),
                    NativeSlotSurface::World { viewport, terrain } => {
                        // 4340B0 first inverse-projects the cached VIEW point.
                        // Only X/Z are wrapped to shorts for 45860. Replacing
                        // Y then re-projects the displacement, not a body-local
                        // point, and preserves each separate Q31 shift.
                        let world = viewport.view_point_to_world(source_view);
                        let mut displacement = std::array::from_fn(|axis| {
                            world[axis].wrapping_sub(viewport.origin_raw[axis])
                        });
                        displacement[1] = i32::from(
                            terrain?.bilinear_height_raw(world[0] as i16, world[2] as i16),
                        )
                        .wrapping_sub(viewport.origin_raw[1]);
                        Some(viewport.world_vector_to_view(displacement))
                    }
                }
            }
            _ => None,
        }
    }
}

fn mul_q31(a: i32, b: i32) -> i32 {
    ((i64::from(a) * i64::from(b)) >> 31) as i32
}
fn dot_q31(a: [i32; 3], b: [i32; 3]) -> i32 {
    (0..3).fold(0_i32, |sum, axis| {
        sum.wrapping_add(mul_q31(a[axis], b[axis]))
    })
}
fn sub(a: [i32; 3], b: [i32; 3]) -> [i32; 3] {
    std::array::from_fn(|axis| a[axis].wrapping_sub(b[axis]))
}
fn cross_q31(a: [i32; 3], b: [i32; 3]) -> [i32; 3] {
    std::array::from_fn(|axis| {
        mul_q31(a[(axis + 1) % 3], b[(axis + 2) % 3])
            .wrapping_sub(mul_q31(a[(axis + 2) % 3], b[(axis + 1) % 3]))
    })
}

fn permute_axes(axes: [[i32; 3]; 3], code: u16) -> [[i32; 3]; 3] {
    let destinations = match code & 7 {
        1 => [1, 2, 0],
        2 => [2, 0, 1],
        3 => [2, 1, 0],
        4 => [1, 0, 2],
        5 => [0, 2, 1],
        _ => [0, 1, 2],
    };
    let mut out = [[0; 3]; 3];
    for source in 0..3 {
        out[destinations[source]] = axes[source].map(|value| {
            if code & (0x20 >> source) != 0 {
                value.wrapping_neg()
            } else {
                value
            }
        });
    }
    out
}

fn sine_table_q31(angle: u16) -> i32 {
    let sine = retail_sine_q15(u32::from(angle));
    let magnitude = u32::from(sine.unsigned_abs());
    let repeated = ((magnitude << 16) | magnitude) as i32;
    if sine < 0 {
        repeated.wrapping_neg()
    } else {
        repeated
    }
}

fn rotate_mount(axes: &mut [[i32; 3]; 3], axis: u16, angle: u16) {
    let (first, second) = match axis {
        1 => (2, 0),
        2 => (0, 1),
        _ => (1, 2),
    };
    let old_first = axes[first];
    let old_second = axes[second];
    let sine = sine_table_q31(angle);
    let cosine = sine_table_q31(angle.wrapping_add(0x4000));
    axes[second] = std::array::from_fn(|component| {
        mul_q31(old_second[component], cosine)
            .wrapping_add(mul_q31(old_first[component], sine.wrapping_neg()))
    });
    axes[first] = std::array::from_fn(|component| {
        mul_q31(old_second[component], sine).wrapping_add(mul_q31(old_first[component], cosine))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{AuthoredWorldConstruction, EntityConstructionResources, EntityManager};
    use crate::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
    use crate::model_tree::inherited_child_vars;
    use crate::resource_cache::ResourceCache;
    use crate::session::GameSession;
    use crate::world_fx::WorldFx;
    use v2k_formats::models::ModelMaterializationContext;

    fn viewport() -> NativeWorldViewport {
        // Accepted retail camera words, paired with the accepted factory birth
        // as controlled cross-track inputs, not an identical native callback.
        NativeWorldViewport {
            origin_raw: [17324, 2336, 31398],
            axes_q31: [
                [2147345377, 0, 13827079],
                [-4394708, 2035509248, 682498122],
                [-13168014, -682542069, 2035555527],
            ],
            identity: false,
        }
    }

    fn session(world: u32) -> GameSession {
        let dir = v2k_test_support::retail_dir();
        assert!(
            dir.join("PRELOAD.DAT").is_file(),
            "retail factory corpus is required"
        );
        let mut session = GameSession::init(&dir).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(world, 1).unwrap();
        session
    }

    fn metadata(cache: &ResourceCache) -> Vec<EntityTypeRuntimeMetadata> {
        cache
            .global_entity_model_table()
            .iter()
            .enumerate()
            .map(|(id, slots)| {
                cache
                    .global_entity_type(id)
                    .map(EntityTypeRuntimeMetadata::from_section12)
                    .unwrap_or(EntityTypeRuntimeMetadata {
                        model_slots: *slots,
                        ..Default::default()
                    })
            })
            .collect()
    }

    #[v2k_test_support::retail_test]
    fn level_two_native_view_marker_matches_verified_pe_instructions() {
        let session = session(14);
        let rows = metadata(&session.cache);
        let mut fx = WorldFx::new();
        let manager = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                level: session.cache.level_desc().unwrap(),
                logical_world_index: 2,
                type_metadata: &rows,
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| {
                        session.cache.global_model(id).map(|model| model.radius)
                    }),
                },
                player_arrival: None,
                retail_tick: 0,
            },
            &mut fx,
        )
        .unwrap();
        let factory = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(39))
            .unwrap();
        assert_eq!(factory.model_index, Some(218));
        assert_eq!(factory.position_raw(), [15360, -1024, -32000]);
        let RetailRuntimeValue::Known(body) = factory.physical_body_basis_q31() else {
            panic!("native body owner");
        };
        let axes = [body.lateral, body.up, body.forward];
        assert_eq!(
            axes,
            [
                [1517813760, 0, 1518469120],
                [0, 2147352576, 0],
                [-1518469120, 0, 1517813760]
            ]
        );
        let root = session.cache.global_model(218).unwrap();
        let lift = session.cache.global_model(220).unwrap();
        assert_eq!(lift.records[4], [0, 0, 60, 0]);

        let frame = NativeModelFrame::from_actor(viewport(), factory.position_raw(), axes);
        assert_eq!(frame.origin_view_raw, [-1951, -2502, 3105]);
        assert_eq!(
            frame.axes_view_q31,
            [
                [1527493051, 479483033, 1430018626],
                [0, 2035385010, -682500410],
                [-1508598549, 485488338, 1448015407]
            ]
        );
        // Literal results come from interpreted instructions in the verified
        // PE: 46D610, 46F7D0/70700 and 199B0; no port formula supplied them.
        for (tick, phase, expected_attach, expected_register, expected) in [
            (500, 0, 114, 0, [15655, -964, -32302]),
            (750, 65535, 118, 65534, [15655, -845, -32304]),
        ] {
            let mut vars = factory.presentation_anim_vars(tick);
            vars.dynamic[..5].copy_from_slice(&[tick as i32, phase, 65535, 3, 3]);
            let material = root.materialize_with_context(ModelMaterializationContext {
                view_selection: frame.view_selection(),
                ..ModelMaterializationContext::intrinsic(&vars, None)
            });
            let instance = material
                .instances
                .iter()
                .find(|instance| instance.model_id == 220)
                .unwrap();
            assert_eq!(instance.attach_slot, expected_attach);
            assert_eq!(instance.registers[5], expected_register);
            let child = frame.child(root, instance, &vars).unwrap();
            let point = child
                .resolve_model_slot(lift, &inherited_child_vars(&vars, instance), 8)
                .unwrap();
            assert_eq!(
                viewport()
                    .view_point_to_world(point)
                    .map(|word| word as i16),
                expected
            );
        }

        // The admitted production/OpenGL fixture supplies this retained chase
        // viewport. Its two endpoint words were independently run through the
        // same actual PE instructions and match the rendered product writes.
        let screenshot_viewport = NativeWorldViewport {
            origin_raw: [15360, -617, 31489],
            axes_q31: [
                [2140688235, 0, 170418290],
                [-30595609, 2112445515, 384322943],
                [-167668551, -385542939, 2105803330],
            ],
            identity: false,
        };
        let frame = NativeModelFrame::from_actor(screenshot_viewport, factory.position_raw(), axes);
        for (tick, phase, expected) in [
            (500, 0, [15655, -966, -32301]),
            (750, 65535, [15654, -848, -32301]),
        ] {
            let mut vars = factory.presentation_anim_vars(tick);
            vars.dynamic[..5].copy_from_slice(&[tick as i32, phase, 65535, 3, 3]);
            let material = root.materialize_with_context(ModelMaterializationContext {
                view_selection: frame.view_selection(),
                ..ModelMaterializationContext::intrinsic(&vars, None)
            });
            let instance = material
                .instances
                .iter()
                .find(|instance| instance.model_id == 220)
                .unwrap();
            let child = frame.child(root, instance, &vars).unwrap();
            let point = child
                .resolve_model_slot(lift, &inherited_child_vars(&vars, instance), 8)
                .unwrap();
            assert_eq!(
                screenshot_viewport
                    .view_point_to_world(point)
                    .map(|word| word as i16),
                expected
            );
        }
    }

    fn inspect_factory_node(
        cache: &ResourceCache,
        id: usize,
        frame: Option<NativeModelFrame>,
        vars: &AnimVars,
        surface: NativeSlotSurface<'_>,
        depth: u8,
        markers: &mut std::collections::BTreeSet<usize>,
    ) {
        let Some(model) = cache.global_model(id) else {
            return;
        };
        let material =
            model.materialize_with_context(ModelMaterializationContext::intrinsic(vars, None));
        if material
            .vertex_type_flags
            .iter()
            .zip(&material.vertices)
            .any(|(kind, point)| *kind == 14 && point[0] == 0.0)
        {
            let native = frame
                .as_ref()
                .unwrap_or_else(|| panic!("factory marker model{id} has an unowned integer frame"));
            assert_eq!(model.records[4][0], 0, "factory marker model{id} is plain");
            assert!(
                native
                    .resolve_model_slot_with_surface(model, vars, 8, surface)
                    .is_some(),
                "factory marker model{id} resolves natively"
            );
            markers.insert(id);
        }
        if depth == 0 {
            return;
        }
        for instance in &material.instances {
            let child_id = usize::from(instance.model_id);
            if child_id == id {
                continue;
            }
            let child = frame
                .as_ref()
                .and_then(|frame| frame.child_with_surface(model, instance, vars, surface));
            inspect_factory_node(
                cache,
                child_id,
                child,
                &inherited_child_vars(vars, instance),
                surface,
                depth - 1,
                markers,
            );
        }
    }

    #[v2k_test_support::retail_test]
    fn native_frames_cover_all_authored_factory_markers_and_quiet_bases() {
        let mut session = session(13);
        let rows = metadata(&session.cache);
        let mut markers = std::collections::BTreeSet::new();
        let mut bases = 0;
        for world in 13..=49 {
            session.load_level_by_id(world, 1).unwrap();
            let manager = EntityManager::from_level_with_type_metadata(
                session.cache.level_desc().unwrap(),
                &rows,
                session.cache.terrain(),
            );
            for entity in manager
                .iter_all()
                .filter(|entity| matches!(entity.entity_type, 6 | 66))
            {
                // Corpus inspection explicitly invokes the source Euler writer.
                // This does not promote the legacy allocation into live custody.
                let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
                let body = crate::hover::HoverBasis::from_angle_words(heading, pitch, roll);
                let frame = NativeModelFrame::from_actor(
                    viewport(),
                    entity.position_raw(),
                    [body.lateral, body.up, body.forward],
                );
                let mut seen = std::collections::BTreeSet::new();
                let initial = entity.presentation_anim_vars(0);
                let mut delivered = initial.clone();
                if let RetailRuntimeValue::Known(Some(base)) = entity.base_factory_runtime {
                    let mut state = base;
                    state.current_scientists = state.required_scientists;
                    state.production_progress_raw = u16::MAX;
                    state.lifter_progress_raw = u16::MAX;
                    delivered = state.anim_vars(750);
                }
                let surface = NativeSlotSurface::World {
                    viewport: viewport(),
                    terrain: session.cache.terrain(),
                };
                for vars in [&initial, &delivered] {
                    inspect_factory_node(
                        &session.cache,
                        entity.model_index.unwrap(),
                        Some(frame.clone()),
                        vars,
                        surface,
                        8,
                        &mut seen,
                    );
                }
                if entity.entity_type == 6 {
                    bases += 1;
                    assert!(
                        seen.is_empty(),
                        "world{world} base unexpectedly requested a marker"
                    );
                } else {
                    markers.extend(seen);
                }
            }
        }
        assert_eq!(bases, 28);
        assert_eq!(
            markers,
            [195, 200, 203, 205, 207, 208, 211, 212, 217, 220, 229, 234]
                .into_iter()
                .collect()
        );
    }

    #[test]
    fn native_plain_mirror_negates_shifted_term_and_tf8_interpolates_view_cache() {
        let frame = NativeModelFrame {
            origin_view_raw: [10, -20, 30],
            axes_view_q31: [[0x40000000, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]],
            identity: false,
            linked_view_slots: Vec::new(),
        };
        let model = ModelEntry {
            records: vec![
                [0, 1, 0, 0],
                [0, 0, 0, -420],
                [0, 0, 120, -420],
                [8, 197, 2, 4],
            ],
            ..ModelEntry::default()
        };
        let mut vars = AnimVars::default();
        assert_eq!(
            frame.resolve_model_slot(&model, &vars, 0),
            Some([10, -20, 30])
        );
        assert_eq!(
            frame.resolve_model_slot(&model, &vars, 1),
            Some([10, -20, 30]),
            "mirror applies after the separately shifted product"
        );
        for (phase, expected) in [
            (0, [10, -20, -390]),
            (32768, [10, 38, -390]),
            (65535, [10, 98, -390]),
        ] {
            vars.registers[5] = phase;
            assert_eq!(frame.resolve_model_slot(&model, &vars, 6), Some(expected));
        }
        let unsupported = ModelEntry {
            records: vec![[14, 0, 0, 0]],
            ..ModelEntry::default()
        };
        assert!(frame.resolve_model_slot(&unsupported, &vars, 0).is_none());
    }
    #[test]
    fn native_tf5_averages_view_caches_after_projection_and_mirror() {
        let frame = NativeModelFrame {
            origin_view_raw: [10, -20, 30],
            axes_view_q31: [[0x40000000, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]],
            identity: false,
            linked_view_slots: Vec::new(),
        };
        let model = ModelEntry {
            records: vec![[0, 1, -3, 7], [0, 3, 0, -2], [5, 1, 0, 2]],
            ..ModelEntry::default()
        };
        let vars = AnimVars::default();
        assert_eq!(
            frame.resolve_model_slot(&model, &vars, 4),
            Some([10, -21, 32])
        );
        assert_eq!(
            frame.resolve_model_slot(&model, &vars, 5),
            Some([9, -21, 32])
        );
        // Averaging local X first would project 2 through the half-scale axis
        // and produce VIEW X=11, losing the source callbacks' quantization.
    }

    #[test]
    fn native_tf5_wraps_source_sum_and_truncates_negative_odd_values() {
        let frame = NativeModelFrame {
            origin_view_raw: [0; 3],
            axes_view_q31: [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]],
            identity: true,
            linked_view_slots: vec![
                [Some([i32::MAX, -3, i32::MIN]), Some([-1, 3, i32::MAX])],
                [Some([i32::MAX, 0, i32::MIN]), Some([0, 0, 2])],
            ],
        };
        let model = ModelEntry {
            records: vec![[11, 0, 0, 0], [11, 1, 0, 0], [5, 1, 0, 2]],
            ..ModelEntry::default()
        };
        let vars = AnimVars::default();
        assert_eq!(
            frame.resolve_model_slot(&model, &vars, 4),
            Some([-1, -1, 0])
        );
        assert_eq!(
            frame.resolve_model_slot(&model, &vars, 5),
            Some([0, 1, -1_073_741_823])
        );
    }

    #[test]
    fn native_reflection_parallelogram_and_sum_use_projected_mirrored_dependencies() {
        let frame = NativeModelFrame {
            origin_view_raw: [10, -20, 30],
            axes_view_q31: [[0x40000000, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]],
            identity: false,
            linked_view_slots: Vec::new(),
        };
        let model = ModelEntry {
            records: vec![
                [0, 1, -3, 7],
                [0, 3, 0, -2],
                [0, 5, 4, 2],
                [2, 0, 0, 2],
                [6, 0, 2, 4],
                [7, 0, 0, 2],
            ],
            ..ModelEntry::default()
        };
        let vars = AnimVars::default();
        for (slot, expected) in [
            (6, [9, -20, 32]),
            (7, [11, -20, 32]),
            (8, [9, -26, 33]),
            (9, [11, -26, 33]),
            (10, [11, -23, 34]),
            (11, [9, -23, 34]),
        ] {
            assert_eq!(
                frame.resolve_model_slot(&model, &vars, slot),
                Some(expected)
            );
        }
        // Local parallelogram X is -1; projecting it would yield VIEW X=9.
        // Mirroring that local result would yield 10, not the native odd 11.
    }

    #[test]
    fn native_curves_evaluate_current_unsigned_phase_words_after_control_points() {
        let frame = NativeModelFrame {
            origin_view_raw: [10, -20, 30],
            axes_view_q31: [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]],
            identity: true,
            linked_view_slots: Vec::new(),
        };
        let mut model = ModelEntry {
            records: vec![
                [0, 0, 0, 0],   // B: P0
                [0, 200, 0, 0], // B+2: P3
                [0, 0, 0, 0],   // C: P1
                [0, 200, 0, 0], // C+2: P2
                [8, 0, 0, 2],
                [9, 0, 0, 4],
            ],
            ..ModelEntry::default()
        };
        let mut vars = AnimVars::default();
        vars.registers[5] = 0x8000;
        for (phase_word, dynamic, expected_lerp, expected_curve, expected_odd_curve) in [
            (32, 0, 10, 10, 6),
            (0x60, 0, 110, 102, -94),
            (0x81, 0x18000, 110, 102, -94),
            (0x81, -32768, 110, 102, -94),
            (0xC5, 0, 110, 102, -94),
        ] {
            model.records[4][1] = phase_word;
            model.records[5][1] = phase_word;
            vars.dynamic[1] = dynamic;
            assert_eq!(
                frame.resolve_model_slot(&model, &vars, 8),
                Some([expected_lerp, -20, 30])
            );
            assert_eq!(
                frame.resolve_model_slot(&model, &vars, 10),
                Some([expected_curve, -20, 30])
            );
            assert_eq!(
                frame.resolve_model_slot(&model, &vars, 11),
                Some([expected_odd_curve, -20, 30])
            );
        }
        // Missing native controls must not fall back to a valid P0 coordinate.
        model.records[2][0] = 14;
        assert_eq!(frame.resolve_model_slot(&model, &vars, 10), None);
    }

    #[test]
    fn native_spatial_generators_consume_the_resolved_world_surface_dependency() {
        use v2k_formats::terrain::{TerrainCell, GRID_SIZE};
        let viewport = NativeWorldViewport {
            origin_raw: [0; 3],
            axes_q31: [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]],
            identity: true,
        };
        let frame = NativeModelFrame {
            origin_view_raw: [10, -20, 30],
            axes_view_q31: viewport.axes_q31,
            identity: true,
            linked_view_slots: Vec::new(),
        };
        let terrain = TerrainGrid {
            header: [0; 5],
            cells: vec![
                TerrainCell {
                    height: 2,
                    attribute: 0,
                    terrain_type: 0
                };
                GRID_SIZE * GRID_SIZE
            ],
        };
        let model = ModelEntry {
            records: vec![[0, 20, 60, 5], [12, 0, 0, 0], [2, 0, 0, 2], [7, 0, 0, 2]],
            ..ModelEntry::default()
        };
        let vars = AnimVars::default();
        let surface = NativeSlotSurface::World {
            viewport,
            terrain: Some(&terrain),
        };
        assert_eq!(
            frame.resolve_model_slot(&model, &vars, 4),
            Some([-10, -80, 25])
        );
        assert_eq!(
            frame.resolve_model_slot_with_surface(&model, &vars, 4, surface),
            Some([-10, -104, 25])
        );
        assert_eq!(
            frame.resolve_model_slot_with_surface(&model, &vars, 6, surface),
            Some([50, 124, 40])
        );
        assert_eq!(
            frame.resolve_model_slot_with_surface(
                &model,
                &vars,
                4,
                NativeSlotSurface::World {
                    viewport,
                    terrain: None
                }
            ),
            None
        );
    }

    #[v2k_test_support::retail_test]
    fn high_player4_native_shadow_sources_resolve_nested_tf5_midpoints() {
        let session = session(13);
        let model = session.cache.global_model(41).expect("high player4 model");
        assert_eq!(model.name.as_deref(), Some("player4"));
        let frame = NativeModelFrame {
            origin_view_raw: [10, -20, 30],
            axes_view_q31: [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]],
            identity: true,
            linked_view_slots: Vec::new(),
        };
        // Controlled identity VIEW pose over the actual variant-1 records,
        // including center 118 -> mirrored midpoint sources 76/77.
        for (record, source, expected) in [
            (39, 72, [162, -20, 290]),
            (40, 74, [162, -20, -150]),
            (41, 76, [162, -20, 70]),
            (60, 118, [10, -20, 70]),
            (62, 122, [10, -20, 290]),
            (64, 126, [10, -20, -150]),
        ] {
            assert_eq!(model.records[record], [13, source as i16, 0, 0]);
            assert_eq!(
                frame.resolve_model_slot(model, &AnimVars::default(), source),
                Some(expected),
                "player4 tf13 source {source}"
            );
        }
    }

    #[test]
    fn native_world_tf12_keeps_intrinsic_alias_separate_and_requires_terrain() {
        use v2k_formats::terrain::{TerrainCell, TerrainGrid, GRID_SIZE};
        let viewport = NativeWorldViewport {
            origin_raw: [32760, 500, -32760],
            axes_q31: [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]],
            identity: true,
        };
        let mut frame = NativeModelFrame::from_actor(viewport, [0; 3], viewport.axes_q31);
        frame.origin_view_raw = [10, -20, 30];
        frame.identity = true;
        let model = ModelEntry {
            records: vec![[0, 20, 60, 5], [12, 0, 0, 0]],
            ..ModelEntry::default()
        };
        let terrain = TerrainGrid {
            header: [0; 5],
            cells: vec![
                TerrainCell {
                    height: 2,
                    attribute: 0,
                    terrain_type: 0
                };
                GRID_SIZE * GRID_SIZE
            ],
        };
        let vars = AnimVars::default();
        assert_eq!(
            frame.resolve_model_slot(&model, &vars, 2),
            Some([30, 40, 35])
        );
        assert_eq!(
            frame.resolve_model_slot(&model, &vars, 3),
            Some([-10, 40, 35])
        );
        assert_eq!(
            frame.resolve_model_slot_with_surface(
                &model,
                &vars,
                2,
                NativeSlotSurface::World {
                    viewport,
                    terrain: Some(&terrain)
                }
            ),
            Some([30, -436, 35])
        );
        assert_eq!(
            frame.resolve_model_slot_with_surface(
                &model,
                &vars,
                3,
                NativeSlotSurface::World {
                    viewport,
                    terrain: Some(&terrain)
                }
            ),
            Some([-10, -436, 35])
        );
        assert_eq!(
            frame.resolve_model_slot_with_surface(
                &model,
                &vars,
                2,
                NativeSlotSurface::World {
                    viewport,
                    terrain: None
                }
            ),
            None
        );
    }

    #[v2k_test_support::retail_test]
    fn factory_two_authored_pair_and_unused_factor14_obey_selector_requests() {
        let session = session(13);
        let basis = crate::hover::HoverBasis::from_angle_words(0x6000, 0, 0);
        let frame = NativeModelFrame::from_actor(
            viewport(),
            [0; 3],
            [basis.lateral, basis.up, basis.forward],
        );
        let root = session.cache.global_model(221).unwrap();
        let lift = session.cache.global_model(222).unwrap();
        assert_eq!(lift.records[4], [0, 0, 128, 0]);
        for phase in [0, 65535] {
            let mut vars = AnimVars::default();
            vars.dynamic[1] = phase;
            vars.dynamic[2] = 65535;
            let material = root.materialize(&vars);
            let instance = material
                .instances
                .iter()
                .find(|instance| instance.model_id == 222)
                .unwrap();
            let child = frame
                .child_with_surface(
                    root,
                    instance,
                    &vars,
                    NativeSlotSurface::World {
                        viewport: viewport(),
                        terrain: session.cache.terrain(),
                    },
                )
                .unwrap();
            assert!(child
                .resolve_model_slot(lift, &inherited_child_vars(&vars, instance), 8)
                .is_some());
            assert!(lift
                .materialize(&inherited_child_vars(&vars, instance))
                .vertex_type_flags
                .contains(&14));
        }
        let factor14 = session.cache.global_model(193).unwrap();
        assert!(factor14
            .records
            .iter()
            .any(|record| record[0] == 14 && record[1] == 0));
        assert!(!factor14.materialize(&AnimVars::default()).vertex_type_flags.contains(&14), "the unused tf14 record is referenced only by optional diagnostic0x38, not a normal primitive");
    }

    #[v2k_test_support::retail_test]
    fn world_tf12_factory_attachment_matches_pe_on_five_authored_terrains() {
        // Actual 4340B0 -> 445860 instructions with each complete authored
        // terrain image, verified by the bounded PE oracle. These controlled
        // contexts pair authored factory data and independent captured camera
        // inputs; no simultaneous native callback capture is claimed.
        let cases = [
            (
                19,
                [10535, -4921, -5617],
                [
                    -2057477001,
                    -195144086,
                    -581960518,
                    0,
                    2035385010,
                    -682500410,
                    613982271,
                    -653934950,
                    -1950383408,
                ],
                [10694, -5172, -6369],
            ),
            (
                24,
                [-25604, 7235, 25106],
                [
                    -650229239,
                    650384875,
                    1939796312,
                    0,
                    2035385010,
                    -682500410,
                    -2046307399,
                    -206664582,
                    -616320805,
                ],
                [-26395, 7175, 24922],
            ),
            (
                28,
                [-25571, 4071, 31566],
                [
                    -2147279846,
                    4394573,
                    13167612,
                    0,
                    2035385010,
                    -682500410,
                    -13826658,
                    -682477294,
                    -2035493407,
                ],
                [-25648, 3809, 30785],
            ),
            (
                29,
                [24907, -4716, 425],
                [
                    -13826236,
                    -682456466,
                    -2035431287,
                    0,
                    2035385010,
                    -682500410,
                    2147214313,
                    -4394440,
                    -13167211,
                ],
                [25705, -4740, 349],
            ),
            (
                36,
                [-4778, -580, 589],
                [
                    -285067761,
                    676423707,
                    2017446995,
                    0,
                    2035385010,
                    -682500410,
                    -2128233433,
                    -90604062,
                    -270167087,
                ],
                [-5588, -593, 549],
            ),
        ];
        for (world, origin_view_raw, axes_view_q31, expected) in cases {
            let session = session(world);
            let model = session.cache.global_model(201).unwrap();
            assert_eq!(model.records[28], [12, 54, 0, 0]);
            assert_eq!(model.records[27], [0, 70, 0, 805]);
            let axes_view_q31 = std::array::from_fn(|axis| {
                std::array::from_fn(|component| axes_view_q31[axis * 3 + component])
            });
            let frame = NativeModelFrame {
                origin_view_raw,
                axes_view_q31,
                identity: false,
                linked_view_slots: Vec::new(),
            };
            assert_eq!(
                frame.resolve_model_slot_with_surface(
                    model,
                    &AnimVars::default(),
                    56,
                    NativeSlotSurface::World {
                        viewport: viewport(),
                        terrain: session.cache.terrain()
                    }
                ),
                Some(expected),
                "world{world}"
            );
        }
    }
}
