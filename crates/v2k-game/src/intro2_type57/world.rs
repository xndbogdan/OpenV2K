//! DCA0/E870 detail sound, 13F70, E100/E370, then master motion.

use super::*;
use crate::{
    common_mover::{
        type9_attitude::Type9BodyBasis,
        type9_surface::{decay_actor_surface_timer_ms, ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT},
    },
    entity::{apply_type13_common_environment_raw, EntityManager},
    entity_collision_state::{
        CommonWorldEffectProfile, EntityTypeRuntimeMetadata, BODY_BASIS_REBUILT_STATE_BIT,
    },
};

pub(super) fn effective_flags(entity: &Entity) -> Result<u32, Intro2Type57Block> {
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Intro2Type57Block::Graph);
    };
    if entity.collision.default_state_flags_at_0xc8 != RetailRuntimeValue::Known(8)
        || !matches!(
            context.active_style().style_address(),
            0x4c7a50 | 0x4c7a98 | 0x4c7f60 | 0x4c7fa8
        )
    {
        return Err(Intro2Type57Block::Runtime("effective style flags"));
    }
    Ok(8)
}

pub(super) fn finish(
    manager: &mut EntityManager,
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    frame: &mut Intro2Type57Frame<'_>,
    dt: u32,
    entry_state: u32,
    entry_effective_flags: u32,
) -> Result<(), Intro2Type57Block> {
    use Intro2Type57Block as Block;
    let environment = manager.intro2_type13_environment();
    if environment.0 != 0 {
        return Err(Block::Runtime("wind mode"));
    }
    if metadata.common_world_effects
        != RetailRuntimeValue::Known(CommonWorldEffectProfile::default())
    {
        return Err(Block::Metadata);
    }
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    if entry_state & crate::entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT != 0 {
        let record = frame
            .resources
            .global_entity_type(57)
            .ok_or(Block::Metadata)?;
        let RetailRuntimeValue::Known(health) = entity.collision.health_raw else {
            return Err(Block::Runtime("detailed sound health"));
        };
        let visible = super::live::bits(entity, 0x800)? != 0;
        if let Some(sound) = crate::actor_detailed_sound::plan_actor_detailed_sound(
            crate::actor_detailed_sound::ActorDetailedSoundFrame {
                type_record: record,
                health_raw: health,
                visible,
                callback_elapsed_micros: dt,
            },
            &mut || u32::from(frame.world_fx.next_shared_retail_random_u16()),
        ) {
            frame
                .world_fx
                .queue_fixed_positional_sound_raw(sound, entity.position_raw());
        }
    }
    if entry_effective_flags != 8 {
        return Err(Block::Runtime("entry basis policy"));
    }
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    entity.physical_body_basis_q31 =
        RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(heading, pitch, roll));
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
    effective_flags(entity)?;
    let mut velocity = entity.velocity_raw();
    apply_type13_common_environment_raw(
        &mut velocity,
        dt,
        entity.mass_raw,
        environment.0,
        environment.1,
    );
    entity.set_velocity_raw(velocity);
    if super::live::bits(entity, ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT)? == 0 {
        let RetailRuntimeValue::Known(timer) = entity.surface_lifetime_timer_ms_at_0x48 else {
            return Err(Block::Runtime("surface timer"));
        };
        entity.surface_lifetime_timer_ms_at_0x48 =
            RetailRuntimeValue::Known(decay_actor_surface_timer_ms(timer, dt));
    }
    super::live::commit_motion(entity, dt)
}
