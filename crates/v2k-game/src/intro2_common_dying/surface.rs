//! E370's living common-actor surface path. 162B0 may replace the task graph,
//! but its caller retains the entry model extent and completes the world tail.

use super::*;
use crate::common_mover::type9_surface::*;
use crate::entity_behavior::ReleaseCallbackPolicy;
use crate::entity_relation_release::relation_release_state_word_after;
use crate::world_fx::ParticleEnvironment;
use v2k_formats::terrain::TerrainGrid;

pub(crate) struct Intro2ActorSurfaceFrame<'a> {
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub terrain: &'a TerrainGrid,
    /// Model header +08 cached by E370 before 162B0 invokes lifecycle callbacks.
    pub active_model_extent_raw: u16,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
    pub particle_environment: ParticleEnvironment<'a>,
}

/// Used by the authenticated Type26/53 living world owners. A committed death
/// receipt is delivered independently from a later error, so the scheduler can
/// retain the new class12 graph without replaying the living callback prefix.
pub(crate) fn run_living_actor_surface(
    manager: &mut EntityManager,
    entity_id: u32,
    frame: Intro2ActorSurfaceFrame<'_>,
    world_fx: &mut WorldFx,
    death: &mut Option<Intro2CommonDyingOwner>,
) -> Result<(), Intro2CommonDyingBlock> {
    if !native_manager_allocation(manager, entity_id) {
        return Err(Intro2CommonDyingBlock::UnauthenticatedAllocation);
    }
    run_actor_surface_with_death(
        manager,
        entity_id,
        frame,
        world_fx,
        death,
        native_allocation,
        publish_intro2_common_standard_death,
    )
}

/// The shared E370 phase with an explicit allocation/death policy. Existing
/// common12 callers retain their original receipt and publisher; native Type8
/// supplies its independent allocation and class14 publication instead.
/// Neither policy changes timer -> release -> death -> bubble -> sound order.
pub(crate) fn run_actor_surface_with_death<D>(
    manager: &mut EntityManager,
    entity_id: u32,
    frame: Intro2ActorSurfaceFrame<'_>,
    world_fx: &mut WorldFx,
    death: &mut Option<D>,
    authenticates: impl FnOnce(&Entity) -> bool,
    publish_death: impl FnOnce(
        &mut EntityManager,
        u32,
        &mut WorldFx,
    ) -> Result<Option<D>, Intro2CommonDyingBlock>,
) -> Result<(), Intro2CommonDyingBlock> {
    let metadata = frame.metadata;
    run_actor_surface_with_lifecycle(
        manager,
        entity_id,
        frame,
        world_fx,
        death,
        authenticates,
        |manager, entity_id, world_fx| {
            use Intro2CommonDyingBlock as Block;
            let entity = manager
                .entity_mut(entity_id)
                .ok_or(Block::AllocationUnavailable)?;
            let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
                return Err(Block::Runtime("surface release context"));
            };
            let release = context.active_style().release_callback_policy();
            if release != ReleaseCallbackPolicy::None {
                return Err(Block::UnsupportedReleaseHook(release));
            }
            let default = metadata
                .initializer
                .as_ref()
                .ok_or(Block::Metadata("surface release initializer"))?
                .initializer_state_flags_raw;
            // Existing callers own a null style release. Keep their exact
            // timer -> 16750/D3C0 -> 10C10 decision matrix in this adapter.
            entity.collision.state_flags_at_0x08 =
                relation_release_state_word_after(entity.collision.state_flags_at_0x08, default);
            entity.collision.default_state_flags_at_0xc8 = RetailRuntimeValue::Known(default);
            entity.attached_to = None;
            publish_death(manager, entity_id, world_fx)
        },
    )
}

/// E370 with a caller-owned complete 162B0 lifecycle pair. The timer write
/// precedes this boundary; the supplied policy must perform 16750, dispose
/// its callback result, then 10C10 before fresh bubble/sound inputs are read.
/// Carrying Type9 uses its real CE90 release/reselection here.
pub(crate) fn run_actor_surface_with_lifecycle<D>(
    manager: &mut EntityManager,
    entity_id: u32,
    frame: Intro2ActorSurfaceFrame<'_>,
    world_fx: &mut WorldFx,
    death: &mut Option<D>,
    authenticates: impl FnOnce(&Entity) -> bool,
    run_lifecycle: impl FnOnce(
        &mut EntityManager,
        u32,
        &mut WorldFx,
    ) -> Result<Option<D>, Intro2CommonDyingBlock>,
) -> Result<(), Intro2CommonDyingBlock> {
    use Intro2CommonDyingBlock as Block;
    let entity = manager
        .entity_mut(entity_id)
        .ok_or(Block::AllocationUnavailable)?;
    if !authenticates(entity) {
        return Err(Block::UnauthenticatedAllocation);
    }
    let state = match entity
        .collision
        .state_flags_at_0x08
        .masked(ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT)
    {
        RetailRuntimeValue::Known(value) => value,
        _ => return Err(Block::Runtime("surface disabled bit")),
    };
    if state != 0 {
        return Ok(());
    }
    let RetailRuntimeValue::Known(timer) = entity.surface_lifetime_timer_ms_at_0x48 else {
        return Err(Block::Runtime("surface timer"));
    };
    let RetailRuntimeValue::Known(effects) = frame.metadata.common_world_effects else {
        return Err(Block::Metadata("surface effects"));
    };
    if effects.surface_selectors == [0, 0] {
        // 1E370 does not call 162B0 when both +72/+73 selectors are absent.
        entity.surface_lifetime_timer_ms_at_0x48 =
            RetailRuntimeValue::Known(timer.saturating_sub(frame.elapsed_micros / 1000));
        return Ok(());
    }
    if effects.surface_selectors != [1, 0] {
        return Err(Block::Metadata("surface selectors"));
    }
    let phase = classify_actor_surface_timer_phase(
        timer,
        ActorSurfaceTimerFrame {
            state_flags: state,
            position_y_raw: entity.position_raw()[1],
            active_model_extent_raw: frame.active_model_extent_raw,
            flat_surface_y_raw: frame.terrain.sea_level_raw(),
            elapsed_us: frame.elapsed_micros,
            authored_lifetime_ms: effects.surface_lifetime_ms,
        },
    )
    .map_err(|_| Block::Metadata("surface lifetime"))?;
    let remaining = match phase {
        ActorSurfaceTimerPhase::OwnerDisabled => return Ok(()),
        ActorSurfaceTimerPhase::NonDeep { timer_after_ms }
        | ActorSurfaceTimerPhase::DeepBeforeRandomEffects { timer_after_ms, .. } => {
            entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(timer_after_ms);
            None
        }
        ActorSurfaceTimerPhase::DeepRandomEffects {
            timer_after_ms,
            remaining_percent,
        } => {
            entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(timer_after_ms);
            Some(remaining_percent)
        }
        ActorSurfaceTimerPhase::DeepLifecycle {
            timer_after_ms,
            remaining_percent_after_lifecycle,
        } => {
            // 162B0 writes +48 before calling 16750. If the release policy is
            // unresolved, preserve that timer prefix rather than losing it.
            entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(timer_after_ms);
            *death = run_lifecycle(manager, entity_id, world_fx)?;
            (remaining_percent_after_lifecycle < 75).then_some(remaining_percent_after_lifecycle)
        }
    };
    if let Some(remaining) = remaining {
        // E370 consumes the gate before reading any bubble-only live inputs.
        // A missed gate does not need the forward basis at all.
        if let Some(gate) = gate_actor_surface_bubble(remaining, &mut || {
            u32::from(world_fx.next_shared_retail_random_u16())
        }) {
            let entity = manager
                .entity_mut(entity_id)
                .ok_or(Block::AllocationUnavailable)?;
            let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
                return Err(Block::Runtime("surface emission basis"));
            };
            let RetailRuntimeValue::Known(state_flags) = entity
                .collision
                .state_flags_at_0x08
                .masked(REMOTE_OWNED_STATE_BIT)
            else {
                return Err(Block::Runtime("surface emission state"));
            };
            let bubble = gate.into_request(
                ActorSurfaceBubbleFrame {
                    entity_id,
                    entity_type: entity.entity_type as u8,
                    state_flags,
                    position_raw: entity.position_raw(),
                    emission_axis_q31: basis.forward,
                    active_model_extent_raw: frame.active_model_extent_raw,
                },
                &mut || u32::from(world_fx.next_shared_retail_random_u16()),
            );
            world_fx.materialize_actor_surface_bubble_request(
                bubble,
                frame.particle_environment,
                frame.retail_tick,
            );
        }
        // The sound branch follows synchronous allocation, reads current
        // state and XYZ, and has no dependency on the forward basis.
        let entity = manager
            .entity_mut(entity_id)
            .ok_or(Block::AllocationUnavailable)?;
        let surface = ActorSurfaceSoundFrame {
            state_flags: sound_state(entity)?,
            position_raw: entity.position_raw(),
        };
        if let Some(sound) = plan_ordinary_type9_surface_sound(surface, &mut || {
            u32::from(world_fx.next_shared_retail_random_u16())
        }) {
            world_fx.queue_fixed_positional_sound_raw(sound.sound_id, sound.position_raw);
        }
    }
    Ok(())
}

fn sound_state(entity: &Entity) -> Result<u32, Intro2CommonDyingBlock> {
    let word = entity.collision.state_flags_at_0x08;
    if !matches!(word.masked(DYING_STATE_BIT), RetailRuntimeValue::Known(_)) {
        return Err(Intro2CommonDyingBlock::Runtime("surface sound state"));
    }
    let RetailRuntimeValue::Known(known) = word.masked(word.known_mask()) else {
        unreachable!()
    };
    // E370's sound test is whole-word !=0. A known nonzero bit proves it
    // without inventing unresolved terrain-surface bits; proving zero needs
    // every bit. The bubble's independent remote bit was read only on a hit.
    if known == 0 && word.known_mask() != u32::MAX {
        return Err(Intro2CommonDyingBlock::Runtime("surface sound state"));
    }
    Ok(known)
}

#[cfg(test)]
mod tests;
