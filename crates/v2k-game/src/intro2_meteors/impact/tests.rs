use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskSlot, ActorTaskVisit, PreparedActorTask},
    common_mover::type9_attitude::Type9BodyBasis,
    entity_behavior::BehaviorContextRuntime,
    entity_collision_state::{EntityCollisionRuntimeState, RetailStateWord},
    impact_reaction::ImpactReactionSuppression,
    session::GameSession,
    world_fx::BallisticDamageRequest,
};

fn fixture() -> Option<(
    GameSession,
    EntityManager,
    SpecializedActorTaskScheduler,
    u32,
)> {
    let (session, mut manager, _) = crate::intro2_type47_live::world::native_intro2_fixture()?;
    let id = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(33))
        .unwrap()
        .id;
    let entity = manager.entity_mut(id).unwrap();
    // Explicit local, enabled11030 and415040 inputs. Retain the other native
    // known/unknown fields rather than manufacturing a fully resolved actor.
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x8c00_8000, 0x0400_8000);
    entity.collision.health_raw = RetailRuntimeValue::Known(731);
    entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(47);
    entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(19);
    entity.set_rotation_heading_pitch_roll_raw([123, i16::MIN + 7, i16::MAX]);
    // Nonzero retained private history must survive repeated external hits.
    let primary = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    let ActorTaskRuntime::BoulderRolling(task) =
        entity.actor_tasks.task_state_mut(primary).unwrap()
    else {
        panic!()
    };
    task.elapsed_ms = 123;
    task.stationary_visits = 2;
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(scheduler.adopt_intro2_meteors(&manager), 4);
    Some((session, manager, scheduler, id))
}

fn hit(id: u32) -> ParticleEntityImpact {
    ParticleEntityImpact {
        source_particle_class: 5,
        target_entity_id: id,
        impact_position_argument_va: 0,
        position_world: [0.0; 3],
        velocity_raw: [1700, -900, 450],
        damage: Some(BallisticDamageRequest {
            packet: FUN_0043F780_DAMAGE_DELIVERY.packet,
            // F780 uses its static delivery, not these mutable particle
            // fields; this also reproduces the original missing birth type.
            source_entity_type_at_birth: None,
            source_owner_id: Some(6),
        }),
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Snapshot {
    collision: EntityCollisionRuntimeState,
    position: [i16; 3],
    velocity: [i16; 3],
    euler: [i16; 3],
    basis: RetailRuntimeValue<Type9BodyBasis>,
    context: RetailRuntimeValue<Option<BehaviorContextRuntime>>,
    tasks: [Option<ActorTaskRuntime>; 3],
    owner: Option<Intro2MeteorOwner>,
}

fn snapshot(
    manager: &EntityManager,
    scheduler: &SpecializedActorTaskScheduler,
    id: u32,
) -> Snapshot {
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    Snapshot {
        collision: entity.collision.clone(),
        position: entity.position_raw(),
        velocity: entity.velocity_raw(),
        euler: entity.rotation_heading_pitch_roll_raw(),
        basis: entity.physical_body_basis_q31(),
        context: entity.current_behavior_context,
        tasks: [
            ActorTaskSlot::Primary,
            ActorTaskSlot::Secondary,
            ActorTaskSlot::Tertiary,
        ]
        .map(|slot| entity.actor_task_state(slot).copied()),
        owner: scheduler.intro2_meteor_owner(id),
    }
}

fn assert_no_effects(fx: &mut WorldFx) {
    assert_eq!(fx.pending_event_count(), 0);
    assert_eq!(fx.particle_count(), 0);
    assert!(fx.take_positional_sounds().is_empty());
}

#[v2k_test_support::retail_test]
fn native_meteor_f780_repeated_zero_hits_keep_health_buffer_tasks_and_owner() {
    let Some((_session, mut manager, mut scheduler, id)) = fixture() else {
        return;
    };
    let mut expected = snapshot(&manager, &scheduler, id);
    let mut fx = WorldFx::new();
    let mut oracle = WorldFx::new();
    for _ in 0..2 {
        let samples = std::array::from_fn(|_| oracle.next_shared_retail_random_u16());
        let outcome = apply_intro2_meteor_infected_hit(&mut manager, &mut fx, &scheduler, hit(id));
        let Intro2MeteorInfectedHitOutcome::Applied {
            owner,
            reaction: ImpactReactionOutcome::Applied(reaction),
            damage,
        } = outcome
        else {
            panic!("{outcome:?}")
        };
        assert_eq!(Some(owner), expected.owner);
        assert_eq!(reaction.random_samples_heading_roll_pitch_low16, samples);
        assert_eq!(reaction.scale_raw, 0);
        assert_eq!(reaction.linear_delta_xyz_raw, [0; 3]);
        assert_eq!(reaction.angular_delta_heading_pitch_roll_raw, [-0x400; 3]);
        assert_eq!(reaction.network_request, None);
        assert_eq!(
            damage,
            LiveActorDamageOutcome {
                filtered_damage_raw: 0,
                damage_after_buffer_raw: 0,
                death_publication: None
            }
        );
        expected
            .collision
            .state_flags_at_0x08
            .overwrite(0x2000, 0x2000);
        expected.euler = expected.euler.map(|angle| angle.wrapping_sub(0x400));
        assert_eq!(snapshot(&manager, &scheduler, id), expected);
        assert_eq!(
            Intro2MeteorOwner::adopt_published(manager.entity_mut(id).unwrap()),
            Ok(owner)
        );
        assert_eq!(
            scheduler.adopt_intro2_meteors(&manager),
            0,
            "original receipt remains registered"
        );
        assert_no_effects(&mut fx);
    }
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        oracle.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_meteor_f780_disabled_or_suppressed_reaction_skips_mass_and_later_state() {
    let Some((_session, mut manager, scheduler, id)) = fixture() else {
        return;
    };
    for (value, known_mask, suppression) in [
        (
            0,
            0x0400_8000 | DYING_STATE_BIT,
            ImpactReactionSuppression::EnableBitClear,
        ),
        (
            0x0c00_0000,
            0x0c00_8000 | DYING_STATE_BIT,
            ImpactReactionSuppression::SuppressionBitSet,
        ),
    ] {
        let entity = manager.entity_mut(id).unwrap();
        entity.collision.state_flags_at_0x08 = RetailStateWord::from_known_bits(value, known_mask);
        entity.mass_raw = 0;
        let mut expected = snapshot(&manager, &scheduler, id);
        expected
            .collision
            .state_flags_at_0x08
            .overwrite(0x2000, 0x2000);
        let mut fx = WorldFx::new();
        let outcome = apply_intro2_meteor_infected_hit(&mut manager, &mut fx, &scheduler, hit(id));
        assert!(
            matches!(outcome, Intro2MeteorInfectedHitOutcome::Applied { reaction: ImpactReactionOutcome::Suppressed(actual), .. } if actual == suppression),
            "{outcome:?}"
        );
        assert_eq!(snapshot(&manager, &scheduler, id), expected);
        assert_no_effects(&mut fx);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            WorldFx::new().next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn native_meteor_f780_rejects_wrong_packet_profile_or_graph_before_model_prefix() {
    for case in 0..6 {
        let Some((_session, mut manager, mut scheduler, id)) = fixture() else {
            return;
        };
        let mut impact = hit(id);
        match case {
            0 => impact.damage.as_mut().unwrap().packet.amounts_raw[0] += 1,
            1 => {
                manager.entity_mut(id).unwrap().collision.damage_profile =
                    RetailRuntimeValue::Unresolved
            }
            2 => {
                manager.entity_mut(id).unwrap().current_behavior_context =
                    RetailRuntimeValue::Unresolved
            }
            3 => scheduler = SpecializedActorTaskScheduler::new(),
            4 => manager.entity_mut(id).unwrap().authored_spawn_index = Some(32),
            5 => {
                let entity = manager.entity_mut(id).unwrap();
                let replacement = *entity.actor_task_state(ActorTaskSlot::Primary).unwrap();
                entity
                    .actor_tasks
                    .replace_prepared(ActorTaskSlot::Primary, PreparedActorTask::new(replacement));
            }
            _ => unreachable!(),
        }
        let expected = snapshot(&manager, &scheduler, id);
        let mut fx = WorldFx::new();
        let outcome = apply_intro2_meteor_infected_hit(&mut manager, &mut fx, &scheduler, impact);
        assert!(
            matches!(
                outcome,
                Intro2MeteorInfectedHitOutcome::Blocked {
                    committed_prefix: false,
                    ..
                }
            ),
            "case{case}: {outcome:?}"
        );
        assert_eq!(snapshot(&manager, &scheduler, id), expected);
        assert_no_effects(&mut fx);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            WorldFx::new().next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn native_meteor_f780_rejects_executing_wrapper_without_unwinding_it() {
    let Some((_session, mut manager, scheduler, id)) = fixture() else {
        return;
    };
    let owner = scheduler.intro2_meteor_owner(id).unwrap();
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id: owner.primary,
    };
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .actor_tasks
            .begin_exact_visit_with(visit, |_| ()),
        Some(())
    );
    let expected = snapshot(&manager, &scheduler, id);
    let mut fx = WorldFx::new();
    assert_eq!(
        apply_intro2_meteor_infected_hit(&mut manager, &mut fx, &scheduler, hit(id)),
        Intro2MeteorInfectedHitOutcome::Blocked {
            reason: Intro2MeteorInfectedHitBlock::Owner,
            committed_prefix: false
        }
    );
    assert_eq!(snapshot(&manager, &scheduler, id), expected);
    let entity = manager.entity_mut(id).unwrap();
    assert!(
        entity
            .actor_tasks
            .wrapper_flags(owner.primary)
            .unwrap()
            .in_callback
    );
    assert!(entity.actor_tasks.finish_exact_visit(visit));
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        WorldFx::new().next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_meteor_f780_late_network_boundary_retains_reaction_before_checked_damage() {
    let Some((_session, mut manager, scheduler, id)) = fixture() else {
        return;
    };
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(0x8000_0000, 0x8000_0000);
    let mut expected = snapshot(&manager, &scheduler, id);
    expected
        .collision
        .state_flags_at_0x08
        .overwrite(0x2000, 0x2000);
    expected.euler = expected.euler.map(|angle| angle.wrapping_sub(0x400));
    let mut fx = WorldFx::new();
    let mut oracle = WorldFx::new();
    for _ in 0..3 {
        oracle.next_shared_retail_random_u16();
    }
    assert_eq!(
        apply_intro2_meteor_infected_hit(&mut manager, &mut fx, &scheduler, hit(id)),
        Intro2MeteorInfectedHitOutcome::Blocked {
            reason: Intro2MeteorInfectedHitBlock::Runtime("network impact"),
            committed_prefix: true
        }
    );
    assert_eq!(snapshot(&manager, &scheduler, id), expected);
    assert_no_effects(&mut fx);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        oracle.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_meteor_f780_unresolved_checked_gate_keeps_committed_reaction() {
    let Some((_session, mut manager, scheduler, id)) = fixture() else {
        return;
    };
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .invalidate(0x8000);
    let mut expected = snapshot(&manager, &scheduler, id);
    expected
        .collision
        .state_flags_at_0x08
        .overwrite(0x2000, 0x2000);
    expected.euler = expected.euler.map(|angle| angle.wrapping_sub(0x400));
    let mut fx = WorldFx::new();
    let mut oracle = WorldFx::new();
    for _ in 0..3 {
        oracle.next_shared_retail_random_u16();
    }
    let outcome = apply_intro2_meteor_infected_hit(&mut manager, &mut fx, &scheduler, hit(id));
    assert!(
        matches!(
            outcome,
            Intro2MeteorInfectedHitOutcome::Blocked {
                reason: Intro2MeteorInfectedHitBlock::Damage(LiveActorDamageError {
                    phase: crate::live_actor_checked_damage::LiveActorDamagePhase::Admission,
                    committed_prefix: false,
                    ..
                }),
                committed_prefix: true,
            }
        ),
        "{outcome:?}"
    );
    assert_eq!(snapshot(&manager, &scheduler, id), expected);
    assert_no_effects(&mut fx);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        oracle.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_meteor_f780_zero_mass_keeps_only_model_prefix_and_primary_is_not_admitted() {
    let Some((_session, mut manager, scheduler, id)) = fixture() else {
        return;
    };
    manager.entity_mut(id).unwrap().mass_raw = 0;
    let mut expected = snapshot(&manager, &scheduler, id);
    let mut fx = WorldFx::new();
    let mut primary = hit(id);
    primary.source_particle_class = 1;
    assert_eq!(
        apply_intro2_meteor_infected_hit(&mut manager, &mut fx, &scheduler, primary),
        Intro2MeteorInfectedHitOutcome::NotApplicable
    );
    assert_eq!(snapshot(&manager, &scheduler, id), expected);
    assert_eq!(
        apply_intro2_meteor_infected_hit(&mut manager, &mut fx, &scheduler, hit(id)),
        Intro2MeteorInfectedHitOutcome::Blocked {
            reason: Intro2MeteorInfectedHitBlock::Runtime("zero impact mass"),
            committed_prefix: true
        }
    );
    expected
        .collision
        .state_flags_at_0x08
        .overwrite(0x2000, 0x2000);
    assert_eq!(snapshot(&manager, &scheduler, id), expected);
    assert_no_effects(&mut fx);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        WorldFx::new().next_shared_retail_random_u16()
    );
}
