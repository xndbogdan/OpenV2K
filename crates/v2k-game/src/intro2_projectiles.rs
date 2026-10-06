//! `FUN_00411400` source-FIFO drain during actor presentation.
//!
//! 4942E0 runs the simulation chain before the drawing chain. In normal
//! Intro2 and Playing, 44FFA0 has already completed physical particles and contacts when
//! 53760 -> 11720 reaches this drain. New shots first move on the next update.
//! The final black card submits Klaus alone and does not visit these sources.
//!
//! Every source retains a FIFO through task replacement. Drain sources in
//! manager order, including newly selected emitter families, so allocation
//! and pool saturation do not depend on which Rust adapter owns the actor.

use crate::entity::EntityManager;
use crate::intro2_flyer_aim::{drain_intro2_flyer_shots, Intro2FlyerShotDrainError};
use crate::intro2_type13_aim::{drain_intro2_type13_shots, Type13ShotDrainError};
use crate::ordinary_type47_live::{
    drain_fresh_level1_ordinary_type47_shots, OrdinaryType47LiveError,
};
use crate::world_fx::{ParticleEnvironment, WorldFx};

#[derive(Debug)]
pub enum Intro2ShotDrainError {
    GunTurret(crate::intro2_gun_turret::aim::Intro2GunTurretShotDrainError),
    Type10(crate::intro2_type10::aim::Intro2Type10ShotDrainError),
    Type57(crate::intro2_type57::aim::Intro2Type57ShotDrainError),
    Ptersect(Type13ShotDrainError),
    Ant(OrdinaryType47LiveError),
    Flyer(Intro2FlyerShotDrainError),
    Type16(crate::intro2_type16::aim::Intro2Type16ShotDrainError),
    Type58(crate::intro2_type58::aim::Intro2Type58ShotDrainError),
    Type122(crate::native_type122::aim::Type122ShotDrainError),
    Type56(crate::native_type56::aim::Type56ShotDrainError),
    Type30(crate::native_type30::aim::Type30ShotDrainError),
    Type40(crate::native_type40::aim::Type40ShotDrainError),
    Type94(crate::intro2_type94::aim::Intro2Type94ShotDrainError),
}

#[derive(Clone, Copy)]
enum EmitterFamily {
    GunTurret,
    Type10,
    Type57,
    Ptersect,
    Ant,
    Flyer,
    Type16,
    Type58,
    Type122,
    Type56,
    Type30,
    Type40,
    Type94,
}

pub fn drain_intro2_projectiles(
    entities: &mut EntityManager,
    world_fx: &mut WorldFx,
    resources: &crate::resource_cache::ResourceCache,
    environment: ParticleEnvironment<'_>,
    retail_tick: u32,
) -> Vec<(u32, Result<usize, Intro2ShotDrainError>)> {
    let sources = entities
        .iter_all()
        .filter_map(|entity| emitter_family(entity).map(|family| (entity.id, family)))
        .collect::<Vec<_>>();
    sources
        .into_iter()
        .map(|(id, family)| {
            (
                id,
                drain_emitter_family(
                    entities,
                    world_fx,
                    resources,
                    environment,
                    retail_tick,
                    id,
                    family,
                ),
            )
        })
        .collect()
}

/// Drain one actor immediately after its model callback visitation (11400).
/// An offscreen source still drains its unstamped FIFO in manager order.
pub fn drain_intro2_projectile_source(
    entities: &mut EntityManager,
    world_fx: &mut WorldFx,
    resources: &crate::resource_cache::ResourceCache,
    environment: ParticleEnvironment<'_>,
    retail_tick: u32,
    id: u32,
) -> Option<Result<usize, Intro2ShotDrainError>> {
    let family = entities
        .iter_all()
        .find(|entity| entity.id == id)
        .and_then(emitter_family)?;
    Some(drain_emitter_family(
        entities,
        world_fx,
        resources,
        environment,
        retail_tick,
        id,
        family,
    ))
}

fn emitter_family(entity: &crate::entity::Entity) -> Option<EmitterFamily> {
    let family = if entity
        .intro2_gun_turret_aim_runtime
        .as_ref()
        .is_some_and(|runtime| runtime.queued_shot_count() > 0)
    {
        EmitterFamily::GunTurret
    } else if entity
        .intro2_type13_aim_runtime
        .as_ref()
        .is_some_and(|runtime| runtime.queued_shot_count() > 0)
    {
        EmitterFamily::Ptersect
    } else if entity
        .ordinary_type47_aim_and_fire_runtime
        .as_ref()
        .is_some_and(|runtime| runtime.queued_shot_count() > 0)
    {
        EmitterFamily::Ant
    } else if entity
        .intro2_flyer_aim_runtime
        .as_ref()
        .is_some_and(|runtime| runtime.queued_shot_count() > 0)
    {
        EmitterFamily::Flyer
    } else if entity
        .intro2_type10_aim_runtime
        .as_ref()
        .is_some_and(|runtime| runtime.queued_shot_count() > 0)
    {
        EmitterFamily::Type10
    } else if entity
        .intro2_type57_aim_runtime
        .as_ref()
        .is_some_and(|runtime| runtime.queued_shot_count() > 0)
    {
        EmitterFamily::Type57
    } else if entity
        .intro2_type16_aim_runtime
        .as_ref()
        .is_some_and(|runtime| runtime.queued_shot_count() > 0)
    {
        EmitterFamily::Type16
    } else if entity
        .intro2_type58_aim_runtime
        .as_ref()
        .is_some_and(|runtime| runtime.queued_shot_count() > 0)
    {
        EmitterFamily::Type58
    } else if entity
        .native_type122_aim_runtime
        .as_ref()
        .is_some_and(|runtime| runtime.queued_shot_count() > 0)
    {
        EmitterFamily::Type122
    } else if entity
        .native_type56_aim_runtime
        .as_ref()
        .is_some_and(|runtime| runtime.queued_shot_count() > 0)
    {
        EmitterFamily::Type56
    } else if entity
        .native_type30_aim_runtime
        .as_ref()
        .is_some_and(|runtime| runtime.queued_shot_count() > 0)
    {
        EmitterFamily::Type30
    } else if entity
        .native_type40_aim_runtime
        .as_ref()
        .is_some_and(|runtime| runtime.queued_shot_count() > 0)
    {
        EmitterFamily::Type40
    } else if entity
        .intro2_type94_aim_runtime
        .as_ref()
        .is_some_and(|runtime| runtime.queued_shot_count() > 0)
    {
        EmitterFamily::Type94
    } else {
        return None;
    };
    Some(family)
}

fn drain_emitter_family(
    entities: &mut EntityManager,
    world_fx: &mut WorldFx,
    resources: &crate::resource_cache::ResourceCache,
    environment: ParticleEnvironment<'_>,
    retail_tick: u32,
    id: u32,
    family: EmitterFamily,
) -> Result<usize, Intro2ShotDrainError> {
    match family {
        EmitterFamily::GunTurret => crate::intro2_gun_turret::aim::drain_intro2_gun_turret_shots(
            entities,
            world_fx,
            id,
            resources,
            environment,
            retail_tick,
        )
        .map(|outcome| outcome.consumed_requests)
        .map_err(Intro2ShotDrainError::GunTurret),
        EmitterFamily::Type10 => crate::intro2_type10::aim::drain_intro2_type10_shots(
            entities,
            world_fx,
            id,
            environment,
            retail_tick,
        )
        .map(|outcome| outcome.consumed_requests)
        .map_err(Intro2ShotDrainError::Type10),
        EmitterFamily::Type57 => crate::intro2_type57::aim::drain_intro2_type57_shots(
            entities,
            world_fx,
            id,
            environment,
            retail_tick,
        )
        .map(|outcome| outcome.consumed_requests)
        .map_err(Intro2ShotDrainError::Type57),
        EmitterFamily::Ptersect => {
            drain_intro2_type13_shots(entities, world_fx, id, environment, retail_tick)
                .map(|outcome| outcome.consumed_requests)
                .map_err(Intro2ShotDrainError::Ptersect)
        }
        EmitterFamily::Ant => drain_fresh_level1_ordinary_type47_shots(
            entities,
            world_fx,
            id,
            environment,
            retail_tick,
        )
        .map(|outcome| outcome.consumed_requests)
        .map_err(Intro2ShotDrainError::Ant),
        EmitterFamily::Type16 => crate::intro2_type16::aim::drain_intro2_type16_shots(
            entities,
            world_fx,
            id,
            environment,
            retail_tick,
        )
        .map(|outcome| outcome.consumed_requests)
        .map_err(Intro2ShotDrainError::Type16),
        EmitterFamily::Type58 => crate::intro2_type58::aim::drain_intro2_type58_shots(
            entities,
            world_fx,
            id,
            environment,
            retail_tick,
        )
        .map(|outcome| outcome.consumed_requests)
        .map_err(Intro2ShotDrainError::Type58),
        EmitterFamily::Type122 => crate::native_type122::aim::drain_type122_shots(
            entities,
            world_fx,
            id,
            environment,
            retail_tick,
        )
        .map(|outcome| outcome.consumed_requests)
        .map_err(Intro2ShotDrainError::Type122),
        EmitterFamily::Type56 => crate::native_type56::aim::drain_type56_shots(
            entities,
            world_fx,
            id,
            environment,
            retail_tick,
        )
        .map(|outcome| outcome.consumed_requests)
        .map_err(Intro2ShotDrainError::Type56),
        EmitterFamily::Type30 => crate::native_type30::aim::drain_type30_shots(
            entities,
            world_fx,
            id,
            environment,
            retail_tick,
        )
        .map(|outcome| outcome.consumed_requests)
        .map_err(Intro2ShotDrainError::Type30),
        EmitterFamily::Type40 => crate::native_type40::aim::drain_type40_shots(
            entities,
            world_fx,
            id,
            environment,
            retail_tick,
        )
        .map(|outcome| outcome.consumed_requests)
        .map_err(Intro2ShotDrainError::Type40),
        EmitterFamily::Type94 => crate::intro2_type94::aim::drain_intro2_type94_shots(
            entities,
            world_fx,
            id,
            environment,
            retail_tick,
        )
        .map(|outcome| outcome.consumed_requests)
        .map_err(Intro2ShotDrainError::Type94),
        EmitterFamily::Flyer => {
            drain_intro2_flyer_shots(entities, world_fx, id, environment, retail_tick)
                .map(|outcome| outcome.consumed_requests)
                .map_err(Intro2ShotDrainError::Flyer)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        common_mover::{
            component_dispatch::CommonMoverDispatchMode, type9_attitude::Type9BodyBasis,
        },
        entity_collision_state::RetailRuntimeValue,
        native_ground_actor, native_type56,
        split_and_explode::SplitChildRequest,
    };

    fn queue_native_type56_shots(
        entities: &mut EntityManager,
        resources: &crate::resource_cache::ResourceCache,
        fx: &mut WorldFx,
    ) -> (u32, usize) {
        let mut selected = None;
        for _ in 0..128 {
            let birth = SplitChildRequest {
                requested_entity_handle_raw: 0,
                entity_type: 56,
                position_raw: entities.player().unwrap().position_raw(),
                objective: true,
                velocity_raw: [-2000, -3000, 2000],
                rotation_heading_pitch_roll_raw: [0x9234, 0x8123, 0x7654],
            };
            let publication = entities
                .construct_native_type56(birth, resources, fx, 4793)
                .unwrap();
            if publication.publication.selection.program.class_id == 7 {
                selected = Some(publication.owner.entity_id());
                break;
            }
        }
        let id = selected.expect("genuine selector reaches class7");
        let target = entities.player().unwrap().id;
        let player = entities.entity_mut(target).unwrap();
        player.set_position_raw([0, 1000, 1000]);
        player.set_velocity_raw([0; 3]);
        player.collision.state_flags_at_0x08.overwrite(u32::MAX, 4);
        let entity = entities.entity_mut(id).unwrap();
        entity.set_position_raw([0, 1000, 0]);
        entity.set_velocity_raw([100, 0, -200]);
        entity.set_rotation_heading_pitch_roll_raw([0x4000, 0, 0]);
        entity.physical_body_basis_q31 =
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(0x4000, 0, 0));
        entity.collision.state_flags_at_0x08.overwrite(u32::MAX, 4);
        native_ground_actor::behavior::secondary::<native_type56::profile::Type56Profile>(
            entities, id, 20_000, 4794, fx,
        )
        .unwrap();
        let metadata = entities.type_runtime_metadata(56).unwrap().clone();
        // Exercise real cadence using the process RNG. No captured word or
        // fabricated queue is installed to force a shot.
        for _ in 0..32 {
            let outcome = native_type56::aim::tick_type56_aim(
                CommonMoverDispatchMode::Normal,
                entities,
                fx,
                id,
                125_000,
                Some(&metadata),
            )
            .unwrap();
            if outcome.queued_shots_added > 0 {
                let count = entities
                    .entity_mut(id)
                    .unwrap()
                    .native_type56_aim_runtime
                    .as_ref()
                    .unwrap()
                    .queued_shot_count();
                return (id, count);
            }
        }
        panic!("native cadence did not append a transient");
    }

    #[v2k_test_support::retail_test]
    fn production_source_and_bulk_drains_consume_native_type56_fifo_after_quiet_death() {
        let (session, mut entities, mut fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(19);
        let (id, expected) = queue_native_type56_shots(&mut entities, &session.cache, &mut fx);
        native_type56::death::begin_type56_standard_death(&mut entities, id, &mut fx).unwrap();
        let before = fx.particle_count();
        let result = drain_intro2_projectile_source(
            &mut entities,
            &mut fx,
            &session.cache,
            ParticleEnvironment::Dry,
            4794,
            id,
        )
        .expect("retained Type56 FIFO is a production source")
        .unwrap();
        assert_eq!(result, expected);
        let particles = fx.test_particles_in_virgin_birth_order();
        assert_eq!(particles.len() - before, expected);
        assert!(particles[before..].iter().all(|particle| {
            particle.source_class == 87
                && particle.owner_id == Some(id)
                && particle.source_entity_type_at_birth == Some(56)
                && particle.age_ticks == 0.0
        }));
        assert!(drain_intro2_projectile_source(
            &mut entities,
            &mut fx,
            &session.cache,
            ParticleEnvironment::Dry,
            4794,
            id,
        )
        .is_none());

        let (next, queued) = queue_native_type56_shots(&mut entities, &session.cache, &mut fx);
        let results = drain_intro2_projectiles(
            &mut entities,
            &mut fx,
            &session.cache,
            ParticleEnvironment::Dry,
            4794,
        );
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, next);
        assert_eq!(*results[0].1.as_ref().unwrap(), queued);
        assert!(drain_intro2_projectiles(
            &mut entities,
            &mut fx,
            &session.cache,
            ParticleEnvironment::Dry,
            4794,
        )
        .is_empty());
    }
}
