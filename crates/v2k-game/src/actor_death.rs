//! Inert type-17/type-47 primary-hit and common-dying plan.
//!
//! Retail invokes the current style's `+0x28` before impact reaction and
//! checked damage. That callback can change the style, so the later `+0x2C`
//! death cleanup must use a separately supplied post-impact style. This module
//! records that order without dispatching callbacks, consuming RNG, publishing
//! tasks, playing audio, or mutating an entity.

use crate::{
    damage::{
        generic_entity_damage_transition, GenericEntityDamageStage, GenericEntityDamageState,
        GenericEntityDamageTransition,
    },
    entity_behavior::{
        audited_behavior_style, BehaviorStyle, DeathCallbackPolicy, ImpactCallbackPolicy,
    },
};

pub use crate::entity_behavior::{
    COMMON_ACTOR_DYING_ACTIVE_STYLE, COMMON_ACTOR_DYING_COMPLETION_STYLE,
};

/// Exact class-12 `"Flip Over And Die"` data shared by types 17 and 47.
///
/// The record intentionally contains only identities required to recognize the
/// program and its authored asynchronous timeout. Focused type-17 captures on
/// 2026-07-31 dynamically validate the style/task identities and +500 launch,
/// but also show an actor unlinking near 7.86 seconds without reaching the
/// strict `> 9000` completion. This duration must not be treated as an
/// unconditional corpse lifetime. Address-by-address call provenance remains
/// in `GAME_MECHANICS.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorDyingProgram {
    pub behavior_class_id: u8,
    pub active_style_address: u32,
    pub task_tick_address: u32,
    pub task_duration_ms: u32,
    pub initial_vertical_velocity_raw: i16,
    pub completion_style_address: u32,
    pub common_mover_address: u32,
}

pub const COMMON_ACTOR_DYING_PROGRAM: ActorDyingProgram = ActorDyingProgram {
    behavior_class_id: 12,
    active_style_address: 0x004C_7ED0,
    task_tick_address: 0x0040_4220,
    task_duration_ms: 9_000,
    initial_vertical_velocity_raw: 500,
    completion_style_address: 0x004C_7F18,
    common_mover_address: 0x0040_1430,
};

/// Raw style `+0x38` policies. `FUN_0040EA10` applies this field with the
/// translated set/clear orientation reversed.
pub const COMMON_ACTOR_DYING_ACTIVE_DISABLE_POLICY: u32 = 0x0000_2015;
pub const COMMON_ACTOR_DYING_COMPLETION_DISABLE_POLICY: u32 = 0x0000_2011;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonEnemyActorType {
    SpiderType17,
    NewantType47,
}

impl CommonEnemyActorType {
    const fn authors_behavior_class(self, class_id: u8) -> bool {
        match self {
            Self::SpiderType17 => matches!(class_id, 9 | 10 | 33),
            Self::NewantType47 => matches!(class_id, 6 | 32),
        }
    }
}

/// Evidence needed to classify one already accepted lethal primary hit.
///
/// `accepted_damage_raw` is the nonzero post-filter/post-modifier amount.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorPrimaryLethalHitRequest {
    pub actor_type: CommonEnemyActorType,
    pub pre_impact_style: BehaviorStyle,
    pub post_impact_style: BehaviorStyle,
    pub generic_damage_state: GenericEntityDamageState,
    pub accepted_damage_raw: i32,
}

/// Live owners deliberately excluded from this checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorHitDeathUnresolvedBoundary {
    CurrentBehaviorStyleAndContext,
    SharedBehaviorReselection {
        rng_words: u8,
    },
    CapturePeopleImpactCleanupAndPossibleReselection,
    /// The arithmetic is recovered in [`crate::impact_reaction`]; this plan
    /// still lacks the authenticated hit coordinator, exact caller-supplied
    /// direction, and process-global RNG borrow needed to commit it at the
    /// exact point in the hit transaction. [`crate::entity::Entity`] now
    /// retains the three live angular words required by that future binding.
    ImpactReactionLiveBindingAndSharedRng {
        enabled_branch_rng_words: u8,
    },
    CheckedDamageAdmissionModifierAndBackends,
    /// The class-12 transaction is recovered in [`crate::common_dying`], and
    /// [`crate::actor_standard_death_live`] binds the exact fresh-Level-1
    /// type-17 null-death callback to live storage. This detached cross-type
    /// planner still lacks production routing into that bounded publisher and
    /// the equivalent type-47/general actor adapters.
    DyingTaskLiveBindingAndComponentAdapters,
    DyingSchedulerAudioEnvironmentAndIntegration,
    CommonMover,
    /// Dedicated type-17 evidence adds an approximately 7.86-second unlink to
    /// earlier approximately 3.1/4.1-second cases, all before the authored
    /// strict timeout and without a proven deferred-destroy transition.
    UnresolvedEarlyRemovalCause,
}

/// Small result: arithmetic, both style policies, and remaining live owners.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorPrimaryHitDeathPlan {
    pub actor_type: CommonEnemyActorType,
    pub impact_policy: ImpactCallbackPolicy,
    pub post_impact_death_policy: DeathCallbackPolicy,
    pub generic_damage: GenericEntityDamageTransition,
    pub unresolved_boundaries: Vec<ActorHitDeathUnresolvedBoundary>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorHitDeathPlanError {
    ZeroAcceptedDamage,
    AlreadyDying,
    NonLethal { health_after_raw: i32 },
    UnauthenticatedPreImpactStyle,
    UnauthenticatedPostImpactStyle,
    UnknownImpactCallback { address: u32 },
    StyleChangedAfterNullImpactCallback,
    CaptureDeathMayInitializeDyingProgramTwice,
    UnsupportedDeathCallback { policy: DeathCallbackPolicy },
}

fn actor_style_is_authenticated(actor_type: CommonEnemyActorType, style: BehaviorStyle) -> bool {
    actor_type.authors_behavior_class(style.class_id)
        && audited_behavior_style(style.class_id.into(), style.variant) == Some(&style)
}

/// Plan the common type-17/type-47 lethal branch without executing it.
pub fn plan_type_17_or_47_primary_lethal_hit(
    request: ActorPrimaryLethalHitRequest,
) -> Result<ActorPrimaryHitDeathPlan, ActorHitDeathPlanError> {
    if request.accepted_damage_raw == 0 {
        return Err(ActorHitDeathPlanError::ZeroAcceptedDamage);
    }

    if !actor_style_is_authenticated(request.actor_type, request.pre_impact_style) {
        return Err(ActorHitDeathPlanError::UnauthenticatedPreImpactStyle);
    }
    if !actor_style_is_authenticated(request.actor_type, request.post_impact_style) {
        return Err(ActorHitDeathPlanError::UnauthenticatedPostImpactStyle);
    }

    let impact_policy = request.pre_impact_style.impact_callback_policy();
    if let ImpactCallbackPolicy::UnknownAddress(address) = impact_policy {
        return Err(ActorHitDeathPlanError::UnknownImpactCallback { address });
    }
    if impact_policy == ImpactCallbackPolicy::None
        && request.pre_impact_style != request.post_impact_style
    {
        return Err(ActorHitDeathPlanError::StyleChangedAfterNullImpactCallback);
    }

    let post_impact_death_policy = request.post_impact_style.death_callback_policy();
    match post_impact_death_policy {
        DeathCallbackPolicy::None => {}
        DeathCallbackPolicy::CapturePeopleCleanup => {
            return Err(ActorHitDeathPlanError::CaptureDeathMayInitializeDyingProgramTwice);
        }
        policy => return Err(ActorHitDeathPlanError::UnsupportedDeathCallback { policy }),
    }

    let generic_damage =
        generic_entity_damage_transition(request.generic_damage_state, request.accepted_damage_raw);
    match generic_damage.stage {
        GenericEntityDamageStage::AlreadyDying => {
            return Err(ActorHitDeathPlanError::AlreadyDying);
        }
        GenericEntityDamageStage::Survived => {
            return Err(ActorHitDeathPlanError::NonLethal {
                health_after_raw: generic_damage.health_after_subtraction_raw,
            });
        }
        GenericEntityDamageStage::DeathDispatchRequired => {}
    }

    let mut unresolved_boundaries =
        vec![ActorHitDeathUnresolvedBoundary::CurrentBehaviorStyleAndContext];
    match impact_policy {
        ImpactCallbackPolicy::None => {}
        ImpactCallbackPolicy::ReselectBehavior => unresolved_boundaries
            .push(ActorHitDeathUnresolvedBoundary::SharedBehaviorReselection { rng_words: 1 }),
        ImpactCallbackPolicy::CapturePeopleCleanup => unresolved_boundaries.push(
            ActorHitDeathUnresolvedBoundary::CapturePeopleImpactCleanupAndPossibleReselection,
        ),
        ImpactCallbackPolicy::UnknownAddress(_) => unreachable!("rejected above"),
    }
    unresolved_boundaries.extend([
        ActorHitDeathUnresolvedBoundary::ImpactReactionLiveBindingAndSharedRng {
            enabled_branch_rng_words: 3,
        },
        ActorHitDeathUnresolvedBoundary::CheckedDamageAdmissionModifierAndBackends,
    ]);
    unresolved_boundaries.extend([
        ActorHitDeathUnresolvedBoundary::DyingTaskLiveBindingAndComponentAdapters,
        ActorHitDeathUnresolvedBoundary::DyingSchedulerAudioEnvironmentAndIntegration,
        ActorHitDeathUnresolvedBoundary::CommonMover,
        ActorHitDeathUnresolvedBoundary::UnresolvedEarlyRemovalCause,
    ]);

    Ok(ActorPrimaryHitDeathPlan {
        actor_type: request.actor_type,
        impact_policy,
        post_impact_death_policy,
        generic_damage,
        unresolved_boundaries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style(class_id: u32, variant: u8) -> BehaviorStyle {
        *audited_behavior_style(class_id, variant).unwrap()
    }

    fn request(
        actor_type: CommonEnemyActorType,
        pre: BehaviorStyle,
        post: BehaviorStyle,
    ) -> ActorPrimaryLethalHitRequest {
        ActorPrimaryLethalHitRequest {
            actor_type,
            pre_impact_style: pre,
            post_impact_style: post,
            generic_damage_state: GenericEntityDamageState {
                health_raw: 1_400,
                pre_health_buffer_raw: 0,
                already_dying: false,
            },
            accepted_damage_raw: 1_800,
        }
    }

    fn blocked(plan: &ActorPrimaryHitDeathPlan, boundary: ActorHitDeathUnresolvedBoundary) -> bool {
        plan.unresolved_boundaries.contains(&boundary)
    }

    #[test]
    fn dying_program_pins_the_static_checkpoint() {
        let program = COMMON_ACTOR_DYING_PROGRAM;
        assert_eq!(program.behavior_class_id, 12);
        assert_eq!(program.active_style_address, 0x004C_7ED0);
        assert_eq!(program.task_tick_address, 0x0040_4220);
        assert_eq!(program.task_duration_ms, 9_000);
        assert_eq!(program.initial_vertical_velocity_raw, 500);
        assert_eq!(program.completion_style_address, 0x004C_7F18);
        assert_eq!(program.common_mover_address, 0x0040_1430);
        assert_eq!(COMMON_ACTOR_DYING_ACTIVE_STYLE.class_id, 12);
        assert_eq!(COMMON_ACTOR_DYING_ACTIVE_STYLE.variant, 0);
        assert_eq!(COMMON_ACTOR_DYING_ACTIVE_STYLE.frame_address, 0x004C_7ED0);
        assert_eq!(COMMON_ACTOR_DYING_COMPLETION_STYLE.variant, 1);
        assert_eq!(
            COMMON_ACTOR_DYING_COMPLETION_STYLE.frame_address,
            0x004C_7F18
        );
        assert_eq!(COMMON_ACTOR_DYING_ACTIVE_DISABLE_POLICY, 0x2015);
        assert_eq!(COMMON_ACTOR_DYING_COMPLETION_DISABLE_POLICY, 0x2011);
    }

    #[test]
    fn reselection_plan_retains_arithmetic_rng_and_live_blockers() {
        let plan = plan_type_17_or_47_primary_lethal_hit(request(
            CommonEnemyActorType::NewantType47,
            style(6, 0),
            style(32, 0),
        ))
        .unwrap();
        assert_eq!(plan.actor_type, CommonEnemyActorType::NewantType47);
        assert_eq!(plan.impact_policy, ImpactCallbackPolicy::ReselectBehavior);
        assert_eq!(plan.post_impact_death_policy, DeathCallbackPolicy::None);
        assert_eq!(plan.generic_damage.health_after_subtraction_raw, -400);
        assert!(blocked(
            &plan,
            ActorHitDeathUnresolvedBoundary::SharedBehaviorReselection { rng_words: 1 }
        ));
        assert!(blocked(
            &plan,
            ActorHitDeathUnresolvedBoundary::ImpactReactionLiveBindingAndSharedRng {
                enabled_branch_rng_words: 3
            }
        ));
        assert!(blocked(&plan, ActorHitDeathUnresolvedBoundary::CommonMover));
        assert!(blocked(
            &plan,
            ActorHitDeathUnresolvedBoundary::UnresolvedEarlyRemovalCause
        ));
    }

    #[test]
    fn null_impact_cannot_silently_change_style() {
        let run_away = style(10, 1);
        assert_eq!(
            plan_type_17_or_47_primary_lethal_hit(request(
                CommonEnemyActorType::SpiderType17,
                run_away,
                style(33, 1),
            )),
            Err(ActorHitDeathPlanError::StyleChangedAfterNullImpactCallback)
        );
        let plan = plan_type_17_or_47_primary_lethal_hit(request(
            CommonEnemyActorType::SpiderType17,
            run_away,
            run_away,
        ))
        .unwrap();
        assert!(!plan.unresolved_boundaries.iter().any(|boundary| matches!(
            boundary,
            ActorHitDeathUnresolvedBoundary::SharedBehaviorReselection { .. }
                | ActorHitDeathUnresolvedBoundary::CapturePeopleImpactCleanupAndPossibleReselection
        )));
    }

    #[test]
    fn capture_impact_is_bounded_but_capture_death_fails_closed() {
        let capture = style(9, 2);
        let plan = plan_type_17_or_47_primary_lethal_hit(request(
            CommonEnemyActorType::SpiderType17,
            capture,
            style(10, 1),
        ))
        .unwrap();
        assert_eq!(
            plan.impact_policy,
            ImpactCallbackPolicy::CapturePeopleCleanup
        );
        assert_eq!(plan.post_impact_death_policy, DeathCallbackPolicy::None);
        assert!(blocked(
            &plan,
            ActorHitDeathUnresolvedBoundary::CapturePeopleImpactCleanupAndPossibleReselection
        ));
        assert_eq!(
            plan_type_17_or_47_primary_lethal_hit(request(
                CommonEnemyActorType::SpiderType17,
                capture,
                capture,
            )),
            Err(ActorHitDeathPlanError::CaptureDeathMayInitializeDyingProgramTwice)
        );
    }

    #[test]
    fn forged_or_wrong_actor_styles_and_nonlethal_hits_fail_closed() {
        let mut forged = style(6, 0);
        forged.impact_callback_address = None;
        assert_eq!(
            plan_type_17_or_47_primary_lethal_hit(request(
                CommonEnemyActorType::NewantType47,
                forged,
                style(6, 0),
            )),
            Err(ActorHitDeathPlanError::UnauthenticatedPreImpactStyle)
        );
        assert_eq!(
            plan_type_17_or_47_primary_lethal_hit(request(
                CommonEnemyActorType::NewantType47,
                style(24, 0),
                style(6, 0),
            )),
            Err(ActorHitDeathPlanError::UnauthenticatedPreImpactStyle)
        );

        let wander = style(6, 0);
        let mut nonlethal = request(CommonEnemyActorType::NewantType47, wander, wander);
        nonlethal.generic_damage_state.health_raw = 3_000;
        assert_eq!(
            plan_type_17_or_47_primary_lethal_hit(nonlethal),
            Err(ActorHitDeathPlanError::NonLethal {
                health_after_raw: 1_200
            })
        );
    }
}
