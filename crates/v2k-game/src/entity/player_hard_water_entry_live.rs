//! Ordered commit for the player's hard whole-body water-entry response.
//!
//! `FUN_004141D0` constructs Type 60 first, submits positional sound 17
//! second, and only then arithmetic-shifts the entering body's signed Y
//! velocity.  The player integrator returns a private receipt so the caller
//! can supply the shared entity-constructor RNG/effect owner without moving
//! those writes out of their authenticated order.

use super::{
    main_base_abort_actor_allocation_identity, EntityManager, RetailStateWord, PLAYER_ENTITY_TYPE,
};
use crate::main_base_abort::MainBaseAbortActorLease;
use crate::type60_exploding_ring::{
    HardWaterType60ConstructionRequest, HardWaterType60Severity, Type60ConstructionOutcome,
};
use crate::world_fx::{WorldFx, PLAYER_WATER_ENTRY_SOUND_ID};
use v2k_formats::terrain::TerrainGrid;

/// One unreplayed hard-entry suffix issued by the player surface classifier.
///
/// Fields are private so a caller cannot manufacture constructor authority.
/// The complete player pose, velocity, collision state, and allocation lease
/// are rechecked before constructor RNG or presentation state can change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerHardWaterEntryCommit {
    actor: MainBaseAbortActorLease,
    body_position_raw: [i16; 3],
    ring_position_raw: [i16; 3],
    velocity_before_raw: [i16; 3],
    vertical_velocity_after_raw: i16,
    collision_state_after_classifier: RetailStateWord,
    severity: HardWaterType60Severity,
}

impl PlayerHardWaterEntryCommit {
    pub(super) const fn issue(
        actor: MainBaseAbortActorLease,
        body_position_raw: [i16; 3],
        surface_y_raw: i16,
        velocity_before_raw: [i16; 3],
        vertical_velocity_after_raw: i16,
        collision_state_after_classifier: RetailStateWord,
        severe: bool,
    ) -> Self {
        Self {
            actor,
            body_position_raw,
            ring_position_raw: [body_position_raw[0], surface_y_raw, body_position_raw[2]],
            velocity_before_raw,
            vertical_velocity_after_raw,
            collision_state_after_classifier,
            severity: if severe {
                HardWaterType60Severity::Severe
            } else {
                HardWaterType60Severity::Moderate
            },
        }
    }

    pub const fn actor(self) -> MainBaseAbortActorLease {
        self.actor
    }

    pub const fn body_position_raw(self) -> [i16; 3] {
        self.body_position_raw
    }

    pub const fn ring_position_raw(self) -> [i16; 3] {
        self.ring_position_raw
    }

    pub const fn vertical_velocity_after_raw(self) -> i16 {
        self.vertical_velocity_after_raw
    }
}

/// A stale or reordered commit cannot consume constructor RNG, queue sound,
/// or damp a different player allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerHardWaterEntryCommitBlock {
    PlayerBindingChanged {
        expected: u32,
        actual: Option<u32>,
    },
    PlayerEntityUnavailable,
    ActorLeaseMismatch {
        expected: MainBaseAbortActorLease,
        actual: MainBaseAbortActorLease,
    },
    WrongEntityType {
        actual: u32,
    },
    BodyPositionChanged {
        expected: [i16; 3],
        actual: [i16; 3],
    },
    VelocityChanged {
        expected: [i16; 3],
        actual: [i16; 3],
    },
    CollisionStateChanged,
}

impl EntityManager {
    /// Commit the constructor -> sound -> signed-damping suffix for one hard
    /// player water-entry edge.
    pub fn commit_player_hard_water_entry(
        &mut self,
        commit: PlayerHardWaterEntryCommit,
        terrain: &TerrainGrid,
        world_fx: &mut WorldFx,
    ) -> Result<Type60ConstructionOutcome, PlayerHardWaterEntryCommitBlock> {
        self.commit_player_hard_water_entry_with(
            commit,
            terrain,
            world_fx,
            |manager, request, terrain, world_fx| {
                manager.construct_hard_water_type60_ring(request, terrain, world_fx)
            },
        )
    }

    fn commit_player_hard_water_entry_with(
        &mut self,
        commit: PlayerHardWaterEntryCommit,
        terrain: &TerrainGrid,
        world_fx: &mut WorldFx,
        construct: impl FnOnce(
            &mut EntityManager,
            HardWaterType60ConstructionRequest,
            &TerrainGrid,
            &mut WorldFx,
        ) -> Type60ConstructionOutcome,
    ) -> Result<Type60ConstructionOutcome, PlayerHardWaterEntryCommitBlock> {
        if self.player_id != Some(commit.actor.entity_id) {
            return Err(PlayerHardWaterEntryCommitBlock::PlayerBindingChanged {
                expected: commit.actor.entity_id,
                actual: self.player_id,
            });
        }
        let Some(player_index) = self
            .entities
            .iter()
            .position(|entity| entity.id == commit.actor.entity_id)
        else {
            return Err(PlayerHardWaterEntryCommitBlock::PlayerEntityUnavailable);
        };
        let player = &self.entities[player_index];
        let expected_actor = MainBaseAbortActorLease {
            entity_id: player.id,
            allocation_identity: main_base_abort_actor_allocation_identity(
                self.allocation_generation,
                player.id,
            ),
        };
        if expected_actor != commit.actor {
            return Err(PlayerHardWaterEntryCommitBlock::ActorLeaseMismatch {
                expected: expected_actor,
                actual: commit.actor,
            });
        }
        if player.entity_type != PLAYER_ENTITY_TYPE {
            return Err(PlayerHardWaterEntryCommitBlock::WrongEntityType {
                actual: player.entity_type,
            });
        }
        let actual_position = player.position_raw();
        if actual_position != commit.body_position_raw {
            return Err(PlayerHardWaterEntryCommitBlock::BodyPositionChanged {
                expected: commit.body_position_raw,
                actual: actual_position,
            });
        }
        let actual_velocity = player.velocity_raw();
        if actual_velocity != commit.velocity_before_raw {
            return Err(PlayerHardWaterEntryCommitBlock::VelocityChanged {
                expected: commit.velocity_before_raw,
                actual: actual_velocity,
            });
        }
        if player.collision.state_flags_at_0x08 != commit.collision_state_after_classifier {
            return Err(PlayerHardWaterEntryCommitBlock::CollisionStateChanged);
        }

        let construction = construct(
            self,
            HardWaterType60ConstructionRequest::new(commit.ring_position_raw, commit.severity),
            terrain,
            world_fx,
        );
        // Retail reaches both of these caller-owned writes even when any of
        // Type 60's three construction stages rejects the allocation.
        world_fx.queue_fixed_positional_sound_raw(
            PLAYER_WATER_ENTRY_SOUND_ID,
            commit.body_position_raw,
        );
        let mut velocity_after = commit.velocity_before_raw;
        velocity_after[1] = commit.vertical_velocity_after_raw;
        debug_assert_eq!(self.entities[player_index].id, commit.actor.entity_id);
        self.entities[player_index].set_velocity_raw(velocity_after);

        Ok(construction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{Entity, EntityKind};
    use crate::entity_collision_state::{
        RetailStateWord, FULLY_BELOW_SURFACE_STATE_BIT, SURFACE_STATE_MASK,
    };

    fn committed_entry_fixture() -> (
        EntityManager,
        PlayerHardWaterEntryCommit,
        TerrainGrid,
        WorldFx,
    ) {
        let mut player = Entity::unresolved_port_entity(46, EntityKind::Player, PLAYER_ENTITY_TYPE);
        player.set_motion_raw([0x120, -0x220, 0x340], [11, -1001, -13]);
        player.collision.state_flags_at_0x08 =
            RetailStateWord::from_known_bits(FULLY_BELOW_SURFACE_STATE_BIT, SURFACE_STATE_MASK);
        let manager = EntityManager::from_entities_for_test(vec![player]);
        let actor = manager
            .main_base_abort_actor_observation(46)
            .expect("the player fixture is live")
            .lease;
        let collision_state = manager
            .player()
            .expect("the fixture binds its player")
            .collision
            .state_flags_at_0x08;
        let commit = PlayerHardWaterEntryCommit::issue(
            actor,
            [0x120, -0x220, 0x340],
            0x55,
            [11, -1001, -13],
            -501,
            collision_state,
            false,
        );
        (
            manager,
            commit,
            TerrainGrid {
                header: [0; 5],
                cells: Vec::new(),
            },
            WorldFx::new(),
        )
    }

    #[test]
    fn rejected_constructor_still_precedes_sound_and_signed_damping() {
        let (mut manager, commit, terrain, mut world_fx) = committed_entry_fixture();

        let outcome = manager
            .commit_player_hard_water_entry_with(
                commit,
                &terrain,
                &mut world_fx,
                |manager, request, _, world_fx| {
                    assert_eq!(request.position_raw(), [0x120, 0x55, 0x340]);
                    assert_eq!(request.severity(), HardWaterType60Severity::Moderate);
                    assert_eq!(manager.player().unwrap().velocity_raw(), [11, -1001, -13]);
                    assert!(world_fx.take_positional_sounds().is_empty());
                    Type60ConstructionOutcome::RejectedBeforeSelector
                },
            )
            .expect("the authenticated suffix commits despite constructor rejection");

        assert_eq!(outcome, Type60ConstructionOutcome::RejectedBeforeSelector);
        assert_eq!(manager.player().unwrap().velocity_raw(), [11, -501, -13]);
        assert_eq!(world_fx.pending_event_count(), 1);
        world_fx.process_pending();
        let sounds = world_fx.take_positional_sounds();
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].sound_id, usize::from(PLAYER_WATER_ENTRY_SOUND_ID));
        assert_eq!(
            sounds[0].position,
            super::super::raw_position_world([0x120, -0x220, 0x340])
        );
    }

    #[test]
    fn stale_velocity_blocks_before_constructor_sound_or_damping() {
        let (mut manager, commit, terrain, mut world_fx) = committed_entry_fixture();
        manager
            .player_mut()
            .unwrap()
            .set_velocity_raw([11, -999, -13]);

        let result = manager.commit_player_hard_water_entry_with(
            commit,
            &terrain,
            &mut world_fx,
            |_, _, _, _| panic!("a stale receipt must not reach construction"),
        );

        assert_eq!(
            result,
            Err(PlayerHardWaterEntryCommitBlock::VelocityChanged {
                expected: [11, -1001, -13],
                actual: [11, -999, -13],
            })
        );
        assert_eq!(manager.player().unwrap().velocity_raw(), [11, -999, -13]);
        assert!(world_fx.take_positional_sounds().is_empty());
    }
}
