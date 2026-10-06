//! Native Sub-M product marker: 0A9F0 -> 198E0 -> 199B0.
//!
//! Production births at the entity origin plus signed descriptor offsets and
//! records the product at Sub-M +88. A referenced tf14 selector zero then
//! resolves the descriptor's vertex slot in its current model context, converts
//! that point to world, and updates that tracked product (state bit20 included).
//! The marker can belong to a nested animated child; names are not authority.

use std::cell::RefCell;

use v2k_formats::models::{
    AnimVars, LinkedModelSlots, ModelEntry, ModelMaterializationContext, ModelVertexKind,
    ModelVertexResolver, ResolvedModelSlot,
};
use v2k_render::{mat3_mul, orientation_f32, orientation_from_ypr};

use crate::entity::{world_position_raw, Entity, EntityManager};
use crate::entity_collision_state::RetailRuntimeValue;
use crate::model_tree::{
    build_child_linked_slots, inherited_child_vars, instance_world_position,
    ModelTreeRootLinkPolicy,
};
use crate::resource_cache::ResourceCache;

const GAMEPLAY_MODEL_SCALE: f32 = 100.0 / 256.0;
const MODEL_TREE_DEPTH: u8 = 8;

/// Resolve the last executed Sub-M marker callback in authored traversal order.
/// A model without a referenced selector-zero tf14 does not publish a marker.
pub fn base_factory_product_marker_world(
    cache: &ResourceCache,
    entity: &Entity,
    retail_tick: u32,
) -> Option<[f32; 3]> {
    base_factory_product_marker_world_with_vars(
        cache,
        entity,
        &entity.presentation_anim_vars(retail_tick),
    )
}

pub fn base_factory_product_marker_world_with_vars(
    cache: &ResourceCache,
    entity: &Entity,
    vars: &AnimVars,
) -> Option<[f32; 3]> {
    let RetailRuntimeValue::Known(Some(base)) = entity.base_factory_runtime else {
        return None;
    };
    let model_id = entity.model_index.or(entity.model_slots[0])?;
    let orientation = match entity.physical_body_basis_q31() {
        RetailRuntimeValue::Known(basis) => basis.orientation_world_from_model(),
        RetailRuntimeValue::Unresolved => {
            orientation_from_ypr(std::f32::consts::FRAC_PI_2 - entity.heading, 0.0, 0.0)
        }
    };
    let mut traversal = MarkerTraversal {
        cache,
        marker_slot: base.status_descriptor.raw_word_at_0x00,
        last_world: None,
    };
    traversal.visit(
        model_id,
        orientation,
        entity.position,
        MODEL_TREE_DEPTH,
        None,
        vars,
    );
    traversal.last_world
}

struct MarkerTraversal<'a> {
    cache: &'a ResourceCache,
    marker_slot: u16,
    last_world: Option<[f32; 3]>,
}

/// The known factory corpus uses a plain point for the Sub-M marker. Keeping
/// this separate from tf14 resolution prevents callback recursion and refuses
/// an unowned view/external/generated marker instead of inventing a socket.
struct MarkerCallback<'a> {
    model: &'a ModelEntry,
    marker_slot: u16,
    vars: &'a AnimVars,
    linked: Option<&'a LinkedModelSlots>,
    calls: RefCell<Vec<[f64; 3]>>,
}

impl ModelVertexResolver for MarkerCallback<'_> {
    fn resolve_vertex_raw(
        &self,
        _kind: ModelVertexKind,
        _source: ResolvedModelSlot,
    ) -> Option<ResolvedModelSlot> {
        None
    }

    fn resolve_external_frame_raw(
        &self,
        _slot: u16,
        parameters: [i16; 3],
    ) -> Option<ResolvedModelSlot> {
        if parameters[0] != 0 || self.model.records.get(usize::from(self.marker_slot >> 1))?[0] != 0
        {
            return None;
        }
        let point = self.model.resolve_slot_with_context(
            self.marker_slot,
            ModelMaterializationContext::intrinsic(self.vars, self.linked),
        )?;
        self.calls.borrow_mut().push(point.position_raw);
        // 198E0 returns the marker through 199B0's output dwords as well as
        // writing the product. In this intrinsic pass the equivalent is local.
        Some(point)
    }
}

impl MarkerTraversal<'_> {
    fn visit(
        &mut self,
        model_id: usize,
        orientation: [[f32; 3]; 3],
        position: [f32; 3],
        depth: u8,
        linked: Option<&LinkedModelSlots>,
        vars: &AnimVars,
    ) {
        let Some(model) = self.cache.global_model(model_id) else {
            return;
        };
        let resolver = MarkerCallback {
            model,
            marker_slot: self.marker_slot,
            vars,
            linked,
            calls: RefCell::new(Vec::new()),
        };
        let materialized = model.materialize_with_context(ModelMaterializationContext {
            vars,
            linked,
            vertex_resolver: Some(&resolver),
            view_selection: v2k_formats::models::ModelViewSelection::IntrinsicAllBranches,
        });
        for point in resolver.calls.borrow_mut().drain(..) {
            self.last_world = Some(instance_world_position(
                orientation,
                position,
                point,
                GAMEPLAY_MODEL_SCALE,
            ));
        }
        if depth == 0 {
            return;
        }
        for instance in &materialized.instances {
            let child_id = usize::from(instance.model_id);
            if child_id == model_id {
                continue;
            }
            let child_orientation = mat3_mul(orientation, orientation_f32(instance.orientation));
            let child_position = instance_world_position(
                orientation,
                position,
                instance.attach_pos.unwrap_or([0.0; 3]),
                GAMEPLAY_MODEL_SCALE,
            );
            let child_linked = build_child_linked_slots(
                model,
                linked,
                instance,
                vars,
                ModelTreeRootLinkPolicy::Authored,
                Some(&resolver),
            );
            let child_vars = inherited_child_vars(vars, instance);
            // Parent imports may execute its tf14 even when no parent face
            // requested it. Preserve that side effect before entering the child.
            for point in resolver.calls.borrow_mut().drain(..) {
                self.last_world = Some(instance_world_position(
                    orientation,
                    position,
                    point,
                    GAMEPLAY_MODEL_SCALE,
                ));
            }
            self.visit(
                child_id,
                child_orientation,
                child_position,
                depth - 1,
                Some(&child_linked),
                &child_vars,
            );
        }
    }
}

impl EntityManager {
    /// Apply 198E0's pose suffix to each live Sub-M +88 product. The +60 recent
    /// relation is not an attachment list and is not the selection authority.
    pub fn follow_base_factory_products_to_marker(
        &mut self,
        cache: &ResourceCache,
        retail_tick: u32,
    ) {
        let mut updates = Vec::new();
        for parent in self.iter_all().filter(|entity| entity.active) {
            let RetailRuntimeValue::Known(Some(base)) = parent.base_factory_runtime else {
                continue;
            };
            let Some(production) = base.production else {
                continue;
            };
            let product_id = production.spawned_pickup_handle;
            if product_id == 0
                || !self
                    .iter_all()
                    .any(|entity| entity.active && entity.id == product_id)
            {
                continue;
            }
            let Some(world) = base_factory_product_marker_world(cache, parent, retail_tick) else {
                continue;
            };
            let tail = base.status_descriptor.raw_tail;
            let offset = [
                i16::from_le_bytes([tail[4], tail[5]]),
                i16::from_le_bytes([tail[6], tail[7]]),
                i16::from_le_bytes([tail[8], tail[9]]),
            ];
            let marker_raw = world_position_raw(world);
            let product_raw =
                std::array::from_fn(|axis| marker_raw[axis].wrapping_add(offset[axis]));
            updates.push((product_id, product_raw));
        }
        for (id, position) in updates {
            if let Some(product) = self.entity_mut(id) {
                product.set_position_raw(position);
                product.collision.state_flags_at_0x08.overwrite(0x20, 0x20);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        entity::EntityManager, entity_collision_state::EntityTypeRuntimeMetadata,
        session::GameSession,
    };

    fn fixture(world: u32) -> Option<(GameSession, EntityManager)> {
        let dir = v2k_test_support::retail_dir();
        assert!(dir.join("PRELOAD.DAT").is_file(), "retail corpus required");
        let mut session = GameSession::init(&dir).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        let metadata: Vec<_> = session
            .cache
            .global_entity_model_table()
            .iter()
            .enumerate()
            .map(|(ty, slots)| {
                session
                    .cache
                    .global_entity_type(ty)
                    .map(EntityTypeRuntimeMetadata::from_section12)
                    .unwrap_or(EntityTypeRuntimeMetadata {
                        model_slots: *slots,
                        ..Default::default()
                    })
            })
            .collect();
        session.load_level_by_id(world, 1).unwrap();
        let manager = EntityManager::from_level_with_type_metadata(
            session.cache.level_desc().unwrap(),
            &metadata,
            session.cache.terrain(),
        );
        Some((session, manager))
    }

    #[v2k_test_support::retail_test]
    fn native_marker_includes_each_nested_lift_authored_point() {
        let Some((session, mut manager)) = fixture(13) else {
            return;
        };
        let factory_id = manager.iter_all().find(|e| e.entity_type == 66).unwrap().id;
        let factory = manager.entity_mut(factory_id).unwrap();
        factory.position = [0.0; 3];
        factory.heading = std::f32::consts::FRAC_PI_2;
        factory.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
        let vars = AnimVars::default();
        for (model, marker_y) in [(222, 128), (229, 78), (220, 60), (217, 102), (205, 60)] {
            factory.model_index = Some(model);
            let world =
                base_factory_product_marker_world_with_vars(&session.cache, &factory, &vars)
                    .unwrap();
            assert_eq!(
                world_position_raw(world)[1],
                marker_y,
                "model{model}: {world:?}"
            );
        }
    }

    #[v2k_test_support::retail_test]
    fn level_two_tracked_product_moves_to_marker_then_adds_world_signed_offsets() {
        let Some((session, mut manager)) = fixture(14) else {
            return;
        };
        let product_id = manager.iter_all().find(|e| e.entity_type == 61).unwrap().id;
        let factory_ids: Vec<_> = manager
            .iter_all()
            .filter(|e| e.entity_type == 66)
            .map(|e| e.id)
            .collect();
        assert!(!factory_ids.is_empty());
        for id in factory_ids {
            let before = manager
                .iter_all()
                .find(|e| e.id == product_id)
                .unwrap()
                .position_raw();
            let (model, marker) = {
                let factory = manager.entity_mut(id).unwrap();
                let RetailRuntimeValue::Known(Some(base)) = &mut factory.base_factory_runtime
                else {
                    panic!("Sub-M");
                };
                base.production.as_mut().unwrap().spawned_pickup_handle = product_id;
                // These are descriptor world offsets; they are not rotated with
                // the building or the child model frame (41995A..419990).
                for (i, value) in [31i16, -17, 23].into_iter().enumerate() {
                    base.status_descriptor.raw_tail[4 + i * 2..6 + i * 2]
                        .copy_from_slice(&value.to_le_bytes());
                }
                (
                    factory.model_index.unwrap(),
                    base_factory_product_marker_world(&session.cache, factory, 0).unwrap(),
                )
            };
            // Product +60 is irrelevant: 198E0 uses this factory's tracked +88.
            manager
                .entity_mut(product_id)
                .unwrap()
                .collision
                .recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
            manager.follow_base_factory_products_to_marker(&session.cache, 0);
            let product = manager.iter_all().find(|e| e.id == product_id).unwrap();
            let raw = world_position_raw(marker);
            assert_eq!(
                product.position_raw(),
                [
                    raw[0].wrapping_add(31),
                    raw[1].wrapping_sub(17),
                    raw[2].wrapping_add(23)
                ]
            );
            assert_eq!(
                product.collision.state_flags_at_0x08.masked(0x20),
                RetailRuntimeValue::Known(0x20)
            );
            assert!(product.attached_to.is_none());
            assert_ne!(product.position_raw(), before);
            eprintln!(
                "LEVEL2 factorymodel={model} before={before:?} marker={raw:?} after={:?}",
                product.position_raw()
            );
            let RetailRuntimeValue::Known(Some(base)) =
                &mut manager.entity_mut(id).unwrap().base_factory_runtime
            else {
                unreachable!();
            };
            base.production.as_mut().unwrap().spawned_pickup_handle = 0;
        }
    }

    #[v2k_test_support::retail_test]
    fn draw_marker_suffix_resolves_allocated_product_without_an_active_state_gate() {
        let Some((_, mut manager)) = fixture(14) else {
            return;
        };
        let product_id = manager
            .iter_all()
            .find(|entity| entity.entity_type == 61)
            .unwrap()
            .id;
        let before_velocity = manager.entity_mut(product_id).unwrap().velocity_raw();
        let product = manager.entity_mut(product_id).unwrap();
        product.active = false;
        product.collision.state_flags_at_0x08 =
            crate::entity_collision_state::RetailStateWord::exact(0x400);
        let position_raw = [15_657, -846, -32_297];
        manager.apply_sub_m_product_marker_write(
            crate::sub_m_external_frame::SubMProductMarkerWrite {
                product_id,
                position_raw,
            },
        );
        let product = manager.entity_mut(product_id).unwrap();
        assert_eq!(product.position_raw(), position_raw);
        assert_eq!(product.velocity_raw(), before_velocity);
        assert_eq!(
            product.collision.state_flags_at_0x08,
            crate::entity_collision_state::RetailStateWord::exact(0x420)
        );
        assert!(
            !product.active,
            "the marker suffix does not activate simulation eligibility"
        );
        manager.apply_sub_m_product_marker_write(
            crate::sub_m_external_frame::SubMProductMarkerWrite {
                product_id: u32::MAX,
                position_raw: [0; 3],
            },
        );
        assert_eq!(
            manager.entity_mut(product_id).unwrap().position_raw(),
            position_raw
        );
    }

    #[v2k_test_support::retail_test]
    fn marker_rule_follows_authored_factory_models_across_worlds_and_base_is_quiet() {
        let Some((mut session, _)) = fixture(13) else {
            return;
        };
        let metadata: Vec<_> = session
            .cache
            .global_entity_model_table()
            .iter()
            .enumerate()
            .map(|(ty, slots)| {
                session
                    .cache
                    .global_entity_type(ty)
                    .map(EntityTypeRuntimeMetadata::from_section12)
                    .unwrap_or(EntityTypeRuntimeMetadata {
                        model_slots: *slots,
                        ..Default::default()
                    })
            })
            .collect();
        let mut models = std::collections::BTreeSet::new();
        let mut marker_models = std::collections::BTreeSet::new();
        let mut bases = 0;
        for world in 13..=49 {
            session.load_level_by_id(world, 1).unwrap();
            let manager = EntityManager::from_level_with_type_metadata(
                session.cache.level_desc().unwrap(),
                &metadata,
                session.cache.terrain(),
            );
            for entity in manager
                .iter_all()
                .filter(|e| matches!(e.entity_type, 6 | 66))
            {
                let marker = base_factory_product_marker_world(&session.cache, entity, 0);
                if entity.entity_type == 6 {
                    bases += 1;
                    assert!(
                        marker.is_none(),
                        "base world{world} model{:?} unexpectedly invokes Sub-M marker",
                        entity.model_index
                    );
                } else {
                    let model = entity.model_index.unwrap();
                    models.insert(model);
                    if marker.is_some() {
                        marker_models.insert(model);
                    }
                }
            }
        }
        eprintln!("factorymodels={models:?}; marker models={marker_models:?}; bases={bases}");
        assert_eq!(bases, 28);
        assert!(marker_models.contains(&218), "Level2 factory3 is covered");
        assert!(marker_models.contains(&227), "lifter is covered");
        assert!(
            marker_models.len() >= 8,
            "actual marker models: {marker_models:?}"
        );
        eprintln!("factorymodels={models:?}; marker models={marker_models:?}; bases={bases}");
    }
}
