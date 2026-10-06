//! Real ordinary Type58 graphs and authored geometry through late 02CA0/11760.

use super::*;
use crate::{
    actor_task_owner::{ActorTaskId, ActorTaskVisit, ActorTaskWrapperFlags},
    common_mover::{type9_attitude::Type9BodyBasis, SubAPropulsionRuntime},
    entity::EntityManager,
    entity_behavior::{behavior_program, BehaviorContextRuntime, BehaviorSelection},
    entity_collision_state::EntityCollisionRuntimeState,
    follow_beacons::{
        live_acquisition::{
            tick_follow_beacons_live_acquisition, FollowBeaconsLiveAcquisitionOutcome,
            FollowBeaconsLiveAcquisitionRequest,
        },
        static_contact::{
            plan_wander_private_static_contact, FollowBeaconsStaticContactTopology,
            WanderPrivateStaticContactRequest,
        },
    },
    gameplay_notifications::GameplayNotifications,
    intro2_type53::authored_tests::native_fixture,
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskFamily, SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
    sub_h_external_frame::SubHRuntimeState,
    wander_near_location::WanderNearPrivateState,
    world_fx::WorldFx,
};

#[derive(Debug, Clone, Copy)]
enum Graph {
    FollowAcquiring,
    Following,
    SearchAcquiring,
    Chase,
}

impl Graph {
    fn class(self) -> u32 {
        match self {
            Self::FollowAcquiring | Self::Following => 33,
            Self::SearchAcquiring | Self::Chase => 7,
        }
    }
}

struct World {
    session: GameSession,
    manager: EntityManager,
    fx: WorldFx,
    scheduler: SpecializedActorTaskScheduler,
    damage: StaticDamageScheduler,
    notifications: GameplayNotifications,
    id: u32,
    contact: StaticModelContact,
}

impl World {
    fn new(graph: Graph) -> Self {
        let (session, mut manager, mut fx) = native_fixture(14);
        let id = manager
            .iter_all()
            .find(|entity| entity.entity_type == 58)
            .unwrap()
            .id;
        let metadata = manager.type_runtime_metadata(58).unwrap().clone();
        let target_id = manager
            .iter_all()
            .find(|entity| {
                entity.entity_type != 58
                    && !(entity.entity_type == 53 && entity.authored_spawn_index == Some(35))
            })
            .unwrap()
            .id;
        let source_position = manager.entity_mut(id).unwrap().position_raw();
        let ids: Vec<_> = manager.iter_all().map(|entity| entity.id).collect();
        for candidate_id in ids {
            let candidate = manager.entity_mut(candidate_id).unwrap();
            candidate.capability_flags &= !0xd03;
            if candidate.entity_type != 58 {
                candidate
                    .collision
                    .state_flags_at_0x08
                    .overwrite(u32::MAX, 4);
            }
        }
        let target = manager.entity_mut(target_id).unwrap();
        target.capability_flags |= if graph.class() == 33 { 0x100 } else { 1 };
        target.authored_follow_beacon_priority_raw = Some(17);
        target.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        target.set_position_raw([
            source_position[0].wrapping_add(200),
            source_position[1],
            source_position[2].wrapping_add(100),
        ]);
        publish_initial(&mut manager, &mut fx, id, graph.class());
        match graph {
            Graph::Following => assert_eq!(
                tick_follow_beacons_live_acquisition(
                    &mut manager,
                    FollowBeaconsLiveAcquisitionRequest {
                        entity_id: id,
                        metadata: &metadata
                    },
                    &mut || u32::from(fx.next_shared_retail_random_u16()),
                )
                .unwrap(),
                FollowBeaconsLiveAcquisitionOutcome::Following { target_id }
            ),
            Graph::Chase => {
                assert!(super::super::search::acquire(&mut manager, id, 20_000, &mut fx).unwrap())
            }
            Graph::FollowAcquiring | Graph::SearchAcquiring => {}
        }
        let entity = manager.entity_mut(id).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(u32::MAX, 0x0406_8000);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
        entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
        entity.set_velocity_raw([0; 3]);
        let primary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        // Preserve an actual aged task's noncontact fields, not just the zero
        // values produced by its constructor.
        match entity.actor_tasks.task_state_mut(primary).unwrap() {
            ActorTaskRuntime::SharedRetarget(task) => {
                task.before_callback(37_000);
            }
            ActorTaskRuntime::FollowBeaconsFollowing(task) => {
                task.before_callback(37_000);
            }
            ActorTaskRuntime::ChaseTarget(task) => {
                task.before_callback(37_000);
            }
            other => panic!("{graph:?}: {other:?}"),
        }
        let (position, contact) = find_overlap(&session, &manager, id);
        manager
            .entity_mut(id)
            .unwrap()
            .set_motion_raw(position, [0; 3]);
        let mut scheduler = SpecializedActorTaskScheduler::default();
        scheduler.register_intro2_type58(Intro2Type58Owner::adopt(&manager, id).unwrap());
        Self {
            session,
            manager,
            fx,
            scheduler,
            damage: StaticDamageScheduler::new(),
            notifications: GameplayNotifications::new(),
            id,
            contact,
        }
    }

    fn resolve(&mut self) -> Intro2Type58ContactOutcome {
        resolve_intro2_type58_static_contact(
            &mut Intro2ContactFrame {
                entities: &mut self.manager,
                resources: &mut self.session.cache,
                world_fx: &mut self.fx,
                static_damage: &mut self.damage,
                notifications: &mut self.notifications,
                retail_tick: 4793,
                actor_tasks: &mut self.scheduler,
            },
            self.id,
        )
    }

    fn snapshot(&mut self) -> Snapshot {
        Snapshot::from_entity(self.manager.entity_mut(self.id).unwrap())
    }
}

fn publish_initial(manager: &mut EntityManager, fx: &mut WorldFx, id: u32, class: u32) {
    let metadata = manager.type_runtime_metadata(58).unwrap().clone();
    let selection = BehaviorSelection {
        choice_index: if class == 33 { 0 } else { 2 },
        program: behavior_program(class).unwrap(),
    };
    assert!(super::super::native::publish_initial_style(
        manager.entity_mut(id).unwrap(),
        &metadata,
        selection,
        BehaviorContextRuntime::from_fresh_weighted_selection(selection).unwrap(),
        &mut || u32::from(fx.next_shared_retail_random_u16()),
    ));
}

fn find_overlap(
    session: &GameSession,
    manager: &EntityManager,
    id: u32,
) -> ([i16; 3], StaticModelContact) {
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    // The authored spawn35 neighborhood contains the same real static
    // geometry used by the Type53 contact regression. Preserve the Type58
    // graph and body basis while moving only this collision probe.
    let center = manager
        .iter_all()
        .find(|entity| entity.entity_type == 53 && entity.authored_spawn_index == Some(35))
        .unwrap()
        .position_raw();
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        panic!("native basis")
    };
    let model = session.cache.global_model(MODEL).unwrap();
    for dy in [0_i16, -64, 64] {
        for dx in (-1024..=1024).step_by(64) {
            for dz in (-1024..=1024).step_by(64) {
                let position = [
                    center[0].wrapping_add(dx as i16),
                    center[1].wrapping_add(dy),
                    center[2].wrapping_add(dz as i16),
                ];
                let contact = scan_deepest_static_contact(StaticContactQuery {
                    terrain: session.cache.terrain().unwrap(),
                    terrain_objects: session.cache.terrain_objects().unwrap(),
                    model_pool: &session.cache,
                    tick: 4793,
                    active_model: model,
                    active_model_to_world_basis: basis
                        .orientation_world_from_model()
                        .map(|row| row.map(f64::from)),
                    active_anim_vars: &entity.presentation_anim_vars(4793),
                    position_raw: position,
                })
                .unwrap();
                if let Some(contact) = contact {
                    return (position, contact);
                }
            }
        }
    }
    panic!("authored static geometry must overlap Type58 model273");
}

type TaskSnapshot = Option<(ActorTaskId, ActorTaskRuntime, ActorTaskWrapperFlags)>;

#[derive(Debug, Clone, PartialEq)]
struct Snapshot {
    position: [i16; 3],
    velocity: [i16; 3],
    rotation: [i16; 3],
    basis: RetailRuntimeValue<Type9BodyBasis>,
    collision: EntityCollisionRuntimeState,
    runtime: Option<Intro2Type58Runtime>,
    sub_a: RetailRuntimeValue<Option<SubAPropulsionRuntime>>,
    sub_h: RetailRuntimeValue<Option<SubHRuntimeState>>,
    context: RetailRuntimeValue<Option<BehaviorContextRuntime>>,
    tasks: [TaskSnapshot; 3],
}

impl Snapshot {
    fn from_entity(entity: &Entity) -> Self {
        Self {
            position: entity.position_raw(),
            velocity: entity.velocity_raw(),
            rotation: entity.rotation_heading_pitch_roll_raw(),
            basis: entity.physical_body_basis_q31(),
            collision: entity.collision.clone(),
            runtime: entity.intro2_type58_runtime,
            sub_a: entity.sub_a_propulsion_runtime,
            sub_h: entity.sub_h_external_frame_runtime.clone(),
            context: entity.current_behavior_context,
            tasks: ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
                entity.actor_tasks.task_in_slot(slot).map(|id| {
                    (
                        id,
                        *entity.actor_tasks.task_state(id).unwrap(),
                        entity.actor_tasks.wrapper_flags(id).unwrap(),
                    )
                })
            }),
        }
    }
}

fn private(task: ActorTaskRuntime) -> WanderNearPrivateState {
    match task {
        ActorTaskRuntime::SharedRetarget(task) => task.private_state(),
        ActorTaskRuntime::FollowBeaconsFollowing(task) => task.private_state(),
        ActorTaskRuntime::ChaseTarget(task) => task.private_state(),
        other => panic!("contact private record: {other:?}"),
    }
}

fn with_private(mut task: ActorTaskRuntime, private: WanderNearPrivateState) -> ActorTaskRuntime {
    match &mut task {
        ActorTaskRuntime::SharedRetarget(task) => task.apply_static_contact_private_state(private),
        ActorTaskRuntime::FollowBeaconsFollowing(task) => {
            let mut stage = task.stage_callback();
            *stage.private_state_mut() = private;
            stage.commit(task);
        }
        ActorTaskRuntime::ChaseTarget(task) => {
            let mut stage = task.stage_callback();
            *stage.private_state_mut() = private;
            stage.commit(task);
        }
        other => panic!("contact private record: {other:?}"),
    }
    task
}

#[v2k_test_support::retail_test]
fn native_type58_follow_and_search_contact_preserve_typed_graphs_and_exact_two_word_hook() {
    for graph in [
        Graph::FollowAcquiring,
        Graph::Following,
        Graph::SearchAcquiring,
        Graph::Chase,
    ] {
        let mut world = World::new(graph);
        let before = world.snapshot();
        let owner = Intro2Type58Owner::adopt(&world.manager, world.id).unwrap();
        let (primary_id, primary, flags) = before.tasks[0].unwrap();
        let before_private = private(primary);
        let RetailRuntimeValue::Known(Some(sub_a)) = before.sub_a else {
            panic!("native A")
        };
        let mut oracle = world.fx.fork_for_main_base_abort_transaction();
        let x_word = oracle.next_shared_retail_random_u16();
        let z_word = oracle.next_shared_retail_random_u16();
        let mut words = [x_word, z_word].into_iter();
        let plan = plan_wander_private_static_contact(
            WanderPrivateStaticContactRequest {
                private_state: before_private,
                controlled_position_raw: before.position,
                heading_raw: before.rotation[0] as u16,
                topology: RetailRuntimeValue::Known(FollowBeaconsStaticContactTopology {
                    sub_i: false,
                    sub_a: Some(sub_a),
                    sub_f: false,
                    sub_g: false,
                }),
            },
            || u32::from(words.next().expect("X then Z")),
        )
        .unwrap();
        assert!(words.next().is_none());
        let offset = |word: u16| ((word >> 6) as i16).wrapping_sub(0x200);
        assert_eq!(
            plan.private_state_after,
            WanderNearPrivateState {
                direction: before_private.direction.wrapping_neg(),
                reversal_timer_ms: 2500,
                target_position_raw: [
                    before.position[0].wrapping_add(offset(x_word)),
                    before_private.target_position_raw[1],
                    before.position[2].wrapping_add(offset(z_word))
                ],
                ..before_private
            }
        );
        assert_eq!(plan.sub_f_reverse_write, None);
        assert_eq!(plan.sub_g_reverse_write, None);
        let mut expected_position = before.position;
        let mut expected_velocity = before.velocity;
        apply_contact_response_raw(
            &mut expected_position,
            &mut expected_velocity,
            world.contact,
        );
        let outcome = world.resolve();
        let Intro2Type58ContactOutcome::Applied(applied) = outcome else {
            panic!("{graph:?}: {outcome:?}")
        };
        assert_eq!(applied.contact, world.contact);
        assert_eq!(applied.position_before_response_raw, before.position);
        assert_eq!(applied.position_after_response_raw, expected_position);
        assert_eq!(applied.velocity_after_response_raw, expected_velocity);
        assert!(applied.furniture_damage.is_none(), "02CA0 has no C890/C690");
        assert_eq!(applied.collision_impact_raw, 0);
        assert!(applied.collision_static_damage.is_none());
        assert!(applied.actor_damage.is_none());
        let after = world.snapshot();
        assert_eq!(
            after.tasks[0],
            Some((
                primary_id,
                with_private(primary, plan.private_state_after),
                flags
            ))
        );
        assert_eq!(
            after.tasks[1..],
            before.tasks[1..],
            "Secondary and Aim survive"
        );
        assert_eq!(after.context, before.context);
        assert_eq!(after.sub_a, RetailRuntimeValue::Known(plan.sub_a_runtime));
        let RetailRuntimeValue::Known(Some(after_a)) = after.sub_a else {
            panic!("retained A")
        };
        assert_eq!(after_a.target_speed_raw(), sub_a.target_speed_raw());
        assert_eq!(after_a.drive_scale_percent(), sub_a.drive_scale_percent());
        assert_eq!(
            (after.rotation, after.basis, after.runtime, after.sub_h),
            (before.rotation, before.basis, before.runtime, before.sub_h)
        );
        assert_eq!(after.collision, before.collision);
        assert_eq!(
            world.scheduler.family_for(world.id),
            Some(SpecializedActorTaskFamily::Intro2Type58)
        );
        assert!(world
            .scheduler
            .intro2_type58_completed_owner(&world.manager, world.id));
        assert_eq!(
            Intro2Type58Owner::adopt(&world.manager, world.id),
            Ok(owner)
        );
        assert_eq!(world.damage.active_program_count(), 0);
        assert_eq!(
            world.fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16(),
            "{graph:?}: only X/Z words"
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type58_pending_executing_stale_and_foreign_contact_custody_reject_before_writes() {
    for invalid in 0..5 {
        let mut world = World::new(Graph::SearchAcquiring);
        match invalid {
            0 => world.scheduler.register_intro2_type58(
                Intro2Type58Owner::adopt_blocked_prefix(&world.manager, world.id).unwrap(),
            ),
            1 => {
                let entity = world.manager.entity_mut(world.id).unwrap();
                let task_id = entity
                    .actor_tasks
                    .task_in_slot(ActorTaskSlot::Primary)
                    .unwrap();
                entity
                    .actor_tasks
                    .begin_exact_visit_with(
                        ActorTaskVisit {
                            slot: ActorTaskSlot::Primary,
                            task_id,
                        },
                        |_| (),
                    )
                    .unwrap();
            }
            2 => publish_initial(&mut world.manager, &mut world.fx, world.id, 7),
            3 => {
                world
                    .manager
                    .entity_mut(world.id)
                    .unwrap()
                    .actor_tasks
                    .clear_slot(ActorTaskSlot::Secondary);
                world.scheduler.register_intro2_type58(
                    Intro2Type58Owner::adopt(&world.manager, world.id).unwrap(),
                );
            }
            4 => {
                let mut other = World::new(Graph::SearchAcquiring);
                std::mem::swap(
                    world.manager.entity_mut(world.id).unwrap(),
                    other.manager.entity_mut(other.id).unwrap(),
                );
                assert!(intro2_type58_allocation_authenticates(
                    world.manager.entity_mut(world.id).unwrap()
                ));
                assert!(!type58_manager_allocation_authenticates(
                    &world.manager,
                    world.id
                ));
            }
            _ => unreachable!(),
        }
        let before = world.snapshot();
        let mut oracle = world.fx.fork_for_main_base_abort_transaction();
        let outcome = world.resolve();
        assert!(
            matches!(
                outcome,
                Intro2Type58ContactOutcome::Blocked {
                    committed_prefix: false,
                    ..
                }
            ),
            "case{invalid}: {outcome:?}"
        );
        assert_eq!(world.snapshot(), before, "case{invalid}");
        assert_eq!(world.damage.active_program_count(), 0);
        assert_eq!(
            world.fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16(),
            "case{invalid}"
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type58_contact_late_damage_failure_parks_the_mutated_private_and_original_plane() {
    let mut world = World::new(Graph::Chase);
    let inward = world
        .contact
        .normal_q12
        .map(|component| component.wrapping_mul(-2));
    let entity = world.manager.entity_mut(world.id).unwrap();
    entity.set_velocity_raw(inward);
    entity.collision.pair_callbacks.damage_modifier_address = RetailRuntimeValue::Unresolved;
    let before = world.snapshot();
    let mut expected_position = before.position;
    let mut expected_velocity = before.velocity;
    apply_contact_response_raw(
        &mut expected_position,
        &mut expected_velocity,
        world.contact,
    );
    assert_ne!(expected_velocity, before.velocity);
    let mut oracle = world.fx.fork_for_main_base_abort_transaction();
    let mut expected_private = private(before.tasks[0].unwrap().1);
    expected_private.direction = expected_private.direction.wrapping_neg();
    expected_private.reversal_timer_ms = 2500;
    expected_private.target_position_raw[0] = before.position[0]
        .wrapping_add(((oracle.next_shared_retail_random_u16() >> 6) as i16).wrapping_sub(0x200));
    expected_private.target_position_raw[2] = before.position[2]
        .wrapping_add(((oracle.next_shared_retail_random_u16() >> 6) as i16).wrapping_sub(0x200));
    let outcome = world.resolve();
    assert!(
        matches!(
            outcome,
            Intro2Type58ContactOutcome::Blocked {
                reason: Intro2Type58ContactBlock::Damage(_),
                committed_prefix: true
            }
        ),
        "{outcome:?}"
    );
    let after = world.snapshot();
    assert_eq!(
        (after.position, after.velocity),
        (expected_position, expected_velocity)
    );
    assert_eq!(private(after.tasks[0].unwrap().1), expected_private);
    assert_eq!(after.tasks[0].unwrap().0, before.tasks[0].unwrap().0);
    assert_eq!(after.tasks[1..], before.tasks[1..]);
    assert_eq!(after.context, before.context);
    assert_eq!((after.basis, after.runtime), (before.basis, before.runtime));
    assert_eq!(after.collision.health_raw, before.collision.health_raw);
    assert!(world.scheduler.intro2_type58_has_pending_prefix(world.id));
    assert!(!world
        .scheduler
        .intro2_type58_completed_owner(&world.manager, world.id));
    let mut after_rng = world.fx.fork_for_main_base_abort_transaction();
    let outcome = world.resolve();
    assert!(
        matches!(
            outcome,
            Intro2Type58ContactOutcome::Blocked {
                committed_prefix: false,
                ..
            }
        ),
        "{outcome:?}"
    );
    assert_eq!(world.snapshot(), after);
    assert_eq!(
        world.fx.next_shared_retail_random_u16(),
        after_rng.next_shared_retail_random_u16()
    );
}
