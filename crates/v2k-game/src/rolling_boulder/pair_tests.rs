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
