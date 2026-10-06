//!33BD0/16FF0 terrain helpers, followed by42EFB0's ordered deferred cleanup.
//!
//! Routing remains a static-model contact policy. These Type111 bodies own
//! model16, class0 custody and constructor sound100 independently of routes.

use super::class0_actor::{construct_class0_actor, Class0ActorConstruction, Class0SpawnInput};
use super::*;
use crate::campaign_transition::collect_authored_warp_markers;

impl EntityManager {
    pub(super) fn construct_authored_gate_helpers(
        &mut self,
        resources: EntityConstructionResources<'_>,
        retail_tick: u32,
        exit_marker_bits: u32,
        world_fx: &mut WorldFx,
    ) -> Result<(), String> {
        let (Some(terrain), Some(objects)) = (resources.terrain, resources.terrain_objects) else {
            return Ok(());
        };
        let markers = collect_authored_warp_markers(terrain, objects)
            .map_err(|error| format!("terrain marker catalog: {error:?}"))?;
        if markers.is_empty() {
            return Ok(());
        }
        let metadata = self
            .type_metadata
            .get(111)
            .ok_or("Type111 metadata")?
            .clone();
        for marker in markers {
            let id = self.next_entity_id;
            let stamp = self.begin_common_body_attempt();
            let mut entity = native_instance_body(
                id,
                stamp,
                &metadata,
                terrain,
                NativeInstanceBodyRequest::zeroed(111),
            );
            entity.set_position_raw(marker.position_raw);
            let allocation =
                observe_main_base_abort_actor(&entity, self.allocation_generation).lease;
            let surface = crate::entity_initializer::constructor_surface_bits_at_tick(
                marker.position_raw,
                terrain,
                retail_tick,
                self.environment_physics.waves_enabled,
            )
            .ok_or("Type111 constructor surface")?;
            let mut runtime = construct_class0_actor(
                &mut entity,
                Class0ActorConstruction {
                    allocation,
                    metadata: &metadata,
                    spawn: Class0SpawnInput::AtPose {
                        entity_type: 111,
                        position_raw: marker.position_raw,
                        rotation_raw: [0; 3],
                        velocity_raw: [0; 3],
                    },
                    resources,
                    constructor_surface_bits: surface,
                },
                &mut || u32::from(world_fx.next_shared_retail_random_u16()),
            )
            .map_err(|error| format!("Type111 {}:{}: {error:?}", marker.cell[0], marker.cell[1]))?;
            //16FF0 applies these writes after104B0/D4A0/C490 returns.
            if marker.terrain_type & 8 == 0 {
                entity.collision.state_flags_at_0x08.overwrite(0x800, 0);
            }
            if exit_marker_bits & (0x10 << (marker.subtype - 1)) != 0 {
                entity.capability_flags |= 0x8000;
            } else {
                entity
                    .collision
                    .state_flags_at_0x08
                    .overwrite(0x1000, 0x1000);
                //+80=self is a Type111 scheduler sentinel, not SubJ cargo.
                runtime.gate_self_relation = Some(id);
            }
            entity.collision.constructor_sound_follow_position_raw =
                RetailRuntimeValue::Known(marker.position_raw);
            entity.collision.constructor_sound_follow_published = true;
            assert_eq!(self.allocate_live_entity_id(), id);
            self.append_live_entity(entity);
            self.native_class0_actors.insert(id, runtime);
        }
        Ok(())
    }

    ///42EFB0 marks helpers with10B70; no allocation/RNG word is refunded.
    /// Marked rows remain in the list until the ordinary14990 sweep.
    pub(super) fn cleanup_authored_gate_helpers(&mut self) {
        let mut pending = Vec::new();
        for (index, current) in self.entities.iter().enumerate() {
            if current.entity_type == 111 && gate_cleanup_alive(current) {
                if self.entities[index + 1..].iter().any(|later| {
                    (later.entity_type == 67
                        || (later.entity_type == 111 && gate_cleanup_alive(later)))
                        && helpers_overlap(
                            current.position_raw(),
                            later.position_raw(),
                            later.entity_type == 67,
                        )
                }) {
                    pending.push(current.id);
                }
            } else if current.entity_type == 67 {
                pending.extend(
                    self.entities[index + 1..]
                        .iter()
                        .filter(|later| {
                            later.entity_type == 111
                                && helpers_overlap(
                                    current.position_raw(),
                                    later.position_raw(),
                                    true,
                                )
                        })
                        .map(|entity| entity.id),
                );
            }
        }
        for id in pending {
            self.entity_mut(id)
                .expect("cleanup live row")
                .mark_actor_deferred_destroy_pending();
            self.queue_actor_deferred_destroy(id);
        }
    }

    pub(crate) fn native_gate_helper_allocation_authenticates(&self, id: u32) -> bool {
        self.native_class0_allocation_authenticates(id)
            && self
                .entities
                .iter()
                .any(|entity| entity.id == id && entity.entity_type == 111)
    }
}

fn gate_cleanup_alive(entity: &Entity) -> bool {
    entity.active
        && entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT)
            == RetailRuntimeValue::Known(0)
}

fn helpers_overlap(a: [i16; 3], b: [i16; 3], hive: bool) -> bool {
    let delta = std::array::from_fn::<_, 3, _>(|axis| i32::from(a[axis].wrapping_sub(b[axis])));
    let absolute = delta.map(i32::abs);
    let largest = *absolute.iter().max().unwrap();
    let approximate = largest + ((absolute.iter().sum::<i32>() - largest) >> 1);
    let horizontal = (delta[0] * delta[0]).wrapping_add(delta[2] * delta[2]);
    approximate < 8000 && horizontal < if hive { 0x100000 } else { 0x40000 }
}

#[cfg(test)]
mod tests;
