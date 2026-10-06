use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    entity::EntityConstructionResources,
    entity_collision_state::{EntityTypeRuntimeMetadata, DYING_STATE_BIT},
    entity_scheduler::{
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT, SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    },
    gameplay_notifications::GameplayNotifications,
    session::GameSession,
    world_fx::WorldFx,
};

fn session() -> Option<GameSession> {
    let directory = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&directory).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    Some(session)
}

fn generic(session: &GameSession) -> (EntityManager, Vec<EntityTypeRuntimeMetadata>) {
    let metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(kind, _)| {
            EntityTypeRuntimeMetadata::from_section12(
                session.cache.global_entity_type(kind).unwrap(),
            )
        })
        .collect();
    (
        EntityManager::from_level_with_type_metadata(
            session.cache.level_desc().unwrap(),
            &metadata,
            EntityConstructionResources::new(
                session.cache.terrain(),
                session.cache.terrain_objects(),
            ),
        ),
        metadata,
    )
}

fn publish_first(
    manager: &mut EntityManager,
    session: &GameSession,
    metadata: &[EntityTypeRuntimeMetadata],
) -> u32 {
    let spawn = session
        .cache
        .level_desc()
        .unwrap()
        .entities
        .iter()
        .find(|spawn| spawn.entity_type == 6)
        .unwrap();
    let id = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(spawn.index))
        .unwrap()
        .id;
    let allocation = manager.main_base_abort_actor_observation(id).unwrap().lease;
    publish_main_base(
        manager.entity_mut(id).unwrap(),
        allocation,
        &metadata[6],
        spawn,
        session.cache.terrain().unwrap(),
        &mut || 0x4567,
    )
    .unwrap();
    id
}

#[v2k_test_support::retail_test]
fn every_authored_main_base_uses_actual_models_and_one_selector_word() {
    let Some(mut session) = session() else {
        return;
    };
    let mut count = 0;
    for level in 13..=49 {
        session.load_level_by_id(level, 1).unwrap();
        let (mut manager, metadata) = generic(&session);
        for spawn in session
            .cache
            .level_desc()
            .unwrap()
            .entities
            .iter()
            .filter(|spawn| spawn.entity_type == 6)
        {
            let id = manager
                .iter_all()
                .find(|entity| entity.authored_spawn_index == Some(spawn.index))
                .unwrap()
                .id;
            let allocation = manager.main_base_abort_actor_observation(id).unwrap().lease;
            let expected_models = manager.entity_mut(id).unwrap().model_slots;
            let mut draws = 0;
            let publication = publish_main_base(
                manager.entity_mut(id).unwrap(),
                allocation,
                &metadata[6],
                spawn,
                session.cache.terrain().unwrap(),
                &mut || {
                    draws += 1;
                    0x4567
                },
            )
            .unwrap();
            assert_eq!(draws, 1, "level {level} spawn {}", spawn.index);
            assert_eq!(publication.selection.program.class_id, 41);
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(entity.model_slots, expected_models);
            assert_eq!(
                entity.collision.health_raw,
                RetailRuntimeValue::Known(99_999)
            );
            assert!(matches!(
                entity.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::MainBase(_))
            ));
            assert_eq!(
                MainBaseOwner::adopt(&manager, id).unwrap().allocation(),
                allocation
            );
            count += 1;
        }
    }
    assert_eq!(count, 28);
}

#[v2k_test_support::retail_test]
fn lethal_main_base_reselects_and_terminal_tick_issues_one_abort_origin() {
    let Some(mut session) = session() else {
        return;
    };
    session.load_level_by_id(14, 1).unwrap();
    let (mut manager, metadata) = generic(&session);
    let id = publish_first(&mut manager, &session, &metadata);
    let first_task = manager
        .entity_mut(id)
        .unwrap()
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary);
    let mut fx = WorldFx::new();
    let death = publish_main_base_standard_death(&mut manager, id, &mut fx).unwrap();
    assert!(!death.returned_nonzero);
    assert!(death.selector_word.is_some());
    let owner = death.owner.unwrap();
    let entity = manager.entity_mut(id).unwrap();
    assert_ne!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        first_task
    );
    assert_eq!(
        entity.collision.health_raw,
        RetailRuntimeValue::Known(10_000_000)
    );
    assert_eq!(
        entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
        RetailRuntimeValue::Known(0)
    );
    let RetailRuntimeValue::Known(Some(base)) = &mut entity.base_factory_runtime else {
        panic!()
    };
    assert_eq!(base.progressive_death.elapsed_micros_raw, 1);
    base.progressive_death.elapsed_micros_raw = 3_100_001;
    entity.collision.state_flags_at_0x08.overwrite(
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    );
    entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    entity.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
    let mut notifications = GameplayNotifications::new();
    let tick = tick_main_base_owner(
        &mut manager,
        owner,
        MainBaseFrame {
            resources: &mut session.cache,
            world_fx: &mut fx,
            notifications: &mut notifications,
            elapsed_micros: 100_000,
            retail_tick: 1000,
            main_base_abort_active: false,
        },
    );
    assert!(
        matches!(tick.outcome, MainBaseOutcome::Advanced { .. }),
        "{:?}",
        tick.outcome
    );
    assert_eq!(
        tick.terminal_abort_origin.unwrap().actor_lease(),
        owner.allocation()
    );
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::Class0Timer(_))
    ));
    let tick = tick_main_base_owner(
        &mut manager,
        tick.retained_owner.unwrap(),
        MainBaseFrame {
            resources: &mut session.cache,
            world_fx: &mut fx,
            notifications: &mut notifications,
            elapsed_micros: 100_000,
            retail_tick: 1005,
            main_base_abort_active: true,
        },
    );
    assert!(
        matches!(tick.outcome, MainBaseOutcome::Advanced { .. }),
        "{:?}",
        tick.outcome
    );
    assert!(tick.terminal_abort_origin.is_none());
}

#[v2k_test_support::retail_test]
fn copied_base_receipt_is_rejected_before_death_prefix() {
    let Some(mut session) = session() else {
        return;
    };
    session.load_level_by_id(14, 1).unwrap();
    let (mut first, metadata) = generic(&session);
    let id = publish_first(&mut first, &session, &metadata);
    let (mut second, metadata) = generic(&session);
    let second_id = publish_first(&mut second, &session, &metadata);
    let copied = first.entity_mut(id).unwrap().main_base_runtime;
    second.entity_mut(second_id).unwrap().main_base_runtime = copied;
    assert_eq!(
        MainBaseOwner::adopt(&second, second_id),
        Err(MainBaseError::Identity)
    );
    let block =
        publish_main_base_standard_death(&mut second, second_id, &mut WorldFx::new()).unwrap_err();
    assert!(!block.committed_prefix);
    assert_eq!(
        second.entity_mut(second_id).unwrap().collision.health_raw,
        RetailRuntimeValue::Known(99_999)
    );
}

#[v2k_test_support::retail_test]
fn unavailable_death_sound_keeps_health_and_dying_prefix() {
    let Some(mut session) = session() else {
        return;
    };
    session.load_level_by_id(14, 1).unwrap();
    let (mut manager, metadata) = generic(&session);
    let id = publish_first(&mut manager, &session, &metadata);
    manager.entity_mut(id).unwrap().collision.death_sound_id = RetailRuntimeValue::Unresolved;
    let block =
        publish_main_base_standard_death(&mut manager, id, &mut WorldFx::new()).unwrap_err();
    assert!(block.committed_prefix);
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
    assert_eq!(
        entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
        RetailRuntimeValue::Known(DYING_STATE_BIT)
    );
}

#[v2k_test_support::retail_test]
fn ordinary_main_base_particle_hits_keep_fixed_pose_and_publish_real_death_task() {
    use super::impact::{apply_main_base_particle_hit, MainBaseImpactOutcome};
    use crate::{
        damage::DamagePacket,
        specialized_actor_task_production::SpecializedActorTaskScheduler,
        world_fx::{BallisticDamageRequest, ParticleEntityImpact},
    };
    let Some(mut session) = session() else {
        return;
    };
    session.load_level_by_id(14, 1).unwrap();
    let (mut manager, metadata) = generic(&session);
    let id = publish_first(&mut manager, &session, &metadata);
    let entity = manager.entity_mut(id).unwrap();
    let fixed = (
        entity.position_raw(),
        entity.velocity_raw(),
        entity.rotation_heading_pitch_roll_raw(),
    );
    let original_task = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(scheduler.adopt_main_base(&manager), 1);
    let mut fx = WorldFx::new();
    let impact = |amount| ParticleEntityImpact {
        source_particle_class: 16,
        impact_position_argument_va: 0,
        target_entity_id: id,
        position_world: [0.0; 3],
        velocity_raw: [0, 0, 8192],
        damage: Some(BallisticDamageRequest {
            packet: DamagePacket {
                channels: [2, 0],
                amounts_raw: [amount, 0],
            },
            source_entity_type_at_birth: Some(34),
            source_owner_id: Some(35),
        }),
    };
    let MainBaseImpactOutcome::Applied(nonlethal) =
        apply_main_base_particle_hit(&mut manager, &mut fx, &mut scheduler, impact(2000), 251)
    else {
        panic!("native nonlethal hit")
    };
    assert_eq!(nonlethal.filtered_damage_raw, 1800);
    assert!(nonlethal.death_publication.is_none());
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        entity.collision.health_raw,
        RetailRuntimeValue::Known(98_199)
    );
    assert_eq!(
        entity.collision.last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(251)
    );
    assert_eq!(
        (
            entity.position_raw(),
            entity.velocity_raw(),
            entity.rotation_heading_pitch_roll_raw()
        ),
        fixed
    );
    assert_eq!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        original_task
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        38,
        "null hooks/fixed impulse use no RNG"
    );
    let result =
        apply_main_base_particle_hit(&mut manager, &mut fx, &mut scheduler, impact(100_000), 252);
    let MainBaseImpactOutcome::Applied(lethal) = result else {
        panic!("{result:?}")
    };
    assert!(lethal.death_publication.is_some());
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        entity.collision.health_raw,
        RetailRuntimeValue::Known(10_000_000)
    );
    assert_ne!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        original_task
    );
    assert_eq!(
        scheduler
            .main_base_type66_actor_claims()
            .collect::<Vec<_>>(),
        [MainBaseOwner::adopt(&manager, id).unwrap().allocation()]
    );
}

#[v2k_test_support::retail_test]
fn parked_base_retains_partial_graph_only_for_the_original_allocation() {
    let Some(mut session) = session() else {
        return;
    };
    session.load_level_by_id(14, 1).unwrap();
    let (mut manager, metadata) = generic(&session);
    let id = publish_first(&mut manager, &session, &metadata);
    let owner = MainBaseOwner::adopt(&manager, id).unwrap();
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Unresolved;
    entity.collision.state_flags_at_0x08.overwrite(
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    );
    entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    entity.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
    let mut fx = WorldFx::new();
    let mut notifications = GameplayNotifications::new();
    let tick = tick_main_base_owner(
        &mut manager,
        owner,
        MainBaseFrame {
            resources: &mut session.cache,
            world_fx: &mut fx,
            notifications: &mut notifications,
            elapsed_micros: 20_000,
            retail_tick: 1000,
            main_base_abort_active: false,
        },
    );
    assert!(
        matches!(
            tick.outcome,
            MainBaseOutcome::Blocked {
                prefix_committed: true,
                ..
            }
        ),
        "{:?}",
        tick.outcome
    );
    let owner = tick.retained_owner.unwrap();
    manager.entity_mut(id).unwrap().current_behavior_context = RetailRuntimeValue::Unresolved;
    let tick = tick_main_base_owner(
        &mut manager,
        owner,
        MainBaseFrame {
            resources: &mut session.cache,
            world_fx: &mut fx,
            notifications: &mut notifications,
            elapsed_micros: 20_000,
            retail_tick: 1001,
            main_base_abort_active: false,
        },
    );
    assert!(matches!(tick.outcome, MainBaseOutcome::Pending { .. }));
    let (mut replacement, metadata) = generic(&session);
    let new_id = publish_first(&mut replacement, &session, &metadata);
    assert_eq!(id, new_id);
    let tick = tick_main_base_owner(
        &mut replacement,
        owner,
        MainBaseFrame {
            resources: &mut session.cache,
            world_fx: &mut fx,
            notifications: &mut notifications,
            elapsed_micros: 20_000,
            retail_tick: 1002,
            main_base_abort_active: false,
        },
    );
    assert!(matches!(tick.outcome, MainBaseOutcome::Dropped { .. }));
    assert!(tick.retained_owner.is_none());
}
