//! External hits may consume only a completed native class45 visit.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskId, ActorTaskSlot, ActorTaskWrapperFlags},
    attract_attention::ATTRACT_ATTENTION_INITIAL_STYLE_ADDRESS,
    damage::DamagePacket,
    entity::{Entity, EntityManager},
    entity_collision_state::{RetailRuntimeValue, RetailStateWord},
    entity_view_detail::RetailViewDetailContext,
    guard_location_owner::acquisition::GuardLocationEntityRef,
    impact_reaction::{IMPACT_REACTION_ENABLED_STATE_BIT, IMPACT_REACTION_SUPPRESSED_STATE_BIT},
    ordinary_type9_attract_attention_production::{
        OrdinaryType9AttractAttentionProductionBlock, OrdinaryType9AttractAttentionProductionState,
    },
    ordinary_type9_impact::{apply_native_type9_particle_hit, NativeType9ImpactOutcome},
    ordinary_type9_root_reselection::{
        plan_ordinary_type9_root_reselection, OrdinaryType9RootReselectionRequest,
        OrdinaryType9RootSelection,
    },
    shared_actor_impact::type9_tests::{acquired_attract, initial_attract, World},
    world_fx::{BallisticDamageRequest, ParticleEntityImpact},
};

fn actor(manager: &EntityManager, id: u32) -> &Entity {
    manager.iter_all().find(|entity| entity.id == id).unwrap()
}

fn tasks(
    entity: &Entity,
) -> [(
    Option<ActorTaskId>,
    Option<ActorTaskRuntime>,
    Option<ActorTaskWrapperFlags>,
); 3] {
    ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
        let id = entity.actor_tasks.task_in_slot(slot);
        (
            id,
            entity.actor_task_state(slot).copied(),
            id.and_then(|id| entity.actor_tasks.wrapper_flags(id)),
        )
    })
}

fn take_attract(world: &mut World, id: u32) -> OrdinaryType9AttractAttentionProductionOwner {
    let index = world
        .scheduler
        .owners
        .iter()
        .position(|owner| owner.entity_id() == id)
        .unwrap();
    let SpecializedActorTaskOwner::OrdinaryType9AttractAttention(owner) =
        world.scheduler.owners.remove(index)
    else {
        panic!("the actual class45 constructor or handoff must retain its Attract owner");
    };
    owner
}

fn present(world: &mut World) {
    let player = world.entities.player().unwrap().position;
    world.scheduler.publish_presented_view_detail(
        &mut world.entities,
        RetailViewDetailContext::from_world(
            [player[0], player[1] + 8., player[2] - 8.],
            0.7,
            (52, 30),
        ),
    );
}

#[v2k_test_support::retail_test]
fn completed_initial_class45_hit_custody_keeps_candidate_cue_clocks_and_next_visit() {
    let (mut world, id) = initial_attract();
    present(&mut world);
    let before = tasks(actor(&world.entities, id));
    let owner = take_attract(&mut world, id);
    let tick = tick_ordinary_type9_attract_attention_owner_with_random(
        &mut world.entities,
        owner,
        OrdinaryType9AttractAttentionProductionFrame {
            resources: &world.session.cache,
            retail_tick: world.tick,
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
            candidates_in_intrusive_order: &[],
            dispatch_resource_text: &|_, _| {},
        },
        &mut world.fx,
        // The real Candidate one-in-four gate rejects this word, preserving
        // its actual initializer lease and reaching the Cue callback.
        |_| 1,
    );
    assert!(
        matches!(
            tick.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::OuterTailComplete { .. }
        ),
        "{:?}",
        tick.outcome
    );
    let owner = tick.retained_owner.unwrap();
    assert!(matches!(
        owner.state(),
        OrdinaryType9AttractAttentionProductionState::PostBasisTailPending { .. }
    ));
    assert!(world
        .scheduler
        .register_ordinary_type9_attract_attention(owner)
        .unwrap()
        .is_none());
    let completed = tasks(actor(&world.entities, id));
    assert_eq!(before.map(|(id, _, _)| id), completed.map(|(id, _, _)| id));
    assert!(
        matches!(completed[2].1, Some(ActorTaskRuntime::AttractAttentionCue(cue)) if cue.elapsed_ms() == 20)
    );

    let first = world
        .scheduler
        .begin_native_type9_external_mutation(&world.entities, id)
        .unwrap();
    assert!(
        matches!(first, NativeType9HitCustody::Selected { next_transaction_id, .. } if next_transaction_id > 1)
    );
    assert_eq!(
        tasks(actor(&world.entities, id)),
        completed,
        "custody transfer consumes no task time"
    );
    assert_eq!(
        world
            .scheduler
            .begin_native_type9_external_mutation(&world.entities, id),
        Some(first)
    );
    assert_eq!(
        tasks(actor(&world.entities, id)),
        completed,
        "repeated null hits retain both linear callbacks"
    );

    // Production resumes the transferred graph, including the actual Cue or
    // Candidate-to-TargetRoute handoff; no task/context is synthesized here.
    world.tick += 1;
    world.step(20_000);
    assert!(world
        .scheduler
        .begin_native_type9_external_mutation(&world.entities, id)
        .is_some());
}

#[v2k_test_support::retail_test]
fn completed_target_route_transfers_repeatedly_without_restarting_its_clock() {
    let (mut world, id) = acquired_attract();
    let before = tasks(actor(&world.entities, id));
    assert!(matches!(
        before[0].1,
        Some(ActorTaskRuntime::AttractAttentionTargetRoute(_))
    ));
    let first = world
        .scheduler
        .begin_native_type9_external_mutation(&world.entities, id)
        .unwrap();
    assert_eq!(
        world
            .scheduler
            .begin_native_type9_external_mutation(&world.entities, id),
        Some(first)
    );
    assert_eq!(tasks(actor(&world.entities, id)), before);
    world.step(20_000);
    assert!(world
        .scheduler
        .begin_native_type9_external_mutation(&world.entities, id)
        .is_some());
}

#[v2k_test_support::retail_test]
fn external_c690_class45_publication_survives_nonzero_reaction_and_next_callback() {
    let (mut world, id) = acquired_attract();
    // Consume the previous completed observation before configuring this
    // controlled reaction case. Allocation, class45 and all task leases are
    // still those constructed and acquired by the canonical live scheduler.
    assert!(world
        .scheduler
        .begin_native_type9_external_mutation(&world.entities, id)
        .is_some());
    world
        .entities
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(
            IMPACT_REACTION_ENABLED_STATE_BIT | IMPACT_REACTION_SUPPRESSED_STATE_BIT,
            IMPACT_REACTION_ENABLED_STATE_BIT,
        );
    let candidate = |entity: &Entity| GuardLocationEntityRef {
        id: entity.id,
        entity_type: entity.entity_type,
        position_raw: entity.position_raw(),
        state_flags_raw: entity.collision.state_flags_at_0x08,
        capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
    };
    let candidates: Vec<_> = world
        .entities
        .retail_live_order_ids()
        .map(|id| candidate(actor(&world.entities, id)))
        .collect();
    // Find a reachable process cursor using the actual weighted selector.
    // No root plan, selected style, task or random seed is installed by hand.
    let mut class45_next = false;
    for _ in 0..128 {
        let mut predicted_fx = world.fx.fork_for_main_base_abort_transaction();
        let entity = actor(&world.entities, id);
        let plan = plan_ordinary_type9_root_reselection(
            OrdinaryType9RootReselectionRequest {
                active_model_id: entity.model_index.unwrap(),
                metadata: world.entities.type_runtime_metadata(9).unwrap(),
                owner: candidate(entity),
                current_context: entity.current_behavior_context,
                candidates_in_intrusive_order: &candidates,
            },
            || u32::from(predicted_fx.next_shared_retail_random_u16()),
        )
        .unwrap();
        if matches!(plan.selection(), OrdinaryType9RootSelection::Weighted { selection, .. } if selection.program.class_id == 45)
        {
            class45_next = true;
            break;
        }
        world.fx.next_shared_retail_random_u16();
    }
    assert!(class45_next, "nearby actual player leaves class45 eligible");
    let before = actor(&world.entities, id);
    let old_tasks = tasks(before);
    let old_velocity = before.velocity_raw();
    let old_angles = before.rotation_heading_pitch_roll_raw();
    let old_health = before.collision.health_raw;
    let impact = ParticleEntityImpact {
        source_particle_class: 16,
        impact_position_argument_va: 0,
        target_entity_id: id,
        position_world: before.position,
        velocity_raw: [700, -200, 900],
        damage: Some(BallisticDamageRequest {
            packet: DamagePacket {
                channels: [2, 0],
                amounts_raw: [1_000, 0],
            },
            source_entity_type_at_birth: Some(46),
            source_owner_id: Some(1),
        }),
    };
    let outcome = apply_native_type9_particle_hit(
        &mut world.entities,
        &mut world.fx,
        &mut world.scheduler,
        &mut world.notifications,
        impact,
        world.tick,
    );
    assert!(
        matches!(outcome, NativeType9ImpactOutcome::Applied(ref result)
        if result.filtered_damage_raw > 0 && result.death_publication.is_none()),
        "{outcome:?}"
    );
    let entity = actor(&world.entities, id);
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        panic!("current context");
    };
    assert_eq!(
        context.active_style().style_address(),
        ATTRACT_ATTENTION_INITIAL_STYLE_ADDRESS
    );
    assert_ne!(entity.velocity_raw(), old_velocity);
    assert_ne!(entity.rotation_heading_pitch_roll_raw(), old_angles);
    assert_ne!(entity.collision.health_raw, old_health);
    assert_eq!(
        entity.collision.last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(world.tick)
    );
    let published_tasks = tasks(entity);
    assert_ne!(
        published_tasks.map(|(id, _, _)| id),
        old_tasks.map(|(id, _, _)| id)
    );

    world.tick += 1;
    let outcome = apply_native_type9_particle_hit(
        &mut world.entities,
        &mut world.fx,
        &mut world.scheduler,
        &mut world.notifications,
        impact,
        world.tick,
    );
    assert!(
        matches!(outcome, NativeType9ImpactOutcome::Applied(_)),
        "{outcome:?}"
    );
    assert_eq!(
        tasks(actor(&world.entities, id)),
        published_tasks,
        "initial null hook keeps the newly published callbacks"
    );
    world.tick += 1;
    world.step(20_000);
}

#[v2k_test_support::retail_test]
fn pending_target_route_callback_keeps_its_committed_prefix_and_rejects_hit_custody() {
    let (mut world, id) = acquired_attract();
    present(&mut world);
    let before = actor(&world.entities, id);
    let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
        before.actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("actual target route");
    };
    let target_id = route.target_id().unwrap();
    let route_elapsed = route.elapsed_ms();
    let RetailRuntimeValue::Known(relation_elapsed) =
        before.collision.recent_relation_elapsed_us_at_0x68
    else {
        panic!("native scheduler prefix");
    };
    let saved_target_state = actor(&world.entities, target_id)
        .collision
        .state_flags_at_0x08;
    // A missing live observation is encountered inside the genuine callback,
    // after it advances its clock. The task graph and native receipt stay real.
    world
        .entities
        .entity_mut(target_id)
        .unwrap()
        .collision
        .state_flags_at_0x08 = RetailStateWord::unknown();
    let owner = take_attract(&mut world, id);
    let tick = tick_ordinary_type9_attract_attention_owner_with_random(
        &mut world.entities,
        owner,
        OrdinaryType9AttractAttentionProductionFrame {
            resources: &world.session.cache,
            retail_tick: world.tick,
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
            candidates_in_intrusive_order: &[],
            dispatch_resource_text: &|_, _| {},
        },
        &mut world.fx,
        |_| panic!("target-state failure precedes mover and random reads"),
    );
    assert!(
        matches!(
            tick.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::Blocked {
                reason: OrdinaryType9AttractAttentionProductionBlock::TargetRoute(_),
                ..
            }
        ),
        "{:?}",
        tick.outcome
    );
    let owner = tick.retained_owner.unwrap();
    assert_eq!(
        owner.state(),
        OrdinaryType9AttractAttentionProductionState::CallbackFailurePending
    );
    assert!(world
        .scheduler
        .register_ordinary_type9_attract_attention(owner)
        .unwrap()
        .is_none());
    let entity = actor(&world.entities, id);
    assert_eq!(
        entity.collision.recent_relation_elapsed_us_at_0x68,
        RetailRuntimeValue::Known(relation_elapsed.wrapping_add(20_000))
    );
    let prefix = tasks(entity);
    assert!(
        matches!(prefix[0].1, Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) if route.elapsed_ms() == route_elapsed.wrapping_add(20))
    );
    let retained_owner = format!(
        "{:?}",
        world
            .scheduler
            .owners
            .iter()
            .find(|owner| owner.entity_id() == id)
            .unwrap()
    );

    // Restoring the external observation cannot revive a consumed callback.
    world
        .entities
        .entity_mut(target_id)
        .unwrap()
        .collision
        .state_flags_at_0x08 = saved_target_state;
    for _ in 0..2 {
        assert_eq!(
            world
                .scheduler
                .begin_native_type9_external_mutation(&world.entities, id),
            None
        );
        assert_eq!(tasks(actor(&world.entities, id)), prefix);
        assert_eq!(
            format!(
                "{:?}",
                world
                    .scheduler
                    .owners
                    .iter()
                    .find(|owner| owner.entity_id() == id)
                    .unwrap()
            ),
            retained_owner
        );
    }
}
