//! The player's pair pass runs a native worker's 02DA0 at its component
//! callback and commits the Sub-I reversal with the body pass.
use super::*;
use crate::{
    active_pair::PairCandidateDisposition,
    gameplay_notifications::GameplayNotifications,
    intro2_radial::Intro2RadialTaskCustody,
    player::PlayerCraft,
    player_active_contact::{resolve_player_active_contacts, PlayerActivePairFrame},
    player_hull::PlayerHull,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    world_fx::WorldFx,
};

struct WorkerPair {
    session: crate::session::GameSession,
    manager: EntityManager,
    fx: WorldFx,
    tasks: SpecializedActorTaskScheduler,
    hull: PlayerHull,
    id: u32,
}

impl WorkerPair {
    /// World19's first Type79 with a descriptor task, every other body out
    /// of the pass, and the player overlapping it `offset` along its forward.
    fn new(offset: i64) -> Self {
        let (session, mut manager, fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(19);
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert!(tasks.adopt_intro2_type8(&mut manager) > 0);
        let player = manager.player().unwrap().id;
        let id = manager
            .iter_all()
            .find(|entity| {
                entity.entity_type == 79
                    && ActorTaskSlot::IN_RETAIL_TICK_ORDER.into_iter().any(|slot| {
                        entity
                            .actor_task_state(slot)
                            .is_some_and(task_has_descriptor_contact)
                    })
            })
            .unwrap()
            .id;
        let others: Vec<_> = manager
            .iter_all()
            .map(|entity| entity.id)
            .filter(|&other| other != id && other != player)
            .collect();
        for other in others {
            manager
                .entity_mut(other)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(u32::MAX, 0);
        }
        let worker = actor(&manager, id).unwrap();
        let RetailRuntimeValue::Known(basis) = worker.physical_body_basis_q31() else {
            panic!("worker basis");
        };
        let position = worker.position_raw();
        let player_position = std::array::from_fn(|axis| {
            position[axis].wrapping_add(((i64::from(basis.forward[axis]) * offset) >> 31) as i16)
        });
        manager
            .player_mut()
            .unwrap()
            .set_motion_raw(player_position, [0, 0, 0]);
        let hull = PlayerHull::default();
        manager.sync_player_hull_collision_state(&hull);
        Self {
            session,
            manager,
            fx,
            tasks,
            hull,
            id,
        }
    }

    fn pass(&mut self) -> crate::player_active_contact::PlayerActivePairPass {
        let mut notifications = GameplayNotifications::new();
        resolve_player_active_contacts(
            &mut self.manager,
            &self.session.cache,
            PlayerActivePairFrame {
                resources: &self.session.cache,
                retail_tick: 5000,
                player_craft: &PlayerCraft::new(),
                player_hull: &mut self.hull,
                scheduler: &mut self.tasks,
                world_fx: &mut self.fx,
                notifications: &mut notifications,
                extra_lives: 3,
            },
        )
        .unwrap()
    }

    fn touched(&self, pass: &crate::player_active_contact::PlayerActivePairPass) -> bool {
        pass.core.dispositions.iter().any(|disposition| {
            matches!(disposition, PairCandidateDisposition::Resolved { candidate_id, .. }
                if *candidate_id == self.id)
        })
    }
}

#[v2k_test_support::retail_test]
fn the_player_ahead_of_a_worker_turns_it_through_its_02da0() {
    let mut pair = WorkerPair::new(24);
    let worker = actor(&pair.manager, pair.id).unwrap();
    let orientation = worker.rotation_heading_pitch_roll_raw();
    let tasks =
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| worker.actor_task_state(slot).copied());
    let next_word = pair
        .fx
        .fork_for_main_base_abort_transaction()
        .next_shared_retail_random_u16();
    let pass = pair.pass();
    assert!(pair.touched(&pass), "the player overlaps the worker");
    let worker = actor(&pair.manager, pair.id).unwrap();
    // 01A20's Sub-I branch: a quarter-turn heading, no RNG, task unchanged.
    assert_eq!(
        worker.rotation_heading_pitch_roll_raw(),
        [
            orientation[0].wrapping_add(0x2000),
            orientation[1],
            orientation[2]
        ]
    );
    assert_eq!(
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| worker.actor_task_state(slot).copied()),
        tasks
    );
    assert_eq!(
        pair.fx
            .fork_for_main_base_abort_transaction()
            .next_shared_retail_random_u16(),
        next_word
    );
    assert!(pair
        .tasks
        .prepare_native_actor_mutation(&pair.manager, pair.id));
}

#[v2k_test_support::retail_test]
fn the_player_behind_a_worker_leaves_its_heading() {
    let mut pair = WorkerPair::new(-24);
    let orientation = actor(&pair.manager, pair.id)
        .unwrap()
        .rotation_heading_pitch_roll_raw();
    let pass = pair.pass();
    assert!(pair.touched(&pass), "the player overlaps the worker");
    assert_eq!(
        actor(&pair.manager, pair.id)
            .unwrap()
            .rotation_heading_pitch_roll_raw(),
        orientation
    );
}

#[v2k_test_support::retail_test]
fn a_later_slot_reads_the_earlier_slots_01a20_writes() {
    let mut pair = WorkerPair::new(24);
    let mut source =
        NativeDescriptorContactSource::capture(&pair.manager, &mut pair.tasks, pair.id).unwrap();
    let slot = source.slots.into_iter().flatten().next().unwrap();
    source.slots = [Some(slot); 3];
    // An A/D shape: 01A20 turns the heading and redraws Sub-A's speed.
    let mut topology = source.topology.unwrap();
    topology.sub_i = false;
    topology.sub_d = Some(
        crate::common_mover::target_prelude::CommonMoverPreludeSubD {
            steering_divisor_raw: 64,
            couple_yaw_into_roll: true,
        },
    );
    source.topology = Ok(topology);
    let player = pair.manager.player().unwrap();
    let target = DescriptorContactTargetSnapshot {
        entity_id: player.id,
        position_raw: player.position_raw(),
    };
    let mut word = 7u32;
    let plans = source
        .plan(target, &mut || {
            word = word.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            word >> 16
        })
        .unwrap();
    assert_eq!(plans.len(), 3);
    for step in plans.windows(2) {
        let (earlier, later) = (step[0].plan, step[1].plan);
        assert_eq!(
            later.expected_source.heading_raw,
            earlier.effect.heading_raw
        );
        assert_eq!(later.expected_source.roll_raw, earlier.effect.roll_raw);
        assert_eq!(
            later.expected_topology.sub_a.map(|sub_a| sub_a.runtime),
            earlier.effect.sub_a_runtime
        );
    }
}
