//! The explicit bounded world-state transition after an atomic abort rejection.
//!
//! The strict 170A0 actor walk remains authoritative. Until every authored actor
//! family has a native lifecycle, the interactive game retains its existing
//! compatibility policy: arm known factories, transform terrain, then publish
//! controller state5 and the replacement frame. Missing callbacks remain in the
//! returned diagnostic; this policy does not pretend to execute their deaths.

use super::*;
use crate::entity::FactoryAbortProgressionStarted;
use crate::main_base_abort::{MainBaseAbortActorLease, MainBaseAbortWorldControlLease};

pub struct MainBaseAbortFallbackRequest<'a> {
    pub world_control_lease: MainBaseAbortWorldControlLease,
    pub controller: &'a mut MainBaseAbortControllerStorage,
    pub entities: &'a mut EntityManager,
    pub resources: &'a mut ResourceCache,
    pub world_fx: &'a mut WorldFx,
}

#[derive(Debug)]
pub struct MainBaseAbortFallbackReport {
    pub diagnostic: MainBaseAbortProductionDiagnostic,
    /// Only a terminal Main Base origin contributes this receipt. A casualty
    /// predicate uses the same fallback with no manufactured actor origin.
    pub terminal_actor: Option<MainBaseAbortActorLease>,
    pub factories: Vec<FactoryAbortProgressionStarted>,
    pub terrain: Option<MainBaseAbortTerrainOutcome>,
    pub frame_request: Result<MainBaseAbortFrameRequest, MainBaseAbortControllerLeaseMismatch>,
}

impl MainBaseAbortProductionFailure {
    /// Consume the failed strict transaction and finish the existing bounded
    /// world-state policy. The outer 456960 header owns repeat suppression,
    /// resource3E and the final full-frame sequence; this method owns its body.
    ///
    /// Decision matrix retained from the interactive caller:
    /// - every strict rejection has rolled back actors, tasks, RNG and terrain;
    /// - a terminal origin is optional, independent of the world-loss trigger;
    /// - eligible known factories arm once and emit their authored death cue;
    /// - terrain's null/already-applied result still permits the handoff;
    /// - a stale controller lease leaves the earlier phases committed and
    ///   reports the failed handoff, as the former inline body did.
    pub fn apply_bounded_world_fallback(
        self,
        request: MainBaseAbortFallbackRequest<'_>,
    ) -> MainBaseAbortFallbackReport {
        debug_assert!(self.progress.processed_actors.is_empty());
        debug_assert!(self.progress.world_effects.is_none());
        debug_assert!(self.progress.terrain.is_none());
        debug_assert!(self.progress.frame_request.is_none());
        debug_assert!(self.unscheduled_owner.is_none());
        let terminal_actor = self
            .terminal_origin
            .as_ref()
            .map(|origin| origin.actor_lease());
        let factories = request.entities.arm_live_factories_for_main_base_abort();
        for started in &factories {
            if let Some(sound_id) = started.death_sound_id {
                request
                    .world_fx
                    .queue_fixed_positional_sound_raw(sound_id, started.target_position_raw);
            }
        }
        let terrain = request
            .resources
            .apply_main_base_abort_terrain_transform(|| {
                request.world_fx.next_shared_retail_random_u16()
            });
        let frame_request = request
            .controller
            .commit_post_terrain_abort(request.world_control_lease);
        MainBaseAbortFallbackReport {
            diagnostic: self.diagnostic,
            terminal_actor,
            factories,
            terrain,
            frame_request,
        }
    }
}
