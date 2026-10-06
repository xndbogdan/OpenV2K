//! Fresh Level-1 Type-47 `FUN_00411030` after impact C690.
//!
//! Retail reaches this impulse/jolt after style `+0x28` and before
//! `FUN_00415040`. The detached [`apply_impact_reaction`] owner is exact.
//! Original state bit `0x80000000` would request `FUN_00469200(1, 10,
//! &handle)`; this adapter reports that instead of inventing a network
//! submit.

use crate::entity::{Entity, EntityManager};
use crate::entity_collision_state::RetailRuntimeValue;
use crate::impact_reaction::{
    apply_impact_reaction, ImpactReactionBody, ImpactReactionError, ImpactReactionOutcome,
    IMPACT_REACTION_ENABLED_STATE_BIT, IMPACT_REACTION_NETWORKED_STATE_BIT,
    IMPACT_REACTION_SUPPRESSED_STATE_BIT,
};
use crate::ordinary_type47_death_live::TYPE47_COMMON_DYING_ENTITY_TYPE;
use crate::ordinary_type47_live::FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID;
use crate::world_fx::WorldFx;

const TYPE47_IMPACT_REACTION_STATE_BITS: u32 = IMPACT_REACTION_ENABLED_STATE_BIT
    | IMPACT_REACTION_SUPPRESSED_STATE_BIT
    | IMPACT_REACTION_NETWORKED_STATE_BIT;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type47ImpactReactionBlock {
    UnauthenticatedSpawn {
        actual: Option<usize>,
    },
    UnexpectedModelSlots {
        actual: [Option<usize>; 4],
    },
    UnexpectedActiveModel {
        actual: Option<usize>,
    },
    StateFlagsUnresolved,
    /// Entry bit `0x80000000` would call `FUN_00469200`; do not invent it.
    NetworkRequest,
    ZeroMass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type47ImpactReactionOutcome {
    NotApplicable,
    Applied(ImpactReactionOutcome),
    Blocked {
        entity_id: u32,
        reason: Type47ImpactReactionBlock,
    },
}

/// Apply `FUN_00411030` to one fresh Level-1 Type-47.
pub fn apply_type47_impact_reaction_after_c690(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    impact_sum_raw: i32,
    direction_q15: [i16; 3],
) -> Type47ImpactReactionOutcome {
    let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
        return Type47ImpactReactionOutcome::NotApplicable;
    };
    if entity.entity_type != TYPE47_COMMON_DYING_ENTITY_TYPE {
        return Type47ImpactReactionOutcome::NotApplicable;
    }
    if let Err(reason) = authenticate_type47_impact_reaction_owner(entity) {
        return Type47ImpactReactionOutcome::Blocked { entity_id, reason };
    }
    let flags = match entity
        .collision
        .state_flags_at_0x08
        .masked(TYPE47_IMPACT_REACTION_STATE_BITS)
    {
        RetailRuntimeValue::Unresolved => {
            return Type47ImpactReactionOutcome::Blocked {
                entity_id,
                reason: Type47ImpactReactionBlock::StateFlagsUnresolved,
            };
        }
        RetailRuntimeValue::Known(flags) => flags,
    };
    if flags & IMPACT_REACTION_NETWORKED_STATE_BIT != 0 {
        return Type47ImpactReactionOutcome::Blocked {
            entity_id,
            reason: Type47ImpactReactionBlock::NetworkRequest,
        };
    }

    let Some(entity) = manager.ordinary_type47_entity_mut(entity_id) else {
        return Type47ImpactReactionOutcome::NotApplicable;
    };
    let mut body = ImpactReactionBody {
        state_flags_at_0x08: flags,
        mass_raw_at_0xb0: entity.mass_raw,
        linear_velocity_xyz_raw: entity.velocity_raw(),
        angular_heading_pitch_roll_raw: entity.rotation_heading_pitch_roll_raw(),
    };
    let outcome =
        match apply_impact_reaction(&mut body, entity_id, impact_sum_raw, direction_q15, || {
            u32::from(world_fx.next_shared_retail_random_u16())
        }) {
            Ok(outcome) => outcome,
            Err(ImpactReactionError::ZeroMass) => {
                return Type47ImpactReactionOutcome::Blocked {
                    entity_id,
                    reason: Type47ImpactReactionBlock::ZeroMass,
                };
            }
        };
    entity.set_velocity_raw(body.linear_velocity_xyz_raw);
    entity.set_rotation_heading_pitch_roll_raw(body.angular_heading_pitch_roll_raw);
    Type47ImpactReactionOutcome::Applied(outcome)
}

fn authenticate_type47_impact_reaction_owner(
    entity: &Entity,
) -> Result<(), Type47ImpactReactionBlock> {
    if crate::type47_initial_behavior_live::live_type47_cohort(entity).is_none() {
        return Err(Type47ImpactReactionBlock::UnauthenticatedSpawn {
            actual: entity.authored_spawn_index,
        });
    }
    let expected_slots = [Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID); 4];
    if entity.model_slots != expected_slots {
        return Err(Type47ImpactReactionBlock::UnexpectedModelSlots {
            actual: entity.model_slots,
        });
    }
    if entity.model_index != Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID) {
        return Err(Type47ImpactReactionBlock::UnexpectedActiveModel {
            actual: entity.model_index,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::damage::PRIMARY_PROJECTILE_DAMAGE_PACKET;
    use crate::entity::EntityKind;
    use crate::entity_collision_state::RetailStateWord;
    use crate::impact_reaction::ImpactReactionSuppression;
    use crate::ordinary_type47_death_live::TYPE47_COMMON_DYING_MASS_RAW;

    const ENTITY_ID: u32 = 0x042F_000B;
    const DIRECTION: [i16; 3] = [0x1000, 0, -0x1000];

    fn type47_entity(state_flags: u32) -> Entity {
        let mut entity = Entity::unresolved_port_entity(ENTITY_ID, EntityKind::Enemy, 47);
        entity.authored_spawn_index = Some(11);
        entity.model_slots = [Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID); 4];
        entity.model_index = Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID);
        entity.mass_raw = TYPE47_COMMON_DYING_MASS_RAW;
        entity.collision.state_flags_at_0x08 = RetailStateWord::exact(state_flags);
        entity.set_velocity_raw([10, 20, 30]);
        entity.set_rotation_heading_pitch_roll_raw([100, 200, 300]);
        entity
    }

    #[test]
    fn typical_type47_state_suppresses_without_rng() {
        let mut manager = EntityManager::from_entities_for_test(vec![type47_entity(0x0142_8805)]);
        let mut world_fx = WorldFx::new();
        let mut oracle = WorldFx::new();
        let expected_first = oracle.next_shared_retail_random_u16();
        let outcome = apply_type47_impact_reaction_after_c690(
            &mut manager,
            &mut world_fx,
            ENTITY_ID,
            PRIMARY_PROJECTILE_DAMAGE_PACKET.impact_sum_raw(),
            DIRECTION,
        );
        assert!(matches!(
            outcome,
            Type47ImpactReactionOutcome::Applied(ImpactReactionOutcome::Suppressed(
                ImpactReactionSuppression::EnableBitClear
            ))
        ));
        assert_eq!(world_fx.next_shared_retail_random_u16(), expected_first);
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap();
        assert_eq!(entity.velocity_raw(), [10, 20, 30]);
        assert_eq!(entity.rotation_heading_pitch_roll_raw(), [100, 200, 300]);
    }

    #[test]
    fn enabled_reaction_commits_impulse_without_a_network_request() {
        let mut manager = EntityManager::from_entities_for_test(vec![type47_entity(
            0x0142_8805 | IMPACT_REACTION_ENABLED_STATE_BIT,
        )]);
        let mut world_fx = WorldFx::new();
        let mut oracle = WorldFx::new();
        let _ = oracle.next_shared_retail_random_u16();
        let _ = oracle.next_shared_retail_random_u16();
        let _ = oracle.next_shared_retail_random_u16();
        let expected_fourth = oracle.next_shared_retail_random_u16();
        let outcome = apply_type47_impact_reaction_after_c690(
            &mut manager,
            &mut world_fx,
            ENTITY_ID,
            PRIMARY_PROJECTILE_DAMAGE_PACKET.impact_sum_raw(),
            DIRECTION,
        );
        assert!(matches!(
            outcome,
            Type47ImpactReactionOutcome::Applied(ImpactReactionOutcome::Applied(applied))
                if applied.network_request.is_none()
        ));
        assert_eq!(world_fx.next_shared_retail_random_u16(), expected_fourth);
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap();
        assert_ne!(entity.velocity_raw(), [10, 20, 30]);
        assert_ne!(entity.rotation_heading_pitch_roll_raw(), [100, 200, 300]);
    }

    #[test]
    fn networked_bit_does_not_invent_fun_00469200() {
        let mut manager = EntityManager::from_entities_for_test(vec![type47_entity(
            IMPACT_REACTION_ENABLED_STATE_BIT | IMPACT_REACTION_NETWORKED_STATE_BIT,
        )]);
        let mut world_fx = WorldFx::new();
        assert_eq!(
            apply_type47_impact_reaction_after_c690(
                &mut manager,
                &mut world_fx,
                ENTITY_ID,
                2_000,
                DIRECTION,
            ),
            Type47ImpactReactionOutcome::Blocked {
                entity_id: ENTITY_ID,
                reason: Type47ImpactReactionBlock::NetworkRequest,
            }
        );
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap();
        assert_eq!(entity.velocity_raw(), [10, 20, 30]);
        assert_eq!(entity.rotation_heading_pitch_roll_raw(), [100, 200, 300]);
    }

    #[test]
    fn other_entity_types_are_not_applicable() {
        let mut entity = type47_entity(0x0142_8805);
        entity.entity_type = 17;
        let mut manager = EntityManager::from_entities_for_test(vec![entity]);
        let mut world_fx = WorldFx::new();
        assert_eq!(
            apply_type47_impact_reaction_after_c690(
                &mut manager,
                &mut world_fx,
                ENTITY_ID,
                2_000,
                DIRECTION,
            ),
            Type47ImpactReactionOutcome::NotApplicable
        );
    }
}
