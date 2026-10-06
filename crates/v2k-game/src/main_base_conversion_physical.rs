//! Bounded post-component physical suffix for Main Base conversion.
//!
//! The accepted
//! `20260727-235038-base-conversion-pair-tail.txt` trace proves that the type-6
//! Main Base remains fixed while the accepted type-9 source receives the
//! normal-projected response after its descriptor component has run. Both
//! health values remain unchanged and retail skips both directional damage
//! calls because the ordered shared cap is zero.
//!
//! This module preflights the no-damage suffix, including a nonzero shared cap
//! which both local `15040 -> 255E0` directional filters reduce to zero before
//! any modifier/hit callback. Actual damage and remote delivery remain blocked
//! before event 1, deferred destruction, or replacement allocation is visible.

use crate::active_pair::{
    plan_active_pair_response_and_damage_cap, ActivePairBody, ActivePairContact,
    ActivePairModelState, ActivePairPhysicalPlan, ActivePairUnresolved,
};
use crate::damage::DamagePacket;
use crate::entity::Entity;
use crate::entity_collision_state::{
    pair_collision_response_is_fixed, RetailRuntimeValue, PAIR_COLLISION_FIXED_STATE_BIT,
    REMOTE_OWNED_STATE_BIT,
};
use crate::main_base_type9_component::FIRST_WORLD_MAIN_BASE_SOURCE_ENTITY_TYPE;
use crate::player_active_contact::active_pair_body_from_entity;
use v2k_formats::models::CollisionModelPool;

/// First-world Main Base entity type observed as the subject of the accepted
/// conversion pair.
pub const FIRST_WORLD_MAIN_BASE_ENTITY_TYPE: u32 = 6;

/// Immutable, fully closed no-damage physical suffix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainBaseType9PhysicalPlan {
    expected_main_base: ActivePairBody,
    expected_source: ActivePairBody,
    physical: ActivePairPhysicalPlan,
}

impl MainBaseType9PhysicalPlan {
    pub fn expected_main_base(&self) -> &ActivePairBody {
        &self.expected_main_base
    }

    pub fn expected_source(&self) -> &ActivePairBody {
        &self.expected_source
    }

    pub fn resolved_main_base(&self) -> &ActivePairBody {
        &self.physical.subject
    }

    pub fn resolved_source(&self) -> &ActivePairBody {
        &self.physical.candidate
    }

    pub const fn contact(&self) -> ActivePairContact {
        self.physical.contact
    }

    pub const fn subject_impact_raw(&self) -> i32 {
        self.physical.subject_impact_raw
    }

    pub const fn source_impact_raw(&self) -> i32 {
        self.physical.candidate_impact_raw
    }

    pub const fn combined_impact_raw(&self) -> i32 {
        self.physical.combined_impact_raw
    }

    pub const fn capped_pair_damage_raw(&self) -> i32 {
        self.physical.capped_pair_damage_raw
    }
}

/// Why the captured physical suffix could not be closed before callback
/// actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType9PhysicalPlanError {
    WrongMainBaseType {
        entity_type: u32,
    },
    WrongSourceType {
        entity_type: u32,
    },
    SameEntity {
        entity_id: u32,
    },
    MissingActiveModel {
        entity_id: u32,
        active_slot: usize,
        global_id: Option<usize>,
    },
    ActivePair(ActivePairUnresolved),
    UnexpectedFixedness {
        entity_id: u32,
        expected_fixed: bool,
        actual_fixed: bool,
    },
    /// At least one directional delivery cannot be proven to return before
    /// modifier/hit callbacks, which require a separately owned transaction.
    DirectionalDamageRequired {
        capped_pair_damage_raw: i32,
    },
}

/// Snapshot and preflight the accepted type-6/type-9 physical suffix.
///
/// The supplied contact must be the exact oriented Section-8 result already
/// classified for these same live entities. The model pool is sampled here so
/// a later atomic commit can reject a stale active-model binding along with
/// stale position, velocity, mass, and collision state.
pub fn plan_main_base_type9_physical_suffix<P: CollisionModelPool + ?Sized>(
    main_base: &Entity,
    source: &Entity,
    model_pool: &P,
    contact: ActivePairContact,
) -> Result<MainBaseType9PhysicalPlan, MainBaseType9PhysicalPlanError> {
    if main_base.entity_type != FIRST_WORLD_MAIN_BASE_ENTITY_TYPE {
        return Err(MainBaseType9PhysicalPlanError::WrongMainBaseType {
            entity_type: main_base.entity_type,
        });
    }
    if source.entity_type != FIRST_WORLD_MAIN_BASE_SOURCE_ENTITY_TYPE {
        return Err(MainBaseType9PhysicalPlanError::WrongSourceType {
            entity_type: source.entity_type,
        });
    }
    if main_base.id == source.id {
        return Err(MainBaseType9PhysicalPlanError::SameEntity {
            entity_id: main_base.id,
        });
    }

    let expected_main_base = active_pair_body_from_entity(main_base, model_pool);
    let expected_source = active_pair_body_from_entity(source, model_pool);
    plan_main_base_type9_physical_bodies(expected_main_base, expected_source, contact)
}

/// Shared body-level response used by the captured Type9 fixture and native
/// person conversions. Type/profile admission belongs to the live caller;
/// model, mass, collision filters and fixedness come from these actual bodies.
pub(crate) fn plan_main_base_type9_physical_bodies(
    expected_main_base: ActivePairBody,
    expected_source: ActivePairBody,
    contact: ActivePairContact,
) -> Result<MainBaseType9PhysicalPlan, MainBaseType9PhysicalPlanError> {
    require_active_model(&expected_main_base)?;
    require_active_model(&expected_source)?;

    let physical = plan_active_pair_response_and_damage_cap(
        expected_main_base.clone(),
        expected_source.clone(),
        contact,
    )
    .map_err(MainBaseType9PhysicalPlanError::ActivePair)?;

    require_fixedness(&expected_main_base, true)?;
    require_fixedness(&expected_source, false)?;
    // 15040 returns before modifier/14E90 when 255E0 filters the packet to
    // zero. Its remaining resource-message branch also rejects the pair's
    // exact channel words [1, 0], so these calls have no retained side effects.
    let packet = DamagePacket::collision(physical.capped_pair_damage_raw);
    let both_directional_deliveries_filter_out = [&physical.subject, &physical.candidate]
        .into_iter()
        .all(|body| {
            body.collision
                .state_flags_at_0x08
                .masked(REMOTE_OWNED_STATE_BIT)
                == RetailRuntimeValue::Known(0)
                && matches!(body.collision.damage_profile,
                    RetailRuntimeValue::Known(profile) if profile.filter(packet) == 0)
        });
    if physical.capped_pair_damage_raw != 0 && !both_directional_deliveries_filter_out {
        return Err(MainBaseType9PhysicalPlanError::DirectionalDamageRequired {
            capped_pair_damage_raw: physical.capped_pair_damage_raw,
        });
    }

    Ok(MainBaseType9PhysicalPlan {
        expected_main_base,
        expected_source,
        physical,
    })
}

fn require_active_model(body: &ActivePairBody) -> Result<(), MainBaseType9PhysicalPlanError> {
    match body.active_model {
        ActivePairModelState::Resolved(_) => Ok(()),
        ActivePairModelState::Missing {
            active_slot,
            global_id,
        } => Err(MainBaseType9PhysicalPlanError::MissingActiveModel {
            entity_id: body.id,
            active_slot,
            global_id,
        }),
    }
}

fn require_fixedness(
    body: &ActivePairBody,
    expected_fixed: bool,
) -> Result<(), MainBaseType9PhysicalPlanError> {
    let RetailRuntimeValue::Known(state) = body
        .collision
        .state_flags_at_0x08
        .masked(PAIR_COLLISION_FIXED_STATE_BIT)
    else {
        unreachable!("the shared physical planner already rejected unknown fixedness");
    };
    let actual_fixed = pair_collision_response_is_fixed(state);
    if actual_fixed == expected_fixed {
        Ok(())
    } else {
        Err(MainBaseType9PhysicalPlanError::UnexpectedFixedness {
            entity_id: body.id,
            expected_fixed,
            actual_fixed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::active_pair::{ActivePairModel, ActivePairModelState};
    use crate::damage::DamageProfile;
    use crate::entity_collision_state::{
        EntityCollisionRuntimeState, RetailStateWord, PAIR_COLLISION_ENABLED_STATE_BIT,
    };

    const IDENTITY_DAMAGE: DamageProfile = DamageProfile {
        thresholds_raw: [0; 7],
        multipliers_q8: [256; 7],
    };
    const MAIN_BASE_ID: u32 = 0x04B5_0001;
    const SOURCE_ID: u32 = 0x04AC_0001;

    fn body(
        id: u32,
        position_raw: [i16; 3],
        velocity_raw: [i16; 3],
        mass_raw: u16,
        fixed: bool,
        health_raw: i32,
    ) -> ActivePairBody {
        let fixed_state = if fixed {
            PAIR_COLLISION_FIXED_STATE_BIT
        } else {
            0
        };
        ActivePairBody {
            id,
            position_raw,
            velocity_raw,
            mass_raw,
            active_model: ActivePairModelState::Resolved(ActivePairModel {
                active_slot: 0,
                global_id: id as usize,
                collision_radius_raw: 100,
            }),
            collision: EntityCollisionRuntimeState {
                health_raw: RetailRuntimeValue::Known(health_raw),
                pre_health_damage_buffer_raw: RetailRuntimeValue::Known(0),
                damage_profile: RetailRuntimeValue::Known(IDENTITY_DAMAGE),
                state_flags_at_0x08: RetailStateWord::exact(
                    PAIR_COLLISION_ENABLED_STATE_BIT | fixed_state,
                ),
                ..EntityCollisionRuntimeState::unresolved_port_entity(0)
            },
        }
    }

    #[test]
    fn captured_no_damage_shape_preflights_without_changing_health() {
        let main_base = body(
            MAIN_BASE_ID,
            [0x5000, -0x0300, 0x3c00],
            [0; 3],
            1_000,
            true,
            99_999,
        );
        let source = body(
            SOURCE_ID,
            [0x4e17, -0x02e5, 0x3cdf],
            [0x000c, 0, -0x0098],
            10,
            false,
            1_500,
        );
        // A Q12 normal reconstructed only for arithmetic coverage. The
        // production contact comes from the exact oriented Section-8 probe.
        let contact = ActivePairContact {
            normal_q12: [-2_985, 2_467, 1_328],
            penetration_raw: 1,
        };

        let plan = plan_main_base_type9_physical_bodies(main_base.clone(), source.clone(), contact)
            .expect("accepted conversion suffix is zero-damage");

        assert_eq!(plan.expected_main_base(), &main_base);
        assert_eq!(plan.expected_source(), &source);
        assert_eq!(
            plan.resolved_main_base().position_raw,
            main_base.position_raw
        );
        assert_eq!(plan.resolved_main_base().velocity_raw, [0; 3]);
        assert_eq!(
            plan.resolved_source().velocity_raw,
            [-0x0019, 0x001f, -0x0087],
            "the captured type-9 response is reproduced by the shared fixed-body arithmetic"
        );
        assert_eq!(plan.subject_impact_raw(), 0);
        assert_eq!(plan.source_impact_raw(), 0);
        assert_eq!(plan.combined_impact_raw(), 0);
        assert_eq!(
            plan.resolved_source().collision.health_raw,
            RetailRuntimeValue::Known(1_500)
        );
    }

    #[test]
    fn nonzero_shared_damage_fails_before_a_plan_is_published() {
        let main_base = body(MAIN_BASE_ID, [0; 3], [0; 3], 1_000, true, 99_999);
        let source = body(SOURCE_ID, [0; 3], [4_000, 0, 0], 1_000, false, 1_500);
        let contact = ActivePairContact {
            normal_q12: [4_096, 0, 0],
            penetration_raw: 0,
        };

        assert!(matches!(
            plan_main_base_type9_physical_bodies(main_base, source, contact),
            Err(MainBaseType9PhysicalPlanError::DirectionalDamageRequired {
                capped_pair_damage_raw
            }) if capped_pair_damage_raw > 0
        ));
    }

    #[test]
    fn nonzero_cap_is_safe_only_when_both_local_directional_filters_return_zero() {
        let mut main_base = body(MAIN_BASE_ID, [0; 3], [0; 3], 1_000, true, 99_999);
        let mut source = body(SOURCE_ID, [0; 3], [512, 0, 0], 10, false, 1_500);
        let profile = crate::damage::TYPE_9_DAMAGE_PROFILE;
        main_base.collision.damage_profile = RetailRuntimeValue::Known(profile);
        source.collision.damage_profile = RetailRuntimeValue::Known(profile);
        let contact = ActivePairContact {
            normal_q12: [4_096, 0, 0],
            penetration_raw: 0,
        };
        let plan = plan_main_base_type9_physical_bodies(main_base.clone(), source.clone(), contact)
            .expect("both directional filters return before all callback writes");
        assert!(plan.capped_pair_damage_raw() > 0);
        assert_eq!(
            plan.resolved_main_base().collision.health_raw,
            main_base.collision.health_raw
        );
        assert_eq!(
            plan.resolved_source().collision.health_raw,
            source.collision.health_raw
        );
        for target_is_source in [false, true] {
            let (mut base, mut actor) = (main_base.clone(), source.clone());
            let target = if target_is_source {
                &mut actor
            } else {
                &mut base
            };
            target.collision.damage_profile = RetailRuntimeValue::Known(IDENTITY_DAMAGE);
            assert!(matches!(
                plan_main_base_type9_physical_bodies(base, actor, contact),
                Err(MainBaseType9PhysicalPlanError::DirectionalDamageRequired { .. })
            ));
        }
        for ownership in [
            RetailStateWord::exact(REMOTE_OWNED_STATE_BIT),
            RetailStateWord::unknown(),
        ] {
            let mut actor = source.clone();
            actor.collision.state_flags_at_0x08 = RetailStateWord::from_known_bits(
                actor.collision.state_flags_at_0x08.known_value_bits()
                    | ownership.known_value_bits(),
                (u32::MAX & !REMOTE_OWNED_STATE_BIT) | ownership.known_mask(),
            );
            assert!(matches!(
                plan_main_base_type9_physical_bodies(main_base.clone(), actor, contact),
                Err(MainBaseType9PhysicalPlanError::DirectionalDamageRequired { .. })
            ));
        }
    }

    #[test]
    fn bounded_shape_rejects_wrong_fixedness_and_missing_models() {
        let main_base = body(MAIN_BASE_ID, [0; 3], [0; 3], 1_000, false, 99_999);
        let source = body(SOURCE_ID, [0; 3], [0; 3], 10, false, 1_500);
        let contact = ActivePairContact {
            normal_q12: [4_096, 0, 0],
            penetration_raw: 0,
        };
        assert_eq!(
            plan_main_base_type9_physical_bodies(main_base, source, contact),
            Err(MainBaseType9PhysicalPlanError::UnexpectedFixedness {
                entity_id: MAIN_BASE_ID,
                expected_fixed: true,
                actual_fixed: false,
            })
        );

        let main_base = body(MAIN_BASE_ID, [0; 3], [0; 3], 1_000, true, 99_999);
        let mut source = body(SOURCE_ID, [0; 3], [0; 3], 10, false, 1_500);
        source.active_model = ActivePairModelState::Missing {
            active_slot: 0,
            global_id: Some(558),
        };
        assert_eq!(
            plan_main_base_type9_physical_bodies(main_base, source, contact),
            Err(MainBaseType9PhysicalPlanError::MissingActiveModel {
                entity_id: SOURCE_ID,
                active_slot: 0,
                global_id: Some(558),
            })
        );
    }
}
