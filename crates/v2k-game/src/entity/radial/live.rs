//! Callback-bearing 14AE0 traversal for a world without a player controller.
//!
//! Unlike the atomic, callback-free planner, this owner commits each retail
//! phase synchronously. A block retains the exact completed prefix, including
//! impulse, buffer, sound, or negative health at the blocked death call. The
//! caller must consume its report once; replaying the request repeats damage.

use super::*;
use crate::gameplay_notifications::GameplayNotifications;
use crate::live_actor_checked_damage::{
    LiveActorDamageEntry, LiveActorDamageError, LiveActorDamagePhase, LiveActorDamageRequest,
    LiveActorDeathResult,
};
use crate::main_base_type9_abort::MainBaseType9ExplodingTaskLease;
use crate::ordinary_type47_death_live::{
    FreshLevelOneType47CommonDyingOwner, Type47CommonDyingPublicationError,
};
use crate::ordinary_type9_standard_death::OrdinaryType9StandardDeathBlock;
use crate::world_fx::WorldFx;

/// Explicit live callback custody. Intro2 has no player entity or hull; an
/// accidental gameplay manager is rejected before the traversal starts.
pub struct DynamicRadialLiveRequest<'a> {
    pub origin_raw: [i16; 3],
    pub template: RadialDamageTemplate,
    pub world_fx: &'a mut WorldFx,
    pub retail_tick: u32,
    pub notifications: &'a mut GameplayNotifications,
    /// Authenticate native actor custody before the first radial write.
    /// Type9 transfers its completed visit; Type66 retains its current,
    /// non-pending task receipt. Both paths use the caller's live cursor.
    pub callbacks: &'a mut dyn DynamicRadialLiveCallbacks,
}

/// The current world owns reentrant death callbacks as well as task custody.
/// A class49 explosion must finish its nested radial traversal before the
/// outer checked-damage call returns or advances to its next target.
pub trait DynamicRadialLiveCallbacks {
    /// Capture's DB80 cleanup mutates the child synchronously. Production
    /// callers lend the same owner storage used by this radial traversal.
    fn capture_task_custody(
        &mut self,
    ) -> Option<&mut dyn crate::intro2_type17::capture::CaptureTaskCustody> {
        None
    }
    fn before_native_actor_mutation(&mut self, manager: &EntityManager, id: u32) -> bool;
    fn active_terminal_call(&self, _manager: &EntityManager, _id: u32) -> bool {
        false
    }
    /// Adopt a replacement before a later target can start a nested radial
    /// pass that revisits it. The outcome still records the publication.
    fn retain_death_publication(&mut self, _publication: DynamicRadialDeathPublication) {}

    fn standard_death(
        &mut self,
        _manager: &mut EntityManager,
        _id: u32,
        _kind: u32,
        _world_fx: &mut WorldFx,
        _retail_tick: u32,
        _notifications: &mut GameplayNotifications,
    ) -> Option<
        Result<LiveActorDeathResult<DynamicRadialDeathPublication>, DynamicRadialLiveBlockReason>,
    > {
        None
    }

    /// Section-8 header `+08` for a Hive67 dying model, used by `FUN_00440950`.
    fn hive_dying_burst_model_extent_raw(&self, _model_id: usize) -> Option<u16> {
        None
    }

    /// Authored sea plane when water is enabled; `None` is the dry-world burst.
    fn hive_dying_burst_sea_level_raw(&self) -> Option<i16> {
        None
    }
}

impl<F: FnMut(&EntityManager, u32) -> bool> DynamicRadialLiveCallbacks for F {
    fn before_native_actor_mutation(&mut self, manager: &EntityManager, id: u32) -> bool {
        self(manager, id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicRadialDeathPublication {
    NativeGroundDeferred(crate::native_ground_actor::NativeGroundDeferredDeathReceipt),
    Intro2Class12(crate::intro2_common_dying::Intro2CommonDyingOwner),
    Intro2Type8(crate::intro2_type8::Intro2Type8Owner),
    NativeType123(crate::native_type123::Type123Owner),
    NativeType86(crate::native_type86::Type86Owner),
    Intro2Type66(crate::intro2_type66::Intro2Type66Owner),
    MainBase(crate::main_base_runtime::MainBaseOwner),
    Intro2Type10Tumble(crate::intro2_type10::death::Intro2Type10TumbleOwner),
    Intro2Type57Tumble(crate::intro2_type57::death::Intro2Type57TumbleOwner),
    Type47Class12(FreshLevelOneType47CommonDyingOwner),
    Type9Class14(MainBaseType9ExplodingTaskLease),
    Intro2Type9Class14(MainBaseType9ExplodingTaskLease),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicRadialLivePhase {
    Eligibility,
    MutationCustody,
    Impulse,
    Filter,
    Modifier,
    Buffer,
    Dying,
    HitSound,
    TypeHit,
    Health,
    Death,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DynamicRadialLiveBlockReason {
    NativeActorMutationCustody,
    Checked(Box<LiveActorDamageError<DynamicRadialLiveBlockReason, DynamicRadialDeathPublication>>),
    Intro2Class12(crate::intro2_common_dying::Intro2CommonDyingBlock),
    Type17Capture(crate::intro2_type17::capture::CaptureBlock),
    Intro2Type8(crate::intro2_type8::Intro2Type8Block),
    NativeType123(crate::native_type123::Type123Block),
    NativeType86(crate::native_type86::Type86Block),
    Intro2Type66(crate::intro2_type66::death::Intro2Type66DeathBlock),
    MainBase(crate::main_base_runtime::MainBaseDeathBlock),
    Intro2Type10(crate::intro2_type10::death::Intro2Type10DeathBlock),
    Intro2Type57(crate::intro2_type57::death::Intro2Type57DeathBlock),
    Class49(Box<crate::class49_terminal::Class49TerminalBlock>),
    Runtime(DynamicRadialUnresolvedReason),
    Type47Death(Type47CommonDyingPublicationError),
    Type9Death(OrdinaryType9StandardDeathBlock),
    Fish(crate::shared_fish::death::SharedFishDeathBlock),
    NativeType56(crate::native_type56::death::Type56DeathBlock),
    NativeType40(crate::native_type40::death::Type40DeathBlock),
    UnsupportedDeath {
        entity_type: u32,
        alternate_class: Option<u32>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicRadialLiveBlock {
    pub target_id: u32,
    pub phase: DynamicRadialLivePhase,
    /// Includes mutation or audio already committed for the blocked target.
    pub target_prefix_committed: bool,
    pub reason: DynamicRadialLiveBlockReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DynamicRadialLiveOutcome {
    pub accepted_targets: usize,
    pub completed_target_ids: Vec<u32>,
    /// These tasks are published synchronously, before the next radial target.
    /// The outer scheduler adopts the receipts before another actor visit.
    pub death_publications: Vec<DynamicRadialDeathPublication>,
    pub blocked: Option<DynamicRadialLiveBlock>,
}

impl DynamicRadialLiveOutcome {
    pub const fn completed(&self) -> bool {
        self.blocked.is_none()
    }
}

impl EntityManager {
    /// Visit one native callback-bearing actor inside Playing's source-order
    /// radial coordinator. The coordinator owns the player/hull route, so a
    /// player allocation elsewhere in this manager does not reject this actor.
    pub(crate) fn apply_native_playing_radial_target(
        &mut self,
        id: u32,
        request: &mut DynamicRadialLiveRequest<'_>,
        result: &mut DynamicRadialLiveOutcome,
    ) -> Result<bool, DynamicRadialLiveBlock> {
        apply_live_target(self, id, request, result)
    }

    pub fn apply_dynamic_radial_damage_live(
        &mut self,
        mut request: DynamicRadialLiveRequest<'_>,
    ) -> DynamicRadialLiveOutcome {
        let mut result = DynamicRadialLiveOutcome::default();
        if let Some(target_id) = self.player_id {
            result.blocked = Some(DynamicRadialLiveBlock {
                target_id,
                phase: DynamicRadialLivePhase::Eligibility,
                target_prefix_committed: false,
                reason: DynamicRadialLiveBlockReason::Runtime(
                    DynamicRadialUnresolvedReason::PlayerHullUnavailable,
                ),
            });
            return result;
        }
        // 14AE0 reads the current node's next link after its callback. Class49
        // retains the source for deferred removal and may append a Type60;
        // that new tail allocation belongs to this same radial traversal.
        let mut current = self.retail_live_order_ids().next();
        while let Some(id) = current {
            match apply_live_target(self, id, &mut request, &mut result) {
                Ok(true) => result.completed_target_ids.push(id),
                Ok(false) => {}
                Err(block) => {
                    result.blocked = Some(block);
                    break;
                }
            }
            current = self
                .retail_live_order_ids()
                .skip_while(|candidate| *candidate != id)
                .nth(1);
        }
        result
    }
}

fn apply_live_target(
    manager: &mut EntityManager,
    id: u32,
    request: &mut DynamicRadialLiveRequest<'_>,
    result: &mut DynamicRadialLiveOutcome,
) -> Result<bool, DynamicRadialLiveBlock> {
    use DynamicRadialLivePhase as Phase;
    let mut committed = false;
    let block = |phase, committed, reason| DynamicRadialLiveBlock {
        target_id: id,
        phase,
        target_prefix_committed: committed,
        reason: DynamicRadialLiveBlockReason::Runtime(reason),
    };
    let runtime_error =
        |phase, committed, error: DynamicRadialUnresolved| block(phase, committed, error.reason);
    let entity = manager
        .entity_mut(id)
        .expect("retained live radial identity");
    let position = entity.position_raw();
    // Both reads are passive; an outer miss does not require unrelated state.
    if radial_distance_raw(request.origin_raw, position)
        >= i32::from(request.template.outer_radius_raw)
    {
        return Ok(false);
    }
    let enabled = required(
        entity
            .collision
            .state_flags_at_0x08
            .masked(CHECKED_DAMAGE_ENABLED_STATE_BIT),
        id,
        DynamicRadialRuntimeField::EligibilityState,
    )
    .map_err(|e| runtime_error(Phase::Eligibility, false, e))?;
    if enabled == 0 {
        return Ok(false);
    }
    let alternate = required(
        entity
            .collision
            .state_flags_at_0x08
            .masked(RADIAL_DAMAGE_ALTERNATE_ELIGIBLE_STATE_BIT),
        id,
        DynamicRadialRuntimeField::EligibilityState,
    )
    .map_err(|e| runtime_error(Phase::Eligibility, false, e))?;
    if alternate == 0
        && required(
            entity
                .collision
                .state_flags_at_0x08
                .masked(PAIR_COLLISION_INELIGIBLE_STATE_BIT),
            id,
            DynamicRadialRuntimeField::EligibilityState,
        )
        .map_err(|e| runtime_error(Phase::Eligibility, false, e))?
            != 0
    {
        return Ok(false);
    }
    let impulse_requested = entity.mass_raw >= 2
        && required(
            entity
                .collision
                .state_flags_at_0x08
                .masked(PAIR_COLLISION_FIXED_STATE_BIT),
            id,
            DynamicRadialRuntimeField::FixedState,
        )
        .map_err(|e| runtime_error(Phase::Impulse, false, e))?
            == 0;
    let Some(scaled) = scale_radial_damage(
        request.template,
        request.origin_raw,
        position,
        impulse_requested,
    ) else {
        return Ok(false);
    };
    result.accepted_targets += 1;
    let kind = entity.entity_type;
    let impulse = (kind != IMPULSE_SUPPRESSED_ENTITY_TYPE)
        .then_some(scaled.impulse_vector_raw)
        .flatten();
    if impulse.is_some() {
        require_local_ownership(id, &entity.collision)
            .map_err(|e| runtime_error(Phase::Impulse, false, e))?;
    }
    if !crate::native_checked_damage::prepare_native_actor_damage_mutation(
        manager,
        id,
        crate::native_checked_damage::NativeMutationCaller::Radial,
        request.callbacks,
    ) {
        return Err(DynamicRadialLiveBlock {
            target_id: id,
            phase: Phase::MutationCustody,
            target_prefix_committed: false,
            reason: DynamicRadialLiveBlockReason::NativeActorMutationCustody,
        });
    }
    // The transfer above consumes only a completed observation. It does not
    // tick or reset the current graph. Keep that authority if a later filter
    // or callback blocks: any committed impulse must not restore stale pose.
    if let Some(impulse) = impulse {
        let entity = manager.entity_mut(id).expect("retained radial target");
        let velocity = entity.velocity_raw();
        entity.velocity = super::super::raw_position_world(std::array::from_fn(|i| {
            velocity[i].wrapping_add(impulse[i])
        }));
        committed = true;
    }
    let checked = crate::native_checked_damage::apply_native_actor_checked_damage(
        manager,
        LiveActorDamageRequest {
            ratio_numerator: 0,
            ratio_denominator: 0,
            feedback: None,
            entity_id: id,
            delivery: crate::damage::DamageDeliveryRecord {
                packet: scaled.packet,
                source_entity_type_raw: scaled.trailing_raw[0] as u32,
                owner_handle: scaled.trailing_raw[1] as u32,
            },
            entry: if scaled.checked_damage {
                LiveActorDamageEntry::Checked
            } else {
                LiveActorDamageEntry::Unchecked
            },
        },
        crate::native_checked_damage::NativeCheckedDamageContext {
            world_fx: request.world_fx,
            retail_tick: request.retail_tick,
            notifications: request.notifications,
            callbacks: request.callbacks,
        },
    )
    .map_err(|error| {
        if let Some(publication) = error.death_publication {
            result.death_publications.push(publication);
        }
        let phase = match error.phase {
            LiveActorDamagePhase::Admission => Phase::Eligibility,
            LiveActorDamagePhase::Filter => Phase::Filter,
            LiveActorDamagePhase::Modifier => Phase::Modifier,
            LiveActorDamagePhase::RemoteOwner | LiveActorDamagePhase::Buffer => Phase::Buffer,
            LiveActorDamagePhase::Dying => Phase::Dying,
            LiveActorDamagePhase::HitSound => Phase::HitSound,
            LiveActorDamagePhase::HitCallback => Phase::TypeHit,
            LiveActorDamagePhase::Health => Phase::Health,
            LiveActorDamagePhase::Death | LiveActorDamagePhase::PlayerFeedback => Phase::Death,
        };
        DynamicRadialLiveBlock {
            target_id: id,
            phase,
            target_prefix_committed: committed || error.committed_prefix,
            reason: DynamicRadialLiveBlockReason::Checked(Box::new(error)),
        }
    })?;
    if let Some(publication) = checked.death_publication {
        result.death_publications.push(publication);
    }
    Ok(true)
}
