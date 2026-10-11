//! Type10's own allocation binds the shared DEGKL frame to its retail data.

use super::*;
use crate::{
    common_mover::{
        component_dispatch::CommonMoverDispatchMode,
        target_prelude::CommonMoverTrackedTargetSnapshot,
    },
    entity_collision_state::{CommonMoverGklPayloads, EntityTypeRuntimeMetadata},
    gkl_common_mover::{
        evaluate_gkl_common_mover, GklCommonMoverProfile, GklCommonMoverRequest,
        GklCommonMoverRuntime,
    },
    wander_near_location::WanderNearPrivateState,
};
use v2k_formats::terrain::TerrainGrid;

#[derive(Clone, Copy)]
pub(super) struct MoverFrame<'a> {
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub terrain: &'a TerrainGrid,
    pub active_model_extent_raw: u16,
    pub dispatch_mode: CommonMoverDispatchMode,
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
    pub retail_tick: u32,
}

pub(super) fn run(
    entity: &mut Entity,
    frame: MoverFrame<'_>,
    target: &mut WanderNearPrivateState,
    tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    world_fx: &mut crate::world_fx::WorldFx,
) -> Result<bool, Intro2Type10Block> {
    use Intro2Type10Block as Block;
    if !intro2_type10_allocation_authenticates(entity) {
        return Err(Block::Allocation);
    }
    let runtime = entity.intro2_type10_runtime.ok_or(Block::Allocation)?;
    authenticate_metadata(runtime.profile, frame.metadata).map_err(|_| Block::Metadata)?;
    let RetailRuntimeValue::Known(Some(sub_g)) = entity.sub_g_06070_runtime else {
        return Err(Block::Runtime("Sub-G allocation"));
    };
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    let result = match evaluate_gkl_common_mover(
        GklCommonMoverRequest {
            entity_id: entity.id,
            entity_type: entity.entity_type,
            metadata: Some(frame.metadata),
            dispatch_mode: frame.dispatch_mode,
            position_raw: entity.position_raw(),
            velocity_raw: entity.velocity_raw(),
            heading_raw: heading as u16,
            pitch_raw: pitch,
            roll_raw: roll as u16,
            body_basis: entity.physical_body_basis_q31(),
            runtime: GklCommonMoverRuntime {
                sub_d_runtime: runtime.sub_d_runtime,
                sub_d_frame_owner: runtime.sub_d_frame_owner,
                sub_k_smoothed_raw: runtime.sub_k_smoothed_raw,
                sub_k_output_raw: runtime.sub_k_output_raw,
                sub_l_target_raw: runtime.sub_l_target_raw,
                sub_l_exact_raw: runtime.sub_l_exact_raw,
                sub_l_output_raw: runtime.sub_l_output_raw,
            },
            sub_g_runtime: sub_g,
            target_private: *target,
            tracked_target,
            terrain: frame.terrain,
            active_model_extent_raw: frame.active_model_extent_raw,
            self_mass_raw: entity.mass_raw,
            attached_cargo_mass: 0,
            capability_flags: frame.metadata.capability_flags,
            retail_tick: frame.retail_tick,
            elapsed_micros: frame.elapsed_micros,
            global_elapsed_micros: frame.global_elapsed_micros,
        },
        GklCommonMoverProfile {
            topology: TOPOLOGY,
            sub_d: runtime.profile.sub_d(),
            gkl: CommonMoverGklPayloads {
                sub_g: Some(*runtime.profile.sub_g()),
                sub_k: Some(SUB_K),
                sub_l: Some(SUB_L),
            },
        },
        &mut || u32::from(world_fx.next_shared_retail_random_u16()),
    ) {
        Ok(result) => result,
        Err(failure) => {
            commit_failure_prefix(entity, target, world_fx, failure.prefix);
            return Err(Block::Mover(failure.reason));
        }
    };
    let live = entity
        .intro2_type10_runtime
        .as_mut()
        .ok_or(Block::Allocation)?;
    live.sub_d_runtime = result.runtime.sub_d_runtime;
    live.sub_d_frame_owner = result.runtime.sub_d_frame_owner;
    live.sub_k_smoothed_raw = result.runtime.sub_k_smoothed_raw;
    live.sub_k_output_raw = result.runtime.sub_k_output_raw;
    live.sub_l_target_raw = result.runtime.sub_l_target_raw;
    live.sub_l_exact_raw = result.runtime.sub_l_exact_raw;
    live.sub_l_output_raw = result.runtime.sub_l_output_raw;
    entity.sub_g_06070_runtime = RetailRuntimeValue::Known(Some(result.sub_g_runtime));
    entity.set_position_raw(result.position_raw);
    entity.set_velocity_raw(result.velocity_raw);
    entity.set_rotation_heading_pitch_roll_raw([
        result.heading_raw as i16,
        result.pitch_raw,
        result.roll_raw as i16,
    ]);
    *target = result.target_private;
    if let Some(sound) = result.sound {
        world_fx.queue_fixed_positional_sound_raw_at_rate(
            sound.sound_id,
            sound.position_raw,
            sound.rate_q16,
        );
    }
    Ok(result.result == crate::chase_target::ChaseTargetCommonMoverReturn::NonZero)
}

fn commit_failure_prefix(
    entity: &mut Entity,
    target: &mut WanderNearPrivateState,
    world_fx: &mut crate::world_fx::WorldFx,
    prefix: crate::gkl_common_mover::GklCommonMoverPrefix,
) {
    let runtime = entity
        .intro2_type10_runtime
        .as_mut()
        .expect("admitted allocation retained through detached mover");
    if let Some(value) = prefix.sub_d_runtime {
        runtime.sub_d_runtime = value;
    }
    if let Some(value) = prefix.sub_d_frame_owner {
        runtime.sub_d_frame_owner = value;
    }
    if let Some(value) = prefix.sub_k_smoothed_raw {
        runtime.sub_k_smoothed_raw = value;
    }
    if let Some(value) = prefix.sub_k_output_raw {
        runtime.sub_k_output_raw = value;
    }
    if let Some(value) = prefix.sub_l_target_raw {
        runtime.sub_l_target_raw = value;
    }
    if let Some(value) = prefix.sub_l_exact_raw {
        runtime.sub_l_exact_raw = value;
    }
    if let Some(value) = prefix.sub_l_output_raw {
        runtime.sub_l_output_raw = value;
    }
    if let Some(value) = prefix.sub_g_reverse_write {
        let RetailRuntimeValue::Known(Some(sub_g)) = &mut entity.sub_g_06070_runtime else {
            unreachable!("admitted G allocation retained through detached mover")
        };
        sub_g.apply_direction_reverse_write(value);
    }
    if let Some(value) = prefix.sub_g_runtime {
        entity.sub_g_06070_runtime = RetailRuntimeValue::Known(Some(value));
    }
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    if prefix.heading_raw.is_some() || prefix.pitch_raw.is_some() || prefix.roll_raw.is_some() {
        entity.set_rotation_heading_pitch_roll_raw([
            prefix.heading_raw.map_or(heading, |value| value as i16),
            prefix.pitch_raw.unwrap_or(pitch),
            prefix.roll_raw.map_or(roll, |value| value as i16),
        ]);
    }
    if let Some(value) = prefix.velocity_raw {
        entity.set_velocity_raw(value);
    }
    if let Some(value) = prefix.target_private {
        *target = value;
    }
    if let Some(sound) = prefix.sound {
        world_fx.queue_fixed_positional_sound_raw_at_rate(
            sound.sound_id,
            sound.position_raw,
            sound.rate_q16,
        );
    }
}

#[cfg(test)]
mod tests;
