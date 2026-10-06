//! Type-13 DCA0/E870 body-basis publication after the complete task traversal.
//!
//! Both retail wrappers latch `(entity +0xC8 | style +0x34) & ~style +0x38`
//! before A800. Intro2 Type 13 authors `+0xC8 = 8`; the class-5 and class-7
//! masks below preserve that value. E640 (`0x10`) remains an unported boundary.

use super::{
    Intro2Type13ChaseVisitResult, Intro2Type13SchedulerOwnerTick,
    Intro2Type13SchedulerProductionOutcome, Intro2Type13SchedulerStage,
};
use crate::common_mover::type9_attitude::{Type9BodyBasis, TERRAIN_ATTITUDE_EFFECTIVE_FLAG};
use crate::entity::Entity;
use crate::entity_collision_state::{RetailRuntimeValue, BODY_BASIS_REBUILT_STATE_BIT};
use crate::search_attack_live::SearchAttackLiveAcquisitionOutcome;

const BODY_BASIS_REBUILD_SUPPRESSED_EFFECTIVE_FLAG: u32 = 0x4000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type13PostTaskBasisBlock {
    EffectiveFlagsUnavailable,
    BehaviorContextUnavailable,
    UnsupportedStyle { style_address: u32 },
    TerrainAttitudeUnsupported { effective_flags: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Type13PostTaskBasisFrame {
    pub(super) effective_flags: u32,
}

impl Type13PostTaskBasisFrame {
    pub(super) fn capture(entity: &Entity) -> Result<Self, Intro2Type13PostTaskBasisBlock> {
        let RetailRuntimeValue::Known(default_flags) = entity.collision.default_state_flags_at_0xc8
        else {
            return Err(Intro2Type13PostTaskBasisBlock::EffectiveFlagsUnavailable);
        };
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(Intro2Type13PostTaskBasisBlock::BehaviorContextUnavailable);
        };
        // Audited executable style +0x34/+0x38. All three enable masks are zero.
        let style_address = context.active_style().style_address();
        let disable_flags = match style_address {
            0x004C_7930 => 0,           // class 5: Move About Aimlessly
            0x004C_7A50 => 0x0002_1080, // class 7/v0: acquisition
            0x004C_7A98 => 0x0000_0080, // class 7/v1: pursuing
            _ => {
                return Err(Intro2Type13PostTaskBasisBlock::UnsupportedStyle { style_address });
            }
        };
        let effective_flags = default_flags & !disable_flags;
        if effective_flags & TERRAIN_ATTITUDE_EFFECTIVE_FLAG != 0 {
            return Err(Intro2Type13PostTaskBasisBlock::TerrainAttitudeUnsupported {
                effective_flags,
            });
        }
        Ok(Self { effective_flags })
    }

    pub(super) fn publish(self, entity: &mut Entity) {
        if self.effective_flags & BODY_BASIS_REBUILD_SUPPRESSED_EFFECTIVE_FLAG == 0 {
            // FUN_00413F70 reads the post-task angle words only after every
            // resumed root transition and same-pass Secondary/Tertiary visit.
            let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
            entity.physical_body_basis_q31 =
                RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(heading, pitch, roll));
            entity
                .collision
                .state_flags_at_0x08
                .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
        }
    }
}

fn acquisition_completed(acquisition: Option<&SearchAttackLiveAcquisitionOutcome>) -> bool {
    matches!(
        acquisition,
        None | Some(SearchAttackLiveAcquisitionOutcome::Applied { .. })
    )
}

pub(super) fn task_traversal_completed(tick: &Intro2Type13SchedulerOwnerTick) -> bool {
    if !tick
        .retained_owner
        .is_some_and(|owner| matches!(owner.stage, Intro2Type13SchedulerStage::Running(_)))
    {
        return false;
    }
    match &tick.outcome {
        Intro2Type13SchedulerProductionOutcome::Class5AimlessVisit { .. } => true,
        Intro2Type13SchedulerProductionOutcome::B6c0Visit {
            acquisition, aim, ..
        } => acquisition_completed(Some(acquisition)) && !matches!(aim, Some(Err(_))),
        Intro2Type13SchedulerProductionOutcome::C690Transition {
            acquisition, aim, ..
        } => acquisition_completed(acquisition.as_ref()) && !matches!(aim, Some(Err(_))),
        Intro2Type13SchedulerProductionOutcome::PursuingVisit {
            chase,
            acquisition,
            aim,
            ..
        } => {
            !chase.is_some_and(|chase| {
                matches!(
                    chase.result,
                    Intro2Type13ChaseVisitResult::CommonMoverBlocked(_)
                )
            }) && acquisition_completed(acquisition.as_ref())
                && !matches!(aim, Some(Err(_)))
        }
        Intro2Type13SchedulerProductionOutcome::PursuingChaseBlocked { .. }
        | Intro2Type13SchedulerProductionOutcome::PursuingC690Blocked { .. }
        | Intro2Type13SchedulerProductionOutcome::PrimaryBlocked { .. }
        | Intro2Type13SchedulerProductionOutcome::CallbackFailurePending { .. }
        | Intro2Type13SchedulerProductionOutcome::PostTaskBasisBlocked { .. }
        | Intro2Type13SchedulerProductionOutcome::C690Blocked { .. }
        | Intro2Type13SchedulerProductionOutcome::Dropped { .. } => false,
    }
}
