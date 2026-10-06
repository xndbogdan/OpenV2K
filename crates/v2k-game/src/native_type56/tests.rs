use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime as Task,
    actor_task_owner::ActorTaskSlot as Slot,
    common_mover::type9_attitude::Type9BodyBasis,
    entity_collision_state::{DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT},
};

fn request(manager: &EntityManager) -> SplitChildRequest {
    SplitChildRequest {
        requested_entity_handle_raw: 0,
        entity_type: 56,
        position_raw: manager.player().unwrap().position_raw(),
        objective: true,
        velocity_raw: [-2000, -3000, 2000],
        rotation_heading_pitch_roll_raw: [0x9234, 0x8123, 0x7654],
    }
}

fn construct_class(
    manager: &mut EntityManager,
    resources: &ResourceCache,
    fx: &mut WorldFx,
    class: u8,
) -> Type56BirthPublication {
    for _ in 0..128 {
        let birth = request(manager);
        let publication = manager
            .construct_native_type56(birth, resources, fx, 4793)
            .unwrap();
        if publication.publication.selection.program.class_id == class {
            return publication;
        }
    }
    panic!("genuine selector did not reach class{class}");
}

#[v2k_test_support::retail_test]
fn source_acquisition_publishes_type56_chase_aim_and_runaway_graphs() {
    let (session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(19);
    for class in [7, 10] {
        let publication = construct_class(&mut manager, &session.cache, &mut fx, class);
        let id = publication.owner.entity_id();
        let old_secondary = manager
            .entity_mut(id)
            .unwrap()
            .actor_tasks
            .task_in_slot(Slot::Secondary)
            .unwrap();
        shared::behavior::secondary::<profile::Type56Profile>(
            &mut manager,
            id,
            20_000,
            4794,
            &mut fx,
        )
        .unwrap();
        let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
        assert!(entity.actor_tasks.task_in_slot(Slot::Secondary).is_none());
        assert!(entity.actor_tasks.wrapper_flags(old_secondary).is_none());
        assert!(Type56Owner::adopt(&manager, id).is_ok());
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("actual context");
        };
        let RetailRuntimeValue::Known(Some(target)) = context.target_handle_at_0x08() else {
            panic!("actual acquisition target");
        };
        assert_eq!(target, manager.player().unwrap().id);
        if class == 7 {
            assert!(
                matches!(entity.actor_task_state(Slot::Primary), Some(Task::ChaseTarget(task)) if task.target_id() == target)
            );
            assert!(
                matches!(entity.actor_task_state(Slot::Tertiary), Some(Task::AimAndFire(task)) if task.private_state().target_entity_id() == target)
            );
            assert_eq!(
                entity
                    .native_type56_runtime
                    .unwrap()
                    .sub_e_runtime
                    .projectile_method,
                30
            );
        } else {
            assert!(
                matches!(entity.actor_task_state(Slot::Primary), Some(Task::RunAway(task)) if task.target_id() == target)
            );
            assert!(entity.actor_task_state(Slot::Tertiary).is_none());
        }
    }
}

#[v2k_test_support::retail_test]
fn method30_native_fifo_survives_quiet_death_and_materializes_newborn_particle87() {
    use crate::{
        common_mover::component_dispatch::CommonMoverDispatchMode,
        generic_projectile_emitter::GenericEmitterSpeedField, world_fx::ParticleEnvironment,
    };
    let (session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(19);
    let publication = construct_class(&mut manager, &session.cache, &mut fx, 7);
    let id = publication.owner.entity_id();
    let player = manager.player().unwrap().id;
    let target = manager.entity_mut(player).unwrap();
    target.set_position_raw([0, 1000, 1000]);
    target.set_velocity_raw([0; 3]);
    target.collision.state_flags_at_0x08.overwrite(u32::MAX, 4);
    let entity = manager.entity_mut(id).unwrap();
    entity.set_position_raw([0, 1000, 0]);
    entity.set_velocity_raw([100, 0, -200]);
    entity.set_rotation_heading_pitch_roll_raw([0x4000, 0, 0]);
    entity.physical_body_basis_q31 =
        RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(0x4000, 0, 0));
    entity.collision.state_flags_at_0x08.overwrite(u32::MAX, 4);
    shared::behavior::secondary::<profile::Type56Profile>(&mut manager, id, 20_000, 4794, &mut fx)
        .unwrap();
    let metadata = manager.type_runtime_metadata(56).unwrap().clone();
    // Select a passing point in the real process stream; no captured endpoint
    // is installed as native cadence state.
    for _ in 0..65536 {
        let mut peek = fx.fork_for_main_base_abort_transaction();
        if u32::from(peek.next_shared_retail_random_u16()) % (300_000u32 / 125_000u32) == 0 {
            break;
        }
        fx.next_shared_retail_random_u16();
    }
    let outcome = aim::tick_type56_aim(
        CommonMoverDispatchMode::Normal,
        &mut manager,
        &mut fx,
        id,
        125_000,
        Some(&metadata),
    )
    .unwrap();
    assert!(outcome.queued_shots_added > 0, "{outcome:?}");
    let queue = manager
        .entity_mut(id)
        .unwrap()
        .native_type56_aim_runtime
        .clone()
        .unwrap();
    assert!(queue
        .transient_shots()
        .iter()
        .all(|shot| shot.source_handle == id
            && shot.owner_handle == id
            && shot.projectile_method == 30
            && shot.speed_field == GenericEmitterSpeedField::Explicit(3000)));
    death::begin_type56_standard_death(&mut manager, id, &mut fx).unwrap();
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .native_type56_aim_runtime
            .as_ref(),
        Some(&queue)
    );
    let before = fx.particle_count();
    let drained =
        aim::drain_type56_shots(&mut manager, &mut fx, id, ParticleEnvironment::Dry, 4794).unwrap();
    assert_eq!(
        drained.materialized_particle_classes,
        vec![87; queue.queued_shot_count()]
    );
    let particles = fx.test_particles_in_virgin_birth_order();
    assert!(particles[before..]
        .iter()
        .all(|particle| particle.owner_id == Some(id)
            && particle.source_entity_type_at_birth == Some(56)
            && particle.age_ticks == 0.0));
    assert!(
        aim::drain_type56_shots(&mut manager, &mut fx, id, ParticleEnvironment::Dry, 4794)
            .unwrap()
            .materialized_particle_classes
            .is_empty()
    );
}

#[v2k_test_support::retail_test]
fn real_dynamic_constructor_owns_all_three_roots_and_four_ordered_draws() {
    let (session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(19);
    let metadata = manager.type_runtime_metadata(56).unwrap();
    authenticate_metadata(metadata).expect("canonical Type56 ABCDEH metadata");
    assert_eq!(
        metadata.projectile_emitter_descriptor,
        RetailRuntimeValue::Known(Some(EMITTER))
    );
    assert_eq!(
        metadata.terrain_contact_task_lifetime_ms,
        RetailRuntimeValue::Known(0)
    );
    let mut reached = [false; 3];
    for _ in 0..64 {
        let birth = request(&manager);
        let expected_seed = fx.next_sub_d_allocation_seed();
        let mut expected = fx.fork_for_main_base_abort_transaction();
        let words = std::array::from_fn::<_, 4, _>(|_| expected.next_shared_retail_random_u16());
        let publication = manager
            .construct_native_type56(birth, &session.cache, &mut fx, 4793)
            .unwrap();
        assert!(!publication.publication.initializer_fallback);
        assert!(publication.publication.player_nearby);
        assert_eq!(publication.publication.selector_word, u32::from(words[1]));
        let id = publication.owner.entity_id();
        let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
        let runtime = entity.native_type56_runtime.unwrap();
        assert!(manager_allocation_authenticates(&manager, id));
        assert!(entity.authored_spawn_index.is_none());
        assert_eq!(runtime.birth(), birth);
        assert_eq!(
            runtime.sub_d_owner.classifier_cache().stagger_counter(),
            expected_seed
        );
        assert_eq!(entity.velocity_raw(), birth.velocity_raw);
        assert_eq!(entity.position_raw(), runtime.anchor_raw());
        assert_eq!(
            entity.physical_body_basis_q31(),
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(
                0x9234u16 as i16,
                0x8123u16 as i16,
                0x7654
            ))
        );
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(6000));
        assert_eq!(
            entity.collision.pre_health_damage_buffer_raw,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(entity.authored_follow_beacon_priority_raw, Some(0));
        assert_eq!(entity.model_slots, [Some(MODEL); 4]);
        assert_eq!(
            entity.sub_j_attachment_runtime,
            RetailRuntimeValue::Known(None)
        );
        let RetailRuntimeValue::Known(Some(h)) = &entity.sub_h_external_frame_runtime else {
            panic!("own fourteen-row H");
        };
        assert_eq!(h.records().len(), 14);
        assert!(h.is_enabled());
        let RetailRuntimeValue::Known(Some(a)) = entity.sub_a_propulsion_runtime else {
            panic!("actual A");
        };
        assert_eq!(a.direction_multiplier(), 1);
        match publication.publication.selection.program.class_id {
            4 => {
                reached[1] = true;
                assert!(matches!(
                    entity.actor_task_state(Slot::Primary),
                    Some(Task::DefecateVirusWander(_))
                ));
                assert!(entity.actor_task_state(Slot::Secondary).is_none());
                assert!(matches!(
                    entity.actor_task_state(Slot::Tertiary),
                    Some(Task::DefecateVirusTerrain(_))
                ));
                assert_eq!(a.target_speed_raw(), RetailRuntimeValue::Known(1));
            }
            class @ (7 | 10) => {
                reached[if class == 7 { 0 } else { 2 }] = true;
                assert!(matches!(
                    entity.actor_task_state(Slot::Primary),
                    Some(Task::SharedRetarget(_))
                ));
                assert!(
                    matches!(entity.actor_task_state(Slot::Secondary), Some(Task::TargetAcquisition(task)) if task.constructor_filter_override_raw() == 0)
                );
                assert!(entity.actor_task_state(Slot::Tertiary).is_none());
                assert_eq!(
                    a.target_speed_raw(),
                    RetailRuntimeValue::Known(
                        crate::common_mover::shared_initializer_target_speed_raw(600, words[3])
                    )
                );
            }
            other => panic!("unowned root {other}"),
        }
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
        if reached == [true; 3] {
            break;
        }
    }
    assert_eq!(
        reached, [true; 3],
        "all genuine selector branches must execute"
    );
}

#[v2k_test_support::retail_test]
fn dynamic_lease_rejects_authored_borrow_and_foreign_manager() {
    let (session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(19);
    let birth = request(&manager);
    let publication = manager
        .construct_native_type56(birth, &session.cache, &mut fx, 4793)
        .unwrap();
    let id = publication.owner.entity_id();
    let (foreign_session, mut foreign, mut foreign_fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(19);
    let foreign_birth = request(&foreign);
    let foreign_publication = foreign
        .construct_native_type56(foreign_birth, &foreign_session.cache, &mut foreign_fx, 4793)
        .unwrap();
    let issued_runtime = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap()
        .native_type56_runtime;
    foreign
        .entity_mut(foreign_publication.owner.entity_id())
        .unwrap()
        .native_type56_runtime = issued_runtime;
    assert!(!manager_allocation_authenticates(&foreign, id));
    manager.entity_mut(id).unwrap().authored_spawn_index = Some(0);
    assert!(!manager_allocation_authenticates(&manager, id));
    assert_eq!(
        Type56Owner::adopt(&manager, id),
        Err(shared::NativeGroundActorBlock::Allocation)
    );
}

#[v2k_test_support::retail_test]
fn class2_death_commits_once_and_remote_entry_preserves_living_graph() {
    let (session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(19);
    let birth = request(&manager);
    let publication = manager
        .construct_native_type56(birth, &session.cache, &mut fx, 4793)
        .unwrap();
    let id = publication.owner.entity_id();
    let context = manager.entity_mut(id).unwrap().current_behavior_context;
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(REMOTE_OWNED_STATE_BIT, REMOTE_OWNED_STATE_BIT);
    let result = death::begin_type56_standard_death(&mut manager, id, &mut fx).unwrap();
    assert!(!result.returned_nonzero);
    assert!(result.publication.is_none());
    assert_eq!(
        manager.entity_mut(id).unwrap().current_behavior_context,
        context
    );
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(REMOTE_OWNED_STATE_BIT, 0);
    let mut expected = fx.fork_for_main_base_abort_transaction();
    fx.take_positional_sounds();
    let result = death::begin_type56_standard_death(&mut manager, id, &mut fx).unwrap();
    assert!(
        matches!(result.publication, Some(shared::NativeGroundTerminalPublication::Deferred(receipt)) if receipt.allocation() == publication.owner.actor_lease())
    );
    assert!(death::finished_terminal_authenticates(&manager, id));
    assert!(manager.iter_all().any(|entity| entity.id == id));
    assert_eq!(
        manager
            .pending_actor_deferred_destroy_ids()
            .iter()
            .filter(|&&pending| pending == id)
            .count(),
        1
    );
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .masked(DYING_STATE_BIT),
        RetailRuntimeValue::Known(DYING_STATE_BIT)
    );
    fx.take_positional_sounds();
    let second = death::begin_type56_standard_death(&mut manager, id, &mut fx).unwrap();
    assert!(second.returned_nonzero && second.publication.is_none());
    assert!(fx.take_positional_sounds().is_empty());
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn completed_class2_accepts_primary_infected_and_cured_without_reselecting() {
    use crate::{
        damage::{DamagePacket, FUN_0043F780_DAMAGE_DELIVERY, FUN_0043F7C0_DAMAGE_DELIVERY},
        gameplay_notifications::GameplayNotifications,
        specialized_actor_task_production::SpecializedActorTaskScheduler,
        world_fx::{BallisticDamageRequest, ParticleEntityImpact},
    };
    let (session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(19);
    let birth = request(&manager);
    let publication = manager
        .construct_native_type56(birth, &session.cache, &mut fx, 4793)
        .unwrap();
    let id = publication.owner.entity_id();
    let mut tasks = SpecializedActorTaskScheduler::new();
    tasks.register_type56(publication.owner);
    let terminal = death::begin_type56_standard_death(&mut manager, id, &mut fx)
        .unwrap()
        .publication
        .unwrap();
    tasks.register_native_ground_terminal(terminal);
    fx.take_positional_sounds();
    let context = manager.entity_mut(id).unwrap().current_behavior_context;
    let player_id = manager.player().unwrap().id;
    let infected = (0u8..=255)
        .find(|&class| crate::world_fx::particle_uses_fun_0043f780_entity_hit(class))
        .unwrap();
    let cured = (0u8..=255)
        .find(|&class| crate::world_fx::particle_uses_fun_0043f7c0_entity_hit(class))
        .unwrap();
    for (class, packet) in [
        (
            16,
            DamagePacket {
                channels: [2, 0],
                amounts_raw: [9000, 0],
            },
        ),
        (infected, FUN_0043F780_DAMAGE_DELIVERY.packet),
        (cured, FUN_0043F7C0_DAMAGE_DELIVERY.packet),
    ] {
        let result = impact::apply_type56_particle_hit(
            crate::shared_actor_impact::SharedActorImpactFrame {
                resources: &session.cache,
                entities: &mut manager,
                world_fx: &mut fx,
                scheduler: &mut tasks,
                notifications: &mut GameplayNotifications::new(),
                retail_tick: 5000,
            },
            ParticleEntityImpact {
                source_particle_class: class,
                impact_position_argument_va: 0x004d_cf48,
                target_entity_id: id,
                position_world: [0.0; 3],
                velocity_raw: [0; 3],
                damage: Some(BallisticDamageRequest {
                    packet,
                    source_entity_type_at_birth: Some(42),
                    source_owner_id: Some(player_id),
                }),
            },
        );
        let impact::Type56ImpactOutcome::Applied(result) = result else {
            panic!("class={class}: {result:?}");
        };
        assert!(result.death_publication.is_none());
        assert_eq!(
            manager.entity_mut(id).unwrap().current_behavior_context,
            context
        );
        assert!(death::finished_terminal_authenticates(&manager, id));
    }
    assert!(fx.take_positional_sounds().is_empty());
}

#[v2k_test_support::retail_test]
fn nonzero_requested_handle_does_not_borrow_a_captured_native_handle_or_consume_constructor_prefix()
{
    let (session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(19);
    let mut birth = request(&manager);
    birth.requested_entity_handle_raw = 0x12345678;
    let count = manager.iter_all().count();
    let stamp = manager.next_common_body_ordinal();
    let seed = fx.next_sub_d_allocation_seed();
    let mut expected = fx.fork_for_main_base_abort_transaction();
    assert!(matches!(
        manager.construct_native_type56(birth, &session.cache, &mut fx, 4794),
        Err(Type56Block::Runtime("nonzero requested native handle"))
    ));
    assert_eq!(manager.iter_all().count(), count);
    assert_eq!(manager.next_common_body_ordinal(), stamp);
    assert_eq!(fx.next_sub_d_allocation_seed(), seed);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}
