//! Canonical world49/Intro1 corpus and 104B0/09A80/D4A0 constructor checks.
use super::*;
use crate::{
    damage::DamagePacket,
    entity::{AuthoredWorldConstruction, EntityConstructionResources},
    entity_collision_state::{
        EntityTypeRuntimeMetadata, RetailRuntimeValue, BODY_BASIS_REBUILT_STATE_BIT,
        DYING_STATE_BIT,
    },
    gameplay_notifications::GameplayNotifications,
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
    world_fx::{BallisticDamageRequest, ParticleEntityImpact},
};

pub(super) fn load(level_id: u32, fx: &mut WorldFx) -> Option<(GameSession, EntityManager)> {
    let data = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(level_id, 1).unwrap();
    let metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .copied()
        .enumerate()
        .map(|(id, model_slots)| {
            session
                .cache
                .global_entity_type(id)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots,
                    ..Default::default()
                })
        })
        .collect();
    let manager = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            logical_world_index: (level_id - 12) as i32,
            level: session.cache.level_desc().unwrap(),
            type_metadata: &metadata,
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
        fx,
    )
    .unwrap_or_else(|error| panic!("level{level_id} native construction: {error:?}"));
    Some((session, manager))
}

#[v2k_test_support::retail_test]
fn all_eleven_intro1_type123_publish_with_own_descriptors() {
    let mut fx = WorldFx::new();
    let Some((_session, manager)) = load(49, &mut fx) else {
        return;
    };
    let people: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.entity_type == 123)
        .collect();
    assert_eq!(people.len(), 11);
    for entity in people {
        assert!(
            native_type123_allocation_authenticates(entity),
            "world49 Type123 failed allocation"
        );
        assert_eq!(entity.model_slots, [Some(MODEL); 4]);
        assert_eq!(entity.model_index, Some(MODEL));
        assert_eq!(entity.capability_flags, CAPABILITY);
        assert_eq!(
            entity.collision.health_raw,
            RetailRuntimeValue::Known(HEALTH)
        );
        let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
            panic!("world49: missing Type123 Sub-I");
        };
        assert_eq!(animation.descriptor().attention_stop_sound_id, 56);
        assert_eq!(animation.descriptor().capability_bit_3_sound_id, 0);
        assert_eq!(
            entity.collision.accepted_hit_presentation_sound_id,
            RetailRuntimeValue::Known(Some(ACCEPTED_HIT_SOUND))
        );
        assert_eq!(
            entity.collision.death_sound_id,
            RetailRuntimeValue::Known(Some(DEATH_SOUND))
        );
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(BODY_BASIS_REBUILT_STATE_BIT),
            RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT)
        );
        // Each birth retains its own authored spawn and Sub-D seed.
        assert!(entity.authored_spawn_index.is_some());
        assert!(entity.type8_sub_d_frame_owner.is_some());
        assert!(entity.type8_sub_d_runtime.is_some());
    }
    // World49's Type122 uses the same loader route; the people must not
    // borrow its receipt.
    let ants = manager
        .iter_all()
        .filter(|entity| entity.entity_type == 122)
        .count();
    assert_eq!(ants, 1);
}

#[v2k_test_support::retail_test]
fn type123_metadata_rejects_foreign_model_health_axis_choices_and_sounds() {
    let mut fx = WorldFx::new();
    let Some((session, _)) = load(49, &mut fx) else {
        return;
    };
    let metadata =
        EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(123).unwrap());
    birth::validate_metadata(&metadata).unwrap();
    let mut metadata = metadata;
    metadata.model_slots = [559; 4];
    assert!(birth::validate_metadata(&metadata).is_err());
    let mut metadata =
        EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(123).unwrap());
    metadata.initial_health_raw = Some(2000);
    assert!(birth::validate_metadata(&metadata).is_err());
    let mut metadata =
        EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(123).unwrap());
    metadata
        .initializer
        .as_mut()
        .unwrap()
        .common_axis_descriptor
        .strict_axis_limit_raw = 0xf00;
    assert!(birth::validate_metadata(&metadata).is_err());
    let mut metadata =
        EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(123).unwrap());
    metadata
        .initializer
        .as_mut()
        .unwrap()
        .behavior_choices
        .swap(0, 1);
    assert!(birth::validate_metadata(&metadata).is_err());
    let mut metadata =
        EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(123).unwrap());
    metadata.death_sound_id = RetailRuntimeValue::Known(None);
    assert!(birth::validate_metadata(&metadata).is_err());
}

#[v2k_test_support::retail_test]
fn type123_lifecycle_runs_move_hit_death_and_class14_removal() {
    let mut fx = WorldFx::new();
    let Some((mut session, mut manager)) = load(49, &mut fx) else {
        return;
    };
    let before: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.entity_type == 123)
        .map(|entity| (entity.id, entity.position_raw()))
        .collect();
    assert_eq!(before.len(), 11);
    let mut scheduler = SpecializedActorTaskScheduler::default();
    assert_eq!(scheduler.adopt_native_type123(&mut manager), 11);
    // Second adoption retains single ownership.
    assert_eq!(scheduler.adopt_native_type123(&mut manager), 0);
    let mut static_damage = StaticDamageScheduler::default();
    let mut notifications = GameplayNotifications::new();
    let mut moved = false;
    for step in 1..=30 {
        let pass = scheduler.tick(
            &mut manager,
            SpecializedActorTaskProductionFrame {
                world:
                    crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut static_damage,
                elapsed_micros: 100_000,
                global_elapsed_micros: 100_000,
                retail_tick: step * 5,
                main_base_abort_active: false,
            },
            &mut notifications,
        );
        assert!(pass.block.is_none(), "world49: {:?}", pass.block);
        for outcome in &pass.outcomes {
            if let SpecializedActorTaskProductionOutcome::NativeType123(outcome) = outcome {
                assert!(
                    !matches!(
                        outcome,
                        Type123Outcome::Blocked { .. }
                            | Type123Outcome::Dropped { .. }
                            | Type123Outcome::Pending { .. }
                    ),
                    "world49: {outcome:?}"
                );
            }
        }
        moved |= before.iter().any(|(id, position)| {
            manager
                .iter_all()
                .find(|entity| entity.id == *id)
                .is_some_and(|entity| entity.position_raw() != *position)
        });
        manager.cleanup_pending_actor_deferred_destroys();
    }
    assert!(moved, "world49 Type123 people never moved");
    let target = before[0].0;
    let hit = impact::apply_native_type123_particle_hit(
        &mut manager,
        &mut fx,
        &mut scheduler,
        ParticleEntityImpact {
            source_particle_class: 38,
            impact_position_argument_va: 0,
            target_entity_id: target,
            position_world: [0.; 3],
            velocity_raw: [-2303, 297, -182],
            damage: Some(BallisticDamageRequest {
                packet: DamagePacket {
                    channels: [2, 3],
                    amounts_raw: [500, 6000],
                },
                source_entity_type_at_birth: Some(10),
                source_owner_id: Some(0x045f_0001),
            }),
        },
        151,
    );
    assert!(
        matches!(hit, impact::NativeType123ImpactOutcome::Applied(ref result)
        if result.filtered_damage_raw == 12100 && result.death_publication.is_some()),
        "world49: {hit:?}"
    );
    let actor = manager.entity_mut(target).unwrap();
    assert_eq!(actor.collision.health_raw, RetailRuntimeValue::Known(0));
    assert_eq!(
        actor.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
        RetailRuntimeValue::Known(DYING_STATE_BIT)
    );
    let mut removed = false;
    for step in 31..=90 {
        let pass = scheduler.tick(
            &mut manager,
            SpecializedActorTaskProductionFrame {
                world:
                    crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut static_damage,
                elapsed_micros: 100_000,
                global_elapsed_micros: 100_000,
                retail_tick: step * 5,
                main_base_abort_active: false,
            },
            &mut notifications,
        );
        assert!(pass.block.is_none(), "world49: {:?}", pass.block);
        removed |= manager
            .cleanup_pending_actor_deferred_destroys()
            .contains(&target);
        if removed {
            break;
        }
    }
    assert!(
        removed,
        "world49 Type123 class14 did not remove the killed allocation"
    );
}

#[v2k_test_support::retail_test]
fn transplanted_type123_receipt_cannot_enter_scheduler_or_commit_death() {
    let Some((_session, first)) = load(49, &mut WorldFx::new()) else {
        return;
    };
    let Some((_session, mut second)) = load(49, &mut WorldFx::new()) else {
        return;
    };
    let actor = first
        .iter_all()
        .find(|entity| entity.entity_type == 123)
        .unwrap();
    let id = actor.id;
    second.entity_mut(id).unwrap().native_type123_runtime = actor.native_type123_runtime;
    assert!(!native_type123_manager_allocation_authenticates(
        &second, id
    ));
    let mut scheduler = SpecializedActorTaskScheduler::default();
    assert_eq!(scheduler.adopt_native_type123(&mut second), 10);
    assert!(
        second
            .entity_mut(id)
            .unwrap()
            .native_type123_runtime
            .unwrap()
            .birth_pending
    );
    assert!(matches!(
        impact::run_native_type123_standard_death(&mut second, id, &mut WorldFx::new()),
        Err(Type123Block::Runtime("native allocation"))
    ));
    assert_eq!(
        second.entity_mut(id).unwrap().collision.health_raw,
        RetailRuntimeValue::Known(HEALTH)
    );
    assert!(!scheduler.begin_native_type123_external_mutation(&second, id));
}
