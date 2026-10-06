//! Detached runtime contract for class-4 `"Defecate Virus"`.
//!
//! Behavior initializer `FUN_0040B9E0` installs two concurrent tasks:
//! `FUN_00402850` emits class-5 infection carriers (or writes terrain directly
//! in coarse updates), while the 2,000-ms `FUN_00402BA0` companion retargets
//! and delegates movement to `FUN_00401430`. This module recovers those bounded
//! callbacks without attaching them to live entities, implementing common
//! mover internals, or dispatching an owner transition.

use crate::actor_task_owner::{ActorTaskId, ActorTaskSlot, ActorTaskVisit};
use crate::component_update::ComponentUpdateMode;
use crate::hover::q31_mul;
use crate::infection_evolution::{InfectionCellWrite, INFECTION_TERRAIN_TYPE_BIT};
use crate::shared_retarget_mover::{
    map_shared_retarget_common_mover_return, stage_shared_retarget_callback,
    SharedRetargetCallbackResult, SharedRetargetCallbackStage,
};
pub use crate::shared_retarget_mover::{
    SharedRetarget as DefecateVirusWanderRetarget,
    SharedRetargetTrigger as DefecateVirusWanderRetargetTrigger,
    SHARED_RETARGET_COMMON_MOVER_ADDRESS as DEFECATE_VIRUS_COMMON_MOVER_ADDRESS,
    SHARED_RETARGET_GATE_MASK as DEFECATE_VIRUS_RETARGET_GATE_MASK,
    SHARED_RETARGET_NEAR_TARGET_DISTANCE_RAW as DEFECATE_VIRUS_NEAR_TARGET_DISTANCE_RAW,
    SHARED_RETARGET_OWNER_TRANSITION_SINGLETON_ADDRESS as DEFECATE_VIRUS_OWNER_TRANSITION_SINGLETON_ADDRESS,
    SHARED_RETARGET_OWNER_TRANSITION_TAG as DEFECATE_VIRUS_OWNER_TRANSITION_TAG,
    SHARED_RETARGET_RADIUS_RAW as DEFECATE_VIRUS_RETARGET_RADIUS_RAW,
    SHARED_RETARGET_TASK_TICK_ADDRESS as DEFECATE_VIRUS_WANDER_TICK_ADDRESS,
};
use crate::wander_near_location::{
    WanderNearCommonMoverReturn, WanderNearPrivateState, WanderNearTaggedResult,
};

/// Class-4 behavior descriptor.
pub const DEFECATE_VIRUS_BEHAVIOR_DESCRIPTOR_ADDRESS: u32 = 0x004C_88C8;
/// Behavior prototype selected by the class-4 table entry.
pub const DEFECATE_VIRUS_PROTOTYPE_ADDRESS: u32 = 0x004C_7E88;
/// Initializer which installs the emitter and wander companion.
pub const DEFECATE_VIRUS_INITIALIZER_ADDRESS: u32 = 0x0040_B9E0;
/// Detailed/coarse emitter callback.
pub const DEFECATE_VIRUS_EMITTER_TICK_ADDRESS: u32 = 0x0040_2850;

/// Packed high-word mode consumed by the shared terrain-contact callback.
///
/// Retail uses the same `FUN_00402850` callback for both named behaviors:
/// class 5 sets terrain infection for `"Defecate Virus"`, while class 6
/// clears it for `"Cleansing Landscape"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum TerrainContactMode {
    Infect = 5,
    Cleanse = 6,
}

impl TerrainContactMode {
    pub const fn particle_class(self) -> u8 {
        self as u8
    }

    pub const fn infected(self) -> bool {
        matches!(self, Self::Infect)
    }

    pub const fn packed(self) -> u32 {
        (self as u32) << 16
    }
}

/// Entity-state bit which suppresses both detailed and coarse work.
pub const DEFECATE_VIRUS_SUPPRESSION_STATE_BIT: u32 = 0x0000_1000;
/// State bits selecting one of the four live model slots.
pub const DEFECATE_VIRUS_MODEL_SLOT_STATE_BITS: u32 = 0x0000_6000;
const DEFECATE_VIRUS_MODEL_SLOT_PLUS_TWO_BIT: u32 = 0x0000_2000;
const DEFECATE_VIRUS_MODEL_SLOT_PLUS_ONE_BIT: u32 = 0x0000_4000;
/// Authored particle class submitted by the detailed callback.
pub const DEFECATE_VIRUS_PARTICLE_CLASS: u8 = TerrainContactMode::Infect.particle_class();
/// Authored particle scale in signed-8.8-compatible render units.
pub const DEFECATE_VIRUS_PARTICLE_SCALE_RAW: u32 = 0x0800;
/// The packed low payload is zero, so this callback requests no sound.
pub const DEFECATE_VIRUS_POSITIONAL_SOUND_GLOBAL_ID: u16 = 0;
/// Terrain-type bit set by coarse writes and detailed carrier impacts.
pub const DEFECATE_VIRUS_TERRAIN_TYPE_BIT: u8 = INFECTION_TERRAIN_TYPE_BIT;

/// Inputs already resolved from the live actor and its selected model records.
///
/// `forward_q31` is entity offsets `+0x24/+0x28/+0x2C`: the Q31 forward
/// model-basis column. It is deliberately not named velocity; retail stores
/// actual signed-16 velocity elsewhere at `+0x9C/+0x9E/+0xA0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefecateVirusCallbackRequest {
    pub update_mode: ComponentUpdateMode,
    pub elapsed_micros: u32,
    pub entity_state_flags: u32,
    pub position_raw: [i16; 3],
    pub forward_q31: [i32; 3],
    /// Unsigned model-record `+0x08` extents for slots
    /// `+0xA8/+0xAA/+0xAC/+0xAE`, in that order.
    pub model_extent_raw_by_state: [Option<u16>; 4],
    pub owner_entity_handle: u32,
}

/// Exact `FUN_00440DC0` request owned by a passing detailed callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefecateVirusParticleEmission {
    position_raw: [i16; 3],
    owner_entity_handle: u32,
    terrain_contact_mode: TerrainContactMode,
    /// Retail copies `entity_state_flags >> 31` into the request. The particle
    /// allocator consumes that owner-sign as its suppress-impact flag.
    suppress_impact_damage: bool,
}

impl DefecateVirusParticleEmission {
    pub(crate) const fn new(
        position_raw: [i16; 3],
        owner_entity_handle: u32,
        suppress_impact_damage: bool,
        terrain_contact_mode: TerrainContactMode,
    ) -> Self {
        Self {
            position_raw,
            owner_entity_handle,
            terrain_contact_mode,
            suppress_impact_damage,
        }
    }

    pub const fn position_raw(self) -> [i16; 3] {
        self.position_raw
    }

    pub const fn particle_class(self) -> u8 {
        self.terrain_contact_mode.particle_class()
    }

    pub const fn terrain_contact_mode(self) -> TerrainContactMode {
        self.terrain_contact_mode
    }

    pub const fn scale_raw(self) -> u32 {
        DEFECATE_VIRUS_PARTICLE_SCALE_RAW
    }

    pub const fn owner_entity_handle(self) -> u32 {
        self.owner_entity_handle
    }

    pub const fn suppress_impact_damage(self) -> bool {
        self.suppress_impact_damage
    }

    pub const fn positional_sound_global_id(self) -> u16 {
        DEFECATE_VIRUS_POSITIONAL_SOUND_GLOBAL_ID
    }
}

/// Bounded result of one `FUN_00402850` callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefecateVirusCallbackPlan {
    /// State bit `0x1000` suppresses all work and consumes no RNG.
    SuppressedByEntityState,
    /// Detailed callback consumed its one gate word but did not pass.
    DetailedChanceRejected,
    /// The chance gate passed, but the selected model extent is unresolved.
    ///
    /// No approximation is substituted because the extent controls the exact
    /// carrier origin.
    DetailedModelExtentUnavailable {
        model_slot_index: u8,
    },
    DetailedParticle(DefecateVirusParticleEmission),
    /// Coarse mode writes one wrapped terrain cell without a particle.
    CoarseTerrainMutation(InfectionCellWrite),
}

/// Plan one exact shared terrain-contact callback.
///
/// RNG ownership is part of the contract:
///
/// - suppression consumes zero words;
/// - detailed mode consumes exactly one gate word; and
/// - coarse mode consumes Z first, then X, with no chance gate.
pub fn plan_terrain_contact_callback(
    request: DefecateVirusCallbackRequest,
    terrain_contact_mode: TerrainContactMode,
    mut next_random_u16: impl FnMut() -> u16,
) -> DefecateVirusCallbackPlan {
    if request.entity_state_flags & DEFECATE_VIRUS_SUPPRESSION_STATE_BIT != 0 {
        return DefecateVirusCallbackPlan::SuppressedByEntityState;
    }

    match request.update_mode {
        ComponentUpdateMode::Detailed => {
            let gate_word = next_random_u16();
            if u32::from(gate_word) >= request.elapsed_micros >> 2 {
                return DefecateVirusCallbackPlan::DetailedChanceRejected;
            }

            let model_slot_index = selected_model_slot_index(request.entity_state_flags) as usize;
            let Some(extent_raw) = request.model_extent_raw_by_state[model_slot_index] else {
                return DefecateVirusCallbackPlan::DetailedModelExtentUnavailable {
                    model_slot_index: model_slot_index as u8,
                };
            };
            let extent_raw = i32::from(extent_raw);
            let mut position_raw = core::array::from_fn(|axis| {
                request.position_raw[axis]
                    .wrapping_sub(q31_mul(request.forward_q31[axis], extent_raw) as i16)
            });
            position_raw[1] = position_raw[1].wrapping_add(100);

            DefecateVirusCallbackPlan::DetailedParticle(DefecateVirusParticleEmission::new(
                position_raw,
                request.owner_entity_handle,
                request.entity_state_flags >> 31 != 0,
                terrain_contact_mode,
            ))
        }
        ComponentUpdateMode::Coarse => {
            let z_raw = randomized_coarse_axis(request.position_raw[2], next_random_u16());
            let x_raw = randomized_coarse_axis(request.position_raw[0], next_random_u16());
            DefecateVirusCallbackPlan::CoarseTerrainMutation(InfectionCellWrite {
                cell: [raw_cell_coordinate(x_raw), raw_cell_coordinate(z_raw)],
                infected: terrain_contact_mode.infected(),
            })
        }
    }
}

/// Plan one exact class-4 `"Defecate Virus"` callback.
pub fn plan_defecate_virus_callback(
    request: DefecateVirusCallbackRequest,
    next_random_u16: impl FnMut() -> u16,
) -> DefecateVirusCallbackPlan {
    plan_terrain_contact_callback(request, TerrainContactMode::Infect, next_random_u16)
}

const fn selected_model_slot_index(entity_state_flags: u32) -> u8 {
    (((entity_state_flags & DEFECATE_VIRUS_MODEL_SLOT_PLUS_TWO_BIT) >> 12)
        | ((entity_state_flags & DEFECATE_VIRUS_MODEL_SLOT_PLUS_ONE_BIT) >> 14)) as u8
}

fn randomized_coarse_axis(actor_axis_raw: i16, random_word: u16) -> i16 {
    let offset_raw = (random_word >> 7).wrapping_sub(0x0100) as i16;
    actor_axis_raw.wrapping_add(offset_raw)
}

const fn raw_cell_coordinate(position_raw: i16) -> u8 {
    ((position_raw as u16) >> 8) as u8
}

/// Generic-owner state for the concurrent slot-2 terrain-infection task.
///
/// `FUN_00401120` owns this elapsed accumulator independently of
/// `FUN_00402850`. Zero disables the authored timeout; every nonzero lifetime
/// expires only after the strict `lifetime < elapsed` test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefecateVirusTerrainTaskState {
    lifetime_ms: u32,
    elapsed_ms: u32,
}

impl DefecateVirusTerrainTaskState {
    pub const fn new(lifetime_ms: u16) -> Self {
        Self {
            lifetime_ms: lifetime_ms as u32,
            elapsed_ms: 0,
        }
    }

    pub const fn from_parts(lifetime_ms: u32, elapsed_ms: u32) -> Self {
        Self {
            lifetime_ms,
            elapsed_ms,
        }
    }

    pub const fn lifetime_ms(self) -> u32 {
        self.lifetime_ms
    }

    pub const fn elapsed_ms(self) -> u32 {
        self.elapsed_ms
    }

    /// Run generic-owner phase 1 before entering `FUN_00402850`.
    pub fn before_callback(&mut self, elapsed_micros: u32) -> DefecateVirusTerrainCallbackPrefix {
        self.elapsed_ms = self.elapsed_ms.wrapping_add(elapsed_micros / 1_000);
        DefecateVirusTerrainCallbackPrefix {
            elapsed_ms: self.elapsed_ms,
            lifetime_status: if self.lifetime_ms != 0 && self.lifetime_ms < self.elapsed_ms {
                DefecateVirusTerrainLifetimeStatus::OwnerTransitionDue
            } else {
                DefecateVirusTerrainLifetimeStatus::WithinLifetime
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefecateVirusTerrainLifetimeStatus {
    WithinLifetime,
    OwnerTransitionDue,
}

/// Scheduler-owned state committed before the terrain callback begins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefecateVirusTerrainCallbackPrefix {
    pub elapsed_ms: u32,
    pub lifetime_status: DefecateVirusTerrainLifetimeStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefecateVirusTerrainTransitionRequest {
    pub slot: ActorTaskSlot,
    pub task_id: ActorTaskId,
    pub committed_prefix: DefecateVirusTerrainCallbackPrefix,
}

/// Result of resolving the class-4 behavior owner's primary transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefecateVirusTransitionOutcome<R> {
    /// The owner root has no primary callback. A tagged result reaches the
    /// strict-timeout branch, which immediately finds that same pointer absent.
    CallbackAbsent,
    /// `FUN_00416410` rejected the transition. Tagged `0x9C01` returns
    /// immediately and does not fall through to timeout.
    SuppressedByEntityState,
    /// The callback ran. Only this outcome may carry transition-owned
    /// mutations or an owner result.
    Completed(Option<R>),
}

/// Run generic-owner phase 3 after the terrain callback survives and unwinds.
///
/// `FUN_00402850` always returns null, so this task has no tagged-result path;
/// only the generic strict timeout can request the behavior-owner callback.
pub const fn defecate_virus_terrain_after_unwind(
    visit: ActorTaskVisit,
    committed_prefix: DefecateVirusTerrainCallbackPrefix,
) -> Option<DefecateVirusTerrainTransitionRequest> {
    match committed_prefix.lifetime_status {
        DefecateVirusTerrainLifetimeStatus::WithinLifetime => None,
        DefecateVirusTerrainLifetimeStatus::OwnerTransitionDue => {
            Some(DefecateVirusTerrainTransitionRequest {
                slot: visit.slot,
                task_id: visit.task_id,
                committed_prefix,
            })
        }
    }
}

/// Retail lifetime of the concurrent slot-0 companion.
pub const DEFECATE_VIRUS_WANDER_LIFETIME_MS: u32 = 2_000;

/// Detached slot-0 state needed by the recovered scheduler/callback prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefecateVirusWanderTaskState {
    private_state: WanderNearPrivateState,
    elapsed_ms: u32,
}

impl DefecateVirusWanderTaskState {
    /// Construct the static-target state initialized by the shared task helper.
    pub const fn new(current_position_raw: [i16; 3]) -> Self {
        Self {
            private_state: WanderNearPrivateState::ordinary_type9(current_position_raw),
            elapsed_ms: 0,
        }
    }

    pub const fn from_parts(private_state: WanderNearPrivateState, elapsed_ms: u32) -> Self {
        Self {
            private_state,
            elapsed_ms,
        }
    }

    pub const fn private_state(self) -> WanderNearPrivateState {
        self.private_state
    }

    pub const fn elapsed_ms(self) -> u32 {
        self.elapsed_ms
    }

    /// Run generic-owner phase 1 before entering `FUN_00402BA0`.
    pub fn before_callback(&mut self, elapsed_micros: u32) -> u32 {
        self.elapsed_ms = self.elapsed_ms.wrapping_add(elapsed_micros / 1_000);
        self.elapsed_ms
    }

    /// Commit the callback's RNG-owned retarget and stage common-mover writes.
    ///
    /// Retail mutates the target before calling `FUN_00401430`. The returned
    /// copy therefore starts after that committed prefix; only later mover
    /// writes wait for the same wrapper to survive and the return value to be
    /// resolved.
    pub fn stage_callback(
        &mut self,
        actor_position_raw: [i16; 3],
        next_random_u16: impl FnMut() -> u16,
    ) -> DefecateVirusWanderCallbackStage {
        let shared = stage_shared_retarget_callback(
            &mut self.private_state,
            actor_position_raw,
            next_random_u16,
        );
        DefecateVirusWanderCallbackStage { shared }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefecateVirusWanderFrameRequest {
    pub actor_position_raw: [i16; 3],
    pub elapsed_micros: u32,
}

/// Work completed before the unresolved common-mover seam.
///
/// `elapsed_ms` is scheduler-owned and already committed. `retarget` describes
/// a private-state mutation committed before retail calls the common mover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefecateVirusWanderCallbackPrefix {
    pub elapsed_ms: u32,
    pub retarget: DefecateVirusWanderRetarget,
}

/// Staged private state for the movement-owned suffix of `FUN_00402BA0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefecateVirusWanderCallbackStage {
    shared: SharedRetargetCallbackStage,
}

impl DefecateVirusWanderCallbackStage {
    pub const fn retarget(self) -> DefecateVirusWanderRetarget {
        self.shared.retarget()
    }

    pub const fn private_state(self) -> WanderNearPrivateState {
        self.shared.private_state()
    }

    pub fn private_state_mut(&mut self) -> &mut WanderNearPrivateState {
        self.shared.private_state_mut()
    }

    pub fn commit(self, surviving_state: &mut DefecateVirusWanderTaskState) {
        self.shared.commit(&mut surviving_state.private_state);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefecateVirusWanderTransitionReason {
    CommonMoverCompleted(WanderNearTaggedResult),
    LifetimeExpired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefecateVirusWanderFrameOutcome {
    Continue,
    OwnerTransition(DefecateVirusWanderTransitionReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefecateVirusWanderTransitionRequest {
    pub slot: ActorTaskSlot,
    pub task_id: ActorTaskId,
    pub reason: DefecateVirusWanderTransitionReason,
    pub committed_prefix: DefecateVirusWanderCallbackPrefix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefecateVirusWanderPostUnwind {
    Continue,
    Transition(DefecateVirusWanderTransitionRequest),
    UnresolvedCommonMover,
}

/// Resolve `FUN_00402BA0` after its surviving wrapper has unwound.
///
/// The pointer-distinct `0x9C01` result wins over the strict 2,000-ms timeout.
/// An unresolved common mover remains an explicit integration block.
pub const fn defecate_virus_wander_after_unwind(
    visit: ActorTaskVisit,
    committed_prefix: DefecateVirusWanderCallbackPrefix,
    mover_return: WanderNearCommonMoverReturn,
) -> DefecateVirusWanderPostUnwind {
    if matches!(mover_return, WanderNearCommonMoverReturn::Unresolved) {
        return DefecateVirusWanderPostUnwind::UnresolvedCommonMover;
    }
    let Some(reason) =
        defecate_virus_wander_transition_reason(committed_prefix.elapsed_ms, mover_return)
    else {
        return DefecateVirusWanderPostUnwind::Continue;
    };
    DefecateVirusWanderPostUnwind::Transition(DefecateVirusWanderTransitionRequest {
        slot: visit.slot,
        task_id: visit.task_id,
        reason,
        committed_prefix,
    })
}

/// Successfully resolved frame. The returned private state has already been
/// committed to the supplied task state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefecateVirusWanderFrameResolution {
    pub prefix: DefecateVirusWanderCallbackPrefix,
    pub committed_private_state: WanderNearPrivateState,
    pub outcome: DefecateVirusWanderFrameOutcome,
}

/// Fail-closed common-mover boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefecateVirusWanderFrameError<E> {
    CommonMover {
        prefix: DefecateVirusWanderCallbackPrefix,
        error: E,
    },
    UnresolvedCommonMover {
        prefix: DefecateVirusWanderCallbackPrefix,
    },
}

/// Run the exact scheduler prefix and the bounded `FUN_00402BA0` callback.
///
/// The external closure is the still-separate `FUN_00401430` adapter. It
/// receives a staged private record and may mutate it. The scheduler elapsed
/// prefix and the callback's pre-mover retarget are already committed exactly
/// as retail orders them; only mutations made by an adapter which errors or
/// reports [`WanderNearCommonMoverReturn::Unresolved`] are discarded.
///
/// A zero common-mover return requests the tagged owner transition before the
/// strict `elapsed_ms > 2000` timeout is considered. This function only
/// reports the transition; it never dispatches the owner callback.
pub fn tick_defecate_virus_wander<E>(
    state: &mut DefecateVirusWanderTaskState,
    request: DefecateVirusWanderFrameRequest,
    mut next_random_u16: impl FnMut() -> u16,
    common_mover: impl FnOnce(&mut WanderNearPrivateState) -> Result<WanderNearCommonMoverReturn, E>,
) -> Result<DefecateVirusWanderFrameResolution, DefecateVirusWanderFrameError<E>> {
    let elapsed_ms = state.before_callback(request.elapsed_micros);
    let mut stage = state.stage_callback(request.actor_position_raw, &mut next_random_u16);
    let prefix = DefecateVirusWanderCallbackPrefix {
        elapsed_ms,
        retarget: stage.retarget(),
    };

    let mover_return = common_mover(stage.private_state_mut())
        .map_err(|error| DefecateVirusWanderFrameError::CommonMover { prefix, error })?;
    if mover_return == WanderNearCommonMoverReturn::Unresolved {
        return Err(DefecateVirusWanderFrameError::UnresolvedCommonMover { prefix });
    }
    let outcome = match defecate_virus_wander_transition_reason(elapsed_ms, mover_return) {
        Some(reason) => DefecateVirusWanderFrameOutcome::OwnerTransition(reason),
        None => DefecateVirusWanderFrameOutcome::Continue,
    };

    let committed_private_state = stage.private_state();
    stage.commit(state);
    Ok(DefecateVirusWanderFrameResolution {
        prefix,
        committed_private_state,
        outcome,
    })
}

const fn defecate_virus_wander_transition_reason(
    elapsed_ms: u32,
    mover_return: WanderNearCommonMoverReturn,
) -> Option<DefecateVirusWanderTransitionReason> {
    match map_shared_retarget_common_mover_return(mover_return) {
        SharedRetargetCallbackResult::TaggedOwnerTransition(result) => Some(
            DefecateVirusWanderTransitionReason::CommonMoverCompleted(WanderNearTaggedResult {
                singleton_address: result.singleton_address,
                tag: result.tag,
            }),
        ),
        SharedRetargetCallbackResult::Continue
            if DEFECATE_VIRUS_WANDER_LIFETIME_MS < elapsed_ms =>
        {
            Some(DefecateVirusWanderTransitionReason::LifetimeExpired)
        }
        SharedRetargetCallbackResult::Continue
        | SharedRetargetCallbackResult::UnresolvedCommonMover => None,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    fn request(update_mode: ComponentUpdateMode) -> DefecateVirusCallbackRequest {
        DefecateVirusCallbackRequest {
            update_mode,
            elapsed_micros: 400,
            entity_state_flags: 0,
            position_raw: [30_000, -30_000, -32_700],
            forward_q31: [i32::MIN, 0x4000_0000, -0x4000_0000],
            model_extent_raw_by_state: [Some(512); 4],
            owner_entity_handle: 0x047E_0001,
        }
    }

    #[test]
    fn suppression_consumes_no_rng_in_either_update_mode() {
        for update_mode in [ComponentUpdateMode::Detailed, ComponentUpdateMode::Coarse] {
            let draws = Cell::new(0);
            let mut request = request(update_mode);
            request.entity_state_flags = DEFECATE_VIRUS_SUPPRESSION_STATE_BIT;
            let plan = plan_defecate_virus_callback(request, || {
                draws.set(draws.get() + 1);
                0
            });

            assert_eq!(plan, DefecateVirusCallbackPlan::SuppressedByEntityState);
            assert_eq!(draws.get(), 0);
        }
    }

    #[test]
    fn detailed_gate_is_strict_and_consumes_exactly_one_word() {
        let rejected_draws = Cell::new(0);
        let rejected = plan_defecate_virus_callback(request(ComponentUpdateMode::Detailed), || {
            rejected_draws.set(rejected_draws.get() + 1);
            100
        });
        assert_eq!(rejected, DefecateVirusCallbackPlan::DetailedChanceRejected);
        assert_eq!(rejected_draws.get(), 1);

        let accepted_draws = Cell::new(0);
        let accepted = plan_defecate_virus_callback(request(ComponentUpdateMode::Detailed), || {
            accepted_draws.set(accepted_draws.get() + 1);
            99
        });
        assert!(matches!(
            accepted,
            DefecateVirusCallbackPlan::DetailedParticle(_)
        ));
        assert_eq!(accepted_draws.get(), 1);
    }

    #[test]
    fn detailed_origin_uses_forward_q31_extent_and_exact_request_fields() {
        let mut request = request(ComponentUpdateMode::Detailed);
        request.entity_state_flags = 0x8000_0000;
        let plan = plan_defecate_virus_callback(request, || 0);

        assert_eq!(
            plan,
            DefecateVirusCallbackPlan::DetailedParticle(DefecateVirusParticleEmission::new(
                [30_512, -30_156, -32_444],
                0x047E_0001,
                true,
                TerrainContactMode::Infect,
            ))
        );
    }

    #[test]
    fn detailed_model_state_bits_select_all_four_extents() {
        let cases = [
            (0, 0usize, 10i16),
            (0x4000, 1, 20),
            (0x2000, 2, 30),
            (0x6000, 3, 40),
        ];
        for (flags, expected_index, expected_extent) in cases {
            let mut request = request(ComponentUpdateMode::Detailed);
            request.entity_state_flags = flags;
            request.position_raw = [100, 200, 300];
            request.forward_q31 = [i32::MIN, 0, 0];
            request.model_extent_raw_by_state = [Some(10), Some(20), Some(30), Some(40)];
            let plan = plan_defecate_virus_callback(request, || 0);
            let DefecateVirusCallbackPlan::DetailedParticle(emission) = plan else {
                panic!("slot {expected_index} did not produce a particle");
            };
            assert_eq!(
                emission.position_raw,
                [100i16.wrapping_add(expected_extent), 300, 300],
                "slot {expected_index}"
            );
        }
    }

    #[test]
    fn missing_selected_extent_fails_closed_after_the_gate_draw() {
        let draws = Cell::new(0);
        let mut request = request(ComponentUpdateMode::Detailed);
        request.entity_state_flags = 0x6000;
        request.model_extent_raw_by_state[3] = None;
        let plan = plan_defecate_virus_callback(request, || {
            draws.set(draws.get() + 1);
            0
        });

        assert_eq!(
            plan,
            DefecateVirusCallbackPlan::DetailedModelExtentUnavailable {
                model_slot_index: 3
            }
        );
        assert_eq!(draws.get(), 1);
    }

    #[test]
    fn coarse_mode_consumes_z_then_x_and_wraps_into_terrain_cell_bytes() {
        let words = [0x0000, 0xFFFF];
        let index = Cell::new(0);
        let mut request = request(ComponentUpdateMode::Coarse);
        request.position_raw = [32_760, 4, -32_760];
        let plan = plan_defecate_virus_callback(request, || {
            let current = index.get();
            index.set(current + 1);
            words[current]
        });

        assert_eq!(index.get(), 2);
        assert_eq!(
            plan,
            DefecateVirusCallbackPlan::CoarseTerrainMutation(InfectionCellWrite {
                // X consumed 0xFFFF and wraps to -32521 (high byte 0x80);
                // Z consumed 0x0000 and wraps to 32520 (high byte 0x7F).
                cell: [0x80, 0x7F],
                infected: true,
            })
        );
    }

    #[test]
    fn cleansing_mode_reuses_rng_and_geometry_but_clears_infection() {
        let detailed = plan_terrain_contact_callback(
            request(ComponentUpdateMode::Detailed),
            TerrainContactMode::Cleanse,
            || 0,
        );
        let DefecateVirusCallbackPlan::DetailedParticle(emission) = detailed else {
            panic!("cleansing detailed callback did not emit");
        };
        assert_eq!(emission.particle_class(), 6);
        assert_eq!(emission.terrain_contact_mode(), TerrainContactMode::Cleanse);

        let coarse = plan_terrain_contact_callback(
            request(ComponentUpdateMode::Coarse),
            TerrainContactMode::Cleanse,
            || 0,
        );
        assert!(matches!(
            coarse,
            DefecateVirusCallbackPlan::CoarseTerrainMutation(InfectionCellWrite {
                infected: false,
                ..
            })
        ));
    }

    fn resolved_nonzero(
        _state: &mut WanderNearPrivateState,
    ) -> Result<WanderNearCommonMoverReturn, &'static str> {
        Ok(WanderNearCommonMoverReturn::NonZero)
    }

    #[test]
    fn near_axis_retarget_consumes_x_then_z_without_a_gate_draw() {
        let words = [0x0000, 0xFFFF];
        let index = Cell::new(0);
        let actor = [1_000i16, -77, 30_000];
        let mut state = DefecateVirusWanderTaskState::from_parts(
            WanderNearPrivateState {
                target_position_raw: [1_100, 7, -20_000],
                tracked_entity_handle: 0,
                direction: 1,
                reversal_timer_ms: 0,
            },
            0,
        );
        let result = tick_defecate_virus_wander(
            &mut state,
            DefecateVirusWanderFrameRequest {
                actor_position_raw: actor,
                elapsed_micros: 16_666,
            },
            || {
                let current = index.get();
                index.set(current + 1);
                words[current]
            },
            resolved_nonzero,
        )
        .unwrap();
        let expected = [
            actor[0].wrapping_add(-4_096),
            actor[1],
            actor[2].wrapping_add(4_095),
        ];

        assert_eq!(index.get(), 2);
        assert_eq!(
            result.prefix.retarget,
            DefecateVirusWanderRetarget::Replaced {
                trigger: DefecateVirusWanderRetargetTrigger::NearTargetAxis,
                target_position_raw: expected,
            }
        );
        assert_eq!(state.private_state().target_position_raw, expected);
    }

    #[test]
    fn far_axes_failed_gate_consumes_one_word_and_retains_target() {
        let draws = Cell::new(0);
        let private_state = WanderNearPrivateState {
            target_position_raw: [0x1000, 2, 0x1000],
            tracked_entity_handle: 0,
            direction: 1,
            reversal_timer_ms: 0,
        };
        let mut state = DefecateVirusWanderTaskState::from_parts(private_state, 0);
        let result = tick_defecate_virus_wander(
            &mut state,
            DefecateVirusWanderFrameRequest {
                actor_position_raw: [0, 8, 0],
                elapsed_micros: 1_000,
            },
            || {
                draws.set(draws.get() + 1);
                0x003F
            },
            resolved_nonzero,
        )
        .unwrap();

        assert_eq!(draws.get(), 1);
        assert_eq!(
            result.prefix.retarget,
            DefecateVirusWanderRetarget::Retained
        );
        assert_eq!(state.private_state(), private_state);
    }

    #[test]
    fn far_axes_passing_gate_consumes_gate_then_x_then_z() {
        let words = [0xFFC0, 0x0008, 0xFFF8];
        let index = Cell::new(0);
        let actor = [32_767i16, -5, -32_768];
        let mut state = DefecateVirusWanderTaskState::from_parts(
            WanderNearPrivateState {
                target_position_raw: [0, 0, 0],
                tracked_entity_handle: 0,
                direction: 1,
                reversal_timer_ms: 0,
            },
            0,
        );
        let result = tick_defecate_virus_wander(
            &mut state,
            DefecateVirusWanderFrameRequest {
                actor_position_raw: actor,
                elapsed_micros: 1_000,
            },
            || {
                let current = index.get();
                index.set(current + 1);
                words[current]
            },
            resolved_nonzero,
        )
        .unwrap();
        let expected = [
            actor[0].wrapping_add(-4_095),
            actor[1],
            actor[2].wrapping_add(4_095),
        ];

        assert_eq!(index.get(), 3);
        assert_eq!(
            result.prefix.retarget,
            DefecateVirusWanderRetarget::Replaced {
                trigger: DefecateVirusWanderRetargetTrigger::RandomGate,
                target_position_raw: expected,
            }
        );
    }

    #[test]
    fn mover_error_keeps_retarget_but_discards_staged_mover_mutations() {
        let original = WanderNearPrivateState::ordinary_type9([10, 20, 30]);
        let mut state = DefecateVirusWanderTaskState::from_parts(original, 7);
        let error = tick_defecate_virus_wander(
            &mut state,
            DefecateVirusWanderFrameRequest {
                actor_position_raw: [10, 20, 30],
                elapsed_micros: 1_999,
            },
            || 0,
            |staged| {
                staged.tracked_entity_handle = 99;
                Err("blocked")
            },
        )
        .unwrap_err();

        assert!(matches!(
            error,
            DefecateVirusWanderFrameError::CommonMover {
                prefix: DefecateVirusWanderCallbackPrefix { elapsed_ms: 8, .. },
                error: "blocked",
            }
        ));
        assert_eq!(state.elapsed_ms(), 8);
        assert_eq!(
            state.private_state().target_position_raw,
            [10i16.wrapping_sub(4_096), 20, 30i16.wrapping_sub(4_096)]
        );
        assert_eq!(
            state.private_state().tracked_entity_handle,
            original.tracked_entity_handle
        );
    }

    #[test]
    fn unresolved_mover_keeps_retarget_but_discards_staged_mover_mutations() {
        let original = WanderNearPrivateState::ordinary_type9([10, 20, 30]);
        let mut state = DefecateVirusWanderTaskState::from_parts(original, 0);
        let error = tick_defecate_virus_wander(
            &mut state,
            DefecateVirusWanderFrameRequest {
                actor_position_raw: [10, 20, 30],
                elapsed_micros: 1_000,
            },
            || 0,
            |staged| {
                staged.direction = -1;
                Ok::<_, &'static str>(WanderNearCommonMoverReturn::Unresolved)
            },
        )
        .unwrap_err();

        assert!(matches!(
            error,
            DefecateVirusWanderFrameError::UnresolvedCommonMover { .. }
        ));
        assert_eq!(
            state.private_state().target_position_raw,
            [10i16.wrapping_sub(4_096), 20, 30i16.wrapping_sub(4_096)]
        );
        assert_eq!(state.private_state().direction, original.direction);
    }

    #[test]
    fn resolved_mover_commits_its_staged_private_mutations() {
        let mut state = DefecateVirusWanderTaskState::new([0; 3]);
        let result = tick_defecate_virus_wander(
            &mut state,
            DefecateVirusWanderFrameRequest {
                actor_position_raw: [0; 3],
                elapsed_micros: 1_000,
            },
            || 0,
            |staged| {
                staged.direction = -1;
                Ok::<_, &'static str>(WanderNearCommonMoverReturn::NonZero)
            },
        )
        .unwrap();

        assert_eq!(result.committed_private_state.direction, -1);
        assert_eq!(state.private_state().direction, -1);
        assert_eq!(result.outcome, DefecateVirusWanderFrameOutcome::Continue);
    }

    #[test]
    fn scheduler_truncates_each_frame_and_expires_strictly_after_2000_ms() {
        let mut state = DefecateVirusWanderTaskState::new([0; 3]);
        let first = tick_defecate_virus_wander(
            &mut state,
            DefecateVirusWanderFrameRequest {
                actor_position_raw: [0; 3],
                elapsed_micros: 2_000_999,
            },
            || 0,
            resolved_nonzero,
        )
        .unwrap();
        assert_eq!(first.prefix.elapsed_ms, 2_000);
        assert_eq!(first.outcome, DefecateVirusWanderFrameOutcome::Continue);

        let second = tick_defecate_virus_wander(
            &mut state,
            DefecateVirusWanderFrameRequest {
                actor_position_raw: [0; 3],
                elapsed_micros: 999,
            },
            || 0,
            resolved_nonzero,
        )
        .unwrap();
        assert_eq!(second.prefix.elapsed_ms, 2_000);
        assert_eq!(second.outcome, DefecateVirusWanderFrameOutcome::Continue);

        let third = tick_defecate_virus_wander(
            &mut state,
            DefecateVirusWanderFrameRequest {
                actor_position_raw: [0; 3],
                elapsed_micros: 1_000,
            },
            || 0,
            resolved_nonzero,
        )
        .unwrap();
        assert_eq!(third.prefix.elapsed_ms, 2_001);
        assert_eq!(
            third.outcome,
            DefecateVirusWanderFrameOutcome::OwnerTransition(
                DefecateVirusWanderTransitionReason::LifetimeExpired
            )
        );
    }

    #[test]
    fn common_mover_transition_precedes_an_already_expired_timeout() {
        let mut state = DefecateVirusWanderTaskState::new([0; 3]);
        let result = tick_defecate_virus_wander(
            &mut state,
            DefecateVirusWanderFrameRequest {
                actor_position_raw: [0; 3],
                elapsed_micros: 2_001_000,
            },
            || 0,
            |_staged| Ok::<_, &'static str>(WanderNearCommonMoverReturn::Zero),
        )
        .unwrap();

        assert_eq!(result.prefix.elapsed_ms, 2_001);
        assert_eq!(
            result.outcome,
            DefecateVirusWanderFrameOutcome::OwnerTransition(
                DefecateVirusWanderTransitionReason::CommonMoverCompleted(WanderNearTaggedResult {
                    singleton_address: 0x004B_E138,
                    tag: 0x0000_9C01,
                })
            )
        );
    }

    #[test]
    fn elapsed_milliseconds_wrap_like_the_retail_scheduler_dword() {
        let mut state = DefecateVirusWanderTaskState::from_parts(
            WanderNearPrivateState::ordinary_type9([0; 3]),
            u32::MAX,
        );
        let result = tick_defecate_virus_wander(
            &mut state,
            DefecateVirusWanderFrameRequest {
                actor_position_raw: [0; 3],
                elapsed_micros: 1_000,
            },
            || 0,
            resolved_nonzero,
        )
        .unwrap();

        assert_eq!(result.prefix.elapsed_ms, 0);
        assert_eq!(result.outcome, DefecateVirusWanderFrameOutcome::Continue);
    }
}
