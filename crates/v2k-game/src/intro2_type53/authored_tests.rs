//! Canonical authored construction and issuing-manager custody, independent of
//! the four accepted Intro2 first-query replay allocations.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    common_mover::type9_attitude::Type9BodyBasis,
    entity::{AuthoredWorldConstruction, EntityConstructionResources},
    gameplay_notifications::GameplayNotifications,
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
    world_fx::WorldFx,
};

pub(crate) fn native_fixture(level: u32) -> (GameSession, EntityManager, WorldFx) {
    let (mut session, metadata) =
        super::tests::fixture().expect("normal-tier retail corpus required");
    session.load_level_by_id(level, 1).unwrap();
    let mut fx = WorldFx::new();
    let entities = construct(&session, &metadata, level, &mut fx);
    (session, entities, fx)
}

fn construct(
    session: &GameSession,
    metadata: &[EntityTypeRuntimeMetadata],
    level: u32,
    fx: &mut WorldFx,
) -> EntityManager {
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&model_extent),
    };
    if level == 50 {
        EntityManager::from_native_intro2_frontend(
            session.cache.level_desc().unwrap(),
            metadata,
            resources,
            4793,
            fx,
        )
        .unwrap()
    } else {
        EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                level: session.cache.level_desc().unwrap(),
                logical_world_index: level as i32 - 12,
                type_metadata: metadata,
                resources,
                player_arrival: None,
                retail_tick: 4793,
            },
            fx,
        )
        .unwrap()
    }
}

fn check_births(
    session: &GameSession,
    metadata: &[EntityTypeRuntimeMetadata],
    manager: &EntityManager,
    mut seed: u8,
    native_intro: bool,
) -> Vec<u32> {
    if !native_intro
        && manager.player().is_some()
        && matches!(
            metadata[46].sub_d_steering_descriptor,
            RetailRuntimeValue::Known(Some(_))
        )
    {
        seed = seed.wrapping_add(1);
    }
    let mut ids = Vec::new();
    for spawn in &session.cache.level_desc().unwrap().entities {
        if spawn.entity_type == 53 {
            let entity = manager
                .iter_all()
                .find(|entity| entity.authored_spawn_index == Some(spawn.index))
                .unwrap();
            let runtime = entity
                .intro2_type53_runtime
                .expect("actual native constructor receipt");
            assert!(matches!(
                runtime.origin,
                Type53ConstructionOrigin::Native(_)
            ));
            assert!(type53_manager_allocation_authenticates(manager, entity.id));
            assert_eq!(
                runtime.sub_d_owner.classifier_cache().stagger_counter(),
                seed,
                "actual process allocation for spawn{}",
                spawn.index
            );
            assert_eq!(
                runtime.sub_d_owner.classifier_cache().origin(),
                RetailRuntimeValue::Unresolved
            );
            assert_eq!(runtime.sub_d_owner.classifier_cache().rows(), [0; 8]);
            assert!(runtime.sub_d_owner.classifier_cache().can_classify());
            assert_eq!(runtime.sub_d_runtime, Type9SubDRuntime::from_constructor());
            assert_eq!(entity.model_slots, [Some(MODEL); 4]);
            assert_eq!(entity.position_raw(), runtime.anchor_raw);
            assert_eq!(
                runtime.anchor_raw,
                [
                    spawn.position_raw()[0],
                    session
                        .cache
                        .terrain()
                        .unwrap()
                        .bilinear_height_raw(spawn.position_raw()[0], spawn.position_raw()[2])
                        .wrapping_add(50),
                    spawn.position_raw()[2]
                ]
            );
            assert_eq!(
                entity.rotation_heading_pitch_roll_raw(),
                spawn.rotation.map(|word| word as i16)
            );
            let [h, p, r] = entity.rotation_heading_pitch_roll_raw();
            assert_eq!(
                entity.physical_body_basis_q31(),
                RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(h, p, r))
            );
            assert_eq!(
                entity.collision.animation_offset_at_0xb2,
                RetailRuntimeValue::Known(0)
            );
            assert_eq!(
                entity.collision.pre_health_damage_buffer_raw,
                RetailRuntimeValue::Known(spawn.initial_damage_buffer_raw)
            );
            assert!(Intro2Type53Owner::adopt(manager, entity.id).is_ok());
            assert!(
                matches!(entity.current_behavior_context, RetailRuntimeValue::Known(Some(context))
                if matches!(context.descriptor(), crate::entity_behavior::BehaviorDescriptorIdentity::Named(program)
                    if matches!(program.class_id, 7 | 9 | 33)))
            );
            ids.push(entity.id);
        }
        if matches!(
            metadata[spawn.entity_type as usize].sub_d_steering_descriptor,
            RetailRuntimeValue::Known(Some(_))
        ) {
            seed = seed.wrapping_add(1);
        }
    }
    ids
}

#[v2k_test_support::retail_test]
fn every_ordinary_type53_retains_actual_authored_constructor_and_process_sub_d() {
    let (mut session, metadata) =
        super::tests::fixture().expect("normal-tier retail corpus required");
    let mut fx = WorldFx::new();
    let mut census = Vec::new();
    for level in 13..=49 {
        session.load_level_by_id(level, 1).unwrap();
        let count = session
            .cache
            .level_desc()
            .unwrap()
            .entities
            .iter()
            .filter(|spawn| spawn.entity_type == 53)
            .count();
        if count == 0 {
            continue;
        }
        let seed = fx.next_sub_d_allocation_seed();
        let manager = construct(&session, &metadata, level, &mut fx);
        let ids = check_births(&session, &metadata, &manager, seed, false);
        assert_eq!(ids.len(), count);
        let mut scheduler = SpecializedActorTaskScheduler::default();
        assert_eq!(scheduler.adopt_intro2_type53(&manager), count);
        assert_eq!(scheduler.adopt_intro2_type53(&manager), 0);
        if level == 14 {
            assert!(manager
                .iter_all()
                .any(|entity| entity.entity_type == 53 && entity.authored_spawn_index == Some(35)));
        }
        census.push((level, count));
    }
    assert!(census.iter().any(|(level, _)| *level == 14));
    println!("canonical ordinary Type53 census: {census:?}");
}

#[v2k_test_support::retail_test]
fn native_intro_type53_uses_live_process_history_while_replay_retains_its_four_origins() {
    let (mut session, metadata) =
        super::tests::fixture().expect("normal-tier retail corpus required");
    let mut fx = WorldFx::new();
    session.load_level_by_id(14, 1).unwrap();
    let _earlier = construct(&session, &metadata, 14, &mut fx);
    let seed = fx.next_sub_d_allocation_seed();
    session.load_level_by_id(50, 1).unwrap();
    let manager = construct(&session, &metadata, 50, &mut fx);
    assert_eq!(
        check_births(&session, &metadata, &manager, seed, true).len(),
        4
    );
    // Existing tests cover each retained replay first query; this control
    // ensures the native route cannot accidentally inherit those fixed seeds.
    let runtime = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(20))
        .unwrap()
        .intro2_type53_runtime
        .unwrap();
    assert_ne!(
        runtime.sub_d_owner.classifier_cache().stagger_counter(),
        0x13
    );
}

#[v2k_test_support::retail_test]
fn native_type53_foreign_manager_receipt_cannot_be_adopted_or_publish_class12() {
    let (_, mut first, _) = native_fixture(14);
    let (_, mut second, mut fx) = native_fixture(14);
    let entity = first
        .iter_all()
        .find(|entity| entity.entity_type == 53)
        .unwrap();
    let id = entity.id;
    std::mem::swap(
        first.entity_mut(id).unwrap(),
        second.entity_mut(id).unwrap(),
    );
    assert!(
        intro2_type53_allocation_authenticates(second.entity_mut(id).unwrap()),
        "entity fields alone do not authenticate manager allocation generation"
    );
    assert!(!type53_manager_allocation_authenticates(&second, id));
    assert_eq!(
        Intro2Type53Owner::adopt(&second, id),
        Err(Intro2Type53Block::Allocation)
    );
    let health = second.entity_mut(id).unwrap().collision.health_raw;
    assert!(
        crate::intro2_common_dying::publish_intro2_common_standard_death(&mut second, id, &mut fx)
            .is_err()
    );
    assert_eq!(second.entity_mut(id).unwrap().collision.health_raw, health);
}

#[v2k_test_support::retail_test]
fn ordinary_type53_runs_the_actual_scheduler_and_retains_its_components() {
    let (mut session, mut manager, mut fx) = native_fixture(14);
    let ids: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.entity_type == 53)
        .map(|entity| entity.id)
        .collect();
    let before: Vec<_> = ids
        .iter()
        .map(|&id| manager.entity_mut(id).unwrap().position_raw())
        .collect();
    let mut scheduler = SpecializedActorTaskScheduler::default();
    assert_eq!(scheduler.adopt_intro2_type53(&manager), ids.len());
    let mut notifications = GameplayNotifications::new();
    let mut static_damage = StaticDamageScheduler::new();
    for tick in 4794..4894 {
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
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: tick,
                main_base_abort_active: false,
            },
            &mut notifications,
        );
        assert!(pass.block.is_none(), "{pass:?}");
        for outcome in pass.outcomes {
            assert!(
                matches!(
                    outcome,
                    SpecializedActorTaskProductionOutcome::Intro2Type53(
                        Intro2Type53Outcome::Advanced { .. } | Intro2Type53Outcome::Waiting { .. }
                    )
                ),
                "native Type53 scheduler: {outcome:?}"
            );
        }
    }
    assert!(ids.iter().zip(before).any(|(&id, position)| manager
        .entity_mut(id)
        .unwrap()
        .position_raw()
        != position));
    for id in ids {
        let entity = manager.entity_mut(id).unwrap();
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(
                ActorTaskRuntime::SharedRetarget(_)
                    | ActorTaskRuntime::FollowBeaconsFollowing(_)
                    | ActorTaskRuntime::CapturePeoplePursuit(_)
                    | ActorTaskRuntime::ChaseTarget(_)
            )
        ));
        assert!(type53_manager_allocation_authenticates(&manager, id));
    }
}

fn component_allocation(
    metadata: &EntityTypeRuntimeMetadata,
) -> crate::common_mover::sub_d::NativeSubDConstruction {
    let RetailRuntimeValue::Known(Some(descriptor)) = metadata.sub_d_steering_descriptor else {
        panic!("canonical Sub-D");
    };
    crate::common_mover::sub_d::construct_native_sub_d(
        &mut crate::common_mover::sub_d::SubDAllocationCounter::from_next_seed(0xd3),
        descriptor,
    )
}

#[v2k_test_support::retail_test]
fn shared_type53_constructor_preserves_authored_bits_player_prefix_and_four_word_order() {
    let (session, metadata) = super::tests::fixture().expect("normal-tier retail corpus required");
    let mut spawn = session.cache.level_desc().unwrap().entities[20].clone();
    spawn.index = 99;
    spawn.rotation = [0x9000, 0x0800, 0xfc00];
    let x = spawn.position_raw()[0].wrapping_add(19);
    let z = spawn.position_raw()[2].wrapping_sub(37);
    spawn.pos_data_1[..2].copy_from_slice(&x.to_le_bytes());
    spawn.pos_data_2[..2].copy_from_slice(&z.to_le_bytes());
    for (param, capabilities, class, choice) in [(0, 0, 33, 0), (1, 0xc00, 9, 2), (0, 1, 7, 3)] {
        spawn.param = param;
        let mut manager = super::tests::generic(&session, &metadata);
        let allocation = manager.main_base_abort_actor_observation(21).unwrap().lease;
        let mut prefix = super::tests::generic(&session, &metadata);
        let candidate = prefix.entity_mut(1).unwrap();
        // Controlled predicate input representing the persistent player's
        // genuine unindexed position in the already-linked prefix.
        candidate.authored_spawn_index = None;
        candidate.capability_flags = capabilities;
        candidate.collision.state_flags_at_0x08 =
            crate::entity_collision_state::RetailStateWord::exact(4);
        candidate.set_position_raw([
            x,
            session
                .cache
                .terrain()
                .unwrap()
                .bilinear_height_raw(x, z)
                .wrapping_add(50),
            z,
        ]);
        let entity = manager.entity_mut(21).unwrap();
        entity.authored_spawn_index = Some(spawn.index);
        entity.set_rotation_heading_pitch_roll_raw(spawn.rotation.map(|word| word as i16));
        let sub_d = component_allocation(&metadata[53]);
        let mut words = [0x5300, 0xffff, 0x1234, 0x9876].into_iter();
        let publication = publish_authored_type53(
            Type53AuthoredConstructionRequest {
                entity,
                allocation,
                metadata: &metadata[53],
                spawn: &spawn,
                preceding: std::slice::from_ref(candidate),
                resources: EntityConstructionResources::new(
                    session.cache.terrain(),
                    session.cache.terrain_objects(),
                ),
                constructor_surface_bits: 0,
                retail_tick: 0xffff_ffed,
                sub_d,
            },
            &mut || words.next().expect("actual four source constructor words"),
        )
        .unwrap();
        assert!(words.next().is_none());
        assert_eq!(
            (
                publication.selection.program.class_id,
                publication.selection.choice_index
            ),
            (class, choice)
        );
        assert_eq!(publication.people_nearby, capabilities & 0xc00 != 0);
        assert_eq!(publication.player_nearby, capabilities & 1 != 0);
        let entity = manager.entity_mut(21).unwrap();
        let runtime = entity.intro2_type53_runtime.unwrap();
        assert_eq!(runtime.sub_d_owner, sub_d.frame_owner);
        assert_eq!(runtime.sub_d_runtime, sub_d.runtime);
        assert_eq!(runtime.anchor_raw, candidate.position_raw());
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(0x0100_0000),
            RetailRuntimeValue::Known(if param == 0 { 0 } else { 0x0100_0000 })
        );
        assert_eq!(
            entity.rotation_heading_pitch_roll_raw(),
            spawn.rotation.map(|word| word as i16)
        );
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!("constructed A");
        };
        assert_eq!(
            sub_a.target_speed_raw(),
            RetailRuntimeValue::Known(crate::common_mover::shared_initializer_target_speed_raw(
                400, 0x9876
            ))
        );
        assert!(type53_manager_allocation_authenticates(&manager, 21));
    }
}

#[v2k_test_support::retail_test]
fn shared_type53_read_only_admission_and_late_predicate_failure_keep_distinct_prefixes() {
    let (session, metadata) = super::tests::fixture().expect("normal-tier retail corpus required");
    let spawn = &session.cache.level_desc().unwrap().entities[20];
    for missing_terrain in [true, false] {
        let mut manager = super::tests::generic(&session, &metadata);
        let allocation = manager.main_base_abort_actor_observation(21).unwrap().lease;
        let mut prefix = super::tests::generic(&session, &metadata);
        let candidate = prefix.entity_mut(1).unwrap();
        candidate.capability_flags = 1;
        candidate.collision.state_flags_at_0x08 =
            crate::entity_collision_state::RetailStateWord::unknown();
        let entity = manager.entity_mut(21).unwrap();
        let before = (
            entity.collision.state_flags_at_0x08,
            entity.sub_a_propulsion_runtime,
        );
        let sub_d = component_allocation(&metadata[53]);
        let mut draws = 0;
        let result = publish_authored_type53(
            Type53AuthoredConstructionRequest {
                entity,
                allocation,
                metadata: &metadata[53],
                spawn,
                preceding: std::slice::from_ref(candidate),
                resources: EntityConstructionResources::new(
                    if missing_terrain {
                        None
                    } else {
                        session.cache.terrain()
                    },
                    session.cache.terrain_objects(),
                ),
                constructor_surface_bits: 0,
                retail_tick: 111,
                sub_d,
            },
            &mut || {
                draws += 1;
                0x4321
            },
        );
        if missing_terrain {
            assert_eq!(
                result,
                Err(Intro2Type53Error::Runtime("constructor terrain"))
            );
            assert_eq!(draws, 0);
            assert_eq!(
                (
                    entity.collision.state_flags_at_0x08,
                    entity.sub_a_propulsion_runtime
                ),
                before
            );
            assert!(entity.intro2_type53_runtime.is_none());
        } else {
            assert_eq!(result, Err(Intro2Type53Error::NearbyEvidence));
            assert_eq!(draws, 1, "20450 precedes selector predicate reads");
            let runtime = entity.intro2_type53_runtime.unwrap();
            assert_eq!(runtime.sub_d_owner, sub_d.frame_owner);
            let RetailRuntimeValue::Known(Some(a)) = metadata[53].sub_a_propulsion_descriptor
            else {
                panic!();
            };
            assert_eq!(
                entity.sub_a_propulsion_runtime,
                RetailRuntimeValue::Known(Some(
                    crate::common_mover::SubAPropulsionRuntime::from_20450_constructor(a, 0x4321)
                ))
            );
        }
        assert_eq!(entity.initial_behavior, RetailRuntimeValue::Unresolved);
        assert_eq!(
            entity.current_behavior_context,
            RetailRuntimeValue::Unresolved
        );
        assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| entity.actor_task_state(slot).is_none()));
    }
}
