//! Actor Sub-E selector point from `40A9F0 -> 424F20 -> 424FA0`.
//!
//! The current draw frame and viewport own the source slot. This helper keeps
//! VIEW-space narrowing and camera inversion; it does not substitute an
//! actor-centre launch or choose an available emitter slot by compacting zeros.

use crate::native_model_frame::{NativeModelFrame, NativeWorldViewport};
use v2k_formats::collision::ProjectileEmitterDescriptor;
use v2k_formats::models::{AnimVars, ModelEntry};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorEmitterExternalFramePoint {
    pub emitter_index: u16,
    pub source_slot: u16,
    pub position_raw: [i32; 3],
    pub position_words: [i16; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorEmitterExternalFrameBoundary {
    NegativeSelector(i16),
    UnownedSourceSlot(u16),
    UnownedCurrentNode,
}

/// A bounded live provider for plain slots in the current native draw node.
/// Mounted children require their actual frame. Generated slots still need
/// command-register/cache custody and never borrow initial registers.
pub struct ActorEmitterExternalFramePresentation<'a> {
    pub descriptor: ProjectileEmitterDescriptor,
    pub root_model: &'a ModelEntry,
    pub root_vars: &'a AnimVars,
    pub frame: NativeModelFrame,
    pub viewport: NativeWorldViewport,
    pub stamp_origin: &'a mut dyn FnMut(ActorEmitterExternalFramePoint),
    pub last_boundary: &'a mut Option<ActorEmitterExternalFrameBoundary>,
    pub current_node_owned: bool,
    pub current_source_points_view: Option<[Option<[i32; 3]>; 2]>,
}

/// The scene transports native callback words into its selected toroidal
/// image. This policy is independent from intrinsic emitter-slot decoding.
pub struct ActorEmitterModelPresentation<'a> {
    pub emitter: ActorEmitterExternalFramePresentation<'a>,
    pub actor_origin_raw: [i16; 3],
    pub actor_draw_origin: [f32; 3],
}

impl ActorEmitterModelPresentation<'_> {
    pub(crate) fn draw_world(&self, raw: [i32; 3]) -> [f32; 3] {
        let viewport = self.emitter.viewport;
        let point_image = viewport.actor_world_image(raw.map(|v| v as i16));
        let actor_image = viewport.actor_world_image(self.actor_origin_raw);
        std::array::from_fn(|axis| {
            self.actor_draw_origin[axis]
                + point_image[axis].wrapping_sub(actor_image[axis]) as f32 / 256.0
        })
    }
}

impl ActorEmitterExternalFramePresentation<'_> {
    pub fn prepare_node(&mut self, model: &ModelEntry, _vars: &AnimVars) {
        self.current_node_owned = std::ptr::eq(model, self.root_model);
        self.current_source_points_view = None;
    }

    /// A proven mounted node can own plain source slots independently of the
    /// root. Generated slots still need command-register/cache custody.
    pub fn prepare_native_node(&mut self, model: &ModelEntry, frame: Option<&NativeModelFrame>) {
        self.current_node_owned = frame.is_some();
        self.current_source_points_view = frame.map(|frame| {
            self.frame = frame.clone();
            [
                self.descriptor.raw_word_at_0x12,
                self.descriptor.alternate_emitter_raw,
            ]
            .map(|slot| frame.resolve_type0_slot_view_raw(&model.records, slot))
        });
    }

    pub fn resolve(
        &mut self,
        sub_h_records: u16,
        parameters: [i16; 3],
    ) -> Result<Option<ActorEmitterExternalFramePoint>, ActorEmitterExternalFrameBoundary> {
        let selector = i32::from(parameters[0]) - 2 * i32::from(sub_h_records);
        if selector >= 0 && selector < i32::from(emitter_external_selector_count(self.descriptor)) {
            if !self.current_node_owned {
                return self.reject(ActorEmitterExternalFrameBoundary::UnownedCurrentNode);
            }
            let slot = if selector == 0 {
                self.descriptor.raw_word_at_0x12
            } else {
                self.descriptor.alternate_emitter_raw
            };
            if let Some(points) = self.current_source_points_view {
                let point_view = points[usize::from(selector != 0)]
                    .ok_or(ActorEmitterExternalFrameBoundary::UnownedSourceSlot(slot));
                let point_view = match point_view {
                    Ok(point) => point,
                    Err(boundary) => return self.reject(boundary),
                };
                let position_raw = self.viewport.view_point_to_world(point_view);
                let point = ActorEmitterExternalFramePoint {
                    emitter_index: selector as u16,
                    source_slot: slot,
                    position_raw,
                    position_words: position_raw.map(|v| v as i16),
                };
                (self.stamp_origin)(point);
                return Ok(Some(point));
            }
            if self
                .root_model
                .records
                .get(usize::from(slot) >> 1)
                .map(|r| r[0])
                != Some(0)
            {
                return self.reject(ActorEmitterExternalFrameBoundary::UnownedSourceSlot(slot));
            }
        }
        let result = resolve_actor_emitter_external_frame(
            sub_h_records,
            self.descriptor,
            parameters,
            self.root_model,
            self.root_vars,
            &self.frame,
            self.viewport,
        );
        match result {
            Ok(Some(point)) => {
                (self.stamp_origin)(point);
                Ok(Some(point))
            }
            Ok(None) => Ok(None),
            Err(boundary) => self.reject(boundary),
        }
    }

    fn reject<T>(
        &mut self,
        boundary: ActorEmitterExternalFrameBoundary,
    ) -> Result<T, ActorEmitterExternalFrameBoundary> {
        *self.last_boundary = Some(boundary);
        Err(boundary)
    }
}

/// A9F0 counts the two WORDS at Sub-E +12/+14, independently of bindings.
pub fn emitter_external_selector_count(descriptor: ProjectileEmitterDescriptor) -> u8 {
    u8::from(descriptor.raw_word_at_0x12 != 0) + u8::from(descriptor.alternate_emitter_raw != 0)
}

/// Return `None` for a preceding Sub-H or later component/terminal selector.
/// The caller must supply the current node frame; a retained actor root frame
/// cannot authenticate a differently mounted child node.
pub fn resolve_actor_emitter_external_frame(
    sub_h_records: u16,
    descriptor: ProjectileEmitterDescriptor,
    parameters: [i16; 3],
    model: &ModelEntry,
    vars: &AnimVars,
    frame: &NativeModelFrame,
    viewport: NativeWorldViewport,
) -> Result<Option<ActorEmitterExternalFramePoint>, ActorEmitterExternalFrameBoundary> {
    let selector = parameters[0];
    if selector < 0 {
        return Err(ActorEmitterExternalFrameBoundary::NegativeSelector(
            selector,
        ));
    }
    let emitter_index = i32::from(selector) - 2 * i32::from(sub_h_records);
    if emitter_index < 0 || emitter_index >= i32::from(emitter_external_selector_count(descriptor))
    {
        return Ok(None);
    }
    // 424F20 chooses +12 for ordinal zero, +14 otherwise. In particular an
    // absent +12 with a present +14 still resolves source slot zero here.
    let source_slot = if emitter_index == 0 {
        descriptor.raw_word_at_0x12
    } else {
        descriptor.alternate_emitter_raw
    };
    let point_view = frame.resolve_model_slot(model, vars, source_slot).ok_or(
        ActorEmitterExternalFrameBoundary::UnownedSourceSlot(source_slot),
    )?;
    let position_raw = viewport.view_point_to_world(point_view);
    Ok(Some(ActorEmitterExternalFramePoint {
        emitter_index: emitter_index as u16,
        source_slot,
        position_raw,
        position_words: position_raw.map(|component| component as i16),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor(a: u16, b: u16) -> ProjectileEmitterDescriptor {
        ProjectileEmitterDescriptor {
            projectile_method: 30,
            random_interval_us: 750000,
            spread_raw: 100,
            aim_threshold_raw: 16000,
            speed_override_raw: 0,
            target_axis_tolerance_raw: 0x600,
            sound_id: 70,
            raw_word_at_0x12: a,
            alternate_emitter_raw: b,
            stochastic_gate_mode: 0,
            auxiliary_command: 0,
            variable_bindings: [0; 4],
        }
    }

    fn viewport() -> NativeWorldViewport {
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

    fn frame() -> NativeModelFrame {
        NativeModelFrame::from_actor(
            viewport(),
            [15360, -1024, -32000],
            [
                [1517813760, 0, 1518469120],
                [0, 2147352576, 0],
                [-1518469120, 0, 1517813760],
            ],
        )
    }

    #[test]
    fn source_words_reserve_one_selector_even_with_zero_component_bindings() {
        assert_eq!(emitter_external_selector_count(descriptor(150, 0)), 1);
        assert_eq!(emitter_external_selector_count(descriptor(150, 151)), 2);
        assert_eq!(emitter_external_selector_count(descriptor(0, 151)), 1);
        assert_eq!(emitter_external_selector_count(descriptor(0, 0)), 0);
    }

    #[test]
    fn source_ordinal_does_not_compact_a_zero_first_emitter_word() {
        let model = ModelEntry {
            records: vec![[0, 14, -10, 126]],
            ..ModelEntry::default()
        };
        let point = resolve_actor_emitter_external_frame(
            6,
            descriptor(0, 1),
            [12, 0, 0],
            &model,
            &AnimVars::default(),
            &frame(),
            viewport(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(point.emitter_index, 0);
        assert_eq!(point.source_slot, 0);
        let later = resolve_actor_emitter_external_frame(
            6,
            descriptor(0, 1),
            [13, 0, 0],
            &model,
            &AnimVars::default(),
            &frame(),
            viewport(),
        )
        .unwrap();
        assert_eq!(later, None);
    }

    #[test]
    fn other_selector_domains_do_not_require_a_muzzle_source_slot() {
        let model = ModelEntry::default();
        for selector in [0, 11, 13, 32767] {
            assert_eq!(
                resolve_actor_emitter_external_frame(
                    6,
                    descriptor(150, 0),
                    [selector, 0, 0],
                    &model,
                    &AnimVars::default(),
                    &frame(),
                    viewport(),
                ),
                Ok(None)
            );
        }
        assert_eq!(
            resolve_actor_emitter_external_frame(
                6,
                descriptor(150, 0),
                [12, 0, 0],
                &model,
                &AnimVars::default(),
                &frame(),
                viewport(),
            ),
            Err(ActorEmitterExternalFrameBoundary::UnownedSourceSlot(150))
        );
    }

    #[v2k_test_support::retail_test]
    fn authored_newant_emitter_slot_matches_original_pe_424fa0_arithmetic() {
        let data_root = v2k_test_support::retail_dir();
        let mut session = crate::session::GameSession::init(&data_root).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(13, 1).unwrap();
        let model = session.cache.global_model(302).unwrap();
        let descriptor = session
            .cache
            .global_entity_type(47)
            .unwrap()
            .projectile_emitter_descriptor()
            .unwrap();
        assert_eq!(descriptor.raw_word_at_0x12, 150);
        assert_eq!(descriptor.alternate_emitter_raw, 0);
        assert_eq!(model.records[75], [0, 14, -10, 168]);
        assert_eq!(model.records[76], [14, 12, 0, 0]);
        assert_eq!(&model.cmd_words[..6], &[0x83, 668, 0, 58, 59, 152]);
        let (atlas, sprite) = session.cache.global_sprite(668).unwrap();
        let decoded = atlas.decode_sprite(sprite, 28).unwrap();
        assert_eq!([decoded.width, decoded.height], [2, 2]);
        assert!(decoded.rgba.iter().all(|byte| *byte == 0));
        let cases = [
            (
                [15360, -1024, -32000],
                [
                    [1517813760, 0, 1518469120],
                    [0, 2147352576, 0],
                    [-1518469120, 0, 1517813760],
                ],
                [15248, -1035, 33659],
            ),
            (
                [32760, 100, -32760],
                [[2147418112, 0, 0], [0, 2147418112, 0], [0, 0, 2147418112]],
                [32770, 88, 32938],
            ),
            (
                [-32760, -100, 32760],
                [[-2147418112, 0, 0], [0, 2147418112, 0], [0, 0, -2147418112]],
                [32758, -112, 32587],
            ),
        ];
        // These controlled actual-model cases execute original 424FA0 and
        // 46D610 PE instructions independently of Rust; they are not a live
        // Type47 capture or fitted poses. Separate shifted products survive.
        for (origin, axes, expected) in cases {
            let frame = NativeModelFrame::from_actor(viewport(), origin, axes);
            let point = resolve_actor_emitter_external_frame(
                6,
                descriptor,
                [12, 0, 0],
                model,
                &AnimVars::default(),
                &frame,
                viewport(),
            )
            .unwrap()
            .unwrap();
            assert_eq!(point.position_raw, expected);
            assert_eq!(point.position_words, expected.map(|word| word as i16));
        }
    }

    #[test]
    fn live_provider_stamps_only_admitted_owned_root_plain_slots() {
        let mut model = ModelEntry {
            records: vec![[0, 14, -10, 168]],
            ..ModelEntry::default()
        };
        let other = model.clone();
        let vars = AnimVars::default();
        let mut stamps = Vec::new();
        let mut stamp = |point| stamps.push(point);
        let mut boundary = None;
        {
            let mut presentation = ActorEmitterExternalFramePresentation {
                descriptor: descriptor(0, 1),
                root_model: &model,
                root_vars: &vars,
                frame: frame(),
                viewport: viewport(),
                stamp_origin: &mut stamp,
                last_boundary: &mut boundary,
                current_node_owned: true,
                current_source_points_view: None,
            };
            assert_eq!(presentation.resolve(6, [0, 0, 0]), Ok(None));
            presentation.prepare_node(&other, &vars);
            assert_eq!(
                presentation.resolve(6, [12, 0, 0]),
                Err(ActorEmitterExternalFrameBoundary::UnownedCurrentNode)
            );
            presentation.prepare_node(&model, &vars);
            assert!(presentation.resolve(6, [12, 0, 0]).unwrap().is_some());
        }
        assert_eq!(stamps.len(), 1);
        assert_eq!(
            boundary,
            Some(ActorEmitterExternalFrameBoundary::UnownedCurrentNode)
        );
        model.records[0] = [8, 192, 0, 0];
        let mut ignored = |_| panic!("unowned generated slot stamped a queue");
        let mut presentation = ActorEmitterExternalFramePresentation {
            descriptor: descriptor(0, 1),
            root_model: &model,
            root_vars: &vars,
            frame: frame(),
            viewport: viewport(),
            stamp_origin: &mut ignored,
            last_boundary: &mut boundary,
            current_node_owned: true,
            current_source_points_view: None,
        };
        assert_eq!(
            presentation.resolve(6, [12, 0, 0]),
            Err(ActorEmitterExternalFrameBoundary::UnownedSourceSlot(0))
        );
    }
}
