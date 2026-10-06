use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    common_mover::component_dispatch::CommonMoverDispatchMode,
    common_mover::type9_attitude::Type9BodyBasis,
    entity_behavior::{
        audited_behavior_style, behavior_program, BehaviorChoiceListSource, BehaviorContextRuntime,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    generic_projectile_emitter::GenericEmitterSpeedField,
    intro2_type47_live::world::native_intro2_fixture,
    intro2_type57::{
        intro2_type57_allocation_authenticates, tick_intro2_type57, Intro2Type57Frame,
        Intro2Type57Outcome, Intro2Type57Owner,
    },
    search_attack_live::apply_search_attack_ade0_without_mover,
    session::GameSession,
    world_fx::WorldFx,
};

fn prepare(
    session: &GameSession,
    manager: &mut crate::entity::EntityManager,
    fx: &mut WorldFx,
) -> (u32, u32, EntityTypeRuntimeMetadata) {
    let id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(1))
        .unwrap()
        .id;
    let target_id = manager.iter_all().find(|e| e.entity_type == 47).unwrap().id;
    let entity = manager.entity_mut(id).unwrap();
    let metadata = EntityTypeRuntimeMetadata::from_section12(
        session
            .cache
            .global_entity_type(entity.entity_type as usize)
            .unwrap(),
    );
    assert!(intro2_type57_allocation_authenticates(entity));
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(
        BehaviorContextRuntime::named_audited(
            behavior_program(7).unwrap(),
            1,
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            RetailRuntimeValue::Known(Some(target_id)),
            RetailRuntimeValue::Known(0),
            *audited_behavior_style(7, 1).unwrap(),
        )
        .unwrap(),
    ));
    entity.set_position_raw([0, 1000, 0]);
    entity.set_velocity_raw([500, 0, 100]);
    entity.set_rotation_heading_pitch_roll_raw([0x4000, 0, 0]);
    entity.physical_body_basis_q31 =
        RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(0x4000, 0, 0));
    entity.collision.state_flags_at_0x08.overwrite(u32::MAX, 4);
    apply_search_attack_ade0_without_mover(entity, &metadata, target_id, fx).unwrap();
    let target = manager.entity_mut(target_id).unwrap();
    target.set_position_raw([0, 1000, 1000]);
    target.set_velocity_raw([0; 3]);
    target.collision.state_flags_at_0x08.overwrite(u32::MAX, 4);
    (id, target_id, metadata)
}

fn warm(fx: &mut WorldFx) {
    for _ in 0..8 {
        fx.advance_frame_pacing(20_000);
    }
}

#[v2k_test_support::retail_test]
fn native_type57_dormant_callback_clears_b2_without_translation() {
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    manager.disable_authored_behavior_components();
    let id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(1))
        .unwrap()
        .id;
    let owner = Intro2Type57Owner::adopt(&manager, id).unwrap();
    let position = manager.entity_mut(id).unwrap().position_raw();
    let tick = tick_intro2_type57(
        &mut manager,
        owner,
        Intro2Type57Frame {
            resources: &mut session.cache,
            world_fx: &mut WorldFx::new(),
            elapsed_micros: 20_000,
            retail_tick: 1,
        },
    );
    assert!(matches!(
        tick.outcome,
        Intro2Type57Outcome::Advanced {
            callback_enabled: false,
            ..
        }
    ));
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.position_raw(), position);
    assert_eq!(
        entity.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(0)
    );
}

#[v2k_test_support::retail_test]
fn native_type57_method24_queues_sound93_and_drains_particle68() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut fx = WorldFx::new();
    warm(&mut fx);
    let (id, _, metadata) = prepare(&session, &mut manager, &mut fx);
    let result = tick_intro2_type57_aim(
        CommonMoverDispatchMode::Normal,
        &mut manager,
        &mut fx,
        id,
        600_000,
        Some(&metadata),
    )
    .unwrap();
    assert!(result.queued_shots_added >= 1);
    fx.process_pending();
    let sounds = fx.take_positional_sounds();
    assert!(
        sounds
            .iter()
            .any(|sound| sound.sound_id == 93 && sound.frequency_q16 == 0x10000),
        "{sounds:?}"
    );
    let entity = manager.entity_mut(id).unwrap();
    let shots = entity
        .intro2_type57_aim_runtime
        .as_ref()
        .unwrap()
        .transient_shots()
        .to_vec();
    assert!(shots.iter().all(|shot| shot.source_handle == id
        && shot.owner_handle == id
        && shot.projectile_method == 24
        && shot.speed_field == GenericEmitterSpeedField::Explicit(2000)));
    entity.actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
    entity.actor_tasks.clear_slot(ActorTaskSlot::Primary);
    let result =
        drain_intro2_type57_shots(&mut manager, &mut fx, id, ParticleEnvironment::Dry, 30).unwrap();
    assert_eq!(result.consumed_requests, shots.len());
    assert!(result
        .materialized_particle_classes
        .iter()
        .all(|&class| class == 68));
    assert!(!result.materialized_particle_classes.is_empty());
}

#[v2k_test_support::retail_test]
fn native_type57_aim_requires_own_class7_graph() {
    let Some((_, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(1))
        .unwrap()
        .id;
    let Some(ActorTaskRuntime::SharedRetarget(_)) = manager
        .entity_mut(id)
        .unwrap()
        .actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("acquiring graph");
    };
    let err = tick_intro2_type57_aim(
        CommonMoverDispatchMode::Normal,
        &mut manager,
        &mut WorldFx::new(),
        id,
        20_000,
        None,
    );
    assert!(err.is_err(), "{err:?}");
}
