//! Type42's class22 construction, primary force phase and tertiary trail.
//!
//! Retail owners are `40B010`, `403840/403860`, `4069E0/406A70`, and
//! `4061A0`. These kernels require resolved live inputs. Entity allocation,
//! class22's `40CEF0 -> 410C10` lifecycle callback, fresh task-wrapper
//! survival, common E100 environment/integration, and model contact remain
//! with the production owner. Class28 Guided Missile is a distinct program.

use crate::{
    actor_task_owner::ActorTaskSlot,
    common_mover::{
        apply_sub_a_propulsion_raw, apply_sub_b_lateral_raw, shared_initializer_target_speed_raw,
        sub_c::{apply_sub_c_lift_raw, SubCLiftOutcome, SubCSurfaceSample},
        type9_attitude::Type9BodyBasis,
    },
    entity_collision_state::{
        CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
    },
    hover::{q31_mul, HoverLiftConfig},
    world_fx::{DescriptorParticleRequest, ParticleEnvironment, ParticleOwnerAtBirth, WorldFx},
};
use v2k_formats::collision::{SubAPropulsionDescriptor, SubBLateralDescriptor};

pub const ROCKET_ENTITY_TYPE: u16 = 42;
pub const ROCKET_CLASS22_INITIALIZER_ADDRESS: u32 = 0x0040_B010;
pub const ROCKET_FLIGHT_CALLBACK_ADDRESS: u32 = 0x0040_3860;
pub const ROCKET_TRAIL_CALLBACK_ADDRESS: u32 = 0x0040_6A70;
pub const ROCKET_TRAIL_EMITTER_ADDRESS: u32 = 0x0040_61A0;
pub const ROCKET_LIFETIME_MS: u32 = 2_000;
pub const ROCKET_ACQUISITION_RANGE_RAW: i32 = 0x300;
pub const ROCKET_ACQUISITION_FILTER_OVERRIDE_RAW: u32 = 1;
pub const ROCKET_TRAIL_CLASS: u8 = 31;
pub const ROCKET_UNDERWATER_TRAIL_CLASS: u8 = 42;
const ROCKET_TRAIL_STEP_MICROS: i32 = 30_000;
const ROCKET_TRAIL_POSITION_STEP_Q31: i32 = 0x03A9_8000;
const ROCKET_TRAIL_SPEED_SQUARED_MIN_RAW: i32 = 700 * 700;
const ROCKET_TRAIL_MODEL_EFFECT_BIT: u16 = 0x10;

/// Type42's ABC program has no D/01430 steering phase. Admission requires
/// the actual authored singleton class22 and its class1 terminal alternative.
pub(crate) fn authenticate_rocket_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), super::EntityWeaponBlock> {
    let Some(initializer) = metadata.initializer.as_ref() else {
        return Err(super::EntityWeaponBlock::Metadata);
    };
    let expected_topology = CommonMoverComponentTopology {
        sub_a: true,
        sub_b: true,
        sub_c: true,
        ..CommonMoverComponentTopology::default()
    };
    if metadata.model_slots != [240; 4]
        || metadata.mass_raw != 10
        || metadata.capability_flags != 0x40
        || metadata.common_mover_topology != RetailRuntimeValue::Known(expected_topology)
        || !matches!(
            metadata.sub_a_propulsion_descriptor,
            RetailRuntimeValue::Known(Some(_))
        )
        || !matches!(
            metadata.sub_b_lateral_descriptor,
            RetailRuntimeValue::Known(Some(_))
        )
        // The production flight owner samples terrain at the actor's center.
        // Reached wave/offset modes require their own sampling inputs; they
        // cannot enter this owner merely because a SubC record exists.
        || !matches!(
            metadata.sub_c_lift_descriptor,
            RetailRuntimeValue::Known(Some(c)) if c.surface_mode_raw == 0 && c.offset_sample_raw == 0
        )
        || initializer.initializer_state_flags_raw != 12
        || initializer.behavior_choices.len() != 1
        || initializer.behavior_choices[0].weight_rule_id != 1
        || initializer.behavior_choices[0].weight_multiplier != 1
        || initializer.behavior_choices[0].behavior_class_id != 22
        || initializer.alternate_behavior_class_ref != 1
    {
        return Err(super::EntityWeaponBlock::Metadata);
    }
    Ok(())
}

/// Exact three allocations requested by `40B010`, in publication order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RocketTaskPublication {
    /// `401F80(entity, 1, 0, 1)`: existing shared Guard acquisition kernel.
    Acquisition,
    /// `4069E0(entity, 2, 31)`: callback-owned, with no SubA reset.
    Trail,
    /// `403840(entity, 0, 2000)`: C/A/B forces, then strict primary timeout.
    Flight,
}

impl RocketTaskPublication {
    pub const fn slot(self) -> ActorTaskSlot {
        match self {
            Self::Acquisition => ActorTaskSlot::Secondary,
            Self::Trail => ActorTaskSlot::Tertiary,
            Self::Flight => ActorTaskSlot::Primary,
        }
    }

    pub const fn callback_address(self) -> u32 {
        match self {
            Self::Acquisition => 0x0040_1FB0,
            Self::Trail => ROCKET_TRAIL_CALLBACK_ADDRESS,
            Self::Flight => ROCKET_FLIGHT_CALLBACK_ADDRESS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RocketClass22InitRequest {
    pub sub_a: SubAPropulsionDescriptor,
    /// Actual completed SubA allocation value, before B010's x3/2 write.
    pub existing_target_speed_raw: i32,
}

/// Prepare must allocate without replacing the destination slot. Publication
/// consumes the prepared task after the source-owned reset, exactly as
/// `406030 -> 401020 -> 406070 -> 40A7A0` does. A preparation error retains
/// every earlier context write, RNG draw and published slot.
pub trait RocketClass22InitHost {
    type PreparedTask;
    type Error;

    fn write_search_range_raw(&mut self, range_raw: i32);
    fn write_sub_a_target_speed_raw(&mut self, target_speed_raw: i32);
    fn write_sub_a_direction_multiplier_raw(&mut self, direction_multiplier: i32);
    fn prepare_task(
        &mut self,
        request: RocketTaskPublication,
    ) -> Result<Self::PreparedTask, Self::Error>;
    fn next_shared_retail_random_u16(&mut self) -> u16;
    fn publish_task(&mut self, request: RocketTaskPublication, task: Self::PreparedTask);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RocketClass22InitProgress {
    pub target_speed_raw: i32,
    pub acquisition_reset_word: Option<u16>,
    pub flight_reset_word: Option<u16>,
    pub acquisition_published: bool,
    pub trail_published: bool,
    pub flight_published: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RocketClass22InitError<E> {
    pub failed_preparation: RocketTaskPublication,
    pub committed_prefix: RocketClass22InitProgress,
    pub error: E,
}

/// B010 consumes only the two task-reset words here. The entity constructor's
/// earlier SubA and singleton weighted-selector words belong upstream.
pub fn initialize_rocket_class22<H: RocketClass22InitHost>(
    request: RocketClass22InitRequest,
    host: &mut H,
) -> Result<RocketClass22InitProgress, RocketClass22InitError<H::Error>> {
    host.write_search_range_raw(ROCKET_ACQUISITION_RANGE_RAW);
    let initial = request.existing_target_speed_raw.wrapping_mul(3) / 2;
    host.write_sub_a_target_speed_raw(initial);
    let mut progress = RocketClass22InitProgress {
        target_speed_raw: initial,
        acquisition_reset_word: None,
        flight_reset_word: None,
        acquisition_published: false,
        trail_published: false,
        flight_published: false,
    };
    for publication in [
        RocketTaskPublication::Acquisition,
        RocketTaskPublication::Trail,
        RocketTaskPublication::Flight,
    ] {
        let prepared = host
            .prepare_task(publication)
            .map_err(|error| RocketClass22InitError {
                failed_preparation: publication,
                committed_prefix: progress,
                error,
            })?;
        if publication != RocketTaskPublication::Trail {
            let word = host.next_shared_retail_random_u16();
            let speed =
                shared_initializer_target_speed_raw(request.sub_a.target_speed_base_raw, word);
            host.write_sub_a_direction_multiplier_raw(1);
            host.write_sub_a_target_speed_raw(speed);
            progress.target_speed_raw = speed;
            match publication {
                RocketTaskPublication::Acquisition => progress.acquisition_reset_word = Some(word),
                RocketTaskPublication::Flight => progress.flight_reset_word = Some(word),
                RocketTaskPublication::Trail => unreachable!(),
            }
        }
        host.publish_task(publication, prepared);
        match publication {
            RocketTaskPublication::Acquisition => progress.acquisition_published = true,
            RocketTaskPublication::Trail => progress.trail_published = true,
            RocketTaskPublication::Flight => progress.flight_published = true,
        }
    }
    Ok(progress)
}

/// `401120` owns elapsed accounting before `403860`. Its post-unwind test
/// must be invoked only if the same wrapper survived the callback.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RocketFlightTaskState {
    elapsed_ms: u32,
}

impl RocketFlightTaskState {
    pub const fn new() -> Self {
        Self { elapsed_ms: 0 }
    }

    pub const fn elapsed_ms(self) -> u32 {
        self.elapsed_ms
    }

    pub fn before_callback(&mut self, elapsed_micros: u32) {
        self.elapsed_ms = self.elapsed_ms.wrapping_add(elapsed_micros / 1_000);
    }

    /// Request style+00 `40CEF0` after the wrapper survives and unwinds.
    pub const fn timeout_after_unwind(self) -> bool {
        ROCKET_LIFETIME_MS < self.elapsed_ms
    }
}

/// Resolved live input for `403860 -> 4018A0 -> C/A/B`. Type42 owns ABC,
/// with no D steering or target prelude. Previous-frame physical basis and
/// source-selected terrain sample must be supplied; no frozen basis or
/// substitute plane is created by this kernel.
#[derive(Debug, Clone, Copy)]
pub struct RocketFlightRequest {
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub body_basis: Type9BodyBasis,
    pub sub_a: SubAPropulsionDescriptor,
    pub sub_b: SubBLateralDescriptor,
    pub sub_c: HoverLiftConfig,
    pub target_speed_raw: i32,
    pub direction_multiplier: i32,
    pub drive_scale_percent: i32,
    pub surface: SubCSurfaceSample,
    /// Already charged global frame duration. E100 remains a later phase.
    pub elapsed_micros: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RocketFlightOutcome {
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub lift: SubCLiftOutcome,
}

pub fn apply_rocket_flight_forces_raw(request: RocketFlightRequest) -> RocketFlightOutcome {
    let mut position_raw = request.position_raw;
    let mut velocity_raw = request.velocity_raw;
    let lift = apply_sub_c_lift_raw(
        request.sub_c,
        &mut position_raw,
        &mut velocity_raw,
        request.elapsed_micros,
        request.surface,
        request.body_basis.up,
    );
    apply_sub_a_propulsion_raw(
        request.sub_a,
        request.target_speed_raw,
        request.direction_multiplier,
        request.drive_scale_percent,
        request.body_basis.forward,
        &mut velocity_raw,
        request.elapsed_micros,
    );
    apply_sub_b_lateral_raw(
        request.sub_b,
        request.body_basis.lateral,
        &mut velocity_raw,
        request.elapsed_micros,
    );
    RocketFlightOutcome {
        position_raw,
        velocity_raw,
        lift,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RocketTrailRequest {
    /// 406A70 only emits for reason zero. Other source callback reasons do
    /// not consume RNG, allocate, or change model-effect flags.
    pub callback_reason_raw: i32,
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub sea_level_raw: i16,
    pub owner: ParticleOwnerAtBirth,
    /// Exact source state-bit31 copied to 40A60 request flag byte.
    pub source_remote_bit: bool,
    pub elapsed_micros: u32,
}

pub trait RocketTrailHost {
    fn allocate_trail_particle(&mut self, request: DescriptorParticleRequest) -> Option<usize>;
    fn next_shared_retail_random_u16(&mut self) -> u16;
}

pub struct WorldFxRocketTrailHost<'a, 'world> {
    pub fx: &'a mut WorldFx,
    pub environment: ParticleEnvironment<'world>,
    pub retail_tick: u32,
}

impl RocketTrailHost for WorldFxRocketTrailHost<'_, '_> {
    fn allocate_trail_particle(&mut self, request: DescriptorParticleRequest) -> Option<usize> {
        self.fx
            .materialize_descriptor_particle_request(request, self.environment, self.retail_tick)
    }

    fn next_shared_retail_random_u16(&mut self) -> u16 {
        self.fx.next_shared_retail_random_u16()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RocketTrailOutcome {
    pub attempted_particles: u32,
    pub allocated_particles: u32,
    /// Apply to entity model-effect word+84 after the emitter returns, even
    /// if the particle pool refused every attempted allocation.
    pub model_effect_bits_to_set: u16,
}

pub fn emit_rocket_trail<H: RocketTrailHost>(
    request: RocketTrailRequest,
    host: &mut H,
) -> RocketTrailOutcome {
    let speed_squared = request.velocity_raw.into_iter().fold(0i32, |sum, word| {
        let value = i32::from(word);
        sum.wrapping_add(value.wrapping_mul(value))
    });
    if request.callback_reason_raw != 0 || speed_squared <= ROCKET_TRAIL_SPEED_SQUARED_MIN_RAW {
        return RocketTrailOutcome::default();
    }
    let mut outcome = RocketTrailOutcome {
        model_effect_bits_to_set: ROCKET_TRAIL_MODEL_EFFECT_BIT,
        ..RocketTrailOutcome::default()
    };
    let mut position_raw = request.position_raw;
    let position_step_raw = request
        .velocity_raw
        .map(|word| q31_mul(ROCKET_TRAIL_POSITION_STEP_Q31, i32::from(word)) as i16);
    let mut remaining_micros = request.elapsed_micros as i32;
    loop {
        let source_class = if request.sea_level_raw >= position_raw[1] {
            ROCKET_UNDERWATER_TRAIL_CLASS
        } else {
            ROCKET_TRAIL_CLASS
        };
        let allocation = host.allocate_trail_particle(DescriptorParticleRequest {
            source_class,
            position_raw,
            velocity_raw: [0; 3],
            owner: Some(request.owner),
            suppresses_impact_damage: request.source_remote_bit,
        });
        outcome.attempted_particles += 1;
        outcome.allocated_particles += u32::from(allocation.is_some());
        remaining_micros = remaining_micros.wrapping_sub(ROCKET_TRAIL_STEP_MICROS);
        //40629D/4062C7/4062D8 occur after 40A60, including its failure and
        //the final iteration. Updated Y selects the next iteration's class.
        for axis in 0..3 {
            let jitter_raw = ((host.next_shared_retail_random_u16() >> 11) as i16) - 16;
            position_raw[axis] = position_raw[axis]
                .wrapping_add(position_step_raw[axis])
                .wrapping_add(jitter_raw);
        }
        if remaining_micros <= 0 {
            return outcome;
        }
    }
}

#[cfg(test)]
#[path = "rocket_tests.rs"]
mod tests;
