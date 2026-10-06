//! Canonical ordinary Type30 allocation, weighted constructor and custody.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    common_mover::{
        shared_initializer_target_speed_raw,
        sub_d::{construct_native_sub_d, SubDAllocationCounter},
    },
    entity::{EntityConstructionResources, EntityKind},
    entity_behavior::behavior_program,
    entity_collision_state::RetailStateWord,
    session::GameSession,
};

fn fixture() -> (GameSession, Vec<EntityTypeRuntimeMetadata>) {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("normal-tier retail corpus required");
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(17, 1).unwrap();
    let metadata = session
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
    (session, metadata)
}
fn generic(session: &GameSession, metadata: &[EntityTypeRuntimeMetadata]) -> EntityManager {
    EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        metadata,
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&|id| session.cache.global_model(id).map(|model| model.radius)),
        },
    )
}

#[test]
fn type30_weighted_intervals_keep_unconditional_furniture_and_one_selector_word() {
    for (player, cases) in [
        (false, vec![(0, 5), (13107, 5), (13108, 26), (65535, 26)]),
        (
            true,
            vec![
                (0, 5),
                (4681, 5),
                (4682, 7),
                (46811, 7),
                (46812, 26),
                (65535, 26),
            ],
        ),
    ] {
        for (word, class) in cases {
            let mut draws = 0;
            let selection = tasks::select(player, &mut || {
                draws += 1;
                0xdead0000 | word
            })
            .unwrap();
            assert_eq!(draws, 1);
            assert_eq!(selection.program.class_id, class);
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type30_constructor_owns_real_lease_and_three_or_four_rng_words() {
    let (session, metadata) = fixture();
    authenticate_metadata(&metadata[30]).unwrap();
    let spawn = session
        .cache
        .level_desc()
        .unwrap()
        .entities
        .iter()
        .find(|spawn| spawn.entity_type == 30)
        .unwrap()
        .clone();
    for (player, selector, class, suffix_words) in
        [(false, 0, 5, 1), (false, 65535, 26, 1), (true, 10000, 7, 2)]
    {
        let mut manager = generic(&session, &metadata);
        let id = manager
            .iter_all()
            .find(|e| e.authored_spawn_index == Some(spawn.index))
            .unwrap()
            .id;
        let allocation = manager.main_base_abort_actor_observation(id).unwrap().lease;
        let mut candidate = Entity::unresolved_port_entity(0x10000, EntityKind::Player, 46);
        candidate.authored_spawn_index = None;
        candidate.capability_flags = if player { 1 } else { 0 };
        candidate.set_position_raw([spawn.position_raw()[0], 100, spawn.position_raw()[2]]);
        candidate.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
        candidate.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        let RetailRuntimeValue::Known(Some(d)) = metadata[30].sub_d_steering_descriptor else {
            panic!("D")
        };
        let mut counter = SubDAllocationCounter::from_next_seed(0);
        let sub_d = construct_native_sub_d(&mut counter, d);
        let mut words = [0x3000, selector, 0x1234, 0x5678].into_iter();
        let mut draws = 0;
        let publication = construction::publish_authored_type30(
            construction::Type30AuthoredConstructionRequest {
                entity: manager.entity_mut(id).unwrap(),
                allocation,
                metadata: &metadata[30],
                spawn: &spawn,
                preceding: std::slice::from_ref(&candidate),
                resources: EntityConstructionResources::new(
                    session.cache.terrain(),
                    session.cache.terrain_objects(),
                ),
                constructor_surface_bits: 0,
                sub_d,
            },
            &mut || {
                draws += 1;
                words.next().unwrap()
            },
        )
        .unwrap();
        assert_eq!(publication.selection.program.class_id, class);
        assert_eq!(publication.player_nearby, player);
        assert!(!publication.initializer_fallback);
        assert_eq!(
            draws,
            2 + suffix_words,
            "A then selector then selected task suffixes"
        );
        assert!(manager_allocation_authenticates(&manager, id));
        assert!(Type30Owner::adopt(&manager, id).is_ok());
        let entity = manager.entity_mut(id).unwrap();
        let runtime = entity.native_type30_runtime.as_ref().unwrap();
        assert_eq!(runtime.allocation, allocation);
        assert_eq!(runtime.sub_d_owner, sub_d.frame_owner);
        assert_eq!(runtime.sub_d_runtime, sub_d.runtime);
        assert_eq!(runtime.kl_components.model_variables_raw(), &[0; 4]);
        assert_eq!(runtime.sub_e_runtime, tasks::emitter_constructor());
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        let RetailRuntimeValue::Known(Some(a)) = entity.sub_a_propulsion_runtime else {
            panic!("A")
        };
        assert_eq!(
            a.target_speed_raw(),
            RetailRuntimeValue::Known(shared_initializer_target_speed_raw(
                200,
                if suffix_words == 1 { 0x1234 } else { 0x5678 }
            ))
        );
        if class == 26 {
            assert!(
                matches!(entity.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::TrashFurniture(task)) if task.target_y_is_approximation() && task.target_y() == RetailRuntimeValue::Known(entity.position_raw()[1]))
            );
        }
        let mut foreign = generic(&session, &metadata);
        foreign.entity_mut(id).unwrap().native_type30_runtime =
            entity.native_type30_runtime.clone();
        assert!(
            !manager_allocation_authenticates(&foreign, id),
            "copied actor data cannot transfer allocator lease"
        );
    }
}

pub(crate) fn native_actor_fixture() -> (
    GameSession,
    Vec<EntityTypeRuntimeMetadata>,
    EntityManager,
    u32,
) {
    let (session, metadata) = fixture();
    let spawn = session
        .cache
        .level_desc()
        .unwrap()
        .entities
        .iter()
        .find(|spawn| spawn.entity_type == 30)
        .unwrap()
        .clone();
    let mut manager = generic(&session, &metadata);
    let id = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(spawn.index))
        .unwrap()
        .id;
    let allocation = manager.main_base_abort_actor_observation(id).unwrap().lease;
    let RetailRuntimeValue::Known(Some(d)) = metadata[30].sub_d_steering_descriptor else {
        panic!("D")
    };
    let mut counter = SubDAllocationCounter::from_next_seed(0);
    let publication = construction::publish_authored_type30(
        construction::Type30AuthoredConstructionRequest {
            entity: manager.entity_mut(id).unwrap(),
            allocation,
            metadata: &metadata[30],
            spawn: &spawn,
            preceding: &[],
            resources: EntityConstructionResources::new(
                session.cache.terrain(),
                session.cache.terrain_objects(),
            ),
            constructor_surface_bits: 0,
            sub_d: construct_native_sub_d(&mut counter, d),
        },
        &mut || 0,
    )
    .unwrap();
    assert_eq!(publication.selection.program.class_id, 5);
    (session, metadata, manager, id)
}

#[v2k_test_support::retail_test]
fn type30_task_reselection_retains_nonzero_native_kl_bank_emitter_and_transient_mass() {
    let (session, metadata, mut manager, id) = native_actor_fixture();
    let entity = manager.entity_mut(id).unwrap();
    entity.set_velocity_raw([0, 123, 0]);
    let [h, p, r] = entity.rotation_heading_pitch_roll_raw();
    entity.physical_body_basis_q31 = RetailRuntimeValue::Known(
        crate::common_mover::type9_attitude::Type9BodyBasis::from_angle_words(h, p, r),
    );
    let mut runtime = entity.native_type30_runtime.take().unwrap();
    crate::intro2_common_mover::run_intro2_common_mover(
        entity,
        crate::intro2_common_mover::Intro2CommonMoverFrame {
            metadata: &metadata[30],
            topology: TOPOLOGY,
            terrain: session.cache.terrain().unwrap(),
            wave_tick_50hz: None,
            dispatch_mode: crate::common_mover::component_dispatch::CommonMoverDispatchMode::Normal,
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
            sub_d_runtime: &mut runtime.sub_d_runtime,
            sub_d_owner: &mut runtime.sub_d_owner,
            kl_components: Some(&mut runtime.kl_components),
            target: None,
            tracked_target: RetailRuntimeValue::Known(None),
        },
        &mut || panic!("NULL target component phase draws no RNG"),
    )
    .unwrap();
    assert_ne!(runtime.kl_components.model_variables_raw(), &[0; 4]);
    runtime.sub_e_runtime.remaining_time_raw = 1234;
    let before = runtime.clone();
    entity.native_type30_runtime = Some(runtime);
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(17);
    for class in [5, 7, 26, 5] {
        let selection = BehaviorSelection {
            program: behavior_program(class).unwrap(),
            ..tasks::select(false, &mut || 0).unwrap()
        };
        let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection).unwrap();
        let mut draws = 0;
        assert!(tasks::publish_acquiring(
            entity,
            &metadata[30],
            selection,
            context,
            &mut || {
                draws += 1;
                0
            }
        ));
        assert_eq!(draws, if class == 7 { 2 } else { 1 });
        assert_eq!(entity.native_type30_runtime.as_ref(), Some(&before));
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(17)
        );
    }
}

#[v2k_test_support::retail_test]
fn type30_reselection_does_not_read_unused_people_predicate_or_last_hit_word() {
    let (_, _, mut manager, id) = native_actor_fixture();
    let ids: Vec<_> = manager
        .iter_all()
        .map(|entity| entity.id)
        .filter(|candidate| *candidate != id)
        .collect();
    let [player_id, unknown_id, ..] = ids.as_slice() else {
        panic!("canonical world population")
    };
    let position = manager.entity_mut(id).unwrap().position_raw();
    for candidate in &ids {
        manager.entity_mut(*candidate).unwrap().active = false;
    }
    let player = manager.entity_mut(*player_id).unwrap();
    player.active = true;
    player.capability_flags = 1;
    player.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
    player.set_position_raw(position);
    let unknown = manager.entity_mut(*unknown_id).unwrap();
    unknown.active = true;
    unknown.capability_flags = 0xc00;
    unknown.collision.state_flags_at_0x08 = RetailStateWord::unknown();
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
    entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Unresolved;
    let owner = shared::candidate(entity);
    let candidates: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.active)
        .map(shared::candidate)
        .collect();
    assert!(shared::nearby::<profile::Type30Profile>(owner, &candidates, 0xc00).is_err());
    assert_eq!(
        shared::nearby::<profile::Type30Profile>(owner, &candidates, 1),
        Ok(true)
    );
    shared::behavior::reselect::<profile::Type30Profile>(
        &mut manager,
        id,
        1234,
        &mut WorldFx::new(),
        shared::behavior::ReselectionEntry::TaskResult,
    )
    .unwrap();
}

#[v2k_test_support::retail_test]
fn canonical_type30_rejects_old_actor_kl_bindings_and_alternate_rule() {
    let (_, metadata) = fixture();
    let mut damaged = metadata[30].clone();
    authenticate_metadata(&damaged).unwrap();
    damaged.initializer.as_mut().unwrap().behavior_rule_ref = 1;
    assert_eq!(authenticate_metadata(&damaged), Err(Type30Block::Metadata));
    let mut damaged = metadata[30].clone();
    damaged.model_variable_count_raw = RetailRuntimeValue::Known(3);
    assert_eq!(authenticate_metadata(&damaged), Err(Type30Block::Metadata));
    let mut damaged = metadata[30].clone();
    let RetailRuntimeValue::Known(mut payloads) = damaged.common_mover_gkl_payloads else {
        panic!("K/L")
    };
    payloads.sub_k = Some([11, 10]);
    damaged.common_mover_gkl_payloads = RetailRuntimeValue::Known(payloads);
    assert_eq!(authenticate_metadata(&damaged), Err(Type30Block::Metadata));
}

fn prepare_dying_fixture() -> (
    GameSession,
    EntityManager,
    u32,
    WorldFx,
    crate::intro2_common_dying::Intro2CommonDyingOwner,
    Type30Runtime,
) {
    let (session, _, mut manager, id) = native_actor_fixture();
    let entity = manager.entity_mut(id).unwrap();
    entity.set_motion_raw([0, 10_000, 0], [50, -80, 100]);
    entity.set_rotation_heading_pitch_roll_raw([0; 3]);
    entity.physical_body_basis_q31 = RetailRuntimeValue::Known(
        crate::common_mover::type9_attitude::Type9BodyBasis::from_angle_words(0, 0, 0),
    );
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(u32::MAX, 0x68000);
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(9);
    let before = entity.native_type30_runtime.as_ref().unwrap().clone();
    let mut fx = WorldFx::new();
    let owner =
        crate::intro2_common_dying::publish_intro2_common_standard_death(&mut manager, id, &mut fx)
            .unwrap()
            .unwrap();
    // The detailed callback branch is selected by the real02000000 bit, not
    // the callback-enable/motion bits. Its source scheduler skips both waits.
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(
            crate::entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
            crate::entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
        );
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .native_type30_runtime
            .as_ref(),
        Some(&before),
        "C620 preserves native H/K/L variable bank and E/D inputs"
    );
    (session, manager, id, fx, owner, before)
}

#[v2k_test_support::retail_test]
fn type30_class12_null_target_uses_retained_kl_before_c_and_keeps_sub_d_custody() {
    let (session, mut manager, id, mut fx, owner, before) = prepare_dying_fixture();
    let tick = crate::intro2_common_dying::tick_intro2_common_dying(
        &mut manager,
        owner,
        crate::intro2_common_dying::Intro2CommonDyingFrame {
            resources: &session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            retail_tick: 1,
        },
    );
    assert!(
        matches!(
            tick.outcome,
            crate::intro2_common_dying::Intro2CommonDyingOutcome::Advanced {
                detailed: true,
                terminal: false,
                ..
            }
        ),
        "{:?}",
        tick.outcome
    );
    let entity = manager.entity_mut(id).unwrap();
    let runtime = entity.native_type30_runtime.as_ref().unwrap();
    assert_eq!(runtime.allocation, before.allocation);
    assert_eq!(runtime.sub_d_owner, before.sub_d_owner);
    assert_eq!(runtime.sub_d_runtime, before.sub_d_runtime);
    assert_eq!(
        runtime.kl_components.sub_k_smoothed_raw(),
        before.kl_components.sub_k_smoothed_raw()
    );
    assert_eq!(
        runtime.kl_components.sub_l_target_raw(),
        before.kl_components.sub_l_target_raw()
    );
    assert_eq!(
        runtime.kl_components.sub_l_exact_raw(),
        before.kl_components.sub_l_exact_raw()
    );
    assert_eq!(
        runtime.kl_components.model_variables_raw()[3],
        -1000,
        "K reads constructor VY500 before C and world integration"
    );
    assert_eq!(entity.mass_raw, 109);
    assert_eq!(
        entity.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(0)
    );
    assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
}

#[v2k_test_support::retail_test]
fn type30_class12_late_a_block_retains_kl_prefix_counter_and_unwinds_once() {
    let (session, mut manager, id, mut fx, owner, before) = prepare_dying_fixture();
    let entity = manager.entity_mut(id).unwrap();
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
        crate::common_mover::SubAPropulsionRuntime::from_retail_words(
            RetailRuntimeValue::Unresolved,
            1,
            100,
        ),
    ));
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    let tick = crate::intro2_common_dying::tick_intro2_common_dying(
        &mut manager,
        owner,
        crate::intro2_common_dying::Intro2CommonDyingFrame {
            resources: &session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            retail_tick: 1,
        },
    );
    assert!(
        matches!(
            tick.outcome,
            crate::intro2_common_dying::Intro2CommonDyingOutcome::Blocked {
                prefix_committed: true,
                ..
            }
        ),
        "{:?}",
        tick.outcome
    );
    let entity = manager.entity_mut(id).unwrap();
    let runtime = entity.native_type30_runtime.as_ref().unwrap();
    assert_eq!(runtime.kl_components.model_variables_raw()[3], -1000);
    assert_eq!(runtime.sub_d_owner, before.sub_d_owner);
    assert!(
        matches!(entity.actor_task_state(ActorTaskSlot::Primary),Some(ActorTaskRuntime::CommonDying(task)) if task.elapsed_ms() == 20)
    );
    assert!(
        !entity
            .actor_tasks
            .wrapper_flags(task_id)
            .unwrap()
            .in_callback
    );
    let runtime_after = runtime.clone();
    let before_next_word = fx
        .fork_for_main_base_abort_transaction()
        .next_shared_retail_random_u16();
    let next = crate::intro2_common_dying::tick_intro2_common_dying(
        &mut manager,
        tick.retained_owner.unwrap(),
        crate::intro2_common_dying::Intro2CommonDyingFrame {
            resources: &session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            retail_tick: 2,
        },
    );
    assert!(matches!(
        next.outcome,
        crate::intro2_common_dying::Intro2CommonDyingOutcome::Pending { .. }
    ));
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .native_type30_runtime
            .as_ref(),
        Some(&runtime_after)
    );
    assert_eq!(fx.next_shared_retail_random_u16(), before_next_word);
}

#[v2k_test_support::retail_test]
fn type30_class12_coarse_tag_skips_kl_but_runs_world_tail_after_unwind() {
    let (session, mut manager, id, mut fx, owner, before) = prepare_dying_fixture();
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.state_flags_at_0x08.overwrite(
        crate::entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
        0,
    );
    entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(250_000);
    entity.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(125_001);
    let tick = crate::intro2_common_dying::tick_intro2_common_dying(
        &mut manager,
        owner,
        crate::intro2_common_dying::Intro2CommonDyingFrame {
            resources: &session.cache,
            world_fx: &mut fx,
            elapsed_micros: 0,
            retail_tick: 1,
        },
    );
    assert!(
        matches!(
            tick.outcome,
            crate::intro2_common_dying::Intro2CommonDyingOutcome::Advanced {
                detailed: false,
                terminal: true,
                ..
            }
        ),
        "{:?}",
        tick.outcome
    );
    assert!(tick.retained_owner.is_none());
    assert_eq!(manager.pending_actor_deferred_destroy_ids(), [id]);
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.native_type30_runtime.as_ref(), Some(&before));
    assert_eq!(
        entity.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(0)
    );
    assert!(entity.actor_task_state(ActorTaskSlot::Primary).is_none());
}

#[v2k_test_support::retail_test]
fn native_type30_particle_hit_reselects_each_living_style_with_exact_rng_and_receipt() {
    use crate::intro2_radial::Intro2RadialTaskCustody;
    use crate::{
        damage::{DamagePacket, FUN_0043F780_DAMAGE_PACKET, FUN_0043F7C0_DAMAGE_PACKET},
        gameplay_notifications::GameplayNotifications,
        native_ground_actor::impact::{
            apply_native_ground_particle_hit, NativeGroundImpactOutcome,
        },
        shared_actor_impact::SharedActorImpactFrame,
        specialized_actor_task_production::SpecializedActorTaskScheduler,
        world_fx::{BallisticDamageRequest, ParticleEntityImpact},
    };
    let cured_class = (0..=u8::MAX)
        .find(|&class| crate::world_fx::particle_uses_fun_0043f7c0_entity_hit(class))
        .unwrap();
    for class in [5, 7, 26] {
        for (particle_class, packet) in [
            (
                16,
                DamagePacket {
                    channels: [2, 0],
                    amounts_raw: [2000, 0],
                },
            ),
            (5, FUN_0043F780_DAMAGE_PACKET),
            (cured_class, FUN_0043F7C0_DAMAGE_PACKET),
        ] {
            let (session, metadata, mut manager, id) = native_actor_fixture();
            // This phase test supplies a complete immutable ballistic birth
            // record. The source actor may be inactive by impact time; the
            // consumer must retain these words instead of rereading that body.
            let birth_source = manager.iter_all().find(|entity| entity.id != id).unwrap();
            let source_entity_type_at_birth = u8::try_from(birth_source.entity_type).unwrap();
            let source_owner_id_at_birth = birth_source.id;
            let others: Vec<_> = manager
                .iter_all()
                .filter(|e| e.id != id)
                .map(|e| e.id)
                .collect();
            for other in others {
                manager.entity_mut(other).unwrap().active = false;
            }
            let entity = manager.entity_mut(id).unwrap();
            entity
                .collision
                .state_flags_at_0x08
                .overwrite(u32::MAX, 0x68000);
            let selection = tasks::select(class == 7, &mut || match class {
                5 => 0,
                7 => 10000,
                26 => 65535,
                _ => unreachable!(),
            })
            .unwrap();
            assert_eq!(selection.program.class_id, class);
            assert!(tasks::publish_acquiring(
                entity,
                &metadata[30],
                selection,
                BehaviorContextRuntime::from_fresh_weighted_selection(selection).unwrap(),
                &mut || 0
            ));
            let old_primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
            let bank_before = entity.native_type30_runtime.clone();
            let mut scheduler = SpecializedActorTaskScheduler::new();
            scheduler.register_type30(Type30Owner::adopt(&manager, id).unwrap());
            let mut fx = WorldFx::new();
            let mut control = WorldFx::new();
            let selector = u32::from(control.next_shared_retail_random_u16());
            let expected_selection = tasks::select(false, &mut || selector).unwrap();
            control.next_shared_retail_random_u16();
            let impact = ParticleEntityImpact {
                source_particle_class: particle_class,
                impact_position_argument_va: 0,
                target_entity_id: id,
                position_world: [0.0; 3],
                velocity_raw: [0; 3],
                damage: Some(BallisticDamageRequest {
                    packet,
                    source_entity_type_at_birth: Some(source_entity_type_at_birth),
                    source_owner_id: Some(source_owner_id_at_birth),
                }),
            };
            let result = apply_native_ground_particle_hit::<profile::Type30Profile>(
                SharedActorImpactFrame {
                    entities: &mut manager,
                    resources: &session.cache,
                    world_fx: &mut fx,
                    scheduler: &mut scheduler,
                    notifications: &mut GameplayNotifications::new(),
                    retail_tick: 251,
                },
                impact,
            );
            let NativeGroundImpactOutcome::Applied(result) = result else {
                panic!("class{class} particle{particle_class}: {result:?}")
            };
            assert_eq!(result.filtered_damage_raw, 0);
            assert!(result.death_publication.is_none());
            let entity = manager.entity_mut(id).unwrap();
            assert_ne!(
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                old_primary
            );
            assert_eq!(entity.native_type30_runtime, bank_before);
            assert_eq!(
                entity.collision.health_raw,
                RetailRuntimeValue::Known(12_000)
            );
            let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
                panic!("context")
            };
            assert_eq!(
                context.active_style().style_address(),
                expected_selection.program.initial_style.frame_address
            );
            assert_eq!(
                fx.next_shared_retail_random_u16(),
                control.next_shared_retail_random_u16(),
                "one selector and one chosen5/26 suffix; no disabled impulse RNG"
            );
            assert!(scheduler.prepare_native_actor_mutation(&manager, id));
        }
    }
}
