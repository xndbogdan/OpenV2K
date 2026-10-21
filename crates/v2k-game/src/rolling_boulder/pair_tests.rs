//! The player's active pair pass against native class20 boulders.

use super::*;
use crate::{
    gameplay_notifications::GameplayNotifications,
    intro2_radial::Intro2RadialTaskCustody,
    player::PlayerCraft,
    player_active_contact::{
        resolve_player_active_contacts, PlayerActivePairError, PlayerActivePairFrame,
        PlayerActivePairPass,
    },
    player_hull::PlayerHull,
    session::GameSession,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
};

struct PairWorld {
    session: GameSession,
    manager: EntityManager,
    fx: WorldFx,
    tasks: SpecializedActorTaskScheduler,
    hull: PlayerHull,
    id: u32,
}

impl PairWorld {
    /// World27 with its player, one live Type27 and nothing else eligible.
    fn new(style: RollingBoulderStyle) -> Self {
        let (session, mut manager, fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(27);
        let player = manager.player().unwrap().id;
        let id = manager
            .iter_all()
            .find(|entity| entity.entity_type == 27)
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
        let boulder = manager.entity_mut(id).unwrap();
        boulder
            .collision
            .state_flags_at_0x08
            .overwrite(0x0202_8000, 0x0202_8000);
        boulder.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        if style == RollingBoulderStyle::Resting {
            switch_style(boulder, RollingBoulderStyle::Resting).unwrap();
        }
        let position = boulder.position_raw();
        // Drive the player into the boulder from just beside it.
        manager.player_mut().unwrap().set_motion_raw(
            [position[0].wrapping_sub(64), position[1], position[2]],
            [512, 0, 0],
        );
        let hull = PlayerHull::default();
        manager.sync_player_hull_collision_state(&hull);
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_rolling_boulders(&manager), 6);
        Self {
            session,
            manager,
            fx,
            tasks,
            hull,
            id,
        }
    }

    fn pass(&mut self) -> Result<PlayerActivePairPass, PlayerActivePairError> {
        let mut notifications = GameplayNotifications::new();
        resolve_player_active_contacts(
            &mut self.manager,
            PlayerActivePairFrame {
                resources: &mut self.session.cache,
                static_damage: &mut crate::static_damage::StaticDamageScheduler::new(),
                retail_tick: 5000,
                player_craft: &PlayerCraft::new(),
                player_hull: &mut self.hull,
                scheduler: &mut self.tasks,
                world_fx: &mut self.fx,
                notifications: &mut notifications,
                extra_lives: 3,
            },
        )
    }

    fn boulder(&self) -> &Entity {
        self.manager
            .iter_all()
            .find(|entity| entity.id == self.id)
            .unwrap()
    }
}

#[v2k_test_support::retail_test]
fn the_player_pass_pushes_a_rolling_boulder() {
    let mut world = PairWorld::new(RollingBoulderStyle::Rolling);
    let velocity = world.boulder().velocity_raw();
    let pass = world.pass().unwrap();
    assert!(pass
        .core
        .candidates
        .iter()
        .any(|candidate| candidate.id == world.id));
    assert_ne!(
        world.boulder().velocity_raw(),
        velocity,
        "12760 moves the boulder"
    );
    assert_eq!(
        current_style(world.boulder()),
        Ok(RollingBoulderStyle::Rolling)
    );
    assert!(RollingBoulderOwner::adopt(&world.manager, world.id).is_ok());
}

#[v2k_test_support::retail_test]
fn the_player_wakes_a_resting_boulder_through_style1_plus18() {
    let mut world = PairWorld::new(RollingBoulderStyle::Resting);
    world.pass().unwrap();
    let boulder = world.boulder();
    assert_eq!(current_style(boulder), Ok(RollingBoulderStyle::Rolling));
    assert_eq!(
        boulder.collision.state_flags_at_0x08.masked(0x50000),
        RetailRuntimeValue::Known(0x50000)
    );
    assert!(matches!(
        boulder.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::BoulderRolling(_))
    ));
    // The scheduler owns the replaced rolling Primary.
    assert!(world
        .tasks
        .prepare_native_actor_mutation(&world.manager, world.id));
}

#[v2k_test_support::retail_test]
fn a_parked_boulder_holds_the_player_pass() {
    let mut world = PairWorld::new(RollingBoulderStyle::Rolling);
    assert!(world
        .tasks
        .park_native_contact_prefix(&world.manager, world.id));
    let boulder = world.boulder();
    let (position, velocity, collision) = (
        boulder.position_raw(),
        boulder.velocity_raw(),
        boulder.collision.clone(),
    );
    let error = world.pass().unwrap_err();
    assert_eq!(
        error,
        PlayerActivePairError::TaskCustodyUnavailable {
            entity_id: world.id
        }
    );
    let boulder = world.boulder();
    assert_eq!(boulder.position_raw(), position);
    assert_eq!(boulder.velocity_raw(), velocity);
    assert_eq!(boulder.collision, collision);
}

/// Drive the player into the boulder fast enough for a lethal pair packet.
fn ram(world: &mut PairWorld, speed_raw: i16) {
    let position = world.boulder().position_raw();
    world.manager.player_mut().unwrap().set_motion_raw(
        [position[0].wrapping_sub(64), position[1], position[2]],
        [speed_raw, 0, 0],
    );
}

/// A lethal packet kills the Type27 at its pair: the class18 split appends
/// two Type3s to the live list's tail, and the walk resumes past the corpse,
/// so those children are this frame's candidates too.
#[v2k_test_support::retail_test]
fn a_lethal_ram_splits_a_type27_and_the_walk_resumes() {
    let mut world = PairWorld::new(RollingBoulderStyle::Rolling);
    world
        .manager
        .entity_mut(world.id)
        .unwrap()
        .collision
        .health_raw = RetailRuntimeValue::Known(1);
    let before: Vec<u32> = world.manager.iter_all().map(|entity| entity.id).collect();
    ram(&mut world, 12_000);
    let pass = world.pass().unwrap();
    assert!(death::finished_split_authenticates(
        &world.manager,
        world.id
    ));
    let children: Vec<u32> = world
        .manager
        .iter_all()
        .filter(|entity| !before.contains(&entity.id))
        .map(|entity| entity.id)
        .collect();
    assert_eq!(children.len(), 2);
    for &child in &children {
        let entity = world
            .manager
            .iter_all()
            .find(|entity| entity.id == child)
            .unwrap();
        assert_eq!(entity.entity_type, 3);
    }
    // The resumed segment walked the children after the corpse.
    assert!(!matches!(
        pass.core.termination,
        crate::active_pair::ActivePairPassTermination::CandidateDeathDispatched { .. }
    ));
    assert!(pass
        .core
        .candidates
        .iter()
        .any(|candidate| children.contains(&candidate.id)));
}

/// A Type3 child of that split dies through class1 when rammed: BAF0's blast
/// and radial run in the walk, and BAC0 stages its removal.
#[v2k_test_support::retail_test]
fn a_lethal_ram_explodes_a_type3() {
    let mut world = PairWorld::new(RollingBoulderStyle::Rolling);
    world
        .manager
        .entity_mut(world.id)
        .unwrap()
        .collision
        .health_raw = RetailRuntimeValue::Known(1);
    let before: Vec<u32> = world.manager.iter_all().map(|entity| entity.id).collect();
    ram(&mut world, 12_000);
    world.pass().unwrap();
    let child = world
        .manager
        .iter_all()
        .find(|entity| !before.contains(&entity.id))
        .unwrap()
        .id;
    world.id = child;
    for other in world
        .manager
        .iter_all()
        .map(|entity| entity.id)
        .collect::<Vec<_>>()
    {
        if other != child && Some(other) != world.manager.player().map(|player| player.id) {
            world
                .manager
                .entity_mut(other)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(0x8000, 0);
        }
    }
    let boulder = world.manager.entity_mut(child).unwrap();
    boulder.collision.health_raw = RetailRuntimeValue::Known(1);
    boulder
        .collision
        .state_flags_at_0x08
        .overwrite(0x0202_8000, 0x0202_8000);
    boulder.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    ram(&mut world, 12_000);
    let hull_before = world.hull.health_raw;
    world.pass().unwrap();
    assert!(crate::class49_death::finished_terminal_hit_authenticates(
        &world.manager,
        child
    ));
    assert!(world
        .manager
        .pending_actor_deferred_destroy_ids()
        .contains(&child));
    // BAF0's radial reaches the craft beside the blast.
    assert!(world.hull.health_raw <= hull_before);
}
