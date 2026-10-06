use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    intro2_radial::Intro2RadialTaskCustody,
    intro2_type17::capture::{CaptureChildFrame, CaptureChildOperation, CaptureTaskCustody},
    main_base_type9_abort::MainBaseType9ResultScreenState,
    shared_actor_impact::type9_tests::{acquired_attract, initial_attract, World},
    static_damage::StaticDamageScheduler,
};

fn actor(world: &World, id: u32) -> &crate::entity::Entity {
    world
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap()
}

fn actor_snapshot(world: &World, id: u32) -> String {
    let entity = actor(world, id);
    let tasks = ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
        let id = entity.actor_tasks.task_in_slot(slot);
        (
            id,
            entity.actor_task_state(slot).copied(),
            id.and_then(|id| entity.actor_tasks.wrapper_flags(id)),
        )
    });
    format!(
        "{:?}",
        (
            tasks,
            entity.current_behavior_context,
            entity.collision.state_flags_at_0x08,
            entity.collision.health_raw,
            entity.position_raw(),
            entity.velocity_raw(),
            entity.rotation_heading_pitch_roll_raw(),
            entity.physical_body_basis_q31(),
            entity.sub_a_propulsion_runtime,
            &entity.actor_animation_runtime,
        )
    )
}

fn inner_snapshot(scheduler: &SpecializedActorTaskScheduler) -> String {
    let SpecializedActorTaskOwner::NativeContactPrefix { retained, .. } = &scheduler.owners[0]
    else {
        panic!()
    };
    format!("{retained:?}")
}

fn assert_external_denied(world: &mut World, id: u32) {
    assert!(world
        .scheduler
        .begin_native_type9_external_mutation(&world.entities, id)
        .is_none());
    assert!(!world
        .scheduler
        .prepare_native_actor_mutation(&world.entities, id));
    assert!(!world
        .scheduler
        .capture_child_mutation_ready(&world.entities, id));
    assert!(world
        .scheduler
        .prepare_type9_cargo_attach(&world.entities, id)
        .is_none());
    let lease = world
        .entities
        .main_base_abort_actor_observation(id)
        .unwrap()
        .lease;
    assert!(world
        .scheduler
        .take_ordinary_type9_selected_for_main_base_abort(lease)
        .is_err());
    for visited in [false, true] {
        let mut pending = Vec::new();
        let mut retained = Vec::new();
        if visited {
            retained = std::mem::take(&mut world.scheduler.owners);
        } else {
            pending = std::mem::take(&mut world.scheduler.owners);
        }
        let remaining = if visited { Vec::new() } else { vec![id] };
        {
            let mut cursor = Intro2RadialCursorCustody {
                pending: &mut pending,
                retained: &mut retained,
                remaining_live_ids: &remaining,
            };
            assert!(!cursor.prepare_native_actor_mutation(&world.entities, id));
            assert!(!cursor.capture_child_mutation_ready(&world.entities, id));
        }
        assert_eq!(
            (pending.len(), retained.len()),
            if visited { (0, 1) } else { (1, 0) }
        );
        world.scheduler.owners = if visited { retained } else { pending };
    }
}

fn check_retention(mut world: World, id: u32, dying: bool) {
    if dying {
        world
            .scheduler
            .mutate_capture_child(
                &mut world.entities,
                CaptureChildFrame {
                    child: id,
                    operation: CaptureChildOperation::StandardDeath,
                    world_fx: &mut world.fx,
                    notifications: &mut world.notifications,
                    retail_tick: world.tick,
                    result_screen: MainBaseType9ResultScreenState::NotShown,
                },
                &mut |_| Ok(()),
            )
            .unwrap();
        assert_eq!(
            world.scheduler.family_for(id),
            Some(SpecializedActorTaskFamily::Intro2Type9Class14)
        );
    }
    // Isolate the actual actor's task visit so unrelated native owners cannot
    // consume the process RNG while this regression measures a parked visit.
    world
        .scheduler
        .owners
        .retain(|owner| owner.entity_id() == id);
    assert_eq!(world.scheduler.owners.len(), 1);
    let original_owner = format!("{:?}", world.scheduler.owners[0]);
    let family = world.scheduler.owners[0].family();
    let before = actor_snapshot(&world, id);
    let next_word = world
        .fx
        .fork_for_main_base_abort_transaction()
        .next_shared_retail_random_u16();
    let claim = world
        .entities
        .main_base_abort_actor_observation(id)
        .unwrap()
        .lease;

    assert!(world
        .scheduler
        .park_native_contact_prefix(&world.entities, id));
    assert!(world
        .scheduler
        .park_native_contact_prefix(&world.entities, id));
    assert_eq!(
        inner_snapshot(&world.scheduler),
        original_owner,
        "parking moves, never reconstructs, linear custody"
    );
    assert_external_denied(&mut world, id);
    assert_eq!(
        world.scheduler.actor_animation_claims().collect::<Vec<_>>(),
        vec![claim]
    );
    assert_eq!(
        world
            .scheduler
            .adopt_fresh_level1_type9_selected(&mut world.entities)
            .unwrap(),
        0
    );
    assert_eq!(world.scheduler.adopt_intro2_type9(&mut world.entities), 0);
    let replacement = match &world.scheduler.owners[0] {
        SpecializedActorTaskOwner::NativeContactPrefix { retained, .. } => {
            retained.fork_for_main_base_abort_transaction()
        }
        _ => unreachable!(),
    };
    assert!(
        world.scheduler.register(replacement).is_err(),
        "same-family registration cannot restart a parked prefix"
    );

    // A speculative abort keeps the same stop in its isolated scheduler. It
    // cannot consume the retained source owner's continuation or unpark live.
    let forked_manager = world.entities.fork_for_main_base_abort_transaction();
    let mut forked = world.scheduler.fork_for_main_base_abort_transaction();
    assert!(!forked.prepare_native_actor_mutation(&forked_manager, id));
    assert!(forked
        .begin_native_type9_external_mutation(&forked_manager, id)
        .is_none());
    assert!(forked
        .take_ordinary_type9_selected_for_main_base_abort(claim)
        .is_err());
    drop(forked);

    for _ in 0..2 {
        let pass = world.scheduler.tick(
            &mut world.entities,
            SpecializedActorTaskProductionFrame {
                world:
                    crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut world.session.cache,
                world_fx: &mut world.fx,
                static_damage: &mut StaticDamageScheduler::new(),
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: world.tick,
                main_base_abort_active: false,
            },
            &mut world.notifications,
        );
        assert_eq!(pass.block, None);
        assert_eq!(
            pass.outcomes,
            vec![
                SpecializedActorTaskProductionOutcome::NativeContactPrefixBlocked {
                    entity_id: id,
                    family
                }
            ]
        );
        assert_eq!(actor_snapshot(&world, id), before);
        assert_eq!(inner_snapshot(&world.scheduler), original_owner);
        assert_eq!(
            world
                .fx
                .fork_for_main_base_abort_transaction()
                .next_shared_retail_random_u16(),
            next_word
        );
        assert_external_denied(&mut world, id);
    }
}

#[v2k_test_support::retail_test]
fn parked_native_type9_initial_attract_keeps_linear_candidate_and_cue() {
    let (world, id) = initial_attract();
    check_retention(world, id, false);
}

#[v2k_test_support::retail_test]
fn parked_native_type9_target_route_keeps_completed_continuation() {
    let (world, id) = acquired_attract();
    check_retention(world, id, false);
}

#[v2k_test_support::retail_test]
fn parked_native_type9_class14_keeps_corpse_task_and_animation_custody() {
    let (world, id) = initial_attract();
    check_retention(world, id, true);
}

#[v2k_test_support::retail_test]
fn foreign_manager_generation_cannot_park_an_existing_native_owner() {
    let (mut world, id) = initial_attract();
    let (foreign, foreign_id) = initial_attract();
    assert_eq!(
        id, foreign_id,
        "controlled worlds reuse ids but not allocation generations"
    );
    world
        .scheduler
        .owners
        .retain(|owner| owner.entity_id() == id);
    let before = format!("{:?}", world.scheduler.owners);
    assert!(!world
        .scheduler
        .park_native_contact_prefix(&foreign.entities, id));
    assert_eq!(format!("{:?}", world.scheduler.owners), before);
    assert!(world
        .scheduler
        .park_native_contact_prefix(&world.entities, id));
}

#[v2k_test_support::retail_test]
fn native_worker_spider_and_soldier_keep_their_exact_contact_custody() {
    use crate::intro2_type17::pair::tests::{ChildOrder, Fixture};
    let mut fixture = Fixture::ready(50, None, ChildOrder::First);
    fixture
        .scheduler
        .adopt_intro2_type47_guards(&fixture.entities);
    let ids = [8, 17, 47].map(|kind| {
        fixture
            .entities
            .iter_all()
            .find(|entity| entity.entity_type == kind)
            .unwrap()
            .id
    });
    fixture
        .scheduler
        .owners
        .retain(|owner| ids.contains(&owner.entity_id()));
    assert_eq!(fixture.scheduler.owners.len(), 3);
    let before = fixture
        .scheduler
        .owners
        .iter()
        .map(|owner| (owner.entity_id(), format!("{owner:?}")))
        .collect::<Vec<_>>();
    for id in ids {
        assert!(fixture
            .scheduler
            .park_native_contact_prefix(&fixture.entities, id));
        assert!(!fixture
            .scheduler
            .prepare_native_actor_mutation(&fixture.entities, id));
    }
    let worker_lease = fixture
        .entities
        .main_base_abort_actor_observation(ids[0])
        .unwrap()
        .lease;
    assert_eq!(
        fixture
            .scheduler
            .actor_animation_claims()
            .collect::<Vec<_>>(),
        vec![worker_lease]
    );
    // Other real bodies omitted from this isolated scheduler may be
    // adoptable; no adopter may replace these three parked allocations.
    fixture.scheduler.adopt_intro2_type8(&mut fixture.entities);
    fixture.scheduler.adopt_intro2_type17(&fixture.entities);
    fixture
        .scheduler
        .adopt_intro2_type47_guards(&fixture.entities);
    for (id, expected) in before {
        let wrapped = fixture
            .scheduler
            .owners
            .iter()
            .find(|owner| owner.entity_id() == id)
            .unwrap();
        let SpecializedActorTaskOwner::NativeContactPrefix { retained, .. } = wrapped else {
            panic!()
        };
        assert_eq!(format!("{retained:?}"), expected);
        assert!(!fixture
            .scheduler
            .prepare_native_actor_mutation(&fixture.entities, id));
    }
}

#[derive(Debug, PartialEq, Eq)]
enum HitAdmission {
    NotApplicable,
    Applied,
    Blocked { committed_prefix: bool },
}

fn corpse_hit(
    f: &mut crate::intro2_type17::pair::tests::Fixture,
    kind: u32,
    id: u32,
    infected: bool,
) -> HitAdmission {
    use crate::{
        intro2_type16::impact::{apply_intro2_type16_particle_hit, Intro2Type16ImpactOutcome},
        intro2_type26_defecate_virus::{
            apply_intro2_type26_particle_hit, Intro2Type26ImpactOutcome,
        },
        intro2_type58::impact::{apply_intro2_type58_particle_hit, Intro2Type58ImpactOutcome},
        world_fx::{BallisticDamageRequest, ParticleEntityImpact},
    };
    let impact = ParticleEntityImpact {
        source_particle_class: if infected { 5 } else { 1 },
        impact_position_argument_va: if infected { 0x004d_cf48 } else { 0 },
        target_entity_id: id,
        position_world: [0.; 3],
        velocity_raw: [0, 0, 8192],
        damage: Some(BallisticDamageRequest {
            packet: if infected {
                crate::damage::FUN_0043F780_DAMAGE_DELIVERY.packet
            } else {
                crate::damage::DamagePacket {
                    channels: [2, 0],
                    amounts_raw: [0, 0],
                }
            },
            source_entity_type_at_birth: Some(34),
            source_owner_id: Some(35),
        }),
    };
    match kind {
        16 => match apply_intro2_type16_particle_hit(
            &mut f.entities,
            &f.session.cache,
            &mut f.fx,
            &mut f.scheduler,
            impact,
            f.tick,
        ) {
            Intro2Type16ImpactOutcome::NotApplicable => HitAdmission::NotApplicable,
            Intro2Type16ImpactOutcome::Applied(_) => HitAdmission::Applied,
            Intro2Type16ImpactOutcome::Blocked {
                committed_prefix, ..
            } => HitAdmission::Blocked { committed_prefix },
        },
        26 => match apply_intro2_type26_particle_hit(
            &mut f.entities,
            &f.session.cache,
            &mut f.fx,
            &mut f.scheduler,
            impact,
            f.tick,
        ) {
            Intro2Type26ImpactOutcome::NotApplicable => HitAdmission::NotApplicable,
            Intro2Type26ImpactOutcome::Applied(_) => HitAdmission::Applied,
            Intro2Type26ImpactOutcome::Blocked {
                committed_prefix, ..
            } => HitAdmission::Blocked { committed_prefix },
        },
        58 => match apply_intro2_type58_particle_hit(
            crate::shared_actor_impact::SharedActorImpactFrame {
                entities: &mut f.entities,
                resources: &f.session.cache,
                world_fx: &mut f.fx,
                scheduler: &mut f.scheduler,
                notifications: &mut f.notifications,
                retail_tick: f.tick,
            },
            impact,
        ) {
            Intro2Type58ImpactOutcome::NotApplicable => HitAdmission::NotApplicable,
            Intro2Type58ImpactOutcome::Applied(_) => HitAdmission::Applied,
            Intro2Type58ImpactOutcome::Blocked {
                committed_prefix, ..
            } => HitAdmission::Blocked { committed_prefix },
        },
        _ => unreachable!(),
    }
}

#[v2k_test_support::retail_test]
fn parked_common_dying_blocks_primary_and_infected_prefixes_without_suppressing_real_noops() {
    use crate::intro2_type17::pair::tests::{ChildOrder, Fixture};
    let mut f = Fixture::ready(50, None, ChildOrder::First);
    for kind in [16, 26, 58] {
        let id = f
            .entities
            .iter_all()
            .find(|entity| entity.entity_type == kind)
            .unwrap()
            .id;
        let owner = crate::intro2_common_dying::publish_intro2_common_standard_death(
            &mut f.entities,
            id,
            &mut f.fx,
        )
        .unwrap()
        .unwrap();
        f.scheduler.register_intro2_common_dying(owner);
        f.tick += 1;
        assert_eq!(
            corpse_hit(&mut f, kind, id, false),
            HitAdmission::Applied,
            "unparked class12's null style hook and zero-damage packet remain valid"
        );
        assert_eq!(
            f.entities.entity_mut(id).unwrap().collision.health_raw,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            f.entities
                .entity_mut(id)
                .unwrap()
                .collision
                .last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(f.tick)
        );
        f.fx.process_pending();
        f.fx.take_positional_sounds();
        assert!(f.scheduler.park_native_contact_prefix(&f.entities, id));
        let snapshot = |f: &Fixture| {
            let entity = f
                .entities
                .iter_all()
                .find(|entity| entity.id == id)
                .unwrap();
            format!(
                "{:?}",
                (
                    entity.collision.state_flags_at_0x08,
                    entity.collision.health_raw,
                    entity.collision.last_hit_presentation_tick_at_0x34,
                    entity.position_raw(),
                    entity.velocity_raw(),
                    entity.rotation_heading_pitch_roll_raw(),
                    entity.current_behavior_context,
                    entity.actor_task_state(ActorTaskSlot::Primary)
                )
            )
        };
        let before = snapshot(&f);
        let rng =
            f.fx.fork_for_main_base_abort_transaction()
                .next_shared_retail_random_u16();
        let particles = f.fx.particle_count();
        for _ in 0..2 {
            for infected in [false, true] {
                f.tick += 1;
                assert_eq!(
                    corpse_hit(&mut f, kind, id, infected),
                    HitAdmission::Blocked {
                        committed_prefix: false
                    }
                );
                assert_eq!(snapshot(&f), before, "type={kind}, infected={infected}");
                assert_eq!(
                    f.fx.fork_for_main_base_abort_transaction()
                        .next_shared_retail_random_u16(),
                    rng
                );
                assert_eq!(f.fx.particle_count(), particles);
                assert_eq!(f.fx.pending_event_count(), 0);
                assert!(f.fx.take_positional_sounds().is_empty());
            }
        }
        let unrelated = f.spider;
        assert_eq!(
            corpse_hit(&mut f, kind, unrelated, false),
            HitAdmission::NotApplicable,
            "the exact family/target no-op remains before hit custody"
        );
    }
}
