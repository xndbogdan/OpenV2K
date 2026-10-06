//! Live intrusive-list snapshot for class-7 `FUN_00422CD0`.
//!
//! The recovered selector already owns order, dying/zero-state rejection,
//! recent-relation suppression, and fail-closed unresolved fields. This
//! adapter binds that selector to [`EntityManager`]'s current live list and
//! applies the slot-1 `FUN_00402080` filter prefix. An authenticated class-7
//! variant-0 context then runs `FUN_0040C7D0` (store target at `+0x08`) and
//! `FUN_0040C6B0(variant + 1)` (publish style `0x004C7A98`). Variant-1
//! `FUN_0040ADE0` then optionally plays type `+0x9A` through `FUN_0044F480`
//! at entity `+0x96`, prepares slot-2 Aim-and-Fire, clears slot 1, and
//! prepares slot-0 Chase. A successful ADE0 handoff visits the newly
//! published Tertiary Aim in that same owner pass. Primary Chase is not
//! revisited that pass. Generic owners visit that newborn Aim without
//! inventing `FUN_00424650`. Type-13 Intro2 spawn 0 skips that recovered
//! visit so `intro2_type13_aim` can run method 10 once. A later Primary
//! visit advances Chase lifetime, validates the live target, and applies
//! `FUN_00423030`. It does not invoke `FUN_00401430`. A successful
//! selection without that class-7 context is still retail's zero-return
//! `BehaviorHandoffAbsent` path.

use crate::actor_task_dispatcher::{
    prepare_search_attack_aim_and_fire_runtime_task, ActorTaskRuntime,
};
use crate::actor_task_owner::{ActorTaskSlot, ActorTaskVisit};
use crate::aim_and_fire::{
    evaluate_aim_and_fire_callback, AimAndFireCallbackPrefix, AimAndFireCallbackResult,
    AimAndFireFrameError, AimAndFireFrameRequest, AimAndFireInvalidTargetReason,
    AimAndFireTaggedSingleton, AimAndFireTargetRuntimeState,
};
use crate::chase_target::{
    evaluate_chase_target_callback, ChaseTargetCallbackError, ChaseTargetCallbackPrefix,
    ChaseTargetCallbackRequest, ChaseTargetCallbackResult, ChaseTargetCommonMoverPath,
    ChaseTargetProximityControl, ChaseTargetTaggedSingleton, ChaseTargetTargetRuntimeState,
    ChaseTargetTaskState,
};
use crate::entity::{Entity, EntityManager};
use crate::entity_behavior::{
    audited_behavior_style, behavior_program, ActiveBehaviorStyle, BehaviorContextRuntime,
    BehaviorDescriptorIdentity,
};
use crate::entity_collision_state::CommonMoverComponentTopology;
use crate::entity_collision_state::CommonMoverGklPayloads;
use crate::entity_collision_state::EntityTypeRuntimeMetadata;
use crate::entity_collision_state::RetailRuntimeValue;
use crate::entity_collision_state::DYING_STATE_BIT;
use crate::search_attack::{
    search_attack_target_handoff, search_attack_variant_setup, SearchAttackEntityRef,
    SearchAttackSelectionError, SearchAttackTarget, SearchAttackTargetHandoff, SearchAttackVariant,
    SEARCH_ATTACK_BEHAVIOR_CLASS_ID,
};
use crate::search_attack_acquisition::{
    evaluate_target_acquisition_callback, TargetAcquisitionCallbackError,
    TargetAcquisitionCallbackPrefix, TargetAcquisitionCallbackResult,
    TargetAcquisitionTaggedSingleton, TargetAcquisitionZeroReason,
};
use crate::search_attack_owner::SearchAttackTaskSetupRequest;
use crate::world_fx::WorldFx;
use crate::wrapped_axis_range::WrappedAxisRange;
use v2k_formats::collision::{
    BehaviorChoice, CommonAxisDescriptor, ProjectileEmitterDescriptor, SubDSteeringDescriptor,
};

/// Snapshot every current live-list allocation in retail intrusive order.
///
/// `FUN_00422CD0` walks the global list. Attached cargo therefore remains
/// visible here; the selector, not the snapshot, rejects dying, zero-state,
/// related, or filter-mismatched entries. Unresolved consumed fields stay
/// unresolved so the selector can fail closed.
pub fn search_attack_live_list_snapshot(manager: &EntityManager) -> Vec<SearchAttackEntityRef<'_>> {
    manager
        .iter_all()
        .map(search_attack_live_entity_ref)
        .collect()
}

fn search_attack_live_entity_ref(entity: &Entity) -> SearchAttackEntityRef<'_> {
    SearchAttackEntityRef {
        id: entity.id,
        entity_type: entity.entity_type,
        position_raw: entity.position_raw(),
        capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        collision: &entity.collision,
    }
}

/// Why a live `FUN_00402080` visit cannot run the recovered selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchAttackLiveAcquisitionBlock {
    Selection(SearchAttackSelectionError),
    SuccessWithoutOutput,
    HandoffContextUnavailable,
    NotClass7Variant0,
    PursuingStyleUnavailable,
    Ade0TopologyUnresolved,
    Ade0SubARuntimeUnavailable,
    Ade0SubFUnsupported,
    Ade0AimPrepare,
    Ade0ChasePrepare,
    Ade0AimAudioUnresolved,
    Ade0PreludeAudioUnresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchAttackLiveAcquisitionOutcome {
    NotApplicable,
    Applied {
        entity_id: u32,
        prefix: TargetAcquisitionCallbackPrefix,
        result: TargetAcquisitionCallbackResult,
        same_pass_aim: Option<SearchAttackLiveAimOutcome>,
    },
    Blocked {
        entity_id: u32,
        reason: SearchAttackLiveAcquisitionBlock,
    },
}

/// Whether a selected candidate may legitimately lack the built-in class-7
/// C7D0/C6B0 handoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchAttackLiveHandoffRequirement {
    /// Generic acquisition owners may supply a callback outside this adapter.
    Optional,
    /// The authenticated owner is class-7 variant 0, so an unavailable or
    /// mismatched context must block instead of masquerading as no callback.
    RequiredClass7Variant0,
}

/// Whether recovered ADE0 should visit newborn slot-2 Aim here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchAttackLiveSamePassAim {
    /// Lifetime, invalid-target tag, optional `+0x9C` audio, known-null Sub-E
    /// zero, unresolved/present Sub-E fail-closed. Does not invent
    /// `FUN_00424650`.
    WithoutEmitter,
    /// Type-13 Intro2 spawn 0 owns `FUN_00424650` in `intro2_type13_aim`. Skip
    /// so this pass does not advance Aim twice.
    Skip,
}

const SEARCH_ATTACK_PURSUING_STYLE_INDEX: u32 = 1;
/// Generic scheduler mode for class-7 `FUN_00403490`. A nonzero mode would
/// suppress the recovered callback; this visit does not invent one.
const SEARCH_ATTACK_CHASE_SCHEDULER_MODE: u32 = 0;
/// Generic scheduler mode for class-7 `FUN_00402300`. A nonzero mode would
/// suppress the recovered callback; this visit does not invent one.
const SEARCH_ATTACK_AIM_SCHEDULER_MODE: u32 = 0;

/// Exact first-world Section-12 Sub-E for cumulative entity type 13.
///
/// Local `*X3XX` record 11 is global type 13 (`ptersect`, model 291). The
/// payload is presentation-tier invariant. Generic ADE0 Aim may observe it
/// without inventing `FUN_00424650`. Type-13 Intro2 spawn 0 skips that
/// recovered visit; `intro2_type13_aim` owns the method-10 emitter.
pub const TYPE13_SEARCH_ATTACK_PROJECTILE_DESCRIPTOR: ProjectileEmitterDescriptor =
    ProjectileEmitterDescriptor {
        projectile_method: 10,
        random_interval_us: 600_000,
        spread_raw: 128,
        aim_threshold_raw: 20_000,
        speed_override_raw: 0,
        target_axis_tolerance_raw: 0x0F00,
        sound_id: 75,
        raw_word_at_0x12: 26,
        alternate_emitter_raw: 0,
        stochastic_gate_mode: 0,
        auxiliary_command: 0,
        variable_bindings: [0; 4],
    };

/// Exact first-world Section-12 `+0xC8/+0xCC` pair for cumulative type 13.
///
/// `FUN_00423030` consumes the first signed dword as a strict wrapped cube.
/// ADE0 Chase uses this recovered actor-local pair and still does not invent
/// `FUN_00401430`.
pub const TYPE13_SEARCH_ATTACK_COMMON_AXIS: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 0x1900,
    raw_word_at_0x04: 0x0C05,
};

/// Exact first-world Section-12 Sub-D for cumulative entity type 13.
///
/// Divisor 64 and probes `0x200`/`0x100` match the shared layout. Type 13
/// authors `couple_yaw_into_roll = 1` and `classifier_flags = 0`; the latter
/// bypasses every `FUN_0041FCB0` call after the allocation's per-frame stagger
/// maintenance. The detached Type-13 binding applies this Sub-D prefix and
/// captured B6C0 now continues through the exact normal Sub-G transaction for
/// its supplied pre-call basis. Class-38 drain and the post-task DCA0/E870
/// basis refresh are live. The bounded Intro2 world owner now supplies shared
/// timing, activation, environment and master motion. Presentation acceptance
/// and unsupported relation/dying histories remain separate boundaries.
pub const TYPE13_SEARCH_ATTACK_SUB_D: SubDSteeringDescriptor = SubDSteeringDescriptor {
    steering_divisor_raw: 64,
    couple_yaw_into_roll_raw: 1,
    enable_pitch_steering_raw: 0,
    forward_probe_raw: 0x0200,
    lateral_probe_raw: 0x0100,
    classifier_flags: 0,
    reserved_at_0x0b: 0,
};

/// Exact first-world Section-12 Sub-G/K/L bytes for cumulative type 13.
///
/// Retail Sub-G word `+0x0E` is 69; the demo build authors 71 there. K/L are
/// build-invariant. ADE0 Chase binds these payloads and still does not invent
/// `FUN_00401430`.
pub const TYPE13_SEARCH_ATTACK_SUB_G: [u8; 104] = [
    0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 200, 0, 69, 0, 50, 0, 0, 0, 130, 0, 0, 0, 44, 1, 0, 0, 0,
    64, 0, 0, 220, 5, 0, 0, 1, 2, 3, 4, 5, 8, 9, 0, 0, 0, 0, 32, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 240, 0, 17, 230, 0, 0, 0, 1,
    0, 0, 0, 0, 0, 0, 16, 210, 0, 0, 0, 1, 0, 0, 0,
];
/// Entity variable-bank selectors resolved into Sub-G's seven pointer slots.
pub const TYPE13_SEARCH_ATTACK_SUB_G_ANIMATION_BINDINGS: [usize; 7] = [
    TYPE13_SEARCH_ATTACK_SUB_G[0x24] as usize,
    TYPE13_SEARCH_ATTACK_SUB_G[0x25] as usize,
    TYPE13_SEARCH_ATTACK_SUB_G[0x26] as usize,
    TYPE13_SEARCH_ATTACK_SUB_G[0x27] as usize,
    TYPE13_SEARCH_ATTACK_SUB_G[0x28] as usize,
    TYPE13_SEARCH_ATTACK_SUB_G[0x29] as usize,
    TYPE13_SEARCH_ATTACK_SUB_G[0x2a] as usize,
];
/// Descriptor dword `+0x00` consumed by `FUN_0041B980(..., 0)`.
pub const TYPE13_SEARCH_ATTACK_SUB_G_SOURCE_RAW_AT_0X00: u32 = u32::from_le_bytes([
    TYPE13_SEARCH_ATTACK_SUB_G[0],
    TYPE13_SEARCH_ATTACK_SUB_G[1],
    TYPE13_SEARCH_ATTACK_SUB_G[2],
    TYPE13_SEARCH_ATTACK_SUB_G[3],
]);
/// Signed descriptor word `+0x0C` consumed by `FUN_0041B8C0` / `FUN_0041B940`.
pub const TYPE13_SEARCH_ATTACK_SUB_G_RANDOMIZED_TARGET_BASE_RAW_AT_0X0C: i16 =
    i16::from_le_bytes([
        TYPE13_SEARCH_ATTACK_SUB_G[12],
        TYPE13_SEARCH_ATTACK_SUB_G[13],
    ]);
/// Five bytes copied by `FUN_0041B8C0` from descriptor `+0x30` at stride 12
/// into runtime `+0x28..+0x2C`.
pub const TYPE13_SEARCH_ATTACK_SUB_G_1B8C0_TABLE_BYTES: [u8; 5] = [
    TYPE13_SEARCH_ATTACK_SUB_G[0x30],
    TYPE13_SEARCH_ATTACK_SUB_G[0x3C],
    TYPE13_SEARCH_ATTACK_SUB_G[0x48],
    TYPE13_SEARCH_ATTACK_SUB_G[0x54],
    TYPE13_SEARCH_ATTACK_SUB_G[0x60],
];
pub const TYPE13_SEARCH_ATTACK_SUB_K: [u8; 2] = [11, 10];
pub const TYPE13_SEARCH_ATTACK_SUB_L: [u8; 6] = [6, 7, 64, 31, 64, 31];
pub const TYPE13_SEARCH_ATTACK_GKL: CommonMoverGklPayloads = CommonMoverGklPayloads {
    sub_g: Some(TYPE13_SEARCH_ATTACK_SUB_G),
    sub_k: Some(TYPE13_SEARCH_ATTACK_SUB_K),
    sub_l: Some(TYPE13_SEARCH_ATTACK_SUB_L),
};
/// Exact first-world Section-12 optional-component presence for type 13.
///
/// D/E/G/K/L only. Shared `FUN_00406070` therefore cannot use the bounded
/// Sub-A-only or Sub-H-then-Sub-A profiles. Type-13 B6C0 attaches the Sub-G
/// branch (`FUN_0041B970/1B940/1B980/00424380`) without inventing Sub-A.
pub const TYPE13_SEARCH_ATTACK_TOPOLOGY: CommonMoverComponentTopology =
    CommonMoverComponentTopology {
        sub_a: false,
        sub_b: false,
        sub_c: false,
        sub_d: true,
        sub_e: true,
        sub_f: false,
        sub_g: true,
        sub_h: false,
        sub_i: false,
        sub_j: false,
        sub_k: true,
        sub_l: true,
        sub_m: false,
        sub_n: false,
        sub_o: false,
    };

/// Exact first-world Section-12 `+0x118` list for cumulative type 13.
///
/// Always x1 class 5 (`Move About Aimlessly`) then Always x3 class 7
/// (`Search And Attack Target`). Class 7's birth owner is variant-0 B6C0;
/// ADE0 is the post-acquisition variant. The detached type-13 planner
/// consumes one `FUN_00425680` word against this list and does not invent
/// `FUN_00401430`.
pub const TYPE13_SEARCH_ATTACK_BEHAVIOR_CHOICES: [BehaviorChoice; 2] = [
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 1,
        behavior_class_id: 5,
    },
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 3,
        behavior_class_id: 7,
    },
];
/// Section-12 `+0x11C` for cumulative type 13.
pub const TYPE13_SEARCH_ATTACK_BEHAVIOR_RULE_REF: u32 = 3;
/// Section-12 `+0x124` for cumulative type 13. Class 1 is retained as the
/// authored dword; its named program is not invented here.
pub const TYPE13_SEARCH_ATTACK_ALTERNATE_BEHAVIOR_CLASS: u32 = 1;

/// Apply recovered `FUN_0040C7D0` then `FUN_0040C6B0(variant + 1)`.
///
/// This writes context `+0x08` and publishes class-7 style index 1. It does
/// not run variant-1 `FUN_0040ADE0` or the shared mover.
pub fn apply_search_attack_c7d0_c6b0_without_mover(
    entity: &mut Entity,
    target: SearchAttackTarget,
) -> Result<SearchAttackTargetHandoff, SearchAttackLiveAcquisitionBlock> {
    let RetailRuntimeValue::Known(Some(current)) = entity.current_behavior_context else {
        return Err(SearchAttackLiveAcquisitionBlock::HandoffContextUnavailable);
    };
    let program = behavior_program(u32::from(SEARCH_ATTACK_BEHAVIOR_CLASS_ID))
        .ok_or(SearchAttackLiveAcquisitionBlock::HandoffContextUnavailable)?;
    let ActiveBehaviorStyle::Audited(initial_style) = current.active_style() else {
        return Err(SearchAttackLiveAcquisitionBlock::HandoffContextUnavailable);
    };
    if current.descriptor() != BehaviorDescriptorIdentity::Named(program)
        || current.style_table_index_raw_at_0x10() != 0
        || initial_style.class_id != SEARCH_ATTACK_BEHAVIOR_CLASS_ID
        || initial_style.variant != 0
    {
        return Err(SearchAttackLiveAcquisitionBlock::NotClass7Variant0);
    }
    let pursuing_style = *audited_behavior_style(
        u32::from(SEARCH_ATTACK_BEHAVIOR_CLASS_ID),
        SEARCH_ATTACK_PURSUING_STYLE_INDEX as u8,
    )
    .ok_or(SearchAttackLiveAcquisitionBlock::PursuingStyleUnavailable)?;
    debug_assert_eq!(pursuing_style.frame_address, 0x004C_7A98);
    let handoff = search_attack_target_handoff(target);
    debug_assert_eq!(handoff.next_variant, SearchAttackVariant::Pursuing);
    let with_target = BehaviorContextRuntime::named_audited(
        program,
        0,
        current.choice_list_source(),
        RetailRuntimeValue::Known(Some(handoff.target_id)),
        current.auxiliary_word_at_0x0c(),
        initial_style,
    )
    .ok_or(SearchAttackLiveAcquisitionBlock::HandoffContextUnavailable)?;
    let pursuing = BehaviorContextRuntime::named_audited(
        program,
        SEARCH_ATTACK_PURSUING_STYLE_INDEX,
        with_target.choice_list_source(),
        with_target.target_handle_at_0x08(),
        with_target.auxiliary_word_at_0x0c(),
        pursuing_style,
    )
    .ok_or(SearchAttackLiveAcquisitionBlock::PursuingStyleUnavailable)?;
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(pursuing));
    // C6B0 -> EA10 publishes the style masks before ADE0. Class7 variant1
    // has +34=0 and +38=0x80: reversed D440 sets entity bit0x8000. This
    // remains committed even when the subsequent initializer cannot finish.
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x8000, 0x8000);
    Ok(handoff)
}

/// Apply recovered `FUN_0040ADE0` after C6B0 published variant 1.
///
/// A Known nonzero type `+0x9A` plays `FUN_0044F480` at entity `+0x96`
/// before slot-2. Known zero skips. Unresolved prelude fails closed.
/// Slot-2 Aim-and-Fire is prepared next. Failure leaves slot 1. Success
/// clears slot 1, then slot-0 Chase is prepared. Chase failure leaves slot 2.
/// `FUN_00401430` is not called.
pub fn apply_search_attack_ade0_without_mover(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    target_id: u32,
    world_fx: &mut WorldFx,
) -> Result<(), SearchAttackLiveAcquisitionBlock> {
    let topology = match metadata.common_mover_topology {
        RetailRuntimeValue::Known(topology) => topology,
        RetailRuntimeValue::Unresolved => {
            return Err(SearchAttackLiveAcquisitionBlock::Ade0TopologyUnresolved);
        }
    };
    if topology.sub_f {
        return Err(SearchAttackLiveAcquisitionBlock::Ade0SubFUnsupported);
    }
    if topology.sub_a
        && !matches!(
            entity.sub_a_propulsion_runtime,
            RetailRuntimeValue::Known(Some(_))
        )
    {
        return Err(SearchAttackLiveAcquisitionBlock::Ade0SubARuntimeUnavailable);
    }
    let aim_sound_id_raw = match metadata.search_attack_aim_sound_id {
        RetailRuntimeValue::Known(sound) => u32::from(sound.unwrap_or(0)),
        RetailRuntimeValue::Unresolved => {
            return Err(SearchAttackLiveAcquisitionBlock::Ade0AimAudioUnresolved);
        }
    };
    let aim_sound_period_raw = match metadata.search_attack_aim_sound_period_raw {
        RetailRuntimeValue::Known(period) => period,
        RetailRuntimeValue::Unresolved => {
            return Err(SearchAttackLiveAcquisitionBlock::Ade0AimAudioUnresolved);
        }
    };
    let prelude_sound_id = match metadata.search_attack_optional_prelude_sound_id {
        RetailRuntimeValue::Known(sound) => sound.filter(|&id| id != 0),
        RetailRuntimeValue::Unresolved => {
            return Err(SearchAttackLiveAcquisitionBlock::Ade0PreludeAudioUnresolved);
        }
    };

    let owner_position_raw = entity.position_raw();
    if let Some(sound_id) = prelude_sound_id {
        world_fx.queue_fixed_positional_sound_raw(sound_id, owner_position_raw);
    }
    let request = SearchAttackTaskSetupRequest::Pursuing { target_id };
    let setup = search_attack_variant_setup(SearchAttackVariant::Pursuing);
    let aim = prepare_search_attack_aim_and_fire_runtime_task(
        crate::search_attack_owner::SearchAttackTaskPreparation {
            request,
            phase_index: 0,
            task: setup.ordered_phases[0].install,
        },
        entity.id,
        owner_position_raw,
        aim_sound_id_raw,
        aim_sound_period_raw,
        metadata,
    )
    .map_err(|_| SearchAttackLiveAcquisitionBlock::Ade0AimPrepare)?;
    let aim_task = aim.apply_suffix(|_, suffix| {
        debug_assert!(!suffix.enables_sub_f());
    });
    entity
        .actor_tasks
        .replace_prepared(ActorTaskSlot::Tertiary, aim_task);
    entity.actor_tasks.clear_slot(ActorTaskSlot::Secondary);

    let chase = match ChaseTargetTaskState::prepare_after_allocation(
        entity.id,
        owner_position_raw,
        target_id,
        metadata,
    ) {
        Ok(chase) => chase,
        Err(_) => return Err(SearchAttackLiveAcquisitionBlock::Ade0ChasePrepare),
    };
    let Entity {
        actor_tasks,
        sub_a_propulsion_runtime,
        ..
    } = entity;
    let chase_task = chase
        .map_task(ActorTaskRuntime::ChaseTarget)
        .apply_suffix(|_, suffix| {
            debug_assert!(!suffix.enables_sub_f());
            if let Some(speed) = suffix.sub_a_target_speed_raw() {
                if let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime {
                    sub_a.apply_shared_initializer_target_speed_write(speed);
                }
            }
        });
    actor_tasks.replace_prepared(ActorTaskSlot::Primary, chase_task);
    Ok(())
}

/// Why a later ADE0 Chase visit cannot run recovered `FUN_00403490`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchAttackLiveChaseBlock {
    CommonAxisUnresolved,
    TargetStateUnresolved,
    SubDUnresolved,
    GklUnresolved,
    VisitUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchAttackLiveChaseResult {
    Tagged(ChaseTargetTaggedSingleton),
    Type13MoverUninvented { path: ChaseTargetCommonMoverPath },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchAttackLiveChaseOutcome {
    NotApplicable,
    Applied {
        entity_id: u32,
        prefix: ChaseTargetCallbackPrefix,
        result: SearchAttackLiveChaseResult,
        bound_sub_d: Option<SubDSteeringDescriptor>,
        bound_gkl: Option<CommonMoverGklPayloads>,
    },
    Blocked {
        entity_id: u32,
        reason: SearchAttackLiveChaseBlock,
    },
}

/// Visit recovered ADE0 slot-0 Chase without inventing `FUN_00401430`.
///
/// Lifetime advances first. A live target then takes `FUN_00423030` against
/// the actor-local common-axis limit. Authored Sub-D is bound from the type
/// record when present. Both range branches still require the shared mover,
/// so this visit fail-closes there instead of inventing a type-13 return.
/// Invalid-target tags are reported without an owner transition.
pub fn apply_search_attack_chase_visit_without_mover(
    manager: &mut EntityManager,
    owner_id: u32,
    elapsed_micros: u32,
) -> SearchAttackLiveChaseOutcome {
    let (entity_axis, owner_position_raw, target_id, entity_type) = {
        let Some(entity) = manager.iter_all().find(|entity| entity.id == owner_id) else {
            return SearchAttackLiveChaseOutcome::NotApplicable;
        };
        if !class7_variant1_published(entity) {
            return SearchAttackLiveChaseOutcome::NotApplicable;
        }
        let Some(ActorTaskRuntime::ChaseTarget(state)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            return SearchAttackLiveChaseOutcome::NotApplicable;
        };
        (
            entity.actor_common_axis_descriptor,
            entity.position_raw(),
            state.target_id(),
            entity.entity_type,
        )
    };
    let axis = match entity_axis {
        RetailRuntimeValue::Known(axis) => axis,
        RetailRuntimeValue::Unresolved => {
            match manager
                .type_runtime_metadata(entity_type)
                .and_then(|metadata| metadata.initializer.as_ref())
            {
                Some(initializer) => initializer.common_axis_descriptor,
                None => {
                    return SearchAttackLiveChaseOutcome::Blocked {
                        entity_id: owner_id,
                        reason: SearchAttackLiveChaseBlock::CommonAxisUnresolved,
                    };
                }
            }
        }
    };
    let metadata = manager.type_runtime_metadata(entity_type);
    let bound_sub_d = match search_attack_chase_sub_d(metadata) {
        Ok(sub_d) => sub_d,
        Err(reason) => {
            return SearchAttackLiveChaseOutcome::Blocked {
                entity_id: owner_id,
                reason,
            };
        }
    };
    let bound_gkl = match search_attack_chase_gkl(metadata) {
        Ok(gkl) => gkl,
        Err(reason) => {
            return SearchAttackLiveChaseOutcome::Blocked {
                entity_id: owner_id,
                reason,
            };
        }
    };
    let target_state = match search_attack_chase_target_runtime_state(manager, target_id) {
        Ok(state) => state,
        Err(()) => {
            return SearchAttackLiveChaseOutcome::Blocked {
                entity_id: owner_id,
                reason: SearchAttackLiveChaseBlock::TargetStateUnresolved,
            };
        }
    };
    let Some(entity) = manager.search_attack_acquisition_entity_mut(owner_id) else {
        return SearchAttackLiveChaseOutcome::NotApplicable;
    };
    let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) else {
        return SearchAttackLiveChaseOutcome::NotApplicable;
    };
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    let Some(prefix) = entity.actor_tasks.begin_exact_visit_with(visit, |runtime| {
        let ActorTaskRuntime::ChaseTarget(state) = runtime else {
            unreachable!("authenticated Primary is ChaseTarget");
        };
        state.before_callback(elapsed_micros)
    }) else {
        return SearchAttackLiveChaseOutcome::Blocked {
            entity_id: owner_id,
            reason: SearchAttackLiveChaseBlock::VisitUnavailable,
        };
    };
    let Some(ActorTaskRuntime::ChaseTarget(state)) =
        entity.actor_tasks.state_in_slot(ActorTaskSlot::Primary)
    else {
        return SearchAttackLiveChaseOutcome::NotApplicable;
    };
    let mut stage = state.stage_callback();
    let mut movement = ();
    let mut controller = ();
    let evaluated = evaluate_chase_target_callback(
        &mut stage,
        ChaseTargetCallbackRequest {
            visit,
            entity_id: owner_id,
            owner_position_raw,
            route_range: WrappedAxisRange::from_raw(axis.strict_axis_limit_raw),
            proximity_control: ChaseTargetProximityControl::Disabled,
            movement_state: &mut movement,
            controller_context: &mut controller,
            elapsed_micros,
            scheduler_mode: SEARCH_ATTACK_CHASE_SCHEDULER_MODE,
        },
        |_| Ok::<_, SearchAttackLiveChaseBlock>(target_state),
        |_| Err(SearchAttackLiveChaseBlock::VisitUnavailable),
        |_, _| {
            unreachable!("class-7 Chase does not invent post-mover positions without FUN_00401430")
        },
    );
    if entity
        .actor_tasks
        .wrapper_flags(visit.task_id)
        .is_some_and(|flags| flags.in_callback)
    {
        let _ = entity.actor_tasks.finish_exact_visit(visit);
    }
    let result = match evaluated {
        Ok(ChaseTargetCallbackResult::Tagged(singleton)) => {
            SearchAttackLiveChaseResult::Tagged(singleton)
        }
        Ok(ChaseTargetCallbackResult::Continue { .. }) => {
            unreachable!("class-7 Chase cannot Continue without FUN_00401430")
        }
        Err(ChaseTargetCallbackError::CommonMover { path, .. }) => {
            SearchAttackLiveChaseResult::Type13MoverUninvented { path }
        }
        Err(_) => {
            return SearchAttackLiveChaseOutcome::Blocked {
                entity_id: owner_id,
                reason: SearchAttackLiveChaseBlock::VisitUnavailable,
            };
        }
    };
    SearchAttackLiveChaseOutcome::Applied {
        entity_id: owner_id,
        prefix,
        result,
        bound_sub_d,
        bound_gkl,
    }
}

fn search_attack_chase_sub_d(
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<Option<SubDSteeringDescriptor>, SearchAttackLiveChaseBlock> {
    let Some(metadata) = metadata else {
        return Ok(None);
    };
    match metadata.sub_d_steering_descriptor {
        RetailRuntimeValue::Known(sub_d) => Ok(sub_d),
        RetailRuntimeValue::Unresolved => match metadata.common_mover_topology {
            RetailRuntimeValue::Known(topology) if topology.sub_d => {
                Err(SearchAttackLiveChaseBlock::SubDUnresolved)
            }
            _ => Ok(None),
        },
    }
}

fn search_attack_chase_gkl(
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<Option<CommonMoverGklPayloads>, SearchAttackLiveChaseBlock> {
    let Some(metadata) = metadata else {
        return Ok(None);
    };
    match metadata.common_mover_gkl_payloads {
        RetailRuntimeValue::Known(payloads) => Ok(Some(payloads).filter(|payloads| {
            payloads.sub_g.is_some() || payloads.sub_k.is_some() || payloads.sub_l.is_some()
        })),
        RetailRuntimeValue::Unresolved => match metadata.common_mover_topology {
            RetailRuntimeValue::Known(topology)
                if topology.sub_g || topology.sub_k || topology.sub_l =>
            {
                Err(SearchAttackLiveChaseBlock::GklUnresolved)
            }
            _ => Ok(None),
        },
    }
}

fn class7_variant1_published(entity: &Entity) -> bool {
    let RetailRuntimeValue::Known(Some(current)) = entity.current_behavior_context else {
        return false;
    };
    let Some(program) = behavior_program(u32::from(SEARCH_ATTACK_BEHAVIOR_CLASS_ID)) else {
        return false;
    };
    let ActiveBehaviorStyle::Audited(style) = current.active_style() else {
        return false;
    };
    current.descriptor() == BehaviorDescriptorIdentity::Named(program)
        && current.style_table_index_raw_at_0x10() == 1
        && style.class_id == SEARCH_ATTACK_BEHAVIOR_CLASS_ID
        && style.variant == 1
        && style.frame_address == 0x004C_7A98
}

fn search_attack_chase_target_runtime_state(
    manager: &EntityManager,
    target_id: u32,
) -> Result<ChaseTargetTargetRuntimeState, ()> {
    let Some(target) = manager.iter_all().find(|entity| entity.id == target_id) else {
        return Ok(ChaseTargetTargetRuntimeState::Missing);
    };
    if !target.active {
        return Ok(ChaseTargetTargetRuntimeState::Inactive);
    }
    let state = target.collision.state_flags_at_0x08;
    match state.masked(DYING_STATE_BIT) {
        RetailRuntimeValue::Unresolved => return Err(()),
        RetailRuntimeValue::Known(bits) if bits != 0 => {
            return Ok(ChaseTargetTargetRuntimeState::Dying);
        }
        RetailRuntimeValue::Known(_) => {}
    }
    if state.known_mask() != u32::MAX {
        return Err(());
    }
    if state.known_value_bits() == 0 {
        return Ok(ChaseTargetTargetRuntimeState::Inactive);
    }
    Ok(ChaseTargetTargetRuntimeState::Live {
        position_raw: target.position_raw(),
    })
}

/// Why a later ADE0 Aim visit cannot finish recovered `FUN_00402300`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchAttackLiveAimBlock {
    TargetStateUnresolved,
    VisitUnavailable,
    SubEUnresolved,
    GenericEmitterUninvented,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchAttackLiveAimResult {
    Zero,
    TaggedInvalidTarget {
        singleton: AimAndFireTaggedSingleton,
        reason: AimAndFireInvalidTargetReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchAttackLiveAimOutcome {
    NotApplicable,
    Applied {
        entity_id: u32,
        prefix: AimAndFireCallbackPrefix,
        result: SearchAttackLiveAimResult,
    },
    Blocked {
        entity_id: u32,
        reason: SearchAttackLiveAimBlock,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchAttackAimSubE {
    Unresolved,
    Absent,
    Present,
}

/// Visit recovered ADE0 slot-2 Aim without inventing `FUN_00424650`.
///
/// Lifetime advances first. An invalid target tags `0x9C00` without an owner
/// transition. A valid target may play the recovered optional `+0x9C` cue.
/// Known-null Sub-E returns zero. Unresolved or present Sub-E fail-closes
/// instead of inventing the generic emitter.
pub fn apply_search_attack_aim_visit_without_emitter(
    manager: &mut EntityManager,
    owner_id: u32,
    elapsed_micros: u32,
    world_fx: &mut WorldFx,
) -> SearchAttackLiveAimOutcome {
    let (owner_position_raw, target_id, entity_type) = {
        let Some(entity) = manager.iter_all().find(|entity| entity.id == owner_id) else {
            return SearchAttackLiveAimOutcome::NotApplicable;
        };
        if !class7_variant1_published(entity) {
            return SearchAttackLiveAimOutcome::NotApplicable;
        }
        let Some(ActorTaskRuntime::AimAndFire(state)) =
            entity.actor_task_state(ActorTaskSlot::Tertiary)
        else {
            return SearchAttackLiveAimOutcome::NotApplicable;
        };
        (
            entity.position_raw(),
            state.private_state().target_entity_id(),
            entity.entity_type,
        )
    };
    let sub_e = match manager
        .type_runtime_metadata(entity_type)
        .map(|metadata| metadata.projectile_emitter_descriptor)
    {
        Some(RetailRuntimeValue::Known(None)) => SearchAttackAimSubE::Absent,
        Some(RetailRuntimeValue::Known(Some(_))) => SearchAttackAimSubE::Present,
        Some(RetailRuntimeValue::Unresolved) | None => SearchAttackAimSubE::Unresolved,
    };
    let target_state = match search_attack_aim_target_runtime_state(manager, target_id) {
        Ok(state) => state,
        Err(()) => {
            return SearchAttackLiveAimOutcome::Blocked {
                entity_id: owner_id,
                reason: SearchAttackLiveAimBlock::TargetStateUnresolved,
            };
        }
    };
    let Some(entity) = manager.search_attack_acquisition_entity_mut(owner_id) else {
        return SearchAttackLiveAimOutcome::NotApplicable;
    };
    let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary) else {
        return SearchAttackLiveAimOutcome::NotApplicable;
    };
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Tertiary,
        task_id,
    };
    let Some((prefix, private_state)) =
        entity.actor_tasks.begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::AimAndFire(state) = runtime else {
                unreachable!("authenticated Tertiary is AimAndFire");
            };
            let private_state = state.private_state();
            (state.before_callback(elapsed_micros), private_state)
        })
    else {
        return SearchAttackLiveAimOutcome::Blocked {
            entity_id: owner_id,
            reason: SearchAttackLiveAimBlock::VisitUnavailable,
        };
    };
    let fx = std::cell::RefCell::new(world_fx);
    let evaluated = evaluate_aim_and_fire_callback(
        private_state,
        AimAndFireFrameRequest {
            owner_entity_id: owner_id,
            elapsed_micros,
            scheduler_mode: SEARCH_ATTACK_AIM_SCHEDULER_MODE,
            target_state,
            owner_sound_position_raw: Some(owner_position_raw),
            sub_e_descriptor: match sub_e {
                SearchAttackAimSubE::Present => Some(()),
                SearchAttackAimSubE::Absent | SearchAttackAimSubE::Unresolved => None,
            },
            emitter_runtime: (),
        },
        || u32::from(fx.borrow_mut().next_shared_retail_random_u16()),
        |effect| {
            let Some(position_raw) = effect.owner_position_raw else {
                return;
            };
            let Ok(sound_id) = u16::try_from(effect.sound_id.get()) else {
                return;
            };
            fx.borrow_mut()
                .queue_fixed_positional_sound_raw(sound_id, position_raw);
        },
        |_| Err(SearchAttackLiveAimBlock::GenericEmitterUninvented),
    );
    if entity
        .actor_tasks
        .wrapper_flags(visit.task_id)
        .is_some_and(|flags| flags.in_callback)
    {
        let _ = entity.actor_tasks.finish_exact_visit(visit);
    }
    let result = match evaluated {
        Ok(AimAndFireCallbackResult::TaggedInvalidTarget { singleton, reason }) => {
            SearchAttackLiveAimResult::TaggedInvalidTarget { singleton, reason }
        }
        Ok(AimAndFireCallbackResult::Zero) => match sub_e {
            SearchAttackAimSubE::Absent => SearchAttackLiveAimResult::Zero,
            SearchAttackAimSubE::Unresolved => {
                return SearchAttackLiveAimOutcome::Blocked {
                    entity_id: owner_id,
                    reason: SearchAttackLiveAimBlock::SubEUnresolved,
                };
            }
            SearchAttackAimSubE::Present => {
                return SearchAttackLiveAimOutcome::Blocked {
                    entity_id: owner_id,
                    reason: SearchAttackLiveAimBlock::GenericEmitterUninvented,
                };
            }
        },
        Ok(AimAndFireCallbackResult::ReturnGenericEmitterResult(_)) => {
            unreachable!("class-7 Aim cannot return FUN_00424650 without a live emitter")
        }
        Err(AimAndFireFrameError::GenericEmitter(
            SearchAttackLiveAimBlock::GenericEmitterUninvented,
        )) => {
            return SearchAttackLiveAimOutcome::Blocked {
                entity_id: owner_id,
                reason: SearchAttackLiveAimBlock::GenericEmitterUninvented,
            };
        }
        Err(_) => {
            return SearchAttackLiveAimOutcome::Blocked {
                entity_id: owner_id,
                reason: SearchAttackLiveAimBlock::VisitUnavailable,
            };
        }
    };
    SearchAttackLiveAimOutcome::Applied {
        entity_id: owner_id,
        prefix,
        result,
    }
}

fn search_attack_aim_target_runtime_state(
    manager: &EntityManager,
    target_id: u32,
) -> Result<AimAndFireTargetRuntimeState, ()> {
    let Some(target) = manager.iter_all().find(|entity| entity.id == target_id) else {
        return Ok(AimAndFireTargetRuntimeState::Missing);
    };
    let state = target.collision.state_flags_at_0x08;
    if state.known_mask() != u32::MAX {
        return Err(());
    }
    Ok(AimAndFireTargetRuntimeState::Present {
        state_flags: state.known_value_bits(),
    })
}

/// Apply slot-1 acquisition against the current live list.
///
/// A class-7 variant-0 owner takes recovered C7D0/C6B0 and ADE0. Other
/// owners keep the no-callback `BehaviorHandoffAbsent` return.
/// `FUN_00401430` is not invented here.
pub fn apply_search_attack_acquisition_live_without_handoff(
    manager: &mut EntityManager,
    owner_id: u32,
    elapsed_micros: u32,
    world_fx: &mut WorldFx,
) -> SearchAttackLiveAcquisitionOutcome {
    apply_search_attack_acquisition_live(
        manager,
        owner_id,
        elapsed_micros,
        world_fx,
        SearchAttackLiveHandoffRequirement::Optional,
        SearchAttackLiveSamePassAim::WithoutEmitter,
    )
}

pub(crate) fn apply_search_attack_acquisition_live(
    manager: &mut EntityManager,
    owner_id: u32,
    elapsed_micros: u32,
    world_fx: &mut WorldFx,
    handoff_requirement: SearchAttackLiveHandoffRequirement,
    same_pass_aim: SearchAttackLiveSamePassAim,
) -> SearchAttackLiveAcquisitionOutcome {
    let prefix = match commit_search_attack_acquisition_prefix(manager, owner_id) {
        PrefixCommit::MissingOwner => return SearchAttackLiveAcquisitionOutcome::NotApplicable,
        PrefixCommit::NotAcquisition => return SearchAttackLiveAcquisitionOutcome::NotApplicable,
        PrefixCommit::Applied(prefix) => prefix,
    };
    let snapshot = search_attack_live_list_snapshot(manager);
    let Some(owner) = snapshot
        .iter()
        .copied()
        .find(|entity| entity.id == owner_id)
    else {
        return SearchAttackLiveAcquisitionOutcome::NotApplicable;
    };
    let selected = match evaluate_target_acquisition_callback(
        prefix,
        owner,
        &snapshot,
        None::<fn(crate::search_attack::SearchAttackTargetHandoff) -> u32>,
    ) {
        Ok(result) => result,
        Err(TargetAcquisitionCallbackError::Selection(error)) => {
            return SearchAttackLiveAcquisitionOutcome::Blocked {
                entity_id: owner_id,
                reason: SearchAttackLiveAcquisitionBlock::Selection(error),
            };
        }
        Err(TargetAcquisitionCallbackError::SuccessWithoutOutput) => {
            return SearchAttackLiveAcquisitionOutcome::Blocked {
                entity_id: owner_id,
                reason: SearchAttackLiveAcquisitionBlock::SuccessWithoutOutput,
            };
        }
    };
    let result = match selected {
        TargetAcquisitionCallbackResult::Zero(
            TargetAcquisitionZeroReason::BehaviorHandoffAbsent { target },
        ) => match apply_live_class7_c7d0_c6b0(
            manager,
            owner_id,
            target,
            elapsed_micros,
            world_fx,
            handoff_requirement,
            same_pass_aim,
        ) {
            LiveHandoff::Absent => selected,
            LiveHandoff::Applied {
                handoff,
                same_pass_aim: visited_aim,
            } => {
                debug_assert_eq!(handoff.target_id, target.id);
                return SearchAttackLiveAcquisitionOutcome::Applied {
                    entity_id: owner_id,
                    prefix,
                    result: TargetAcquisitionCallbackResult::TaggedTargetAccepted {
                        target,
                        singleton: TargetAcquisitionTaggedSingleton::TargetAccepted,
                    },
                    same_pass_aim: visited_aim,
                };
            }
            LiveHandoff::Blocked(reason) => {
                return SearchAttackLiveAcquisitionOutcome::Blocked {
                    entity_id: owner_id,
                    reason,
                };
            }
        },
        other => other,
    };
    SearchAttackLiveAcquisitionOutcome::Applied {
        entity_id: owner_id,
        prefix,
        result,
        same_pass_aim: None,
    }
}

enum LiveHandoff {
    Absent,
    Applied {
        handoff: SearchAttackTargetHandoff,
        same_pass_aim: Option<SearchAttackLiveAimOutcome>,
    },
    Blocked(SearchAttackLiveAcquisitionBlock),
}

fn apply_live_class7_c7d0_c6b0(
    manager: &mut EntityManager,
    owner_id: u32,
    target: SearchAttackTarget,
    elapsed_micros: u32,
    world_fx: &mut WorldFx,
    handoff_requirement: SearchAttackLiveHandoffRequirement,
    same_pass_aim: SearchAttackLiveSamePassAim,
) -> LiveHandoff {
    let Some(entity) = manager.search_attack_acquisition_entity_mut(owner_id) else {
        return LiveHandoff::Absent;
    };
    let entity_type = entity.entity_type;
    let handoff = match apply_search_attack_c7d0_c6b0_without_mover(entity, target) {
        Ok(handoff) => handoff,
        Err(
            reason @ (SearchAttackLiveAcquisitionBlock::NotClass7Variant0
            | SearchAttackLiveAcquisitionBlock::HandoffContextUnavailable),
        ) => match handoff_requirement {
            SearchAttackLiveHandoffRequirement::Optional => return LiveHandoff::Absent,
            SearchAttackLiveHandoffRequirement::RequiredClass7Variant0 => {
                return LiveHandoff::Blocked(reason)
            }
        },
        Err(reason) => return LiveHandoff::Blocked(reason),
    };
    let metadata = manager.type_runtime_metadata(entity_type).cloned();
    let mut visited_aim = None;
    if let Some(metadata) = metadata {
        let ade0_ok = manager
            .search_attack_acquisition_entity_mut(owner_id)
            .is_some_and(|entity| {
                apply_search_attack_ade0_without_mover(
                    entity,
                    &metadata,
                    handoff.target_id,
                    world_fx,
                )
                .is_ok()
            });
        if ade0_ok {
            visited_aim = match same_pass_aim {
                SearchAttackLiveSamePassAim::WithoutEmitter => {
                    Some(apply_search_attack_aim_visit_without_emitter(
                        manager,
                        owner_id,
                        elapsed_micros,
                        world_fx,
                    ))
                }
                SearchAttackLiveSamePassAim::Skip => None,
            };
        }
    }
    LiveHandoff::Applied {
        handoff,
        same_pass_aim: visited_aim,
    }
}

enum PrefixCommit {
    MissingOwner,
    NotAcquisition,
    Applied(TargetAcquisitionCallbackPrefix),
}

fn commit_search_attack_acquisition_prefix(
    manager: &mut EntityManager,
    owner_id: u32,
) -> PrefixCommit {
    let Some(entity) = manager.search_attack_acquisition_entity_mut(owner_id) else {
        return PrefixCommit::MissingOwner;
    };
    let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Secondary) else {
        return PrefixCommit::NotAcquisition;
    };
    let Some(ActorTaskRuntime::TargetAcquisition(state)) =
        entity.actor_tasks.task_state_mut(task_id)
    else {
        return PrefixCommit::NotAcquisition;
    };
    PrefixCommit::Applied(state.before_callback())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor_task_dispatcher::ActorTaskRuntime;
    use crate::actor_task_owner::{ActorTaskSlot, PreparedActorTask};
    use crate::aim_and_fire::{
        AimAndFireInvalidTargetReason, AimAndFireLifetimeStatus, AimAndFireTaggedSingleton,
    };
    use crate::chase_target::{
        ChaseTargetCommonMoverPath, ChaseTargetLifetimeStatus, ChaseTargetTaggedSingleton,
    };
    use crate::entity::EntityKind;
    use crate::entity_behavior::{
        audited_behavior_style, behavior_program, BehaviorChoiceListSource,
    };
    use crate::entity_collision_state::{EntityInitializerSpec, RetailStateWord, DYING_STATE_BIT};
    use crate::search_attack::{
        SearchAttackCandidateFilter, SearchAttackRadius, SearchAttackTarget,
    };
    use crate::search_attack_acquisition::{
        TargetAcquisitionCallbackResult, TargetAcquisitionTaggedSingleton,
        TargetAcquisitionTaskState, TargetAcquisitionZeroReason,
    };
    use crate::world_fx::{PositionalSoundEvent, WorldFx};
    use std::num::NonZeroU32;
    use v2k_formats::collision::CommonAxisDescriptor;

    const OWNER_ID: u32 = 0x0497_0001;
    const NEAR_ID: u32 = 0x047F_0001;
    const FAR_ID: u32 = 0x047F_0002;
    const CARGO_ID: u32 = 0x04C1_0001;

    fn live_entity(id: u32, entity_type: u32, position_raw: [i16; 3]) -> Entity {
        let mut entity = Entity::unresolved_port_entity(id, EntityKind::Enemy, entity_type);
        entity.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        entity.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        entity.set_position_raw(position_raw);
        entity
    }

    fn install_class7_variant0(entity: &mut Entity) {
        let program = behavior_program(u32::from(SEARCH_ATTACK_BEHAVIOR_CLASS_ID)).unwrap();
        let style = *audited_behavior_style(u32::from(SEARCH_ATTACK_BEHAVIOR_CLASS_ID), 0).unwrap();
        entity.current_behavior_context = RetailRuntimeValue::Known(Some(
            BehaviorContextRuntime::named_audited(
                program,
                0,
                RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                RetailRuntimeValue::Known(None),
                RetailRuntimeValue::Known(0),
                style,
            )
            .unwrap(),
        ));
    }

    fn install_acquisition(
        entity: &mut Entity,
        radius: SearchAttackRadius,
        filter: SearchAttackCandidateFilter,
        constructor_filter_override_raw: u32,
    ) {
        entity.actor_tasks.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::TargetAcquisition(
                TargetAcquisitionTaskState::new(radius, filter, constructor_filter_override_raw),
            )),
        );
    }

    fn apply_live(
        manager: &mut EntityManager,
        owner_id: u32,
    ) -> SearchAttackLiveAcquisitionOutcome {
        apply_search_attack_acquisition_live_without_handoff(
            manager,
            owner_id,
            0,
            &mut WorldFx::new(),
        )
    }

    #[test]
    fn live_snapshot_preserves_intrusive_order_including_attached_cargo() {
        let mut cargo = live_entity(CARGO_ID, 61, [4, 0, 0]);
        cargo.attached_to = Some(OWNER_ID);
        let manager = EntityManager::from_entities_for_test(vec![
            live_entity(OWNER_ID, 13, [0; 3]),
            live_entity(NEAR_ID, 17, [10, 0, 0]),
            cargo,
        ]);
        let snapshot = search_attack_live_list_snapshot(&manager);
        assert_eq!(
            snapshot.iter().map(|entity| entity.id).collect::<Vec<_>>(),
            [OWNER_ID, NEAR_ID, CARGO_ID]
        );
        assert_eq!(snapshot[2].capability_flags, RetailRuntimeValue::Known(0));
    }

    #[test]
    fn live_acquisition_selects_nearest_candidate_without_a_behavior_handoff() {
        let mut owner = live_entity(OWNER_ID, 13, [0; 3]);
        install_acquisition(
            &mut owner,
            SearchAttackRadius::strict(200).unwrap(),
            SearchAttackCandidateFilter::CapabilityMask(NonZeroU32::new(1).unwrap()),
            0,
        );
        owner.capability_flags = 1;
        let mut near = live_entity(NEAR_ID, 17, [8, 0, 0]);
        near.capability_flags = 1;
        let mut far = live_entity(FAR_ID, 17, [40, 0, 0]);
        far.capability_flags = 1;
        let mut manager = EntityManager::from_entities_for_test(vec![owner, far, near]);
        let outcome = apply_live(&mut manager, OWNER_ID);
        assert_eq!(
            outcome,
            SearchAttackLiveAcquisitionOutcome::Applied {
                entity_id: OWNER_ID,
                prefix: TargetAcquisitionCallbackPrefix {
                    radius: SearchAttackRadius::strict(200).unwrap(),
                    filter: SearchAttackCandidateFilter::CapabilityMask(
                        NonZeroU32::new(1).unwrap()
                    ),
                },
                result: TargetAcquisitionCallbackResult::Zero(
                    TargetAcquisitionZeroReason::BehaviorHandoffAbsent {
                        target: SearchAttackTarget {
                            id: NEAR_ID,
                            scaled_distance_squared_raw: 16,
                        },
                    }
                ),
                same_pass_aim: None,
            }
        );
    }

    #[test]
    fn live_acquisition_consumes_the_no_target_tag_without_a_handoff() {
        let mut owner = live_entity(OWNER_ID, 13, [0; 3]);
        install_acquisition(
            &mut owner,
            SearchAttackRadius::strict(4).unwrap(),
            SearchAttackCandidateFilter::SameEntityType,
            0,
        );
        let far = live_entity(FAR_ID, 13, [100, 0, 0]);
        let mut manager = EntityManager::from_entities_for_test(vec![owner, far]);
        let SearchAttackLiveAcquisitionOutcome::Applied { result, .. } =
            apply_live(&mut manager, OWNER_ID)
        else {
            panic!("expected applied no-target visit");
        };
        assert!(matches!(
            result,
            TargetAcquisitionCallbackResult::Zero(
                TargetAcquisitionZeroReason::SelectorTagConsumed { .. }
            )
        ));
    }

    #[test]
    fn missing_acquisition_task_is_not_applicable() {
        let owner = live_entity(OWNER_ID, 13, [0; 3]);
        let mut manager = EntityManager::from_entities_for_test(vec![owner]);
        assert_eq!(
            apply_live(&mut manager, OWNER_ID),
            SearchAttackLiveAcquisitionOutcome::NotApplicable
        );
    }

    #[test]
    fn dying_candidate_is_skipped_and_does_not_invent_a_handoff() {
        let mut owner = live_entity(OWNER_ID, 13, [0; 3]);
        install_acquisition(
            &mut owner,
            SearchAttackRadius::strict(200).unwrap(),
            SearchAttackCandidateFilter::SameEntityType,
            0,
        );
        let mut dying = live_entity(NEAR_ID, 13, [8, 0, 0]);
        dying
            .collision
            .state_flags_at_0x08
            .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
        let live = live_entity(FAR_ID, 13, [12, 0, 0]);
        let mut manager = EntityManager::from_entities_for_test(vec![owner, dying, live]);
        let SearchAttackLiveAcquisitionOutcome::Applied { result, .. } =
            apply_live(&mut manager, OWNER_ID)
        else {
            panic!("expected applied live visit");
        };
        assert_eq!(
            result,
            TargetAcquisitionCallbackResult::Zero(
                TargetAcquisitionZeroReason::BehaviorHandoffAbsent {
                    target: SearchAttackTarget {
                        id: FAR_ID,
                        scaled_distance_squared_raw: 36,
                    },
                }
            )
        );
    }

    #[test]
    fn constructor_filter_override_commits_before_the_live_walk() {
        let mut owner = live_entity(OWNER_ID, 13, [0; 3]);
        install_acquisition(
            &mut owner,
            SearchAttackRadius::Unbounded,
            SearchAttackCandidateFilter::SameEntityType,
            0x20,
        );
        let mut masked = live_entity(NEAR_ID, 17, [8, 0, 0]);
        masked.capability_flags = 0x20;
        let same_type = live_entity(FAR_ID, 13, [4, 0, 0]);
        let mut manager = EntityManager::from_entities_for_test(vec![owner, same_type, masked]);
        let outcome = apply_live(&mut manager, OWNER_ID);
        let SearchAttackLiveAcquisitionOutcome::Applied { prefix, result, .. } = outcome else {
            panic!("expected applied override visit: {outcome:?}");
        };
        assert_eq!(
            prefix.filter,
            SearchAttackCandidateFilter::CapabilityMask(NonZeroU32::new(0x20).unwrap())
        );
        let Some(ActorTaskRuntime::TargetAcquisition(state)) = manager
            .iter_all()
            .find(|entity| entity.id == OWNER_ID)
            .and_then(|entity| entity.actor_task_state(ActorTaskSlot::Secondary))
        else {
            panic!("acquisition task remains installed");
        };
        assert_eq!(state.filter(), prefix.filter);
        assert_eq!(
            result,
            TargetAcquisitionCallbackResult::Zero(
                TargetAcquisitionZeroReason::BehaviorHandoffAbsent {
                    target: SearchAttackTarget {
                        id: NEAR_ID,
                        scaled_distance_squared_raw: 16,
                    },
                }
            )
        );
    }

    #[test]
    fn class7_c7d0_stores_target_and_publishes_variant_one_without_ade0_or_mover() {
        let mut owner = live_entity(OWNER_ID, 13, [0; 3]);
        install_class7_variant0(&mut owner);
        owner.collision.state_flags_at_0x08.overwrite(0x8000, 0);
        install_acquisition(
            &mut owner,
            SearchAttackRadius::strict(200).unwrap(),
            SearchAttackCandidateFilter::SameEntityType,
            0,
        );
        let near = live_entity(NEAR_ID, 13, [8, 0, 0]);
        let mut manager = EntityManager::from_entities_for_test(vec![owner, near]);
        let outcome = apply_live(&mut manager, OWNER_ID);
        assert_eq!(
            outcome,
            SearchAttackLiveAcquisitionOutcome::Applied {
                entity_id: OWNER_ID,
                prefix: TargetAcquisitionCallbackPrefix {
                    radius: SearchAttackRadius::strict(200).unwrap(),
                    filter: SearchAttackCandidateFilter::SameEntityType,
                },
                result: TargetAcquisitionCallbackResult::TaggedTargetAccepted {
                    target: SearchAttackTarget {
                        id: NEAR_ID,
                        scaled_distance_squared_raw: 16,
                    },
                    singleton: TargetAcquisitionTaggedSingleton::TargetAccepted,
                },
                same_pass_aim: None,
            }
        );
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == OWNER_ID)
            .unwrap();
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("class-7 context remains published");
        };
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(NEAR_ID))
        );
        assert_eq!(context.style_table_index_raw_at_0x10(), 1);
        let style = context.active_style().audited().expect("audited class-7");
        assert_eq!(style.class_id, SEARCH_ATTACK_BEHAVIOR_CLASS_ID);
        assert_eq!(style.variant, 1);
        assert_eq!(style.frame_address, 0x004C_7A98);
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(0x8000),
            RetailRuntimeValue::Known(0x8000)
        );
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Secondary),
            Some(ActorTaskRuntime::TargetAcquisition(_))
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Primary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
    }

    #[test]
    fn class7_ade0_installs_aim_and_chase_without_a_type13_mover() {
        use crate::entity_collision_state::{
            CommonMoverComponentTopology, EntityTypeRuntimeMetadata,
        };

        let mut owner = live_entity(OWNER_ID, 13, [0; 3]);
        install_class7_variant0(&mut owner);
        install_acquisition(
            &mut owner,
            SearchAttackRadius::strict(200).unwrap(),
            SearchAttackCandidateFilter::SameEntityType,
            0,
        );
        let near = live_entity(NEAR_ID, 13, [8, 0, 0]);
        let mut metadata = EntityTypeRuntimeMetadata::default();
        metadata.common_mover_topology =
            RetailRuntimeValue::Known(CommonMoverComponentTopology::default());
        metadata.search_attack_optional_prelude_sound_id = RetailRuntimeValue::Known(None);
        metadata.search_attack_aim_sound_id = RetailRuntimeValue::Known(None);
        metadata.search_attack_aim_sound_period_raw = RetailRuntimeValue::Known(0);
        let mut table = vec![EntityTypeRuntimeMetadata::default(); 14];
        table[13] = metadata;
        let mut manager = EntityManager::from_entities_with_type_metadata_for_test(
            vec![owner, near],
            table,
            false,
        );
        let outcome = apply_live(&mut manager, OWNER_ID);
        assert!(matches!(
            outcome,
            SearchAttackLiveAcquisitionOutcome::Applied {
                result: TargetAcquisitionCallbackResult::TaggedTargetAccepted { .. },
                ..
            }
        ));
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == OWNER_ID)
            .unwrap();
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Tertiary),
            Some(ActorTaskRuntime::AimAndFire(task)) if task.elapsed_ms() == 0
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::ChaseTarget(task)) if task.target_id() == NEAR_ID
        ));
    }

    #[test]
    fn class7_ade0_copies_authored_aim_sound_and_signed_period() {
        use crate::entity_collision_state::{
            CommonMoverComponentTopology, EntityTypeRuntimeMetadata,
        };

        let mut owner = live_entity(OWNER_ID, 13, [0; 3]);
        install_class7_variant0(&mut owner);
        install_acquisition(
            &mut owner,
            SearchAttackRadius::strict(200).unwrap(),
            SearchAttackCandidateFilter::SameEntityType,
            0,
        );
        let near = live_entity(NEAR_ID, 13, [8, 0, 0]);
        let mut metadata = EntityTypeRuntimeMetadata::default();
        metadata.common_mover_topology =
            RetailRuntimeValue::Known(CommonMoverComponentTopology::default());
        metadata.search_attack_optional_prelude_sound_id = RetailRuntimeValue::Known(None);
        metadata.search_attack_aim_sound_id = RetailRuntimeValue::Known(Some(70));
        metadata.search_attack_aim_sound_period_raw = RetailRuntimeValue::Known(0x0400);
        let mut table = vec![EntityTypeRuntimeMetadata::default(); 14];
        table[13] = metadata;
        let mut manager = EntityManager::from_entities_with_type_metadata_for_test(
            vec![owner, near],
            table,
            false,
        );
        assert!(matches!(
            apply_live(&mut manager, OWNER_ID),
            SearchAttackLiveAcquisitionOutcome::Applied { .. }
        ));
        let Some(ActorTaskRuntime::AimAndFire(task)) = manager
            .iter_all()
            .find(|entity| entity.id == OWNER_ID)
            .and_then(|entity| entity.actor_task_state(ActorTaskSlot::Tertiary))
        else {
            panic!("ADE0 published Aim-and-Fire");
        };
        assert_eq!(task.private_state().optional_sound_id_raw(), 70);
        assert_eq!(task.private_state().sound_period_us_raw(), 0x0400);
    }

    fn ade0_audio_metadata(
        prelude: RetailRuntimeValue<Option<u16>>,
        aim_sound: RetailRuntimeValue<Option<u16>>,
        aim_period: RetailRuntimeValue<i32>,
    ) -> EntityTypeRuntimeMetadata {
        use crate::entity_collision_state::CommonMoverComponentTopology;

        let mut metadata = EntityTypeRuntimeMetadata::default();
        metadata.common_mover_topology =
            RetailRuntimeValue::Known(CommonMoverComponentTopology::default());
        metadata.search_attack_optional_prelude_sound_id = prelude;
        metadata.search_attack_aim_sound_id = aim_sound;
        metadata.search_attack_aim_sound_period_raw = aim_period;
        metadata
    }

    #[test]
    fn class7_ade0_plays_known_prelude_at_entity_position() {
        let owner_position_raw = [256, 512, 768];
        let mut owner = live_entity(OWNER_ID, 13, owner_position_raw);
        install_class7_variant0(&mut owner);
        install_acquisition(
            &mut owner,
            SearchAttackRadius::strict(200).unwrap(),
            SearchAttackCandidateFilter::SameEntityType,
            0,
        );
        let near = live_entity(NEAR_ID, 13, [264, 512, 768]);
        let mut table = vec![EntityTypeRuntimeMetadata::default(); 14];
        table[13] = ade0_audio_metadata(
            RetailRuntimeValue::Known(Some(11)),
            RetailRuntimeValue::Known(None),
            RetailRuntimeValue::Known(0),
        );
        let mut manager = EntityManager::from_entities_with_type_metadata_for_test(
            vec![owner, near],
            table,
            false,
        );
        let mut world_fx = WorldFx::new();
        assert!(matches!(
            apply_search_attack_acquisition_live_without_handoff(
                &mut manager,
                OWNER_ID,
                0,
                &mut world_fx,
            ),
            SearchAttackLiveAcquisitionOutcome::Applied {
                result: TargetAcquisitionCallbackResult::TaggedTargetAccepted { .. },
                ..
            }
        ));
        world_fx.process_pending();
        assert_eq!(
            world_fx.take_positional_sounds(),
            vec![PositionalSoundEvent::fixed(11, [1.0, 2.0, 3.0])]
        );
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == OWNER_ID)
            .unwrap();
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Tertiary),
            Some(ActorTaskRuntime::AimAndFire(_))
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::ChaseTarget(_))
        ));
    }

    #[test]
    fn class7_ade0_skips_zero_prelude() {
        let mut owner = live_entity(OWNER_ID, 13, [256, 512, 768]);
        install_class7_variant0(&mut owner);
        install_acquisition(
            &mut owner,
            SearchAttackRadius::strict(200).unwrap(),
            SearchAttackCandidateFilter::SameEntityType,
            0,
        );
        let near = live_entity(NEAR_ID, 13, [264, 512, 768]);
        let mut table = vec![EntityTypeRuntimeMetadata::default(); 14];
        table[13] = ade0_audio_metadata(
            RetailRuntimeValue::Known(None),
            RetailRuntimeValue::Known(None),
            RetailRuntimeValue::Known(0),
        );
        let mut manager = EntityManager::from_entities_with_type_metadata_for_test(
            vec![owner, near],
            table,
            false,
        );
        let mut world_fx = WorldFx::new();
        assert!(matches!(
            apply_search_attack_acquisition_live_without_handoff(
                &mut manager,
                OWNER_ID,
                0,
                &mut world_fx,
            ),
            SearchAttackLiveAcquisitionOutcome::Applied { .. }
        ));
        world_fx.process_pending();
        assert!(world_fx.take_positional_sounds().is_empty());
    }

    #[test]
    fn class7_ade0_unresolved_prelude_fails_closed_without_slots_or_sound() {
        let mut owner = live_entity(OWNER_ID, 13, [256, 512, 768]);
        install_class7_variant0(&mut owner);
        install_acquisition(
            &mut owner,
            SearchAttackRadius::strict(200).unwrap(),
            SearchAttackCandidateFilter::SameEntityType,
            0,
        );
        let near = live_entity(NEAR_ID, 13, [264, 512, 768]);
        let mut table = vec![EntityTypeRuntimeMetadata::default(); 14];
        table[13] = ade0_audio_metadata(
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Known(None),
            RetailRuntimeValue::Known(0),
        );
        let mut manager = EntityManager::from_entities_with_type_metadata_for_test(
            vec![owner, near],
            table,
            false,
        );
        let mut world_fx = WorldFx::new();
        assert!(matches!(
            apply_search_attack_acquisition_live_without_handoff(
                &mut manager,
                OWNER_ID,
                0,
                &mut world_fx,
            ),
            SearchAttackLiveAcquisitionOutcome::Applied {
                result: TargetAcquisitionCallbackResult::TaggedTargetAccepted { .. },
                ..
            }
        ));
        world_fx.process_pending();
        assert!(world_fx.take_positional_sounds().is_empty());
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == OWNER_ID)
            .unwrap();
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Secondary),
            Some(ActorTaskRuntime::TargetAcquisition(_))
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Primary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
    }

    #[test]
    fn class7_ade0_visits_aim_same_pass_without_chase_or_emitter() {
        let mut owner = live_entity(OWNER_ID, 13, [0; 3]);
        install_class7_variant0(&mut owner);
        install_acquisition(
            &mut owner,
            SearchAttackRadius::strict(200).unwrap(),
            SearchAttackCandidateFilter::SameEntityType,
            0,
        );
        let near = live_entity(NEAR_ID, 13, [8, 0, 0]);
        let mut table = vec![EntityTypeRuntimeMetadata::default(); 14];
        table[13] = aim_metadata(
            RetailRuntimeValue::Known(None),
            RetailRuntimeValue::Known(0),
            RetailRuntimeValue::Known(None),
        );
        let mut manager = EntityManager::from_entities_with_type_metadata_for_test(
            vec![owner, near],
            table,
            false,
        );
        let mut world_fx = WorldFx::new();
        let outcome = apply_search_attack_acquisition_live_without_handoff(
            &mut manager,
            OWNER_ID,
            20_000,
            &mut world_fx,
        );
        assert_eq!(
            outcome,
            SearchAttackLiveAcquisitionOutcome::Applied {
                entity_id: OWNER_ID,
                prefix: TargetAcquisitionCallbackPrefix {
                    radius: SearchAttackRadius::strict(200).unwrap(),
                    filter: SearchAttackCandidateFilter::SameEntityType,
                },
                result: TargetAcquisitionCallbackResult::TaggedTargetAccepted {
                    target: SearchAttackTarget {
                        id: NEAR_ID,
                        scaled_distance_squared_raw: 16,
                    },
                    singleton: TargetAcquisitionTaggedSingleton::TargetAccepted,
                },
                same_pass_aim: Some(SearchAttackLiveAimOutcome::Applied {
                    entity_id: OWNER_ID,
                    prefix: AimAndFireCallbackPrefix {
                        elapsed_ms: 20,
                        lifetime_status: AimAndFireLifetimeStatus::WithinLifetime,
                    },
                    result: SearchAttackLiveAimResult::Zero,
                }),
            }
        );
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == OWNER_ID)
            .unwrap();
        let Some(ActorTaskRuntime::AimAndFire(aim)) =
            entity.actor_task_state(ActorTaskSlot::Tertiary)
        else {
            panic!("same-pass Aim remains installed");
        };
        assert_eq!(aim.elapsed_ms(), 20);
        let Some(ActorTaskRuntime::ChaseTarget(chase)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("Chase is published but not visited this pass");
        };
        assert_eq!(chase.elapsed_ms(), 0);
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
    }

    #[test]
    fn class7_ade0_recovered_type13_sub_e_fail_closes_same_pass_without_emitter() {
        use crate::entity_collision_state::CommonMoverComponentTopology;

        let mut metadata = aim_metadata(
            RetailRuntimeValue::Known(None),
            RetailRuntimeValue::Known(0),
            RetailRuntimeValue::Known(Some(TYPE13_SEARCH_ATTACK_PROJECTILE_DESCRIPTOR)),
        );
        metadata.common_mover_topology = RetailRuntimeValue::Known(CommonMoverComponentTopology {
            sub_d: true,
            sub_e: true,
            sub_g: true,
            sub_k: true,
            sub_l: true,
            ..CommonMoverComponentTopology::default()
        });
        let mut owner = live_entity(OWNER_ID, 13, [0; 3]);
        install_class7_variant0(&mut owner);
        install_acquisition(
            &mut owner,
            SearchAttackRadius::strict(200).unwrap(),
            SearchAttackCandidateFilter::SameEntityType,
            0,
        );
        let near = live_entity(NEAR_ID, 13, [8, 0, 0]);
        let mut table = vec![EntityTypeRuntimeMetadata::default(); 14];
        table[13] = metadata;
        let mut manager = EntityManager::from_entities_with_type_metadata_for_test(
            vec![owner, near],
            table,
            false,
        );
        let mut world_fx = WorldFx::new();
        let outcome = apply_search_attack_acquisition_live_without_handoff(
            &mut manager,
            OWNER_ID,
            20_000,
            &mut world_fx,
        );
        assert_eq!(
            outcome,
            SearchAttackLiveAcquisitionOutcome::Applied {
                entity_id: OWNER_ID,
                prefix: TargetAcquisitionCallbackPrefix {
                    radius: SearchAttackRadius::strict(200).unwrap(),
                    filter: SearchAttackCandidateFilter::SameEntityType,
                },
                result: TargetAcquisitionCallbackResult::TaggedTargetAccepted {
                    target: SearchAttackTarget {
                        id: NEAR_ID,
                        scaled_distance_squared_raw: 16,
                    },
                    singleton: TargetAcquisitionTaggedSingleton::TargetAccepted,
                },
                same_pass_aim: Some(SearchAttackLiveAimOutcome::Blocked {
                    entity_id: OWNER_ID,
                    reason: SearchAttackLiveAimBlock::GenericEmitterUninvented,
                }),
            }
        );
        let Some(ActorTaskRuntime::AimAndFire(aim)) = manager
            .iter_all()
            .find(|entity| entity.id == OWNER_ID)
            .and_then(|entity| entity.actor_task_state(ActorTaskSlot::Tertiary))
        else {
            panic!("Aim remains after the recovered Sub-E fail-closed");
        };
        assert_eq!(aim.elapsed_ms(), 20);
    }

    fn publish_ade0(
        owner_position_raw: [i16; 3],
        target_position_raw: [i16; 3],
        axis_limit_raw: Option<i32>,
    ) -> EntityManager {
        publish_ade0_with_metadata(
            owner_position_raw,
            target_position_raw,
            axis_limit_raw,
            ade0_audio_metadata(
                RetailRuntimeValue::Known(None),
                RetailRuntimeValue::Known(None),
                RetailRuntimeValue::Known(0),
            ),
        )
    }

    fn publish_ade0_with_metadata(
        owner_position_raw: [i16; 3],
        target_position_raw: [i16; 3],
        axis_limit_raw: Option<i32>,
        metadata: EntityTypeRuntimeMetadata,
    ) -> EntityManager {
        let mut owner = live_entity(OWNER_ID, 13, owner_position_raw);
        install_class7_variant0(&mut owner);
        install_acquisition(
            &mut owner,
            SearchAttackRadius::strict(200).unwrap(),
            SearchAttackCandidateFilter::SameEntityType,
            0,
        );
        if let Some(strict_axis_limit_raw) = axis_limit_raw {
            owner.actor_common_axis_descriptor = RetailRuntimeValue::Known(CommonAxisDescriptor {
                strict_axis_limit_raw,
                raw_word_at_0x04: 0,
            });
        }
        let near = live_entity(NEAR_ID, 13, target_position_raw);
        let mut table = vec![EntityTypeRuntimeMetadata::default(); 14];
        table[13] = metadata;
        let mut manager = EntityManager::from_entities_with_type_metadata_for_test(
            vec![owner, near],
            table,
            false,
        );
        assert!(matches!(
            apply_live(&mut manager, OWNER_ID),
            SearchAttackLiveAcquisitionOutcome::Applied {
                result: TargetAcquisitionCallbackResult::TaggedTargetAccepted { .. },
                ..
            }
        ));
        manager
    }

    #[test]
    fn class7_chase_visit_is_not_applicable_before_ade0() {
        let mut owner = live_entity(OWNER_ID, 13, [0; 3]);
        install_class7_variant0(&mut owner);
        install_acquisition(
            &mut owner,
            SearchAttackRadius::strict(200).unwrap(),
            SearchAttackCandidateFilter::SameEntityType,
            0,
        );
        let mut manager =
            EntityManager::from_entities_for_test(vec![owner, live_entity(NEAR_ID, 13, [8, 0, 0])]);
        assert_eq!(
            apply_search_attack_chase_visit_without_mover(&mut manager, OWNER_ID, 20_000),
            SearchAttackLiveChaseOutcome::NotApplicable
        );
    }

    #[test]
    fn class7_chase_visit_blocks_unresolved_axis_before_lifetime() {
        let mut manager = publish_ade0([0; 3], [8, 0, 0], None);
        assert_eq!(
            apply_search_attack_chase_visit_without_mover(&mut manager, OWNER_ID, 20_000),
            SearchAttackLiveChaseOutcome::Blocked {
                entity_id: OWNER_ID,
                reason: SearchAttackLiveChaseBlock::CommonAxisUnresolved,
            }
        );
        let Some(ActorTaskRuntime::ChaseTarget(chase)) = manager
            .iter_all()
            .find(|entity| entity.id == OWNER_ID)
            .and_then(|entity| entity.actor_task_state(ActorTaskSlot::Primary))
        else {
            panic!("ADE0 Primary remains Chase");
        };
        assert_eq!(chase.elapsed_ms(), 0);
    }

    #[test]
    fn class7_chase_visit_advances_lifetime_and_fail_closes_in_range_without_type13_mover() {
        let mut manager = publish_ade0([0; 3], [8, 0, 0], Some(0x1000));
        assert_eq!(
            apply_search_attack_chase_visit_without_mover(&mut manager, OWNER_ID, 20_000),
            SearchAttackLiveChaseOutcome::Applied {
                entity_id: OWNER_ID,
                prefix: ChaseTargetCallbackPrefix {
                    lifetime_status: ChaseTargetLifetimeStatus::WithinLifetime,
                },
                result: SearchAttackLiveChaseResult::Type13MoverUninvented {
                    path: ChaseTargetCommonMoverPath::InRange,
                },
                bound_sub_d: None,
                bound_gkl: None,
            }
        );
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == OWNER_ID)
            .unwrap();
        let Some(ActorTaskRuntime::ChaseTarget(chase)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("Chase remains after the fail-closed mover");
        };
        assert_eq!(chase.elapsed_ms(), 20);
        assert_eq!(chase.target_id(), NEAR_ID);
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Tertiary),
            Some(ActorTaskRuntime::AimAndFire(_))
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
    }

    #[test]
    fn class7_chase_visit_classifies_out_of_range_then_fail_closes_without_type13_mover() {
        let mut manager = publish_ade0([0; 3], [8, 0, 0], Some(4));
        assert_eq!(
            apply_search_attack_chase_visit_without_mover(&mut manager, OWNER_ID, 20_000),
            SearchAttackLiveChaseOutcome::Applied {
                entity_id: OWNER_ID,
                prefix: ChaseTargetCallbackPrefix {
                    lifetime_status: ChaseTargetLifetimeStatus::WithinLifetime,
                },
                result: SearchAttackLiveChaseResult::Type13MoverUninvented {
                    path: ChaseTargetCommonMoverPath::OutOfRange,
                },
                bound_sub_d: None,
                bound_gkl: None,
            }
        );
    }

    #[test]
    fn class7_chase_visit_tags_invalid_target_without_a_mover() {
        let mut manager = publish_ade0([0; 3], [8, 0, 0], Some(0x1000));
        manager.entity_mut_for_test(NEAR_ID).unwrap().active = false;
        assert_eq!(
            apply_search_attack_chase_visit_without_mover(&mut manager, OWNER_ID, 20_000),
            SearchAttackLiveChaseOutcome::Applied {
                entity_id: OWNER_ID,
                prefix: ChaseTargetCallbackPrefix {
                    lifetime_status: ChaseTargetLifetimeStatus::WithinLifetime,
                },
                result: SearchAttackLiveChaseResult::Tagged(
                    ChaseTargetTaggedSingleton::InvalidTarget
                ),
                bound_sub_d: None,
                bound_gkl: None,
            }
        );
        let Some(ActorTaskRuntime::ChaseTarget(chase)) = manager
            .iter_all()
            .find(|entity| entity.id == OWNER_ID)
            .and_then(|entity| entity.actor_task_state(ActorTaskSlot::Primary))
        else {
            panic!("Chase remains after the invalid-target tag");
        };
        assert_eq!(chase.elapsed_ms(), 20);
    }

    fn type13_axis_metadata() -> EntityTypeRuntimeMetadata {
        let mut metadata = ade0_audio_metadata(
            RetailRuntimeValue::Known(None),
            RetailRuntimeValue::Known(None),
            RetailRuntimeValue::Known(0),
        );
        metadata.initializer = Some(EntityInitializerSpec {
            initializer_state_flags_raw: 0x8,
            common_axis_descriptor: TYPE13_SEARCH_ATTACK_COMMON_AXIS,
            behavior_choices: Box::from(TYPE13_SEARCH_ATTACK_BEHAVIOR_CHOICES),
            behavior_rule_ref: TYPE13_SEARCH_ATTACK_BEHAVIOR_RULE_REF,
            alternate_behavior_class_ref: TYPE13_SEARCH_ATTACK_ALTERNATE_BEHAVIOR_CLASS,
        });
        metadata
    }

    #[test]
    fn class7_chase_binds_recovered_type13_axis_from_type_record() {
        let mut manager =
            publish_ade0_with_metadata([0; 3], [8, 0, 0], None, type13_axis_metadata());
        assert_eq!(
            apply_search_attack_chase_visit_without_mover(&mut manager, OWNER_ID, 20_000),
            SearchAttackLiveChaseOutcome::Applied {
                entity_id: OWNER_ID,
                prefix: ChaseTargetCallbackPrefix {
                    lifetime_status: ChaseTargetLifetimeStatus::WithinLifetime,
                },
                result: SearchAttackLiveChaseResult::Type13MoverUninvented {
                    path: ChaseTargetCommonMoverPath::InRange,
                },
                bound_sub_d: None,
                bound_gkl: None,
            }
        );
        manager
            .entity_mut_for_test(NEAR_ID)
            .unwrap()
            .set_position_raw([0x1900, 0, 0]);
        assert_eq!(
            apply_search_attack_chase_visit_without_mover(&mut manager, OWNER_ID, 20_000),
            SearchAttackLiveChaseOutcome::Applied {
                entity_id: OWNER_ID,
                prefix: ChaseTargetCallbackPrefix {
                    lifetime_status: ChaseTargetLifetimeStatus::WithinLifetime,
                },
                result: SearchAttackLiveChaseResult::Type13MoverUninvented {
                    path: ChaseTargetCommonMoverPath::OutOfRange,
                },
                bound_sub_d: None,
                bound_gkl: None,
            }
        );
    }

    #[test]
    fn class7_chase_binds_recovered_type13_sub_d_without_inventing_a_mover() {
        use crate::entity_collision_state::CommonMoverComponentTopology;

        let mut metadata = type13_axis_metadata();
        metadata.common_mover_topology = RetailRuntimeValue::Known(CommonMoverComponentTopology {
            sub_d: true,
            sub_e: true,
            sub_g: true,
            sub_k: true,
            sub_l: true,
            ..CommonMoverComponentTopology::default()
        });
        metadata.sub_d_steering_descriptor =
            RetailRuntimeValue::Known(Some(TYPE13_SEARCH_ATTACK_SUB_D));
        metadata.common_mover_gkl_payloads = RetailRuntimeValue::Known(TYPE13_SEARCH_ATTACK_GKL);
        let mut manager = publish_ade0_with_metadata([0; 3], [8, 0, 0], None, metadata);
        assert_eq!(
            apply_search_attack_chase_visit_without_mover(&mut manager, OWNER_ID, 20_000),
            SearchAttackLiveChaseOutcome::Applied {
                entity_id: OWNER_ID,
                prefix: ChaseTargetCallbackPrefix {
                    lifetime_status: ChaseTargetLifetimeStatus::WithinLifetime,
                },
                result: SearchAttackLiveChaseResult::Type13MoverUninvented {
                    path: ChaseTargetCommonMoverPath::InRange,
                },
                bound_sub_d: Some(TYPE13_SEARCH_ATTACK_SUB_D),
                bound_gkl: Some(TYPE13_SEARCH_ATTACK_GKL),
            }
        );
    }

    #[test]
    fn class7_chase_blocks_authored_unresolved_gkl_before_lifetime() {
        use crate::entity_collision_state::CommonMoverComponentTopology;

        let mut metadata = type13_axis_metadata();
        metadata.common_mover_topology = RetailRuntimeValue::Known(CommonMoverComponentTopology {
            sub_g: true,
            ..CommonMoverComponentTopology::default()
        });
        let mut manager = publish_ade0_with_metadata([0; 3], [8, 0, 0], None, metadata);
        assert_eq!(
            apply_search_attack_chase_visit_without_mover(&mut manager, OWNER_ID, 20_000),
            SearchAttackLiveChaseOutcome::Blocked {
                entity_id: OWNER_ID,
                reason: SearchAttackLiveChaseBlock::GklUnresolved,
            }
        );
        let Some(ActorTaskRuntime::ChaseTarget(chase)) = manager
            .iter_all()
            .find(|entity| entity.id == OWNER_ID)
            .and_then(|entity| entity.actor_task_state(ActorTaskSlot::Primary))
        else {
            panic!("Chase remains before the G/K/L block");
        };
        assert_eq!(chase.elapsed_ms(), 0);
    }

    #[test]
    fn class7_chase_blocks_authored_unresolved_sub_d_before_lifetime() {
        use crate::entity_collision_state::CommonMoverComponentTopology;

        let mut metadata = type13_axis_metadata();
        metadata.common_mover_topology = RetailRuntimeValue::Known(CommonMoverComponentTopology {
            sub_d: true,
            ..CommonMoverComponentTopology::default()
        });
        let mut manager = publish_ade0_with_metadata([0; 3], [8, 0, 0], None, metadata);
        assert_eq!(
            apply_search_attack_chase_visit_without_mover(&mut manager, OWNER_ID, 20_000),
            SearchAttackLiveChaseOutcome::Blocked {
                entity_id: OWNER_ID,
                reason: SearchAttackLiveChaseBlock::SubDUnresolved,
            }
        );
        let Some(ActorTaskRuntime::ChaseTarget(chase)) = manager
            .iter_all()
            .find(|entity| entity.id == OWNER_ID)
            .and_then(|entity| entity.actor_task_state(ActorTaskSlot::Primary))
        else {
            panic!("Chase remains before the Sub-D block");
        };
        assert_eq!(chase.elapsed_ms(), 0);
    }

    #[test]
    fn class7_type13_initializer_authors_search_attack_choice() {
        let initializer = type13_axis_metadata()
            .initializer
            .expect("type-13 initializer");
        assert_eq!(
            initializer.behavior_choices.as_ref(),
            TYPE13_SEARCH_ATTACK_BEHAVIOR_CHOICES.as_slice()
        );
        assert!(initializer
            .behavior_choices
            .iter()
            .any(
                |choice| choice.behavior_class_id == u32::from(SEARCH_ATTACK_BEHAVIOR_CLASS_ID)
                    && choice.weight_rule_id == 1
                    && choice.weight_multiplier == 3
            ));
        assert_eq!(
            initializer.behavior_rule_ref,
            TYPE13_SEARCH_ATTACK_BEHAVIOR_RULE_REF
        );
        assert_eq!(
            initializer.alternate_behavior_class_ref,
            TYPE13_SEARCH_ATTACK_ALTERNATE_BEHAVIOR_CLASS
        );
    }

    fn aim_metadata(
        aim_sound: RetailRuntimeValue<Option<u16>>,
        aim_period: RetailRuntimeValue<i32>,
        sub_e: RetailRuntimeValue<Option<v2k_formats::collision::ProjectileEmitterDescriptor>>,
    ) -> EntityTypeRuntimeMetadata {
        let mut metadata =
            ade0_audio_metadata(RetailRuntimeValue::Known(None), aim_sound, aim_period);
        metadata.projectile_emitter_descriptor = sub_e;
        metadata
    }

    #[test]
    fn class7_aim_visit_is_not_applicable_before_ade0() {
        let mut owner = live_entity(OWNER_ID, 13, [0; 3]);
        install_class7_variant0(&mut owner);
        install_acquisition(
            &mut owner,
            SearchAttackRadius::strict(200).unwrap(),
            SearchAttackCandidateFilter::SameEntityType,
            0,
        );
        let mut manager =
            EntityManager::from_entities_for_test(vec![owner, live_entity(NEAR_ID, 13, [8, 0, 0])]);
        assert_eq!(
            apply_search_attack_aim_visit_without_emitter(
                &mut manager,
                OWNER_ID,
                20_000,
                &mut WorldFx::new(),
            ),
            SearchAttackLiveAimOutcome::NotApplicable
        );
    }

    #[test]
    fn class7_aim_visit_unresolved_sub_e_fail_closes_after_lifetime() {
        let mut manager = publish_ade0([0; 3], [8, 0, 0], Some(0x1000));
        assert_eq!(
            apply_search_attack_aim_visit_without_emitter(
                &mut manager,
                OWNER_ID,
                20_000,
                &mut WorldFx::new(),
            ),
            SearchAttackLiveAimOutcome::Blocked {
                entity_id: OWNER_ID,
                reason: SearchAttackLiveAimBlock::SubEUnresolved,
            }
        );
        let Some(ActorTaskRuntime::AimAndFire(aim)) = manager
            .iter_all()
            .find(|entity| entity.id == OWNER_ID)
            .and_then(|entity| entity.actor_task_state(ActorTaskSlot::Tertiary))
        else {
            panic!("ADE0 Tertiary remains Aim");
        };
        assert_eq!(aim.elapsed_ms(), 20);
    }

    #[test]
    fn class7_aim_visit_known_null_sub_e_returns_zero_without_emitter() {
        let mut manager = publish_ade0_with_metadata(
            [0; 3],
            [8, 0, 0],
            Some(0x1000),
            aim_metadata(
                RetailRuntimeValue::Known(None),
                RetailRuntimeValue::Known(0),
                RetailRuntimeValue::Known(None),
            ),
        );
        assert_eq!(
            apply_search_attack_aim_visit_without_emitter(
                &mut manager,
                OWNER_ID,
                20_000,
                &mut WorldFx::new(),
            ),
            SearchAttackLiveAimOutcome::Applied {
                entity_id: OWNER_ID,
                prefix: AimAndFireCallbackPrefix {
                    elapsed_ms: 20,
                    lifetime_status: AimAndFireLifetimeStatus::WithinLifetime,
                },
                result: SearchAttackLiveAimResult::Zero,
            }
        );
        let Some(ActorTaskRuntime::AimAndFire(aim)) = manager
            .iter_all()
            .find(|entity| entity.id == OWNER_ID)
            .and_then(|entity| entity.actor_task_state(ActorTaskSlot::Tertiary))
        else {
            panic!("Aim remains after the recovered null-Sub-E zero");
        };
        assert_eq!(aim.elapsed_ms(), 20);
    }

    #[test]
    fn class7_aim_visit_tags_invalid_target_without_an_emitter() {
        let mut manager = publish_ade0_with_metadata(
            [0; 3],
            [8, 0, 0],
            Some(0x1000),
            aim_metadata(
                RetailRuntimeValue::Known(None),
                RetailRuntimeValue::Known(0),
                RetailRuntimeValue::Known(None),
            ),
        );
        manager
            .entity_mut_for_test(NEAR_ID)
            .unwrap()
            .collision
            .state_flags_at_0x08 = RetailStateWord::exact(0);
        assert_eq!(
            apply_search_attack_aim_visit_without_emitter(
                &mut manager,
                OWNER_ID,
                20_000,
                &mut WorldFx::new(),
            ),
            SearchAttackLiveAimOutcome::Applied {
                entity_id: OWNER_ID,
                prefix: AimAndFireCallbackPrefix {
                    elapsed_ms: 20,
                    lifetime_status: AimAndFireLifetimeStatus::WithinLifetime,
                },
                result: SearchAttackLiveAimResult::TaggedInvalidTarget {
                    singleton: AimAndFireTaggedSingleton::InvalidTarget,
                    reason: AimAndFireInvalidTargetReason::ZeroStateFlags,
                },
            }
        );
    }

    #[test]
    fn class7_aim_visit_present_sub_e_fail_closes_without_inventing_emitter() {
        let fixture = v2k_formats::collision::ProjectileEmitterDescriptor {
            projectile_method: 0,
            random_interval_us: 0,
            spread_raw: 0,
            aim_threshold_raw: 0,
            speed_override_raw: 0,
            target_axis_tolerance_raw: 0,
            sound_id: 0,
            raw_word_at_0x12: 0,
            alternate_emitter_raw: 0,
            stochastic_gate_mode: 0,
            auxiliary_command: 0,
            variable_bindings: [0; 4],
        };
        let mut manager = publish_ade0_with_metadata(
            [0; 3],
            [8, 0, 0],
            Some(0x1000),
            aim_metadata(
                RetailRuntimeValue::Known(None),
                RetailRuntimeValue::Known(0),
                RetailRuntimeValue::Known(Some(fixture)),
            ),
        );
        assert_eq!(
            apply_search_attack_aim_visit_without_emitter(
                &mut manager,
                OWNER_ID,
                20_000,
                &mut WorldFx::new(),
            ),
            SearchAttackLiveAimOutcome::Blocked {
                entity_id: OWNER_ID,
                reason: SearchAttackLiveAimBlock::GenericEmitterUninvented,
            }
        );
        let Some(ActorTaskRuntime::AimAndFire(aim)) = manager
            .iter_all()
            .find(|entity| entity.id == OWNER_ID)
            .and_then(|entity| entity.actor_task_state(ActorTaskSlot::Tertiary))
        else {
            panic!("Aim remains after the fail-closed emitter");
        };
        assert_eq!(aim.elapsed_ms(), 20);
    }

    #[test]
    fn class7_aim_visit_plays_optional_sound_before_null_sub_e_zero() {
        let mut manager = publish_ade0_with_metadata(
            [256, 512, 768],
            [264, 512, 768],
            Some(0x1000),
            aim_metadata(
                RetailRuntimeValue::Known(Some(70)),
                RetailRuntimeValue::Known(0x0400),
                RetailRuntimeValue::Known(None),
            ),
        );
        let mut world_fx = WorldFx::new();
        assert_eq!(
            apply_search_attack_aim_visit_without_emitter(
                &mut manager,
                OWNER_ID,
                20_000,
                &mut world_fx,
            ),
            SearchAttackLiveAimOutcome::Applied {
                entity_id: OWNER_ID,
                prefix: AimAndFireCallbackPrefix {
                    elapsed_ms: 20,
                    lifetime_status: AimAndFireLifetimeStatus::WithinLifetime,
                },
                result: SearchAttackLiveAimResult::Zero,
            }
        );
        world_fx.process_pending();
        assert_eq!(
            world_fx.take_positional_sounds(),
            vec![PositionalSoundEvent::fixed(70, [1.0, 2.0, 3.0])]
        );
    }
}
