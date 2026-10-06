//! Draw-owned Sub-M selector zero: 0A9F0 -> 198E0 -> 199B0.
//!
//! The callback returns the current node's descriptor-selected model point and
//! independently moves Sub-M +88's product by signed world descriptor offsets.

use crate::entity::{world_position_raw, Entity, EntityManager};
use crate::entity_collision_state::RetailRuntimeValue;
use crate::native_model_frame::{NativeModelFrame, NativeSlotSurface, NativeWorldViewport};
use v2k_formats::collision::StatusComponentDescriptor;
use v2k_formats::models::{AnimVars, LinkedModelSlots, ModelEntry, ModelMaterializationContext};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubMProductMarkerWrite {
    pub product_id: u32,
    pub position_raw: [i16; 3],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeMarkerBoundary {
    UnownedActorBody,
    UnownedChildFrame {
        model_id: u16,
        attach_slot: u16,
    },
    UnsupportedMarkerSlot {
        model_id: usize,
        slot: u16,
        kind: Option<i16>,
    },
    UnownedFacePlane,
}

pub struct SubMPresentation<'a> {
    descriptor: StatusComponentDescriptor,
    product_id: u32,
    actor_origin_world: [f32; 3],
    root_draw_origin: [f32; 3],
    write: &'a mut Option<SubMProductMarkerWrite>,
    pub(crate) marker_local: Option<[f64; 3]>,
    actor_native: Option<([i16; 3], [[i32; 3]; 3])>,
    native_viewport: Option<NativeWorldViewport>,
    pub(crate) native_frame: Option<NativeModelFrame>,
    native_root_image: [i32; 3],
    pub(crate) marker_native_world: Option<[i32; 3]>,
    pub(crate) native_requested: bool,
    native_issue: Option<NativeMarkerBoundary>,
    native_parent_issues: Vec<Option<NativeMarkerBoundary>>,
    native_boundaries: Vec<NativeMarkerBoundary>,
}

impl<'a> SubMPresentation<'a> {
    pub fn new(
        descriptor: StatusComponentDescriptor,
        product_id: u32,
        actor_origin_world: [f32; 3],
        root_draw_origin: [f32; 3],
        write: &'a mut Option<SubMProductMarkerWrite>,
    ) -> Self {
        Self {
            descriptor,
            product_id,
            actor_origin_world,
            root_draw_origin,
            write,
            marker_local: None,
            actor_native: None,
            native_viewport: None,
            native_frame: None,
            native_root_image: [0; 3],
            marker_native_world: None,
            native_requested: false,
            native_issue: None,
            native_parent_issues: Vec::new(),
            native_boundaries: Vec::new(),
        }
    }

    /// Type6 and Type66 share Sub-M. A product handle is optional: 198E0 still
    /// returns geometry when the factory has not allocated a product.
    pub fn for_entity(
        entity: &Entity,
        root_draw_origin: [f32; 3],
        write: &'a mut Option<SubMProductMarkerWrite>,
    ) -> Option<Self> {
        let RetailRuntimeValue::Known(Some(base)) = entity.base_factory_runtime else {
            return None;
        };
        // 0A9F0 consumes preceding Sub-H/I ranges before Sub-M. The authored
        // Type6/66 profiles prove both absent; unavailable/composite dispatch
        // must retain its existing fallback until that policy is recovered.
        if !matches!(
            &entity.sub_h_external_frame_runtime,
            RetailRuntimeValue::Known(None)
        ) || !matches!(
            &entity.actor_animation_runtime,
            RetailRuntimeValue::Known(None)
        ) {
            return None;
        }
        let mut presentation = Self::new(
            base.status_descriptor,
            base.production
                .map_or(0, |state| state.spawned_pickup_handle),
            entity.position,
            root_draw_origin,
            write,
        );
        if let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() {
            presentation.actor_native = Some((
                entity.position_raw(),
                [basis.lateral, basis.up, basis.forward],
            ));
        }
        Some(presentation)
    }

    /// Install the source-owned normal viewport. Free/inspection cameras keep
    /// the existing float lane and do not manufacture native viewport words.
    pub fn with_native_viewport(mut self, viewport: NativeWorldViewport) -> Self {
        self.native_requested = true;
        self.native_issue = Some(NativeMarkerBoundary::UnownedActorBody);
        if let Some((origin, basis)) = self.actor_native {
            self.native_root_image = viewport.actor_world_image(origin);
            self.native_frame = Some(NativeModelFrame::from_actor(viewport, origin, basis));
            self.native_viewport = Some(viewport);
            self.native_issue = None;
        }
        self
    }

    pub(crate) fn native_view_selection(&self) -> Option<v2k_formats::models::ModelViewSelection> {
        self.native_frame
            .as_ref()
            .map(NativeModelFrame::view_selection)
    }

    pub(crate) fn native_slot_surface<'s>(
        &self,
        world: bool,
        terrain: Option<&'s v2k_formats::terrain::TerrainGrid>,
    ) -> NativeSlotSurface<'s> {
        if world {
            if let Some(viewport) = self.native_viewport {
                return NativeSlotSurface::World { viewport, terrain };
            }
        }
        NativeSlotSurface::Intrinsic
    }

    pub(crate) fn begin_child(
        &mut self,
        model: &ModelEntry,
        instance: &v2k_formats::models::ModelInstance,
        vars: &AnimVars,
        authored: bool,
        surface: NativeSlotSurface<'_>,
    ) -> Option<NativeModelFrame> {
        self.native_parent_issues.push(self.native_issue.clone());
        let previous = self.native_frame.take();
        self.native_frame = if authored {
            previous
                .as_ref()
                .and_then(|frame| frame.child_with_surface(model, instance, vars, surface))
        } else {
            None
        };
        if previous.is_some() {
            self.native_issue =
                self.native_frame
                    .is_none()
                    .then_some(NativeMarkerBoundary::UnownedChildFrame {
                        model_id: instance.model_id,
                        attach_slot: instance.attach_slot,
                    });
        }
        previous
    }

    pub fn native_boundaries(&self) -> &[NativeMarkerBoundary] {
        &self.native_boundaries
    }

    pub(crate) fn record_native_boundary(&mut self, override_issue: Option<NativeMarkerBoundary>) {
        if let Some(issue) = override_issue.or_else(|| self.native_issue.clone()) {
            if !self.native_boundaries.contains(&issue) {
                self.native_boundaries.push(issue);
            }
        }
    }

    pub(crate) fn end_child(&mut self, previous: Option<NativeModelFrame>) {
        self.native_frame = previous;
        self.native_issue = self.native_parent_issues.pop().flatten();
    }

    pub(crate) fn prepare_node_with_surface(
        &mut self,
        model: &ModelEntry,
        vars: &AnimVars,
        linked: Option<&LinkedModelSlots>,
        surface: NativeSlotSurface<'_>,
    ) {
        let slot = self.descriptor.raw_word_at_0x00;
        // This publication precedes the node stream. Plain coordinates have
        // no later register dependency; generated descriptor markers need
        // callback-time register/cache custody before live admission.
        let plain_marker = model
            .records
            .get(usize::from(slot >> 1))
            .is_some_and(|record| matches!(record[0], 0 | 4));
        self.marker_native_world = self
            .native_frame
            .as_ref()
            .filter(|_| plain_marker)
            .and_then(|frame| frame.resolve_model_slot_with_surface(model, vars, slot, surface))
            .zip(self.native_viewport)
            .map(|(point, viewport)| viewport.view_point_to_world(point));
        if self.native_requested && self.native_frame.is_some() {
            self.native_issue = self.marker_native_world.is_none().then_some(
                NativeMarkerBoundary::UnsupportedMarkerSlot {
                    model_id: model.index,
                    slot,
                    kind: model
                        .records
                        .get(usize::from(slot >> 1))
                        .map(|record| record[0]),
                },
            );
        }
        // Every applicable authored factory marker is a plain point. Retain
        // fallback for a marker whose callback policy has not been recovered.
        self.marker_local = (model
            .records
            .get(usize::from(slot >> 1))
            .is_some_and(|record| record[0] == 0))
        .then(|| {
            model.resolve_slot_with_context(
                slot,
                ModelMaterializationContext::intrinsic(vars, linked),
            )
        })
        .flatten()
        .map(|point| point.position_raw);
    }

    pub(crate) fn publish_native(&mut self, marker_world_raw: [i32; 3]) -> [f32; 3] {
        let marker_draw_world = std::array::from_fn(|axis| {
            self.root_draw_origin[axis]
                + marker_world_raw[axis].wrapping_sub(self.native_root_image[axis]) as f32 / 256.0
        });
        self.publish_raw(marker_world_raw.map(|value| value as i16));
        marker_draw_world
    }

    fn publish_raw(&mut self, marker_raw: [i16; 3]) {
        if self.product_id == 0 {
            return;
        }
        let tail = self.descriptor.raw_tail;
        let offset = [
            i16::from_le_bytes([tail[4], tail[5]]),
            i16::from_le_bytes([tail[6], tail[7]]),
            i16::from_le_bytes([tail[8], tail[9]]),
        ];
        *self.write = Some(SubMProductMarkerWrite {
            product_id: self.product_id,
            position_raw: std::array::from_fn(|axis| marker_raw[axis].wrapping_add(offset[axis])),
        });
    }

    pub(crate) fn publish(&mut self, marker_draw_world: [f32; 3]) {
        if self.product_id == 0 {
            return;
        }
        // Presentation may use a toroidal camera-relative image. Convert the
        // marker's displacement back to the actor's retained world origin.
        let marker_world = std::array::from_fn(|axis| {
            self.actor_origin_world[axis] + marker_draw_world[axis] - self.root_draw_origin[axis]
        });
        let marker_raw = world_position_raw(marker_world);
        self.publish_raw(marker_raw);
    }
}

impl EntityManager {
    /// Commit the draw callback's tracked-product suffix. The recent +60
    /// relation and cargo attachment are independent of Sub-M +88 authority.
    pub fn apply_sub_m_product_marker_write(&mut self, write: SubMProductMarkerWrite) {
        // 3A580 validates the retained handle, independently of simulation/
        // collision eligibility. 198E0 checks only that this lookup succeeds.
        if let Some(product) = self.entity_mut(write.product_id) {
            product.set_position_raw(write.position_raw);
            product.collision.state_flags_at_0x08.overwrite(0x20, 0x20);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_sub_m_does_not_accept_generated_marker_before_callback_register_custody() {
        let viewport = NativeWorldViewport {
            origin_raw: [0; 3],
            axes_q31: [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]],
            identity: true,
        };
        let model = ModelEntry {
            records: vec![[14, 0, 0, 0], [0, 0, 0, 0], [0, 0, 128, 0], [8, 197, 2, 4]],
            ..ModelEntry::default()
        };
        let descriptor = StatusComponentDescriptor {
            raw_word_at_0x00: 2,
            variable_bindings: [0; 6],
            raw_tail: [0; 10],
        };
        let vars = AnimVars::default();
        let mut write = None;
        let mut presentation =
            SubMPresentation::new(descriptor, 42, [0.0; 3], [0.0; 3], &mut write);
        // Controlled source frame words authenticate the arithmetic fixture;
        // this does not manufacture a live EntityManager allocation receipt.
        presentation.actor_native = Some(([0; 3], viewport.axes_q31));
        let mut presentation = presentation.with_native_viewport(viewport);
        assert_eq!(presentation.native_viewport, Some(viewport));
        assert!(presentation.native_frame.is_some());
        presentation.prepare_node_with_surface(&model, &vars, None, NativeSlotSurface::Intrinsic);
        assert_eq!(
            presentation.marker_native_world,
            Some([0; 3]),
            "the same owned frame admits a plain marker"
        );
        assert_eq!(presentation.native_issue, None);

        let body = presentation.native_frame.as_ref().unwrap();
        assert!(
            body.resolve_model_slot(&model, &vars, 6).is_some(),
            "generated attachments own their register snapshot"
        );
        presentation.descriptor.raw_word_at_0x00 = 6;
        presentation.prepare_node_with_surface(&model, &vars, None, NativeSlotSurface::Intrinsic);
        assert_eq!(
            presentation.marker_native_world, None,
            "a pre-stream marker must not consume the attachment snapshot"
        );
        presentation.record_native_boundary(None);
        assert_eq!(
            presentation.native_boundaries(),
            &[NativeMarkerBoundary::UnsupportedMarkerSlot {
                model_id: model.index,
                slot: 6,
                kind: Some(8),
            }]
        );
        drop(presentation);
        assert!(write.is_none());
    }
}
