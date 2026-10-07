//! Backend-neutral world effects and one-shot timeline events.
//!
//! The retail game routes object impacts, weapon hits, deaths, particles, and
//! positional sounds through shared world systems.  Keeping this state out of
//! the opening renderer lets Intro2 use the same path as later gameplay.

use std::borrow::Cow;
use std::collections::{BTreeSet, VecDeque};

mod attached_update;
pub use attached_update::{
    begin_attached_particle_owner_update, sample_attached_particle_cached_owner,
    AttachedParticleCascadeOwner, AttachedParticleCascadeOwnerRequest,
    AttachedParticleDamageRequest, AttachedParticleOwnerLookup, AttachedParticleOwnerRequest,
    AttachedParticleOwnerState, AttachedParticleUpdateBlock, AttachedParticleUpdateDiagnostic,
    AttachedParticleUpdateObservation,
};
mod combat_projectiles;
pub use combat_projectiles::{CombatGroundProgramRequest, CombatProjectileBirth};
mod intro2_meteors;
mod presentation;
mod projectile_sweep;
use projectile_sweep::{
    detect_damage_projectile_sweep, ProjectileModelSweepProgram, ProjectileSweepFrame,
};
mod surface_transition;
use surface_transition::uses_projectile_surface_callback;
mod traversal;
pub use traversal::{
    ParticleTerrainEvent, ParticleTerrainPublication, ParticleTerrainResponse,
    ParticleTraversalContext, ParticleTraversalHost, ParticleTraversalTiming,
};
mod virus_projectile;
mod whole_body_contact;
pub(crate) use whole_body_contact::WholeBodyContactScatter;

use crate::common_mover::type9_surface::ActorSurfaceBubbleRequest;
use crate::damage::{
    DamageDeliveryRecord, DamagePacket, BALLISTIC_PARTICLE_DAMAGE_PACKET,
    FUN_0043F6E0_DAMAGE_PACKET, FUN_0043F780_DAMAGE_PACKET, FUN_0043F7C0_DAMAGE_PACKET,
    PRIMARY_PROJECTILE_DAMAGE_PACKET, TYPE_47_PROJECTILE_DAMAGE_PACKET,
};
use crate::defecate_virus::{DefecateVirusParticleEmission, TerrainContactMode};
use crate::entity::PlayerSurfaceEffectEmission;
use crate::entity_emitters::{AuthoredHiveComponentEffects, AuthoredRadialEmission};
use crate::infection_evolution::{
    InfectionEvolutionEffects, InfectionTailSound, INFECTION_TERRAIN_TYPE_BIT,
};
use crate::particle_descriptors::{particle_descriptor, particle_frame, PARTICLE_DESCRIPTORS};
use crate::primary_hit::{
    PrimaryHitCapabilityEmission, PRIMARY_HIT_CAPABILITY_PARTICLE_CLASS,
    PRIMARY_HIT_CAPABILITY_PARTICLE_SCALE_RAW,
};
use crate::primary_weapon::PrimaryFireEvent;
use crate::projectile_emitter::{
    CLASS_87_DAMAGE_PACKET_VA, CLASS_87_ENTITY_HIT_CALLBACK_VA, CLASS_87_STATIC_HIT_CALLBACK_VA,
    FIRST_WORLD_SHOOTER_PARTICLE_CLASS, TYPE13_PROJECTILE_PARTICLE_CLASS,
    TYPE13_PROJECTILE_UNDERWATER_PARTICLE_CLASS,
};
use crate::resource_cache::ResourceCache;
use crate::retail_clock::RETAIL_FRAME_DELTA_MAX_US;
use crate::retail_rng::retail_random_u16;
use crate::static_damage_live::{
    resolve_static_damage_snapshot_from_cell, CurrentStaticDamageSnapshot,
};
use crate::static_objects::StaticTerrainObjectInstance;
use v2k_formats::anim_frames::TerrainObjectTable;
use v2k_formats::fixed_math::retail_sine_q15;
use v2k_formats::models::{AnimVars, CollisionModelPool, ModelEntry};
use v2k_formats::terrain::{wave_surface_raw, TerrainGrid};

pub use crate::particle_descriptors::ParticleFrame;

impl CollisionModelPool for ResourceCache {
    fn collision_model(&self, global_id: usize) -> Option<&ModelEntry> {
        self.global_model(global_id)
    }
}

/// The original simulation clock advances at 50 Hz.
pub const WORLD_FX_TICKS_PER_SECOND: f32 = 50.0;
/// `FUN_0044FFA0` keeps eight uncapped frame-duration samples.
const PARTICLE_FRAME_SAMPLE_COUNT: usize = 8;
/// Durations through 125,000 microseconds enter the rolling window. A larger
/// duration replaces the complete window with this value and disables the
/// variable-count particle loops for that frame.
const PARTICLE_FRAME_SAMPLE_MAX_US: u32 = 125_000;
/// A rolling average below 59,464 microseconds receives the full Q16 scale.
/// At exactly this boundary, subtraction from 125,000 produces the same
/// `0x10000` result.
const PARTICLE_FULL_RATE_AVERAGE_US: u32 = 59_464;
const PARTICLE_FULL_RATE_SCALE_Q16: u32 = 0x1_0000;
/// Retail's fixed particle pool contains exactly 200 records.
pub const MAX_WORLD_PARTICLES: usize = 200;
/// Retail virtual address of physical particle slot zero.
const RETAIL_PARTICLE_POOL_VA: usize = 0x004D_CF40;
/// One retail particle record is exactly 0x20 bytes.
const RETAIL_PARTICLE_RECORD_BYTES: usize = 0x20;
/// Free list zero followed by six live priority lists.
const PARTICLE_LIST_COUNT: usize = 7;
/// Descriptor callback addresses used to identify the exact mode-1 family.
const GRAVITY_UPDATE_CALLBACK_VA: u32 = 0x0043_F260;
const MODE_ONE_SURFACE_CALLBACK_VA: u32 = 0x0043_E230;
const ALIEN_HIVE_SURFACE_CALLBACK_VA: u32 = 0x0043_E180;
const CLEANSING_LANDSCAPE_SURFACE_CALLBACK_VA: u32 = 0x0043_E1A0;
const PLAYER_SURFACE_PROBE_CALLBACK_VA: u32 = 0x0043_E8A0;
const DOWNWASH_DEBRIS_UPDATE_CALLBACK_VA: u32 = 0x0044_23C0;
const BALLISTIC_TRAIL_UPDATE_CALLBACK_VA: u32 = 0x0043_ECD0;
const BALLISTIC_SURFACE_CALLBACK_VA: u32 = 0x0043_E4F0;
const BALLISTIC_ENTITY_CALLBACK_VA: u32 = 0x0043_F590;
const BALLISTIC_DAMAGE_PACKET_VA: u32 = 0x004C_C000;
const BALLISTIC_STATIC_CALLBACK_VA: u32 = 0x0043_F800;
/// `FUN_00442950` entity-hit slot shared by the static-route family
/// (particle classes 52/68/85). Unlike `FUN_0043F590`, it first runs the
/// `FUN_00441180` damage sequence, then the `FUN_0043F610` visual, then an
/// attached-class allocation unless suppressed.
const STATIC_ROUTE_ENTITY_CALLBACK_VA: u32 = 0x0044_2950;
/// Class52's `+0x20` packet; its row is shared with Type-47 class-87 fire.
const STATIC_ROUTE_CLASS52_DAMAGE_PACKET_VA: u32 = 0x004C_C0F0;
/// Class68's `+0x20` packet (Type57 bat fire).
const STATIC_ROUTE_CLASS68_DAMAGE_PACKET_VA: u32 = 0x004C_C108;
/// Class85's `+0x20` packet (shared row with dragon class-38 fire).
const STATIC_ROUTE_CLASS85_DAMAGE_PACKET_VA: u32 = 0x004C_C048;
/// Static no-op shared by classes 52/68 (`+0x24`); class85 uses `FUN_0043F800`.
const STATIC_ROUTE_NOOP_STATIC_CALLBACK_VA: u32 = 0x0042_E8E0;
/// `FUN_004425D0` attached-follow update for classes 83/84/86: repositions at
/// the owner each tick (directly, or basis-rotated when owner state
/// `0x2000000` is set), kills on owner loss. Per-tick damage, effect spawns,
/// and cascade allocation stay open.
const ATTACHED_FOLLOW_UPDATE_CALLBACK_VA: u32 = 0x0044_25D0;
/// Class-1/2 primary damage packet (`descriptor + 0x20` at `0x004CBF70`).
/// Class 3 uses the adjacent identical-value record `0x004CBF88`. Requiring
/// only the class-3 VA drops default bullets out of `detect_primary_collision`.
const PRIMARY_CLASS1_DAMAGE_PACKET_VA: u32 = 0x004C_BF70;
const PRIMARY_CLASS3_DAMAGE_PACKET_VA: u32 = 0x004C_BF88;

/// Retail default-primary bullet particle (class 1, record `0x004CC16C`).
pub const PRIMARY_BULLET_PARTICLE_CLASS: u8 = 1;
/// Factory machine-gun bullet (class 2, record `0x004CC1A0`). Its descriptor
/// matches class 1 byte-for-byte except for draw scale 0x400 instead of 0x300.
pub const RAPID_PRIMARY_PARTICLE_CLASS: u8 = 2;
pub const PRIMARY_BULLET_LIFETIME_TICKS: u16 =
    PARTICLE_DESCRIPTORS[PRIMARY_BULLET_PARTICLE_CLASS as usize].lifetime_ticks();
pub const PRIMARY_BULLET_RADIUS_RAW: u16 =
    PARTICLE_DESCRIPTORS[PRIMARY_BULLET_PARTICLE_CLASS as usize].collision_radius_raw();
pub const PRIMARY_BULLET_ANIMATION_RATE_RAW: u8 =
    PARTICLE_DESCRIPTORS[PRIMARY_BULLET_PARTICLE_CLASS as usize].animation_rate_raw();
pub const PRIMARY_BULLET_DRAW_SCALE_RAW: u16 =
    PARTICLE_DESCRIPTORS[PRIMARY_BULLET_PARTICLE_CLASS as usize].draw_scale_raw();
/// Level-1 weapon-upgrade projectile (class 3, record `0x004CC204`).
pub const UPGRADED_PRIMARY_PARTICLE_CLASS: u8 = 3;
pub const UPGRADED_PRIMARY_LIFETIME_TICKS: u16 =
    PARTICLE_DESCRIPTORS[UPGRADED_PRIMARY_PARTICLE_CLASS as usize].lifetime_ticks();
pub const UPGRADED_PRIMARY_RADIUS_RAW: u16 =
    PARTICLE_DESCRIPTORS[UPGRADED_PRIMARY_PARTICLE_CLASS as usize].collision_radius_raw();
pub const UPGRADED_PRIMARY_ANIMATION_RATE_RAW: u8 =
    PARTICLE_DESCRIPTORS[UPGRADED_PRIMARY_PARTICLE_CLASS as usize].animation_rate_raw();
pub const UPGRADED_PRIMARY_DRAW_SCALE_RAW: u16 =
    PARTICLE_DESCRIPTORS[UPGRADED_PRIMARY_PARTICLE_CLASS as usize].draw_scale_raw();
/// Bullet and flare classes selected by the recovered Level-1 loadouts.
pub const SUPPORTED_PRIMARY_PARTICLE_CLASSES: [u8; 3] = [
    PRIMARY_BULLET_PARTICLE_CLASS,
    RAPID_PRIMARY_PARTICLE_CLASS,
    UPGRADED_PRIMARY_PARTICLE_CLASS,
];
/// `FUN_00441770` child emitted when class 3 meets response selector 6/7.
pub const UPGRADED_PRIMARY_SUBMERGED_SURFACE_CLASS: u8 = 43;
pub const UPGRADED_PRIMARY_ABOVE_SURFACE_CLASS: u8 = 75;
pub const UPGRADED_PRIMARY_SURFACE_VELOCITY_RAW: [i32; 3] = [0, 200, 0];
/// Alien-Hive component particle emitted by `FUN_0041BEB0` (class 5).
pub const ALIEN_HIVE_PARTICLE_CLASS: u8 = 5;
/// Smooth equivalent of the underwater class-1 callback damping factor
/// `1 - (dt_us >> 7) / 32768`.
pub const PRIMARY_BULLET_UNDERWATER_DRAG_PER_SECOND: f32 = 1_000_000.0 / 4_194_304.0;
/// Class-1 frees itself when every raw velocity component is below 400.
pub const PRIMARY_BULLET_MIN_SPEED_COMPONENT_WORLD_PER_SECOND: f32 =
    400.0 * (1_000_000.0 / 1_048_576.0) / 256.0;
/// Class-15's resolved above-water muzzle flash/smoke particle (class 32).
pub const PRIMARY_MUZZLE_PARTICLE_CLASS: u8 = 32;
pub const PRIMARY_MUZZLE_LIFETIME_TICKS: u16 =
    PARTICLE_DESCRIPTORS[PRIMARY_MUZZLE_PARTICLE_CLASS as usize].lifetime_ticks();
pub const PRIMARY_MUZZLE_RADIUS_RAW: u16 =
    PARTICLE_DESCRIPTORS[PRIMARY_MUZZLE_PARTICLE_CLASS as usize].collision_radius_raw();
pub const PRIMARY_MUZZLE_ANIMATION_RATE_RAW: u8 =
    PARTICLE_DESCRIPTORS[PRIMARY_MUZZLE_PARTICLE_CLASS as usize].animation_rate_raw();
pub const PRIMARY_MUZZLE_DRAW_SCALE_RAW: u16 =
    PARTICLE_DESCRIPTORS[PRIMARY_MUZZLE_PARTICLE_CLASS as usize].draw_scale_raw();
pub const PRIMARY_MUZZLE_Y_OFFSET_RAW: i16 = 35;
/// Class-1 entity-hit effects selected by `FUN_0043F610`.
pub const PRIMARY_ABOVE_WATER_IMPACT_CLASS: u8 = 34;
pub const PRIMARY_ABOVE_WATER_IMPACT_LIFETIME_TICKS: u16 =
    PARTICLE_DESCRIPTORS[PRIMARY_ABOVE_WATER_IMPACT_CLASS as usize].lifetime_ticks();
pub const PRIMARY_ABOVE_WATER_IMPACT_RADIUS_RAW: u16 =
    PARTICLE_DESCRIPTORS[PRIMARY_ABOVE_WATER_IMPACT_CLASS as usize].collision_radius_raw();
pub const PRIMARY_ABOVE_WATER_IMPACT_ANIMATION_RATE_RAW: u8 =
    PARTICLE_DESCRIPTORS[PRIMARY_ABOVE_WATER_IMPACT_CLASS as usize].animation_rate_raw();
pub const PRIMARY_ABOVE_WATER_IMPACT_DRAW_SCALE_RAW: u16 =
    PARTICLE_DESCRIPTORS[PRIMARY_ABOVE_WATER_IMPACT_CLASS as usize].draw_scale_raw();
pub const PRIMARY_ABOVE_WATER_IMPACT_SIZE_JITTER_DIVISOR_RAW: u8 =
    PARTICLE_DESCRIPTORS[PRIMARY_ABOVE_WATER_IMPACT_CLASS as usize].size_jitter_divisor_raw();
pub const PRIMARY_ABOVE_WATER_IMPACT_INITIAL_VELOCITY_RAW: [i32; 3] = [0, 80, 0];
pub const PRIMARY_UNDERWATER_IMPACT_CLASS: u8 = 42;
pub const PRIMARY_UNDERWATER_IMPACT_LIFETIME_TICKS: u16 =
    PARTICLE_DESCRIPTORS[PRIMARY_UNDERWATER_IMPACT_CLASS as usize].lifetime_ticks();
pub const PRIMARY_UNDERWATER_IMPACT_RADIUS_RAW: u16 =
    PARTICLE_DESCRIPTORS[PRIMARY_UNDERWATER_IMPACT_CLASS as usize].collision_radius_raw();
pub const PRIMARY_UNDERWATER_IMPACT_ANIMATION_RATE_RAW: u8 =
    PARTICLE_DESCRIPTORS[PRIMARY_UNDERWATER_IMPACT_CLASS as usize].animation_rate_raw();
pub const PRIMARY_UNDERWATER_IMPACT_DRAW_SCALE_RAW: u16 =
    PARTICLE_DESCRIPTORS[PRIMARY_UNDERWATER_IMPACT_CLASS as usize].draw_scale_raw();
pub const PRIMARY_UNDERWATER_IMPACT_SIZE_JITTER_DIVISOR_RAW: u8 =
    PARTICLE_DESCRIPTORS[PRIMARY_UNDERWATER_IMPACT_CLASS as usize].size_jitter_divisor_raw();
pub const PRIMARY_UNDERWATER_IMPACT_INITIAL_VELOCITY_RAW: [i32; 3] = [0, 400, 0];
/// Ordinary class-1 terrain-hit sound from `FUN_0043DEB0`.
pub const PRIMARY_TERRAIN_IMPACT_SOUND_ID: usize = 83;
pub const PRIMARY_TERRAIN_IMPACT_RATE_MIN_16_16: u32 = 0xE000;
pub const PRIMARY_TERRAIN_IMPACT_RATE_MAX_EXCLUSIVE_16_16: u32 = 0x1_2000;
pub const SPECIAL_SURFACE_SOUND_IDS: [usize; 2] = [26, 27];
pub const SPECIAL_SURFACE_RATE_MIN_16_16: u32 = 0xC000;
pub const SPECIAL_SURFACE_RATE_MAX_EXCLUSIVE_16_16: u32 = 0x1_0000;
/// Exact `FUN_0043DEB0` lookup at retail `0x004CD718`, through the
/// highest selector used by authored terrain. Selectors 6 and 7 branch to
/// their dedicated water paths before this table is read.
pub const SURFACE_RESPONSE_PARTICLE_CLASS_BY_SELECTOR: [u8; 13] =
    [7, 8, 9, 10, 7, 11, 0, 0, 59, 10, 71, 70, 72];
/// Exact `FUN_0043E4F0` lookup at retail `0x004CD7C0`. Unlike the earlier
/// `FUN_0043DEB0` table, selectors 6 and 7 map to the authored water classes
/// 13 and 12. Selector 6 takes its dedicated spray path before this table is
/// read; selector 7 reaches the table normally.
pub const BALLISTIC_SURFACE_RESPONSE_CLASS_BY_SELECTOR: [u8; 13] =
    [7, 8, 9, 10, 7, 11, 13, 12, 59, 10, 71, 70, 72];
pub const SURFACE_RESPONSE_LIFETIME_TICKS: u16 = PARTICLE_DESCRIPTORS[7].lifetime_ticks();
pub const SURFACE_RESPONSE_RADIUS_RAW: u16 = PARTICLE_DESCRIPTORS[7].collision_radius_raw();
pub const SURFACE_RESPONSE_SIZE_JITTER_DIVISOR_RAW: u8 =
    PARTICLE_DESCRIPTORS[7].size_jitter_divisor_raw();
/// Full-rate directional count for the ordinary `FUN_0043E4F0` and
/// `FUN_00440DC0` response paths. Runtime allocation attempts are scaled by
/// `DAT_004F72CC`; this constant remains the ordinary-frame reference count.
pub const SURFACE_RESPONSE_BURST_COUNT: usize = 4;
pub const SURFACE_RESPONSE_DRAW_SCALE_RAW: u16 = PARTICLE_DESCRIPTORS[7].draw_scale_raw();
pub const WATER_SPLASH_PARTICLE_CLASS: u8 = 13;
pub const WATER_SPLASH_ANIMATION_RATE_RAW: u8 =
    PARTICLE_DESCRIPTORS[WATER_SPLASH_PARTICLE_CLASS as usize].animation_rate_raw();
pub const WATER_SPLASH_DRAW_SCALE_RAW: u16 =
    PARTICLE_DESCRIPTORS[WATER_SPLASH_PARTICLE_CLASS as usize].draw_scale_raw();
/// `FUN_004141D0`'s whole-body water-entry response table. Unlike the
/// class-1/mode-1 special-surface path, selector six still uses the ordinary
/// four-way `FUN_00440DC0` scatter, with class 13 as its selected descriptor.
/// Selector seven deliberately has no entry and suppresses the burst.
pub const WHOLE_BODY_WATER_ENTRY_CLASS_BY_SELECTOR: [u8; 7] = [7, 8, 9, 10, 7, 11, 13];
pub const WHOLE_BODY_WATER_ENTRY_BURST_COUNT: usize = SURFACE_RESPONSE_BURST_COUNT;
pub const WATER_DELETE_PARTICLE_CLASS: u8 = 12;
pub const WATER_DELETE_ANIMATION_RATE_RAW: u8 =
    PARTICLE_DESCRIPTORS[WATER_DELETE_PARTICLE_CLASS as usize].animation_rate_raw();
pub const WATER_DELETE_DRAW_SCALE_RAW: u16 =
    PARTICLE_DESCRIPTORS[WATER_DELETE_PARTICLE_CLASS as usize].draw_scale_raw();

/// Invisible craft downwash probe constructed by `FUN_00440E80` above the
/// static sea plane. Its `FUN_0043E8A0` callback emits the visible response
/// only after the probe reaches terrain or a displaced wave.
pub const PLAYER_SURFACE_PROBE_PARTICLE_CLASS: u8 = 19;
/// Underwater branch of `FUN_00440E80`.
pub const PLAYER_SUBMERGED_DOWNWASH_PARTICLE_CLASS: u8 = 44;
/// Wave-surface child emitted by class 19's selector-six callback. This is
/// class 14, deliberately distinct from E230's class-13 impact splash.
pub const PLAYER_WATER_DOWNWASH_PARTICLE_CLASS: u8 = 14;
/// `FUN_0043E8A0` maps the live Section-13 response selector through this
/// exact class table. Selectors 6 and 7 deliberately have no terrain debris.
pub const PLAYER_DOWNWASH_CLASS_BY_SELECTOR: [u8; 8] = [60, 61, 62, 63, 60, 64, 0, 0];
/// Ordinary-frame `DAT_004F72CC == 0x10000` yields eight children per probe.
pub const PLAYER_DOWNWASH_RESPONSE_COUNT: usize = 8;
/// Ordinary-frame `DAT_004F72CC == 0x10000` also yields eight selector-six
/// attempts in `FUN_0043E4F0`. Runtime uses the direct
/// `DAT_004F72CC >> 13` count and therefore permits zero after a hard stall.
pub const BALLISTIC_WATER_RESPONSE_COUNT: usize = 8;

/// Cargo-transfer/settling particle emitted by the type-93 drop proxy and by
/// a successful player collection (`FUN_004091B0`/`FUN_00443AE0`).
pub const CARGO_TRANSFER_PARTICLE_CLASS: u8 = 0x33;
pub const CARGO_TRANSFER_LIFETIME_TICKS: u16 =
    PARTICLE_DESCRIPTORS[CARGO_TRANSFER_PARTICLE_CLASS as usize].lifetime_ticks();
pub const CARGO_TRANSFER_RADIUS_RAW: u16 =
    PARTICLE_DESCRIPTORS[CARGO_TRANSFER_PARTICLE_CLASS as usize].collision_radius_raw();
pub const CARGO_TRANSFER_ANIMATION_RATE_RAW: u8 =
    PARTICLE_DESCRIPTORS[CARGO_TRANSFER_PARTICLE_CLASS as usize].animation_rate_raw();
pub const CARGO_TRANSFER_DRAW_SCALE_RAW: u16 =
    PARTICLE_DESCRIPTORS[CARGO_TRANSFER_PARTICLE_CLASS as usize].draw_scale_raw();
/// Positional Section-11 cue shared by dropping and collecting cargo.
pub const CARGO_TRANSFER_SOUND_ID: usize = 8;
/// Positional failure cue when the player's authored Sub-J list is full.
pub const CARGO_FULL_SOUND_ID: usize = 2;

/// Positional cue shared by accepted controller pickup operations `0x33`
/// (fuel) and `0x37` (hull repair). Both pass global Section-11 sound id 5 at
/// the player's live position, independently of their notification slots.
pub const PLAYER_PICKUP_SOUND_ID: u16 = 5;
/// Managed non-positional warning cue submitted below 10000 fuel. Retail uses
/// the same global sample as the positional empty-VTOL cue below.
pub const FUEL_LOW_SOUND_ID: usize = 0x31;
/// Managed non-positional warning cue submitted below `0x1389` hull health.
/// `FUN_00446640` resolves physical global Section-11 slot `0xB4 / 4`.
pub const HULL_LOW_SOUND_ID: usize = 0x2d;
/// Centered wrapper gain submitted with the low-hull cue (`0x8000` in Q16).
pub const HULL_LOW_SOUND_GAIN: f32 = 0.5;
/// Positional cue emitted when `FUN_00445310` begins a VTOL callback empty.
/// This is global Section-11 sound id 0x31, not entity/particle class 0x31.
pub const FUEL_EXHAUSTED_SOUND_ID: u16 = 0x31;
/// Hard whole-body water-entry cue selected by `FUN_004141D0` only when the
/// entering entity's signed raw Y velocity is below -1000.
pub const PLAYER_WATER_ENTRY_SOUND_ID: u16 = 17;

/// `FUN_004141D0`'s severe hard-entry ring (`splash`, global Section-8 model
/// 130) and moderate ring (`splashmid`, model 132). Both models instance five
/// authored segment children and morph them through dynamic channel one.
pub const HARD_WATER_ENTRY_SPLASH_MODEL_ID: usize = 130;
pub const HARD_WATER_ENTRY_SPLASH_MID_MODEL_ID: usize = 132;
pub const EXPLODING_RING_INITIAL_CONTROL_RAW: u16 = u16::MAX;
/// Section-8 model selected by Type 60's ordinary Section-12 row.
const TYPE60_EXPLODING_RING_MODEL_ID: usize = 243;

/// Static terrain-object particle classes emitted by `FUN_0042F650` through
/// `FUN_00441670` and `FUN_00441700`.
pub const STATIC_PUFF_ABOVE_WATER_CLASS: u8 = 20;
pub const STATIC_FLAME_ABOVE_WATER_CLASS: u8 = 21;
pub const STATIC_PUFF_UNDERWATER_CLASS: u8 = 42;
pub const STATIC_FLAME_UNDERWATER_CLASS: u8 = 43;
pub const STATIC_PUFF_Y_OFFSET_RAW: i16 = 200;
pub const STATIC_FLAME_Y_OFFSET_MIN_RAW: i16 = 300;
pub const STATIC_PUFF_ABOVE_WATER_VELOCITY_ARGUMENT_RAW: [i32; 3] = [0, 80, 0];
pub const STATIC_PUFF_UNDERWATER_VELOCITY_ARGUMENT_RAW: [i32; 3] = [0, 400, 0];
pub const STATIC_FLAME_VELOCITY_ARGUMENT_RAW: [i32; 3] = [0, 200, 0];

/// `FUN_00446640` places the controlled player's low-hull plume this many
/// signed 8.8 units behind the live body-forward Q31 vector.
pub const PLAYER_LOW_HULL_SMOKE_REAR_DISTANCE_RAW: i32 = 0x5a;
/// Vertical offset of the low-hull plume from the player entity center.
pub const PLAYER_LOW_HULL_SMOKE_Y_OFFSET_RAW: i16 = 0x14;

/// Recovered meteor-impact facts from `FUN_0040BAF0`/`FUN_00440950`.
pub const METEOR_IMPACT_SCATTER_COUNT: usize = 10;
pub const METEOR_IMPACT_SURFACE_COUNT: usize = 1;
pub const METEOR_IMPACT_PARTICLE_COUNT: usize =
    METEOR_IMPACT_SCATTER_COUNT + METEOR_IMPACT_SURFACE_COUNT;
pub const METEOR_IMPACT_SOUND_ID: usize = 0x3e;
pub const METEOR_IMPACT_RATE_MIN_16_16: u32 = 0x1_0000;
pub const METEOR_IMPACT_SOURCE_EXTENT_RAW: u16 = 0x100;
/// `FUN_004475F0` reuses `FUN_00440950` for player dying/contact bursts,
/// then submits the same cue a second time at a fixed 1.0 wrapper rate.
pub const PLAYER_WRECK_SOUND_ID: usize = METEOR_IMPACT_SOUND_ID;
pub const PLAYER_WRECK_FIXED_RATE_16_16: u32 = 0x1_0000;
pub const METEOR_SCATTER_CLASS: u8 = 0x10;
pub const METEOR_SCATTER_LIFETIME_TICKS: u16 =
    PARTICLE_DESCRIPTORS[METEOR_SCATTER_CLASS as usize].lifetime_ticks();
pub const METEOR_SCATTER_RADIUS_RAW: u16 =
    PARTICLE_DESCRIPTORS[METEOR_SCATTER_CLASS as usize].collision_radius_raw();
pub const METEOR_SCATTER_ANIMATION_RATE_RAW: u8 =
    PARTICLE_DESCRIPTORS[METEOR_SCATTER_CLASS as usize].animation_rate_raw();
pub const METEOR_SCATTER_DRAW_SCALE_RAW: u16 =
    PARTICLE_DESCRIPTORS[METEOR_SCATTER_CLASS as usize].draw_scale_raw();
pub const METEOR_SCATTER_SIZE_JITTER_DIVISOR_RAW: u8 =
    PARTICLE_DESCRIPTORS[METEOR_SCATTER_CLASS as usize].size_jitter_divisor_raw();
/// Smooth-unit equivalent of class-0x10 `FUN_0043ECD0` gravity.
///
/// Retail subtracts `i16((dt_us * 0x300000) >> 31)` from raw Y velocity after
/// integrating position. Converting that raw velocity scale to port world
/// units gives 5.45696821 units/s².
pub const METEOR_SCATTER_GRAVITY_WORLD_PER_SEC2: f32 = 5.456_968_3;
pub const METEOR_SURFACE_CLASS: u8 = 0x12;
pub const METEOR_SURFACE_LIFETIME_TICKS: u16 =
    PARTICLE_DESCRIPTORS[METEOR_SURFACE_CLASS as usize].lifetime_ticks();
pub const METEOR_SURFACE_RADIUS_RAW: u16 =
    PARTICLE_DESCRIPTORS[METEOR_SURFACE_CLASS as usize].collision_radius_raw();
pub const METEOR_SURFACE_ANIMATION_RATE_RAW: u8 =
    PARTICLE_DESCRIPTORS[METEOR_SURFACE_CLASS as usize].animation_rate_raw();
pub const METEOR_SURFACE_DRAW_SCALE_RAW: u16 =
    PARTICLE_DESCRIPTORS[METEOR_SURFACE_CLASS as usize].draw_scale_raw();
pub const METEOR_SURFACE_SIZE_JITTER_DIVISOR_RAW: u8 =
    PARTICLE_DESCRIPTORS[METEOR_SURFACE_CLASS as usize].size_jitter_divisor_raw();
pub const METEOR_TRAIL_CLASS: u8 = 0x1f;
pub const METEOR_TRAIL_LIFETIME_TICKS: u16 =
    PARTICLE_DESCRIPTORS[METEOR_TRAIL_CLASS as usize].lifetime_ticks();
pub const METEOR_TRAIL_RADIUS_RAW: u16 =
    PARTICLE_DESCRIPTORS[METEOR_TRAIL_CLASS as usize].collision_radius_raw();
pub const METEOR_TRAIL_ANIMATION_RATE_RAW: u8 =
    PARTICLE_DESCRIPTORS[METEOR_TRAIL_CLASS as usize].animation_rate_raw();
pub const METEOR_TRAIL_DRAW_SCALE_RAW: u16 =
    PARTICLE_DESCRIPTORS[METEOR_TRAIL_CLASS as usize].draw_scale_raw();
pub const METEOR_TRAIL_SIZE_JITTER_DIVISOR_RAW: u8 =
    PARTICLE_DESCRIPTORS[METEOR_TRAIL_CLASS as usize].size_jitter_divisor_raw();
pub const METEOR_TRAIL_INITIAL_VELOCITY_RAW: [i32; 3] = [0, 80, 0];
/// At or below the authored sea plane, `FUN_0043ECD0` substitutes class 42
/// for its ordinary above-sea class-31 trail.
pub const BALLISTIC_UNDERWATER_TRAIL_CLASS: u8 = PRIMARY_UNDERWATER_IMPACT_CLASS;
/// Stationary class deposited along a dying meteor's incoming path by
/// `FUN_00412DA0` -> `FUN_004061A0`.
pub const METEOR_WAKE_CLASS: u8 = 0x28;
pub const METEOR_WAKE_LIFETIME_TICKS: u16 =
    PARTICLE_DESCRIPTORS[METEOR_WAKE_CLASS as usize].lifetime_ticks();
pub const METEOR_WAKE_RADIUS_RAW: u16 =
    PARTICLE_DESCRIPTORS[METEOR_WAKE_CLASS as usize].collision_radius_raw();
pub const METEOR_WAKE_ANIMATION_RATE_RAW: u8 =
    PARTICLE_DESCRIPTORS[METEOR_WAKE_CLASS as usize].animation_rate_raw();
pub const METEOR_WAKE_DRAW_SCALE_RAW: u16 =
    PARTICLE_DESCRIPTORS[METEOR_WAKE_CLASS as usize].draw_scale_raw();
pub const METEOR_WAKE_SIZE_JITTER_DIVISOR_RAW: u8 =
    PARTICLE_DESCRIPTORS[METEOR_WAKE_CLASS as usize].size_jitter_divisor_raw();
pub const METEOR_WAKE_INITIAL_VELOCITY_RAW: [i32; 3] = [0; 3];
/// Smooth equivalent of strength-3, mass-5 drag applied after integration.
///
/// Retail computes `scale=(dt_us*3)/(5*8)` and then
/// `v -= (scale*v)>>15` independently for each raw velocity component.
pub const METEOR_TRAIL_DRAG_PER_SECOND: f32 = 2.288_818_4;

/// Shared layered explosion emitted by `FUN_00441200` for staged model
/// destruction. The first scatter uses class 30; the four independent debris
/// attempts select classes 22..=29 from the process-global RNG stream.
pub const COMMON_EXPLOSION_SCATTER_CLASS: u8 = 30;
pub const COMMON_EXPLOSION_SCATTER_COUNT: usize = 10;
pub const COMMON_EXPLOSION_DEBRIS_COUNT: usize = 4;
pub const COMMON_EXPLOSION_SOUND_ID: usize = 62;
pub const COMMON_EXPLOSION_TERRAIN_LIGHT_RADIUS_RAW: i32 = 0x600;
/// Static destruction event class 79. Kind 9 supplies it to both alternating
/// `FUN_004407D0` scatter slots; kind 29 submits one ordinary
/// `FUN_004410B0` event with the same descriptor.
const STATIC_DESTRUCTION_EFFECT_CLASS_79: u8 = 79;
const STATIC_KIND_9_SCATTER_COUNT: i32 = 2;
const STATIC_KIND_9_SCATTER_EXTENT_RAW: u16 = 0;
const STATIC_KIND_9_SCATTER_VELOCITY_SCALE_RAW: i32 = 1;
/// `_DAT_004CC5C0` is the signed raw Y lift used as the debris origin.
const COMMON_EXPLOSION_DEBRIS_Y_LIFT_RAW: i16 = 300;

#[derive(Debug, Clone, Copy)]
struct DirectionTableScatterEmission {
    base_count: i32,
    extent_raw: u16,
    source_classes: [u8; 2],
    velocity_scale_raw: i32,
    owner_id: Option<u32>,
    source_entity_type_at_birth: Option<u8>,
    suppresses_impact_damage: bool,
}

fn first_particle_frame(class: u8) -> ParticleFrame {
    particle_frame(class, 0).expect("retail particle class has an authored frame list")
}

const FUN_0043F590_HIT_VA: u32 = 0x0043_F590;
/// `FUN_0043F6E0` shares F610 then always calls `FUN_00410EB0` and sound 90.
const FUN_0043F6E0_HIT_VA: u32 = 0x0043_F6E0;
/// `FUN_0043F780` class-5 entity-hit slot. Entity `FUN_00411250` and static
/// `FUN_0043F920` / `FUN_00427DE0` are admitted.
const FUN_0043F780_HIT_VA: u32 = 0x0043_F780;
const FUN_0043F7C0_HIT_VA: u32 = 0x0043_F7C0;
/// `DAT_004D02C0` selector-6 particle class. `FUN_00441850` case 0.
const FUN_0043F6E0_CLASS_4: u8 = 4;
/// `DAT_004D02C0` selector-`0x13` particle class. `FUN_00441850` jump index 1.
const FUN_0043F6E0_CLASS_54: u8 = 54;
/// `FUN_00441850` class-4 debris class for `FUN_00441A50`.
const FUN_00441850_CLASS_4_DEBRIS_CLASS: u8 = 1;
/// `FUN_00441850` class-54 debris class for `FUN_004410B0`.
const FUN_00441850_CLASS_54_DEBRIS_CLASS: u8 = 0x27;
/// `FUN_004410B0` underwater substitute for class `0x27`.
const FUN_004410B0_CLASS_54_UNDERWATER_CLASS: u8 = 0x2e;
/// `0x13880 >> 10` count base for class-4 `FUN_00441A50`.
const FUN_00441A50_CLASS_4_COUNT_BASE: i32 = 0x13_880 >> 10;

/// The class-1/2/3 primary projectile family. `FUN_00440120`'s mode dispatch
/// routes on the full descriptor signature: class 87 shares only the
/// `FUN_0043F590` entity-hit slot. Classes 1 and 2 share the complete update,
/// surface, entity-hit, damage-packet and static-hit signature.
/// Gating on `+0x1C` alone admitted class 87 into this family's water/terrain
/// surface policy; gating on class 3's packet alone dropped default bullets.
fn is_supported_primary_projectile(source_class: u8) -> bool {
    particle_descriptor(source_class).is_some_and(|descriptor| {
        descriptor.raw_u32(0x1c) == FUN_0043F590_HIT_VA
            && descriptor.raw_u32(0x24) == BALLISTIC_STATIC_CALLBACK_VA
            && matches!(
                (source_class, descriptor.raw_u32(0x20)),
                (
                    PRIMARY_BULLET_PARTICLE_CLASS | RAPID_PRIMARY_PARTICLE_CLASS,
                    PRIMARY_CLASS1_DAMAGE_PACKET_VA
                ) | (
                    UPGRADED_PRIMARY_PARTICLE_CLASS,
                    PRIMARY_CLASS3_DAMAGE_PACKET_VA
                )
            )
    })
}

/// `FUN_00444FA0` emits these classes, including selector7's separate cure entry.
fn is_emitted_primary_projectile(source_class: u8) -> bool {
    particle_descriptor(source_class).is_some_and(|descriptor| {
        matches!(
            descriptor.raw_u32(0x1c),
            FUN_0043F590_HIT_VA | FUN_0043F6E0_HIT_VA | FUN_0043F780_HIT_VA | FUN_0043F7C0_HIT_VA
        )
    })
}

fn is_f6e0_primary_projectile(source_class: u8) -> bool {
    particle_descriptor(source_class)
        .is_some_and(|descriptor| descriptor.raw_u32(0x1c) == FUN_0043F6E0_HIT_VA)
}

fn is_f780_primary_projectile(source_class: u8) -> bool {
    particle_descriptor(source_class)
        .is_some_and(|descriptor| descriptor.raw_u32(0x1c) == FUN_0043F780_HIT_VA)
}

/// Class 5's `FUN_0043F780` entity-hit slot. Live `FUN_00411250` consumes this.
pub fn particle_uses_fun_0043f780_entity_hit(source_class: u8) -> bool {
    is_f780_primary_projectile(source_class)
}

/// Class6's separate 11320 cure wrapper, selected by its authored hit slot.
pub fn particle_uses_fun_0043f7c0_entity_hit(source_class: u8) -> bool {
    particle_descriptor(source_class)
        .is_some_and(|descriptor| descriptor.raw_u32(0x1c) == FUN_0043F7C0_HIT_VA)
}

/// Class-1/3 `FUN_0043F590` collision dispatch. Default bullets are class 1
/// (`+0x20 = 0x004CBF70`); upgraded primary is class 3 (`0x004CBF88`).
pub fn particle_uses_primary_collision_dispatch(source_class: u8) -> bool {
    is_supported_primary_projectile(source_class)
}

/// These values belong to `FUN_00440A20`'s *below-sea* fallback.  The captured
/// meteor hits above the sea plane and therefore does not emit these classes.
/// Keeping them named as underwater-only prevents sprites 790..792 from being
/// mistaken for the observed Intro2 impact again.
pub const UNDERWATER_IMPACT_CLASSES: [u8; 2] = [0x2d, 0x2e];
pub const UNDERWATER_IMPACT_LIFETIME_TICKS: u16 =
    PARTICLE_DESCRIPTORS[UNDERWATER_IMPACT_CLASSES[0] as usize].lifetime_ticks();
pub const UNDERWATER_IMPACT_COLLISION_RADII_RAW: [u16; 2] = [
    PARTICLE_DESCRIPTORS[UNDERWATER_IMPACT_CLASSES[0] as usize].collision_radius_raw(),
    PARTICLE_DESCRIPTORS[UNDERWATER_IMPACT_CLASSES[1] as usize].collision_radius_raw(),
];
pub const UNDERWATER_IMPACT_SPRITE_SEQUENCE: [u16; 4] = [790, 792, 791, 792];

/// Caller-selected BAF0 policy passed to shared 440950. Native callers
/// authenticate their component branches and solo network disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExplodeWithRingBurstRequest {
    pub position_raw: [i16; 3],
    pub source_extent_raw: u16,
    pub sea_level_raw: Option<i16>,
    pub logical_owner_entity_id: u32,
    pub logical_owner_entity_type: u8,
    pub scatter_count: i32,
    pub scatter_classes: [u8; 2],
    pub suppresses_impact_damage: bool,
}

/// Type61's class49 BAF0 policy alternates classes94/95 over sixteen attempts.
pub const TYPE61_EXPLODE_WITH_RING_SCATTER_COUNT: usize = 16;
pub const TYPE61_EXPLODE_WITH_RING_SCATTER_CLASSES: [u8; 2] = [0x5e, 0x5f];
pub const EXPLODE_WITH_RING_VELOCITY_SCALE_RAW: i32 = 8;

// Shared 100-entry signed direction table at retail address `DAT_004CD4B8`.
// `FUN_004407D0` and `FUN_00440DC0` advance one global cursor through it.
const RETAIL_DIRECTION_TABLE_RAW: [[i16; 3]; 100] = [
    [-356, -150, -1],
    [681, -488, -467],
    [592, -1000, 1164],
    [-551, 1526, 563],
    [372, -51, -478],
    [611, 395, 28],
    [891, 1707, 445],
    [1, 6, 6],
    [-765, 738, -21],
    [216, -724, -950],
    [-405, -770, -233],
    [509, 426, 1017],
    [-262, -983, 212],
    [602, -1021, 840],
    [-954, -485, 133],
    [-412, 111, -197],
    [-355, 1509, 164],
    [-1762, -40, 942],
    [142, 333, -390],
    [15, -40, 20],
    [430, 744, 1046],
    [1806, 101, 322],
    [-109, 1163, -128],
    [-429, -1238, 627],
    [19, 1429, 411],
    [-181, 495, -1823],
    [-646, 753, 669],
    [-40, -1212, -107],
    [44, 231, -190],
    [-1088, 727, -930],
    [-80, -94, 1497],
    [298, 1147, 566],
    [32, 544, 1289],
    [-17, 1518, 1140],
    [1035, -778, 493],
    [-73, 62, -102],
    [124, -279, -13],
    [-469, -1048, 190],
    [589, 1469, 404],
    [43, 835, 561],
    [308, 312, -345],
    [133, -1376, -609],
    [156, 38, -186],
    [14, -60, 32],
    [23, -728, -442],
    [1656, -576, 565],
    [82, 777, -528],
    [138, -519, 335],
    [-37, 1947, -266],
    [-45, -1131, -61],
    [97, -1674, 68],
    [-562, -10, -1689],
    [1140, -38, 98],
    [-375, -75, -60],
    [810, -940, -435],
    [109, -206, 78],
    [-25, -627, -9],
    [148, 655, -27],
    [-160, -1659, -75],
    [734, -880, -958],
    [1, 2, 1],
    [579, -1599, 112],
    [652, 734, 508],
    [685, 595, 30],
    [302, -1000, -298],
    [-362, 156, -85],
    [126, 845, -178],
    [-700, -500, -627],
    [-212, 851, 495],
    [614, -1743, -71],
    [-151, 305, -103],
    [1924, 527, -29],
    [-127, -133, -40],
    [17, -40, 86],
    [106, 440, -36],
    [255, 214, -226],
    [280, -549, -659],
    [-109, -732, -1031],
    [-532, -1099, 261],
    [439, -301, -528],
    [-70, -1107, -46],
    [-16, 377, 4],
    [120, -1108, -351],
    [-72, 5, -143],
    [649, -803, -687],
    [-56, -280, 82],
    [-290, -1346, 1194],
    [4, -85, -11],
    [-313, -92, 538],
    [25, 326, -116],
    [1091, -1247, 387],
    [-255, 540, 264],
    [1101, 593, 280],
    [726, -250, -461],
    [-79, 887, -269],
    [-1, 528, 370],
    [439, 1521, -473],
    [-1006, -1035, 130],
    [140, -87, 193],
    [223, -1023, 46],
];

/// An individual particle requested by a [`ParticleBurst`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleSpawn {
    /// Global Section-3 sprite id.
    pub sprite_id: u16,
    /// Retail particle-class byte.  This is useful when renderer behavior for
    /// more classes is recovered later.
    pub source_class: u8,
    /// Offset from the burst origin, in port world units.
    pub offset: [f32; 3],
    /// Constant presentation-only displacement from the simulated position.
    /// Collision, water classification, and callbacks never consume it.
    pub presentation_offset: [f32; 3],
    /// World-space velocity in port units per second.
    pub velocity: [f32; 3],
    /// Source entity retained for owner exclusion/damage attribution by
    /// projectile collision callbacks and for class-32's owner-motion lookup.
    pub owner_id: Option<u32>,
    /// Attached-follow owner from retail particle word `+0x14`
    /// (`FUN_00442950` writes the hit target handle). `None` means the
    /// particle is not attached; only `FUN_004425D0` classes consume this.
    pub attached_owner_handle: Option<u32>,
    /// Attached-follow offset from retail particle words `+0x0E/+0x10/+0x12`
    /// (impact-minus-target raw words, capability-gated at birth). Consumed
    /// only by the basis-rotated follow branch.
    pub attached_offset_raw: [i16; 3],
    /// Birth-time owner type copied into retail particle byte `+0x1C`.
    ///
    /// Emitters without proven immutable provenance retain `None` rather than
    /// resolving the owner's potentially changed current type at impact time.
    pub source_entity_type_at_birth: Option<u8>,
    /// Recovered predicate from retail particle byte `+0x1D` bit zero. It
    /// suppresses impact damage while preserving visuals and is inherited by
    /// `FUN_0043ECD0`/`FUN_0043F610` children.
    pub suppresses_impact_damage: bool,
    /// Lifetime expressed in original 50 Hz ticks.
    pub lifetime_ticks: u16,
    /// Original class collision-radius word. It is simulation metadata and
    /// must not be reused as the billboard's visual size.
    pub collision_radius_raw: u16,
    /// Raw presentation values from the selected retail frame tuple.
    pub frame_middle_raw: i16,
    pub frame_scale_raw: u16,
    pub animation_rate_raw: u8,
    /// Class-record draw scale passed to `FUN_0043D410`; retail multiplies it
    /// by `frame_scale_raw` when presenting the sprite.
    pub draw_scale_raw: u16,
    /// Class-record `+0x0C` size-jitter divisor. Presentation combines it with
    /// the stable physical slot's retail address phase.
    pub size_jitter_divisor_raw: u8,
}

/// Owner provenance resolved by `FUN_00440A60` when it follows the request's
/// entity handle and copies entity type byte `+0x58` into particle byte
/// `+0x1C`.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParticleOwnerAtBirth {
    pub entity_id: u32,
    pub entity_type: u8,
}

/// Descriptor-bearing request consumed by `FUN_00440A60`.
///
/// Target acquisition, target lead, and deferred-shot ordering remain
/// upstream. The owner type is deliberately tied to the owner handle because
/// retail derives it by lookup rather than accepting independent metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DescriptorParticleRequest {
    /// Authored descriptor selected by the upstream projectile method.
    pub source_class: u8,
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub owner: Option<ParticleOwnerAtBirth>,
    pub suppresses_impact_damage: bool,
}

/// Supported class-38 slice of the request consumed by `FUN_004410B0` then
/// `FUN_00440A60`. Method 10's table row selects class 38; at or below the
/// authored sea plane, `FUN_004410B0` substitutes class `0x2E` and subtracts
/// 100 from the 40A60 input. Class46's +0x28 bias adds those units back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Class38ParticleRequest {
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub owner_entity_id: u32,
    pub owner_entity_type: u8,
    pub suppresses_impact_damage: bool,
}

/// One successful class-38/`0x2E` `FUN_00440A60` birth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Class38ParticleBirth {
    pub slot: usize,
    pub particle_class: u8,
}

/// Process-global owner provenance consumed by callback child allocations:
/// `FUN_0043E3D0` -> `FUN_00441770` and class50's virus-carrier callbacks.
///
/// Retail reads the owner handle from `DAT_004DCA00` at callback time; it does
/// not copy particle `+0x18`. The port caller snapshots that current handle
/// together with the entity type which `FUN_00440A60` would resolve. `None`
/// represents the unresolved retail sentinel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ParticleEmitterContext {
    pub current_owner: Option<ParticleOwnerAtBirth>,
}

/// A group of particles which share one world-space origin.
#[derive(Debug, Clone, PartialEq)]
pub struct ParticleBurst {
    pub origin: [f32; 3],
    pub particles: Vec<ParticleSpawn>,
}

/// A one-shot sound tied to a world-space source.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionalSoundEvent {
    /// Global Section-11 sound-pool id.
    pub sound_id: usize,
    pub position: [f32; 3],
    /// Playback frequency in original unsigned 16.16 units. Callers own any
    /// random pitch selection before creating this logical sound record;
    /// later audible playback separately resolves Section-11 aliases.
    pub frequency_q16: u32,
}

/// One active entity's selected Section-8 collision model.
///
/// `radius_raw` comes from the active Section-8 model header at `+0x0A` and
/// remains in retail unsigned 8.8 units. `orientation_world_from_model` is the
/// entity's live local-to-world basis; retail stores the equivalent matrix as
/// signed Q31 and uses it to forward-transform authored primitive centers.
#[derive(Debug, Clone)]
pub struct EntityCollisionModel {
    pub entity_id: u32,
    pub center_world: [f32; 3],
    pub radius_raw: u16,
    pub model_id: usize,
    pub orientation_world_from_model: [[f32; 3]; 3],
    pub anim_vars: AnimVars,
    /// Live state word at `+0x08` for state-gated callbacks (attached-follow
    /// branch selection). `None` when the projection could not resolve it.
    pub state_flags_at_0x08: Option<u32>,
    /// Type capability word used by 442950's post-damage attachment offset.
    pub capability_flags_at_0x64: u32,
}

/// Live-entity collision-cache state published by an inline F590 handler
/// before retail advances to the next physical particle slot.
///
/// Retail re-scans the active entity list for every slot. The port starts from
/// an owned frame snapshot to avoid aliasing the entity manager during damage
/// callbacks, then replaces it at the same mutation point. Replacing the whole
/// ordered list preserves side effects beyond the direct target and avoids
/// inventing per-target mutation equivalence.
#[derive(Debug, Clone)]
pub enum ParticleCollisionCacheRefresh {
    Unchanged,
    Replace(Vec<EntityCollisionModel>),
}

/// Level-authored environment data required by class-1 collision dispatch.
#[derive(Debug, Clone, Copy)]
pub struct TerrainCollisionContext<'a> {
    pub terrain: &'a TerrainGrid,
    /// Active biome's Section-9 static terrain-object descriptor table.
    /// Missing tables disable only the swept static-object stage.
    pub terrain_objects: Option<&'a TerrainObjectTable>,
    pub ground_response_selectors: [u8; 8],
    pub water_response_selectors: [u8; 8],
}

impl<'a> TerrainCollisionContext<'a> {
    /// Build gameplay collision context without crossing current-level
    /// Section-10/Section-13 ownership boundaries.
    ///
    /// Section 9 is a layered biome resource and therefore follows the active
    /// cache table. Terrain and its response selectors are level-local: either
    /// missing section makes the complete context unavailable rather than
    /// falling back to an unrelated auxiliary or PRELOAD allocation.
    pub fn from_current_level_cache(cache: &'a ResourceCache) -> Option<Self> {
        let level = cache.level()?;
        let terrain = cache.level_terrain()?;
        let descriptor = level.level.as_ref()?;
        Some(Self {
            terrain,
            terrain_objects: cache.terrain_objects(),
            ground_response_selectors: descriptor.ground_response_selectors(),
            water_response_selectors: descriptor.water_response_selectors(),
        })
    }
}

/// Environment consulted by one retail physical-particle traversal.
///
/// Flat water is retained as an explicit test/inspection mode. Gameplay uses
/// [`Self::Terrain`], which classifies water from the authored tile and the
/// displaced wave surface instead of treating the sea plane as globally wet.
#[derive(Debug, Clone, Copy)]
pub enum ParticleEnvironment<'a> {
    Dry,
    FlatWater { sea_level: f32 },
    Terrain(TerrainCollisionContext<'a>),
}

impl<'a> ParticleEnvironment<'a> {
    fn terrain_context(self) -> Option<TerrainCollisionContext<'a>> {
        match self {
            Self::Terrain(context) => Some(context),
            Self::Dry | Self::FlatWater { .. } => None,
        }
    }

    fn sea_level(self) -> Option<f32> {
        match self {
            Self::Dry => None,
            Self::FlatWater { sea_level } => Some(sea_level),
            Self::Terrain(context) => context
                .terrain
                .water_enabled()
                .then(|| context.terrain.sea_level_world_y()),
        }
    }
}

/// One entity handle and the live velocity consumed by class 32's recovered
/// `FUN_00442920` callback after generic integration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleOwnerMotion {
    pub owner_id: u32,
    pub velocity: [f32; 3],
}

/// Shared swept-particle collision inputs which are not part of the authored
/// environment.
#[derive(Clone, Copy)]
pub struct ParticleCollisionContext<'a> {
    pub entities: &'a [EntityCollisionModel],
    pub model_pool: &'a dyn CollisionModelPool,
}

/// Entity-backed callbacks available during one physical-slot traversal.
#[derive(Clone, Copy, Default)]
pub struct ParticleCallbackContext<'a> {
    pub owner_motions: &'a [ParticleOwnerMotion],
    pub collision: Option<ParticleCollisionContext<'a>>,
}

fn apply_particle_collision_cache_refresh(
    collision_entities: &mut Option<Cow<'_, [EntityCollisionModel]>>,
    refresh: ParticleCollisionCacheRefresh,
) {
    let Some(entities) = collision_entities.as_mut() else {
        return;
    };
    match refresh {
        ParticleCollisionCacheRefresh::Unchanged => {}
        ParticleCollisionCacheRefresh::Replace(replacement) => {
            *entities = Cow::Owned(replacement);
        }
    }
}

/// Complete input to one `FUN_00440120`-equivalent traversal.
#[derive(Clone, Copy)]
pub struct ParticleUpdateRequest<'a> {
    pub elapsed_micros: u32,
    pub retail_tick: u32,
    pub environment: ParticleEnvironment<'a>,
    pub callbacks: ParticleCallbackContext<'a>,
    /// Snapshot of the process-global allocator owner for callback children.
    pub particle_emitter: ParticleEmitterContext,
}

impl<'a> ParticleUpdateRequest<'a> {
    pub fn new(
        elapsed_micros: u32,
        retail_tick: u32,
        environment: ParticleEnvironment<'a>,
    ) -> Self {
        Self {
            elapsed_micros,
            retail_tick,
            environment,
            callbacks: ParticleCallbackContext::default(),
            particle_emitter: ParticleEmitterContext::default(),
        }
    }

    pub fn dry(elapsed_micros: u32, retail_tick: u32) -> Self {
        Self::new(elapsed_micros, retail_tick, ParticleEnvironment::Dry)
    }

    pub fn flat_water(elapsed_micros: u32, retail_tick: u32, sea_level: f32) -> Self {
        Self::new(
            elapsed_micros,
            retail_tick,
            ParticleEnvironment::FlatWater { sea_level },
        )
    }

    pub fn terrain(
        elapsed_micros: u32,
        retail_tick: u32,
        context: TerrainCollisionContext<'a>,
    ) -> Self {
        Self::new(
            elapsed_micros,
            retail_tick,
            ParticleEnvironment::Terrain(context),
        )
    }

    pub fn with_callbacks(mut self, callbacks: ParticleCallbackContext<'a>) -> Self {
        self.callbacks = callbacks;
        self
    }

    pub fn with_particle_emitter(mut self, emitter: ParticleEmitterContext) -> Self {
        self.particle_emitter = emitter;
        self
    }
}

/// Exact Section-10 writes shared by physical particle callbacks. The existing
/// infection writers own bit 0x10; 27950's null static program owns bit 0x08.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleTerrainMutation {
    Infection { cell: [u8; 2], infected: bool },
    ImmediateBurn { cell: [u8; 2] },
}

impl ParticleTerrainMutation {
    pub(crate) fn cell(self) -> [u8; 2] {
        match self {
            Self::Infection { cell, .. } | Self::ImmediateBurn { cell } => cell,
        }
    }

    pub(crate) fn apply(self, terrain_type: u8) -> u8 {
        match self {
            Self::Infection { infected: true, .. } => terrain_type | INFECTION_TERRAIN_TYPE_BIT,
            Self::Infection {
                infected: false, ..
            } => terrain_type & !INFECTION_TERRAIN_TYPE_BIT,
            Self::ImmediateBurn { .. } => {
                terrain_type | crate::static_damage::BURNED_TERRAIN_TYPE_BIT
            }
        }
    }
}

/// Observable traversal trace and deferred terrain writes in physical-slot order.
#[derive(Debug, Default, PartialEq)]
pub struct ParticleUpdateOutcome {
    /// Completed 425D0 callbacks whose checked-damage admission bit was clear.
    /// These are retrospective observations, never requests to replay.
    pub attached_updates: Vec<AttachedParticleUpdateObservation>,
    /// Per-particle boundaries. These particles were retired after retaining
    /// their committed prefixes; their missing damage must not count as applied.
    pub blocked_attached_updates: Vec<AttachedParticleUpdateDiagnostic>,
    /// Entity callbacks reached through the shared `FUN_0043F590` path, in
    /// exact physical-slot order. The corresponding live handler has already
    /// run when this retrospective trace is returned.
    pub entity_impacts: Vec<ParticleEntityImpact>,
    /// Primary collision observations. Static entries have already passed
    /// through the live F800 handler and must not be replayed.
    pub primary_impacts: Vec<PrimaryImpact>,
    /// Retrospective observations of ballistic F800 callbacks. The live
    /// handler has already run; callers must never replay these packets.
    pub ballistic_static_impacts: Vec<ParticleStaticImpact>,
    /// Class 87's static callback is a true no-op, but retaining its exact
    /// ordered hit makes that consumed-parent decision observable.
    pub class_87_static_impacts: Vec<Class87StaticTileImpact>,
    /// Ordered terrain-type writes requested by particle callbacks.
    ///
    /// Live Playing/Intro2 hosts commit these synchronously through
    /// [`ParticleTraversalHost::publish_terrain_mutations`]; their returned
    /// journal is history and must not be replayed over later callback writes.
    /// Borrowed update contexts instead retain an ordered collision overlay
    /// and require their caller to apply this journal after traversal. Both
    /// preserve opposite class-5/class-6 contacts and F800's immediate burn
    /// before the next slot's static-model lookup, without copying the grid.
    pub terrain_type_mutations: Vec<ParticleTerrainMutation>,
    /// Surface programs not accepted by an owning world host. They remain
    /// explicit evidence boundaries and must not silently be treated as applied.
    pub unhandled_ground_programs: Vec<CombatGroundProgramRequest>,
}

#[derive(Clone, Copy)]
pub struct ParticleBirthContext<'a> {
    ///440A60 stores the current terrain/wave classification at allocation.
    pub environment: ParticleEnvironment<'a>,
    pub retail_tick: u32,
}

enum PrimaryImpactEffect {
    /// Shared `FUN_0043F610` burst used by entity and solid static-model hits.
    HitBurst([f32; 3]),
    Surface {
        sound_position: [f32; 3],
        effect_position: [f32; 3],
        response_selector: u8,
    },
    /// Class 3's selector-6/7 `FUN_00441770` child. The helper chooses class
    /// 43 at/below the sea plane and class 75 above it, with raw Y velocity
    /// 200. Unlike class 1's surface callback, this path plays no sound.
    UpgradedSurfaceBurst([f32; 3]),
}

struct PrimaryCollisionDispatch {
    keep_particle: bool,
    entity_impact: Option<ParticleEntityImpact>,
    impacts: Vec<PrimaryImpact>,
    effects: Vec<PrimaryImpactEffect>,
}

/// Complete argument view published by the shared entity-hit callback
/// `FUN_0043F590`.
///
/// The live update hook receives this after `FUN_0043F610` has materialized
/// its visual and before the outer particle traversal frees the parent slot.
/// `source_particle_class` is provenance only; callback policy must continue
/// to come from the decoded descriptor rather than numeric class dispatch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleEntityImpact {
    pub source_particle_class: u8,
    /// Exact opaque `particle + 8` argument passed to `FUN_00410EB0`.
    pub impact_position_argument_va: u32,
    pub target_entity_id: u32,
    /// Integrated endpoint stored at `particle + 8`, not the refined probe.
    pub position_world: [f32; 3],
    /// Live signed velocity words passed directly as `FUN_00410EB0`'s fourth
    /// argument after the class update callback has applied drag/gravity.
    pub velocity_raw: [i16; 3],
    /// `None` means live particle byte `+0x1D` bit zero suppressed only the
    /// damage delivery; the F610 visual and parent deletion still occurred.
    pub damage: Option<BallisticDamageRequest>,
}

impl ParticleEntityImpact {
    pub fn entity_hit_entry(self) -> crate::damage::EntityHitEntry {
        if particle_uses_fun_0043f7c0_entity_hit(self.source_particle_class) {
            crate::damage::EntityHitEntry::Cured
        } else if particle_uses_fun_0043f780_entity_hit(self.source_particle_class) {
            crate::damage::EntityHitEntry::Infected
        } else {
            crate::damage::EntityHitEntry::PrimaryProjectile
        }
    }
    /// Complete packet selected by the actual entity-hit callback. F780/F7C0
    /// select static004CBFD0/004CBFE8 with zero trailing words; the other
    /// supported callbacks copy immutable particle birth provenance instead.
    pub fn damage_delivery_record(self) -> Option<DamageDeliveryRecord> {
        let damage = self.damage?;
        if particle_uses_fun_0043f780_entity_hit(self.source_particle_class) {
            let descriptor = particle_descriptor(self.source_particle_class)?;
            (descriptor.raw_u32(0x20) == 0x004C_BFD0 && damage.packet == FUN_0043F780_DAMAGE_PACKET)
                .then_some(crate::damage::FUN_0043F780_DAMAGE_DELIVERY)
        } else if particle_uses_fun_0043f7c0_entity_hit(self.source_particle_class) {
            let descriptor = particle_descriptor(self.source_particle_class)?;
            (descriptor.raw_u32(0x20) == 0x004C_BFE8 && damage.packet == FUN_0043F7C0_DAMAGE_PACKET)
                .then_some(crate::damage::FUN_0043F7C0_DAMAGE_DELIVERY)
        } else {
            damage.delivery_record()
        }
    }
}

/// One primary or ballistic static-object callback at `FUN_0043F800`.
///
/// The collision fields retain the authored model that produced the refined
/// hit. `current` is a separate post-F610 re-read of the same Section-10 cell,
/// matching `FUN_00427950`; it is never rejected merely because the collision
/// snapshot changed. `None` means suppression skipped the re-read or the live
/// target could not be resolved faithfully.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleStaticImpact {
    pub source_particle_class: u8,
    pub source_owner_id: Option<u32>,
    /// Refined signed-8.8 sweep probe, converted back into port world space.
    pub position_world: [f32; 3],
    pub cell: [u8; 2],
    pub attribute: u8,
    pub terrain_type: u8,
    pub model_id: u16,
    pub kind_index: u32,
    /// Live target reacquired after the F610 visual and suppression re-read.
    pub current: Option<CurrentStaticDamageSnapshot>,
    /// Descriptor-selected packet and immutable birth provenance. F800
    /// re-reads runtime byte `+0x1D` after F610, so `None` suppresses only this
    /// delivery while retaining the visual and parent deletion.
    pub damage: Option<BallisticDamageRequest>,
}

/// Observable result of class-1 collision dispatch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrimaryImpact {
    StaticTile(ParticleStaticImpact),
    WaterEntry {
        projectile_owner_id: u32,
        position_world: [f32; 3],
        surface_y: f32,
        material_code: u8,
        response_selector: u8,
    },
    Terrain {
        projectile_owner_id: u32,
        position_world: [f32; 3],
        surface_y: f32,
        /// Low three bits authored in the nearest terrain cell.
        material_code: u8,
        /// Level-authored response selected through Section 13.
        response_selector: u8,
    },
}

/// Provenance-bearing damage delivery selected by the shared ballistic
/// particle callbacks.
///
/// Retail copied the source type into particle byte `+0x1C` at birth. Emitters
/// without that immutable value retain `None` rather than looking up the
/// owner's potentially changed current runtime type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BallisticDamageRequest {
    pub packet: DamagePacket,
    pub source_entity_type_at_birth: Option<u8>,
    pub source_owner_id: Option<u32>,
}

impl BallisticDamageRequest {
    /// Materialize the complete six-dword retail delivery record.
    ///
    /// Missing birth provenance remains a hard boundary: neither field may be
    /// reconstructed from the owner's mutable impact-time state.
    pub fn delivery_record(self) -> Option<DamageDeliveryRecord> {
        Some(DamageDeliveryRecord {
            packet: self.packet,
            source_entity_type_raw: u32::from(self.source_entity_type_at_birth?),
            owner_handle: self.source_owner_id?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum BallisticSweepImpact {
    Entity(ParticleEntityImpact),
    StaticTile(ParticleStaticImpact),
    /// `FUN_0042E8E0` static no-op (static-route classes 52/68): consume the
    /// parent with no visual, damage, or static program. Position retained
    /// for diagnostics.
    ConsumedStaticNoOp {
        position_world: [f32; 3],
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Class87StaticTileImpact {
    /// Refined signed-8.8 probe published by `FUN_0043FF10`.
    pub position_world: [f32; 3],
    pub cell: [u8; 2],
    pub attribute: u8,
    pub terrain_type: u8,
    pub model_id: u16,
    pub kind_index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Class87SweepImpact {
    Entity(ParticleEntityImpact),
    StaticTile(Class87StaticTileImpact),
}

impl PositionalSoundEvent {
    pub fn fixed(sound_id: usize, position: [f32; 3]) -> Self {
        Self {
            sound_id,
            position,
            frequency_q16: 0x1_0000,
        }
    }
}

/// Backend-independent events accepted by [`WorldFx`].
#[derive(Debug, Clone, PartialEq)]
pub enum WorldEvent {
    ParticleBurst(ParticleBurst),
    ///443B50's complete transfer packet. Playing's early controller adapter
    /// submits it before the particle pass, although the retail entity
    /// callback allocates after that pass. Materialization has an explicit
    /// end-of-pass boundary and consumes no RNG for descriptor33.
    DeferredCargoTransfer {
        position_raw: [i16; 3],
        parent_id: u32,
        parent_entity_type: u8,
    },
    /// `FUN_00440950` has asymmetric failure semantics: class-16 scatter
    /// allocation stops at its first rejection, but class 18 is still tried.
    MeteorImpact {
        position: [f32; 3],
        first_direction_index: usize,
        owner_id: Option<u32>,
    },
    PositionalSound(PositionalSoundEvent),
}

/// A live particle after its burst event has been materialized.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldParticle {
    pub sprite_id: u16,
    pub source_class: u8,
    pub position: [f32; 3],
    /// Constant visual displacement retained while the simulated particle
    /// follows its retail center-origin trajectory.
    pub presentation_offset: [f32; 3],
    /// Position at the beginning of the most recent generic integration step.
    /// Class-1 uses this to reproduce the retail swept collision query instead
    /// of tunnelling through a target between render frames.
    pub step_start_position: [f32; 3],
    /// Retail water-sphere classification before and after the latest step:
    /// 0 fully submerged, 1 intersecting the surface, 2 above/non-water.
    pub step_start_water_state: u8,
    pub water_state: u8,
    /// `FUN_00440A60` classifies a newborn immediately. Host-side queued
    /// materialization can occur without a world context, so this explicit bit
    /// defers only that first classification until the next known environment.
    water_state_initialized: bool,
    pub velocity: [f32; 3],
    pub owner_id: Option<u32>,
    pub attached_owner_handle: Option<u32>,
    pub attached_offset_raw: [i16; 3],
    pub source_entity_type_at_birth: Option<u8>,
    pub suppresses_impact_damage: bool,
    /// Particle+1D bit80: consumed by the next common traversal precheck.
    pub pending_destruction: bool,
    pub age_ticks: f32,
    pub lifetime_ticks: u16,
    pub collision_radius_raw: u16,
    pub frame_middle_raw: i16,
    pub frame_scale_raw: u16,
    pub animation_rate_raw: u8,
    pub draw_scale_raw: u16,
    pub size_jitter_divisor_raw: u8,
}

/// Live type-60 `Exploding Ring` presentation.
///
/// Retail publishes component control output one to the model callback, starts
/// it at `0xffff`, and deletes the entity when `FUN_00406DC0` exhausts it. The
/// authored `splashseg`/`splashsegmid` type-8 vertices turn that descending
/// word into the expanding, settling water ring. Every record is owned by its
/// real Type-60 entity; the actor task is the sole lifecycle authority.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExplodingRing {
    pub model_id: usize,
    pub position: [f32; 3],
    associated_entity_id: u32,
    control_output_1_raw: u16,
}

impl ExplodingRing {
    pub fn associated_entity_id(&self) -> u32 {
        self.associated_entity_id
    }

    pub fn control_output_1_raw(&self) -> u16 {
        self.control_output_1_raw
    }

    pub fn anim_vars(&self) -> AnimVars {
        let mut vars = AnimVars::default();
        vars.dynamic[1] = i32::from(self.control_output_1_raw);
        vars
    }
}

/// One post-visibility terrain-light contribution from `FUN_0043D410`.
///
/// The center and radius retain retail's signed 8.8 integer domain so the
/// renderer can feed them directly to `FUN_004385E0`'s reconstructed radial
/// writer without another float conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParticleTerrainLight {
    pub x_raw: i16,
    pub z_raw: i16,
    pub radius_raw: i32,
}

/// Immediate terrain-light write requested by `FUN_00441200` before any of
/// its particle allocations or random draws.
///
/// The renderer owns the mutable 32x32 light window, so [`WorldFx`] returns
/// this signed-8.8 command to its caller rather than keeping a second terrain
/// lighting buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainExplosionLight {
    pub x_raw: i16,
    pub z_raw: i16,
    pub radius_raw: i32,
}

impl WorldParticle {
    pub fn presentation_position(&self) -> [f32; 3] {
        [
            v2k_core::world::wrap(self.position[0] + self.presentation_offset[0]),
            self.position[1] + self.presentation_offset[1],
            v2k_core::world::wrap(self.position[2] + self.presentation_offset[2]),
        ]
    }

    fn retail_age_byte(&self) -> u8 {
        // Retail stores age at particle +0x1B. Narrow only after restoring the
        // integral tick so a hypothetical age 256 wraps to zero instead of
        // saturating at 255 as a direct float-to-u8 cast would.
        (self.age_ticks.max(0.0).floor() as u32 & 0xff) as u8
    }

    fn descriptor_current_frame(&self) -> Option<ParticleFrame> {
        let descriptor = particle_descriptor(self.source_class)?;
        let frame_count = usize::from(descriptor.frame_count());
        if frame_count == 0 || descriptor.frame_list_va() == 0 {
            return None;
        }
        let age_tick = usize::from(self.retail_age_byte());
        let frame_index =
            ((usize::from(descriptor.animation_rate_raw()) * age_tick) >> 6) % frame_count;
        particle_frame(self.source_class, frame_index)
    }

    /// Resolve the retail animation frame for the current floored 50 Hz age.
    ///
    /// `FUN_0043D410` computes
    /// `((animation_rate_raw * age_tick) >> 6) % frame_count`. Particles are
    /// initialized at age zero by `FUN_00440A60`, so no random frame phase is
    /// involved. An unresolved class keeps the explicit frame tuple stored on
    /// the particle.
    pub fn current_frame(&self) -> ParticleFrame {
        if let Some(frame) = self.descriptor_current_frame() {
            return frame;
        }

        ParticleFrame {
            sprite_id: self.sprite_id,
            middle_raw: self.frame_middle_raw,
            scale_raw: self.frame_scale_raw,
        }
    }

    pub fn current_sprite_id(&self) -> u16 {
        self.current_frame().sprite_id
    }

    /// Resolve the ordinary post-projection terrain light from `FUN_0043D410`.
    ///
    /// This is deliberately a pure calculation. `FUN_0043D410` performs its
    /// CPU projection/coarse-cull gate before reaching these rules; that gate
    /// is not reconstructed here and callers must not treat this method as a
    /// visibility test. Likewise, `WorldParticle` does not yet represent the
    /// unresolved flag-`0x20` fixed reference-height branch. The common retail
    /// traversal clears flag `0x10` before presentation, so every currently
    /// implemented ordinary particle uses the signed current-cell terrain
    /// height reproduced below.
    pub fn terrain_light_emission(&self, terrain: &TerrainGrid) -> Option<ParticleTerrainLight> {
        // A null descriptor frame pointer makes retail free the particle before
        // lighting. Do not use `current_frame`'s presentation-only fallback.
        let frame = self.descriptor_current_frame()?;
        if frame.middle_raw == 0 {
            return None;
        }

        // D410 consumes the simulated particle words at +8/+A/+C, not any
        // port-only presentation displacement applied to its sprite.
        let position_raw = world_position_to_raw(self.position);
        let cell_x = usize::from((position_raw[0] as u16) >> 8);
        let cell_z = usize::from((position_raw[2] as u16) >> 8);
        let ground_raw = terrain
            .cell(cell_x, cell_z)
            .map(|cell| i16::from(cell.height as i8) << 5)?;
        let radius_raw = particle_terrain_light_radius_raw(
            frame.middle_raw,
            self.retail_age_byte(),
            position_raw[1],
            ground_raw,
        )?;
        Some(ParticleTerrainLight {
            x_raw: position_raw[0],
            z_raw: position_raw[2],
            radius_raw,
        })
    }

    /// Age in `[0, 1]`, useful to presentation code without prescribing a
    /// particular fade or scale curve.
    pub fn normalized_age(&self) -> f32 {
        if self.lifetime_ticks == 0 {
            1.0
        } else {
            (self.age_ticks / f32::from(self.lifetime_ticks)).clamp(0.0, 1.0)
        }
    }

    pub fn is_alive(&self) -> bool {
        self.lifetime_ticks != 0
            && self.age_ticks.max(0.0).floor() <= f32::from(self.lifetime_ticks)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ParticleList {
    head: Option<usize>,
    tail: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
struct ParticleLinks {
    previous: Option<usize>,
    next: Option<usize>,
    list: u8,
}

impl ParticleLinks {
    const EMPTY: Self = Self {
        previous: None,
        next: None,
        list: 0,
    };
}

/// One particle together with its stable physical retail-pool slot.
///
/// Slot identity is presentation data in V2000: `FUN_0043D410` derives a
/// repeating size phase from the particle record's address.
#[derive(Debug, Clone, Copy)]
pub struct PresentedParticle<'a> {
    pub slot: usize,
    pub particle: &'a WorldParticle,
}

impl PresentedParticle<'_> {
    /// Class draw scale after retail's physical-slot variation term.
    pub fn effective_draw_scale_raw(self) -> i32 {
        effective_particle_draw_scale_raw(self.slot, self.particle)
    }
}

/// Result of `FUN_0043D410`'s coarse center gate for one live particle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticlePreCull {
    /// Continue to frame selection and terrain lighting. Sprite visibility is
    /// decided later because a centered particle beyond the far word still
    /// reaches the retail light writer.
    Active,
    /// Descriptor flag `0x02` keeps a hard-rejected particle in its list while
    /// suppressing both its light and sprite for this traversal.
    RetainHidden,
    /// Ordinary hard rejection sets particle flag `0x80`, freeing the slot.
    Delete,
}

/// Address used by D410's physical-record size phase. A copied draw record
/// has its own stack address; it must never borrow its source pool slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleDrawRecordAddress {
    LivePoolSlot,
    Stack {
        callback_va: u32,
        record_address: Option<u32>,
    },
}

/// Captured or controlled-oracle stack addresses for the two copied-record
/// callbacks. A host stack pointer is not an authority for the retail stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ParticlePresentationStackAddresses {
    pub forward_streak_record: Option<u32>,
    pub backward_streak_record: Option<u32>,
}

/// The source scale argument is owned, but D410 also reads this stack
/// record's pointer phase. No accepted observation has supplied that phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParticleStackSizeJitterBoundary {
    pub source_class: u8,
    pub callback_va: u32,
    pub sample_scale_raw: i32,
    pub divisor_raw: i8,
}

/// One owned survivor from the frame's destructive presentation traversal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreparedParticle {
    pub slot: usize,
    pub particle: WorldParticle,
    pub projection: v2k_render::ParticleCenterProjection,
    pub draw_record_address: ParticleDrawRecordAddress,
}

impl PreparedParticle {
    /// Exact D410 size including its record-address jitter when that address
    /// is owned. Stack copies with a zero divisor need no address authority.
    pub fn native_effective_draw_scale_raw(self) -> Result<i32, ParticleStackSizeJitterBoundary> {
        let base = i32::from(self.particle.draw_scale_raw);
        let divisor = self.particle.size_jitter_divisor_raw as i8;
        if divisor == 0 {
            return Ok(base);
        }
        let phase = match self.draw_record_address {
            ParticleDrawRecordAddress::LivePoolSlot => {
                ((retail_particle_slot_va(self.slot) >> 5) & 0x0f) as i32
            }
            ParticleDrawRecordAddress::Stack {
                record_address: Some(address),
                ..
            } => ((address >> 5) & 0x0f) as i32,
            ParticleDrawRecordAddress::Stack {
                callback_va,
                record_address: None,
            } => {
                return Err(ParticleStackSizeJitterBoundary {
                    source_class: self.particle.source_class,
                    callback_va,
                    sample_scale_raw: base,
                    divisor_raw: divisor,
                });
            }
        };
        Ok(base + phase * base / i32::from(divisor))
    }

    /// Compatibility rendering scale. An unowned stack phase uses only the
    /// proven source scale argument; this is an explicit presentation bound,
    /// not an assertion of retail-matched size. Fidelity checks must consume
    /// native_effective_draw_scale_raw and report its error.
    pub fn effective_draw_scale_raw(self) -> i32 {
        self.native_effective_draw_scale_raw()
            .unwrap_or(i32::from(self.particle.draw_scale_raw))
    }
}

/// Stable output of one retail particle-presentation traversal.
///
/// Both the terrain-light phase and later sprite phase consume this same owned
/// list. That prevents a destructive cull from being evaluated twice around
/// terrain drawing and preserves the unusual centered-beyond-far light case.
#[derive(Debug, Clone, PartialEq)]
pub struct ParticlePresentationFrame {
    particles: Vec<PreparedParticle>,
    viewport: [u32; 2],
    far_depth_raw: i32,
}

impl ParticlePresentationFrame {
    pub fn particles(&self) -> impl ExactSizeIterator<Item = &PreparedParticle> {
        self.particles.iter()
    }

    /// Reproject the exact draw records admitted by the live traversal for a
    /// frozen redraw. This preserves their ordering, copied streak positions,
    /// scale and address-dependent jitter; it does not repeat lifecycle culling
    /// or admit any later births from the live particle pool.
    pub fn reprojected(
        &self,
        viewport: [u32; 2],
        mut project: impl FnMut(&WorldParticle) -> v2k_render::ParticleCenterProjection,
    ) -> Self {
        Self {
            particles: self
                .particles
                .iter()
                .map(|prepared| PreparedParticle {
                    projection: project(&prepared.particle),
                    ..*prepared
                })
                .collect(),
            viewport,
            far_depth_raw: self.far_depth_raw,
        }
    }

    pub fn sprite_visible(&self, particle: &PreparedParticle, half_extent: [i32; 2]) -> bool {
        d410_sprite_visible(
            particle.projection,
            self.viewport,
            self.far_depth_raw,
            half_extent,
        )
    }
}

fn effective_particle_draw_scale_raw(slot: usize, particle: &WorldParticle) -> i32 {
    let base = i32::from(particle.draw_scale_raw);
    let divisor = i32::from(particle.size_jitter_divisor_raw as i8);
    if divisor == 0 {
        return base;
    }
    let address = retail_particle_slot_va(slot) as usize;
    let phase = ((address >> 5) & 0x0f) as i32;
    base + phase * base / divisor
}

fn retail_particle_slot_va(slot: usize) -> u32 {
    debug_assert!(slot < MAX_WORLD_PARTICLES);
    (RETAIL_PARTICLE_POOL_VA + slot * RETAIL_PARTICLE_RECORD_BYTES) as u32
}

fn retail_particle_impact_position_argument_va(slot: usize) -> u32 {
    retail_particle_slot_va(slot) + 8
}

/// Exact lifecycle policy at `FUN_0043D410:31512-31523`, operating on an
/// already projected center.
pub fn d410_pre_cull(
    projection: v2k_render::ParticleCenterProjection,
    viewport: [u32; 2],
    far_depth_raw: i32,
    descriptor_flags: u8,
) -> ParticlePreCull {
    const OUTSIDE_OR_BEHIND: u8 = 0x6d;
    if projection.clip & OUTSIDE_OR_BEHIND == 0 {
        return ParticlePreCull::Active;
    }

    let width = viewport[0] as i32;
    let height = viewport[1] as i32;
    let extreme_x =
        width.wrapping_mul(2) < projection.screen[0].wrapping_add(width >> 1).wrapping_abs();
    let extreme_y = height.wrapping_mul(2)
        < projection.screen[1]
            .wrapping_add(height >> 1)
            .wrapping_abs();
    let hard_reject = projection.clip & 0x40 != 0
        || projection.depth_raw >= far_depth_raw
        || extreme_x
        || extreme_y;
    if !hard_reject {
        ParticlePreCull::Active
    } else if descriptor_flags & 0x02 != 0 {
        ParticlePreCull::RetainHidden
    } else {
        ParticlePreCull::Delete
    }
}

/// Exact depth and expanded-rectangle test at
/// `FUN_0043D410:31558,31577-31580`.
pub fn d410_sprite_visible(
    projection: v2k_render::ParticleCenterProjection,
    viewport: [u32; 2],
    far_depth_raw: i32,
    half_extent: [i32; 2],
) -> bool {
    const OUTSIDE_OR_BEHIND: u8 = 0x6d;
    if projection.depth_raw >= far_depth_raw {
        return false;
    }
    if projection.clip & OUTSIDE_OR_BEHIND == 0 {
        return true;
    }

    let x_lhs = projection.screen[0].wrapping_add(half_extent[0]) as u32;
    let x_rhs = (viewport[0] as i32).wrapping_add(half_extent[0].wrapping_mul(2)) as u32;
    let y_lhs = projection.screen[1].wrapping_add(half_extent[1]) as u32;
    let y_rhs = (viewport[1] as i32).wrapping_add(half_extent[1].wrapping_mul(2)) as u32;
    x_lhs < x_rhs && y_lhs < y_rhs
}

/// Immutable retail presentation traversal: priorities 1..=6, then each
/// intrusive list from its newest head to its oldest tail.
pub struct ParticlePresentationIter<'a> {
    pool: &'a ParticlePool,
    priority: usize,
    next_slot: Option<usize>,
    remaining: usize,
}

impl<'a> Iterator for ParticlePresentationIter<'a> {
    type Item = PresentedParticle<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(slot) = self.next_slot {
                self.next_slot = self.pool.links[slot].next;
                self.remaining -= 1;
                return Some(PresentedParticle {
                    slot,
                    particle: self.pool.slots[slot]
                        .as_ref()
                        .expect("active particle list contains a live slot"),
                });
            }
            self.priority += 1;
            if self.priority >= PARTICLE_LIST_COUNT {
                return None;
            }
            self.next_slot = self.pool.lists[self.priority].head;
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for ParticlePresentationIter<'_> {}

#[derive(Debug)]
struct ParticlePool {
    slots: [Option<WorldParticle>; MAX_WORLD_PARTICLES],
    links: [ParticleLinks; MAX_WORLD_PARTICLES],
    lists: [ParticleList; PARTICLE_LIST_COUNT],
    live_count: usize,
}

impl Default for ParticlePool {
    fn default() -> Self {
        let mut pool = Self {
            slots: [None; MAX_WORLD_PARTICLES],
            links: [ParticleLinks::EMPTY; MAX_WORLD_PARTICLES],
            lists: [ParticleList::default(); PARTICLE_LIST_COUNT],
            live_count: 0,
        };

        // FUN_0043D290 appends physical slots 0..199 to the free-list tail.
        // FUN_00440A60 also allocates from the tail, so the virgin allocation
        // sequence is 199, 198, 197, ... rather than host-vector order.
        for slot in 0..MAX_WORLD_PARTICLES {
            pool.push_back(0, slot);
        }
        pool
    }
}

impl ParticlePool {
    fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            slots: self.slots,
            links: self.links,
            lists: self.lists,
            live_count: self.live_count,
        }
    }

    fn len(&self) -> usize {
        self.live_count
    }

    fn presentation(&self) -> ParticlePresentationIter<'_> {
        ParticlePresentationIter {
            pool: self,
            priority: 0,
            next_slot: None,
            remaining: self.live_count,
        }
    }

    fn remove(&mut self, slot: usize) {
        let links = self.links[slot];
        let list = usize::from(links.list);
        if let Some(previous) = links.previous {
            self.links[previous].next = links.next;
        } else {
            self.lists[list].head = links.next;
        }
        if let Some(next) = links.next {
            self.links[next].previous = links.previous;
        } else {
            self.lists[list].tail = links.previous;
        }
        self.links[slot].previous = None;
        self.links[slot].next = None;
    }

    fn push_front(&mut self, list: usize, slot: usize) {
        let former_head = self.lists[list].head;
        self.links[slot] = ParticleLinks {
            previous: None,
            next: former_head,
            list: list as u8,
        };
        if let Some(head) = former_head {
            self.links[head].previous = Some(slot);
        } else {
            self.lists[list].tail = Some(slot);
        }
        self.lists[list].head = Some(slot);
    }

    fn push_back(&mut self, list: usize, slot: usize) {
        let former_tail = self.lists[list].tail;
        self.links[slot] = ParticleLinks {
            previous: former_tail,
            next: None,
            list: list as u8,
        };
        if let Some(tail) = former_tail {
            self.links[tail].next = Some(slot);
        } else {
            self.lists[list].head = Some(slot);
        }
        self.lists[list].tail = Some(slot);
    }

    fn allocation_victim(&self, priority: usize) -> Option<usize> {
        for list in 0..=priority {
            let Some(candidate) = self.lists[list].tail else {
                continue;
            };
            if list < priority
                || self.slots[candidate]
                    .as_ref()
                    .is_some_and(|particle| particle.retail_age_byte() != 0)
            {
                return Some(candidate);
            }
        }
        None
    }

    fn allocate(&mut self, particle: WorldParticle) -> Option<usize> {
        let priority = particle_descriptor(particle.source_class)?.priority_raw();
        if !(1..PARTICLE_LIST_COUNT as i8).contains(&priority) {
            return None;
        }
        let priority = priority as usize;
        let slot = self.allocation_victim(priority)?;
        let replaces_live_particle = self.slots[slot].is_some();
        self.remove(slot);
        self.slots[slot] = Some(particle);
        self.push_front(priority, slot);
        if !replaces_live_particle {
            self.live_count += 1;
        }
        Some(slot)
    }

    /// Apply `FUN_00442420`'s in-place descriptor replacement. Retail keeps
    /// the physical address and provenance bytes, resets age/velocity, then
    /// relinks that same address at the replacement class's priority.
    fn transition_class_in_place(
        &mut self,
        slot: usize,
        particle: &mut WorldParticle,
        source_class: u8,
    ) -> bool {
        let Some(descriptor) = particle_descriptor(source_class) else {
            return false;
        };
        let priority = descriptor.priority_raw();
        if !(1..PARTICLE_LIST_COUNT as i8).contains(&priority) {
            return false;
        }
        let Some(frame) = particle_frame(source_class, 0) else {
            return false;
        };

        self.remove(slot);
        particle.source_class = source_class;
        particle.sprite_id = frame.sprite_id;
        particle.velocity = [0.0; 3];
        particle.age_ticks = 0.0;
        particle.lifetime_ticks = descriptor.lifetime_ticks();
        particle.collision_radius_raw = descriptor.collision_radius_raw();
        particle.frame_middle_raw = frame.middle_raw;
        particle.frame_scale_raw = frame.scale_raw;
        particle.animation_rate_raw = descriptor.animation_rate_raw();
        particle.draw_scale_raw = descriptor.draw_scale_raw();
        particle.size_jitter_divisor_raw = descriptor.size_jitter_divisor_raw();
        self.slots[slot] = Some(*particle);
        // Both F42420 and F3E3D0 unlink the old descriptor record and append
        // the same physical address at the replacement priority's tail.
        self.push_back(priority as usize, slot);
        true
    }

    fn free(&mut self, slot: usize) -> bool {
        if self.slots[slot].is_none() {
            return false;
        }
        self.remove(slot);
        self.slots[slot] = None;
        self.live_count -= 1;
        self.push_front(0, slot);
        true
    }

    fn clear_active(&mut self) {
        // FUN_00441B10 walks live lists in ascending priority and frees each
        // head inline. Unlike FUN_00442B40's ordinary free path, this bulk
        // reset does not dispatch descriptor cleanup hooks.
        for priority in 1..PARTICLE_LIST_COUNT {
            while let Some(slot) = self.lists[priority].head {
                self.free(slot);
            }
        }
    }

    #[cfg(test)]
    fn assert_valid_topology(&self) {
        let mut seen = [false; MAX_WORLD_PARTICLES];
        for list in 0..PARTICLE_LIST_COUNT {
            let mut previous = None;
            let mut next = self.lists[list].head;
            while let Some(slot) = next {
                assert!(
                    !seen[slot],
                    "physical slot {slot} appears in multiple lists"
                );
                seen[slot] = true;
                assert_eq!(usize::from(self.links[slot].list), list);
                assert_eq!(self.links[slot].previous, previous);
                assert_eq!(self.slots[slot].is_some(), list != 0);
                previous = Some(slot);
                next = self.links[slot].next;
            }
            assert_eq!(self.lists[list].tail, previous);
        }
        assert!(seen.into_iter().all(|present| present));
        assert_eq!(
            self.slots.iter().flatten().count(),
            self.live_count,
            "live count must match occupied physical records"
        );
    }

    #[cfg(test)]
    fn test_iter_in_virgin_birth_order(&self) -> impl Iterator<Item = &WorldParticle> {
        // Existing behavioral tests were written against Vec insertion order.
        // Virgin retail allocation descends through physical slots, so reverse
        // physical order preserves that test vocabulary without becoming a
        // production traversal contract.
        self.slots.iter().rev().flatten()
    }

    #[cfg(test)]
    fn test_iter_mut_in_physical_order(&mut self) -> impl Iterator<Item = &mut WorldParticle> {
        self.slots.iter_mut().flatten()
    }

    #[cfg(test)]
    fn test_allocate(&mut self, particle: WorldParticle) {
        self.allocate(particle)
            .expect("test particle should fit the retail allocator");
    }

    #[cfg(test)]
    fn test_fill_to(&mut self, target: usize, particle: WorldParticle) {
        assert!(target >= self.live_count, "test helper only grows the pool");
        while self.live_count < target {
            self.test_allocate(particle);
        }
    }

    #[cfg(test)]
    fn test_slot_at(&self, index: usize) -> usize {
        self.slots
            .iter()
            .enumerate()
            .rev()
            .filter_map(|(slot, particle)| particle.as_ref().map(|_| slot))
            .nth(index)
            .expect("particle test index out of bounds")
    }
}

#[cfg(test)]
impl std::ops::Index<usize> for ParticlePool {
    type Output = WorldParticle;

    fn index(&self, index: usize) -> &Self::Output {
        let slot = self.test_slot_at(index);
        self.slots[slot].as_ref().unwrap()
    }
}

#[cfg(test)]
impl std::ops::IndexMut<usize> for ParticlePool {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let slot = self.test_slot_at(index);
        self.slots[slot].as_mut().unwrap()
    }
}

/// Process-global particle-density governor written by `FUN_0044FFA0`.
///
/// Retail averages the latest eight *uncapped* frame durations and lowers
/// selected particle allocation counts only after that average exceeds about
/// 59.5 ms. This is load shedding, not simulation scaling: particle motion
/// continues to use the separately capped frame duration.
#[derive(Debug, Clone)]
struct ParticleFramePacing {
    samples_us: [u32; PARTICLE_FRAME_SAMPLE_COUNT],
    rolling_sum_us: u32,
    frame_counter: u32,
    count_scale_q16: u32,
}

impl Default for ParticleFramePacing {
    fn default() -> Self {
        Self {
            // FUN_0044FA30 seeds a cold eight-frame history. The active-world
            // writer replaces one slot per frame, so particle density ramps
            // back up instead of starting at full rate.
            samples_us: [PARTICLE_FRAME_SAMPLE_MAX_US; PARTICLE_FRAME_SAMPLE_COUNT],
            rolling_sum_us: PARTICLE_FRAME_SAMPLE_MAX_US * PARTICLE_FRAME_SAMPLE_COUNT as u32,
            frame_counter: 0,
            count_scale_q16: 0,
        }
    }
}

impl ParticleFramePacing {
    fn advance(&mut self, uncapped_elapsed_micros: u32) {
        self.frame_counter = self.frame_counter.wrapping_add(1);
        if uncapped_elapsed_micros <= PARTICLE_FRAME_SAMPLE_MAX_US {
            let sample_index = (self.frame_counter as usize) & (PARTICLE_FRAME_SAMPLE_COUNT - 1);
            self.rolling_sum_us = self
                .rolling_sum_us
                .wrapping_sub(self.samples_us[sample_index])
                .wrapping_add(uncapped_elapsed_micros);
            self.samples_us[sample_index] = uncapped_elapsed_micros;
            let average_us = self.rolling_sum_us >> 3;
            self.count_scale_q16 = if average_us < PARTICLE_FULL_RATE_AVERAGE_US {
                PARTICLE_FULL_RATE_SCALE_Q16
            } else {
                PARTICLE_FRAME_SAMPLE_MAX_US - average_us
            };
            return;
        }

        self.samples_us.fill(PARTICLE_FRAME_SAMPLE_MAX_US);
        self.rolling_sum_us = PARTICLE_FRAME_SAMPLE_MAX_US * PARTICLE_FRAME_SAMPLE_COUNT as u32;
        self.count_scale_q16 = 0;
    }

    /// Count policy shared by `FUN_004407D0`, `FUN_00440DC0`,
    /// `FUN_00440E80`, and `FUN_00441A50`.
    ///
    /// The x86 multiply keeps its low 32 bits, the arithmetic shift preserves
    /// the sign, and an exact zero is promoted to one. Callers retain their
    /// original handling of a negative result.
    fn scaled_count_min_one(&self, base_count: i32) -> i32 {
        let scaled = base_count.wrapping_mul(self.count_scale_q16 as i32) >> 16;
        if scaled == 0 {
            1
        } else {
            scaled
        }
    }

    /// Direct-count callbacks `FUN_0043E4F0` and `FUN_0043E8A0` do not apply
    /// the min-one rule; a sufficiently reduced scale intentionally emits no
    /// children.
    fn shifted_count(&self, shift: u32) -> usize {
        (self.count_scale_q16 >> shift) as usize
    }
}

/// Cumulative splash-path counters for the live diagnostics panel. The panel
/// reads and resets them at its own cadence, so values read as per-panel-tick
/// activity. Allocation counters only advance on successful pool placement;
/// compare them against `pool_live`/`pool_capacity` to expose rejection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SplashPathCounters {
    /// Class-19 surface probes the player surface-effect branch allocated.
    pub probes_spawned: u32,
    /// Class-14 downwash spray particles allocated by wet jitter attempts.
    pub spray_particles_spawned: u32,
    /// Class-14 spray sprites that passed the retail presentation visibility
    /// gate and were submitted to the renderer.
    pub spray_presented: u32,
    /// Special surface sounds (ids 26/27) queued by the 1-in-32 wet gate.
    pub splash_sounds: u32,
    /// Whole-body water-entry contacts of any response class.
    pub water_entries: u32,
    /// Hard water-entry commits (the ring-producing class).
    pub hard_entries: u32,
    /// Hard commits whose Type-60 construction rejected (no ring actor).
    pub ring_rejections: u32,
    /// Live Type-60 ring presentations at read time.
    pub rings_live: u32,
    /// Fixed particle-pool occupancy at read time.
    pub pool_live: usize,
    /// Fixed particle-pool capacity at read time.
    pub pool_capacity: usize,
}

/// Shared event queue and live-particle state.
///
/// `update` advances the physical pool and lets descriptor callbacks allocate
/// inline. Externally deferred events are materialized only after that scan, so
/// they render once at age zero rather than silently losing their birth frame.
#[derive(Debug, Default)]
pub struct WorldFx {
    pending: VecDeque<WorldEvent>,
    particles: ParticlePool,
    exploding_rings: Vec<ExplodingRing>,
    ready_sounds: Vec<PositionalSoundEvent>,
    /// State for retail's `FUN_00457930` MSVC linear-congruential generator.
    /// The original global seed at the moment of a hit remains session-state
    /// dependent; retaining one stream preserves the exact update law.
    rng_state: u32,
    /// DAT_004D04C8: 44EBD0's wrapping gust phase. 44EB40 and level teardown
    /// replace wind settings but do not reset this process-global word.
    wind_phase: crate::common_mover::environment::GustPhase,
    /// Process-global 203D0 seed, retained across level and New Game teardown.
    sub_d_allocation_counter: crate::common_mover::sub_d::SubDAllocationCounter,
    /// Retail's process-global cursor into the fixed 100-vector scatter table.
    /// `FUN_00440DC0` advances it once before every attempted particle spawn,
    /// including attempts rejected by the fixed-size allocator.
    direction_cursor: usize,
    /// Last process-global 50-Hz tick observed by the particle traversal.
    /// `None` establishes a reset/loading boundary without aging particles by
    /// an arbitrary absolute clock value on the first update.
    last_retail_tick: Option<u32>,
    /// Eight-frame process-global particle-density governor. Level teardown
    /// clears live effects but deliberately does not reset this state.
    frame_pacing: ParticleFramePacing,
    /// Live diagnostics counters; never simulated state.
    splash_path_counters: SplashPathCounters,
}

impl InfectionEvolutionEffects for WorldFx {
    fn next_shared_random_u16(&mut self) -> u16 {
        self.next_shared_retail_random_u16()
    }

    fn queue_infection_tail_sound(&mut self, sound: InfectionTailSound) {
        self.queue_fixed_positional_sound_raw(sound.global_sound_id, sound.position_raw);
    }
}

impl AuthoredHiveComponentEffects for WorldFx {
    fn queue_hive_detailed_sound(&mut self, sound_id: u16, position_raw: [i16; 3]) {
        self.queue_fixed_positional_sound_raw(sound_id, position_raw);
    }

    fn emit_authored_radial(&mut self, emission: AuthoredRadialEmission) {
        debug_assert_eq!(emission.particle_class, ALIEN_HIVE_PARTICLE_CLASS);
        self.emit_alien_hive_particle_raw(emission.position_raw, emission.source_id);
    }
}

impl WorldFx {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn advance_wind_phase(&mut self, elapsed_micros: u32) -> u32 {
        self.wind_phase.advance(elapsed_micros)
    }

    /// Deep-copy the process effect/RNG owner only for the isolated Main Base
    /// abort transaction.
    ///
    /// A failed speculative sweep drops this fork, leaving queued effects,
    /// physical particle slots, sound order, and the shared retail RNG cursor
    /// exactly as they were at admission.
    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            pending: self.pending.clone(),
            particles: self.particles.fork_for_main_base_abort_transaction(),
            exploding_rings: self.exploding_rings.clone(),
            ready_sounds: self.ready_sounds.clone(),
            rng_state: self.rng_state,
            wind_phase: self.wind_phase,
            sub_d_allocation_counter: self.sub_d_allocation_counter,
            direction_cursor: self.direction_cursor,
            last_retail_tick: self.last_retail_tick,
            frame_pacing: self.frame_pacing.clone(),
            splash_path_counters: self.splash_path_counters,
        }
    }

    /// Read and reset the splash-path diagnostics counters, snapshotting the
    /// particle-pool occupancy at read time.
    pub fn take_splash_path_counters(&mut self) -> SplashPathCounters {
        let mut counters = std::mem::take(&mut self.splash_path_counters);
        counters.pool_live = self.particle_count();
        counters.pool_capacity = MAX_WORLD_PARTICLES;
        counters.rings_live = self.exploding_rings.len() as u32;
        counters
    }

    /// Presentation-side splash counter feed (called from the draw path).
    pub fn note_spray_presented(&mut self, count: u32) {
        self.splash_path_counters.spray_presented += count;
    }

    /// Diagnostics feed: one whole-body water-entry contact of any class.
    pub fn note_water_entry(&mut self) {
        self.splash_path_counters.water_entries += 1;
    }

    /// Diagnostics feed: one hard water-entry commit attempt.
    pub fn note_hard_entry(&mut self) {
        self.splash_path_counters.hard_entries += 1;
    }

    /// Diagnostics feed: one hard commit whose Type-60 construction rejected.
    pub fn note_ring_rejection(&mut self) {
        self.splash_path_counters.ring_rejections += 1;
    }

    /// Feed the uncapped wall-frame duration to retail's particle-density
    /// governor. Simulation continues to use its independently capped delta.
    pub fn advance_frame_pacing(&mut self, uncapped_elapsed_micros: u32) {
        self.frame_pacing.advance(uncapped_elapsed_micros);
    }

    /// Consume one word from the process-wide retail random stream.
    ///
    /// Static-object chance gates run in the same `FUN_00457930` stream as
    /// particle gates and sound variation. Exposing one draw keeps the
    /// scheduler stateless without accidentally giving it a private seed.
    pub fn next_shared_retail_random_u16(&mut self) -> u16 {
        retail_random_u16(&mut self.rng_state)
    }

    /// Disjoint process state for the legacy Intro2 constructor's RNG callback.
    /// The level owner may advance these cursors, but must never reset them.
    pub(crate) fn entity_construction_state(
        &mut self,
    ) -> (
        &mut u32,
        &mut crate::common_mover::sub_d::SubDAllocationCounter,
    ) {
        (&mut self.rng_state, &mut self.sub_d_allocation_counter)
    }

    pub(crate) fn construct_entity_sub_d(
        &mut self,
        descriptor: v2k_formats::collision::SubDSteeringDescriptor,
    ) -> crate::common_mover::sub_d::NativeSubDConstruction {
        crate::common_mover::sub_d::construct_native_sub_d(
            &mut self.sub_d_allocation_counter,
            descriptor,
        )
    }

    /// Next process allocation seed; diagnostics do not advance the constructor.
    pub fn next_sub_d_allocation_seed(&self) -> u8 {
        self.sub_d_allocation_counter.next_seed()
    }

    /// Materialize one passing shared mode-5/mode-6 terrain-contact callback.
    ///
    /// `FUN_00440DC0` scales `scale >> 10` through the process-global particle
    /// pacing governor, promoting an exact zero result to one. It advances the
    /// shared 100-vector cursor before every attempted allocation, including
    /// attempts rejected by the fixed particle pool, and consumes no RNG.
    pub fn emit_terrain_contact_particle_raw(&mut self, emission: DefecateVirusParticleEmission) {
        self.emit_fun_00440dc0_raw(
            emission.position_raw(),
            emission.particle_class(),
            emission.scale_raw(),
            emission.owner_entity_handle(),
            emission.suppress_impact_damage(),
        );
    }

    /// Materialize primary hit's exact class-5 capability follow-up.
    ///
    /// `FUN_00410EB0` has already selected the active model, adjusted the
    /// emission Z coordinate by that model's signed `+0x08` word, and copied
    /// the cached target state sign into `owner_sign`. `FUN_00440DC0` then
    /// applies the shared pacing scale, pre-increments the direction cursor
    /// for every attempted allocation, and copies that sign into particle
    /// byte `+0x1D` bit zero. A set bit does not suppress carrier allocation;
    /// it suppresses the eventual terrain mutation before that carrier is
    /// removed by the class-5 contact callback.
    pub fn emit_primary_hit_capability_follow_up_raw(
        &mut self,
        emission: PrimaryHitCapabilityEmission,
    ) {
        assert_eq!(
            emission.particle_class, PRIMARY_HIT_CAPABILITY_PARTICLE_CLASS,
            "primary-hit capability follow-up must retain authored class 5"
        );
        assert_eq!(
            emission.particle_scale_raw, PRIMARY_HIT_CAPABILITY_PARTICLE_SCALE_RAW,
            "primary-hit capability follow-up must retain authored scale 0x0800"
        );
        assert!(
            emission.owner_sign <= 1,
            "primary-hit owner sign must be the exact state-word sign bit"
        );
        debug_assert_eq!(
            PRIMARY_HIT_CAPABILITY_PARTICLE_CLASS,
            ALIEN_HIVE_PARTICLE_CLASS
        );
        self.emit_fun_00440dc0_raw(
            emission.position_raw,
            emission.particle_class,
            u32::from(emission.particle_scale_raw),
            emission.target_handle,
            emission.owner_sign != 0,
        );
    }

    /// Shared `FUN_00440DC0` directional carrier allocator.
    ///
    /// Allocation failure does not terminate the loop: retail advances the
    /// process-global direction cursor once for every scaled attempt even
    /// when the fixed pool rejects that attempt.
    fn emit_fun_00440dc0_raw(
        &mut self,
        position_raw: [i16; 3],
        particle_class: u8,
        scale_raw: u32,
        owner_entity_handle: u32,
        suppress_impact_damage: bool,
    ) {
        let attempt_count = self
            .frame_pacing
            .scaled_count_min_one((scale_raw >> 10) as i32)
            .max(0) as usize;
        for _ in 0..attempt_count {
            self.direction_cursor = (self.direction_cursor + 1) % RETAIL_DIRECTION_TABLE_RAW.len();
            let direction = RETAIL_DIRECTION_TABLE_RAW[self.direction_cursor];
            let raw_velocity = [
                i32::from(direction[0]) >> 1,
                (i32::from(direction[1]) >> 2).abs(),
                i32::from(direction[2]) >> 1,
            ];
            let mut burst = single_descriptor_particle_burst(
                raw_position_to_world(position_raw),
                particle_class,
                raw_velocity,
                Some(owner_entity_handle),
            );
            burst.particles[0].suppresses_impact_damage = suppress_impact_damage;
            self.materialize_particle_burst(burst);
        }
    }

    /// Materialize one passing class-4 `"Defecate Virus"` detailed callback.
    ///
    /// This named wrapper rejects accidental use of class 6 at the
    /// class-4-only call site. Cleansing Landscape uses the shared emitter
    /// directly after its distinct owner transaction has selected mode 6.
    pub fn emit_defecate_virus_particle_raw(&mut self, emission: DefecateVirusParticleEmission) {
        assert_eq!(
            emission.terrain_contact_mode(),
            TerrainContactMode::Infect,
            "class-4 Defecate Virus emission cannot carry cleansing mode"
        );
        self.emit_terrain_contact_particle_raw(emission);
    }

    /// Emit one native Alien-Hive class-5 particle synchronously.
    ///
    /// `FUN_0041BEB0` consumes three consecutive words from the shared
    /// `FUN_00457930` stream before calling `FUN_00440A60`: packed direction,
    /// horizontal radius, then vertical velocity. All three draws therefore
    /// survive a rejected fixed-pool allocation. The descriptor allocator
    /// subsequently applies class 5's zero `+0x2A` spawn-velocity bias. The
    /// signed -100 in descriptor word `+0x12` is its sort bias, not velocity.
    pub fn emit_alien_hive_particle_raw(&mut self, position_raw: [i16; 3], source_id: u32) {
        let angle = self.next_shared_retail_random_u16();
        let radius = i32::from(self.next_shared_retail_random_u16() % 0x640);
        let vertical_velocity_raw =
            i32::from(self.next_shared_retail_random_u16() & 0x01ff) + 0x02ee;
        let raw_velocity = [
            (retail_sine_q15(u32::from(angle)) * radius) >> 16,
            vertical_velocity_raw,
            (retail_sine_q15(u32::from(angle.wrapping_add(0x4000))) * radius) >> 16,
        ];

        self.materialize_particle_burst(single_descriptor_particle_burst(
            raw_position_to_world(position_raw),
            ALIEN_HIVE_PARTICLE_CLASS,
            raw_velocity,
            Some(source_id),
        ));
    }

    /// `FUN_00442950` attached-class allocation (83 for source 52, 84 for 68,
    /// 86 otherwise).
    ///
    /// Retail `FUN_00440A60` stores zero velocity words, the hit-target handle
    /// at `+0x14`, and the impact-relative offset at `+0x0E/+0x10/+0x12`; motion
    /// comes from the `FUN_004425D0` follow. `None` mirrors retail ignoring a
    /// fixed-pool rejection: the damage prefix still stands.
    pub fn emit_attached_static_particle_raw(
        &mut self,
        emission: AttachedStaticEmission,
    ) -> Option<AttachedParticleBirth> {
        debug_assert!(
            matches!(emission.particle_class, 83 | 84 | 86),
            "attached class comes from the 442950 source mapping"
        );
        let mut spawn = invisible_descriptor_particle_spawn(
            emission.particle_class,
            [0; 3],
            Some(emission.target_handle),
        );
        spawn.attached_owner_handle = Some(emission.target_handle);
        spawn.attached_offset_raw = emission.offset_raw;
        self.materialize_particle_spawn_with_birth_context(emission.position_world, spawn, None)
            .map(|slot| AttachedParticleBirth {
                slot,
                particle_class: emission.particle_class,
            })
    }

    /// Emit kind 9's exact opcode-13 class-79 scatter synchronously.
    ///
    /// The authored 20-byte payload at `0x004C9B58` requests two particles,
    /// stores zero extent (clamped by retail to 16), selects class 79 in both
    /// alternating class bytes, uses the known zero owner sentinel, and passes
    /// velocity scale one to `FUN_004407D0`. The supported single-player path
    /// leaves impact suppression clear.
    pub fn emit_static_kind9_scatter_raw(&mut self, position_raw: [i16; 3]) {
        self.emit_direction_table_scatter_raw(
            position_raw,
            DirectionTableScatterEmission {
                base_count: STATIC_KIND_9_SCATTER_COUNT,
                extent_raw: STATIC_KIND_9_SCATTER_EXTENT_RAW,
                source_classes: [
                    STATIC_DESTRUCTION_EFFECT_CLASS_79,
                    STATIC_DESTRUCTION_EFFECT_CLASS_79,
                ],
                velocity_scale_raw: STATIC_KIND_9_SCATTER_VELOCITY_SCALE_RAW,
                owner_id: Some(0),
                source_entity_type_at_birth: Some(0),
                suppresses_impact_damage: false,
            },
        );
    }

    /// Exact shared `FUN_004407D0` direction-table allocator.
    ///
    /// Retail scales the count through the process-global pacing governor,
    /// advances the direction cursor before every attempt, alternates the two
    /// authored classes by successful index, and returns at the first failed
    /// allocation. Its visible normalized-position bug stores Y in both the Y
    /// and Z offsets while velocity retains the real direction Z component.
    fn emit_direction_table_scatter_raw(
        &mut self,
        position_raw: [i16; 3],
        emission: DirectionTableScatterEmission,
    ) {
        let extent_raw = i32::from(emission.extent_raw.max(0x10));
        let attempt_count = self
            .frame_pacing
            .scaled_count_min_one(emission.base_count)
            .max(0) as usize;
        for attempt_index in 0..attempt_count {
            self.direction_cursor = (self.direction_cursor + 1) % RETAIL_DIRECTION_TABLE_RAW.len();
            let direction = RETAIL_DIRECTION_TABLE_RAW[self.direction_cursor];
            let l1_length = direction
                .iter()
                .map(|component| i32::from(*component).abs())
                .sum::<i32>();
            let offset_raw = if l1_length == 0 {
                [0; 3]
            } else {
                let x = extent_raw * i32::from(direction[0]) / l1_length;
                let y = extent_raw * i32::from(direction[1]) / l1_length;
                [x, y, y]
            };
            let spawn_position_raw = std::array::from_fn(|axis| {
                position_raw[axis].wrapping_add(offset_raw[axis] as i16)
            });
            let raw_velocity = direction.map(|component| {
                i32::from(component).wrapping_mul(emission.velocity_scale_raw) >> 4
            });
            let source_class = emission.source_classes[attempt_index & 1];
            let mut burst = single_descriptor_particle_burst(
                raw_position_to_world(spawn_position_raw),
                source_class,
                raw_velocity,
                emission.owner_id,
            );
            burst.particles[0].source_entity_type_at_birth = emission.source_entity_type_at_birth;
            burst.particles[0].suppresses_impact_damage = emission.suppresses_impact_damage;
            let spawn = burst.particles[0];
            if self
                .materialize_particle_spawn_with_birth_context(burst.origin, spawn, None)
                .is_none()
            {
                break;
            }
        }
    }

    /// Emit `FUN_00441200`'s common staged-destruction bundle immediately.
    ///
    /// Retail first writes a radius-`0x600` terrain light, then runs the
    /// failure-short-circuiting ten-way class-30 scatter from
    /// `FUN_004407D0`. One deliberately dead random class-selection word is
    /// consumed before the fixed-rate sound-62 request. Finally four debris
    /// allocations consume their class and XYZ words even when the bounded
    /// particle pool rejects an attempt. Keeping all of this synchronous
    /// preserves the shared RNG and direction-cursor order between successive
    /// model-effect anchors. Every currently admitted caller stores the
    /// zero-initialized `DAT_004DCA00` sentinel in the owner word. The allocator
    /// retains that known zero and writes birth type zero after its failed
    /// entity lookup; neither word is missing provenance.
    pub fn emit_common_explosion_bundle_raw(
        &mut self,
        position_raw: [i16; 3],
        source_extent_raw: u16,
    ) -> TerrainExplosionLight {
        self.emit_direction_table_scatter_raw(
            position_raw,
            DirectionTableScatterEmission {
                base_count: COMMON_EXPLOSION_SCATTER_COUNT as i32,
                extent_raw: source_extent_raw,
                source_classes: [
                    COMMON_EXPLOSION_SCATTER_CLASS,
                    COMMON_EXPLOSION_SCATTER_CLASS,
                ],
                velocity_scale_raw: 8,
                owner_id: Some(0),
                source_entity_type_at_birth: Some(0),
                suppresses_impact_damage: false,
            },
        );

        // FUN_00441200 computes class 22/24 from this word but overwrites the
        // local class byte before calling the allocator. The draw is real even
        // though the corresponding center particle is not.
        let _dead_center_class_word = self.next_shared_retail_random_u16();
        let sound_rate_q16 = 0xC000 + u32::from(self.next_shared_retail_random_u16() >> 1);
        let debris_origin_raw = [
            position_raw[0],
            position_raw[1].wrapping_add(COMMON_EXPLOSION_DEBRIS_Y_LIFT_RAW),
            position_raw[2],
        ];
        self.ready_sounds.push(PositionalSoundEvent {
            sound_id: COMMON_EXPLOSION_SOUND_ID,
            position: raw_position_to_world(debris_origin_raw),
            frequency_q16: sound_rate_q16,
        });

        // The first class gate is one bit wide; the remaining three use the
        // full authored 22..=29 debris family. The four quadrant biases match
        // the unrolled retail calls and every call retains raw Y velocity 100.
        const DEBRIS_LAYOUTS: [(i16, i16); COMMON_EXPLOSION_DEBRIS_COUNT] =
            [(-0x52, -0x52), (-0x52, 0x12), (0x12, 0x12), (0x12, -0x52)];
        for (index, (x_bias, y_bias)) in DEBRIS_LAYOUTS.into_iter().enumerate() {
            let class_mask = if index == 0 { 1 } else { 7 };
            let source_class = 0x16 + (self.next_shared_retail_random_u16() as u8 & class_mask);
            let x_offset = (self.next_shared_retail_random_u16() & 0x3f) as i16 + x_bias;
            let y_offset = (self.next_shared_retail_random_u16() & 0x3f) as i16 + y_bias;
            let z_offset = (self.next_shared_retail_random_u16() & 0x3f) as i16 - 0x20;
            let spawn_position_raw = [
                debris_origin_raw[0].wrapping_add(x_offset),
                debris_origin_raw[1].wrapping_add(y_offset),
                debris_origin_raw[2].wrapping_add(z_offset),
            ];
            let mut burst = single_descriptor_particle_burst(
                raw_position_to_world(spawn_position_raw),
                source_class,
                [0, 100, 0],
                Some(0),
            );
            burst.particles[0].source_entity_type_at_birth = Some(0);
            let spawn = burst.particles[0];
            let _ = self.materialize_particle_spawn_with_birth_context(burst.origin, spawn, None);
        }

        TerrainExplosionLight {
            x_raw: position_raw[0],
            z_raw: position_raw[2],
            radius_raw: COMMON_EXPLOSION_TERRAIN_LIGHT_RADIUS_RAW,
        }
    }

    /// Materialize type 46's `FUN_00440E80` surface-effect branch.
    ///
    /// Above the static sea word this creates one invisible class-19 probe.
    /// At or below it, retail substitutes a strength-scaled class-44 spray.
    /// Both branches consume the pre-force pose captured by the controller;
    /// later gravity, drag, and position integration must not leak into it.
    pub fn emit_player_surface_effect_raw(
        &mut self,
        emission: PlayerSurfaceEffectEmission,
        sea_level_raw: i16,
    ) {
        if sea_level_raw < emission.position_raw[1] {
            let velocity_raw = std::array::from_fn(|axis| {
                emission.velocity_raw[axis].wrapping_sub((emission.up_q31[axis] >> 20) as i16)
            });
            let allocated = self.materialize_particle_spawn_with_birth_context(
                raw_position_to_world(emission.position_raw),
                invisible_descriptor_particle_spawn(
                    PLAYER_SURFACE_PROBE_PARTICLE_CLASS,
                    velocity_raw.map(i32::from),
                    Some(emission.owner_entity_id),
                ),
                None,
            );
            if allocated.is_some() {
                self.splash_path_counters.probes_spawned += 1;
            }
            return;
        }

        // The executable computes `(strength >> 7) * DAT_004F72CC >> 16`,
        // forces an exact zero to one, and uses the sign only to mirror the
        // body-up Y subtraction. Preserve those signed integer boundaries.
        let signed_count = self
            .frame_pacing
            .scaled_count_min_one(emission.strength_raw >> 7);
        let vertical_sign = if signed_count < 0 { -1 } else { 1 };
        let count = if signed_count == 0 {
            1
        } else {
            signed_count.unsigned_abs() as usize
        };
        let base_velocity_raw = [
            emission.velocity_raw[0].wrapping_sub((emission.up_q31[0] >> 21) as i16),
            emission.velocity_raw[1]
                .wrapping_sub(((emission.up_q31[1] >> 21) * vertical_sign) as i16),
            emission.velocity_raw[2].wrapping_sub((emission.up_q31[2] >> 21) as i16),
        ];
        let spawn_y_raw = emission.position_raw[1].wrapping_sub(100);

        for _ in 0..count {
            let angle = self.next_shared_retail_random_u16();
            let sine = retail_sine_q15(u32::from(angle));
            let cosine = retail_sine_q15(u32::from(angle.wrapping_add(0x4000)));
            let x_jitter = ((self.next_shared_retail_random_u16() >> 9) as i16).wrapping_sub(0x40);
            let z_jitter = ((self.next_shared_retail_random_u16() >> 9) as i16).wrapping_sub(0x40);
            let vertical_jitter =
                ((self.next_shared_retail_random_u16() >> 7) as i16).wrapping_sub(0x100);
            let position_raw = [
                emission.position_raw[0].wrapping_add(x_jitter),
                spawn_y_raw,
                emission.position_raw[2].wrapping_add(z_jitter),
            ];
            let velocity_x_raw = (i32::from(base_velocity_raw[0])
                + ((sine * (emission.forward_q31[0] >> 20)) >> 15)
                + ((sine * (emission.lateral_q31[0] >> 20)) >> 15))
                as i16;
            let velocity_z_raw = (i32::from(base_velocity_raw[2])
                + ((cosine * (emission.lateral_q31[2] >> 20)) >> 15)
                + ((cosine * (emission.forward_q31[2] >> 20)) >> 15))
                as i16;
            let raw_velocity = [
                i32::from(velocity_x_raw),
                i32::from(base_velocity_raw[1].wrapping_add(vertical_jitter)),
                i32::from(velocity_z_raw),
            ];
            self.materialize_particle_burst(single_descriptor_particle_burst(
                raw_position_to_world(position_raw),
                PLAYER_SUBMERGED_DOWNWASH_PARTICLE_CLASS,
                raw_velocity,
                Some(emission.owner_entity_id),
            ));
        }
    }

    /// Emit `FUN_004141D0`'s normal whole-body water-entry response.
    ///
    /// `position_raw` is the entering entity center; `surface_y_raw` replaces
    /// its Y at the live displaced-wave surface. Selectors zero through six
    /// choose the authored class and attempt exactly four allocations from the
    /// shared deterministic direction cursor. Selector seven intentionally
    /// emits nothing and consumes no directions. Every successful particle
    /// retains the entering entity as its owner.
    pub fn emit_whole_body_water_entry_burst_raw(
        &mut self,
        position_raw: [i16; 3],
        surface_y_raw: i16,
        response_selector: u8,
        owner_entity_id: u32,
    ) {
        let Some(&source_class) =
            WHOLE_BODY_WATER_ENTRY_CLASS_BY_SELECTOR.get(usize::from(response_selector))
        else {
            debug_assert_eq!(
                response_selector, 7,
                "whole-body water-entry selector must be in 0..=7"
            );
            return;
        };
        let animated_surface_position =
            raw_position_to_world([position_raw[0], surface_y_raw, position_raw[2]]);
        let attempt_count = self
            .frame_pacing
            .scaled_count_min_one(SURFACE_RESPONSE_BURST_COUNT as i32)
            .max(0) as usize;
        let burst = directional_surface_response_burst(
            animated_surface_position,
            source_class,
            Some(owner_entity_id),
            &mut self.direction_cursor,
            attempt_count,
        );
        self.materialize_particle_burst(burst);
    }

    /// Materialize an authored descriptor through `FUN_00440A60`.
    ///
    /// This enforces retail's descriptor/request suppression rejection before
    /// allocating. It does not invent the unresolved upstream target/lead
    /// computation or deferred-list scheduling.
    pub fn materialize_descriptor_particle_request(
        &mut self,
        request: DescriptorParticleRequest,
        environment: ParticleEnvironment<'_>,
        retail_tick: u32,
    ) -> Option<usize> {
        let descriptor = particle_descriptor(request.source_class)?;
        if request.suppresses_impact_damage && descriptor.flags() & 0x02 != 0 {
            return None;
        }
        let owner_id = request.owner.map(|owner| owner.entity_id);
        let source_entity_type_at_birth = Some(request.owner.map_or(0, |owner| owner.entity_type));
        let mut position_raw = request.position_raw;
        position_raw[1] = position_raw[1].wrapping_add(descriptor.spawn_position_y_bias_raw());
        let mut velocity_words = request.velocity_raw;
        velocity_words[1] = velocity_words[1].wrapping_add(descriptor.spawn_velocity_y_bias_raw());
        let velocity_raw = velocity_words.map(i32::from);
        let frame = first_particle_frame(request.source_class);
        let spawn = ParticleSpawn {
            sprite_id: frame.sprite_id,
            source_class: request.source_class,
            offset: [0.0; 3],
            presentation_offset: [0.0; 3],
            velocity: raw_velocity_to_world(velocity_raw),
            owner_id,
            attached_owner_handle: None,
            attached_offset_raw: [0; 3],
            source_entity_type_at_birth,
            suppresses_impact_damage: request.suppresses_impact_damage,
            lifetime_ticks: descriptor.lifetime_ticks(),
            collision_radius_raw: descriptor.collision_radius_raw(),
            frame_middle_raw: frame.middle_raw,
            frame_scale_raw: frame.scale_raw,
            animation_rate_raw: descriptor.animation_rate_raw(),
            draw_scale_raw: descriptor.draw_scale_raw(),
            size_jitter_divisor_raw: descriptor.size_jitter_divisor_raw(),
        };
        self.materialize_particle_spawn_with_birth_context(
            raw_position_to_world(position_raw),
            spawn,
            Some(ParticleBirthContext {
                environment,
                retail_tick,
            }),
        )
    }

    /// Materialize the supported class-38 `FUN_004410B0` / `FUN_00440A60` slice.
    ///
    /// Dry levels and positions strictly above the signed raw sea plane keep
    /// class 38. At or below the plane, retail substitutes class `0x2E` and
    /// subtracts 100 from Y before `FUN_00440A60`; class46's +28 bias adds
    /// those 100 units back during construction.
    pub fn materialize_class_38_request(
        &mut self,
        request: Class38ParticleRequest,
        environment: ParticleEnvironment<'_>,
        retail_tick: u32,
    ) -> Option<Class38ParticleBirth> {
        self.materialize_combat_projectile_410b0(
            DescriptorParticleRequest {
                source_class: TYPE13_PROJECTILE_PARTICLE_CLASS,
                position_raw: request.position_raw,
                velocity_raw: request.velocity_raw,
                owner: Some(ParticleOwnerAtBirth {
                    entity_id: request.owner_entity_id,
                    entity_type: request.owner_entity_type,
                }),
                suppresses_impact_damage: request.suppresses_impact_damage,
            },
            environment,
            retail_tick,
        )
        .map(|birth| Class38ParticleBirth {
            slot: birth.slot,
            particle_class: birth.particle_class,
        })
    }

    /// Materialize the class-42 request emitted by an authenticated
    /// `FUN_0040E370` actor-surface owner.
    ///
    /// The request retains the allocator arguments before descriptor
    /// processing. In particular, its Y velocity is zero; `FUN_00440A60`
    /// adds class 42's signed bias stored at descriptor `+0x2A` exactly once
    /// here. Owner type and impact-suppression provenance are copied at birth
    /// just as the retail allocator does after resolving the supplied entity
    /// handle.
    pub fn materialize_actor_surface_bubble_request(
        &mut self,
        request: ActorSurfaceBubbleRequest,
        environment: ParticleEnvironment<'_>,
        retail_tick: u32,
    ) -> Option<usize> {
        let descriptor = particle_descriptor(PRIMARY_UNDERWATER_IMPACT_CLASS)
            .expect("class 42 has a retail particle descriptor");
        if request.suppresses_impact_damage && descriptor.flags() & 0x02 != 0 {
            return None;
        }
        let mut velocity_raw = request.velocity_argument_raw.map(i32::from);
        velocity_raw[1] += i32::from(descriptor.spawn_velocity_y_bias_raw());
        let frame = first_particle_frame(PRIMARY_UNDERWATER_IMPACT_CLASS);
        let spawn = ParticleSpawn {
            sprite_id: frame.sprite_id,
            source_class: PRIMARY_UNDERWATER_IMPACT_CLASS,
            offset: [0.0; 3],
            presentation_offset: [0.0; 3],
            velocity: raw_velocity_to_world(velocity_raw),
            owner_id: Some(request.owner_entity_id),
            attached_owner_handle: None,
            attached_offset_raw: [0; 3],
            source_entity_type_at_birth: Some(request.owner_entity_type),
            suppresses_impact_damage: request.suppresses_impact_damage,
            lifetime_ticks: descriptor.lifetime_ticks(),
            collision_radius_raw: descriptor.collision_radius_raw(),
            frame_middle_raw: frame.middle_raw,
            frame_scale_raw: frame.scale_raw,
            animation_rate_raw: descriptor.animation_rate_raw(),
            draw_scale_raw: descriptor.draw_scale_raw(),
            size_jitter_divisor_raw: descriptor.size_jitter_divisor_raw(),
        };
        self.materialize_particle_spawn_with_birth_context(
            raw_position_to_world(request.position_raw),
            spawn,
            Some(ParticleBirthContext {
                environment,
                retail_tick,
            }),
        )
    }

    pub fn queue(&mut self, event: WorldEvent) {
        self.pending.push_back(event);
    }

    pub fn pending_event_count(&self) -> usize {
        self.pending.len()
    }

    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }

    pub fn exploding_rings(&self) -> &[ExplodingRing] {
        &self.exploding_rings
    }

    /// Publish the constructor-selected presentation owned by a real Type-60
    /// actor. Class 49 retains model 243; hard-water calls retain their four-
    /// slot model-132 or model-130 override in this entity-backed record.
    ///
    /// `FUN_0040B290` leaves bound output one at zero until its Primary task is
    /// installed; successful `FUN_00406D20` then publishes `0xffff`. The caller
    /// supplies either authenticated value so the fallback actor does not
    /// masquerade as a successfully initialized ring. [`WorldFx`] never
    /// advances this control independently: the authenticated Type-60 task
    /// owner commits each callback output and removes the record at terminal.
    pub(crate) fn materialize_type60_exploding_ring_raw(
        &mut self,
        entity_id: u32,
        position_raw: [i16; 3],
        model_id: usize,
        initial_control_output_1_raw: u16,
    ) {
        debug_assert!(matches!(
            model_id,
            TYPE60_EXPLODING_RING_MODEL_ID
                | HARD_WATER_ENTRY_SPLASH_MID_MODEL_ID
                | HARD_WATER_ENTRY_SPLASH_MODEL_ID
        ));
        debug_assert!(matches!(
            initial_control_output_1_raw,
            0 | EXPLODING_RING_INITIAL_CONTROL_RAW
        ));
        debug_assert!(
            self.exploding_rings
                .iter()
                .all(|ring| ring.associated_entity_id != entity_id),
            "one Type-60 actor cannot own multiple exploding-ring presentations"
        );
        self.exploding_rings.push(ExplodingRing {
            model_id,
            position: raw_position_to_world(position_raw),
            associated_entity_id: entity_id,
            control_output_1_raw: initial_control_output_1_raw,
        });
    }

    /// Commit one authenticated Type-60 callback output to its exact
    /// entity-backed presentation. Missing/duplicate ownership and stale
    /// expected control all fail without mutation.
    pub(crate) fn set_type60_exploding_ring_control_raw(
        &mut self,
        entity_id: u32,
        expected_control_output_1_raw: u16,
        next_control_output_1_raw: u16,
    ) -> bool {
        let index = {
            let mut matches = self
                .exploding_rings
                .iter()
                .enumerate()
                .filter(|(_, ring)| ring.associated_entity_id == entity_id);
            let Some((index, ring)) = matches.next() else {
                return false;
            };
            if matches.next().is_some()
                || ring.control_output_1_raw != expected_control_output_1_raw
            {
                return false;
            }
            index
        };
        self.exploding_rings[index].control_output_1_raw = next_control_output_1_raw;
        true
    }

    /// Remove only the presentation owned by `entity_id` during Type 60's
    /// synchronous class-2 death callback.
    pub(crate) fn remove_type60_exploding_ring(&mut self, entity_id: u32) -> bool {
        let index = {
            let mut matches = self
                .exploding_rings
                .iter()
                .enumerate()
                .filter(|(_, ring)| ring.associated_entity_id == entity_id);
            let Some((index, _)) = matches.next() else {
                return false;
            };
            if matches.next().is_some() {
                return false;
            }
            index
        };
        self.exploding_rings.remove(index);
        true
    }

    /// Emit shared helper `FUN_00440950` without a caller-owned follow-up cue.
    ///
    /// The helper first calls `FUN_004407D0` for ten
    /// class-16 attempts. That scatter advances the process-global direction
    /// cursor before every attempt and stops on the first allocator rejection.
    /// Its surface tail is independent: class 18 is still attempted above the
    /// sea plane, while positions at/below it attempt class 45 and then class
    /// 46. The helper consumes one shared RNG word for sound 62's wrapper rate.
    ///
    /// Both player wrecks and the Alien-Hive death continuation use this
    /// helper, but only `FUN_004475F0` submits the additional fixed-rate sound.
    pub fn emit_scatter_surface_burst_raw(
        &mut self,
        position_raw: [i16; 3],
        source_extent_raw: u16,
        sea_level_raw: Option<i16>,
        owner_entity_id: u32,
    ) {
        self.emit_direction_table_scatter_raw(
            position_raw,
            DirectionTableScatterEmission {
                base_count: METEOR_IMPACT_SCATTER_COUNT as i32,
                extent_raw: source_extent_raw,
                source_classes: [METEOR_SCATTER_CLASS; 2],
                velocity_scale_raw: 8,
                owner_id: Some(owner_entity_id),
                source_entity_type_at_birth: None,
                suppresses_impact_damage: false,
            },
        );
        self.emit_scatter_surface_sound_tail_raw(
            position_raw,
            sea_level_raw,
            owner_entity_id,
            None,
        );
    }

    /// Shared 440950 explosion prefix: scatter, the caller-authenticated
    /// solo kind-2 network no-op, then independent surface particles/sound62.
    /// Logical owner attribution and the source's impact-suppression flag
    /// remain distinct inputs even when the source allocation later disappears.
    pub(crate) fn emit_explode_with_ring_burst_raw(
        &mut self,
        request: ExplodeWithRingBurstRequest,
    ) {
        self.emit_direction_table_scatter_raw(
            request.position_raw,
            DirectionTableScatterEmission {
                base_count: request.scatter_count,
                extent_raw: request.source_extent_raw,
                source_classes: request.scatter_classes,
                velocity_scale_raw: EXPLODE_WITH_RING_VELOCITY_SCALE_RAW,
                owner_id: Some(request.logical_owner_entity_id),
                source_entity_type_at_birth: Some(request.logical_owner_entity_type),
                suppresses_impact_damage: request.suppresses_impact_damage,
            },
        );
        self.emit_scatter_surface_sound_tail_raw(
            request.position_raw,
            request.sea_level_raw,
            request.logical_owner_entity_id,
            Some(request.logical_owner_entity_type),
        );
    }

    /// Shared post-scatter portion of `FUN_00440950`.
    ///
    /// This phase is deliberately independent of direction-scatter allocation
    /// success. It attempts the surface-selected descriptors in retail order,
    /// then consumes exactly one shared RNG word for sound 62.
    fn emit_scatter_surface_sound_tail_raw(
        &mut self,
        position_raw: [i16; 3],
        sea_level_raw: Option<i16>,
        owner_entity_id: u32,
        source_entity_type_at_birth: Option<u8>,
    ) {
        let mut sound_position_raw = position_raw;
        if sea_level_raw.is_none_or(|sea_level| sea_level < position_raw[1]) {
            self.materialize_owned_descriptor_particle_raw(
                position_raw,
                METEOR_SURFACE_CLASS,
                [0, 500, 0],
                owner_entity_id,
                source_entity_type_at_birth,
            );
        } else {
            self.materialize_owned_descriptor_particle_raw(
                position_raw,
                UNDERWATER_IMPACT_CLASSES[0],
                [0; 3],
                owner_entity_id,
                source_entity_type_at_birth,
            );
            // 440A20 mutates 440950's local request even if either allocation
            // fails. The later sound62 reads that same displaced position.
            sound_position_raw[0] = sound_position_raw[0].wrapping_sub(0x40);
            self.materialize_owned_descriptor_particle_raw(
                sound_position_raw,
                UNDERWATER_IMPACT_CLASSES[1],
                [0; 3],
                owner_entity_id,
                source_entity_type_at_birth,
            );
        }

        let randomized_rate =
            METEOR_IMPACT_RATE_MIN_16_16 + u32::from(self.next_shared_retail_random_u16() >> 3);
        self.ready_sounds.push(PositionalSoundEvent {
            sound_id: METEOR_IMPACT_SOUND_ID,
            position: raw_position_to_world(sound_position_raw),
            frequency_q16: randomized_rate,
        });
    }

    fn materialize_owned_descriptor_particle_raw(
        &mut self,
        position_raw: [i16; 3],
        source_class: u8,
        raw_velocity: [i32; 3],
        owner_entity_id: u32,
        source_entity_type_at_birth: Option<u8>,
    ) {
        let mut burst = single_descriptor_particle_burst(
            raw_position_to_world(position_raw),
            source_class,
            raw_velocity,
            Some(owner_entity_id),
        );
        burst.particles[0].source_entity_type_at_birth = source_entity_type_at_birth;
        self.materialize_particle_burst(burst);
    }

    /// Emit type 46's delayed `FUN_004475F0` wreck presentation immediately.
    ///
    /// The shared burst owns the first randomized-rate sound. The player-only
    /// wrapper then submits the same cue a second time at a fixed 1.0 rate.
    pub fn emit_player_wreck_burst_raw(
        &mut self,
        position_raw: [i16; 3],
        source_extent_raw: u16,
        sea_level_raw: Option<i16>,
        owner_entity_id: u32,
    ) {
        self.emit_scatter_surface_burst_raw(
            position_raw,
            source_extent_raw,
            sea_level_raw,
            owner_entity_id,
        );
        let origin = raw_position_to_world(position_raw);
        self.ready_sounds.push(PositionalSoundEvent {
            sound_id: PLAYER_WRECK_SOUND_ID,
            position: origin,
            frequency_q16: PLAYER_WRECK_FIXED_RATE_16_16,
        });
    }

    #[cfg(test)]
    pub(crate) fn test_particles_in_virgin_birth_order(&self) -> Vec<WorldParticle> {
        self.particles
            .test_iter_in_virgin_birth_order()
            .copied()
            .collect()
    }

    /// Run `FUN_0042F650`'s exact static-object particle gates for one visible
    /// Section-10 traversal.
    ///
    /// Every qualifying cell (`terrain_type & 8`, object kind other than 9)
    /// consumes one RNG word. A low-three-bit zero emits the class-20/42 puff.
    /// Kind zero then consumes a second word; low-four-bit zero emits the
    /// class-21/43 flame after one further word supplies its authored Y jitter.
    /// Retail runs this path once per static-object render traversal. It has no
    /// 50-Hz clock latch: captures contain repeated births from one cell under
    /// the same clock value, so every invocation must consume its own RNG.
    pub fn emit_static_terrain_object_particles<'a>(
        &mut self,
        objects: impl IntoIterator<Item = &'a StaticTerrainObjectInstance>,
        sea_level: Option<f32>,
    ) {
        for object in objects {
            if object.terrain_type & 8 == 0 || object.kind_index == 9 {
                continue;
            }

            if retail_random_u16(&mut self.rng_state) & 7 == 0 {
                let mut position = object.position;
                position[1] += f32::from(STATIC_PUFF_Y_OFFSET_RAW) / 256.0;
                let above_water = sea_level.is_none_or(|sea| sea < position[1]);
                let (source_class, raw_velocity) = if above_water {
                    (
                        STATIC_PUFF_ABOVE_WATER_CLASS,
                        STATIC_PUFF_ABOVE_WATER_VELOCITY_ARGUMENT_RAW,
                    )
                } else {
                    let jitter =
                        i32::from((retail_random_u16(&mut self.rng_state) >> 9) & 0x7f) - 0x40;
                    position[0] = v2k_core::world::wrap(position[0] + jitter as f32 / 256.0);
                    (
                        STATIC_PUFF_UNDERWATER_CLASS,
                        STATIC_PUFF_UNDERWATER_VELOCITY_ARGUMENT_RAW,
                    )
                };
                self.materialize_particle_burst(single_descriptor_particle_burst(
                    position,
                    source_class,
                    raw_velocity,
                    None,
                ));
            }

            if object.kind_index == 0 && retail_random_u16(&mut self.rng_state) & 0x0f == 0 {
                let height_jitter = (retail_random_u16(&mut self.rng_state) & 0x7f) as i16;
                let mut position = object.position;
                position[1] += f32::from(STATIC_FLAME_Y_OFFSET_MIN_RAW + height_jitter) / 256.0;
                let source_class = if sea_level.is_none_or(|sea| sea < position[1]) {
                    STATIC_FLAME_ABOVE_WATER_CLASS
                } else {
                    STATIC_FLAME_UNDERWATER_CLASS
                };
                self.materialize_particle_burst(single_descriptor_particle_burst(
                    position,
                    source_class,
                    STATIC_FLAME_VELOCITY_ARGUMENT_RAW,
                    None,
                ));
            }
        }
    }

    /// Emit `FUN_00446640`'s recurring controlled-player low-hull plume.
    ///
    /// This path has no chance gate and no 50-Hz tick latch: every successful
    /// controller callback below the strict hull threshold consumes two RNG
    /// words for X/Z placement and attempts one allocation. At or below the
    /// raw sea plane, `FUN_00441670` consumes a third word for another X
    /// displacement and substitutes the class-42 bubble for class-20 smoke.
    /// The source handle survives in the particle record even though neither
    /// class currently uses it for collision attribution.
    pub fn emit_player_low_hull_smoke_raw(
        &mut self,
        position_raw: [i16; 3],
        body_forward_q31: [i32; 3],
        source_id: u32,
        sea_level_raw: Option<i16>,
    ) {
        let x_jitter = ((self.next_shared_retail_random_u16() >> 9) as i16).wrapping_sub(0x40);
        let z_jitter = ((self.next_shared_retail_random_u16() >> 9) as i16).wrapping_sub(0x40);
        let mut effect_position_raw = [
            position_raw[0]
                .wrapping_add(x_jitter)
                .wrapping_sub(q31_mul_raw(
                    body_forward_q31[0],
                    PLAYER_LOW_HULL_SMOKE_REAR_DISTANCE_RAW,
                ) as i16),
            position_raw[1].wrapping_add(PLAYER_LOW_HULL_SMOKE_Y_OFFSET_RAW),
            position_raw[2]
                .wrapping_add(z_jitter)
                .wrapping_sub(q31_mul_raw(
                    body_forward_q31[2],
                    PLAYER_LOW_HULL_SMOKE_REAR_DISTANCE_RAW,
                ) as i16),
        ];

        let (source_class, raw_velocity) =
            if sea_level_raw.is_none_or(|sea_level| sea_level < effect_position_raw[1]) {
                (
                    STATIC_PUFF_ABOVE_WATER_CLASS,
                    STATIC_PUFF_ABOVE_WATER_VELOCITY_ARGUMENT_RAW,
                )
            } else {
                let underwater_x_jitter =
                    ((self.next_shared_retail_random_u16() >> 9) as i16).wrapping_sub(0x40);
                effect_position_raw[0] = effect_position_raw[0].wrapping_add(underwater_x_jitter);
                (
                    STATIC_PUFF_UNDERWATER_CLASS,
                    STATIC_PUFF_UNDERWATER_VELOCITY_ARGUMENT_RAW,
                )
            };

        self.materialize_particle_burst(single_descriptor_particle_burst(
            raw_position_to_world(effect_position_raw),
            source_class,
            raw_velocity,
            Some(source_id),
        ));
    }

    /// Move queued events into live particle/audio state without advancing
    /// time.  Most callers can simply call [`Self::update`].
    pub fn process_pending(&mut self) {
        self.process_pending_with_birth_context(None);
    }

    fn process_pending_with_birth_context(
        &mut self,
        birth_context: Option<ParticleBirthContext<'_>>,
    ) {
        for _ in 0..self.pending.len() {
            let event = self.pending.pop_front().expect("pending event count");
            match event {
                WorldEvent::ParticleBurst(burst) => {
                    self.materialize_particle_burst_with_birth_context(burst, birth_context)
                }
                WorldEvent::MeteorImpact {
                    position,
                    first_direction_index,
                    owner_id,
                } => self.materialize_meteor_impact(
                    position,
                    first_direction_index,
                    owner_id,
                    birth_context,
                ),
                WorldEvent::PositionalSound(sound) => self.ready_sounds.push(sound),
                event @ WorldEvent::DeferredCargoTransfer { .. } => self.pending.push_back(event),
            }
        }
    }

    /// Finish the443B50 transfer presentation at the end of the entity pass,
    /// or immediately after campaign loading. Ordinary pending processing
    /// deliberately leaves these records newborn until this named boundary.
    pub fn process_deferred_cargo_transfers(&mut self, birth: ParticleBirthContext<'_>) {
        for _ in 0..self.pending.len() {
            let event = self.pending.pop_front().expect("pending event count");
            if let WorldEvent::DeferredCargoTransfer {
                position_raw,
                parent_id,
                parent_entity_type,
            } = event
            {
                let position = raw_position_to_world(position_raw);
                let mut burst = single_descriptor_particle_burst(
                    position,
                    CARGO_TRANSFER_PARTICLE_CLASS,
                    [0; 3],
                    Some(parent_id),
                );
                burst.particles[0].source_entity_type_at_birth = Some(parent_entity_type);
                self.materialize_particle_burst_with_birth_context(burst, Some(birth));
            } else {
                self.pending.push_back(event);
            }
        }
    }

    fn materialize_particle_burst(&mut self, burst: ParticleBurst) {
        self.materialize_particle_burst_with_birth_context(burst, None);
    }

    fn materialize_particle_burst_with_birth_context(
        &mut self,
        burst: ParticleBurst,
        birth_context: Option<ParticleBirthContext<'_>>,
    ) {
        for spawn in burst.particles {
            self.materialize_particle_spawn_with_birth_context(burst.origin, spawn, birth_context);
        }
    }

    fn materialize_particle_spawn_with_birth_context(
        &mut self,
        origin: [f32; 3],
        spawn: ParticleSpawn,
        birth_context: Option<ParticleBirthContext<'_>>,
    ) -> Option<usize> {
        if spawn.lifetime_ticks == 0 {
            return None;
        }
        let mut particle = world_particle(origin, spawn);
        if let Some(context) = birth_context {
            initialize_particle_water_state(
                &mut particle,
                context.environment,
                context.retail_tick,
            );
        }
        self.particles.allocate(particle)
    }

    fn materialize_meteor_impact(
        &mut self,
        position: [f32; 3],
        first_direction_index: usize,
        owner_id: Option<u32>,
        birth_context: Option<ParticleBirthContext<'_>>,
    ) {
        let scatter_count = self
            .frame_pacing
            .scaled_count_min_one(METEOR_IMPACT_SCATTER_COUNT as i32)
            .max(0) as usize;
        let mut particles =
            meteor_impact_burst_with_scatter_count(position, first_direction_index, scatter_count)
                .particles;
        for particle in &mut particles {
            particle.owner_id = owner_id;
        }
        let surface = particles
            .pop()
            .expect("retail meteor impact always contains its surface particle");
        debug_assert_eq!(particles.len(), scatter_count);
        materialize_meteor_impact_spawns(particles, surface, |spawn| {
            self.materialize_particle_spawn_with_birth_context(position, spawn, birth_context)
        });
    }

    /// Advance one effects traversal in physical slot order.
    ///
    /// `FUN_00440120` integrates a record, invokes its class update callback,
    /// then dispatches its collision mode before advancing to the next physical
    /// slot. Keeping environment and entity callback data in one request makes
    /// that ordering explicit and lets later-slot children run in this pass.
    pub fn update(&mut self, request: ParticleUpdateRequest<'_>) -> ParticleUpdateOutcome {
        self.update_with_event_handlers(
            request,
            |_, _| ParticleCollisionCacheRefresh::Unchanged,
            |_, _| ParticleTerrainResponse::Unhandled,
        )
    }

    /// Advance one effects traversal with the recovered entity/terrain
    /// callback boundaries exposed at each physical slot.
    ///
    /// Both handlers run after the common F610 visual and before the outer
    /// traversal frees the parent. The entity handler may replace the owned
    /// collision projection so later slots see synchronous entity mutations.
    /// F800 receives a post-visual current-cell snapshot. GroundProgram instead
    /// submits the exact 28720 request at its surface callback. The response
    /// distinguishes accepted scheduler custody from an F800 result: an
    /// immediate burn joins the same ordered terrain overlay before the next
    /// physical slot. Unhandled programs remain in `unhandled_ground_programs`.
    pub fn update_with_event_handlers(
        &mut self,
        request: ParticleUpdateRequest<'_>,
        on_entity_impact: impl FnMut(
            &mut WorldFx,
            ParticleEntityImpact,
        ) -> ParticleCollisionCacheRefresh,
        on_terrain_event: impl FnMut(&mut WorldFx, ParticleTerrainEvent) -> ParticleTerrainResponse,
    ) -> ParticleUpdateOutcome {
        self.update_with_borrowed_event_handlers(request, on_entity_impact, on_terrain_event)
    }

    fn materialize_primary_impact_effect(
        &mut self,
        effect: PrimaryImpactEffect,
        suppresses_impact_damage: bool,
        birth_context: ParticleBirthContext<'_>,
    ) {
        match effect {
            PrimaryImpactEffect::HitBurst(position) => {
                self.materialize_common_hit_burst(
                    position,
                    suppresses_impact_damage,
                    birth_context,
                );
            }
            PrimaryImpactEffect::Surface {
                sound_position,
                effect_position,
                response_selector,
            } => {
                if matches!(response_selector, 6 | 7) {
                    // FUN_0043E060 consumes sound choice before playback rate,
                    // then attempts the response allocation immediately.
                    self.emit_special_surface_sound(sound_position);
                    self.materialize_particle_burst_with_birth_context(
                        special_surface_response_burst(effect_position, response_selector),
                        Some(birth_context),
                    );
                } else {
                    let attempt_count = self
                        .frame_pacing
                        .scaled_count_min_one(SURFACE_RESPONSE_BURST_COUNT as i32)
                        .max(0) as usize;
                    let burst = default_surface_response_burst(
                        effect_position,
                        response_selector,
                        &mut self.direction_cursor,
                        attempt_count,
                    );
                    self.materialize_particle_burst_with_birth_context(burst, Some(birth_context));
                    let rate = PRIMARY_TERRAIN_IMPACT_RATE_MIN_16_16
                        + u32::from(retail_random_u16(&mut self.rng_state) >> 2);
                    self.ready_sounds.push(PositionalSoundEvent {
                        sound_id: PRIMARY_TERRAIN_IMPACT_SOUND_ID,
                        position: sound_position,
                        frequency_q16: rate,
                    });
                }
            }
            PrimaryImpactEffect::UpgradedSurfaceBurst(position) => {
                let source_class = birth_context.environment.sea_level().map_or(
                    UPGRADED_PRIMARY_ABOVE_SURFACE_CLASS,
                    |sea_level| {
                        if position[1] <= sea_level {
                            UPGRADED_PRIMARY_SUBMERGED_SURFACE_CLASS
                        } else {
                            UPGRADED_PRIMARY_ABOVE_SURFACE_CLASS
                        }
                    },
                );
                self.materialize_particle_burst_with_birth_context(
                    single_descriptor_particle_burst(
                        position,
                        source_class,
                        UPGRADED_PRIMARY_SURFACE_VELOCITY_RAW,
                        None,
                    ),
                    Some(birth_context),
                );
            }
        }
    }

    fn materialize_common_hit_burst(
        &mut self,
        position: [f32; 3],
        suppresses_impact_damage: bool,
        birth_context: ParticleBirthContext<'_>,
    ) {
        // F3F610 compares against the authored flat sea plane even where the
        // terrain tile itself is dry.
        let flat_sea_level = match birth_context.environment {
            ParticleEnvironment::Dry => None,
            ParticleEnvironment::FlatWater { sea_level } => Some(sea_level),
            ParticleEnvironment::Terrain(context) => Some(context.terrain.sea_level_world_y()),
        };
        let mut burst = primary_entity_impact_burst(position, flat_sea_level);
        for child in &mut burst.particles {
            child.suppresses_impact_damage = suppresses_impact_damage;
        }
        self.materialize_particle_burst_with_birth_context(burst, Some(birth_context));
    }

    /// `FUN_0043F6E0` after `FUN_00410EB0`: `FUN_00457930`, then
    /// `FUN_0044F450(90, particle+8, 0x10000, (rng16 >> 3) + 0x10000)`.
    fn emit_fun_0043f6e0_hit_sound(&mut self, position: [f32; 3]) {
        self.emit_fun_0044f450_hit_sound(90, position);
    }

    /// `FUN_0044F450` / `FUN_004575A0` with scale `(rng16 >> 3) + 0x10000`.
    fn emit_fun_0044f450_hit_sound(&mut self, sound_id: usize, position: [f32; 3]) {
        let sample = retail_random_u16(&mut self.rng_state);
        let rate = 0x1_0000 + u32::from(sample >> 3);
        self.ready_sounds.push(PositionalSoundEvent {
            sound_id,
            position,
            frequency_q16: rate,
        });
    }

    /// `FUN_00441A50`: count `(base * DAT_004F72CC) >> 16` min 1, no early
    /// abort on allocator miss. Velocity is table X, `abs(Y >> 1)`, Z.
    fn emit_fun_00441a50(
        &mut self,
        position: [f32; 3],
        debris_class: u8,
        count_base: i32,
        owner_id: Option<u32>,
        source_entity_type_at_birth: Option<u8>,
        suppresses_impact_damage: bool,
        birth_context: ParticleBirthContext<'_>,
    ) {
        let mut remaining = self.frame_pacing.scaled_count_min_one(count_base);
        while remaining != 0 {
            self.direction_cursor = (self.direction_cursor + 1) % RETAIL_DIRECTION_TABLE_RAW.len();
            let direction = RETAIL_DIRECTION_TABLE_RAW[self.direction_cursor];
            let velocity_y = direction[1] >> 1;
            let raw_velocity = [
                i32::from(direction[0]),
                i32::from(velocity_y.wrapping_abs()),
                i32::from(direction[2]),
            ];
            let mut burst =
                single_descriptor_particle_burst(position, debris_class, raw_velocity, owner_id);
            burst.particles[0].source_entity_type_at_birth = source_entity_type_at_birth;
            burst.particles[0].suppresses_impact_damage = suppresses_impact_damage;
            let spawn = burst.particles[0];
            self.materialize_particle_spawn_with_birth_context(
                burst.origin,
                spawn,
                Some(birth_context),
            );
            remaining -= 1;
        }
    }

    /// `FUN_00441850` class-54: skip `FUN_004566E0`, then `FUN_004410B0`
    /// class `0x27`. Unsuppressed count is 1; suppressed count is
    /// `DAT_004F72CC >> 12` and a non-positive result emits nothing.
    /// Velocity is the packed direction dword `* 2` plus independent `Z * 2`.
    /// At or below the authored sea plane, `FUN_004410B0` substitutes class
    /// `0x2E` and subtracts 100 from Y.
    fn emit_fun_00441850_class_54_410b0(
        &mut self,
        position: [f32; 3],
        owner_id: Option<u32>,
        source_entity_type_at_birth: Option<u8>,
        suppresses_impact_damage: bool,
        birth_context: ParticleBirthContext<'_>,
    ) {
        let remaining = if suppresses_impact_damage {
            (self.frame_pacing.count_scale_q16 as i32) >> 12
        } else {
            1
        };
        if remaining <= 0 {
            return;
        }
        let sea_raw = match birth_context.environment {
            ParticleEnvironment::Terrain(context) => Some(context.terrain.sea_level_raw()),
            ParticleEnvironment::FlatWater { sea_level } => {
                Some(world_position_to_raw([0.0, sea_level, 0.0])[1])
            }
            ParticleEnvironment::Dry => None,
        };
        let mut remaining = remaining;
        while remaining != 0 {
            self.direction_cursor = (self.direction_cursor + 1) % RETAIL_DIRECTION_TABLE_RAW.len();
            let direction = RETAIL_DIRECTION_TABLE_RAW[self.direction_cursor];
            let raw_velocity = fun_00441850_class_54_direction_velocity(direction);
            let mut spawn_position_raw = world_position_to_raw(position);
            let mut debris_class = FUN_00441850_CLASS_54_DEBRIS_CLASS;
            if sea_raw.is_some_and(|sea| sea >= spawn_position_raw[1]) {
                debris_class = FUN_004410B0_CLASS_54_UNDERWATER_CLASS;
                spawn_position_raw[1] = spawn_position_raw[1].wrapping_sub(100);
            }
            let mut burst = single_descriptor_particle_burst(
                raw_position_to_world(spawn_position_raw),
                debris_class,
                raw_velocity,
                owner_id,
            );
            burst.particles[0].source_entity_type_at_birth = source_entity_type_at_birth;
            burst.particles[0].suppresses_impact_damage = suppresses_impact_damage;
            let spawn = burst.particles[0];
            self.materialize_particle_spawn_with_birth_context(
                burst.origin,
                spawn,
                Some(birth_context),
            );
            remaining -= 1;
        }
    }

    /// Dispatch class 5's `FUN_0043E180` surface callback after generic
    /// integration and gravity.
    ///
    /// Class 5 uses collision mode 3. Entity `FUN_0043F780` and solid-static
    /// `FUN_0043F920` already ran; this restores the water-transition and
    /// endpoint-terrain tail shared by `FUN_00440120` after those sweeps miss.
    fn dispatch_terrain_contact_surface_collision(
        particle: &mut WorldParticle,
        terrain_contact_mode: TerrainContactMode,
        context: TerrainCollisionContext<'_>,
        retail_tick: u32,
        terrain_type_mutations: &mut Vec<ParticleTerrainMutation>,
    ) -> bool {
        if particle.step_start_water_state == 2
            && particle.water_state < 2
            && displaced_water_surface(context, particle.position, retail_tick).is_some()
        {
            let material_code = nearest_material_code(context.terrain, particle.position);
            let selector = context.water_response_selectors[usize::from(material_code)];
            if !Self::apply_terrain_contact_surface_response(
                particle,
                terrain_contact_mode,
                selector,
                context.terrain,
                terrain_type_mutations,
            ) {
                return false;
            }
        }

        let collision_surface_y = context
            .terrain
            .height_at(particle.position[0], particle.position[2]);
        let radius = f32::from(particle.collision_radius_raw) / 256.0;
        if particle.position[1] - radius <= collision_surface_y {
            let material_code = nearest_material_code(context.terrain, particle.position);
            let selector = context.ground_response_selectors[usize::from(material_code)];
            return Self::apply_terrain_contact_surface_response(
                particle,
                terrain_contact_mode,
                selector,
                context.terrain,
                terrain_type_mutations,
            );
        }
        true
    }

    fn apply_terrain_contact_surface_response(
        particle: &mut WorldParticle,
        terrain_contact_mode: TerrainContactMode,
        selector: u8,
        terrain: &TerrainGrid,
        terrain_type_mutations: &mut Vec<ParticleTerrainMutation>,
    ) -> bool {
        if selector == 6 {
            // FUN_0043E1C0 divides the signed i16 velocity words with C's
            // truncation toward zero and returns zero so the record survives.
            // Unlike E230, E180 does not snap the particle to surface Y.
            let raw_velocity = particle.velocity.map(world_velocity_component_to_raw);
            particle.velocity = raw_velocity_to_world([
                raw_velocity[0] / 4,
                raw_velocity[1] / 2,
                raw_velocity[2] / 4,
            ]);
            return true;
        }

        // The recovered hive emitter requests allocator mode zero, so runtime
        // state +0x1D bit zero is clear. A class-4 Defecate-Virus request
        // instead copies the entity sign bit into +0x1D bit zero; when set,
        // E180 skips FUN_00433720 but still returns one so FUN_00440120 frees
        // the carrier. Mode 6's sibling E1A0 callback shares this tail and
        // clears the same bit. The mutation cell uses truncated unsigned 8.8
        // words, not the nearest-cell coordinates used to select the response.
        if particle.suppresses_impact_damage {
            return false;
        }
        let position_raw = world_position_to_raw(particle.position);
        let cell = [
            ((position_raw[0] as u16) >> 8) as u8,
            ((position_raw[2] as u16) >> 8) as u8,
        ];
        let terrain_type = terrain
            .cell(usize::from(cell[0]), usize::from(cell[1]))
            .map_or(0, |terrain_cell| terrain_cell.terrain_type);
        let effective_infected = effective_terrain_type(terrain_type, cell, terrain_type_mutations)
            & INFECTION_TERRAIN_TYPE_BIT
            != 0;
        let requested_infected = terrain_contact_mode.infected();
        if effective_infected != requested_infected {
            terrain_type_mutations.push(ParticleTerrainMutation::Infection {
                cell,
                infected: requested_infected,
            });
        }
        false
    }

    /// Dispatch class 19's `FUN_0043E8A0` callback after its ordinary F260
    /// gravity update. A first above-to-water transition wins; otherwise the
    /// endpoint is tested against bilinear terrain. Every callback response
    /// consumes the invisible probe after attempting its child allocations.
    fn dispatch_player_surface_probe_collision(
        &mut self,
        particle: &WorldParticle,
        context: TerrainCollisionContext<'_>,
        birth_context: ParticleBirthContext<'_>,
    ) -> bool {
        if particle.step_start_water_state == 2 && particle.water_state < 2 {
            if let Some(surface_y) =
                displaced_water_surface(context, particle.position, birth_context.retail_tick)
            {
                let position_raw = world_position_to_raw(particle.position);
                let material_code =
                    nearest_material_code_raw(context.terrain, position_raw[0], position_raw[2]);
                let selector = context.water_response_selectors[usize::from(material_code)];
                self.emit_player_surface_probe_response(
                    particle,
                    selector,
                    world_position_to_raw([0.0, surface_y, 0.0])[1],
                    context,
                    birth_context,
                );
                return false;
            }
        }

        let position_raw = world_position_to_raw(particle.position);
        let surface_raw =
            bilinear_terrain_height_raw(context.terrain, position_raw[0], position_raw[2]);
        let radius_raw = particle.collision_radius_raw as i32;
        if i32::from(position_raw[1]) - radius_raw <= i32::from(surface_raw) {
            let material_code =
                nearest_material_code_raw(context.terrain, position_raw[0], position_raw[2]);
            let selector = context.ground_response_selectors[usize::from(material_code)];
            self.emit_player_surface_probe_response(
                particle,
                selector,
                surface_raw,
                context,
                birth_context,
            );
            return false;
        }
        true
    }

    fn emit_player_surface_probe_response(
        &mut self,
        particle: &WorldParticle,
        selector: u8,
        collision_surface_raw: i16,
        context: TerrainCollisionContext<'_>,
        birth_context: ParticleBirthContext<'_>,
    ) {
        if selector == 6 {
            self.emit_player_water_downwash(particle, context, birth_context);
            return;
        }
        if matches!(selector, 11 | 12) {
            return;
        }

        let parent_raw = world_position_to_raw(particle.position);
        let attempt_count = self.frame_pacing.shifted_count(13);
        for _ in 0..attempt_count {
            // FUN_0043E8A0 consumes exactly one angle word per terrain child.
            let angle = self.next_shared_retail_random_u16();
            let radial_x_raw =
                ((retail_sine_q15(u32::from(angle.wrapping_add(0x4000))) * 128) >> 15) as i16;
            let radial_z_raw = ((retail_sine_q15(u32::from(angle)) * 128) >> 15) as i16;
            let child_x_raw = parent_raw[0].wrapping_add(radial_x_raw);
            let child_z_raw = parent_raw[2].wrapping_add(radial_z_raw);
            let child_y_raw =
                bilinear_terrain_height_raw(context.terrain, child_x_raw, child_z_raw);
            let child_position_raw = [child_x_raw, child_y_raw, child_z_raw];
            let material_code =
                nearest_material_code_raw(context.terrain, child_x_raw, child_z_raw);
            let response_selector = context.ground_response_selectors[usize::from(material_code)];
            let source_class = PLAYER_DOWNWASH_CLASS_BY_SELECTOR
                .get(usize::from(response_selector))
                .copied()
                .unwrap_or(0);
            if source_class == 0 {
                continue;
            }
            let raw_velocity = [
                i32::from(radial_x_raw) * 6,
                (i32::from(child_y_raw) - i32::from(collision_surface_raw)) * 6,
                i32::from(radial_z_raw) * 6,
            ];
            let mut burst = single_descriptor_particle_burst(
                raw_position_to_world(child_position_raw),
                source_class,
                raw_velocity,
                None,
            );
            for child in &mut burst.particles {
                // FUN_0043E8A0 copies parent runtime byte +0x1D bit zero into
                // every terrain-debris allocation request.
                child.suppresses_impact_damage = particle.suppresses_impact_damage;
            }
            self.materialize_particle_burst_with_birth_context(burst, Some(birth_context));
        }
    }

    fn emit_player_water_downwash(
        &mut self,
        particle: &WorldParticle,
        context: TerrainCollisionContext<'_>,
        birth_context: ParticleBirthContext<'_>,
    ) {
        self.emit_jittered_water_surface_spray(
            particle.position,
            context,
            birth_context,
            self.frame_pacing.shifted_count(13),
            false,
        );
    }

    /// Shared selector-six inner loop recovered from `FUN_0043E8A0` and
    /// `FUN_0043E4F0`. Both callbacks always consume X and Z jitter words. A
    /// wet point then attempts the allocation and consumes its chance word
    /// even when the fixed-pool allocator rejects; a dry point skips both.
    /// A successful sound gate consumes the two further words inside
    /// `FUN_0043E060`.
    fn emit_jittered_water_surface_spray(
        &mut self,
        parent_position: [f32; 3],
        context: TerrainCollisionContext<'_>,
        birth_context: ParticleBirthContext<'_>,
        attempt_count: usize,
        suppresses_impact_damage: bool,
    ) {
        let parent_raw = world_position_to_raw(parent_position);
        for _ in 0..attempt_count {
            let x_offset = ((self.next_shared_retail_random_u16() >> 7) as i16).wrapping_sub(0x100);
            let z_offset = ((self.next_shared_retail_random_u16() >> 7) as i16).wrapping_sub(0x100);
            let x_raw = parent_raw[0].wrapping_add(x_offset);
            let z_raw = parent_raw[2].wrapping_add(z_offset);
            let terrain_y_raw = bilinear_terrain_height_raw(context.terrain, x_raw, z_raw);
            let wave_floor_raw = coarse_terrain_height_raw(context.terrain, x_raw, z_raw);
            let wave_y_raw = wave_surface_raw(
                x_raw,
                z_raw,
                birth_context.retail_tick as i32,
                context.terrain.sea_level_raw(),
                wave_floor_raw,
            );
            if terrain_y_raw <= wave_y_raw {
                let mut burst = single_descriptor_particle_burst(
                    raw_position_to_world([x_raw, wave_y_raw, z_raw]),
                    PLAYER_WATER_DOWNWASH_PARTICLE_CLASS,
                    [0; 3],
                    None,
                );
                for child in &mut burst.particles {
                    child.suppresses_impact_damage = suppresses_impact_damage;
                }
                let origin = burst.origin;
                for spawn in burst.particles {
                    if self
                        .materialize_particle_spawn_with_birth_context(
                            origin,
                            spawn,
                            Some(birth_context),
                        )
                        .is_some()
                    {
                        self.splash_path_counters.spray_particles_spawned += 1;
                    }
                }

                // The chance word follows every wet allocation attempt,
                // including a rejected one. A successful 1-in-32 gate
                // consumes the authored sound choice and playback-rate words
                // inside FUN_0043E060.
                if self.next_shared_retail_random_u16() & 0x1f == 0 {
                    self.emit_special_surface_sound(parent_position);
                }
            }
        }
    }

    /// Dispatch the mode-3 `FUN_0043E4F0` surface callback after its
    /// descriptor-selected update callback. An above-to-water transition is
    /// handled first, but every E4F0 response preserves the parent, so the
    /// terrain endpoint can invoke the callback again in the same traversal.
    fn dispatch_ballistic_surface_collision(
        &mut self,
        slot: usize,
        particle: &mut WorldParticle,
        context: TerrainCollisionContext<'_>,
        birth_context: ParticleBirthContext<'_>,
    ) -> bool {
        if particle.step_start_water_state == 2 && particle.water_state < 2 {
            if let Some(surface_y) =
                displaced_water_surface(context, particle.position, birth_context.retail_tick)
            {
                let material_code = nearest_material_code(context.terrain, particle.position);
                let selector = context.water_response_selectors[usize::from(material_code)];
                if !self.apply_ballistic_surface_response(
                    slot,
                    particle,
                    selector,
                    surface_y,
                    context,
                    birth_context,
                ) {
                    return false;
                }
            }
        }

        let collision_surface_y = context
            .terrain
            .height_at(particle.position[0], particle.position[2]);
        let radius = f32::from(particle.collision_radius_raw) / 256.0;
        if particle.position[1] - radius <= collision_surface_y {
            let material_code = nearest_material_code(context.terrain, particle.position);
            let selector = context.ground_response_selectors[usize::from(material_code)];
            let response_surface_y =
                coarse_particle_ground_response_y(context.terrain, particle.position);
            return self.apply_ballistic_surface_response(
                slot,
                particle,
                selector,
                response_surface_y,
                context,
                birth_context,
            );
        }
        true
    }

    fn apply_ballistic_surface_response(
        &mut self,
        slot: usize,
        particle: &mut WorldParticle,
        selector: u8,
        surface_y: f32,
        context: TerrainCollisionContext<'_>,
        birth_context: ParticleBirthContext<'_>,
    ) -> bool {
        if selector == 6 {
            // E4F0 performs all spray/audio attempts before damping the live
            // parent. Its direct `DAT_004F72CC >> 13` count deliberately has
            // no min-one fallback after a severe frame stall.
            self.emit_jittered_water_surface_spray(
                particle.position,
                context,
                birth_context,
                self.frame_pacing.shifted_count(13),
                particle.suppresses_impact_damage,
            );
            let raw_velocity = particle.velocity.map(world_velocity_component_to_raw);
            particle.velocity = raw_velocity_to_world([
                raw_velocity[0] / 4,
                raw_velocity[1] / 2,
                raw_velocity[2] / 4,
            ]);
            self.particles.slots[slot] = Some(*particle);
            return true;
        }

        particle.position[1] = surface_y;
        let vertical_raw = world_velocity_component_to_raw(particle.velocity[1]);
        particle.velocity[1] = raw_velocity_to_world([0, -(vertical_raw / 2), 0])[1];
        self.particles.slots[slot] = Some(*particle);

        let source_class = BALLISTIC_SURFACE_RESPONSE_CLASS_BY_SELECTOR
            .get(usize::from(selector))
            .copied()
            .expect("authored E4F0 surface selector has a response class");
        let attempt_count = self
            .frame_pacing
            .scaled_count_min_one(SURFACE_RESPONSE_BURST_COUNT as i32)
            .max(0) as usize;
        let mut burst = directional_surface_response_burst(
            particle.position,
            source_class,
            None,
            &mut self.direction_cursor,
            attempt_count,
        );
        for child in &mut burst.particles {
            child.suppresses_impact_damage = particle.suppresses_impact_damage;
        }
        self.materialize_particle_burst_with_birth_context(burst, Some(birth_context));
        true
    }

    fn dispatch_mode_one_surface_collision(
        &mut self,
        slot: usize,
        particle: &mut WorldParticle,
        context: TerrainCollisionContext<'_>,
        birth_context: ParticleBirthContext<'_>,
    ) -> bool {
        // Mode 1 has no entity/static-model sweep. It first handles an
        // above-to-water transition, then (unless E230 returned one) performs
        // the endpoint terrain test and may invoke E230 a second time.
        if particle.step_start_water_state == 2 && particle.water_state < 2 {
            if let Some(surface_y) =
                displaced_water_surface(context, particle.position, birth_context.retail_tick)
            {
                let material_code = nearest_material_code(context.terrain, particle.position);
                let selector = context.water_response_selectors[usize::from(material_code)];
                if !self.apply_mode_one_surface_response(
                    slot,
                    particle,
                    selector,
                    surface_y,
                    birth_context,
                ) {
                    return false;
                }
            }
        }

        // FUN_00440120's near-surface test uses the four-corner interpolated
        // terrain height. Only the later FUN_0043DB60 response chooses this
        // descriptor family's coarse current-cell snap height.
        let collision_surface_y = context
            .terrain
            .height_at(particle.position[0], particle.position[2]);
        let radius = f32::from(particle.collision_radius_raw) / 256.0;
        if particle.position[1] - radius <= collision_surface_y {
            let material_code = nearest_material_code(context.terrain, particle.position);
            let selector = context.ground_response_selectors[usize::from(material_code)];
            let response_surface_y =
                coarse_particle_ground_response_y(context.terrain, particle.position);
            return self.apply_mode_one_surface_response(
                slot,
                particle,
                selector,
                response_surface_y,
                birth_context,
            );
        }
        true
    }

    fn apply_mode_one_surface_response(
        &mut self,
        slot: usize,
        particle: &mut WorldParticle,
        selector: u8,
        surface_y: f32,
        birth_context: ParticleBirthContext<'_>,
    ) -> bool {
        if selector != 6 {
            particle.position[1] = surface_y;
            let vertical_raw = world_velocity_component_to_raw(particle.velocity[1]);
            particle.velocity[1] = raw_velocity_to_world([0, -(vertical_raw / 2), 0])[1];
            return true;
        }

        // E230 consumes the 1-in-4 gate even when no audio is emitted. E060
        // consumes the following choice/rate words only on a successful gate.
        if retail_random_u16(&mut self.rng_state) & 3 == 0 {
            self.emit_special_surface_sound(particle.position);
        }

        let effect_position = [particle.position[0], surface_y, particle.position[2]];
        self.particles.slots[slot] = Some(*particle);
        self.materialize_particle_burst_with_birth_context(
            special_surface_response_burst(effect_position, 6),
            Some(birth_context),
        );
        false
    }

    fn emit_special_surface_sound(&mut self, position: [f32; 3]) {
        self.splash_path_counters.splash_sounds += 1;
        let sound_roll = retail_random_u16(&mut self.rng_state);
        let sound_id = if sound_roll & 0x1000 != 0 {
            SPECIAL_SURFACE_SOUND_IDS[0]
        } else {
            SPECIAL_SURFACE_SOUND_IDS[1]
        };
        let rate =
            SPECIAL_SURFACE_RATE_MIN_16_16 + u32::from(retail_random_u16(&mut self.rng_state) >> 2);
        self.ready_sounds.push(PositionalSoundEvent {
            sound_id,
            position,
            frequency_q16: rate,
        });
    }

    /// Transfer positional one-shots to the audio layer in emission order.
    pub fn take_positional_sounds(&mut self) -> Vec<PositionalSoundEvent> {
        std::mem::take(&mut self.ready_sounds)
    }

    /// Garbage-collect disposable logical one-shot requests before a world
    /// abort, matching `FUN_0044C940`'s selective list sweep.
    ///
    /// `WorldFx` owns only the not-yet-submitted disposable subset: queued
    /// positional events and materialized requests waiting for the audio
    /// layer. Existing mixer voices and owner-held persistent loops live
    /// outside this queue. Retaining every non-sound event preserves particle
    /// order and does not consume the shared random stream.
    pub fn garbage_collect_disposable_positional_sounds(&mut self) {
        self.pending
            .retain(|event| !matches!(event, WorldEvent::PositionalSound(_)));
        self.ready_sounds.clear();
    }

    pub fn clear(&mut self) {
        self.pending.clear();
        self.particles.clear_active();
        self.exploding_rings.clear();
        self.ready_sounds.clear();
        self.last_retail_tick = None;
    }

    /// Enqueue one full-gain, fixed-rate positional sound at signed-word
    /// retail coordinates.
    pub fn queue_fixed_positional_sound_raw(&mut self, sound_id: u16, position_raw: [i16; 3]) {
        self.queue_fixed_positional_sound_raw_at_rate(sound_id, position_raw, 0x1_0000);
    }

    /// Submit a synchronous fixed-rate cue before another immediate effect.
    /// Class49's 10C10 death cue precedes BAF0's scatter/sound suffix; deferring
    /// this request through `pending` would reverse those audible requests.
    pub fn emit_fixed_positional_sound_raw(&mut self, sound_id: u16, position_raw: [i16; 3]) {
        self.ready_sounds.push(PositionalSoundEvent::fixed(
            usize::from(sound_id),
            raw_position_to_world(position_raw),
        ));
    }

    /// Enqueue one full-gain positional sound at a fixed unsigned 16.16
    /// playback rate. Retail `FUN_0044F450` receives gain and rate as its
    /// third and fourth arguments; this helper models the common full-gain
    /// calls without misrepresenting those values as a randomized range.
    pub fn queue_fixed_positional_sound_raw_at_rate(
        &mut self,
        sound_id: u16,
        position_raw: [i16; 3],
        rate_q16: u32,
    ) {
        self.queue(WorldEvent::PositionalSound(PositionalSoundEvent {
            sound_id: usize::from(sound_id),
            position: raw_position_to_world(position_raw),
            frequency_q16: rate_q16,
        }));
    }

    /// `281A0` allocates before its next opcode and before particle traversal.
    /// Deferring through the event queue reverses fixed-pool custody against
    /// synchronous crater/scatter listeners and changes their admission.
    /// Emit opcode 4's class-18 effect through `FUN_004410B0`.
    ///
    /// Positions strictly above the signed raw sea plane retain class 18.
    /// `4410B0` reads that plane independently of the water-render gate; None
    /// is only the detached no-terrain fallback. At or below the plane it invokes
    /// `FUN_00440A20`: class 45 at the supplied position followed by class 46
    /// with X shifted by `-0x40` raw. All three allocator calls receive zero
    /// input velocity; `single_descriptor_particle_burst` then applies each
    /// descriptor's authored Y-velocity bias exactly once.
    pub fn emit_static_effect18_raw(&mut self, position_raw: [i16; 3], sea_level_raw: Option<i16>) {
        if sea_level_raw.is_none_or(|sea_level| sea_level < position_raw[1]) {
            self.materialize_owned_descriptor_particle_raw(
                position_raw,
                METEOR_SURFACE_CLASS,
                [0; 3],
                0,
                Some(0),
            );
            return;
        }

        self.materialize_owned_descriptor_particle_raw(
            position_raw,
            UNDERWATER_IMPACT_CLASSES[0],
            [0; 3],
            0,
            Some(0),
        );
        let mut second_position_raw = position_raw;
        second_position_raw[0] = second_position_raw[0].wrapping_sub(0x40);
        self.materialize_owned_descriptor_particle_raw(
            second_position_raw,
            UNDERWATER_IMPACT_CLASSES[1],
            [0; 3],
            0,
            Some(0),
        );
    }

    /// Emit kind 29's ordinary opcode-3 class-79 event.
    ///
    /// `FUN_004281A0` supplies zero input velocity to `FUN_004410B0`.
    /// Class 79 follows that routine's ordinary path unchanged, so this does
    /// not inherit class 18's water-surface substitution. Its owner argument
    /// is the zero `DAT_004DCA00` sentinel: `FUN_00440A60` retains that dword
    /// and writes birth type zero after the failed entity lookup. These are
    /// known packet words, distinct from an emitter with unknown provenance.
    pub fn emit_static_effect79_raw(&mut self, position_raw: [i16; 3]) {
        let mut burst = single_descriptor_particle_burst(
            raw_position_to_world(position_raw),
            STATIC_DESTRUCTION_EFFECT_CLASS_79,
            [0; 3],
            Some(0),
        );
        burst.particles[0].source_entity_type_at_birth = Some(0);
        self.materialize_particle_burst(burst);
    }

    /// Enqueue opcode 4's class-18 effect through `FUN_004410B0`.
    ///
    /// Positions strictly above the signed raw sea plane retain class 18.
    /// `4410B0` reads that plane independently of the water-render gate; None
    /// is only the detached no-terrain fallback. At or below the plane it invokes
    /// `FUN_00440A20`: class 45 at the supplied position followed by class 46
    /// with X shifted by `-0x40` raw. All three allocator calls receive zero
    /// input velocity; `single_descriptor_particle_burst` then applies each
    /// descriptor's authored Y-velocity bias exactly once.
    pub fn queue_static_effect18_raw(
        &mut self,
        position_raw: [i16; 3],
        sea_level_raw: Option<i16>,
    ) {
        if sea_level_raw.is_none_or(|sea_level| sea_level < position_raw[1]) {
            self.queue(WorldEvent::ParticleBurst(single_descriptor_particle_burst(
                raw_position_to_world(position_raw),
                METEOR_SURFACE_CLASS,
                [0; 3],
                None,
            )));
            return;
        }

        self.queue(WorldEvent::ParticleBurst(single_descriptor_particle_burst(
            raw_position_to_world(position_raw),
            UNDERWATER_IMPACT_CLASSES[0],
            [0; 3],
            None,
        )));
        let mut second_position_raw = position_raw;
        second_position_raw[0] = second_position_raw[0].wrapping_sub(0x40);
        self.queue(WorldEvent::ParticleBurst(single_descriptor_particle_burst(
            raw_position_to_world(second_position_raw),
            UNDERWATER_IMPACT_CLASSES[1],
            [0; 3],
            None,
        )));
    }

    /// Enqueue kind 29's ordinary opcode-3 class-79 event.
    ///
    /// `FUN_004281A0` supplies zero input velocity to `FUN_004410B0`.
    /// Class 79 follows that routine's ordinary path unchanged, so this does
    /// not inherit class 18's water-surface substitution. Its owner argument
    /// is the zero `DAT_004DCA00` sentinel: `FUN_00440A60` retains that dword
    /// and writes birth type zero after the failed entity lookup. These are
    /// known packet words, distinct from an emitter with unknown provenance.
    pub fn queue_static_effect79_raw(&mut self, position_raw: [i16; 3]) {
        let mut burst = single_descriptor_particle_burst(
            raw_position_to_world(position_raw),
            STATIC_DESTRUCTION_EFFECT_CLASS_79,
            [0; 3],
            Some(0),
        );
        burst.particles[0].source_entity_type_at_birth = Some(0);
        self.queue(WorldEvent::ParticleBurst(burst));
    }

    /// Enqueue retail's positional cargo transfer cue without conflating it
    /// with the type-93 proxy's later movement particle.
    pub fn queue_cargo_transfer_sound(&mut self, position: [f32; 3]) {
        self.queue(WorldEvent::PositionalSound(PositionalSoundEvent::fixed(
            CARGO_TRANSFER_SOUND_ID,
            position,
        )));
    }

    pub(crate) fn queue_deferred_cargo_transfer(
        &mut self,
        position_raw: [i16; 3],
        parent_id: u32,
        parent_entity_type: u8,
    ) {
        self.queue(WorldEvent::DeferredCargoTransfer {
            position_raw,
            parent_id,
            parent_entity_type,
        });
        // The request is made after the particle packet even if the pool
        // later rejects it. Keep audio in the ordinary sound owner, so an
        // intervening mute/discard also consumes this request.
        self.queue_cargo_transfer_sound(raw_position_to_world(position_raw));
    }

    pub fn queue_cargo_full_sound(&mut self, position: [f32; 3]) {
        self.queue(WorldEvent::PositionalSound(PositionalSoundEvent::fixed(
            CARGO_FULL_SOUND_ID,
            position,
        )));
    }

    /// Enqueue one exact class-0x33 particle at the proxy/collected cargo.
    pub fn queue_cargo_transfer_particle(&mut self, position: [f32; 3], owner_id: Option<u32>) {
        let frame = first_particle_frame(CARGO_TRANSFER_PARTICLE_CLASS);
        self.queue(WorldEvent::ParticleBurst(ParticleBurst {
            origin: position,
            particles: vec![ParticleSpawn {
                sprite_id: frame.sprite_id,
                source_class: CARGO_TRANSFER_PARTICLE_CLASS,
                offset: [0.0; 3],
                presentation_offset: [0.0; 3],
                velocity: [0.0; 3],
                owner_id,
                attached_owner_handle: None,
                attached_offset_raw: [0; 3],
                source_entity_type_at_birth: None,
                suppresses_impact_damage: false,
                lifetime_ticks: CARGO_TRANSFER_LIFETIME_TICKS,
                collision_radius_raw: CARGO_TRANSFER_RADIUS_RAW,
                frame_middle_raw: frame.middle_raw,
                frame_scale_raw: frame.scale_raw,
                animation_rate_raw: CARGO_TRANSFER_ANIMATION_RATE_RAW,
                draw_scale_raw: CARGO_TRANSFER_DRAW_SCALE_RAW,
                size_jitter_divisor_raw: 0,
            }],
        }));
    }

    /// Enqueue one `FUN_00424650` callback's firing batch.
    ///
    /// Retail's firing transaction attempts the descriptor-selected
    /// projectile before finite-ammunition commit and submits one sound after
    /// the catch-up loop, even when it emits multiple shots. The selector-2
    /// class-3 flare nevertheless has no
    /// visible particle at or below the water plane; preserve the discharge
    /// (and therefore caller-owned ammunition and audio) while condensing that
    /// immediate underwater lifecycle to no queued class-3 particle. The
    /// default and rapid class-1/2 bullets remain valid underwater.
    ///
    /// The separate class-15 command resolves to class-32 flash/smoke only
    /// while the shooter origin is strictly above the water plane; dry levels
    /// take the visible path too.
    pub fn queue_primary_fire_batch(
        &mut self,
        events: &[PrimaryFireEvent],
        sea_level: Option<f32>,
    ) {
        for event in events {
            if event.delivery == crate::primary_weapon::PlayerProjectileDelivery::Particle {
                let projectile_class = event.projectile.spec.projectile_class;
                assert!(
                    is_emitted_primary_projectile(projectile_class),
                    "unsupported primary projectile class {projectile_class}"
                );
                let descriptor = particle_descriptor(projectile_class)
                    .expect("emitted primary projectile has a retail descriptor");
                let bullet_frame = first_particle_frame(projectile_class);
                debug_assert_eq!(event.projectile.spec.sprite_id, bullet_frame.sprite_id);
                debug_assert_eq!(
                    event.projectile.spec.lifetime_ticks,
                    descriptor.lifetime_ticks()
                );
                let above_water = sea_level
                    .map(|water_y| water_y < event.auxiliary.origin_world[1])
                    .unwrap_or(true);
                let visible_projectile =
                    projectile_class != UPGRADED_PRIMARY_PARTICLE_CLASS || above_water;
                if visible_projectile {
                    // 424650 -> 44E770 -> 4410B0 selects each plasma's own
                    // submerged descriptor at/below the flat sea plane. Keep
                    // this birth policy shared with native turret emissions.
                    let projectile_class = if above_water {
                        projectile_class
                    } else if let Some(combat_projectiles::CombatProjectileProgram::TurretBolt(
                        pair,
                    )) =
                        combat_projectiles::CombatProjectileProgram::for_class(projectile_class)
                    {
                        pair.submerged()
                    } else {
                        projectile_class
                    };
                    let descriptor = particle_descriptor(projectile_class)
                        .expect("resolved primary projectile has a retail descriptor");
                    let bullet_frame = first_particle_frame(projectile_class);
                    self.queue(WorldEvent::ParticleBurst(ParticleBurst {
                        origin: event.projectile.origin_world,
                        particles: vec![ParticleSpawn {
                            sprite_id: bullet_frame.sprite_id,
                            source_class: projectile_class,
                            offset: [0.0; 3],
                            presentation_offset: presentation_delta(
                                event.projectile.presentation_basis.origin_world,
                                event.projectile.origin_world,
                            ),
                            velocity: event.projectile.velocity_world_per_second,
                            owner_id: Some(event.projectile.owner_id),
                            attached_owner_handle: None,
                            attached_offset_raw: [0; 3],
                            source_entity_type_at_birth: event.source_entity_type_at_birth,
                            suppresses_impact_damage: false,
                            lifetime_ticks: descriptor.lifetime_ticks(),
                            collision_radius_raw: descriptor.collision_radius_raw(),
                            frame_middle_raw: bullet_frame.middle_raw,
                            frame_scale_raw: bullet_frame.scale_raw,
                            animation_rate_raw: descriptor.animation_rate_raw(),
                            draw_scale_raw: descriptor.draw_scale_raw(),
                            size_jitter_divisor_raw: descriptor.size_jitter_divisor_raw(),
                        }],
                    }));
                }
            }
            let above_water =
                sea_level.is_none_or(|water_y| water_y < event.auxiliary.origin_world[1]);
            if event.emits_auxiliary && above_water {
                let muzzle_frame = first_particle_frame(PRIMARY_MUZZLE_PARTICLE_CLASS);
                self.queue(WorldEvent::ParticleBurst(ParticleBurst {
                    origin: event.auxiliary.origin_world,
                    particles: vec![ParticleSpawn {
                        sprite_id: muzzle_frame.sprite_id,
                        source_class: PRIMARY_MUZZLE_PARTICLE_CLASS,
                        offset: [0.0, f32::from(PRIMARY_MUZZLE_Y_OFFSET_RAW) / 256.0, 0.0],
                        presentation_offset: presentation_delta(
                            event.auxiliary.presentation_origin_world,
                            event.auxiliary.origin_world,
                        ),
                        velocity: event.auxiliary.inherited_velocity_world_per_second,
                        owner_id: Some(event.auxiliary.owner_id),
                        attached_owner_handle: None,
                        attached_offset_raw: [0; 3],
                        source_entity_type_at_birth: event.source_entity_type_at_birth,
                        suppresses_impact_damage: false,
                        lifetime_ticks: PRIMARY_MUZZLE_LIFETIME_TICKS,
                        collision_radius_raw: PRIMARY_MUZZLE_RADIUS_RAW,
                        frame_middle_raw: muzzle_frame.middle_raw,
                        frame_scale_raw: muzzle_frame.scale_raw,
                        animation_rate_raw: PRIMARY_MUZZLE_ANIMATION_RATE_RAW,
                        draw_scale_raw: PRIMARY_MUZZLE_DRAW_SCALE_RAW,
                        size_jitter_divisor_raw: 0,
                    }],
                }));
            }
        }
        if let Some(event) = events.last() {
            self.queue(WorldEvent::PositionalSound(PositionalSoundEvent::fixed(
                event.sound.sound_id,
                event.sound.origin_world,
            )));
        }
    }

    /// Enqueue a diagnostic meteor burst and a fixed native-rate sound.
    ///
    /// `first_direction_index` is the first direction-table entry consumed by
    /// the burst, after retail increments its process-global cursor. Keeping
    /// that lookup explicit lets captured impacts reproduce their exact debris
    /// vectors without disguising the value as a private random seed. This
    /// diagnostic helper does not reproduce BAF0/40950's emission sequence;
    /// native Intro2 uses `emit_intro2_meteor_impact_raw`, which samples its
    /// sound pitch after the real scatter and surface allocations.
    pub fn queue_meteor_impact(
        &mut self,
        position: [f32; 3],
        first_direction_index: usize,
        owner_id: Option<u32>,
    ) {
        self.queue(WorldEvent::MeteorImpact {
            position,
            first_direction_index,
            owner_id,
        });
        self.queue(WorldEvent::PositionalSound(PositionalSoundEvent::fixed(
            METEOR_IMPACT_SOUND_ID,
            position,
        )));
    }

    /// Gate and enqueue one meteor impact while a timeline moves forward.
    #[allow(clippy::too_many_arguments)]
    pub fn queue_meteor_impact_once(
        &mut self,
        timeline: &mut OneShotTimeline,
        event_id: u64,
        previous_secs: f32,
        current_secs: f32,
        impact_secs: f32,
        position: [f32; 3],
        first_direction_index: usize,
        owner_id: Option<u32>,
    ) -> bool {
        if !timeline.crossed_once(event_id, previous_secs, current_secs, impact_secs) {
            return false;
        }
        self.queue_meteor_impact(position, first_direction_index, owner_id);
        true
    }

    /// Enqueue one stationary, entity-owned sample of the incoming meteor
    /// wake. Pool priority and descriptor animation remain shared with every
    /// other retail particle rather than becoming cinematic-only state.
    pub fn queue_meteor_wake(&mut self, position: [f32; 3], owner_id: u32) {
        self.queue(WorldEvent::ParticleBurst(single_descriptor_particle_burst(
            position,
            METEOR_WAKE_CLASS,
            METEOR_WAKE_INITIAL_VELOCITY_RAW,
            Some(owner_id),
        )));
    }
}

/// Build the deterministic meteor particle burst without mutating a queue.
pub fn meteor_impact_burst(position: [f32; 3], first_direction_index: usize) -> ParticleBurst {
    meteor_impact_burst_with_scatter_count(
        position,
        first_direction_index,
        METEOR_IMPACT_SCATTER_COUNT,
    )
}

fn meteor_impact_burst_with_scatter_count(
    position: [f32; 3],
    first_direction_index: usize,
    scatter_count: usize,
) -> ParticleBurst {
    let mut particles = Vec::with_capacity(scatter_count + METEOR_IMPACT_SURFACE_COUNT);
    for particle_index in 0..scatter_count {
        let direction = RETAIL_DIRECTION_TABLE_RAW
            [(first_direction_index + particle_index) % RETAIL_DIRECTION_TABLE_RAW.len()];
        particles.push(meteor_scatter_particle(
            direction,
            METEOR_IMPACT_SOURCE_EXTENT_RAW,
            None,
        ));
    }

    // The captured impact Y is above `sea_level >> 8`, so `FUN_00440950`
    // takes its class-0x12 surface branch exactly once. Age starts at zero, so
    // the first surface frame is always sprite 873.
    let surface_frame = first_particle_frame(METEOR_SURFACE_CLASS);
    particles.push(ParticleSpawn {
        sprite_id: surface_frame.sprite_id,
        source_class: METEOR_SURFACE_CLASS,
        offset: [0.0; 3],
        presentation_offset: [0.0; 3],
        velocity: raw_velocity_to_world([0, 500, 0]),
        owner_id: None,
        attached_owner_handle: None,
        attached_offset_raw: [0; 3],
        source_entity_type_at_birth: None,
        suppresses_impact_damage: false,
        lifetime_ticks: METEOR_SURFACE_LIFETIME_TICKS,
        collision_radius_raw: METEOR_SURFACE_RADIUS_RAW,
        frame_middle_raw: surface_frame.middle_raw,
        frame_scale_raw: surface_frame.scale_raw,
        animation_rate_raw: METEOR_SURFACE_ANIMATION_RATE_RAW,
        draw_scale_raw: METEOR_SURFACE_DRAW_SCALE_RAW,
        size_jitter_divisor_raw: METEOR_SURFACE_SIZE_JITTER_DIVISOR_RAW,
    });

    ParticleBurst {
        origin: position,
        particles,
    }
}

/// Build one `FUN_004407D0(..., class 0x10, scale 8)` allocation argument.
///
/// `source_extent_raw` is caller-owned: Intro2's captured impacts use 0x100,
/// while `FUN_004475F0` passes half the active wreck model's `+0x08` radius.
fn meteor_scatter_particle(
    direction: [i16; 3],
    source_extent_raw: u16,
    owner_id: Option<u32>,
) -> ParticleSpawn {
    // `FUN_00440A60` initializes age to zero, so every class-0x10 particle
    // begins on frame zero. `WorldParticle::current_frame` advances it by the
    // recovered age formula.
    let frame = first_particle_frame(METEOR_SCATTER_CLASS);

    // `FUN_004407D0(..., 0x10, 8)` scales each table component by 8/16.
    // Particle positions are 8.8 fixed point. `FUN_00440120` integrates them
    // with `(dt_us >> 5) * velocity >> 15`, so one real second scales raw
    // velocity by 1_000_000 / 2^20 before the 8.8 conversion.
    let raw_velocity = direction.map(|component| (i32::from(component) * 8) >> 4);
    let l1_length = direction
        .iter()
        .map(|component| i32::from(*component).abs())
        .sum::<i32>();
    let offset_raw = if l1_length == 0 {
        [0; 3]
    } else {
        let extent_raw = i32::from(source_extent_raw);
        let x = extent_raw * i32::from(direction[0]) / l1_length;
        let y = extent_raw * i32::from(direction[1]) / l1_length;
        // Retail `FUN_004407D0` stores the normalized Y component into both
        // Y and Z. Velocity still uses the independent Z component above.
        [x, y, y]
    };

    ParticleSpawn {
        sprite_id: frame.sprite_id,
        source_class: METEOR_SCATTER_CLASS,
        offset: offset_raw.map(|component| component as f32 / 256.0),
        presentation_offset: [0.0; 3],
        velocity: raw_velocity_to_world(raw_velocity),
        owner_id,
        attached_owner_handle: None,
        attached_offset_raw: [0; 3],
        source_entity_type_at_birth: None,
        suppresses_impact_damage: false,
        lifetime_ticks: METEOR_SCATTER_LIFETIME_TICKS,
        collision_radius_raw: METEOR_SCATTER_RADIUS_RAW,
        frame_middle_raw: frame.middle_raw,
        frame_scale_raw: frame.scale_raw,
        animation_rate_raw: METEOR_SCATTER_ANIMATION_RATE_RAW,
        draw_scale_raw: METEOR_SCATTER_DRAW_SCALE_RAW,
        size_jitter_divisor_raw: METEOR_SCATTER_SIZE_JITTER_DIVISOR_RAW,
    }
}

fn world_particle(origin: [f32; 3], spawn: ParticleSpawn) -> WorldParticle {
    let position = [
        v2k_core::world::wrap(origin[0] + spawn.offset[0]),
        origin[1] + spawn.offset[1],
        v2k_core::world::wrap(origin[2] + spawn.offset[2]),
    ];
    WorldParticle {
        sprite_id: spawn.sprite_id,
        source_class: spawn.source_class,
        position,
        presentation_offset: spawn.presentation_offset,
        step_start_position: position,
        step_start_water_state: 2,
        water_state: 2,
        water_state_initialized: false,
        velocity: spawn.velocity,
        owner_id: spawn.owner_id,
        attached_owner_handle: spawn.attached_owner_handle,
        attached_offset_raw: spawn.attached_offset_raw,
        source_entity_type_at_birth: spawn.source_entity_type_at_birth,
        suppresses_impact_damage: spawn.suppresses_impact_damage,
        pending_destruction: false,
        age_ticks: 0.0,
        lifetime_ticks: spawn.lifetime_ticks,
        collision_radius_raw: spawn.collision_radius_raw,
        frame_middle_raw: spawn.frame_middle_raw,
        frame_scale_raw: spawn.frame_scale_raw,
        animation_rate_raw: spawn.animation_rate_raw,
        draw_scale_raw: spawn.draw_scale_raw,
        size_jitter_divisor_raw: spawn.size_jitter_divisor_raw,
    }
}

fn initialize_particle_water_state(
    particle: &mut WorldParticle,
    environment: ParticleEnvironment<'_>,
    retail_tick: u32,
) {
    let radius = f32::from(particle.collision_radius_raw) / 256.0;
    particle.water_state = classify_particle_water(
        environment.terrain_context(),
        environment.sea_level(),
        particle.position,
        radius,
        retail_tick,
    );
    particle.step_start_water_state = particle.water_state;
    particle.water_state_initialized = true;
}

/// Preserve `FUN_00440950`'s asymmetric allocator-failure boundary: the
/// class-16 scatter loop stops at its first rejected record, then the class-18
/// surface allocation is still attempted exactly once.
fn materialize_meteor_impact_spawns(
    scatter: impl IntoIterator<Item = ParticleSpawn>,
    surface: ParticleSpawn,
    mut allocate: impl FnMut(ParticleSpawn) -> Option<usize>,
) {
    for spawn in scatter {
        if allocate(spawn).is_none() {
            break;
        }
    }
    allocate(surface);
}

fn raw_velocity_to_world(raw_velocity: [i32; 3]) -> [f32; 3] {
    raw_velocity.map(|component| component as f32 * (1_000_000.0 / 1_048_576.0) / 256.0)
}

fn world_velocity_component_to_raw(component: f32) -> i32 {
    (component * 256.0 * (1_048_576.0 / 1_000_000.0)).round() as i32
}

/// Signed Q31 multiply used by the retail entity callbacks when applying a
/// signed 8.8 distance along one body-basis component. The high word of the
/// 64-bit product is the exact `imul`/`shl`/`rcl` result in `FUN_00446640`.
fn q31_mul_raw(component_q31: i32, distance_raw: i32) -> i32 {
    ((i64::from(component_q31) * i64::from(distance_raw)) >> 31) as i32
}

fn presentation_delta(presentation: [f32; 3], simulation: [f32; 3]) -> [f32; 3] {
    [
        v2k_core::world::delta(presentation[0], simulation[0]),
        presentation[1] - simulation[1],
        v2k_core::world::delta(presentation[2], simulation[2]),
    ]
}

/// Exact signed-8.8 bilinear terrain lookup used by FUN_0043E8A0 and the
/// downwash-debris callback. The 256x256 grid wraps independently of signed
/// world-word interpretation.
fn bilinear_terrain_height_raw(terrain: &TerrainGrid, x_raw: i16, z_raw: i16) -> i16 {
    terrain.bilinear_height_raw(x_raw, z_raw)
}

fn nearest_material_code(terrain: &TerrainGrid, position: [f32; 3]) -> u8 {
    let x = (position[0] + 0.5).floor().rem_euclid(256.0) as usize;
    let z = (position[2] + 0.5).floor().rem_euclid(256.0) as usize;
    terrain.cell(x, z).map_or(0, |cell| cell.terrain_type & 7)
}

/// Exact raw nearest-cell material lookup used by class 19's terrain-downwash
/// callback only.
///
/// `FUN_0043E8A0` rounds each unsigned 8.8 position word by adding `0x80`
/// with word wrap, then reads only the selected cell's terrain-type low three
/// bits. Its preceding reads of the coarse cell's attribute and high type bits
/// cannot survive the executable's later `& 0x700`; neither an authored
/// triangle choice nor the Section-10 attribute participates in this result.
fn nearest_material_code_raw(terrain: &TerrainGrid, x_raw: i16, z_raw: i16) -> u8 {
    let x = usize::from((x_raw as u16).wrapping_add(0x80) >> 8);
    let z = usize::from((z_raw as u16).wrapping_add(0x80) >> 8);
    terrain.cell(x, z).map_or(0, |cell| cell.terrain_type & 7)
}

fn coarse_terrain_height_raw(terrain: &TerrainGrid, x_raw: i16, z_raw: i16) -> i16 {
    let cell_x = usize::from((x_raw as u16) >> 8);
    let cell_z = usize::from((z_raw as u16) >> 8);
    terrain
        .cell(cell_x, cell_z)
        .map_or(0, |cell| i16::from(cell.height as i8) << 5)
}

fn coarse_particle_ground_response_y(terrain: &TerrainGrid, position: [f32; 3]) -> f32 {
    // Every descriptor in the E230/mode-1 family and the E4F0/mode-3 family
    // carries runtime flag 0x04. Both therefore take FUN_0043DB60's coarse
    // current-cell branch rather than its flag-0x10 bilinear path.
    let position_raw = world_position_to_raw(position);
    let surface_raw = coarse_terrain_height_raw(terrain, position_raw[0], position_raw[2]);
    f32::from(surface_raw) / 256.0
}

fn displaced_water_surface(
    context: TerrainCollisionContext<'_>,
    position: [f32; 3],
    retail_tick: u32,
) -> Option<f32> {
    if !context.terrain.water_enabled() {
        return None;
    }
    let position_raw = world_position_to_raw(position);
    let terrain_y_raw =
        coarse_terrain_height_raw(context.terrain, position_raw[0], position_raw[2]);
    let sea_y_raw = context.terrain.sea_level_raw();
    if terrain_y_raw >= sea_y_raw {
        return None;
    }
    Some(
        f32::from(wave_surface_raw(
            position_raw[0],
            position_raw[2],
            retail_tick as i32,
            sea_y_raw,
            terrain_y_raw,
        )) / 256.0,
    )
}

fn classify_particle_water(
    terrain_context: Option<TerrainCollisionContext<'_>>,
    flat_sea_level: Option<f32>,
    position: [f32; 3],
    radius: f32,
    retail_tick: u32,
) -> u8 {
    let surface = if let Some(context) = terrain_context {
        displaced_water_surface(context, position, retail_tick)
    } else {
        flat_sea_level
    };
    let Some(surface_y) = surface else {
        return 2;
    };
    if position[1] + radius < surface_y {
        0
    } else if surface_y < position[1] - radius {
        2
    } else {
        1
    }
}

fn uses_mode_one_surface_callback(source_class: u8) -> bool {
    particle_descriptor(source_class).is_some_and(|descriptor| {
        descriptor.raw_byte(0x0e) == 1
            && uses_gravity_update_callback(source_class)
            && descriptor.raw_u32(0x18) == MODE_ONE_SURFACE_CALLBACK_VA
    })
}

fn terrain_contact_mode_for_particle(source_class: u8) -> Option<TerrainContactMode> {
    let descriptor = particle_descriptor(source_class)?;
    if descriptor.raw_byte(0x0e) != 3 {
        return None;
    }
    match descriptor.raw_u32(0x18) {
        ALIEN_HIVE_SURFACE_CALLBACK_VA => Some(TerrainContactMode::Infect),
        CLEANSING_LANDSCAPE_SURFACE_CALLBACK_VA => Some(TerrainContactMode::Cleanse),
        _ => None,
    }
}

fn uses_player_surface_probe_callback(source_class: u8) -> bool {
    particle_descriptor(source_class).is_some_and(|descriptor| {
        descriptor.raw_byte(0x0e) == 1
            && descriptor.raw_u32(0x18) == PLAYER_SURFACE_PROBE_CALLBACK_VA
    })
}

fn uses_downwash_debris_update_callback(source_class: u8) -> bool {
    particle_descriptor(source_class)
        .is_some_and(|descriptor| descriptor.raw_u32(0x14) == DOWNWASH_DEBRIS_UPDATE_CALLBACK_VA)
}

fn uses_ballistic_trail_update_callback(source_class: u8) -> bool {
    particle_descriptor(source_class)
        .is_some_and(|descriptor| descriptor.raw_u32(0x14) == BALLISTIC_TRAIL_UPDATE_CALLBACK_VA)
}

fn uses_ballistic_surface_callback(source_class: u8) -> bool {
    particle_descriptor(source_class).is_some_and(|descriptor| {
        descriptor.raw_byte(0x0e) == 3 && descriptor.raw_u32(0x18) == BALLISTIC_SURFACE_CALLBACK_VA
    })
}

fn uses_ballistic_sweep_callbacks(source_class: u8) -> bool {
    particle_descriptor(source_class).is_some_and(|descriptor| {
        descriptor.raw_byte(0x0e) == 3
            && descriptor.raw_u32(0x1c) == BALLISTIC_ENTITY_CALLBACK_VA
            && descriptor.raw_u32(0x20) == BALLISTIC_DAMAGE_PACKET_VA
            && descriptor.raw_u32(0x24) == BALLISTIC_STATIC_CALLBACK_VA
    })
}

/// `FUN_00442950` static-route family: classes 52/68/85 only, each with its
/// exact damage-packet word. Gating on the full (class, packet) pair keeps
/// class 87 (`0x004CC0F0` with `FUN_0043F590`) and dragon class 38
/// (`0x004CC048` with `FUN_0043F590`) in their own sweep policies.
pub fn particle_uses_static_route_entity_hit(source_class: u8) -> bool {
    particle_descriptor(source_class).is_some_and(|descriptor| {
        descriptor.raw_byte(0x0e) == 3
            && descriptor.raw_u32(0x1c) == STATIC_ROUTE_ENTITY_CALLBACK_VA
            && matches!(
                (source_class, descriptor.raw_u32(0x20)),
                (52, STATIC_ROUTE_CLASS52_DAMAGE_PACKET_VA)
                    | (68, STATIC_ROUTE_CLASS68_DAMAGE_PACKET_VA)
                    | (85, STATIC_ROUTE_CLASS85_DAMAGE_PACKET_VA)
            )
    })
}

/// `FUN_004425D0` attached-follow family: classes 83/84/86 only. Their null
/// entity-hit, null surface, and null cleanup slots keep them out of every
/// sweep; only this update owns them.
pub fn particle_uses_attached_follow_update(source_class: u8) -> bool {
    particle_descriptor(source_class).is_some_and(|descriptor| {
        matches!(source_class, 83 | 84 | 86)
            && descriptor.raw_u32(0x14) == ATTACHED_FOLLOW_UPDATE_CALLBACK_VA
            && descriptor.raw_u32(0x1c) == 0
    })
}

/// Owner state word selecting `FUN_004425D0`'s basis-rotated follow branch.
/// Without it the attached particle copies the owner center directly.
pub const ATTACHED_FOLLOW_ROTATED_STATE_BIT: u32 = 0x0200_0000;

/// `FUN_00442950` attached-class allocation request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AttachedStaticEmission {
    /// Hit-target handle stored at retail particle word `+0x14`.
    pub target_handle: u32,
    /// Impact endpoint in port world units.
    pub position_world: [f32; 3],
    /// Impact-minus-target raw words for `+0x0E/+0x10/+0x12`
    /// (capability-gated at birth).
    pub offset_raw: [i16; 3],
    /// Attached class from the 442950 source mapping (83/84/86).
    pub particle_class: u8,
}

/// Successful `FUN_00440A60` attached allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachedParticleBirth {
    pub slot: usize,
    pub particle_class: u8,
}

#[derive(Debug, Clone, Copy)]
pub enum ParticleAttachmentBasis {
    Native(crate::common_mover::type9_attitude::Type9BodyBasis),
    /// Explicit detached-fixture projection. Live hosts retain the Q31 words.
    Projected([[f32; 3]; 3]),
    Unresolved,
}

/// 3A580 handle lookup for 425D0/442950, independent of collision eligibility.
#[derive(Debug, Clone, Copy)]
pub struct ParticleAttachmentOwner {
    pub entity_id: u32,
    pub position_raw: [i16; 3],
    pub state_flags_at_0x08: Option<u32>,
    pub capability_flags_at_0x64: u32,
    pub basis: ParticleAttachmentBasis,
}

impl ParticleAttachmentOwner {
    pub fn from_entity(entity: &crate::entity::Entity) -> Self {
        Self {
            entity_id: entity.id,
            position_raw: entity.position_raw(),
            state_flags_at_0x08: match entity
                .collision
                .state_flags_at_0x08
                .masked(ATTACHED_FOLLOW_ROTATED_STATE_BIT)
            {
                crate::entity_collision_state::RetailRuntimeValue::Known(flags) => Some(flags),
                _ => None,
            },
            capability_flags_at_0x64: entity.capability_flags,
            basis: match entity.physical_body_basis_q31() {
                crate::entity_collision_state::RetailRuntimeValue::Known(basis) => {
                    ParticleAttachmentBasis::Native(basis)
                }
                _ => ParticleAttachmentBasis::Unresolved,
            },
        }
    }

    pub(super) fn from_collision_model(model: &EntityCollisionModel) -> Self {
        Self {
            entity_id: model.entity_id,
            position_raw: world_position_to_raw(model.center_world),
            state_flags_at_0x08: model.state_flags_at_0x08,
            capability_flags_at_0x64: model.capability_flags_at_0x64,
            basis: ParticleAttachmentBasis::Projected(model.orientation_world_from_model),
        }
    }
}

/// 442950's impact-minus-target words. Capability-1 targets use zero Y
/// and signed truncation toward zero of three-quarter X/Z differences.
pub(crate) fn attached_impact_offset_raw(
    endpoint_raw: [i16; 3],
    target_raw: [i16; 3],
    capability_flags: u32,
) -> [i16; 3] {
    let mut offset = std::array::from_fn(|axis| endpoint_raw[axis].wrapping_sub(target_raw[axis]));
    if capability_flags & 1 != 0 {
        offset[1] = 0;
        for axis in [0, 2] {
            offset[axis] = (i32::from(offset[axis]) * 3 / 4) as i16;
        }
    }
    offset
}

/// `FUN_004425D0` attached follow through the full owner-handle snapshot.
///
/// A missing owner sets particle byte `+0x1D` bit `0x80`; mode0 leaves the
/// record until the next visit's pre-update free. Unresolvable owner state keeps
/// the current position because the direct/rotated branch cannot be selected.
/// The complete callback owns B2, effects, damage admission and cascade around
/// this pose phase. This helper alone consumes no shared RNG.
pub(crate) fn apply_attached_owner_follow(
    particle: &mut WorldParticle,
    owner: crate::entity_collision_state::RetailRuntimeValue<Option<ParticleAttachmentOwner>>,
) {
    use crate::entity_collision_state::RetailRuntimeValue;
    if particle.attached_owner_handle.is_none() {
        return;
    }
    let owner = match owner {
        RetailRuntimeValue::Unresolved => return,
        RetailRuntimeValue::Known(None) => {
            particle.pending_destruction = true;
            return;
        }
        RetailRuntimeValue::Known(Some(owner)) => owner,
    };
    match owner.state_flags_at_0x08 {
        None => (),
        Some(flags) if flags & ATTACHED_FOLLOW_ROTATED_STATE_BIT == 0 => {
            particle.position = raw_position_to_world(owner.position_raw);
        }
        Some(_) => match owner.basis {
            ParticleAttachmentBasis::Native(basis) => {
                let columns = [basis.lateral, basis.up, basis.forward];
                let position_raw = std::array::from_fn(|axis| {
                    columns.iter().zip(particle.attached_offset_raw).fold(
                        owner.position_raw[axis],
                        |position, (column, offset)| {
                            position.wrapping_add(
                                ((i64::from(column[axis]) * i64::from(offset)) >> 31) as i16,
                            )
                        },
                    )
                });
                particle.position = raw_position_to_world(position_raw);
            }
            ParticleAttachmentBasis::Projected(matrix) => {
                for (axis, row) in matrix.iter().enumerate() {
                    particle.position[axis] = raw_position_to_world(owner.position_raw)[axis]
                        + row
                            .iter()
                            .zip(particle.attached_offset_raw)
                            .map(|(basis, offset)| basis * f32::from(offset) / 256.0)
                            .sum::<f32>();
                }
            }
            ParticleAttachmentBasis::Unresolved => {}
        },
    }
}

fn uses_class_87_sweep_callbacks(source_class: u8) -> bool {
    particle_descriptor(source_class).is_some_and(|descriptor| {
        source_class == FIRST_WORLD_SHOOTER_PARTICLE_CLASS
            && descriptor.raw_byte(0x0e) == 3
            && descriptor.raw_u32(0x1c) == CLASS_87_ENTITY_HIT_CALLBACK_VA
            && descriptor.raw_u32(0x20) == CLASS_87_DAMAGE_PACKET_VA
            && descriptor.raw_u32(0x24) == CLASS_87_STATIC_HIT_CALLBACK_VA
    })
}

fn uses_gravity_update_callback(source_class: u8) -> bool {
    particle_descriptor(source_class)
        .is_some_and(|descriptor| descriptor.raw_u32(0x14) == GRAVITY_UPDATE_CALLBACK_VA)
}

/// Run class 1's descriptor collision dispatch for one already-integrated
/// physical record. Entity and static-model sweeps short-circuit the later
/// water/heightfield stages, matching `FUN_00440120`'s collision-mode branch.
fn detect_primary_collision(
    physical_slot: usize,
    particle: &mut WorldParticle,
    primary: ParticleCollisionContext<'_>,
    terrain_context: Option<TerrainCollisionContext<'_>>,
    retail_tick: u32,
    terrain_type_mutations: &[ParticleTerrainMutation],
) -> PrimaryCollisionDispatch {
    let mut dispatch = PrimaryCollisionDispatch {
        keep_particle: true,
        entity_impact: None,
        impacts: Vec::new(),
        effects: Vec::new(),
    };
    let Some(owner_id) = particle.owner_id else {
        particle.step_start_position = particle.position;
        return dispatch;
    };

    let projectile_radius_raw = particle.collision_radius_raw;
    if let Some(target_entity_id) = retail_swept_model_hit(
        particle.step_start_position,
        particle.position,
        projectile_radius_raw,
        Some(owner_id),
        particle.age_ticks,
        primary.entities,
        primary.model_pool,
    ) {
        // FUN_0043F590 deliberately uses the integrated endpoint for both the
        // effect and damage descriptor, not the refined collision probe.
        let position_world = particle.position;
        let velocity_raw = particle
            .velocity
            .map(|component| world_velocity_component_to_raw(component) as i16);
        dispatch.entity_impact = Some(ParticleEntityImpact {
            source_particle_class: particle.source_class,
            impact_position_argument_va: retail_particle_impact_position_argument_va(physical_slot),
            target_entity_id,
            position_world,
            velocity_raw,
            damage: (!particle.suppresses_impact_damage).then_some(BallisticDamageRequest {
                packet: PRIMARY_PROJECTILE_DAMAGE_PACKET,
                source_entity_type_at_birth: particle.source_entity_type_at_birth,
                source_owner_id: particle.owner_id,
            }),
        });
        dispatch
            .effects
            .push(PrimaryImpactEffect::HitBurst(position_world));
        dispatch.keep_particle = false;
        return dispatch;
    }

    if let Some(context) = terrain_context {
        if let Some(terrain_objects) = context.terrain_objects {
            if let Some(static_hit) = retail_swept_static_tile_hit(
                particle.step_start_position,
                particle.position,
                projectile_radius_raw,
                context.terrain,
                terrain_objects,
                retail_tick,
                primary.model_pool,
                terrain_type_mutations,
            ) {
                // FUN_0043FF10 writes its refined probe back before dispatching
                // FUN_0043F800, unlike the endpoint-based entity path above.
                particle.position = static_hit.position_world;
                dispatch
                    .impacts
                    .push(PrimaryImpact::StaticTile(ParticleStaticImpact {
                        source_particle_class: particle.source_class,
                        source_owner_id: particle.owner_id,
                        position_world: static_hit.position_world,
                        cell: static_hit.cell,
                        attribute: static_hit.attribute,
                        terrain_type: static_hit.terrain_type,
                        model_id: static_hit.model_id,
                        kind_index: static_hit.kind_index,
                        current: None,
                        damage: (!particle.suppresses_impact_damage).then_some(
                            BallisticDamageRequest {
                                packet: PRIMARY_PROJECTILE_DAMAGE_PACKET,
                                source_entity_type_at_birth: particle.source_entity_type_at_birth,
                                source_owner_id: particle.owner_id,
                            },
                        ),
                    }));
                dispatch
                    .effects
                    .push(PrimaryImpactEffect::HitBurst(static_hit.position_world));
                dispatch.keep_particle = false;
                return dispatch;
            }
        }

        if particle.step_start_water_state == 2 && particle.water_state < 2 {
            if let Some(surface_y) =
                displaced_water_surface(context, particle.position, retail_tick)
            {
                let position_world = particle.position;
                let material_code = nearest_material_code(context.terrain, position_world);
                let response_selector =
                    context.water_response_selectors[usize::from(material_code)];
                dispatch.impacts.push(PrimaryImpact::WaterEntry {
                    projectile_owner_id: owner_id,
                    position_world,
                    surface_y,
                    material_code,
                    response_selector,
                });
                dispatch.keep_particle = apply_primary_surface_callback(
                    particle,
                    response_selector,
                    surface_y,
                    position_world,
                    [position_world[0], surface_y, position_world[2]],
                    &mut dispatch.effects,
                );
                if !dispatch.keep_particle {
                    return dispatch;
                }
            }
        }

        let radius_world = f32::from(projectile_radius_raw) / 256.0;
        let terrain_y = context
            .terrain
            .height_at(particle.position[0], particle.position[2]);
        if particle.position[1] - radius_world <= terrain_y {
            let position_world = particle.position;
            let material_code = nearest_material_code(context.terrain, position_world);
            let response_selector = context.ground_response_selectors[usize::from(material_code)];
            dispatch.impacts.push(PrimaryImpact::Terrain {
                projectile_owner_id: owner_id,
                position_world,
                surface_y: terrain_y,
                material_code,
                response_selector,
            });
            dispatch.keep_particle = apply_primary_surface_callback(
                particle,
                response_selector,
                terrain_y,
                position_world,
                [position_world[0], terrain_y, position_world[2]],
                &mut dispatch.effects,
            );
            if dispatch.keep_particle {
                particle.step_start_position = particle.position;
            }
            return dispatch;
        }
    }

    particle.step_start_position = particle.position;
    dispatch
}

/// `FUN_0043F6E0` always calls `FUN_00410EB0` after F610. A null `+0x20` skips
/// the wrapper instead of dereferencing retail's crash path.
fn fun_0043f6e0_damage_request(particle: &WorldParticle) -> Option<BallisticDamageRequest> {
    let packet_va = particle_descriptor(particle.source_class)?.raw_u32(0x20);
    let packet = match packet_va {
        0x004C_BFA0 => FUN_0043F6E0_DAMAGE_PACKET,
        _ => return None,
    };
    Some(BallisticDamageRequest {
        packet,
        source_entity_type_at_birth: particle.source_entity_type_at_birth,
        source_owner_id: particle.owner_id,
    })
}

/// F780/F7C0 skip their11250/11320 entry when `+0x1D` bit0 is set.
fn model_switch_particle_damage_request(
    particle: &WorldParticle,
) -> Option<BallisticDamageRequest> {
    if particle.suppresses_impact_damage {
        return None;
    }
    let packet_va = particle_descriptor(particle.source_class)?.raw_u32(0x20);
    let packet = match (
        particle_descriptor(particle.source_class)?.raw_u32(0x1c),
        packet_va,
    ) {
        (FUN_0043F780_HIT_VA, 0x004C_BFD0) => FUN_0043F780_DAMAGE_PACKET,
        (FUN_0043F7C0_HIT_VA, 0x004C_BFE8) => FUN_0043F7C0_DAMAGE_PACKET,
        _ => return None,
    };
    Some(BallisticDamageRequest {
        packet,
        source_entity_type_at_birth: particle.source_entity_type_at_birth,
        source_owner_id: particle.owner_id,
    })
}

fn detect_model_switch_particle_entity_hit(
    physical_slot: usize,
    particle: &WorldParticle,
    primary: ParticleCollisionContext<'_>,
) -> Option<ParticleEntityImpact> {
    let owner_id = particle.owner_id?;
    let target_entity_id = retail_swept_model_hit(
        particle.step_start_position,
        particle.position,
        particle.collision_radius_raw,
        Some(owner_id),
        particle.age_ticks,
        primary.entities,
        primary.model_pool,
    )?;
    let position_world = particle.position;
    let velocity_raw = particle
        .velocity
        .map(|component| world_velocity_component_to_raw(component) as i16);
    Some(ParticleEntityImpact {
        source_particle_class: particle.source_class,
        impact_position_argument_va: retail_particle_impact_position_argument_va(physical_slot),
        target_entity_id,
        position_world,
        velocity_raw,
        damage: model_switch_particle_damage_request(particle),
    })
}

fn detect_f6e0_entity_hit(
    physical_slot: usize,
    particle: &WorldParticle,
    primary: ParticleCollisionContext<'_>,
) -> Option<ParticleEntityImpact> {
    let owner_id = particle.owner_id?;
    let target_entity_id = retail_swept_model_hit(
        particle.step_start_position,
        particle.position,
        particle.collision_radius_raw,
        Some(owner_id),
        particle.age_ticks,
        primary.entities,
        primary.model_pool,
    )?;
    let position_world = particle.position;
    let velocity_raw = particle
        .velocity
        .map(|component| world_velocity_component_to_raw(component) as i16);
    Some(ParticleEntityImpact {
        source_particle_class: particle.source_class,
        impact_position_argument_va: retail_particle_impact_position_argument_va(physical_slot),
        target_entity_id,
        position_world,
        velocity_raw,
        damage: fun_0043f6e0_damage_request(particle),
    })
}

/// Class 87 shares the common mode-3 sweep but not the static callback policy
/// of the older ballistic family. Entity hits emit F3F610 and may deliver the
/// type-47 packet; static hits call the no-op F2E8E0 and only consume parent.
fn detect_class_87_sweep(
    physical_slot: usize,
    particle: &mut WorldParticle,
    collision: ParticleCollisionContext<'_>,
    terrain_context: Option<TerrainCollisionContext<'_>>,
    retail_tick: u32,
    terrain_type_mutations: &[ParticleTerrainMutation],
) -> Option<Class87SweepImpact> {
    debug_assert!(uses_class_87_sweep_callbacks(particle.source_class));
    let damage = (!particle.suppresses_impact_damage).then_some(BallisticDamageRequest {
        packet: TYPE_47_PROJECTILE_DAMAGE_PACKET,
        source_entity_type_at_birth: particle.source_entity_type_at_birth,
        source_owner_id: particle.owner_id,
    });

    if let Some(target_entity_id) = retail_swept_model_hit(
        particle.step_start_position,
        particle.position,
        particle.collision_radius_raw,
        particle.owner_id,
        particle.age_ticks,
        collision.entities,
        collision.model_pool,
    ) {
        return Some(Class87SweepImpact::Entity(ParticleEntityImpact {
            source_particle_class: particle.source_class,
            impact_position_argument_va: retail_particle_impact_position_argument_va(physical_slot),
            target_entity_id,
            position_world: particle.position,
            velocity_raw: particle
                .velocity
                .map(|component| world_velocity_component_to_raw(component) as i16),
            damage,
        }));
    }

    let context = terrain_context?;
    let terrain_objects = context.terrain_objects?;
    let hit = retail_swept_static_tile_hit(
        particle.step_start_position,
        particle.position,
        particle.collision_radius_raw,
        context.terrain,
        terrain_objects,
        retail_tick,
        collision.model_pool,
        terrain_type_mutations,
    )?;
    particle.position = hit.position_world;
    Some(Class87SweepImpact::StaticTile(Class87StaticTileImpact {
        position_world: hit.position_world,
        cell: hit.cell,
        attribute: hit.attribute,
        terrain_type: hit.terrain_type,
        model_id: hit.model_id,
        kind_index: hit.kind_index,
    }))
}

/// Dispatch the descriptor-selected surface callback shared by the entity and
/// static sweep tail in `FUN_00440120`.
///
/// Class 1 uses `FUN_0043DEB0`: selector 6 survives, every other selector
/// deletes, and all paths emit its recovered response effects/audio. Class 3
/// instead uses `FUN_0043E0D0`: selectors 6/7 spawn one `FUN_00441770` child
/// and delete; every other selector snaps to the surface, zeroes raw Y
/// velocity, divides raw X/Z velocity by four toward zero, and survives.
fn apply_primary_surface_callback(
    particle: &mut WorldParticle,
    response_selector: u8,
    surface_y: f32,
    sound_position: [f32; 3],
    effect_position: [f32; 3],
    effects: &mut Vec<PrimaryImpactEffect>,
) -> bool {
    match particle.source_class {
        PRIMARY_BULLET_PARTICLE_CLASS | RAPID_PRIMARY_PARTICLE_CLASS => {
            effects.push(PrimaryImpactEffect::Surface {
                sound_position,
                effect_position,
                response_selector,
            });
            response_selector == 6
        }
        UPGRADED_PRIMARY_PARTICLE_CLASS if matches!(response_selector, 6 | 7) => {
            effects.push(PrimaryImpactEffect::UpgradedSurfaceBurst(sound_position));
            false
        }
        UPGRADED_PRIMARY_PARTICLE_CLASS => {
            let x_raw = world_velocity_component_to_raw(particle.velocity[0]) / 4;
            let z_raw = world_velocity_component_to_raw(particle.velocity[2]) / 4;
            particle.velocity = raw_velocity_to_world([x_raw, 0, z_raw]);
            particle.position[1] = surface_y;
            true
        }
        _ => unreachable!("unsupported primary projectile reached surface callback"),
    }
}

fn primary_entity_impact_burst(position: [f32; 3], sea_level: Option<f32>) -> ParticleBurst {
    let above_water = sea_level.map_or(true, |water_y| position[1] > water_y);
    let (
        frame,
        source_class,
        lifetime_ticks,
        collision_radius_raw,
        animation_rate_raw,
        draw_scale_raw,
        size_jitter_divisor_raw,
        velocity,
    ) = if above_water {
        (
            first_particle_frame(PRIMARY_ABOVE_WATER_IMPACT_CLASS),
            PRIMARY_ABOVE_WATER_IMPACT_CLASS,
            PRIMARY_ABOVE_WATER_IMPACT_LIFETIME_TICKS,
            PRIMARY_ABOVE_WATER_IMPACT_RADIUS_RAW,
            PRIMARY_ABOVE_WATER_IMPACT_ANIMATION_RATE_RAW,
            PRIMARY_ABOVE_WATER_IMPACT_DRAW_SCALE_RAW,
            PRIMARY_ABOVE_WATER_IMPACT_SIZE_JITTER_DIVISOR_RAW,
            raw_velocity_to_world(PRIMARY_ABOVE_WATER_IMPACT_INITIAL_VELOCITY_RAW),
        )
    } else {
        (
            first_particle_frame(PRIMARY_UNDERWATER_IMPACT_CLASS),
            PRIMARY_UNDERWATER_IMPACT_CLASS,
            PRIMARY_UNDERWATER_IMPACT_LIFETIME_TICKS,
            PRIMARY_UNDERWATER_IMPACT_RADIUS_RAW,
            PRIMARY_UNDERWATER_IMPACT_ANIMATION_RATE_RAW,
            PRIMARY_UNDERWATER_IMPACT_DRAW_SCALE_RAW,
            PRIMARY_UNDERWATER_IMPACT_SIZE_JITTER_DIVISOR_RAW,
            raw_velocity_to_world(PRIMARY_UNDERWATER_IMPACT_INITIAL_VELOCITY_RAW),
        )
    };
    ParticleBurst {
        origin: position,
        particles: vec![ParticleSpawn {
            sprite_id: frame.sprite_id,
            source_class,
            offset: [0.0; 3],
            presentation_offset: [0.0; 3],
            velocity,
            owner_id: None,
            attached_owner_handle: None,
            attached_offset_raw: [0; 3],
            source_entity_type_at_birth: None,
            suppresses_impact_damage: false,
            lifetime_ticks,
            collision_radius_raw,
            frame_middle_raw: frame.middle_raw,
            frame_scale_raw: frame.scale_raw,
            animation_rate_raw,
            draw_scale_raw,
            size_jitter_divisor_raw,
        }],
    }
}

/// Packed `DAT_004CD4B8` dword `* 2` for X/Y, independent wrapping `Z * 2`.
fn fun_00441850_class_54_direction_velocity(direction: [i16; 3]) -> [i32; 3] {
    let packed = u32::from(direction[0] as u16) | (u32::from(direction[1] as u16) << 16);
    let doubled = packed.wrapping_mul(2);
    [
        i32::from(doubled as i16),
        i32::from((doubled >> 16) as i16),
        i32::from(direction[2].wrapping_mul(2)),
    ]
}

fn single_descriptor_particle_burst(
    position: [f32; 3],
    source_class: u8,
    mut raw_velocity: [i32; 3],
    owner_id: Option<u32>,
) -> ParticleBurst {
    let descriptor = particle_descriptor(source_class)
        .expect("static emitter class has a retail particle descriptor");
    let frame = first_particle_frame(source_class);
    // These helpers reproduce the arguments passed to `FUN_00440A60`, not a
    // post-allocation record. The allocator adds the class's signed +0x2A
    // bias to Y; this is zero for 20/21, +400 for 42, and +500 for 43.
    raw_velocity[1] += i32::from(descriptor.spawn_velocity_y_bias_raw());
    ParticleBurst {
        origin: position,
        particles: vec![ParticleSpawn {
            sprite_id: frame.sprite_id,
            source_class,
            offset: [0.0; 3],
            presentation_offset: [0.0; 3],
            velocity: raw_velocity_to_world(raw_velocity),
            owner_id,
            attached_owner_handle: None,
            attached_offset_raw: [0; 3],
            source_entity_type_at_birth: None,
            suppresses_impact_damage: false,
            lifetime_ticks: descriptor.lifetime_ticks(),
            collision_radius_raw: descriptor.collision_radius_raw(),
            frame_middle_raw: frame.middle_raw,
            frame_scale_raw: frame.scale_raw,
            animation_rate_raw: descriptor.animation_rate_raw(),
            draw_scale_raw: descriptor.draw_scale_raw(),
            size_jitter_divisor_raw: descriptor.size_jitter_divisor_raw(),
        }],
    }
}

fn invisible_descriptor_particle_spawn(
    source_class: u8,
    mut raw_velocity: [i32; 3],
    owner_id: Option<u32>,
) -> ParticleSpawn {
    let descriptor = particle_descriptor(source_class)
        .expect("invisible emitter class has a retail particle descriptor");
    debug_assert_eq!(descriptor.frame_list_va(), 0);
    debug_assert_eq!(descriptor.frame_count(), 0);
    raw_velocity[1] += i32::from(descriptor.spawn_velocity_y_bias_raw());
    ParticleSpawn {
        sprite_id: 0,
        source_class,
        offset: [0.0; 3],
        presentation_offset: [0.0; 3],
        velocity: raw_velocity_to_world(raw_velocity),
        owner_id,
        attached_owner_handle: None,
        attached_offset_raw: [0; 3],
        source_entity_type_at_birth: None,
        suppresses_impact_damage: false,
        lifetime_ticks: descriptor.lifetime_ticks(),
        collision_radius_raw: descriptor.collision_radius_raw(),
        frame_middle_raw: 0,
        frame_scale_raw: 0,
        animation_rate_raw: descriptor.animation_rate_raw(),
        draw_scale_raw: descriptor.draw_scale_raw(),
        size_jitter_divisor_raw: descriptor.size_jitter_divisor_raw(),
    }
}

fn surface_response_particle(source_class: u8, raw_velocity: [i32; 3]) -> ParticleSpawn {
    assert!(
        uses_mode_one_surface_callback(source_class)
            || matches!(
                source_class,
                WATER_DELETE_PARTICLE_CLASS | WATER_SPLASH_PARTICLE_CLASS
            ),
        "particle class {source_class} is not a retail surface-response class"
    );
    let descriptor = particle_descriptor(source_class)
        .expect("surface response class has a retail particle descriptor");
    let frame = first_particle_frame(source_class);
    ParticleSpawn {
        sprite_id: frame.sprite_id,
        source_class,
        offset: [0.0; 3],
        presentation_offset: [0.0; 3],
        velocity: raw_velocity_to_world(raw_velocity),
        owner_id: None,
        attached_owner_handle: None,
        attached_offset_raw: [0; 3],
        source_entity_type_at_birth: None,
        suppresses_impact_damage: false,
        lifetime_ticks: descriptor.lifetime_ticks(),
        collision_radius_raw: descriptor.collision_radius_raw(),
        frame_middle_raw: frame.middle_raw,
        frame_scale_raw: frame.scale_raw,
        animation_rate_raw: descriptor.animation_rate_raw(),
        draw_scale_raw: descriptor.draw_scale_raw(),
        size_jitter_divisor_raw: descriptor.size_jitter_divisor_raw(),
    }
}

fn default_surface_response_burst(
    position: [f32; 3],
    response_selector: u8,
    direction_cursor: &mut usize,
    attempt_count: usize,
) -> ParticleBurst {
    let source_class = SURFACE_RESPONSE_PARTICLE_CLASS_BY_SELECTOR
        .get(usize::from(response_selector))
        .copied()
        .filter(|source_class| *source_class != 0)
        .expect("ordinary surface selector has a retail particle class");
    directional_surface_response_burst(
        position,
        source_class,
        None,
        direction_cursor,
        attempt_count,
    )
}

fn directional_surface_response_burst(
    position: [f32; 3],
    source_class: u8,
    owner_id: Option<u32>,
    direction_cursor: &mut usize,
    attempt_count: usize,
) -> ParticleBurst {
    let mut particles = Vec::with_capacity(attempt_count);
    for _ in 0..attempt_count {
        // `FUN_00440DC0` increments DAT_004DE840 before each table lookup.
        *direction_cursor = (*direction_cursor + 1) % RETAIL_DIRECTION_TABLE_RAW.len();
        let direction = RETAIL_DIRECTION_TABLE_RAW[*direction_cursor];
        let raw_velocity = [
            i32::from(direction[0]) >> 1,
            (i32::from(direction[1]) >> 2).abs(),
            i32::from(direction[2]) >> 1,
        ];
        let mut particle = surface_response_particle(source_class, raw_velocity);
        particle.owner_id = owner_id;
        particles.push(particle);
    }
    ParticleBurst {
        origin: position,
        particles,
    }
}

fn special_surface_response_burst(position: [f32; 3], response_selector: u8) -> ParticleBurst {
    let source_class = match response_selector {
        6 => WATER_SPLASH_PARTICLE_CLASS,
        7 => WATER_DELETE_PARTICLE_CLASS,
        _ => unreachable!("special surface selector {response_selector}"),
    };
    ParticleBurst {
        origin: position,
        particles: vec![surface_response_particle(source_class, [0; 3])],
    }
}

/// Reconstruct a retail signed-8.8 position from the port's world-space float.
/// X/Z wrap naturally when the rounded fixed-point value is narrowed to i16.
pub(crate) fn world_position_to_raw(position: [f32; 3]) -> [i16; 3] {
    position.map(|component| ((component * 256.0).round() as i32) as i16)
}

/// Exact post-frame-selection radius policy from `FUN_0043D410`.
fn particle_terrain_light_radius_raw(
    middle_raw: i16,
    age: u8,
    particle_y_raw: i16,
    ground_raw: i16,
) -> Option<i32> {
    match middle_raw.cmp(&0) {
        std::cmp::Ordering::Equal => None,
        std::cmp::Ordering::Less => {
            let full_radius = -i32::from(middle_raw) * 0x100;
            if age < 0xc9 {
                Some(full_radius)
            } else {
                Some((0x100 - i32::from(age)) * full_radius / 0x38)
            }
        }
        std::cmp::Ordering::Greater => {
            let height_raw = (i32::from(particle_y_raw) - i32::from(ground_raw)).max(0x100);
            Some(i32::from(middle_raw) * 0x100 / height_raw)
        }
    }
}

fn raw_position_delta(a: [i16; 3], b: [i16; 3]) -> [i16; 3] {
    std::array::from_fn(|axis| a[axis].wrapping_sub(b[axis]))
}

fn raw_position_add(position: [i16; 3], offset: [i16; 3]) -> [i16; 3] {
    std::array::from_fn(|axis| position[axis].wrapping_add(offset[axis]))
}

fn raw_position_sub(position: [i16; 3], offset: [i16; 3]) -> [i16; 3] {
    std::array::from_fn(|axis| position[axis].wrapping_sub(offset[axis]))
}

fn raw_vector_half(vector: [i16; 3]) -> [i16; 3] {
    vector.map(|component| component / 2)
}

fn raw_vector_length(vector: [i16; 3]) -> u16 {
    let squared = vector
        .iter()
        .map(|&component| {
            let component = i64::from(component);
            component * component
        })
        .sum::<i64>();
    (squared as f64).sqrt().floor() as u16
}

fn raw_spheres_overlap(a: [i16; 3], b: [i16; 3], radius_raw: u32) -> bool {
    let delta = raw_position_delta(a, b);
    let squared = delta
        .iter()
        .map(|&component| {
            let component = i64::from(component);
            component * component
        })
        .sum::<i64>() as u64;
    squared <= u64::from(radius_raw) * u64::from(radius_raw)
}

fn raw_spheres_overlap_nonwrapping(a: [i16; 3], b: [i16; 3], radius_raw: u32) -> bool {
    let squared = (0..3)
        .map(|axis| {
            let component = i64::from(a[axis]) - i64::from(b[axis]);
            component * component
        })
        .sum::<i64>() as u64;
    squared <= u64::from(radius_raw) * u64::from(radius_raw)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StaticTileProbeHit {
    cell: [u8; 2],
    attribute: u8,
    terrain_type: u8,
    model_id: u16,
    kind_index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct StaticTileSweepHit {
    position_world: [f32; 3],
    cell: [u8; 2],
    attribute: u8,
    terrain_type: u8,
    model_id: u16,
    kind_index: u32,
}

/// Convert signed retail words back into the port's canonical world domain.
/// X/Z are toroidal unsigned words; Y remains a genuinely signed coordinate.
fn raw_position_to_world(position: [i16; 3]) -> [f32; 3] {
    [
        f32::from(position[0] as u16) / 256.0,
        f32::from(position[1]) / 256.0,
        f32::from(position[2] as u16) / 256.0,
    ]
}

/// `FUN_004276D0`'s phase-anchored tile lattice.
///
/// Offsets begin at exactly `-scan_radius`; they are not rounded to a cell
/// boundary. Consequently a primary scan radius of 44 visits one cell per
/// axis, while radius 200 visits offsets -200 and +56. X is the outer loop and
/// Z the inner loop, and candidate coordinates narrow back to wrapping words.
fn scan_static_tile_lattice<T>(
    probe_raw: [i16; 3],
    scan_radius_raw: u16,
    mut visit: impl FnMut(i16, i16) -> Option<T>,
) -> Option<T> {
    debug_assert!(scan_radius_raw <= i16::MAX as u16);
    let radius = i32::from(scan_radius_raw);
    let mut x_offset = -radius;
    while x_offset <= radius {
        let candidate_x = probe_raw[0].wrapping_add(x_offset as i16);
        let mut z_offset = -radius;
        while z_offset <= radius {
            let candidate_z = probe_raw[2].wrapping_add(z_offset as i16);
            if let Some(hit) = visit(candidate_x, candidate_z) {
                return Some(hit);
            }
            z_offset += 0x100;
        }
        x_offset += 0x100;
    }
    None
}

/// Collision center constructed by `FUN_00427410` for one static tile model.
fn static_tile_center_raw(terrain: &TerrainGrid, x: usize, z: usize) -> [i16; 3] {
    let next_x = (x + 1) & 0xff;
    let next_z = (z + 1) & 0xff;
    let height_sum_raw = [(x, z), (next_x, z), (x, next_z), (next_x, next_z)]
        .map(|(corner_x, corner_z)| {
            let height = terrain
                .cell(corner_x, corner_z)
                .expect("wrapped static-object terrain corner")
                .height as i8;
            i32::from(height) << 5
        })
        .into_iter()
        .sum::<i32>();

    [
        (((x as u16) << 8) | 0x7f) as i16,
        (height_sum_raw / 4) as i16,
        (((z as u16) << 8) | 0x7f) as i16,
    ]
}

fn static_collision_anim_vars(retail_tick: u32) -> AnimVars {
    let mut vars = AnimVars::default();
    // LAB_00427030 exposes only g_default_param. Rendering uses the same
    // masked 50-Hz clock on dynamic channel zero; every other channel is zero.
    vars.dynamic[0] = (retail_tick & 0xffff) as i32;
    vars
}

/// F920/F950 call27DE0/27E00 with probe+1C/+20 tile-center words.
///
/// Those words are `FUN_00427410`'s tile-center 8.8 pair (`cell<<8|0x7F`).
/// Matched C closes27DE0/27E00 as33720(x,z,1/0); the extra cdecl
/// `0x7D0` and `g_default_param` arguments are unused. Same-state writes stay
/// off the deferred overlay, matching the helper's change-only notify path.
fn queue_static_model_switch_infection(
    cell: [u8; 2],
    mode: TerrainContactMode,
    terrain: &TerrainGrid,
    terrain_type_mutations: &mut Vec<ParticleTerrainMutation>,
) {
    let terrain_type = terrain
        .cell(usize::from(cell[0]), usize::from(cell[1]))
        .map_or(0, |terrain_cell| terrain_cell.terrain_type);
    let infected = mode == TerrainContactMode::Infect;
    if (effective_terrain_type(terrain_type, cell, terrain_type_mutations)
        & INFECTION_TERRAIN_TYPE_BIT
        != 0)
        != infected
    {
        terrain_type_mutations.push(ParticleTerrainMutation::Infection { cell, infected });
    }
}

/// Resolve one terrain-type byte as it exists at the current physical-pool
/// position.
///
/// Retail's class-5/class-6 callbacks mutate Section 10 synchronously. The
/// port defers those writes until the immutable terrain borrow ends, so every
/// later same-pass terrain consumer must overlay the latest queued write.
/// Infection set/clear and immediate burn writes compose in order; all other
/// authored sibling bits remain those of the underlying cell.
fn effective_terrain_type(
    terrain_type: u8,
    cell: [u8; 2],
    terrain_type_mutations: &[ParticleTerrainMutation],
) -> u8 {
    terrain_type_mutations
        .iter()
        .filter(|write| write.cell() == cell)
        .fold(terrain_type, |current, write| write.apply(current))
}

/// Reacquire F800's live static target after the common F610 visual.
///
/// The immutable terrain borrow still contains the frame-start byte, while
/// earlier class-5/class-6 callbacks in this same physical traversal are held
/// in `terrain_type_mutations`. Overlaying them before the shared target
/// resolver preserves retail's synchronous Section-10 view without comparing
/// the result to the model that happened to produce the collision.
fn resolve_particle_static_damage_snapshot(
    context: Option<TerrainCollisionContext<'_>>,
    model_pool: Option<&dyn CollisionModelPool>,
    cell: [u8; 2],
    terrain_type_mutations: &[ParticleTerrainMutation],
) -> Option<CurrentStaticDamageSnapshot> {
    let context = context?;
    let terrain_objects = context.terrain_objects?;
    let model_pool = model_pool?;
    let mut terrain_cell = *context
        .terrain
        .cell(usize::from(cell[0]), usize::from(cell[1]))?;
    terrain_cell.terrain_type =
        effective_terrain_type(terrain_cell.terrain_type, cell, terrain_type_mutations);
    resolve_static_damage_snapshot_from_cell(cell, terrain_cell, terrain_objects, model_pool)
        .ok()
        .flatten()
}

fn exact_static_tile_probe<P: CollisionModelPool + ?Sized>(
    terrain: &TerrainGrid,
    terrain_objects: &TerrainObjectTable,
    model_pool: &P,
    probe_raw: [i16; 3],
    scan_radius_raw: u16,
    anim_vars: &AnimVars,
    terrain_type_mutations: &[ParticleTerrainMutation],
) -> Option<StaticTileProbeHit> {
    scan_static_tile_lattice(probe_raw, scan_radius_raw, |candidate_x, candidate_z| {
        let x = usize::from((candidate_x as u16) >> 8);
        let z = usize::from((candidate_z as u16) >> 8);
        let mut cell = *terrain.cell(x, z).expect("wrapped static-object cell");
        cell.terrain_type = effective_terrain_type(
            cell.terrain_type,
            [x as u8, z as u8],
            terrain_type_mutations,
        );
        if cell.attribute == 0 {
            return None;
        }
        let descriptor = terrain_objects.records.get(usize::from(cell.attribute))?;
        let model_id = descriptor.model_id_for(cell.terrain_type);
        let model = model_pool.collision_model(usize::from(model_id))?;
        if model.collision_radius_raw == 0 {
            return None;
        }

        let center_raw = static_tile_center_raw(terrain, x, z);
        if !raw_spheres_overlap(
            probe_raw,
            center_raw,
            u32::from(model.collision_radius_raw) + u32::from(scan_radius_raw),
        ) {
            return None;
        }

        let query_delta = raw_position_delta(probe_raw, center_raw).map(f64::from);
        match model.collide_sphere_raw(query_delta, scan_radius_raw, anim_vars, model_pool) {
            Ok(None) => None,
            // Missing interpreter coverage is not an authored miss. Keep
            // the already-successful retail broad sphere conservatively,
            // matching the active-entity collision path's explicit rule.
            Ok(Some(_)) | Err(_) => Some(StaticTileProbeHit {
                cell: [x as u8, z as u8],
                attribute: cell.attribute,
                terrain_type: cell.terrain_type,
                model_id,
                kind_index: descriptor.kind_index,
            }),
        }
    })
}

/// `FUN_0043FF10`'s unusual refinement control flow, separated from resource
/// lookup so each retail branch can be tested directly.
fn refine_static_tile_sweep<T: Copy>(
    endpoint_raw: [i16; 3],
    movement_raw: [i16; 3],
    segment_length_raw: i16,
    class_radius_raw: i16,
    mut scan: impl FnMut([i16; 3], u16) -> Option<T>,
) -> Option<([i16; 3], T)> {
    let mut half_movement = raw_vector_half(movement_raw);
    let mut probe = raw_position_sub(endpoint_raw, half_movement);
    let mut query_radius = (segment_length_raw / 2).max(class_radius_raw);
    let mut hit = scan(probe, query_radius.wrapping_add(4) as u16)?;

    while query_radius > class_radius_raw {
        query_radius = (query_radius / 2).max(class_radius_raw);
        half_movement = raw_vector_half(half_movement);
        probe = raw_position_sub(probe, half_movement);

        if scan(probe, query_radius.wrapping_add(4) as u16).is_none() {
            // Retail keeps the backward probe and clears its cancellation flag.
            continue;
        }

        // A backward hit advances by twice the new half-vector and requires a
        // second hit. A forward miss cancels the entire coarse result.
        probe = raw_position_add(raw_position_add(probe, half_movement), half_movement);
        let forward_hit = scan(probe, query_radius.wrapping_add(4) as u16)?;
        hit = forward_hit;
    }

    Some((probe, hit))
}

fn retail_swept_static_tile_hit<P: CollisionModelPool + ?Sized>(
    start_world: [f32; 3],
    end_world: [f32; 3],
    class_radius_raw: u16,
    terrain: &TerrainGrid,
    terrain_objects: &TerrainObjectTable,
    retail_tick: u32,
    model_pool: &P,
    terrain_type_mutations: &[ParticleTerrainMutation],
) -> Option<StaticTileSweepHit> {
    let start_raw = world_position_to_raw(start_world);
    let endpoint_raw = world_position_to_raw(end_world);
    let movement_raw = raw_position_delta(endpoint_raw, start_raw);
    let anim_vars = static_collision_anim_vars(retail_tick);
    let (probe_raw, hit) = refine_static_tile_sweep(
        endpoint_raw,
        movement_raw,
        raw_vector_length(movement_raw) as i16,
        class_radius_raw as i16,
        |probe, scan_radius| {
            exact_static_tile_probe(
                terrain,
                terrain_objects,
                model_pool,
                probe,
                scan_radius,
                &anim_vars,
                terrain_type_mutations,
            )
        },
    )?;

    Some(StaticTileSweepHit {
        position_world: raw_position_to_world(probe_raw),
        cell: hit.cell,
        attribute: hit.attribute,
        terrain_type: hit.terrain_type,
        model_id: hit.model_id,
        kind_index: hit.kind_index,
    })
}

/// `FUN_0043F980`'s swept active-model query in the retail signed-8.8 domain.
/// The initial broad candidate list remains in active-list order throughout
/// the earlier-half-first refinement, so a later probe can change the target
/// without turning this into a nearest-contact search.
fn retail_swept_model_hit<P: CollisionModelPool + ?Sized>(
    start_world: [f32; 3],
    end_world: [f32; 3],
    projectile_radius_raw: u16,
    owner_id: Option<u32>,
    age_ticks: f32,
    entities: &[EntityCollisionModel],
    model_pool: &P,
) -> Option<u32> {
    swept_model_hit_with_owner_gate(
        start_world,
        end_world,
        projectile_radius_raw,
        owner_id,
        age_ticks <= 50.0,
        entities,
        model_pool,
    )
}

fn swept_model_hit_with_owner_gate<P: CollisionModelPool + ?Sized>(
    start_world: [f32; 3],
    end_world: [f32; 3],
    projectile_radius_raw: u16,
    owner_id: Option<u32>,
    excludes_owner: bool,
    entities: &[EntityCollisionModel],
    model_pool: &P,
) -> Option<u32> {
    let start = world_position_to_raw(start_world);
    let end = world_position_to_raw(end_world);
    let movement = raw_position_delta(end, start);
    let mut half_movement = raw_vector_half(movement);
    let mut probe = raw_position_sub(end, half_movement);
    let mut query_radius = (raw_vector_length(movement) / 2).max(projectile_radius_raw);

    let candidates = entities
        .iter()
        .filter(|entity| {
            // The caller retains its own owner-grace policy: native particles
            // use50ticks; the local entity adapter uses strict750ms relation age.
            entity.radius_raw != 0
                && (Some(entity.entity_id) != owner_id || !excludes_owner)
                // F980's one-time candidate gather promotes the two signed
                // words before subtraction (no seam wrap). FCE0's later probe
                // checks narrow each subtraction back to i16; preserve that
                // odd asymmetry rather than normalizing both paths.
                && raw_spheres_overlap_nonwrapping(
                    probe,
                    world_position_to_raw(entity.center_world),
                    u32::from(query_radius) + u32::from(entity.radius_raw),
                )
        })
        .collect::<Vec<_>>();

    let mut target = exact_model_probe(&candidates, model_pool, probe, query_radius)?;

    while query_radius > projectile_radius_raw {
        let next_radius = (query_radius / 2).max(projectile_radius_raw);
        let next_half_movement = raw_vector_half(half_movement);
        let earlier = raw_position_sub(probe, next_half_movement);
        if let Some(earlier_target) =
            exact_model_probe(&candidates, model_pool, earlier, next_radius)
        {
            probe = earlier;
            target = earlier_target;
        } else {
            let later = raw_position_add(probe, next_half_movement);
            let Some(later_target) = exact_model_probe(&candidates, model_pool, later, next_radius)
            else {
                // FUN_0043F980 clears the coarse result when neither refined
                // half reproduces it; retaining the parent hit creates false
                // positives around gated/concave authored shapes.
                return None;
            };
            probe = later;
            target = later_target;
        }
        query_radius = next_radius;
        half_movement = next_half_movement;
    }

    Some(target)
}

fn exact_model_probe<P: CollisionModelPool + ?Sized>(
    candidates: &[&EntityCollisionModel],
    model_pool: &P,
    probe_raw: [i16; 3],
    query_radius_raw: u16,
) -> Option<u32> {
    for entity in candidates {
        let entity_raw = world_position_to_raw(entity.center_world);
        if !raw_spheres_overlap(
            probe_raw,
            entity_raw,
            u32::from(query_radius_raw) + u32::from(entity.radius_raw),
        ) {
            continue;
        }

        let Some(model) = model_pool.collision_model(entity.model_id) else {
            // Missing resources are not an authored miss. Preserve the old
            // conservative broad result until this model can be interpreted.
            return Some(entity.entity_id);
        };
        let world_delta = raw_position_delta(probe_raw, entity_raw).map(f64::from);
        let basis = entity
            .orientation_world_from_model
            .map(|row| row.map(f64::from));
        // Retail forward-transforms authored centers with the entity's Q31
        // basis. This matters for opcode 0x8F: inverse-transforming the query
        // would turn its query-frame AABB into an oriented box.
        match model.collide_sphere_raw_oriented(
            world_delta,
            query_radius_raw,
            basis,
            &entity.anim_vars,
            model_pool,
        ) {
            Ok(Some(_)) => return Some(entity.entity_id),
            Ok(None) => {}
            // Known but not-yet-ported later-level opcodes must remain a
            // deliberate conservative fallback, never masquerade as a miss.
            Err(_) => return Some(entity.entity_id),
        }
    }
    None
}

fn ballistic_trail_spawn(source_class: u8) -> ParticleSpawn {
    debug_assert!(matches!(
        source_class,
        METEOR_TRAIL_CLASS | BALLISTIC_UNDERWATER_TRAIL_CLASS
    ));
    let descriptor = particle_descriptor(source_class).expect("ECD0 trail descriptor");
    let frame = first_particle_frame(source_class);
    ParticleSpawn {
        sprite_id: frame.sprite_id,
        source_class,
        offset: [0.0; 3],
        presentation_offset: [0.0; 3],
        velocity: raw_velocity_to_world([0, i32::from(descriptor.spawn_velocity_y_bias_raw()), 0]),
        owner_id: None,
        attached_owner_handle: None,
        attached_offset_raw: [0; 3],
        source_entity_type_at_birth: None,
        suppresses_impact_damage: false,
        lifetime_ticks: descriptor.lifetime_ticks(),
        collision_radius_raw: descriptor.collision_radius_raw(),
        frame_middle_raw: frame.middle_raw,
        frame_scale_raw: frame.scale_raw,
        animation_rate_raw: descriptor.animation_rate_raw(),
        draw_scale_raw: descriptor.draw_scale_raw(),
        size_jitter_divisor_raw: descriptor.size_jitter_divisor_raw(),
    }
}

#[cfg(test)]
fn meteor_trail_particle(position: [f32; 3]) -> WorldParticle {
    world_particle(position, ballistic_trail_spawn(METEOR_TRAIL_CLASS))
}

/// Tracks timeline event ids which have already fired.
///
/// A forward `[previous, current]` interval is accepted.  Including the lower
/// boundary lets a timeline fire an event authored at time zero; the id latch
/// prevents a shared frame boundary from firing twice.
#[derive(Debug, Default)]
pub struct OneShotTimeline {
    fired: BTreeSet<u64>,
}

impl OneShotTimeline {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn crossed_once(
        &mut self,
        event_id: u64,
        previous_secs: f32,
        current_secs: f32,
        event_secs: f32,
    ) -> bool {
        if self.fired.contains(&event_id)
            || !previous_secs.is_finite()
            || !current_secs.is_finite()
            || !event_secs.is_finite()
            || current_secs <= previous_secs
            || event_secs < previous_secs
            || event_secs > current_secs
        {
            return false;
        }
        self.fired.insert(event_id);
        true
    }

    pub fn has_fired(&self, event_id: u64) -> bool {
        self.fired.contains(&event_id)
    }

    pub fn reset(&mut self) {
        self.fired.clear();
    }
}

#[cfg(test)]
mod tests {
    mod antidote;
    mod attached_follow;
    mod combat_sweep;
    mod live_host;
    mod rapid_primary;
    mod static_burn;
    mod surface_transition;
    mod virus_projectile;

    use super::*;
    use crate::defecate_virus::DEFECATE_VIRUS_PARTICLE_CLASS;
    use crate::level::LevelState;
    use crate::primary_weapon::{
        PrimaryAuthoredGunMounts, PrimaryFireGeometry, PrimaryGunMounts, PrimaryLaunchBasis,
        PrimaryShotBudget, PrimaryTriggerInput, PrimaryWeapon, PRIMARY_PROJECTILE_SPEC,
        PRIMARY_SOUND_ID, UPGRADED_PRIMARY_PROFILE, UPGRADED_PRIMARY_PROJECTILE_SPEC,
        UPGRADED_PRIMARY_SOUND_ID,
    };
    use crate::static_damage::{StaticDamageOutcome, StaticDamageScheduler, KIND_9_STATIC_OBJECT};
    use std::time::Duration;
    use v2k_formats::anim_frames::{ModelSlotPattern, TerrainObjectDescriptor, TerrainObjectTable};
    use v2k_formats::levels::LevelDescriptor;
    use v2k_formats::models::{Billboard, ModelEntry, ModelInstance};
    use v2k_formats::terrain::{wave_surface_y, TerrainCell, TerrainGrid, GRID_SIZE};

    fn expect_static_event(event: ParticleTerrainEvent) -> ParticleStaticImpact {
        let ParticleTerrainEvent::StaticImpact(impact) = event else {
            panic!("unexpected ground program: {event:?}");
        };
        impact
    }

    fn full_rate_world_fx() -> WorldFx {
        let mut fx = WorldFx::new();
        for _ in 0..PARTICLE_FRAME_SAMPLE_COUNT {
            fx.advance_frame_pacing(20_000);
        }
        assert_eq!(
            fx.frame_pacing.count_scale_q16,
            PARTICLE_FULL_RATE_SCALE_Q16
        );
        fx
    }

    fn collision_level_descriptor(selectors: [u8; 8]) -> LevelDescriptor {
        let mut raw_header = [0; 0xd0];
        for (index, selector) in selectors.into_iter().enumerate() {
            raw_header[0xa8 + index * 4..0xac + index * 4]
                .copy_from_slice(&u32::from(selector).to_le_bytes());
        }
        LevelDescriptor {
            raw_header,
            name: "collision-context-test".into(),
            world_style: 0,
            terrain_sprite_base: 0,
            sky_color_index: 0,
            sky_model: 0,
            main_base_abort_sky_color_index: 0,
            main_base_abort_sky_model: 0,
            terrain_draw_depth: 0,
            sub_count: 0,
            campaign_record_count: 0,
            entities: Vec::new(),
            campaign_records: Vec::new(),
        }
    }

    fn collision_level_state(
        source_path: &str,
        terrain: Option<TerrainGrid>,
        descriptor: Option<LevelDescriptor>,
        terrain_objects: Option<TerrainObjectTable>,
    ) -> LevelState {
        LevelState {
            source_path: source_path.into(),
            system_level: None,
            fixup_data: None,
            fixup_code: None,
            strings: Vec::new(),
            sprites: None,
            params: None,
            display_modes: None,
            fog_gradient: None,
            color_palettes: None,
            models: None,
            anim_frames: terrain_objects,
            terrain,
            anim_sound: None,
            collision: None,
            level: descriptor,
            linkage: None,
        }
    }

    #[test]
    fn terrain_collision_context_keeps_level_selectors_and_active_section_nine() {
        let preload = collision_level_state(
            "preload.ovl",
            Some(flat_terrain(1, 0x01)),
            Some(collision_level_descriptor([1, 2, 3, 4, 5, 6, 7, 0])),
            Some(terrain_object_table(1, [1, 1, 1, 1], 11)),
        );
        let auxiliary = collision_level_state(
            "active-world-style.ovl",
            Some(flat_terrain(2, 0x02)),
            Some(collision_level_descriptor([4, 3, 2, 1, 0, 7, 6, 5])),
            Some(terrain_object_table(1, [2, 2, 2, 2], 22)),
        );
        let level = collision_level_state(
            "level.ovl",
            Some(flat_terrain(3, 0x03)),
            Some(collision_level_descriptor([7, 6, 5, 4, 3, 2, 1, 0])),
            None,
        );
        let mut cache = ResourceCache::new(vec![preload]);
        cache.add_auxiliary(auxiliary);
        cache.load_level(level);

        let context = TerrainCollisionContext::from_current_level_cache(&cache).unwrap();
        assert_eq!(context.terrain.cells[0].height, 3);
        assert_eq!(context.ground_response_selectors, [7, 6, 5, 4, 3, 2, 1, 0]);
        assert_eq!(context.water_response_selectors, [7; 8]);
        assert_eq!(
            context.terrain_objects.unwrap().records[1].kind_index,
            22,
            "Section 9 follows the selected active world-style layer"
        );
    }

    #[test]
    fn terrain_collision_context_rejects_missing_level_owned_sections() {
        let fallback = collision_level_state(
            "fallback.ovl",
            Some(flat_terrain(1, 0x01)),
            Some(collision_level_descriptor([7; 8])),
            Some(terrain_object_table(1, [1, 1, 1, 1], 11)),
        );
        let mut cache = ResourceCache::new(vec![fallback]);

        cache.load_level(collision_level_state(
            "level-without-terrain.ovl",
            None,
            Some(collision_level_descriptor([6; 8])),
            None,
        ));
        assert!(TerrainCollisionContext::from_current_level_cache(&cache).is_none());

        cache.load_level(collision_level_state(
            "level-without-descriptor.ovl",
            Some(flat_terrain(3, 0x03)),
            None,
            None,
        ));
        assert!(
            TerrainCollisionContext::from_current_level_cache(&cache).is_none(),
            "Section-13 selectors must not fall back independently of level terrain"
        );
    }

    fn primary_hit_capability_emission(
        position_raw: [i16; 3],
        target_handle: u32,
        owner_sign: u32,
    ) -> PrimaryHitCapabilityEmission {
        PrimaryHitCapabilityEmission {
            target_handle,
            target_allocation_identity: 0x5000,
            selected_model_slot: 1,
            selected_global_model_id: 256,
            position_raw,
            particle_class: PRIMARY_HIT_CAPABILITY_PARTICLE_CLASS,
            particle_scale_raw: PRIMARY_HIT_CAPABILITY_PARTICLE_SCALE_RAW,
            owner_sign,
        }
    }

    #[test]
    fn particle_frame_pacing_matches_retail_warmup_thresholds_and_stall_reset() {
        let mut pacing = ParticleFramePacing::default();
        assert_eq!(
            pacing.samples_us,
            [PARTICLE_FRAME_SAMPLE_MAX_US; PARTICLE_FRAME_SAMPLE_COUNT]
        );
        assert_eq!(
            pacing.rolling_sum_us,
            PARTICLE_FRAME_SAMPLE_MAX_US * PARTICLE_FRAME_SAMPLE_COUNT as u32
        );
        assert_eq!(pacing.count_scale_q16, 0);

        for _ in 0..PARTICLE_FRAME_SAMPLE_COUNT {
            pacing.advance(PARTICLE_FULL_RATE_AVERAGE_US);
        }
        assert_eq!(pacing.count_scale_q16, PARTICLE_FULL_RATE_SCALE_Q16);

        for _ in 0..PARTICLE_FRAME_SAMPLE_COUNT {
            pacing.advance(PARTICLE_FULL_RATE_AVERAGE_US + 1);
        }
        assert_eq!(pacing.count_scale_q16, PARTICLE_FULL_RATE_SCALE_Q16 - 1);

        let mut boundary = ParticleFramePacing::default();
        for _ in 0..PARTICLE_FRAME_SAMPLE_COUNT {
            boundary.advance(20_000);
        }
        boundary.advance(PARTICLE_FRAME_SAMPLE_MAX_US);
        assert_eq!(
            boundary.count_scale_q16, PARTICLE_FULL_RATE_SCALE_Q16,
            "exactly 125 ms replaces one ring entry and does not hard-reset"
        );
        boundary.advance(PARTICLE_FRAME_SAMPLE_MAX_US + 1);
        assert_eq!(
            boundary.samples_us,
            [PARTICLE_FRAME_SAMPLE_MAX_US; PARTICLE_FRAME_SAMPLE_COUNT]
        );
        assert_eq!(
            boundary.rolling_sum_us,
            PARTICLE_FRAME_SAMPLE_MAX_US * PARTICLE_FRAME_SAMPLE_COUNT as u32
        );
        assert_eq!(boundary.count_scale_q16, 0);

        boundary.advance(20_000);
        assert_eq!(boundary.rolling_sum_us, 895_000);
        assert_eq!(boundary.count_scale_q16, 13_125);
    }

    #[test]
    fn particle_frame_pacing_preserves_distinct_min_one_and_direct_counts() {
        let mut pacing = ParticleFramePacing::default();
        let cases = [
            (0x1_0000, 10, 4, 8),
            (0xffff, 9, 3, 7),
            (0x2000, 1, 1, 1),
            (0x1fff, 1, 1, 0),
            (0, 1, 1, 0),
        ];
        for (scale, expected_ten, expected_four, expected_direct) in cases {
            pacing.count_scale_q16 = scale;
            assert_eq!(pacing.scaled_count_min_one(10), expected_ten);
            assert_eq!(pacing.scaled_count_min_one(4), expected_four);
            assert_eq!(pacing.shifted_count(13), expected_direct);
        }

        pacing.count_scale_q16 = PARTICLE_FULL_RATE_SCALE_Q16;
        assert_eq!(pacing.scaled_count_min_one(1000 >> 7), 7);
        assert_eq!(pacing.scaled_count_min_one(-1000 >> 7), -8);
        pacing.count_scale_q16 = 0xffff;
        assert_eq!(pacing.scaled_count_min_one(1000 >> 7), 6);
        assert_eq!(pacing.scaled_count_min_one(-1000 >> 7), -8);
        pacing.count_scale_q16 = 0;
        assert_eq!(pacing.scaled_count_min_one(1000 >> 7), 1);
        assert_eq!(
            pacing.scaled_count_min_one(-1000 >> 7),
            1,
            "retail promotes an exact zero product before inspecting its sign"
        );
    }

    #[test]
    fn clearing_level_effects_preserves_process_global_particle_pacing() {
        let mut fx = WorldFx::new();
        for _ in 0..PARTICLE_FRAME_SAMPLE_COUNT {
            fx.advance_frame_pacing(60_000);
        }
        let samples = fx.frame_pacing.samples_us;
        let rolling_sum = fx.frame_pacing.rolling_sum_us;
        let frame_counter = fx.frame_pacing.frame_counter;
        let scale = fx.frame_pacing.count_scale_q16;

        fx.clear();

        assert_eq!(fx.frame_pacing.samples_us, samples);
        assert_eq!(fx.frame_pacing.rolling_sum_us, rolling_sum);
        assert_eq!(fx.frame_pacing.frame_counter, frame_counter);
        assert_eq!(fx.frame_pacing.count_scale_q16, scale);
    }

    #[test]
    fn cold_particle_pacing_scales_only_adaptive_children_and_keeps_minimum_tails() {
        let mut common = WorldFx::new();
        common.emit_common_explosion_bundle_raw([0; 3], 0x100);
        let common_particles = common.test_particles_in_virgin_birth_order();
        assert_eq!(
            common_particles
                .iter()
                .filter(|particle| particle.source_class == COMMON_EXPLOSION_SCATTER_CLASS)
                .count(),
            1
        );
        assert_eq!(
            common_particles.len(),
            1 + COMMON_EXPLOSION_DEBRIS_COUNT,
            "fixed debris remains independent of the adaptive scatter count"
        );
        assert_eq!(common.direction_cursor, 1);
        assert_eq!(common.take_positional_sounds().len(), 1);

        let mut meteor = WorldFx::new();
        meteor.queue_meteor_impact([0.0, 1.0, 0.0], 0, None);
        meteor.process_pending();
        let meteor_particles = meteor.test_particles_in_virgin_birth_order();
        assert_eq!(meteor_particles.len(), 2);
        assert_eq!(meteor_particles[0].source_class, METEOR_SCATTER_CLASS);
        assert_eq!(meteor_particles[1].source_class, METEOR_SURFACE_CLASS);
        assert_eq!(meteor.take_positional_sounds().len(), 1);

        let mut entry = WorldFx::new();
        entry.emit_whole_body_water_entry_burst_raw([0; 3], 0, 6, 46);
        assert_eq!(entry.particle_count(), 1);
        assert_eq!(entry.direction_cursor, 1);

        let terrain = flat_terrain(0, 0);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [0; 8],
            water_response_selectors: [0; 8],
        };
        let parent = world_particle(
            [0.0; 3],
            invisible_descriptor_particle_spawn(
                PLAYER_SURFACE_PROBE_PARTICLE_CLASS,
                [0; 3],
                Some(46),
            ),
        );
        let mut direct = WorldFx::new();
        direct.emit_player_surface_probe_response(
            &parent,
            0,
            0,
            context,
            ParticleBirthContext {
                environment: ParticleEnvironment::Terrain(context),
                retail_tick: 0,
            },
        );
        assert_eq!(direct.particle_count(), 0);
        assert_eq!(direct.rng_state, 0);
    }

    #[test]
    fn degraded_particle_pacing_reaches_each_recovered_runtime_count_family() {
        let mut common = WorldFx::new();
        common.frame_pacing.count_scale_q16 = 0xffff;
        common.emit_common_explosion_bundle_raw([0; 3], 0x100);
        assert_eq!(
            common
                .test_particles_in_virgin_birth_order()
                .iter()
                .filter(|particle| particle.source_class == COMMON_EXPLOSION_SCATTER_CLASS)
                .count(),
            9
        );

        let mut wreck = WorldFx::new();
        wreck.frame_pacing.count_scale_q16 = 0xffff;
        wreck.emit_player_wreck_burst_raw([0; 3], 0x100, None, 46);
        let wreck_particles = wreck.test_particles_in_virgin_birth_order();
        assert_eq!(
            wreck_particles
                .iter()
                .filter(|particle| particle.source_class == METEOR_SCATTER_CLASS)
                .count(),
            9
        );
        assert_eq!(
            wreck_particles
                .iter()
                .filter(|particle| particle.source_class == METEOR_SURFACE_CLASS)
                .count(),
            1,
            "the independent surface tail is not load-shed"
        );
        assert_eq!(wreck.take_positional_sounds().len(), 2);

        let mut entry = WorldFx::new();
        entry.frame_pacing.count_scale_q16 = 0xffff;
        entry.emit_whole_body_water_entry_burst_raw([0; 3], 0, 6, 46);
        assert_eq!(entry.particle_count(), 3);
        assert_eq!(entry.direction_cursor, 3);

        let terrain = flat_terrain(0, 0);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [0; 8],
            water_response_selectors: [0; 8],
        };
        let mut primary = WorldFx::new();
        primary.frame_pacing.count_scale_q16 = 0xffff;
        primary.materialize_primary_impact_effect(
            PrimaryImpactEffect::Surface {
                sound_position: [0.0; 3],
                effect_position: [0.0; 3],
                response_selector: 0,
            },
            false,
            ParticleBirthContext {
                environment: ParticleEnvironment::Terrain(context),
                retail_tick: 0,
            },
        );
        assert_eq!(primary.particle_count(), 3);
        assert_eq!(primary.direction_cursor, 3);
        assert_eq!(primary.take_positional_sounds().len(), 1);

        let parent = world_particle(
            [0.0; 3],
            invisible_descriptor_particle_spawn(
                PLAYER_SURFACE_PROBE_PARTICLE_CLASS,
                [0; 3],
                Some(46),
            ),
        );
        let mut direct = WorldFx::new();
        direct.frame_pacing.count_scale_q16 = 0xffff;
        direct.emit_player_surface_probe_response(
            &parent,
            0,
            0,
            context,
            ParticleBirthContext {
                environment: ParticleEnvironment::Terrain(context),
                retail_tick: 0,
            },
        );
        assert_eq!(direct.particle_count(), 7);
        assert_eq!(direct.rng_state, {
            let mut state = 0;
            for _ in 0..7 {
                retail_random_u16(&mut state);
            }
            state
        });

        let mut submerged = WorldFx::new();
        submerged.frame_pacing.count_scale_q16 = 0xffff;
        submerged.emit_player_surface_effect_raw(player_surface_emission([0; 3], [0; 3], 1000), 0);
        assert_eq!(submerged.particle_count(), 6);
    }

    #[test]
    fn ballistic_pacing_keeps_minimum_ordinary_burst_but_permits_zero_wet_spray() {
        let terrain = flat_terrain(0, 0);
        let ordinary_context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [7; 8],
            water_response_selectors: [7; 8],
        };
        for (scale, expected_children) in [(0, 1), (0xffff, 3)] {
            let mut fx = WorldFx::new();
            fx.frame_pacing.count_scale_q16 = scale;
            fx.particles.test_allocate(descriptor_test_particle(
                METEOR_SCATTER_CLASS,
                [4.75, 1.0, 5.75],
                [-7, -5, 7],
                Some(66),
            ));
            let slot = fx
                .particles
                .slots
                .iter()
                .position(|particle| {
                    particle.is_some_and(|particle| particle.source_class == METEOR_SCATTER_CLASS)
                })
                .unwrap();
            let mut parent = fx.particles.slots[slot].unwrap();

            assert!(fx.apply_ballistic_surface_response(
                slot,
                &mut parent,
                7,
                0.0,
                ordinary_context,
                ParticleBirthContext {
                    environment: ParticleEnvironment::Terrain(ordinary_context),
                    retail_tick: 0,
                },
            ));
            assert_eq!(
                fx.particles
                    .slots
                    .iter()
                    .flatten()
                    .filter(|particle| particle.source_class == WATER_DELETE_PARTICLE_CLASS)
                    .count(),
                expected_children
            );
            assert_eq!(fx.direction_cursor, expected_children);
            assert_eq!(
                parent.velocity.map(world_velocity_component_to_raw),
                [-7, 2, 7],
                "load shedding changes only the child count"
            );
        }

        let mut wet_terrain = flat_terrain(-128, 6);
        wet_terrain.header[0] = 0;
        let wet_context = TerrainCollisionContext {
            terrain: &wet_terrain,
            terrain_objects: None,
            ground_response_selectors: [6; 8],
            water_response_selectors: [6; 8],
        };
        let mut wet = WorldFx::new();
        wet.particles.test_allocate(descriptor_test_particle(
            COMMON_EXPLOSION_SCATTER_CLASS,
            [4.0, 0.0, 5.0],
            [-7, -5, 7],
            Some(66),
        ));
        let wet_slot = wet
            .particles
            .slots
            .iter()
            .position(|particle| {
                particle
                    .is_some_and(|particle| particle.source_class == COMMON_EXPLOSION_SCATTER_CLASS)
            })
            .unwrap();
        let mut wet_parent = wet.particles.slots[wet_slot].unwrap();

        assert!(wet.apply_ballistic_surface_response(
            wet_slot,
            &mut wet_parent,
            6,
            99.0,
            wet_context,
            ParticleBirthContext {
                environment: ParticleEnvironment::Terrain(wet_context),
                retail_tick: 0,
            },
        ));
        assert_eq!(
            wet.particles
                .slots
                .iter()
                .flatten()
                .filter(|particle| {
                    particle.source_class == PLAYER_WATER_DOWNWASH_PARTICLE_CLASS
                })
                .count(),
            0
        );
        assert_eq!(wet.rng_state, 0);
        assert_eq!(
            wet_parent.velocity.map(world_velocity_component_to_raw),
            [-1, -2, 1],
            "a zero spray count must not suppress parent damping"
        );
    }

    #[test]
    fn class_87_request_and_in_place_surface_conversion_preserve_provenance() {
        let mut fx = WorldFx::new();
        let preexisting_replacement = fx
            .particles
            .allocate(descriptor_test_particle(88, [1.0, 1.0, 1.0], [0; 3], None))
            .unwrap();
        let request = DescriptorParticleRequest {
            source_class: 87,
            position_raw: [0x1200, 0x0340, -0x2100],
            velocity_raw: [300, -40, 900],
            owner: Some(ParticleOwnerAtBirth {
                entity_id: 0x1234,
                entity_type: 47,
            }),
            suppresses_impact_damage: false,
        };
        let slot = fx
            .materialize_descriptor_particle_request(request, ParticleEnvironment::Dry, 17)
            .expect("class-87 request should fit an empty retail pool");
        let mut particle = fx.particles.slots[slot].unwrap();
        assert_eq!(particle.source_class, FIRST_WORLD_SHOOTER_PARTICLE_CLASS);
        assert_eq!(
            particle.position,
            raw_position_to_world(request.position_raw)
        );
        assert_eq!(
            particle.velocity.map(world_velocity_component_to_raw),
            request.velocity_raw.map(i32::from)
        );

        assert!(fx.apply_projectile_surface_response(slot, &mut particle, 0, 9.5));
        let converted = fx.particles.slots[slot].unwrap();
        assert_eq!(converted.source_class, 88);
        assert_eq!(converted.position[1], 9.5);
        assert_eq!(converted.velocity, [0.0; 3]);
        assert_eq!(converted.age_ticks, 0.0);
        assert_eq!(converted.owner_id, Some(0x1234));
        assert_eq!(converted.source_entity_type_at_birth, Some(47));
        assert!(!converted.suppresses_impact_damage);
        let replacement_priority = particle_descriptor(88).unwrap().priority_raw() as usize;
        assert_eq!(
            usize::from(fx.particles.links[slot].list),
            replacement_priority
        );
        assert_ne!(preexisting_replacement, slot);
        assert_eq!(fx.particles.links[preexisting_replacement].next, Some(slot));
        assert_eq!(
            fx.particles.lists[replacement_priority].tail,
            Some(slot),
            "F42420 replacement must append at the intrusive-list tail"
        );
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn class_38_request_keeps_class_above_water_and_substitutes_underwater() {
        let request = Class38ParticleRequest {
            position_raw: [0x1200, 0x0340, -0x2100],
            velocity_raw: [300, -40, 900],
            owner_entity_id: 0x13,
            owner_entity_type: 13,
            suppresses_impact_damage: false,
        };
        let mut dry = WorldFx::new();
        let dry_birth = dry
            .materialize_class_38_request(request, ParticleEnvironment::Dry, 0)
            .expect("class-38 dry birth");
        assert_eq!(dry_birth.particle_class, TYPE13_PROJECTILE_PARTICLE_CLASS);
        assert_eq!(
            dry.particles.slots[dry_birth.slot].unwrap().source_class,
            TYPE13_PROJECTILE_PARTICLE_CLASS
        );

        let mut wet = WorldFx::new();
        let wet_birth = wet
            .materialize_class_38_request(
                request,
                ParticleEnvironment::FlatWater { sea_level: 8.0 },
                0,
            )
            .expect("class-38 underwater birth");
        assert_eq!(
            wet_birth.particle_class,
            TYPE13_PROJECTILE_UNDERWATER_PARTICLE_CLASS
        );
        let wet_particle = wet.particles.slots[wet_birth.slot].unwrap();
        assert_eq!(
            wet_particle.source_class,
            TYPE13_PROJECTILE_UNDERWATER_PARTICLE_CLASS
        );
        // 410B0's -100 input displacement and 40A60's class46 +100 bias cancel.
        assert_eq!(
            wet_particle.position,
            raw_position_to_world(request.position_raw)
        );
    }

    #[test]
    fn actor_surface_bubble_adapter_adds_class42_bias_once_and_keeps_provenance() {
        let mut fx = WorldFx::new();
        let request = ActorSurfaceBubbleRequest {
            position_raw: [0x1200, -10_000, -0x2100],
            velocity_argument_raw: [511, 0, 257],
            owner_entity_id: 0x04fc_0001,
            owner_entity_type: 9,
            suppresses_impact_damage: false,
        };
        let slot = fx
            .materialize_actor_surface_bubble_request(
                request,
                ParticleEnvironment::FlatWater { sea_level: 0.0 },
                17,
            )
            .expect("class-42 request should fit an empty retail pool");
        let particle = fx.particles.slots[slot].unwrap();
        assert_eq!(particle.source_class, PRIMARY_UNDERWATER_IMPACT_CLASS);
        assert_eq!(
            particle.position,
            raw_position_to_world(request.position_raw)
        );
        assert_eq!(
            particle.velocity.map(world_velocity_component_to_raw),
            [511, 400, 257],
            "the request's zero Y argument receives the descriptor +0x190 once"
        );
        assert_eq!(particle.owner_id, Some(request.owner_entity_id));
        assert_eq!(
            particle.source_entity_type_at_birth,
            Some(request.owner_entity_type)
        );
        assert!(!particle.suppresses_impact_damage);
        assert!(particle.water_state_initialized);
        assert_eq!(particle.water_state, 0);
    }

    #[test]
    fn actor_surface_bubble_adapter_reports_a_full_retail_pool() {
        let mut fx = WorldFx::new();
        for index in 0..MAX_WORLD_PARTICLES {
            assert!(fx
                .particles
                .allocate(descriptor_test_particle(
                    COMMON_EXPLOSION_SCATTER_CLASS,
                    [index as f32, 0.0, 0.0],
                    [0; 3],
                    None,
                ))
                .is_some());
        }
        let request = ActorSurfaceBubbleRequest {
            position_raw: [0; 3],
            velocity_argument_raw: [0; 3],
            owner_entity_id: 7,
            owner_entity_type: 9,
            suppresses_impact_damage: false,
        };
        assert!(fx
            .materialize_actor_surface_bubble_request(request, ParticleEnvironment::Dry, 0,)
            .is_none());
        assert_eq!(fx.particle_count(), MAX_WORLD_PARTICLES);
    }

    #[test]
    fn class_87_request_reproduces_allocator_suppression_gate_and_zero_owner_type() {
        let mut fx = WorldFx::new();
        let suppressed = DescriptorParticleRequest {
            source_class: 87,
            position_raw: [0; 3],
            velocity_raw: [0; 3],
            owner: None,
            suppresses_impact_damage: true,
        };
        assert_eq!(
            particle_descriptor(FIRST_WORLD_SHOOTER_PARTICLE_CLASS)
                .unwrap()
                .flags()
                & 0x02,
            0x02
        );
        assert!(fx
            .materialize_descriptor_particle_request(suppressed, ParticleEnvironment::Dry, 0)
            .is_none());
        assert_eq!(fx.particle_count(), 0);

        let slot = fx
            .materialize_descriptor_particle_request(
                DescriptorParticleRequest {
                    suppresses_impact_damage: false,
                    ..suppressed
                },
                ParticleEnvironment::Dry,
                0,
            )
            .unwrap();
        assert_eq!(
            fx.particles.slots[slot]
                .unwrap()
                .source_entity_type_at_birth,
            Some(0),
            "F40A60 writes zero when no owner lookup succeeds"
        );
    }

    #[test]
    fn class_87_callback_identity_and_terminal_surface_selectors_are_bounded() {
        assert!(uses_class_87_sweep_callbacks(
            FIRST_WORLD_SHOOTER_PARTICLE_CLASS
        ));
        assert!(uses_projectile_surface_callback(
            FIRST_WORLD_SHOOTER_PARTICLE_CLASS
        ));
        assert_eq!(
            (0..PARTICLE_DESCRIPTORS.len())
                .filter(|source_class| uses_class_87_sweep_callbacks(*source_class as u8))
                .collect::<Vec<_>>(),
            vec![usize::from(FIRST_WORLD_SHOOTER_PARTICLE_CLASS)]
        );
        // Class 87 shares FUN_0043F590 with the primary family but carries its
        // own packet (`0x004CC0F0`) and static callback (`FUN_0042E8E0`). Class
        // 1's packet is `0x004CBF70`; class 3's adjacent `0x004CBF88` is not a
        // substitute. The primary gate must admit 1, 2 and 3 only so player
        // bullets still hit and the ant projectile reaches its own sweep.
        assert_eq!(
            particle_descriptor(PRIMARY_BULLET_PARTICLE_CLASS)
                .unwrap()
                .raw_u32(0x20),
            PRIMARY_CLASS1_DAMAGE_PACKET_VA
        );
        assert_eq!(
            particle_descriptor(UPGRADED_PRIMARY_PARTICLE_CLASS)
                .unwrap()
                .raw_u32(0x20),
            PRIMARY_CLASS3_DAMAGE_PACKET_VA
        );
        let primary_admitted: Vec<usize> = (0..PARTICLE_DESCRIPTORS.len())
            .filter(|source_class| is_supported_primary_projectile(*source_class as u8))
            .collect();
        assert_eq!(
            primary_admitted,
            vec![
                usize::from(PRIMARY_BULLET_PARTICLE_CLASS),
                usize::from(RAPID_PRIMARY_PARTICLE_CLASS),
                usize::from(UPGRADED_PRIMARY_PARTICLE_CLASS)
            ]
        );
        assert!(!is_supported_primary_projectile(
            FIRST_WORLD_SHOOTER_PARTICLE_CLASS
        ));

        let mut fx = WorldFx::new();
        let slot = fx
            .materialize_descriptor_particle_request(
                DescriptorParticleRequest {
                    source_class: 87,
                    position_raw: [0; 3],
                    velocity_raw: [0; 3],
                    owner: Some(ParticleOwnerAtBirth {
                        entity_id: 47,
                        entity_type: 47,
                    }),
                    suppresses_impact_damage: false,
                },
                ParticleEnvironment::Dry,
                0,
            )
            .unwrap();
        let before = fx.particles.slots[slot].unwrap();
        let mut particle = before;
        assert!(!fx.apply_projectile_surface_response(slot, &mut particle, 7, 12.0));
        assert_eq!(particle, before);
        assert!(!fx.apply_projectile_surface_response(slot, &mut particle, 13, 12.0));
        assert_eq!(particle, before);
    }

    #[test]
    fn class_87_entity_update_emits_visual_typed_damage_velocity_and_deletes_parent() {
        let mut fx = WorldFx::new();
        let request = DescriptorParticleRequest {
            source_class: 87,
            position_raw: [4 << 8, 2 << 8, 5 << 8],
            velocity_raw: [300, -40, 900],
            owner: Some(ParticleOwnerAtBirth {
                entity_id: 47,
                entity_type: 47,
            }),
            suppresses_impact_damage: false,
        };
        let parent_slot = fx
            .materialize_descriptor_particle_request(request, ParticleEnvironment::Dry, 17)
            .unwrap();
        let parent = fx.particles.slots[parent_slot].unwrap();
        let pool = solid_sphere_pool(100);
        let entities = [collision_entity(9, parent.position, 100)];

        let outcome = fx.update(ParticleUpdateRequest::dry(0, 17).with_callbacks(
            ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &entities,
                    model_pool: &pool,
                }),
            },
        ));

        assert_eq!(
            outcome.entity_impacts,
            vec![ParticleEntityImpact {
                source_particle_class: FIRST_WORLD_SHOOTER_PARTICLE_CLASS,
                impact_position_argument_va: retail_particle_impact_position_argument_va(
                    parent_slot,
                ),
                target_entity_id: 9,
                position_world: parent.position,
                velocity_raw: request.velocity_raw,
                damage: Some(BallisticDamageRequest {
                    packet: TYPE_47_PROJECTILE_DAMAGE_PACKET,
                    source_entity_type_at_birth: Some(47),
                    source_owner_id: Some(47),
                }),
            }]
        );
        assert!(fx.particles.slots[parent_slot].is_none());
        assert!(fx.particles.slots.iter().flatten().any(|particle| {
            particle.source_class == PRIMARY_ABOVE_WATER_IMPACT_CLASS
                && particle.position == parent.position
        }));
        assert!(outcome.primary_impacts.is_empty());
        assert!(outcome.ballistic_static_impacts.is_empty());
        assert!(outcome.terrain_type_mutations.is_empty());
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn class_87_synthetic_suppression_preserves_visual_and_delete_without_damage() {
        let mut fx = WorldFx::new();
        let parent_slot = fx
            .materialize_descriptor_particle_request(
                DescriptorParticleRequest {
                    source_class: 87,
                    position_raw: [4 << 8, 2 << 8, 5 << 8],
                    velocity_raw: [0; 3],
                    owner: Some(ParticleOwnerAtBirth {
                        entity_id: 47,
                        entity_type: 47,
                    }),
                    suppresses_impact_damage: false,
                },
                ParticleEnvironment::Dry,
                0,
            )
            .unwrap();
        // F40A60 rejects a suppressed class-87 request. Mutating an already
        // live record explicitly isolates the collision callback's +1D bit-0
        // behavior without misrepresenting this as an allocator outcome.
        fx.particles.slots[parent_slot]
            .as_mut()
            .unwrap()
            .suppresses_impact_damage = true;
        let parent = fx.particles.slots[parent_slot].unwrap();
        let pool = solid_sphere_pool(100);
        let entities = [collision_entity(9, parent.position, 100)];

        let outcome = fx.update(ParticleUpdateRequest::dry(0, 0).with_callbacks(
            ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &entities,
                    model_pool: &pool,
                }),
            },
        ));

        assert!(matches!(
            outcome.entity_impacts.as_slice(),
            [ParticleEntityImpact {
                target_entity_id: 9,
                damage: None,
                ..
            }]
        ));
        assert!(fx.particles.slots[parent_slot].is_none());
        let visuals = fx
            .particles
            .slots
            .iter()
            .flatten()
            .filter(|particle| particle.source_class == PRIMARY_ABOVE_WATER_IMPACT_CLASS)
            .collect::<Vec<_>>();
        assert_eq!(visuals.len(), 1);
        assert!(visuals[0].suppresses_impact_damage);
    }

    #[test]
    fn class_87_static_update_deletes_without_visual_or_damage_family_leakage() {
        let mut terrain = flat_terrain(16, 0);
        *terrain_cell_mut(&mut terrain, 3, 4) = TerrainCell {
            height: 16,
            attribute: 1,
            terrain_type: 0x18,
        };
        let objects = terrain_object_table(1, [0, 0, 0, 0], 29);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: Some(&objects),
            ground_response_selectors: [5; 8],
            water_response_selectors: [7; 8],
        };
        let pool = solid_sphere_pool(200);
        let mut fx = WorldFx::new();
        let parent_slot = fx
            .materialize_descriptor_particle_request(
                DescriptorParticleRequest {
                    source_class: 87,
                    position_raw: [4 << 8, 2 << 8, 5 << 8],
                    velocity_raw: [0; 3],
                    owner: Some(ParticleOwnerAtBirth {
                        entity_id: 66,
                        entity_type: 47,
                    }),
                    suppresses_impact_damage: false,
                },
                ParticleEnvironment::Terrain(context),
                0,
            )
            .unwrap();

        let outcome = fx.update(
            ParticleUpdateRequest::terrain(0, 0, context).with_callbacks(ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &[],
                    model_pool: &pool,
                }),
            }),
        );

        assert_eq!(
            outcome.class_87_static_impacts,
            vec![Class87StaticTileImpact {
                position_world: [4.0, 2.0, 5.0],
                cell: [3, 4],
                attribute: 1,
                terrain_type: 0x18,
                model_id: 0,
                kind_index: 29,
            }]
        );
        assert!(fx.particles.slots[parent_slot].is_none());
        assert!(
            !fx.particles.slots.iter().flatten().any(|particle| matches!(
                particle.source_class,
                PRIMARY_ABOVE_WATER_IMPACT_CLASS | PRIMARY_UNDERWATER_IMPACT_CLASS
            ))
        );
        assert!(outcome.primary_impacts.is_empty());
        assert!(outcome.ballistic_static_impacts.is_empty());
        assert!(outcome.terrain_type_mutations.is_empty());
    }

    #[test]
    fn class_87_water_conversion_continues_into_replacement_ground_tail() {
        let mut terrain = flat_terrain(0, 0);
        terrain.header[0] = 256;
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [0; 8],
            water_response_selectors: [0; 8],
        };
        let mut fx = WorldFx::new();
        let parent_slot = fx
            .materialize_descriptor_particle_request(
                DescriptorParticleRequest {
                    source_class: 87,
                    position_raw: [4 << 8, 128, 2 << 8],
                    velocity_raw: [0, -8_000, 0],
                    owner: Some(ParticleOwnerAtBirth {
                        entity_id: 47,
                        entity_type: 47,
                    }),
                    suppresses_impact_damage: false,
                },
                ParticleEnvironment::Terrain(context),
                0,
            )
            .unwrap();

        let outcome = fx.update(ParticleUpdateRequest::terrain(20_000, 1, context));

        let replacement = fx.particles.slots[parent_slot].unwrap();
        assert_eq!(replacement.source_class, 73);
        assert_eq!(replacement.position[1], 0.0);
        assert_eq!(replacement.velocity, [0.0; 3]);
        assert_eq!(replacement.age_ticks, 0.0);
        let priority = particle_descriptor(73).unwrap().priority_raw() as usize;
        assert_eq!(fx.particles.lists[priority].tail, Some(parent_slot));
        assert!(outcome.entity_impacts.is_empty());
        assert!(outcome.class_87_static_impacts.is_empty());
        assert!(outcome.primary_impacts.is_empty());
        assert!(outcome.ballistic_static_impacts.is_empty());
        assert!(outcome.terrain_type_mutations.is_empty());
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn class_87_replacement_selector_seven_uses_current_emitter_owner_then_deletes() {
        let mut terrain = flat_terrain(0, 0);
        terrain.header[0] = 256;
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [7; 8],
            water_response_selectors: [0; 8],
        };
        let mut fx = WorldFx::new();
        let parent_slot = fx
            .materialize_descriptor_particle_request(
                DescriptorParticleRequest {
                    source_class: 87,
                    position_raw: [4 << 8, 128, 2 << 8],
                    velocity_raw: [0, -8_000, 0],
                    owner: Some(ParticleOwnerAtBirth {
                        entity_id: 47,
                        entity_type: 47,
                    }),
                    suppresses_impact_damage: false,
                },
                ParticleEnvironment::Terrain(context),
                0,
            )
            .unwrap();
        fx.particles.slots[parent_slot]
            .as_mut()
            .unwrap()
            .suppresses_impact_damage = true;

        let current_emitter_owner = ParticleOwnerAtBirth {
            entity_id: 0x04c7_0001,
            entity_type: 68,
        };
        let outcome = fx.update(
            ParticleUpdateRequest::terrain(20_000, 1, context).with_particle_emitter(
                ParticleEmitterContext {
                    current_owner: Some(current_emitter_owner),
                },
            ),
        );

        assert!(fx.particles.slots[parent_slot].is_none());
        let children = fx
            .particles
            .slots
            .iter()
            .flatten()
            .filter(|particle| {
                matches!(
                    particle.source_class,
                    UPGRADED_PRIMARY_SUBMERGED_SURFACE_CLASS | UPGRADED_PRIMARY_ABOVE_SURFACE_CLASS
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(children.len(), 1);
        assert_eq!(
            children[0].owner_id,
            Some(current_emitter_owner.entity_id),
            "F3E3D0 takes the F41770 owner from current DAT_004DCA00, not parent +0x18"
        );
        assert_eq!(
            children[0].source_entity_type_at_birth,
            Some(current_emitter_owner.entity_type),
            "the explicit context carries the F40A60 lookup result without inheriting parent +0x1C"
        );
        assert!(children[0].suppresses_impact_damage);
        let expected_velocity_raw = [
            UPGRADED_PRIMARY_SURFACE_VELOCITY_RAW[0],
            UPGRADED_PRIMARY_SURFACE_VELOCITY_RAW[1]
                + i32::from(
                    particle_descriptor(children[0].source_class)
                        .unwrap()
                        .spawn_velocity_y_bias_raw(),
                ),
            UPGRADED_PRIMARY_SURFACE_VELOCITY_RAW[2],
        ];
        assert_eq!(
            children[0].velocity.map(world_velocity_component_to_raw),
            expected_velocity_raw
        );
        assert!(outcome.entity_impacts.is_empty());
        assert!(outcome.class_87_static_impacts.is_empty());
    }

    fn center_projection(
        screen: [i32; 2],
        depth_raw: i32,
        clip: u8,
    ) -> v2k_render::ParticleCenterProjection {
        v2k_render::ParticleCenterProjection {
            screen,
            depth_raw,
            clip,
        }
    }

    #[test]
    fn frozen_particle_reprojection_preserves_admitted_streaks_and_jitter() {
        const FAR: i32 = 0x1800;
        for stack_address in [None, Some(0x0090_01a0)] {
            let mut fx = WorldFx::new();
            let slot = fx
                .particles
                .allocate(descriptor_test_particle(
                    87,
                    [4.0, 2.0, 0.0],
                    [3, 4, 0],
                    None,
                ))
                .unwrap();
            let frame = fx.prepare_presentation_with_stack_addresses(
                [320, 240],
                FAR,
                ParticlePresentationStackAddresses {
                    backward_streak_record: stack_address,
                    ..Default::default()
                },
                |_| center_projection([160, 120], 0x100, 0),
            );
            assert_eq!(frame.particles().len(), 7);
            let original = frame.clone();
            let original_scales: Vec<_> = frame
                .particles()
                .map(|sample| sample.native_effective_draw_scale_raw())
                .collect();
            assert!(original_scales[0].is_ok());
            assert_eq!(original_scales[1].is_ok(), stack_address.is_some());

            // A birth after the live traversal must not enter frozen replay.
            fx.particles
                .allocate(descriptor_test_particle(31, [9.0, 2.0, 0.0], [0; 3], None))
                .unwrap();
            let rng = fx.rng_state;
            let live_source = fx.particles.slots[slot];
            let mut projected_inputs = Vec::new();
            let mut projections = Vec::new();
            let replay = frame.reprojected([640, 480], |particle| {
                let index = projected_inputs.len();
                projected_inputs.push(*particle);
                let projection = match index {
                    0 => center_projection([500, 360], 0x100, 0x14),
                    1 => center_projection([-10_000, -10_000], -1, 0x41),
                    2 => center_projection([320, 240], FAR, 0),
                    _ => center_projection([320 + index as i32, 240], 0x100, 0),
                };
                projections.push(projection);
                projection
            });

            assert_eq!(frame, original, "the frozen frame remains immutable");
            assert_eq!(replay.particles().len(), original.particles().len());
            assert_eq!(
                projected_inputs,
                original
                    .particles()
                    .map(|sample| sample.particle)
                    .collect::<Vec<_>>(),
                "each admitted copied record is projected once in its original order"
            );
            for ((updated, retained), projection) in replay
                .particles()
                .zip(original.particles())
                .zip(projections)
            {
                assert_eq!(updated.projection, projection);
                assert_eq!(
                    PreparedParticle {
                        projection: retained.projection,
                        ..*updated
                    },
                    *retained,
                    "only the projection changes, including for out-of-view records"
                );
            }
            assert_eq!(
                replay
                    .particles()
                    .map(|sample| sample.native_effective_draw_scale_raw())
                    .collect::<Vec<_>>(),
                original_scales,
                "live-slot and known/unknown copied-stack jitter custody is retained"
            );
            let samples: Vec<_> = replay.particles().collect();
            assert!(!original.sprite_visible(samples[0], [8, 8]));
            assert!(replay.sprite_visible(samples[0], [8, 8]));
            assert!(!replay.sprite_visible(samples[1], [8, 8]));
            assert!(!replay.sprite_visible(samples[2], [8, 8]));
            assert!(replay.sprite_visible(samples[3], [8, 8]));
            assert_eq!(
                fx.particle_count(),
                2,
                "reprojection does not cull the pool"
            );
            assert_eq!(fx.particles.slots[slot], live_source);
            assert_eq!(fx.rng_state, rng);
        }
    }

    #[test]
    fn d410_centered_beyond_far_remains_active_for_light_but_not_sprite() {
        let projection = center_projection([160, 120], 0x1800, 0x12);
        assert_eq!(
            d410_pre_cull(projection, [320, 240], 0x1800, 0),
            ParticlePreCull::Active
        );
        assert!(!d410_sprite_visible(projection, [320, 240], 0x1800, [8, 8]));
    }

    #[test]
    fn d410_hard_reject_obeys_retain_hidden_descriptor_flag() {
        let behind = center_projection([0, 0], -1, 0x40);
        assert_eq!(
            d410_pre_cull(behind, [320, 240], 0x1800, 0),
            ParticlePreCull::Delete
        );
        assert_eq!(
            d410_pre_cull(behind, [320, 240], 0x1800, 0x02),
            ParticlePreCull::RetainHidden
        );
    }

    #[test]
    fn d410_extreme_center_boundary_uses_strict_greater_than() {
        let at_boundary = center_projection([480, 120], 0x100, 0x14);
        let beyond_boundary = center_projection([481, 120], 0x100, 0x14);
        assert_eq!(
            d410_pre_cull(at_boundary, [320, 240], 0x1800, 0),
            ParticlePreCull::Active
        );
        assert_eq!(
            d410_pre_cull(beyond_boundary, [320, 240], 0x1800, 0),
            ParticlePreCull::Delete
        );
    }

    #[test]
    fn d410_offscreen_far_center_is_deleted_before_lighting() {
        let offscreen = center_projection([-1, 120], 0x1800, 0x11);
        assert_eq!(
            d410_pre_cull(offscreen, [320, 240], 0x1800, 0),
            ParticlePreCull::Delete
        );
    }

    #[test]
    fn d410_expanded_sprite_overlap_keeps_left_touch_and_rejects_right_touch() {
        let left_touch = center_projection([-10, 120], 0x100, 0x11);
        let left_outside = center_projection([-11, 120], 0x100, 0x11);
        let right_touch = center_projection([330, 120], 0x100, 0x14);
        let right_inside = center_projection([329, 120], 0x100, 0x14);
        assert!(d410_sprite_visible(
            left_touch,
            [320, 240],
            0x1800,
            [10, 10]
        ));
        assert!(!d410_sprite_visible(
            left_outside,
            [320, 240],
            0x1800,
            [10, 10]
        ));
        assert!(!d410_sprite_visible(
            right_touch,
            [320, 240],
            0x1800,
            [10, 10]
        ));
        assert!(d410_sprite_visible(
            right_inside,
            [320, 240],
            0x1800,
            [10, 10]
        ));
    }

    struct TestModelPool(Vec<ModelEntry>);

    impl CollisionModelPool for TestModelPool {
        fn collision_model(&self, global_id: usize) -> Option<&ModelEntry> {
            self.0.get(global_id)
        }
    }

    fn static_emitter_source(
        position: [f32; 3],
        terrain_type: u8,
        kind_index: u32,
    ) -> StaticTerrainObjectInstance {
        StaticTerrainObjectInstance {
            model_id: 425,
            kind_index,
            attribute: 21,
            terrain_type,
            cell: [188, 134],
            position,
        }
    }

    fn pool_test_particle(source_class: u8, marker: f32) -> WorldParticle {
        let mut particle = meteor_trail_particle([marker, 2.0, 3.0]);
        particle.source_class = source_class;
        particle
    }

    fn descriptor_test_particle(
        source_class: u8,
        position: [f32; 3],
        raw_velocity: [i32; 3],
        owner_id: Option<u32>,
    ) -> WorldParticle {
        let spawn =
            single_descriptor_particle_burst(position, source_class, raw_velocity, owner_id)
                .particles[0];
        world_particle(position, spawn)
    }

    fn seed_for_above_static_pair() -> u32 {
        (0..u32::MAX)
            .find(|&seed| {
                let mut state = seed;
                retail_random_u16(&mut state) & 7 == 0 && retail_random_u16(&mut state) & 0x0f == 0
            })
            .expect("small seed for both static gates")
    }

    fn seed_for_underwater_static_pair() -> u32 {
        (0..u32::MAX)
            .find(|&seed| {
                let mut state = seed;
                let puff_gate = retail_random_u16(&mut state) & 7 == 0;
                let _puff_x_jitter = retail_random_u16(&mut state);
                let flame_gate = retail_random_u16(&mut state) & 0x0f == 0;
                puff_gate && flame_gate
            })
            .expect("small seed for both underwater static gates")
    }

    fn collision_model(
        records: Vec<[i16; 4]>,
        collision_program: Vec<u8>,
        collision_radius_raw: u16,
    ) -> ModelEntry {
        ModelEntry {
            index: 0,
            cmd_word_count: 0,
            extra_count: collision_program.len().div_ceil(4) as u8,
            flags: 0x40,
            slot_count: (records.len() * 2) as u16,
            face_val: 2,
            radius: 0,
            collision_radius_raw,
            collision_program,
            records,
            normal_pool: Vec::new(),
            cmd_words: Vec::new(),
            has_view_commands: false,
            vertices: Vec::new(),
            vertex_type_flags: Vec::new(),
            vertex_projection: Vec::new(),
            vertex_clip: Vec::new(),
            vertex_surface_origin: Vec::new(),
            triangles: Vec::new(),
            face_vertices: Vec::new(),
            normals: Vec::new(),
            face_cull: Vec::new(),
            face_materials: Vec::new(),
            face_uvs: Vec::new(),
            face_corner_normals: Vec::new(),
            face_shading: Vec::new(),
            shadow_triangles: Vec::new(),
            edges: Vec::new(),
            billboards: Vec::<Billboard>::new(),
            instances: Vec::<ModelInstance>::new(),
            painter_program: Vec::new(),
            name: None,
        }
    }

    fn solid_sphere_pool(radius_raw: u32) -> TestModelPool {
        TestModelPool(vec![collision_model(
            vec![[0, 0, 0, 0]],
            vec![
                0x8E,
                0,
                0,
                0,
                radius_raw as u8,
                (radius_raw >> 8) as u8,
                (radius_raw >> 16) as u8,
                (radius_raw >> 24) as u8,
                0,
                0,
                0x88,
                0,
            ],
            radius_raw.min(u32::from(u16::MAX)) as u16,
        )])
    }

    fn collision_entity(
        entity_id: u32,
        center_world: [f32; 3],
        radius_raw: u16,
    ) -> EntityCollisionModel {
        EntityCollisionModel {
            entity_id,
            center_world,
            radius_raw,
            model_id: 0,
            orientation_world_from_model: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            anim_vars: AnimVars::default(),
            state_flags_at_0x08: Some(0),
            capability_flags_at_0x64: 0,
        }
    }

    fn primary_geometry(origin_y: f32, shooter_velocity: [f32; 3]) -> PrimaryFireGeometry {
        PrimaryFireGeometry {
            launch_basis: PrimaryLaunchBasis {
                origin_world: [4.0, origin_y, 5.0],
                direction_unit: [1.0, 0.0, 0.0],
            },
            gun_mounts: PrimaryGunMounts::Authored(PrimaryAuthoredGunMounts {
                gun_a: PrimaryLaunchBasis {
                    origin_world: [4.5, origin_y + 0.25, 5.75],
                    direction_unit: [1.0, 0.0, 0.0],
                },
                gun_b: PrimaryLaunchBasis {
                    origin_world: [3.5, origin_y + 0.25, 5.75],
                    direction_unit: [1.0, 0.0, 0.0],
                },
            }),
            shooter_origin_world: [4.0, origin_y, 5.0],
            shooter_id: 7,
            shooter_entity_type_at_birth: Some(46),
            shooter_velocity_world: shooter_velocity,
            sound_origin_world: [4.0, origin_y, 5.0],
        }
    }

    fn primary_event(origin_y: f32, shooter_velocity: [f32; 3]) -> PrimaryFireEvent {
        let mut weapon = PrimaryWeapon::new();
        weapon.update(
            Duration::ZERO,
            PrimaryTriggerInput {
                source_a: true,
                source_b: false,
            },
            primary_geometry(origin_y, shooter_velocity),
        )[0]
    }

    fn upgraded_primary_event(origin_y: f32, shooter_velocity: [f32; 3]) -> PrimaryFireEvent {
        let mut weapon = PrimaryWeapon::new();
        weapon.update_with_profile(
            Duration::ZERO,
            PrimaryTriggerInput {
                source_a: true,
                source_b: false,
            },
            primary_geometry(origin_y, shooter_velocity),
            UPGRADED_PRIMARY_PROFILE,
            PrimaryShotBudget::Limited(1),
        )[0]
    }

    fn player_surface_emission(
        position_raw: [i16; 3],
        velocity_raw: [i16; 3],
        strength_raw: i32,
    ) -> PlayerSurfaceEffectEmission {
        PlayerSurfaceEffectEmission {
            position_raw,
            velocity_raw,
            lateral_q31: [i32::MAX, 0, 0],
            up_q31: [0, i32::MAX, 0],
            forward_q31: [0, 0, i32::MAX],
            strength_raw,
            owner_entity_id: 46,
        }
    }

    fn flat_terrain(height: i8, terrain_type: u8) -> TerrainGrid {
        TerrainGrid {
            header: [i32::MIN, 0, 0, 0, 0],
            cells: vec![
                TerrainCell {
                    height: height as u8,
                    attribute: 0,
                    terrain_type,
                };
                GRID_SIZE * GRID_SIZE
            ],
        }
    }

    fn terrain_cell_mut(terrain: &mut TerrainGrid, x: usize, z: usize) -> &mut TerrainCell {
        &mut terrain.cells[x * GRID_SIZE + z]
    }

    fn terrain_object_table(
        attribute: u8,
        model_ids: [u16; 4],
        kind_index: u32,
    ) -> TerrainObjectTable {
        let empty = TerrainObjectDescriptor {
            model_ids: [0; 4],
            kind_index: 0,
            pattern: ModelSlotPattern::Static,
        };
        let mut records = vec![empty; 256];
        records[usize::from(attribute)] = TerrainObjectDescriptor {
            model_ids,
            kind_index,
            pattern: ModelSlotPattern::Varied,
        };
        TerrainObjectTable { records }
    }

    #[test]
    fn particle_pool_virgin_allocations_consume_physical_tail() {
        let mut pool = ParticlePool::default();
        assert_eq!(pool.lists[0].head, Some(0));
        assert_eq!(pool.lists[0].tail, Some(199));

        assert_eq!(pool.allocate(pool_test_particle(31, 1.0)), Some(199));
        assert_eq!(pool.allocate(pool_test_particle(31, 2.0)), Some(198));
        assert_eq!(pool.lists[0].tail, Some(197));
        assert_eq!(pool.lists[2].head, Some(198));
        assert_eq!(pool.lists[2].tail, Some(199));
        pool.assert_valid_topology();
    }

    #[test]
    fn particle_pool_recycles_lower_priority_even_at_age_zero() {
        let mut pool = ParticlePool::default();
        for marker in 0..MAX_WORLD_PARTICLES {
            assert_eq!(
                pool.allocate(pool_test_particle(31, marker as f32)),
                Some(MAX_WORLD_PARTICLES - 1 - marker)
            );
        }

        // Class 16 is priority four; class 31 is priority two. Retail takes
        // the lower-priority tail without applying the same-priority age gate.
        assert_eq!(pool.allocate(pool_test_particle(16, 400.0)), Some(199));
        assert_eq!(pool.slots[199].unwrap().source_class, 16);
        assert_eq!(pool.live_count, MAX_WORLD_PARTICLES);
        pool.assert_valid_topology();
    }

    #[test]
    fn particle_pool_same_priority_requires_nonzero_age() {
        let mut pool = ParticlePool::default();
        for marker in 0..MAX_WORLD_PARTICLES {
            pool.allocate(pool_test_particle(31, marker as f32))
                .unwrap();
        }

        assert_eq!(pool.allocate(pool_test_particle(31, 300.0)), None);
        pool.slots[199].as_mut().unwrap().age_ticks = 1.0;
        assert_eq!(pool.allocate(pool_test_particle(31, 301.0)), Some(199));
        assert_eq!(pool.slots[199].unwrap().position[0], 45.0);
        assert_eq!(pool.lists[2].head, Some(199));
        pool.assert_valid_topology();
    }

    #[test]
    fn freed_slots_enter_at_head_while_allocator_consumes_tail() {
        let mut pool = ParticlePool::default();
        for marker in 0..MAX_WORLD_PARTICLES {
            pool.allocate(pool_test_particle(31, marker as f32))
                .unwrap();
        }
        assert_eq!(pool.lists[0], ParticleList::default());

        assert!(pool.free(50));
        assert!(pool.free(51));
        assert_eq!(pool.lists[0].head, Some(51));
        assert_eq!(pool.lists[0].tail, Some(50));
        assert_eq!(pool.allocate(pool_test_particle(31, 250.0)), Some(50));
        assert_eq!(pool.allocate(pool_test_particle(31, 251.0)), Some(51));
        pool.assert_valid_topology();
    }

    #[test]
    fn particle_presentation_walks_priority_then_newest_to_oldest() {
        let mut pool = ParticlePool::default();
        for (source_class, marker) in [
            (83, 60.0),
            (31, 20.0),
            (18, 30.0),
            (16, 40.0),
            (14, 10.0),
            (1, 50.0),
            (31, 21.0),
            (18, 31.0),
        ] {
            pool.allocate(pool_test_particle(source_class, marker))
                .unwrap();
        }

        let presented = pool
            .presentation()
            .map(|entry| (entry.particle.source_class, entry.particle.position[0]))
            .collect::<Vec<_>>();
        assert_eq!(
            presented,
            [
                (14, 10.0),
                (31, 21.0),
                (31, 20.0),
                (18, 31.0),
                (18, 30.0),
                (16, 40.0),
                (1, 50.0),
                (83, 60.0),
            ]
        );
        pool.assert_valid_topology();
    }

    #[test]
    fn particle_slot_address_supplies_exact_repeating_size_phase() {
        let mut particle = pool_test_particle(16, 0.0);
        particle.draw_scale_raw = 320;
        particle.size_jitter_divisor_raw = 32;
        let scale = |slot| {
            PresentedParticle {
                slot,
                particle: &particle,
            }
            .effective_draw_scale_raw()
        };

        // Pool base 0x4DCF40 starts at phase 10; 0x20-byte records advance
        // it once per slot and wrap after physical slot five.
        assert_eq!(scale(0), 420);
        assert_eq!(scale(5), 470);
        assert_eq!(scale(6), 320);
        assert_eq!(scale(15), 410);
        assert_eq!(scale(16), 420);
    }

    #[test]
    fn clearing_active_lists_restores_one_valid_free_pool() {
        let mut pool = ParticlePool::default();
        for (source_class, marker) in [(14, 1.0), (31, 2.0), (18, 3.0), (16, 4.0), (1, 5.0)] {
            pool.allocate(pool_test_particle(source_class, marker))
                .unwrap();
        }
        pool.clear_active();

        assert_eq!(pool.live_count, 0);
        assert!(pool.slots.iter().all(Option::is_none));
        assert!(pool.lists[1..]
            .iter()
            .all(|list| *list == ParticleList::default()));
        let mut free_order = Vec::with_capacity(MAX_WORLD_PARTICLES);
        let mut next = pool.lists[0].head;
        while let Some(slot) = next {
            free_order.push(slot);
            next = pool.links[slot].next;
        }
        assert_eq!(free_order, (195..=199).chain(0..=194).collect::<Vec<_>>());
        pool.assert_valid_topology();
    }

    #[test]
    fn public_static_chance_draw_uses_the_existing_shared_retail_stream() {
        let mut expected_state = 0;
        let expected_first = retail_random_u16(&mut expected_state);
        let expected_second = retail_random_u16(&mut expected_state);

        let mut fx = WorldFx::new();
        assert_eq!(fx.next_shared_retail_random_u16(), expected_first);
        assert_eq!(fx.next_shared_retail_random_u16(), expected_second);
        assert_eq!(fx.rng_state, expected_state);
    }

    #[test]
    fn primary_hit_capability_follow_up_emits_exact_full_rate_class_5_carriers() {
        let position_raw = [-128, 2_048, i16::MIN];
        let target_handle = 0x04aa_0001;
        let mut fx = full_rate_world_fx();

        fx.emit_primary_hit_capability_follow_up_raw(primary_hit_capability_emission(
            position_raw,
            target_handle,
            0,
        ));

        assert_eq!(fx.direction_cursor, 2);
        assert_eq!(fx.rng_state, 0, "FUN_00440DC0 consumes no RNG");
        let particles = fx.test_particles_in_virgin_birth_order();
        assert_eq!(particles.len(), 2);
        for (particle, expected_velocity_raw) in
            particles.iter().zip([[340, 122, -234], [296, 250, 582]])
        {
            assert_eq!(particle.source_class, ALIEN_HIVE_PARTICLE_CLASS);
            assert_eq!(particle.owner_id, Some(target_handle));
            assert_eq!(particle.position, raw_position_to_world(position_raw));
            assert_eq!(
                particle.velocity.map(world_velocity_component_to_raw),
                expected_velocity_raw
            );
            assert!(!particle.suppresses_impact_damage);
        }
    }

    #[test]
    fn primary_hit_capability_owner_sign_deletes_carrier_without_terrain_mutation() {
        let terrain = flat_terrain(0, 0);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [0; 8],
            water_response_selectors: [0; 8],
        };
        let target_handle = 0x04aa_0001;
        let mut fx = WorldFx::new();
        fx.emit_primary_hit_capability_follow_up_raw(primary_hit_capability_emission(
            [0; 3],
            target_handle,
            1,
        ));

        let carriers = fx.test_particles_in_virgin_birth_order();
        assert_eq!(carriers.len(), 1, "cold pacing retains the min-one tail");
        assert_eq!(carriers[0].owner_id, Some(target_handle));
        assert!(carriers[0].suppresses_impact_damage);

        let outcome = fx.update(ParticleUpdateRequest::terrain(0, 0, context));

        assert_eq!(fx.particle_count(), 0);
        assert!(outcome.terrain_type_mutations.is_empty());
    }

    #[test]
    fn primary_hit_capability_rejected_allocations_advance_every_direction_attempt() {
        let mut fx = full_rate_world_fx();
        for marker in 0..MAX_WORLD_PARTICLES {
            // Class 1 occupies priority five, so lower-priority class 5 has
            // no free or recyclable candidate for either full-rate attempt.
            fx.particles
                .allocate(pool_test_particle(
                    PRIMARY_BULLET_PARTICLE_CLASS,
                    marker as f32,
                ))
                .unwrap();
        }

        fx.emit_primary_hit_capability_follow_up_raw(primary_hit_capability_emission(
            [0; 3],
            0x04aa_0001,
            0,
        ));

        assert_eq!(fx.direction_cursor, 2);
        assert_eq!(fx.rng_state, 0);
        assert_eq!(fx.particle_count(), MAX_WORLD_PARTICLES);
        assert!(fx
            .particles
            .slots
            .iter()
            .flatten()
            .all(|particle| particle.source_class == PRIMARY_BULLET_PARTICLE_CLASS));
        fx.particles.assert_valid_topology();
    }

    #[test]
    #[should_panic(expected = "primary-hit capability follow-up must retain authored class 5")]
    fn primary_hit_capability_follow_up_rejects_a_forged_particle_class() {
        let mut emission = primary_hit_capability_emission([0; 3], 1, 0);
        emission.particle_class = ALIEN_HIVE_PARTICLE_CLASS + 1;
        WorldFx::new().emit_primary_hit_capability_follow_up_raw(emission);
    }

    #[test]
    fn defecate_virus_full_pacing_materializes_two_exact_class_5_carriers() {
        let position_raw = [-128, 2_048, i16::MIN];
        let owner_entity_handle = 0x04fc_0001;
        let mut fx = full_rate_world_fx();

        fx.emit_defecate_virus_particle_raw(DefecateVirusParticleEmission::new(
            position_raw,
            owner_entity_handle,
            true,
            TerrainContactMode::Infect,
        ));

        assert_eq!(fx.direction_cursor, 2);
        assert_eq!(fx.rng_state, 0, "the direction helper consumes no RNG");
        let particles = fx.test_particles_in_virgin_birth_order();
        assert_eq!(particles.len(), 2);
        for (particle, expected_velocity_raw) in
            particles.iter().zip([[340, 122, -234], [296, 250, 582]])
        {
            assert_eq!(particle.source_class, DEFECATE_VIRUS_PARTICLE_CLASS);
            assert_eq!(particle.owner_id, Some(owner_entity_handle));
            assert_eq!(particle.position, raw_position_to_world(position_raw));
            assert_eq!(
                particle.velocity.map(world_velocity_component_to_raw),
                expected_velocity_raw
            );
            assert!(particle.suppresses_impact_damage);
        }
    }

    #[test]
    fn defecate_virus_cold_pacing_preserves_the_minimum_one_attempt_tail() {
        let mut fx = WorldFx::new();
        assert_eq!(fx.frame_pacing.count_scale_q16, 0);

        fx.emit_defecate_virus_particle_raw(DefecateVirusParticleEmission::new(
            [16, 32, 48],
            77,
            false,
            TerrainContactMode::Infect,
        ));

        assert_eq!(fx.direction_cursor, 1);
        let particles = fx.test_particles_in_virgin_birth_order();
        assert_eq!(particles.len(), 1);
        assert_eq!(
            particles[0].velocity.map(world_velocity_component_to_raw),
            [340, 122, -234]
        );
        assert!(!particles[0].suppresses_impact_damage);
    }

    #[test]
    fn defecate_virus_rejected_allocations_still_advance_every_direction_attempt() {
        let mut fx = full_rate_world_fx();
        for marker in 0..MAX_WORLD_PARTICLES {
            // Class 1 occupies priority five, so the lower-priority class-5
            // carrier has no free or recyclable candidate.
            fx.particles
                .allocate(pool_test_particle(
                    PRIMARY_BULLET_PARTICLE_CLASS,
                    marker as f32,
                ))
                .unwrap();
        }

        fx.emit_defecate_virus_particle_raw(DefecateVirusParticleEmission::new(
            [0; 3],
            99,
            false,
            TerrainContactMode::Infect,
        ));

        assert_eq!(fx.direction_cursor, 2);
        assert_eq!(fx.rng_state, 0);
        assert_eq!(fx.particle_count(), MAX_WORLD_PARTICLES);
        assert!(fx
            .particles
            .slots
            .iter()
            .flatten()
            .all(|particle| particle.source_class == PRIMARY_BULLET_PARTICLE_CLASS));
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn alien_hive_birth_uses_exact_three_draw_velocity_and_descriptor_bias() {
        let mut expected_state = 0;
        let angle = retail_random_u16(&mut expected_state);
        let radius = i32::from(retail_random_u16(&mut expected_state) % 0x640);
        let vertical_argument = i32::from(retail_random_u16(&mut expected_state) & 0x01ff) + 0x02ee;
        let descriptor = particle_descriptor(ALIEN_HIVE_PARTICLE_CLASS).unwrap();
        let expected_velocity_raw = [
            (retail_sine_q15(u32::from(angle)) * radius) >> 16,
            vertical_argument + i32::from(descriptor.spawn_velocity_y_bias_raw()),
            (retail_sine_q15(u32::from(angle.wrapping_add(0x4000))) * radius) >> 16,
        ];

        let position_raw = [-128, 4096, i16::MIN];
        let source_id = 0x1234_5678;
        let mut fx = WorldFx::new();
        fx.emit_alien_hive_particle_raw(position_raw, source_id);

        assert_eq!(fx.rng_state, expected_state);
        let particle = fx.test_particles_in_virgin_birth_order()[0];
        assert_eq!(particle.source_class, ALIEN_HIVE_PARTICLE_CLASS);
        assert_eq!(particle.owner_id, Some(source_id));
        assert_eq!(particle.position, raw_position_to_world(position_raw));
        assert_eq!(
            particle.velocity.map(world_velocity_component_to_raw),
            expected_velocity_raw
        );
        assert_eq!(
            particle.sprite_id,
            first_particle_frame(ALIEN_HIVE_PARTICLE_CLASS).sprite_id
        );
        assert_eq!(particle.lifetime_ticks, descriptor.lifetime_ticks());
        assert_eq!(
            particle.collision_radius_raw,
            descriptor.collision_radius_raw()
        );
    }

    #[test]
    fn alien_hive_rejected_allocation_still_consumes_exactly_three_draws() {
        let mut fx = WorldFx::new();
        for marker in 0..MAX_WORLD_PARTICLES {
            // Class 1 occupies priority five, so the lower-priority virgin
            // class-5 allocation has no free or recyclable candidate.
            fx.particles
                .allocate(pool_test_particle(
                    PRIMARY_BULLET_PARTICLE_CLASS,
                    marker as f32,
                ))
                .unwrap();
        }
        let mut expected_state = 0;
        for _ in 0..3 {
            retail_random_u16(&mut expected_state);
        }

        fx.emit_alien_hive_particle_raw([0; 3], 99);

        assert_eq!(fx.rng_state, expected_state);
        assert_eq!(fx.particle_count(), MAX_WORLD_PARTICLES);
        assert!(fx
            .particles
            .slots
            .iter()
            .flatten()
            .all(|particle| particle.source_class == PRIMARY_BULLET_PARTICLE_CLASS));
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn player_surface_emitter_above_sea_creates_one_invisible_probe() {
        let emission = player_surface_emission([0x120, 0x200, -0x180], [11, 22, -33], 1000);
        let mut fx = WorldFx::new();

        fx.emit_player_surface_effect_raw(emission, 0);

        let particles = fx.test_particles_in_virgin_birth_order();
        assert_eq!(particles.len(), 1);
        let probe = particles[0];
        assert_eq!(probe.source_class, PLAYER_SURFACE_PROBE_PARTICLE_CLASS);
        assert_eq!(probe.owner_id, Some(46));
        assert_eq!(probe.position, raw_position_to_world(emission.position_raw));
        assert_eq!(
            probe.velocity.map(world_velocity_component_to_raw),
            [11, 22 - (i32::MAX >> 20), -33]
        );

        let presented = fx.prepare_presentation([320, 240], 0x1800, |_| {
            panic!("the invisible class-19 probe must not be projected")
        });
        assert_eq!(presented.particles().len(), 0);
        assert_eq!(fx.particle_count(), 1);
    }

    #[test]
    fn player_surface_emitter_at_sea_uses_strength_scaled_class_44_spray() {
        let emission = player_surface_emission([0x120, 0, -0x180], [11, 22, -33], 1000);
        let mut expected_rng = 0;
        for _ in 0..(7 * 4) {
            retail_random_u16(&mut expected_rng);
        }
        let mut fx = full_rate_world_fx();

        fx.emit_player_surface_effect_raw(emission, 0);

        let particles = fx.test_particles_in_virgin_birth_order();
        assert_eq!(particles.len(), 7);
        assert!(particles.iter().all(|particle| {
            particle.source_class == PLAYER_SUBMERGED_DOWNWASH_PARTICLE_CLASS
                && particle.owner_id == Some(46)
                && world_position_to_raw(particle.position)[1] == -100
        }));
        assert_eq!(fx.rng_state, expected_rng);
    }

    #[test]
    fn submerged_downwash_narrows_angular_velocity_to_retail_words() {
        let mut emission = player_surface_emission([0, 0, 0], [i16::MAX, 0, i16::MAX], 128);
        emission.up_q31 = [0; 3];
        emission.lateral_q31 = [i32::MAX, 0, i32::MAX];
        emission.forward_q31 = [i32::MAX, 0, i32::MAX];
        let mut expected_rng = 0;
        let angle = retail_random_u16(&mut expected_rng);
        let sine = retail_sine_q15(u32::from(angle));
        let cosine = retail_sine_q15(u32::from(angle.wrapping_add(0x4000)));
        let _x_jitter = retail_random_u16(&mut expected_rng);
        let _z_jitter = retail_random_u16(&mut expected_rng);
        let vertical_jitter =
            ((retail_random_u16(&mut expected_rng) >> 7) as i16).wrapping_sub(0x100);
        let wide_x = i32::from(i16::MAX)
            + ((sine * (i32::MAX >> 20)) >> 15)
            + ((sine * (i32::MAX >> 20)) >> 15);
        let wide_z = i32::from(i16::MAX)
            + ((cosine * (i32::MAX >> 20)) >> 15)
            + ((cosine * (i32::MAX >> 20)) >> 15);
        assert!(wide_x > i32::from(i16::MAX) || wide_z > i32::from(i16::MAX));
        let class_44_bias = i32::from(
            particle_descriptor(PLAYER_SUBMERGED_DOWNWASH_PARTICLE_CLASS)
                .unwrap()
                .spawn_velocity_y_bias_raw(),
        );

        let mut fx = WorldFx::new();
        fx.emit_player_surface_effect_raw(emission, 0);

        let particle = fx.test_particles_in_virgin_birth_order()[0];
        assert_eq!(
            particle.velocity.map(world_velocity_component_to_raw),
            [
                i32::from(wide_x as i16),
                i32::from(vertical_jitter) + class_44_bias,
                i32::from(wide_z as i16),
            ]
        );
        assert_eq!(fx.rng_state, expected_rng);
    }

    #[test]
    fn submerged_downwash_strength_sign_mirrors_only_body_up_y() {
        let positive = player_surface_emission([0x120, 0, -0x180], [0, 100, 0], 128);
        let negative = PlayerSurfaceEffectEmission {
            strength_raw: -128,
            ..positive
        };
        let mut positive_fx = full_rate_world_fx();
        let mut negative_fx = full_rate_world_fx();

        positive_fx.emit_player_surface_effect_raw(positive, 0);
        negative_fx.emit_player_surface_effect_raw(negative, 0);

        let positive_particle = positive_fx.test_particles_in_virgin_birth_order()[0];
        let negative_particle = negative_fx.test_particles_in_virgin_birth_order()[0];
        assert_eq!(positive_fx.rng_state, negative_fx.rng_state);
        assert_eq!(positive_particle.position, negative_particle.position);
        assert_eq!(positive_particle.velocity[0], negative_particle.velocity[0]);
        assert_eq!(positive_particle.velocity[2], negative_particle.velocity[2]);
        assert_eq!(
            world_velocity_component_to_raw(negative_particle.velocity[1])
                - world_velocity_component_to_raw(positive_particle.velocity[1]),
            2 * (i32::MAX >> 21)
        );
    }

    #[test]
    fn class_19_terrain_contact_emits_eight_material_colored_children() {
        let terrain = flat_terrain(0, 3);
        let mut ground_responses = [0; 8];
        ground_responses[3] = 2;
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: ground_responses,
            water_response_selectors: [0; 8],
        };
        let mut expected_rng = 0;
        for _ in 0..PLAYER_DOWNWASH_RESPONSE_COUNT {
            retail_random_u16(&mut expected_rng);
        }
        let mut fx = full_rate_world_fx();
        fx.emit_player_surface_effect_raw(
            player_surface_emission([0x400, 10, 0x500], [0; 3], 1000),
            -100,
        );

        fx.update(ParticleUpdateRequest::terrain(0, 0, context));

        let particles = fx.test_particles_in_virgin_birth_order();
        assert_eq!(particles.len(), PLAYER_DOWNWASH_RESPONSE_COUNT);
        assert!(particles.iter().all(|particle| {
            particle.source_class == 62
                && particle.owner_id.is_none()
                && world_position_to_raw(particle.position)[1] == 0
        }));
        assert_eq!(fx.rng_state, expected_rng);
        assert!(!particles
            .iter()
            .any(|particle| particle.source_class == PLAYER_SURFACE_PROBE_PARTICLE_CLASS));
    }

    #[test]
    fn class_19_terrain_response_preserves_the_retail_selector_matrix() {
        let terrain = flat_terrain(0, 0);
        for (selector, expected_class) in PLAYER_DOWNWASH_CLASS_BY_SELECTOR
            .into_iter()
            .enumerate()
            .filter(|(selector, _)| *selector != 6)
        {
            let mut ground_responses = [0; 8];
            ground_responses[0] = selector as u8;
            let context = TerrainCollisionContext {
                terrain: &terrain,
                terrain_objects: None,
                ground_response_selectors: ground_responses,
                water_response_selectors: [0; 8],
            };
            let mut expected_rng = 0;
            for _ in 0..PLAYER_DOWNWASH_RESPONSE_COUNT {
                retail_random_u16(&mut expected_rng);
            }
            let mut fx = full_rate_world_fx();
            fx.emit_player_surface_effect_raw(
                player_surface_emission([0x400, 10, 0x500], [0; 3], 1000),
                -100,
            );

            fx.update(ParticleUpdateRequest::terrain(0, 0, context));

            let particles = fx.test_particles_in_virgin_birth_order();
            let expected_count = if expected_class == 0 {
                0
            } else {
                PLAYER_DOWNWASH_RESPONSE_COUNT
            };
            assert_eq!(particles.len(), expected_count, "selector {selector}");
            assert!(particles
                .iter()
                .all(|particle| particle.source_class == expected_class));
            assert_eq!(fx.rng_state, expected_rng, "selector {selector}");
        }
    }

    #[test]
    fn class_19_terrain_children_inherit_parent_suppression_without_extra_rng() {
        let terrain = flat_terrain(0, 3);
        let mut ground_responses = [0; 8];
        ground_responses[3] = 2;
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: ground_responses,
            water_response_selectors: [0; 8],
        };
        let position_raw = [0x400, 10, 0x500];
        let position = raw_position_to_world(position_raw);
        let mut expected_rng = 0;
        for _ in 0..PLAYER_DOWNWASH_RESPONSE_COUNT {
            retail_random_u16(&mut expected_rng);
        }

        for expected_suppression in [false, true] {
            let mut spawn = invisible_descriptor_particle_spawn(
                PLAYER_SURFACE_PROBE_PARTICLE_CLASS,
                [0; 3],
                Some(46),
            );
            spawn.suppresses_impact_damage = expected_suppression;
            let mut parent = world_particle(position, spawn);
            parent.water_state_initialized = true;
            parent.step_start_water_state = 2;
            parent.water_state = 2;
            let mut fx = full_rate_world_fx();
            fx.particles.test_allocate(parent);

            fx.update(ParticleUpdateRequest::terrain(0, 0, context));

            let particles = fx.test_particles_in_virgin_birth_order();
            assert_eq!(particles.len(), PLAYER_DOWNWASH_RESPONSE_COUNT);
            assert!(particles
                .iter()
                .all(|particle| particle.suppresses_impact_damage == expected_suppression));
            assert_eq!(fx.rng_state, expected_rng);
        }
    }

    #[test]
    fn submerged_downwash_debris_skips_4423c0_gravity_and_bounce() {
        let spawn =
            single_descriptor_particle_burst([0.0, -8.0, 0.0], 60, [0, 256, 0], None).particles[0];
        let mut particle = world_particle([0.0, -8.0, 0.0], spawn);
        particle.water_state = 0;
        particle.step_start_water_state = 0;
        particle.water_state_initialized = true;
        let velocity_before = particle.velocity;
        let mut fx = WorldFx::new();
        fx.particles.test_allocate(particle);
        fx.last_retail_tick = Some(0);

        fx.update(ParticleUpdateRequest::flat_water(20_000, 1, 0.0));

        let after = fx.test_particles_in_virgin_birth_order()[0];
        assert_eq!(after.water_state, 0);
        assert_eq!(after.velocity, velocity_before);
        assert!(after.position[1] > particle.position[1]);
    }

    #[test]
    fn downwash_debris_clamps_to_terrain_and_bounces_one_eighth() {
        let terrain = flat_terrain(0, 0);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [0; 8],
            water_response_selectors: [0; 8],
        };
        let spawn =
            single_descriptor_particle_burst([0.0, -1.0, 0.0], 60, [0, -400, 0], None).particles[0];
        let mut particle = world_particle([0.0, -1.0, 0.0], spawn);
        particle.water_state = 2;
        particle.step_start_water_state = 2;
        particle.water_state_initialized = true;
        let post_gravity_raw = world_velocity_component_to_raw(
            particle.velocity[1] - METEOR_SCATTER_GRAVITY_WORLD_PER_SEC2 * 0.02,
        );
        let mut fx = WorldFx::new();
        fx.particles.test_allocate(particle);
        fx.last_retail_tick = Some(0);

        fx.update(ParticleUpdateRequest::terrain(20_000, 1, context));

        let after = fx.test_particles_in_virgin_birth_order()[0];
        assert_eq!(world_position_to_raw(after.position)[1], 0);
        assert_eq!(
            world_velocity_component_to_raw(after.velocity[1]),
            -(post_gravity_raw / 8)
        );
    }

    #[test]
    fn class_19_wave_entry_emits_eight_unowned_class_14_splashes() {
        let mut terrain = flat_terrain(-128, 6);
        terrain.header[0] = 0;
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [0; 8],
            water_response_selectors: [6; 8],
        };
        let x_raw = 0x400;
        let z_raw = 0x500;
        let surface_raw = wave_surface_raw(x_raw, z_raw, 0, 0, -0x1000);
        let mut fx = full_rate_world_fx();
        fx.emit_player_surface_effect_raw(
            player_surface_emission(
                [x_raw, surface_raw.wrapping_add(0x100), z_raw],
                [0, -14_000, 0],
                1000,
            ),
            0,
        );
        // Establish the allocator's initial above-water classification before
        // the next traversal carries the probe through the displaced wave.
        fx.update(ParticleUpdateRequest::terrain(0, 0, context));

        fx.update(ParticleUpdateRequest::terrain(20_000, 1, context));

        let particles = fx.test_particles_in_virgin_birth_order();
        let splashes = particles
            .iter()
            .filter(|particle| particle.source_class == PLAYER_WATER_DOWNWASH_PARTICLE_CLASS)
            .collect::<Vec<_>>();
        assert_eq!(PLAYER_WATER_DOWNWASH_PARTICLE_CLASS, 14);
        assert_eq!(splashes.len(), PLAYER_DOWNWASH_RESPONSE_COUNT);
        let first_frame = first_particle_frame(PLAYER_WATER_DOWNWASH_PARTICLE_CLASS);
        assert!(splashes.iter().all(|particle| {
            particle.owner_id.is_none()
                && particle.current_sprite_id() == first_frame.sprite_id
                && particle.lifetime_ticks
                    == particle_descriptor(PLAYER_WATER_DOWNWASH_PARTICLE_CLASS)
                        .unwrap()
                        .lifetime_ticks()
        }));
        assert!(!particles
            .iter()
            .any(|particle| particle.source_class == PLAYER_SURFACE_PROBE_PARTICLE_CLASS));
    }

    #[test]
    fn shared_water_spray_dry_attempt_consumes_only_xz_jitter_words() {
        let seed = 0;
        let mut expected_rng = seed;
        let x_offset = ((retail_random_u16(&mut expected_rng) >> 7) as i16).wrapping_sub(0x100);
        let z_offset = ((retail_random_u16(&mut expected_rng) >> 7) as i16).wrapping_sub(0x100);
        let child_raw = [0x04c0_i16, 0, 0x05c0_i16];
        let parent_raw = [
            child_raw[0].wrapping_sub(x_offset),
            0,
            child_raw[2].wrapping_sub(z_offset),
        ];

        // E8A0/E4F0 feed the coarse low corner into the wave function but
        // compare the result against a separate bilinear height. Raising the
        // other corners makes this precise jitter point dry.
        let mut terrain = flat_terrain(127, 6);
        terrain.header[0] = 0;
        terrain_cell_mut(&mut terrain, 4, 5).height = (-128_i8) as u8;
        let coarse_raw = coarse_terrain_height_raw(&terrain, child_raw[0], child_raw[2]);
        let bilinear_raw = bilinear_terrain_height_raw(&terrain, child_raw[0], child_raw[2]);
        let wave_raw = wave_surface_raw(
            child_raw[0],
            child_raw[2],
            0,
            terrain.sea_level_raw(),
            coarse_raw,
        );
        assert!(bilinear_raw > wave_raw);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [0; 8],
            water_response_selectors: [6; 8],
        };
        let mut fx = WorldFx::new();
        fx.rng_state = seed;

        fx.emit_jittered_water_surface_spray(
            raw_position_to_world(parent_raw),
            context,
            ParticleBirthContext {
                environment: ParticleEnvironment::Terrain(context),
                retail_tick: 0,
            },
            1,
            false,
        );

        assert_eq!(fx.rng_state, expected_rng);
        assert_eq!(fx.particle_count(), 0);
        assert!(fx.take_positional_sounds().is_empty());
    }

    #[test]
    fn shared_water_spray_wet_rng_and_audio_survive_allocator_rejection() {
        let mut terrain = flat_terrain(-128, 6);
        terrain.header[0] = 0;
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [0; 8],
            water_response_selectors: [6; 8],
        };
        let parent_position = [4.0, 0.0, 5.0];
        let seed = (0..u32::MAX)
            .find(|seed| {
                let mut state = *seed;
                let mut gated = false;
                for _ in 0..BALLISTIC_WATER_RESPONSE_COUNT {
                    retail_random_u16(&mut state);
                    retail_random_u16(&mut state);
                    if retail_random_u16(&mut state) & 0x1f == 0 {
                        gated = true;
                        retail_random_u16(&mut state);
                        retail_random_u16(&mut state);
                    }
                }
                gated
            })
            .expect("small seed with a selector-six sound gate");
        let mut expected_rng = seed;
        let mut expected_sounds = Vec::new();
        for _ in 0..BALLISTIC_WATER_RESPONSE_COUNT {
            retail_random_u16(&mut expected_rng);
            retail_random_u16(&mut expected_rng);
            if retail_random_u16(&mut expected_rng) & 0x1f == 0 {
                let sound_roll = retail_random_u16(&mut expected_rng);
                let sound_id = if sound_roll & 0x1000 != 0 {
                    SPECIAL_SURFACE_SOUND_IDS[0]
                } else {
                    SPECIAL_SURFACE_SOUND_IDS[1]
                };
                let rate = SPECIAL_SURFACE_RATE_MIN_16_16
                    + u32::from(retail_random_u16(&mut expected_rng) >> 2);
                expected_sounds.push(PositionalSoundEvent {
                    sound_id,
                    position: parent_position,
                    frequency_q16: rate,
                });
            }
        }
        assert!(!expected_sounds.is_empty());

        for saturated in [false, true] {
            let mut fx = full_rate_world_fx();
            if saturated {
                fx.particles.test_fill_to(
                    MAX_WORLD_PARTICLES,
                    descriptor_test_particle(METEOR_SCATTER_CLASS, [0.0; 3], [0; 3], None),
                );
            }
            fx.rng_state = seed;
            fx.emit_jittered_water_surface_spray(
                parent_position,
                context,
                ParticleBirthContext {
                    environment: ParticleEnvironment::Terrain(context),
                    retail_tick: 0,
                },
                BALLISTIC_WATER_RESPONSE_COUNT,
                false,
            );

            assert_eq!(fx.rng_state, expected_rng, "saturated={saturated}");
            assert_eq!(
                fx.particles
                    .slots
                    .iter()
                    .flatten()
                    .filter(|particle| {
                        particle.source_class == PLAYER_WATER_DOWNWASH_PARTICLE_CLASS
                    })
                    .count(),
                if saturated {
                    0
                } else {
                    BALLISTIC_WATER_RESPONSE_COUNT
                },
                "saturated={saturated}"
            );
            assert_eq!(
                fx.take_positional_sounds(),
                expected_sounds,
                "saturated={saturated}"
            );
            fx.particles.assert_valid_topology();
        }
    }

    #[test]
    fn alien_hive_uses_gravity_callback_without_mode_one_collision() {
        assert!(uses_gravity_update_callback(ALIEN_HIVE_PARTICLE_CLASS));
        assert!(!uses_mode_one_surface_callback(ALIEN_HIVE_PARTICLE_CLASS));

        let mut fx = WorldFx::new();
        fx.emit_alien_hive_particle_raw([0, 1024, 0], 7);
        let before = fx.test_particles_in_virgin_birth_order()[0];
        fx.last_retail_tick = Some(0);

        fx.update(ParticleUpdateRequest::dry(20_000, 1));

        let after = fx.test_particles_in_virgin_birth_order()[0];
        assert!((after.velocity[0] - before.velocity[0]).abs() < f32::EPSILON);
        assert!((after.velocity[2] - before.velocity[2]).abs() < f32::EPSILON);
        assert!(
            (after.velocity[1]
                - (before.velocity[1] - METEOR_SCATTER_GRAVITY_WORLD_PER_SEC2 * 0.02))
                .abs()
                < 1e-6
        );
    }

    #[test]
    fn terrain_contact_modes_come_from_the_two_exact_descriptor_callbacks() {
        let classes = (0..PARTICLE_DESCRIPTORS.len() as u8)
            .filter_map(|source_class| {
                terrain_contact_mode_for_particle(source_class)
                    .map(|terrain_contact_mode| (source_class, terrain_contact_mode))
            })
            .collect::<Vec<_>>();
        assert_eq!(
            classes,
            vec![
                (ALIEN_HIVE_PARTICLE_CLASS, TerrainContactMode::Infect),
                (6, TerrainContactMode::Cleanse),
            ]
        );
    }

    #[test]
    fn alien_hive_selector_six_damps_signed_raw_velocity_and_keeps_particle() {
        let terrain = flat_terrain(0, 0);
        let mut particle = pool_test_particle(ALIEN_HIVE_PARTICLE_CLASS, 0.0);
        particle.velocity = raw_velocity_to_world([-7, -5, 7]);
        let position_before = particle.position;
        let mut mutations = Vec::new();

        assert!(WorldFx::apply_terrain_contact_surface_response(
            &mut particle,
            TerrainContactMode::Infect,
            6,
            &terrain,
            &mut mutations,
        ));

        assert_eq!(
            particle.velocity.map(world_velocity_component_to_raw),
            [-1, -2, 1]
        );
        assert_eq!(particle.position, position_before);
        assert!(mutations.is_empty());
    }

    #[test]
    fn alien_hive_ground_contact_deletes_and_returns_one_infection_write() {
        let terrain = flat_terrain(0, 0);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [0; 8],
            water_response_selectors: [0; 8],
        };
        let mut fx = WorldFx::new();
        fx.emit_alien_hive_particle_raw([-128, 0, i16::MIN], 7);
        fx.emit_alien_hive_particle_raw([-128, 0, i16::MIN], 7);

        let outcome = fx.update(ParticleUpdateRequest::terrain(0, 0, context));

        assert_eq!(fx.particle_count(), 0);
        assert_eq!(
            outcome.terrain_type_mutations,
            vec![ParticleTerrainMutation::Infection {
                cell: [255, 128],
                infected: true,
            }]
        );

        let infected_terrain = flat_terrain(0, 0x10);
        let infected_context = TerrainCollisionContext {
            terrain: &infected_terrain,
            terrain_objects: None,
            ground_response_selectors: [0; 8],
            water_response_selectors: [0; 8],
        };
        let mut infected_fx = WorldFx::new();
        infected_fx.emit_alien_hive_particle_raw([-128, 0, i16::MIN], 7);
        let infected_outcome =
            infected_fx.update(ParticleUpdateRequest::terrain(0, 0, infected_context));
        assert_eq!(infected_fx.particle_count(), 0);
        assert!(infected_outcome.terrain_type_mutations.is_empty());
    }

    #[test]
    fn cleansing_landscape_ground_contact_deletes_and_returns_one_clear_write() {
        let terrain = flat_terrain(0, INFECTION_TERRAIN_TYPE_BIT);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [0; 8],
            water_response_selectors: [0; 8],
        };
        let mut fx = WorldFx::new();
        fx.emit_terrain_contact_particle_raw(DefecateVirusParticleEmission::new(
            [0; 3],
            0x04b3_0003,
            false,
            TerrainContactMode::Cleanse,
        ));

        let outcome = fx.update(ParticleUpdateRequest::terrain(0, 0, context));

        assert_eq!(fx.particle_count(), 0);
        assert_eq!(
            outcome.terrain_type_mutations,
            vec![ParticleTerrainMutation::Infection {
                cell: [0, 0],
                infected: false,
            }]
        );
    }

    #[test]
    fn opposite_same_cell_contacts_preserve_descriptor_and_physical_pool_order() {
        let clear_terrain = flat_terrain(0, 0);
        let infected_terrain = flat_terrain(0, INFECTION_TERRAIN_TYPE_BIT);
        let clear_context = TerrainCollisionContext {
            terrain: &clear_terrain,
            terrain_objects: None,
            ground_response_selectors: [0; 8],
            water_response_selectors: [0; 8],
        };
        let infected_context = TerrainCollisionContext {
            terrain: &infected_terrain,
            terrain_objects: None,
            ground_response_selectors: [0; 8],
            water_response_selectors: [0; 8],
        };

        // Virgin allocation descends through physical slots. Allocate class 6
        // first so class 5 occupies the earlier slot and sets before class 6
        // clears during the ascending retail traversal.
        let mut set_then_clear_fx = WorldFx::new();
        set_then_clear_fx
            .particles
            .test_allocate(descriptor_test_particle(
                TerrainContactMode::Cleanse.particle_class(),
                [0.0; 3],
                [0; 3],
                None,
            ));
        set_then_clear_fx
            .particles
            .test_allocate(descriptor_test_particle(
                TerrainContactMode::Infect.particle_class(),
                [0.0; 3],
                [0; 3],
                None,
            ));
        let set_then_clear = set_then_clear_fx
            .update(ParticleUpdateRequest::terrain(0, 0, clear_context))
            .terrain_type_mutations;
        assert_eq!(
            set_then_clear,
            [
                ParticleTerrainMutation::Infection {
                    cell: [0, 0],
                    infected: true,
                },
                ParticleTerrainMutation::Infection {
                    cell: [0, 0],
                    infected: false,
                },
            ]
        );
        assert_eq!(
            effective_terrain_type(0, [0, 0], &set_then_clear) & INFECTION_TERRAIN_TYPE_BIT,
            0
        );

        let mut clear_then_set_fx = WorldFx::new();
        clear_then_set_fx
            .particles
            .test_allocate(descriptor_test_particle(
                TerrainContactMode::Infect.particle_class(),
                [0.0; 3],
                [0; 3],
                None,
            ));
        clear_then_set_fx
            .particles
            .test_allocate(descriptor_test_particle(
                TerrainContactMode::Cleanse.particle_class(),
                [0.0; 3],
                [0; 3],
                None,
            ));
        let clear_then_set = clear_then_set_fx
            .update(ParticleUpdateRequest::terrain(0, 0, infected_context))
            .terrain_type_mutations;
        assert_eq!(
            clear_then_set,
            [
                ParticleTerrainMutation::Infection {
                    cell: [0, 0],
                    infected: false,
                },
                ParticleTerrainMutation::Infection {
                    cell: [0, 0],
                    infected: true,
                },
            ]
        );
        assert_ne!(
            effective_terrain_type(INFECTION_TERRAIN_TYPE_BIT, [0, 0], &clear_then_set,)
                & INFECTION_TERRAIN_TYPE_BIT,
            0
        );
    }

    #[test]
    #[should_panic(expected = "class-4 Defecate Virus emission cannot carry cleansing mode")]
    fn named_defecate_emitter_rejects_cleansing_mode_in_all_builds() {
        WorldFx::new().emit_defecate_virus_particle_raw(DefecateVirusParticleEmission::new(
            [0; 3],
            1,
            false,
            TerrainContactMode::Cleanse,
        ));
    }

    #[test]
    fn defecate_virus_owner_sign_deletes_carrier_without_infecting_terrain() {
        let terrain = flat_terrain(0, 0);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [0; 8],
            water_response_selectors: [0; 8],
        };
        let mut fx = WorldFx::new();
        fx.emit_defecate_virus_particle_raw(DefecateVirusParticleEmission::new(
            [0; 3],
            0x04fc_0001,
            true,
            TerrainContactMode::Infect,
        ));

        let outcome = fx.update(ParticleUpdateRequest::terrain(0, 0, context));

        assert_eq!(fx.particle_count(), 0);
        assert!(outcome.terrain_type_mutations.is_empty());
    }

    #[test]
    fn alien_hive_non_six_water_response_short_circuits_ground_response() {
        let mut terrain = flat_terrain(0, 0);
        terrain.header[0] = 0x100;
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [6; 8],
            water_response_selectors: [0; 8],
        };
        let x = 4.75;
        let z = 5.75;
        let water_y = displaced_water_surface(context, [x, 0.0, z], 0).unwrap();
        let mut particle = pool_test_particle(ALIEN_HIVE_PARTICLE_CLASS, 0.0);
        particle.position = [x, water_y - 0.01, z];
        particle.step_start_water_state = 2;
        particle.water_state = 1;
        particle.water_state_initialized = true;
        particle.velocity = raw_velocity_to_world([8, -10, -12]);
        let velocity_before = particle.velocity;
        let mut mutations = Vec::new();

        assert!(!WorldFx::dispatch_terrain_contact_surface_collision(
            &mut particle,
            TerrainContactMode::Infect,
            context,
            0,
            &mut mutations,
        ));

        assert_eq!(particle.velocity, velocity_before);
        assert_eq!(
            mutations,
            vec![ParticleTerrainMutation::Infection {
                cell: [4, 5],
                infected: true,
            }]
        );
    }

    #[test]
    fn fixed_raw_sound_preserves_signed_y_and_toroidal_word_coordinates() {
        let position_raw = [-128, -4_096, i16::MIN];
        let mut fx = WorldFx::new();
        fx.queue_fixed_positional_sound_raw(68, position_raw);
        assert_eq!(fx.pending_event_count(), 1);

        fx.process_pending();
        assert_eq!(
            fx.take_positional_sounds(),
            vec![PositionalSoundEvent::fixed(68, [255.5, -16.0, 128.0])]
        );
    }

    #[test]
    fn raw_sound_preserves_fixed_playback_rate() {
        let mut fx = WorldFx::new();
        fx.queue_fixed_positional_sound_raw_at_rate(0x32, [256, -512, 768], 0xAAAA);

        fx.process_pending();
        assert_eq!(
            fx.take_positional_sounds(),
            vec![PositionalSoundEvent {
                sound_id: 0x32,
                position: [1.0, -2.0, 3.0],
                frequency_q16: 0xAAAA,
            }]
        );
    }

    #[test]
    fn synchronous_fixed_death_cue_precedes_baf0_sound_without_extra_rng() {
        let position = [100, 10_000, -300];
        let seed = 0x1234_5678;
        let mut oracle = seed;
        let expected_random_rate =
            METEOR_IMPACT_RATE_MIN_16_16 + u32::from(retail_random_u16(&mut oracle) >> 3);
        let mut fx = full_rate_world_fx();
        fx.rng_state = seed;

        // 10C10 submits a nonzero header+90 cue before entering BD20/BAF0.
        // Type83's recovered cue is62, also used independently by440950.
        fx.emit_fixed_positional_sound_raw(62, position);
        assert_eq!(fx.rng_state, seed);
        assert_eq!(fx.particle_count(), 0);
        fx.emit_explode_with_ring_burst_raw(ExplodeWithRingBurstRequest {
            position_raw: position,
            source_extent_raw: 0,
            sea_level_raw: None,
            logical_owner_entity_id: 7,
            logical_owner_entity_type: 83,
            scatter_count: 10,
            scatter_classes: [16, 16],
            suppresses_impact_damage: false,
        });
        assert_eq!(fx.particle_count(), 11);
        assert_eq!(fx.rng_state, oracle);
        // The normal pending flush must not reverse or duplicate these cues.
        fx.process_pending();
        assert_eq!(fx.rng_state, oracle);
        assert_eq!(
            fx.take_positional_sounds(),
            vec![
                PositionalSoundEvent::fixed(62, raw_position_to_world(position)),
                PositionalSoundEvent {
                    sound_id: 62,
                    position: raw_position_to_world(position),
                    frequency_q16: expected_random_rate,
                },
            ]
        );
        fx.process_pending();
        assert!(fx.take_positional_sounds().is_empty());
    }

    #[test]
    fn disposable_sound_gc_preserves_pending_particles_order_and_rng() {
        let mut fx = WorldFx::new();
        fx.rng_state = 0x3141_5926;

        fx.queue_fixed_positional_sound_raw(1, [0, 0, 0]);
        fx.process_pending();
        fx.queue_static_effect79_raw([256, 512, 768]);
        fx.queue_fixed_positional_sound_raw(2, [256, 512, 768]);
        fx.queue_static_effect79_raw([1024, 1280, 1536]);
        assert_eq!(fx.ready_sounds.len(), 1);
        assert_eq!(fx.pending_event_count(), 3);

        fx.garbage_collect_disposable_positional_sounds();

        assert!(fx.ready_sounds.is_empty());
        assert_eq!(fx.pending_event_count(), 2);
        assert!(matches!(
            fx.pending.front(),
            Some(WorldEvent::ParticleBurst(_))
        ));
        assert_eq!(fx.rng_state, 0x3141_5926);

        fx.process_pending();
        assert_eq!(
            fx.particles
                .test_iter_in_virgin_birth_order()
                .map(|particle| (particle.source_class, particle.position))
                .collect::<Vec<_>>(),
            [
                (STATIC_DESTRUCTION_EFFECT_CLASS_79, [1.0, 2.0, 3.0]),
                (STATIC_DESTRUCTION_EFFECT_CLASS_79, [4.0, 5.0, 6.0]),
            ]
        );
        assert!(fx.take_positional_sounds().is_empty());
    }

    #[test]
    fn opcode_four_effect_keeps_class_eighteen_strictly_above_water() {
        let position_raw = [-128, 513, i16::MIN];
        let descriptor = particle_descriptor(METEOR_SURFACE_CLASS).unwrap();
        let mut fx = WorldFx::new();
        fx.queue_static_effect18_raw(position_raw, Some(512));
        fx.process_pending();

        assert_eq!(fx.particles.len(), 1);
        let particle = fx.particles[0];
        assert_eq!(particle.source_class, METEOR_SURFACE_CLASS);
        assert_eq!(particle.position, [255.5, 513.0 / 256.0, 128.0]);
        assert_eq!(
            particle.velocity,
            raw_velocity_to_world([0, i32::from(descriptor.spawn_velocity_y_bias_raw()), 0])
        );
    }

    #[test]
    fn opcode_four_effect_converts_equal_or_below_water_to_45_then_46() {
        let position_raw = [32, -100, -1];
        let mut fx = WorldFx::new();
        fx.queue_static_effect18_raw(position_raw, Some(-100));
        assert_eq!(fx.pending_event_count(), 2);
        fx.process_pending();

        assert_eq!(
            fx.particles
                .test_iter_in_virgin_birth_order()
                .map(|particle| particle.source_class)
                .collect::<Vec<_>>(),
            UNDERWATER_IMPACT_CLASSES
        );
        assert_eq!(
            fx.particles[0].position,
            [32.0 / 256.0, -100.0 / 256.0, 65_535.0 / 256.0]
        );
        assert_eq!(
            fx.particles[1].position,
            [65_504.0 / 256.0, -100.0 / 256.0, 65_535.0 / 256.0]
        );
        for (particle, source_class) in fx
            .particles
            .test_iter_in_virgin_birth_order()
            .zip(UNDERWATER_IMPACT_CLASSES)
        {
            let descriptor = particle_descriptor(source_class).unwrap();
            assert_eq!(
                particle.velocity,
                raw_velocity_to_world([0, i32::from(descriptor.spawn_velocity_y_bias_raw()), 0])
            );
        }
    }

    #[test]
    fn kind_29_ordinary_class79_effect_preserves_zero_input_velocity() {
        let position_raw = [-128, -100, i16::MIN];
        let descriptor =
            particle_descriptor(STATIC_DESTRUCTION_EFFECT_CLASS_79).expect("class 79 descriptor");
        let mut fx = WorldFx::new();

        fx.queue_static_effect79_raw(position_raw);

        assert_eq!(fx.pending_event_count(), 1);
        fx.process_pending();
        let particles = fx.test_particles_in_virgin_birth_order();
        assert_eq!(particles.len(), 1);
        assert_eq!(
            particles[0].source_class,
            STATIC_DESTRUCTION_EFFECT_CLASS_79
        );
        assert_eq!(world_position_to_raw(particles[0].position), position_raw);
        assert_eq!(
            particles[0].velocity.map(world_velocity_component_to_raw),
            [0, i32::from(descriptor.spawn_velocity_y_bias_raw()), 0]
        );
        assert_eq!(particles[0].owner_id, Some(0));
        assert_eq!(particles[0].source_entity_type_at_birth, Some(0));
        assert_eq!(
            static_ballistic_entity_hit_delivery(&particles[0]),
            DamageDeliveryRecord {
                packet: BALLISTIC_PARTICLE_DAMAGE_PACKET,
                source_entity_type_raw: 0,
                owner_handle: 0,
            },
            "static owner zero must survive the entity-hit packet boundary"
        );
    }

    #[test]
    fn static_effect18_uses_authored_sea_plane_when_water_rendering_is_disabled() {
        let terrain = v2k_formats::terrain::TerrainGrid {
            header: [-1_048_576, 0, 0, 0, 0],
            cells: Vec::new(),
        };
        assert!(!terrain.water_enabled());
        assert_eq!(terrain.sea_level_raw(), -4_096);
        for (height, expected) in [
            (-4_095, vec![METEOR_SURFACE_CLASS]),
            (-4_096, UNDERWATER_IMPACT_CLASSES.to_vec()),
            (-4_100, UNDERWATER_IMPACT_CLASSES.to_vec()),
        ] {
            let mut fx = WorldFx::new();
            fx.emit_static_effect18_raw([128, height, 128], Some(terrain.sea_level_raw()));
            assert_eq!(fx.pending_event_count(), 0);
            assert_eq!(
                fx.test_particles_in_virgin_birth_order()
                    .iter()
                    .map(|particle| particle.source_class)
                    .collect::<Vec<_>>(),
                expected,
            );
        }
    }

    #[test]
    fn static_fifo_effect_allocation_preserves_source_order_and_native_priority() {
        let mut fx = WorldFx::new();
        for _ in 0..8 {
            fx.advance_frame_pacing(20_000);
        }
        for _ in 0..199 {
            fx.emit_static_effect79_raw([100, 200, 300]);
        }
        let first = [100, 200, 300];
        let later = [400, 500, 600];
        // A fresh equal-priority particle cannot be displaced. Deferring the
        // first allocation gives the last free slot to the later callback.
        let mut before = fx.fork_for_main_base_abort_transaction();
        before.queue_static_effect18_raw(first, None);
        before.emit_static_effect18_raw(later, None);
        before.process_pending();
        assert_eq!(before.particle_count(), MAX_WORLD_PARTICLES);
        assert_eq!(
            before
                .test_particles_in_virgin_birth_order()
                .iter()
                .find(|particle| particle.source_class == 18)
                .unwrap()
                .position,
            raw_position_to_world(later)
        );

        // 281A0 -> 4410B0 owns this allocation before the next opcode.
        fx.emit_static_effect18_raw(first, None);
        fx.emit_static_effect18_raw(later, None);
        assert_eq!(fx.pending_event_count(), 0);
        let particles = fx.test_particles_in_virgin_birth_order();
        assert_eq!(particles.len(), MAX_WORLD_PARTICLES);
        assert_eq!(
            particles
                .iter()
                .find(|particle| particle.source_class == 18)
                .unwrap()
                .position,
            raw_position_to_world(first)
        );
        fx.process_pending();
        assert_eq!(
            fx.test_particles_in_virgin_birth_order()
                .iter()
                .find(|particle| particle.source_class == 18)
                .unwrap()
                .position,
            raw_position_to_world(first)
        );
        // Native priorities still admit a later class79 over fresh class18.
        fx.emit_static_effect79_raw(later);
        assert!(!fx
            .test_particles_in_virgin_birth_order()
            .iter()
            .any(|particle| particle.source_class == 18));
    }

    #[test]
    fn static_object_above_water_runs_both_exact_rng_gates_per_traversal() {
        let source = static_emitter_source([188.5, -2.625, 134.5], 0x0a, 0);
        let seed = seed_for_above_static_pair();
        let mut expected_state = seed;
        assert_eq!(retail_random_u16(&mut expected_state) & 7, 0);
        assert_eq!(retail_random_u16(&mut expected_state) & 0x0f, 0);
        let height_jitter = retail_random_u16(&mut expected_state) & 0x7f;

        let mut fx = WorldFx::new();
        fx.rng_state = seed;
        fx.emit_static_terrain_object_particles([&source], Some(-3.308_593_8));
        assert_eq!(fx.pending_event_count(), 0);
        assert_eq!(fx.rng_state, expected_state);

        assert_eq!(fx.particles.len(), 2);
        assert_eq!(fx.particles[0].source_class, STATIC_PUFF_ABOVE_WATER_CLASS);
        assert_eq!(fx.particles[0].position, [188.5, -1.843_75, 134.5]);
        assert_eq!(
            fx.particles[0].velocity,
            raw_velocity_to_world(STATIC_PUFF_ABOVE_WATER_VELOCITY_ARGUMENT_RAW)
        );
        assert_eq!(fx.particles[1].source_class, STATIC_FLAME_ABOVE_WATER_CLASS);
        assert_eq!(
            fx.particles[1].position,
            [
                188.5,
                -2.625 + f32::from(STATIC_FLAME_Y_OFFSET_MIN_RAW + height_jitter as i16) / 256.0,
                134.5,
            ]
        );

        // Retail can traverse this source twice under one clock value. Both
        // visits independently consume RNG and may allocate a particle.
        fx.rng_state = seed;
        fx.emit_static_terrain_object_particles([&source], Some(-3.308_593_8));
        assert_eq!(fx.particles.len(), 4);
        assert_eq!(fx.rng_state, expected_state);
    }

    #[test]
    fn static_object_underwater_uses_puff_x_and_flame_y_jitter_words() {
        let source = static_emitter_source([12.5, -2.0, 4.5], 0x89, 0);
        let seed = seed_for_underwater_static_pair();
        let mut expected_state = seed;
        assert_eq!(retail_random_u16(&mut expected_state) & 7, 0);
        let x_jitter = i32::from((retail_random_u16(&mut expected_state) >> 9) & 0x7f) - 0x40;
        assert_eq!(retail_random_u16(&mut expected_state) & 0x0f, 0);
        let y_jitter = retail_random_u16(&mut expected_state) & 0x7f;

        let mut fx = WorldFx::new();
        fx.rng_state = seed;
        fx.emit_static_terrain_object_particles([&source], Some(100.0));

        assert_eq!(fx.rng_state, expected_state);
        assert_eq!(fx.particles.len(), 2);
        assert_eq!(fx.particles[0].source_class, STATIC_PUFF_UNDERWATER_CLASS);
        assert_eq!(
            fx.particles[0].position,
            [
                12.5 + x_jitter as f32 / 256.0,
                -2.0 + f32::from(STATIC_PUFF_Y_OFFSET_RAW) / 256.0,
                4.5,
            ]
        );
        assert_eq!(fx.particles[0].velocity, raw_velocity_to_world([0, 800, 0]));
        assert_eq!(fx.particles[1].source_class, STATIC_FLAME_UNDERWATER_CLASS);
        assert_eq!(
            fx.particles[1].position[1],
            -2.0 + f32::from(STATIC_FLAME_Y_OFFSET_MIN_RAW + y_jitter as i16) / 256.0
        );
        assert_eq!(fx.particles[1].velocity, raw_velocity_to_world([0, 700, 0]));
    }

    #[test]
    fn player_low_hull_plume_uses_body_forward_and_two_rng_words_above_water() {
        let position_raw = [i16::MAX - 5, 100, -100];
        let body_forward_q31 = [0x4000_0000, 0, -0x4000_0000];
        let seed = 0x1234_5678;
        let mut expected_state = seed;
        let x_jitter = ((retail_random_u16(&mut expected_state) >> 9) as i16) - 0x40;
        let z_jitter = ((retail_random_u16(&mut expected_state) >> 9) as i16) - 0x40;
        let expected_position_raw = [
            position_raw[0]
                .wrapping_add(x_jitter)
                .wrapping_sub(q31_mul_raw(
                    body_forward_q31[0],
                    PLAYER_LOW_HULL_SMOKE_REAR_DISTANCE_RAW,
                ) as i16),
            position_raw[1].wrapping_add(PLAYER_LOW_HULL_SMOKE_Y_OFFSET_RAW),
            position_raw[2]
                .wrapping_add(z_jitter)
                .wrapping_sub(q31_mul_raw(
                    body_forward_q31[2],
                    PLAYER_LOW_HULL_SMOKE_REAR_DISTANCE_RAW,
                ) as i16),
        ];

        let mut fx = WorldFx::new();
        fx.rng_state = seed;
        fx.emit_player_low_hull_smoke_raw(
            position_raw,
            body_forward_q31,
            46,
            Some(expected_position_raw[1] - 1),
        );

        assert_eq!(fx.rng_state, expected_state);
        assert_eq!(fx.particles.len(), 1);
        let particle = fx.particles[0];
        assert_eq!(particle.source_class, STATIC_PUFF_ABOVE_WATER_CLASS);
        assert_eq!(particle.current_sprite_id(), 0x37f);
        assert_eq!(
            particle.position,
            raw_position_to_world(expected_position_raw)
        );
        assert_eq!(particle.velocity, raw_velocity_to_world([0, 0x50, 0]));
        assert_eq!(particle.owner_id, Some(46));
        assert_eq!(particle.lifetime_ticks, 0xaa);
    }

    #[test]
    fn player_low_hull_plume_uses_third_rng_word_and_bubble_at_sea_plane() {
        let position_raw = [100, -PLAYER_LOW_HULL_SMOKE_Y_OFFSET_RAW, 200];
        let seed = 0x8765_4321;
        let mut expected_state = seed;
        let first_x_jitter = ((retail_random_u16(&mut expected_state) >> 9) as i16) - 0x40;
        let z_jitter = ((retail_random_u16(&mut expected_state) >> 9) as i16) - 0x40;
        let underwater_x_jitter = ((retail_random_u16(&mut expected_state) >> 9) as i16) - 0x40;
        let expected_position_raw = [
            position_raw[0]
                .wrapping_add(first_x_jitter)
                .wrapping_add(underwater_x_jitter),
            0,
            position_raw[2].wrapping_add(z_jitter),
        ];

        let mut fx = WorldFx::new();
        fx.rng_state = seed;
        fx.emit_player_low_hull_smoke_raw(position_raw, [0; 3], 99, Some(0));

        assert_eq!(fx.rng_state, expected_state);
        assert_eq!(fx.particles.len(), 1);
        let particle = fx.particles[0];
        assert_eq!(particle.source_class, STATIC_PUFF_UNDERWATER_CLASS);
        assert_eq!(particle.current_sprite_id(), 0x313);
        assert_eq!(
            particle.position,
            raw_position_to_world(expected_position_raw)
        );
        // The allocator adds class 42's authored +0x190 Y bias to the
        // callback's own +0x190 input velocity.
        assert_eq!(particle.velocity, raw_velocity_to_world([0, 0x320, 0]));
        assert_eq!(particle.owner_id, Some(99));
        assert_eq!(particle.lifetime_ticks, 0x40);
    }

    #[test]
    fn player_low_hull_plume_has_no_tick_latch_and_full_pool_still_consumes_rng() {
        let mut fx = WorldFx::new();
        fx.rng_state = 0x3141_5926;
        let mut expected_state = fx.rng_state;
        for _ in 0..4 {
            retail_random_u16(&mut expected_state);
        }

        fx.emit_player_low_hull_smoke_raw([0, 0, 0], [0; 3], 1, None);
        fx.emit_player_low_hull_smoke_raw([0, 0, 0], [0; 3], 1, None);
        assert_eq!(fx.particles.len(), 2);
        assert_eq!(fx.rng_state, expected_state);

        let filler = fx.particles[0];
        fx.particles.test_fill_to(MAX_WORLD_PARTICLES, filler);
        retail_random_u16(&mut expected_state);
        retail_random_u16(&mut expected_state);
        fx.emit_player_low_hull_smoke_raw([0, 0, 0], [0; 3], 1, None);

        assert_eq!(fx.particles.len(), MAX_WORLD_PARTICLES);
        assert_eq!(fx.rng_state, expected_state);
    }

    #[test]
    fn static_object_emitter_excludes_unflagged_and_kind_nine_cells_without_rng() {
        let unflagged = static_emitter_source([0.5, 0.0, 0.5], 0x01, 0);
        let fence = static_emitter_source([1.5, 0.0, 0.5], 0x89, 9);
        let mut fx = WorldFx::new();
        fx.rng_state = 0x1234_5678;
        fx.emit_static_terrain_object_particles([&unflagged, &fence], None);

        assert_eq!(fx.pending_event_count(), 0);
        assert_eq!(fx.rng_state, 0x1234_5678);
    }

    #[test]
    fn static_object_emission_does_not_drain_unrelated_pending_events() {
        let source = static_emitter_source([188.5, -2.625, 134.5], 0x0a, 0);
        let mut fx = WorldFx::new();
        fx.rng_state = seed_for_above_static_pair();
        fx.queue(WorldEvent::PositionalSound(PositionalSoundEvent::fixed(
            CARGO_TRANSFER_SOUND_ID,
            [1.0, 2.0, 3.0],
        )));

        fx.emit_static_terrain_object_particles([&source], Some(-3.308_593_8));

        assert_eq!(fx.particles.len(), 2);
        assert_eq!(fx.pending_event_count(), 1);
        assert!(fx.ready_sounds.is_empty());
    }

    #[test]
    fn static_above_water_classes_share_retail_velocity_jitter_callback() {
        let mut fx = WorldFx::new();
        fx.queue(WorldEvent::ParticleBurst(single_descriptor_particle_burst(
            [0.0; 3],
            STATIC_PUFF_ABOVE_WATER_CLASS,
            STATIC_PUFF_ABOVE_WATER_VELOCITY_ARGUMENT_RAW,
            None,
        )));
        fx.queue(WorldEvent::ParticleBurst(single_descriptor_particle_burst(
            [0.0; 3],
            STATIC_FLAME_ABOVE_WATER_CLASS,
            STATIC_FLAME_VELOCITY_ARGUMENT_RAW,
            None,
        )));
        fx.process_pending();
        fx.rng_state = 0x1234;
        let mut expected_state = fx.rng_state;
        for _ in 0..6 {
            retail_random_u16(&mut expected_state);
        }

        fx.update(ParticleUpdateRequest::dry(0, 0));
        assert_eq!(fx.rng_state, expected_state);
    }

    #[test]
    fn primary_fire_materializes_one_bullet_above_water_smoke_and_sound() {
        let mut fx = WorldFx::new();
        fx.queue_primary_fire_batch(&[primary_event(-4.0, [1.0, 2.0, 3.0])], Some(-4.410_156_2));
        assert_eq!(fx.pending_event_count(), 3);
        fx.process_pending();

        assert_eq!(fx.test_particles_in_virgin_birth_order().len(), 2);
        let bullet = fx.test_particles_in_virgin_birth_order()[0];
        assert_eq!(bullet.source_class, PRIMARY_BULLET_PARTICLE_CLASS);
        assert_eq!(bullet.current_sprite_id(), 705);
        assert_eq!(bullet.lifetime_ticks, PRIMARY_BULLET_LIFETIME_TICKS);
        assert_eq!(bullet.collision_radius_raw, PRIMARY_BULLET_RADIUS_RAW);
        assert_eq!(bullet.position, [4.0, -4.0, 5.0]);
        assert_eq!(bullet.presentation_position(), [4.5, -3.75, 5.75]);
        assert_eq!(bullet.source_entity_type_at_birth, Some(46));
        assert_eq!(
            bullet.velocity,
            [
                PRIMARY_PROJECTILE_SPEC.speed_world_per_second + 1.0,
                2.0,
                3.0,
            ]
        );

        let mut muzzle = fx.test_particles_in_virgin_birth_order()[1];
        assert_eq!(muzzle.source_class, PRIMARY_MUZZLE_PARTICLE_CLASS);
        assert_eq!(muzzle.current_sprite_id(), 843);
        assert_eq!(muzzle.position, [4.0, -4.0 + 35.0 / 256.0, 5.0]);
        assert_eq!(
            muzzle.presentation_position(),
            [4.5, -3.75 + 35.0 / 256.0, 5.75]
        );
        assert_eq!(muzzle.owner_id, Some(7));
        assert_eq!(muzzle.source_entity_type_at_birth, Some(46));
        muzzle.age_ticks = 2.0;
        assert_eq!(muzzle.current_sprite_id(), 844);
        muzzle.age_ticks = 16.0;
        assert_eq!(muzzle.current_sprite_id(), 872);

        assert_eq!(
            fx.take_positional_sounds(),
            vec![PositionalSoundEvent::fixed(
                PRIMARY_SOUND_ID,
                [4.0, -4.0, 5.0]
            )]
        );
    }

    #[test]
    fn unresolved_primary_shooter_type_stays_unresolved_on_both_records() {
        let mut fx = WorldFx::new();
        let mut event = primary_event(-4.0, [0.0; 3]);
        event.source_entity_type_at_birth = None;
        fx.queue_primary_fire_batch(&[event], None);
        fx.process_pending();

        let particles = fx.test_particles_in_virgin_birth_order();
        assert_eq!(particles.len(), 2);
        assert!(particles
            .iter()
            .all(|particle| particle.source_entity_type_at_birth.is_none()));
    }

    #[test]
    fn upgraded_primary_materializes_class_three_descriptor_and_sound() {
        let mut fx = WorldFx::new();
        fx.queue_primary_fire_batch(
            &[upgraded_primary_event(-4.0, [1.0, 2.0, 3.0])],
            Some(-4.410_156_2),
        );
        fx.process_pending();

        let particles = fx.test_particles_in_virgin_birth_order();
        assert_eq!(particles.len(), 2);
        let projectile = particles[0];
        assert_eq!(projectile.source_class, UPGRADED_PRIMARY_PARTICLE_CLASS);
        assert_eq!(projectile.current_sprite_id(), 704);
        assert_eq!(projectile.lifetime_ticks, UPGRADED_PRIMARY_LIFETIME_TICKS);
        assert_eq!(projectile.collision_radius_raw, 20);
        assert_eq!(projectile.collision_radius_raw, UPGRADED_PRIMARY_RADIUS_RAW);
        assert_eq!(projectile.animation_rate_raw, 0x60);
        assert_eq!(
            projectile.animation_rate_raw,
            UPGRADED_PRIMARY_ANIMATION_RATE_RAW
        );
        assert_eq!(projectile.draw_scale_raw, 0x200);
        assert_eq!(projectile.draw_scale_raw, UPGRADED_PRIMARY_DRAW_SCALE_RAW);
        assert_eq!(projectile.frame_middle_raw, -8);
        assert_eq!(projectile.frame_scale_raw, 0x100);
        assert_eq!(
            projectile.velocity,
            [
                UPGRADED_PRIMARY_PROJECTILE_SPEC.speed_world_per_second + 1.0,
                2.0,
                3.0,
            ]
        );
        assert_eq!(particles[1].source_class, PRIMARY_MUZZLE_PARTICLE_CLASS);
        assert_eq!(
            fx.take_positional_sounds(),
            vec![PositionalSoundEvent::fixed(
                UPGRADED_PRIMARY_SOUND_ID,
                [4.0, -4.0, 5.0]
            )]
        );
    }

    #[test]
    fn upgraded_primary_descriptor_pins_all_recovered_callbacks() {
        let descriptor = particle_descriptor(UPGRADED_PRIMARY_PARTICLE_CLASS).unwrap();
        assert_eq!(descriptor.raw_byte(0x0e), 3);
        assert_eq!(descriptor.raw_u32(0x14), 0x0043_F260);
        assert_eq!(descriptor.raw_u32(0x18), 0x0043_E0D0);
        assert_eq!(descriptor.raw_u32(0x1c), 0x0043_F590);
        assert_eq!(descriptor.raw_u32(0x20), 0x004C_BF88);
        assert_eq!(descriptor.raw_u32(0x24), 0x0043_F800);
        assert_eq!(PRIMARY_PROJECTILE_DAMAGE_PACKET.channels, [2, 0]);
        assert_eq!(PRIMARY_PROJECTILE_DAMAGE_PACKET.amounts_raw, [2_000, 0]);
    }

    #[test]
    fn primary_smoke_is_suppressed_at_or_below_water_but_not_on_dry_levels() {
        let mut submerged = WorldFx::new();
        submerged.queue_primary_fire_batch(&[primary_event(-4.5, [0.0; 3])], Some(-4.410_156_2));
        submerged.process_pending();
        assert_eq!(submerged.test_particles_in_virgin_birth_order().len(), 1);
        assert_eq!(
            submerged.test_particles_in_virgin_birth_order()[0].source_class,
            PRIMARY_BULLET_PARTICLE_CLASS
        );

        let mut dry = WorldFx::new();
        dry.queue_primary_fire_batch(&[primary_event(-4.5, [0.0; 3])], None);
        dry.process_pending();
        assert_eq!(dry.test_particles_in_virgin_birth_order().len(), 2);
    }

    #[test]
    fn upgraded_primary_underwater_discharge_keeps_sound_but_has_no_particle() {
        for origin_y in [-4.5, -4.0] {
            let mut fx = WorldFx::new();
            fx.queue_primary_fire_batch(&[upgraded_primary_event(origin_y, [0.0; 3])], Some(-4.0));

            // The firing transaction survives; only its immediately rejected
            // class-3/class-15 particle presentation is absent underwater.
            assert_eq!(fx.pending_event_count(), 1);
            fx.process_pending();
            assert!(fx.test_particles_in_virgin_birth_order().is_empty());
            assert_eq!(
                fx.take_positional_sounds(),
                vec![PositionalSoundEvent::fixed(
                    UPGRADED_PRIMARY_SOUND_ID,
                    [4.0, origin_y, 5.0]
                )]
            );
        }

        let mut dry = WorldFx::new();
        dry.queue_primary_fire_batch(&[upgraded_primary_event(-4.5, [0.0; 3])], None);
        dry.process_pending();
        assert_eq!(dry.test_particles_in_virgin_birth_order().len(), 2);
    }

    #[test]
    fn cargo_transfer_queues_retail_particle_and_sound_as_distinct_events() {
        let position = [57.75, -4.0, 132.25];
        let mut fx = WorldFx::new();
        fx.queue_cargo_transfer_sound(position);
        fx.queue_cargo_transfer_particle(position, Some(93));
        assert_eq!(fx.pending_event_count(), 2);
        fx.process_pending();

        assert_eq!(
            fx.take_positional_sounds(),
            vec![PositionalSoundEvent::fixed(
                CARGO_TRANSFER_SOUND_ID,
                position
            )]
        );
        let mut particle = fx.test_particles_in_virgin_birth_order()[0];
        assert_eq!(particle.position, position);
        assert_eq!(particle.velocity, [0.0; 3]);
        assert_eq!(particle.owner_id, Some(93));
        assert_eq!(particle.source_class, CARGO_TRANSFER_PARTICLE_CLASS);
        assert_eq!(particle.lifetime_ticks, CARGO_TRANSFER_LIFETIME_TICKS);
        assert_eq!(particle.collision_radius_raw, CARGO_TRANSFER_RADIUS_RAW);
        assert_eq!(particle.draw_scale_raw, CARGO_TRANSFER_DRAW_SCALE_RAW);
        assert_eq!(particle.current_sprite_id(), 794);
        particle.age_ticks = 4.0;
        assert_eq!(particle.current_sprite_id(), 795);
        particle.age_ticks = 12.0;
        assert_eq!(particle.current_sprite_id(), 797);
        particle.age_ticks = 24.0;
        assert_eq!(particle.current_sprite_id(), 794);
    }

    #[test]
    fn player_attachment_packet_stays_newborn_through_the_early_particle_pass() {
        let mut fx = WorldFx::new();
        let mut expected_rng = fx.fork_for_main_base_abort_transaction();
        let position_raw = [14_784, -1024, -31_680];
        fx.queue_deferred_cargo_transfer(position_raw, 46, 46);
        fx.process_pending();
        fx.update(ParticleUpdateRequest::dry(20_000, 1));
        assert_eq!(fx.particle_count(), 0);
        assert_eq!(fx.pending_event_count(), 1);
        let birth = ParticleBirthContext {
            environment: ParticleEnvironment::Dry,
            retail_tick: 1,
        };
        fx.process_deferred_cargo_transfers(birth);
        let particle = fx.test_particles_in_virgin_birth_order()[0];
        assert_eq!(particle.source_class, 51);
        assert_eq!(particle.position, raw_position_to_world(position_raw));
        assert_eq!(particle.velocity, [0.; 3]);
        assert_eq!(particle.owner_id, Some(46));
        assert_eq!(particle.source_entity_type_at_birth, Some(46));
        assert_eq!(particle.age_ticks, 0.);
        assert_eq!(particle.current_sprite_id(), 794);
        assert_eq!(particle.size_jitter_divisor_raw, 0);
        assert_eq!(
            fx.take_positional_sounds(),
            vec![PositionalSoundEvent::fixed(
                8,
                raw_position_to_world(position_raw),
            )]
        );
        fx.process_deferred_cargo_transfers(birth);
        assert_eq!(fx.particle_count(), 1);
        assert!(fx.take_positional_sounds().is_empty());
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            expected_rng.next_shared_retail_random_u16()
        );
    }

    #[test]
    fn deferred_transfer_retains_particle_when_disposable_audio_is_collected() {
        let mut fx = WorldFx::new();
        fx.queue_deferred_cargo_transfer([0; 3], 46, 46);
        fx.garbage_collect_disposable_positional_sounds();
        fx.process_pending();
        fx.process_deferred_cargo_transfers(ParticleBirthContext {
            environment: ParticleEnvironment::Dry,
            retail_tick: 0,
        });
        assert_eq!(fx.particle_count(), 1);
        assert!(fx.take_positional_sounds().is_empty());
    }

    #[test]
    fn deferred_transfer_keeps_its_birth_wave_classification_for_the_next_traversal() {
        let mut terrain = flat_terrain(-128, 0);
        terrain.header[0] = 0;
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [0; 8],
            water_response_selectors: [0; 8],
        };
        let radius_raw = CARGO_TRANSFER_RADIUS_RAW as i16;
        let radius = f32::from(CARGO_TRANSFER_RADIUS_RAW) / 256.;
        let (birth_tick, position_raw, birth_state, next_state) = (0..100_u32).find_map(|tick| {
            let origin = [40., 0., 72.];
            let surface = displaced_water_surface(context, origin, tick).unwrap();
            let next_surface = displaced_water_surface(context, origin, tick + 1).unwrap();
            [surface, next_surface].into_iter().find_map(|edge| {
                let raw = [40 * 256, ((edge * 256.).round() as i16).wrapping_add(radius_raw), 72 * 256];
                let position = raw_position_to_world(raw);
                let birth_state = classify_particle_water(Some(context), None, position, radius, tick);
                let next_state = classify_particle_water(Some(context), None, position, radius, tick + 1);
                (birth_state != next_state).then_some((tick, raw, birth_state, next_state))
            })
        }).expect("the displaced surface crosses a signed-word class51 sphere edge between adjacent ticks");
        let mut fx = WorldFx::new();
        fx.queue_deferred_cargo_transfer(position_raw, 46, 46);
        fx.process_deferred_cargo_transfers(ParticleBirthContext {
            environment: ParticleEnvironment::Terrain(context),
            retail_tick: birth_tick,
        });
        let particle = fx.test_particles_in_virgin_birth_order()[0];
        assert!(particle.water_state_initialized);
        assert_eq!(particle.water_state, birth_state);
        assert_eq!(particle.step_start_water_state, birth_state);
        assert_eq!(particle.age_ticks, 0.);
        fx.update(ParticleUpdateRequest::terrain(
            20_000,
            birth_tick + 1,
            context,
        ));
        let particle = fx.test_particles_in_virgin_birth_order()[0];
        assert_eq!(particle.step_start_water_state, birth_state);
        assert_eq!(particle.water_state, next_state);
        assert_ne!(particle.step_start_water_state, particle.water_state);
    }

    #[test]
    fn cargo_full_queues_retail_positional_failure_sound() {
        let position = [10.0, -4.0, 20.0];
        let mut fx = WorldFx::new();
        fx.queue_cargo_full_sound(position);
        fx.process_pending();
        assert_eq!(
            fx.take_positional_sounds(),
            vec![PositionalSoundEvent::fixed(CARGO_FULL_SOUND_ID, position)]
        );
    }

    #[test]
    fn primary_smoke_refreshes_owner_velocity_after_generic_integration() {
        let mut fx = WorldFx::new();
        fx.queue_primary_fire_batch(&[primary_event(-4.0, [1.0, 0.0, 0.0])], None);
        fx.process_pending();
        let muzzle_start = fx.test_particles_in_virgin_birth_order()[1].position;
        fx.update(
            ParticleUpdateRequest::dry(20_000, 1).with_callbacks(ParticleCallbackContext {
                owner_motions: &[ParticleOwnerMotion {
                    owner_id: 7,
                    velocity: [3.0, 4.0, 5.0],
                }],
                collision: None,
            }),
        );
        assert!(
            (fx.test_particles_in_virgin_birth_order()[1].position[0] - (muzzle_start[0] + 0.02))
                .abs()
                < 1.0e-6
        );
        assert_eq!(
            fx.test_particles_in_virgin_birth_order()[1].velocity,
            [3.0, 4.0, 5.0]
        );
        assert_ne!(
            fx.test_particles_in_virgin_birth_order()[0].velocity,
            [3.0, 4.0, 5.0]
        );

        // A failed retail handle lookup leaves class 32's most recently
        // copied velocity untouched rather than zeroing or re-inheriting it.
        fx.update(ParticleUpdateRequest::dry(20_000, 2));
        assert_eq!(
            fx.test_particles_in_virgin_birth_order()[1].velocity,
            [3.0, 4.0, 5.0]
        );
    }

    #[test]
    fn primary_bullet_callback_applies_full_or_underwater_gravity_and_drag() {
        let dt = 0.02;
        let mut above = WorldFx::new();
        above.queue_primary_fire_batch(&[primary_event(-4.0, [0.0; 3])], Some(-10.0));
        above.process_pending();
        let speed_before = above.test_particles_in_virgin_birth_order()[0].velocity[0];
        above.update(ParticleUpdateRequest::flat_water(20_000, 1, -10.0));
        assert!(
            (above.test_particles_in_virgin_birth_order()[0].velocity[0] - speed_before).abs()
                < 1.0e-6
        );
        assert!(
            (above.test_particles_in_virgin_birth_order()[0].velocity[1]
                + METEOR_SCATTER_GRAVITY_WORLD_PER_SEC2 * dt)
                .abs()
                < 1.0e-6
        );

        let mut underwater = WorldFx::new();
        underwater.queue_primary_fire_batch(&[primary_event(-4.0, [0.0; 3])], Some(-3.0));
        underwater.process_pending();
        let underwater_speed_before =
            underwater.test_particles_in_virgin_birth_order()[0].velocity[0];
        underwater.update(ParticleUpdateRequest::flat_water(20_000, 1, -3.0));
        let drag = 1.0 - PRIMARY_BULLET_UNDERWATER_DRAG_PER_SECOND * dt;
        assert!(
            (underwater.test_particles_in_virgin_birth_order()[0].velocity[0]
                - underwater_speed_before * drag)
                .abs()
                < 1.0e-6
        );
        assert!(
            (underwater.test_particles_in_virgin_birth_order()[0].velocity[1]
                + METEOR_SCATTER_GRAVITY_WORLD_PER_SEC2 * 0.5 * dt)
                .abs()
                < 1.0e-6
        );
    }

    #[test]
    fn primary_bullet_callback_frees_slow_records_before_gravity() {
        let mut fx = WorldFx::new();
        fx.queue_primary_fire_batch(&[primary_event(-4.0, [0.0; 3])], None);
        fx.process_pending();
        fx.particles[0].velocity = [
            PRIMARY_BULLET_MIN_SPEED_COMPONENT_WORLD_PER_SECOND * 0.5,
            0.0,
            0.0,
        ];
        fx.update(ParticleUpdateRequest::dry(20_000, 1));
        assert!(fx
            .test_particles_in_virgin_birth_order()
            .iter()
            .all(|particle| particle.source_class != PRIMARY_BULLET_PARTICLE_CLASS));
    }

    #[test]
    fn primary_entity_collision_uses_segment_probe_active_order_and_endpoint_effect() {
        let mut fx = WorldFx::new();
        fx.queue_primary_fire_batch(&[primary_event(2.0, [0.0; 3])], None);
        fx.process_pending();
        fx.take_positional_sounds();
        let start = fx.test_particles_in_virgin_birth_order()[0].position;
        let bullet_slot = fx
            .particles
            .slots
            .iter()
            .position(|particle| {
                particle
                    .is_some_and(|particle| particle.source_class == PRIMARY_BULLET_PARTICLE_CLASS)
            })
            .unwrap();

        // The endpoint travels beyond both targets. Retail's midpoint sphere
        // finds the traversed region, then keeps active-list order.
        let bullet = fx.test_particles_in_virgin_birth_order()[0];
        let endpoint = [
            v2k_core::world::wrap(start[0] + bullet.velocity[0] * 0.1),
            start[1] + bullet.velocity[1] * 0.1,
            v2k_core::world::wrap(start[2] + bullet.velocity[2] * 0.1),
        ];
        let mut post_update_velocity = bullet.velocity;
        post_update_velocity[1] -= METEOR_SCATTER_GRAVITY_WORLD_PER_SEC2 * 0.1;
        let post_update_velocity_raw =
            post_update_velocity.map(|component| world_velocity_component_to_raw(component) as i16);
        assert!(post_update_velocity_raw[1] < 0);
        assert!(endpoint[0] > start[0] + 1.0);
        let midpoint_x = (start[0] + endpoint[0]) * 0.5;
        let pool = solid_sphere_pool(100);
        let entities = [
            collision_entity(9, [midpoint_x, 2.0, 5.0], 80),
            collision_entity(8, [midpoint_x, 2.0, 5.0], 80),
        ];
        let impacts = fx
            .update(
                ParticleUpdateRequest::flat_water(100_000, 5, -10.0).with_callbacks(
                    ParticleCallbackContext {
                        owner_motions: &[],
                        collision: Some(ParticleCollisionContext {
                            entities: &entities,
                            model_pool: &pool,
                        }),
                    },
                ),
            )
            .entity_impacts;

        assert_eq!(
            impacts,
            vec![ParticleEntityImpact {
                source_particle_class: PRIMARY_BULLET_PARTICLE_CLASS,
                impact_position_argument_va: retail_particle_impact_position_argument_va(
                    bullet_slot,
                ),
                target_entity_id: 9,
                position_world: endpoint,
                velocity_raw: post_update_velocity_raw,
                damage: Some(BallisticDamageRequest {
                    packet: PRIMARY_PROJECTILE_DAMAGE_PACKET,
                    source_entity_type_at_birth: Some(46),
                    source_owner_id: Some(7),
                }),
            }]
        );
        assert!(!fx
            .test_particles_in_virgin_birth_order()
            .iter()
            .any(|particle| particle.source_class == PRIMARY_BULLET_PARTICLE_CLASS));
        let effect = fx
            .test_particles_in_virgin_birth_order()
            .into_iter()
            .find(|particle| particle.source_class == PRIMARY_ABOVE_WATER_IMPACT_CLASS)
            .unwrap();
        assert_eq!(effect.position, endpoint);
        assert_eq!(effect.current_sprite_id(), 895);
        assert_eq!(effect.lifetime_ticks, 80);
        assert_eq!(effect.collision_radius_raw, 20);
        assert_eq!(effect.draw_scale_raw, 0x0400);
        assert_eq!(
            effect.velocity,
            raw_velocity_to_world(PRIMARY_ABOVE_WATER_IMPACT_INITIAL_VELOCITY_RAW)
        );
    }

    #[test]
    fn primary_f590_handler_runs_after_visual_before_free_and_honors_suppression() {
        let mut parent = descriptor_test_particle(
            PRIMARY_BULLET_PARTICLE_CLASS,
            [4.0, 2.0, 5.0],
            [1_000, 0, 0],
            Some(7),
        );
        parent.source_entity_type_at_birth = Some(46);
        parent.suppresses_impact_damage = true;
        let mut fx = WorldFx::new();
        let parent_slot = fx.particles.allocate(parent).unwrap();
        let pool = solid_sphere_pool(100);
        let entities = [collision_entity(9, parent.position, 100)];
        let mut handled = Vec::new();
        let mut handler_saw_parent_and_visual = false;

        let outcome = fx.update_with_event_handlers(
            ParticleUpdateRequest::dry(0, 0).with_callbacks(ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &entities,
                    model_pool: &pool,
                }),
            }),
            |live_fx, impact| {
                handler_saw_parent_and_visual = live_fx.particles.slots[parent_slot]
                    .is_some_and(|particle| particle.source_class == PRIMARY_BULLET_PARTICLE_CLASS)
                    && live_fx.particles.slots.iter().flatten().any(|particle| {
                        particle.source_class == PRIMARY_ABOVE_WATER_IMPACT_CLASS
                            && particle.suppresses_impact_damage
                    });
                handled.push(impact);
                ParticleCollisionCacheRefresh::Unchanged
            },
            |_, _| ParticleTerrainResponse::Unhandled,
        );

        assert!(handler_saw_parent_and_visual);
        assert_eq!(handled, outcome.entity_impacts);
        assert!(matches!(
            handled.as_slice(),
            [ParticleEntityImpact {
                source_particle_class: PRIMARY_BULLET_PARTICLE_CLASS,
                impact_position_argument_va,
                target_entity_id: 9,
                damage: None,
                ..
            }] if *impact_position_argument_va
                == retail_particle_impact_position_argument_va(parent_slot)
        ));
        assert!(fx.particles.slots[parent_slot].is_none());
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn f6e0_entity_hit_always_delivers_packet_and_sound_90() {
        let mut parent = descriptor_test_particle(4, [4.0, 2.0, 5.0], [1_000, 0, 0], Some(7));
        parent.source_entity_type_at_birth = Some(46);
        parent.suppresses_impact_damage = true;
        let mut fx = WorldFx::new();
        let parent_slot = fx.particles.allocate(parent).unwrap();
        let pool = solid_sphere_pool(100);
        let entities = [collision_entity(9, parent.position, 100)];
        let mut static_hits = 0;
        let mut handled = Vec::new();

        let outcome = fx.update_with_event_handlers(
            ParticleUpdateRequest::dry(0, 0).with_callbacks(ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &entities,
                    model_pool: &pool,
                }),
            }),
            |_, impact| {
                handled.push(impact);
                ParticleCollisionCacheRefresh::Unchanged
            },
            |_, event| {
                expect_static_event(event);
                static_hits += 1;
                ParticleTerrainResponse::Unhandled
            },
        );

        assert_eq!(static_hits, 0);
        assert_eq!(handled, outcome.entity_impacts);
        assert!(matches!(
            handled.as_slice(),
            [ParticleEntityImpact {
                source_particle_class: 4,
                target_entity_id: 9,
                damage: Some(BallisticDamageRequest {
                    packet: FUN_0043F6E0_DAMAGE_PACKET,
                    source_entity_type_at_birth: Some(46),
                    source_owner_id: Some(7),
                }),
                ..
            }]
        ));
        assert!(fx
            .particles
            .slots
            .iter()
            .flatten()
            .any(|particle| { particle.source_class == PRIMARY_ABOVE_WATER_IMPACT_CLASS }));
        let mut rng_state = 0;
        let expected_rate = 0x1_0000 + u32::from(retail_random_u16(&mut rng_state) >> 3);
        assert_eq!(
            fx.ready_sounds,
            [PositionalSoundEvent {
                sound_id: 90,
                position: parent.position,
                frequency_q16: expected_rate,
            }]
        );
        assert!(fx.particles.slots[parent_slot].is_none());
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn f780_entity_hit_delivers_packet_without_f610() {
        let mut parent = descriptor_test_particle(5, [4.0, 2.0, 5.0], [1_000, 0, 0], Some(7));
        parent.source_entity_type_at_birth = Some(46);
        let mut fx = WorldFx::new();
        let parent_slot = fx.particles.allocate(parent).unwrap();
        let pool = solid_sphere_pool(100);
        let entities = [collision_entity(9, parent.position, 100)];
        let mut handled = Vec::new();

        let outcome = fx.update_with_event_handlers(
            ParticleUpdateRequest::dry(0, 0).with_callbacks(ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &entities,
                    model_pool: &pool,
                }),
            }),
            |_, impact| {
                handled.push(impact);
                ParticleCollisionCacheRefresh::Unchanged
            },
            |_, _| ParticleTerrainResponse::Unhandled,
        );

        assert_eq!(handled, outcome.entity_impacts);
        assert!(matches!(
            handled.as_slice(),
            [ParticleEntityImpact {
                source_particle_class: 5,
                target_entity_id: 9,
                damage: Some(BallisticDamageRequest {
                    packet: FUN_0043F780_DAMAGE_PACKET,
                    source_entity_type_at_birth: Some(46),
                    source_owner_id: Some(7),
                }),
                ..
            }]
        ));
        assert!(!fx
            .particles
            .slots
            .iter()
            .flatten()
            .any(|particle| particle.source_class == PRIMARY_ABOVE_WATER_IMPACT_CLASS));
        assert!(fx.ready_sounds.is_empty());
        assert!(fx.particles.slots[parent_slot].is_none());
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn f920_static_sets_tile_center_infection_without_f610() {
        let mut terrain = flat_terrain(16, 0);
        *terrain_cell_mut(&mut terrain, 3, 4) = TerrainCell {
            height: 16,
            attribute: 1,
            terrain_type: 0x08, // Start uninfected; bit 0x10 already set is a no-op.
        };
        let objects = terrain_object_table(1, [0, 0, 0, 0], 29);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: Some(&objects),
            ground_response_selectors: [5; 8],
            water_response_selectors: [7; 8],
        };
        let pool = solid_sphere_pool(200);
        let mut parent = descriptor_test_particle(5, [4.0, 2.0, 5.0], [1_000, 0, 0], Some(7));
        parent.source_entity_type_at_birth = Some(46);
        let mut fx = WorldFx::new();
        let parent_slot = fx.particles.allocate(parent).unwrap();
        let mut static_hits = 0;

        let outcome = fx.update_with_event_handlers(
            ParticleUpdateRequest::terrain(0, 0, context).with_callbacks(ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &[],
                    model_pool: &pool,
                }),
            }),
            |_, _| ParticleCollisionCacheRefresh::Unchanged,
            |_, event| {
                expect_static_event(event);
                static_hits += 1;
                ParticleTerrainResponse::Unhandled
            },
        );

        assert_eq!(static_hits, 0);
        assert!(outcome.entity_impacts.is_empty());
        assert!(outcome.primary_impacts.is_empty());
        assert_eq!(
            outcome.terrain_type_mutations,
            vec![ParticleTerrainMutation::Infection {
                cell: [3, 4],
                infected: true,
            }]
        );
        assert!(fx.particles.slots[parent_slot].is_none());
        assert!(!fx
            .particles
            .slots
            .iter()
            .flatten()
            .any(|particle| particle.source_class == PRIMARY_ABOVE_WATER_IMPACT_CLASS));
        assert!(fx.ready_sounds.is_empty());
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn f920_static_suppression_skips_33720_and_still_frees() {
        let mut terrain = flat_terrain(16, 0);
        *terrain_cell_mut(&mut terrain, 3, 4) = TerrainCell {
            height: 16,
            attribute: 1,
            terrain_type: 0x18,
        };
        let objects = terrain_object_table(1, [0, 0, 0, 0], 29);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: Some(&objects),
            ground_response_selectors: [5; 8],
            water_response_selectors: [7; 8],
        };
        let pool = solid_sphere_pool(200);
        let mut parent = descriptor_test_particle(5, [4.0, 2.0, 5.0], [1_000, 0, 0], Some(7));
        parent.source_entity_type_at_birth = Some(46);
        parent.suppresses_impact_damage = true;
        let mut fx = WorldFx::new();
        let parent_slot = fx.particles.allocate(parent).unwrap();

        let outcome = fx.update_with_event_handlers(
            ParticleUpdateRequest::terrain(0, 0, context).with_callbacks(ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &[],
                    model_pool: &pool,
                }),
            }),
            |_, _| ParticleCollisionCacheRefresh::Unchanged,
            |_, _| ParticleTerrainResponse::Unhandled,
        );

        assert!(outcome.terrain_type_mutations.is_empty());
        assert!(fx.particles.slots[parent_slot].is_none());
        assert!(!fx
            .particles
            .slots
            .iter()
            .flatten()
            .any(|particle| particle.source_class == PRIMARY_ABOVE_WATER_IMPACT_CLASS));
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn f920_static_already_infected_tile_is_a_silent_set() {
        let mut terrain = flat_terrain(16, 0);
        *terrain_cell_mut(&mut terrain, 3, 4) = TerrainCell {
            height: 16,
            attribute: 1,
            terrain_type: 0x18 | INFECTION_TERRAIN_TYPE_BIT,
        };
        let objects = terrain_object_table(1, [0, 0, 0, 0], 29);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: Some(&objects),
            ground_response_selectors: [5; 8],
            water_response_selectors: [7; 8],
        };
        let pool = solid_sphere_pool(200);
        let mut parent = descriptor_test_particle(5, [4.0, 2.0, 5.0], [1_000, 0, 0], Some(7));
        parent.source_entity_type_at_birth = Some(46);
        let mut fx = WorldFx::new();
        let parent_slot = fx.particles.allocate(parent).unwrap();

        let outcome = fx.update_with_event_handlers(
            ParticleUpdateRequest::terrain(0, 0, context).with_callbacks(ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &[],
                    model_pool: &pool,
                }),
            }),
            |_, _| ParticleCollisionCacheRefresh::Unchanged,
            |_, _| ParticleTerrainResponse::Unhandled,
        );

        assert!(outcome.terrain_type_mutations.is_empty());
        assert!(fx.particles.slots[parent_slot].is_none());
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn f890_static_prefix_runs_f610_and_skips_f800_damage() {
        let mut terrain = flat_terrain(16, 0);
        *terrain_cell_mut(&mut terrain, 3, 4) = TerrainCell {
            height: 16,
            attribute: 1,
            terrain_type: 0x18,
        };
        let objects = terrain_object_table(1, [0, 0, 0, 0], 29);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: Some(&objects),
            ground_response_selectors: [5; 8],
            water_response_selectors: [7; 8],
        };
        let pool = solid_sphere_pool(200);
        let mut parent = descriptor_test_particle(4, [4.0, 2.0, 5.0], [1_000, 0, 0], Some(7));
        parent.source_entity_type_at_birth = Some(46);
        let mut fx = WorldFx::new();
        let parent_slot = fx.particles.allocate(parent).unwrap();
        let mut static_hits = 0;

        let outcome = fx.update_with_event_handlers(
            ParticleUpdateRequest::terrain(0, 0, context).with_callbacks(ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &[],
                    model_pool: &pool,
                }),
            }),
            |_, _| ParticleCollisionCacheRefresh::Unchanged,
            |_, event| {
                expect_static_event(event);
                static_hits += 1;
                ParticleTerrainResponse::Unhandled
            },
        );

        assert_eq!(static_hits, 0);
        assert!(outcome.entity_impacts.is_empty());
        assert!(outcome.primary_impacts.is_empty());
        assert!(fx.particles.slots[parent_slot].is_none());
        assert!(fx
            .particles
            .slots
            .iter()
            .flatten()
            .any(|particle| particle.source_class == PRIMARY_ABOVE_WATER_IMPACT_CLASS));
        assert!(fx
            .particles
            .slots
            .iter()
            .flatten()
            .any(|particle| particle.source_class == PRIMARY_BULLET_PARTICLE_CLASS));
        let mut rng_state = 0;
        let rate_90 = 0x1_0000 + u32::from(retail_random_u16(&mut rng_state) >> 3);
        let rate_62 = 0x1_0000 + u32::from(retail_random_u16(&mut rng_state) >> 3);
        assert_eq!(
            fx.ready_sounds,
            [
                PositionalSoundEvent {
                    sound_id: 90,
                    position: parent.position,
                    frequency_q16: rate_90,
                },
                PositionalSoundEvent {
                    sound_id: 62,
                    position: parent.position,
                    frequency_q16: rate_62,
                },
            ]
        );
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn f890_class_54_static_runs_410b0_class_39() {
        let mut terrain = flat_terrain(16, 0);
        *terrain_cell_mut(&mut terrain, 3, 4) = TerrainCell {
            height: 16,
            attribute: 1,
            terrain_type: 0x18,
        };
        let objects = terrain_object_table(1, [0, 0, 0, 0], 29);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: Some(&objects),
            ground_response_selectors: [5; 8],
            water_response_selectors: [7; 8],
        };
        let pool = solid_sphere_pool(200);
        let mut parent = descriptor_test_particle(54, [4.0, 2.0, 5.0], [1_000, 0, 0], Some(7));
        parent.source_entity_type_at_birth = Some(46);
        let mut fx = WorldFx::new();
        let parent_slot = fx.particles.allocate(parent).unwrap();
        let mut static_hits = 0;

        let outcome = fx.update_with_event_handlers(
            ParticleUpdateRequest::terrain(0, 0, context).with_callbacks(ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &[],
                    model_pool: &pool,
                }),
            }),
            |_, _| ParticleCollisionCacheRefresh::Unchanged,
            |_, event| {
                expect_static_event(event);
                static_hits += 1;
                ParticleTerrainResponse::Unhandled
            },
        );

        assert_eq!(static_hits, 0);
        assert!(outcome.entity_impacts.is_empty());
        assert!(outcome.primary_impacts.is_empty());
        assert!(fx.particles.slots[parent_slot].is_none());
        assert!(fx
            .particles
            .slots
            .iter()
            .flatten()
            .any(|particle| particle.source_class == PRIMARY_ABOVE_WATER_IMPACT_CLASS));
        assert_eq!(
            fx.particles
                .slots
                .iter()
                .flatten()
                .filter(|particle| particle.source_class == FUN_00441850_CLASS_54_DEBRIS_CLASS)
                .count(),
            1
        );
        let mut rng_state = 0;
        let rate_90 = 0x1_0000 + u32::from(retail_random_u16(&mut rng_state) >> 3);
        let rate_62 = 0x1_0000 + u32::from(retail_random_u16(&mut rng_state) >> 3);
        assert_eq!(
            fx.ready_sounds,
            [
                PositionalSoundEvent {
                    sound_id: 90,
                    position: parent.position,
                    frequency_q16: rate_90,
                },
                PositionalSoundEvent {
                    sound_id: 62,
                    position: parent.position,
                    frequency_q16: rate_62,
                },
            ]
        );
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn primary_f590_outer_free_removes_handler_replacement_at_parent_address() {
        let position = [4.0, 2.0, 5.0];
        let mut parent = descriptor_test_particle(
            PRIMARY_BULLET_PARTICLE_CLASS,
            position,
            [1_000, 0, 0],
            Some(7),
        );
        parent.age_ticks = 1.0;
        let mut fx = WorldFx::new();
        let parent_slot = fx.particles.allocate(parent).unwrap();
        assert_eq!(parent_slot, MAX_WORLD_PARTICLES - 1);
        let filler = descriptor_test_particle(
            PRIMARY_BULLET_PARTICLE_CLASS,
            [20.0, 20.0, 20.0],
            [1_000, 0, 0],
            None,
        );
        fx.particles.test_fill_to(MAX_WORLD_PARTICLES, filler);
        let pool = solid_sphere_pool(100);
        let entities = [collision_entity(9, position, 100)];
        let mut recycled_slot = None;

        let outcome = fx.update_with_event_handlers(
            ParticleUpdateRequest::dry(0, 0).with_callbacks(ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &entities,
                    model_pool: &pool,
                }),
            }),
            |live_fx, _| {
                assert_eq!(live_fx.particle_count(), MAX_WORLD_PARTICLES);
                let replacement = descriptor_test_particle(
                    PRIMARY_BULLET_PARTICLE_CLASS,
                    [99.0, 98.0, 97.0],
                    [1_000, 0, 0],
                    None,
                );
                recycled_slot = live_fx.particles.allocate(replacement);
                assert_eq!(recycled_slot, Some(parent_slot));
                assert_eq!(
                    live_fx.particles.slots[parent_slot].unwrap().position,
                    [99.0, 98.0, 97.0]
                );
                ParticleCollisionCacheRefresh::Unchanged
            },
            |_, _| ParticleTerrainResponse::Unhandled,
        );

        assert_eq!(outcome.entity_impacts.len(), 1);
        assert_eq!(recycled_slot, Some(parent_slot));
        assert!(fx.particles.slots[parent_slot].is_none());
        assert_eq!(fx.particle_count(), MAX_WORLD_PARTICLES - 1);
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn primary_f610_uses_authored_flat_sea_plane_even_when_water_is_disabled() {
        let mut terrain = flat_terrain(0, 0);
        terrain.header[0] = (-4096_i32) << 8;
        assert!(!terrain.water_enabled());
        assert_eq!(terrain.sea_level_world_y(), -16.0);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [0; 8],
            water_response_selectors: [0; 8],
        };
        let mut parent = descriptor_test_particle(
            PRIMARY_BULLET_PARTICLE_CLASS,
            [4.0, -16.0, 5.0],
            [1_000, 0, 0],
            Some(7),
        );
        parent.source_entity_type_at_birth = Some(46);
        let mut fx = WorldFx::new();
        fx.particles.allocate(parent).unwrap();
        let pool = solid_sphere_pool(100);
        let entities = [collision_entity(9, parent.position, 100)];

        fx.update(
            ParticleUpdateRequest::terrain(0, 0, context).with_callbacks(ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &entities,
                    model_pool: &pool,
                }),
            }),
        );

        assert!(fx.particles.slots.iter().flatten().any(|particle| {
            particle.source_class == PRIMARY_UNDERWATER_IMPACT_CLASS
                && particle.position == parent.position
        }));
        assert!(!fx
            .particles
            .slots
            .iter()
            .flatten()
            .any(|particle| { particle.source_class == PRIMARY_ABOVE_WATER_IMPACT_CLASS }));
    }

    #[test]
    fn upgraded_primary_uses_shared_entity_callback_and_damage_template() {
        let mut fx = WorldFx::new();
        fx.queue_primary_fire_batch(&[upgraded_primary_event(2.0, [0.0; 3])], None);
        fx.process_pending();
        fx.take_positional_sounds();
        let projectile = fx
            .test_particles_in_virgin_birth_order()
            .into_iter()
            .find(|particle| particle.source_class == UPGRADED_PRIMARY_PARTICLE_CLASS)
            .unwrap();
        let projectile_slot = fx
            .particles
            .slots
            .iter()
            .position(|particle| {
                particle.is_some_and(|particle| {
                    particle.source_class == UPGRADED_PRIMARY_PARTICLE_CLASS
                })
            })
            .unwrap();
        let pool = solid_sphere_pool(100);
        let entities = [collision_entity(9, projectile.position, 100)];

        let impacts = fx
            .update(
                ParticleUpdateRequest::dry(0, 0).with_callbacks(ParticleCallbackContext {
                    owner_motions: &[],
                    collision: Some(ParticleCollisionContext {
                        entities: &entities,
                        model_pool: &pool,
                    }),
                }),
            )
            .entity_impacts;

        assert_eq!(
            impacts,
            vec![ParticleEntityImpact {
                source_particle_class: UPGRADED_PRIMARY_PARTICLE_CLASS,
                impact_position_argument_va: retail_particle_impact_position_argument_va(
                    projectile_slot,
                ),
                target_entity_id: 9,
                position_world: projectile.position,
                velocity_raw: projectile
                    .velocity
                    .map(|component| world_velocity_component_to_raw(component) as i16),
                damage: Some(BallisticDamageRequest {
                    packet: PRIMARY_PROJECTILE_DAMAGE_PACKET,
                    source_entity_type_at_birth: Some(46),
                    source_owner_id: Some(7),
                }),
            }]
        );
        assert!(!fx
            .test_particles_in_virgin_birth_order()
            .iter()
            .any(|particle| particle.source_class == UPGRADED_PRIMARY_PARTICLE_CLASS));
        assert!(fx
            .test_particles_in_virgin_birth_order()
            .iter()
            .any(|particle| particle.source_class == PRIMARY_ABOVE_WATER_IMPACT_CLASS));
    }

    #[test]
    fn primary_impact_allocation_fails_before_bullet_free_in_saturated_pool() {
        let mut source = WorldFx::new();
        source.queue_primary_fire_batch(&[primary_event(2.0, [0.0; 3])], None);
        source.process_pending();
        let bullet = source
            .test_particles_in_virgin_birth_order()
            .into_iter()
            .find(|particle| particle.source_class == PRIMARY_BULLET_PARTICLE_CLASS)
            .unwrap();

        let mut fx = WorldFx::new();
        let filler = pool_test_particle(31, 0.0);
        fx.particles.test_fill_to(MAX_WORLD_PARTICLES - 1, filler);
        fx.particles.test_allocate(bullet);
        assert_eq!(
            fx.particles.slots[0].unwrap().source_class,
            PRIMARY_BULLET_PARTICLE_CLASS
        );

        let pool = solid_sphere_pool(100);
        let entities = [collision_entity(9, bullet.position, 100)];
        let outcome = fx.update(ParticleUpdateRequest::dry(0, 0).with_callbacks(
            ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &entities,
                    model_pool: &pool,
                }),
            },
        ));

        assert_eq!(outcome.entity_impacts.len(), 1);
        assert_eq!(fx.particle_count(), MAX_WORLD_PARTICLES - 1);
        assert!(fx
            .particles
            .slots
            .iter()
            .flatten()
            .all(|particle| particle.source_class == 31));
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn later_slot_primary_impact_child_runs_in_its_birth_pass() {
        let mut source = WorldFx::new();
        source.queue_primary_fire_batch(&[primary_event(2.0, [0.0; 3])], None);
        source.process_pending();
        let bullet = source
            .test_particles_in_virgin_birth_order()
            .into_iter()
            .find(|particle| particle.source_class == PRIMARY_BULLET_PARTICLE_CLASS)
            .unwrap();

        let mut fx = WorldFx::new();
        fx.particles
            .test_fill_to(MAX_WORLD_PARTICLES - 1, pool_test_particle(31, 0.0));
        fx.particles.test_allocate(bullet);
        assert!(fx.particles.free(100));

        let pool = solid_sphere_pool(100);
        let entities = [collision_entity(9, bullet.position, 100)];
        let outcome = fx.update(ParticleUpdateRequest::dry(0, 0).with_callbacks(
            ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &entities,
                    model_pool: &pool,
                }),
            },
        ));

        assert_eq!(outcome.entity_impacts.len(), 1);
        let effect = fx.particles.slots[100].unwrap();
        assert_eq!(effect.source_class, PRIMARY_ABOVE_WATER_IMPACT_CLASS);
        let initial_velocity =
            raw_velocity_to_world(PRIMARY_ABOVE_WATER_IMPACT_INITIAL_VELOCITY_RAW);
        assert_ne!(effect.velocity, initial_velocity);
        let mut expected_rng = 0;
        for _ in 0..3 {
            retail_random_u16(&mut expected_rng);
        }
        assert_eq!(fx.rng_state, expected_rng);
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn primary_impacts_follow_physical_slot_order() {
        let mut fx = WorldFx::new();
        fx.queue_primary_fire_batch(&[primary_event(2.0, [0.0; 3])], None);
        fx.queue_primary_fire_batch(&[primary_event(2.0, [0.0; 3])], None);
        fx.process_pending();
        fx.take_positional_sounds();

        let bullet_slots = fx
            .particles
            .slots
            .iter()
            .enumerate()
            .filter_map(|(slot, particle)| {
                particle
                    .filter(|particle| particle.source_class == PRIMARY_BULLET_PARTICLE_CLASS)
                    .map(|_| slot)
            })
            .collect::<Vec<_>>();
        assert_eq!(bullet_slots, vec![197, 199]);
        fx.particles.slots[bullet_slots[0]]
            .as_mut()
            .unwrap()
            .owner_id = Some(11);
        fx.particles.slots[bullet_slots[1]]
            .as_mut()
            .unwrap()
            .owner_id = Some(22);

        let bullet_position = fx.particles.slots[bullet_slots[0]].unwrap().position;
        let entities = [collision_entity(9, bullet_position, 100)];
        let pool = solid_sphere_pool(100);
        let outcome = fx.update(ParticleUpdateRequest::dry(0, 0).with_callbacks(
            ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &entities,
                    model_pool: &pool,
                }),
            },
        ));

        let owners = outcome
            .entity_impacts
            .iter()
            .map(|impact| impact.damage.unwrap().source_owner_id.unwrap())
            .collect::<Vec<_>>();
        assert_eq!(owners, vec![11, 22]);
    }

    #[test]
    fn impact_handler_refreshes_collision_state_before_the_next_physical_slot() {
        let mut fx = WorldFx::new();
        fx.queue_primary_fire_batch(&[primary_event(2.0, [0.0; 3])], None);
        fx.queue_primary_fire_batch(&[primary_event(2.0, [0.0; 3])], None);
        fx.process_pending();
        fx.take_positional_sounds();

        let bullet_slots = fx
            .particles
            .slots
            .iter()
            .enumerate()
            .filter_map(|(slot, particle)| {
                particle
                    .filter(|particle| particle.source_class == PRIMARY_BULLET_PARTICLE_CLASS)
                    .map(|_| slot)
            })
            .collect::<Vec<_>>();
        assert_eq!(bullet_slots, vec![197, 199]);
        fx.particles.slots[bullet_slots[0]]
            .as_mut()
            .unwrap()
            .owner_id = Some(11);
        fx.particles.slots[bullet_slots[1]]
            .as_mut()
            .unwrap()
            .owner_id = Some(22);

        let bullet_position = fx.particles.slots[bullet_slots[0]].unwrap().position;
        let entities = [collision_entity(9, bullet_position, 100)];
        let pool = solid_sphere_pool(100);
        let mut handled_owners = Vec::new();
        let outcome = fx.update_with_event_handlers(
            ParticleUpdateRequest::dry(0, 0).with_callbacks(ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &entities,
                    model_pool: &pool,
                }),
            }),
            |_, impact| {
                handled_owners.push(impact.damage.unwrap().source_owner_id.unwrap());
                ParticleCollisionCacheRefresh::Replace(vec![collision_entity(
                    9,
                    [100.0, 100.0, 100.0],
                    100,
                )])
            },
            |_, _| ParticleTerrainResponse::Unhandled,
        );

        assert_eq!(handled_owners, [11]);
        assert_eq!(outcome.entity_impacts.len(), 1);
        assert!(fx.particles.slots[bullet_slots[0]].is_none());
        assert!(fx.particles.slots[bullet_slots[1]].is_some_and(|particle| {
            particle.source_class == PRIMARY_BULLET_PARTICLE_CLASS && particle.owner_id == Some(22)
        }));
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn f590_handlers_preserve_physical_order_across_projectile_families() {
        let position = [4.0, 2.0, 5.0];
        let mut ballistic =
            descriptor_test_particle(METEOR_SCATTER_CLASS, position, [1_000, 0, 0], Some(22));
        ballistic.source_entity_type_at_birth = Some(68);
        let mut primary = descriptor_test_particle(
            PRIMARY_BULLET_PARTICLE_CLASS,
            position,
            [1_000, 0, 0],
            Some(11),
        );
        primary.source_entity_type_at_birth = Some(46);
        let mut fx = WorldFx::new();
        let ballistic_slot = fx.particles.allocate(ballistic).unwrap();
        let primary_slot = fx.particles.allocate(primary).unwrap();
        assert!(primary_slot < ballistic_slot);
        let pool = solid_sphere_pool(100);
        let entities = [collision_entity(9, position, 100)];
        let mut handler_classes = Vec::new();

        let outcome = fx.update_with_event_handlers(
            ParticleUpdateRequest::dry(0, 0).with_callbacks(ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &entities,
                    model_pool: &pool,
                }),
            }),
            |_, impact| {
                handler_classes.push(impact.source_particle_class);
                ParticleCollisionCacheRefresh::Unchanged
            },
            |_, _| ParticleTerrainResponse::Unhandled,
        );

        assert_eq!(
            handler_classes,
            vec![PRIMARY_BULLET_PARTICLE_CLASS, METEOR_SCATTER_CLASS]
        );
        assert_eq!(
            outcome
                .entity_impacts
                .iter()
                .map(|impact| impact.impact_position_argument_va)
                .collect::<Vec<_>>(),
            vec![
                retail_particle_impact_position_argument_va(primary_slot),
                retail_particle_impact_position_argument_va(ballistic_slot),
            ]
        );
        assert!(fx.particles.slots[primary_slot].is_none());
        assert!(fx.particles.slots[ballistic_slot].is_none());
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn whole_body_water_entry_maps_all_authored_classes_at_the_surface_with_owner() {
        let position_raw = [0x0c80, 0x0200, 0x2cc0];
        let surface_y_raw = -0x0336;
        let surface_position =
            raw_position_to_world([position_raw[0], surface_y_raw, position_raw[2]]);
        let owner_entity_id = 0x04be_0001;
        let expected_velocities = [
            raw_velocity_to_world([340, 122, -234]),
            raw_velocity_to_world([296, 250, 582]),
            raw_velocity_to_world([-276, 381, 281]),
            raw_velocity_to_world([186, 13, -239]),
        ];

        for (selector, expected_class) in WHOLE_BODY_WATER_ENTRY_CLASS_BY_SELECTOR
            .into_iter()
            .enumerate()
        {
            let mut fx = full_rate_world_fx();
            fx.emit_whole_body_water_entry_burst_raw(
                position_raw,
                surface_y_raw,
                selector as u8,
                owner_entity_id,
            );

            let particles = fx.test_particles_in_virgin_birth_order();
            assert_eq!(particles.len(), WHOLE_BODY_WATER_ENTRY_BURST_COUNT);
            assert_eq!(fx.direction_cursor, WHOLE_BODY_WATER_ENTRY_BURST_COUNT);
            for (particle, expected_velocity) in particles.iter().zip(expected_velocities) {
                assert_eq!(particle.source_class, expected_class);
                assert_eq!(particle.position, surface_position);
                assert_eq!(particle.owner_id, Some(owner_entity_id));
                assert_eq!(particle.velocity, expected_velocity);
            }
            fx.particles.assert_valid_topology();
        }
    }

    #[test]
    fn type60_entity_ring_materializes_with_exact_owner_and_is_removed_by_that_owner_only() {
        const RING_ENTITY_ID: u32 = 0x04be_6100;
        const OTHER_ENTITY_ID: u32 = 0x04be_6101;
        let position_raw = [-0x1234, 0x0200, 0x3456];
        let mut fx = WorldFx::new();

        fx.materialize_type60_exploding_ring_raw(
            RING_ENTITY_ID,
            position_raw,
            TYPE60_EXPLODING_RING_MODEL_ID,
            EXPLODING_RING_INITIAL_CONTROL_RAW,
        );

        assert_eq!(fx.exploding_rings().len(), 1);
        let ring = fx.exploding_rings()[0];
        assert_eq!(ring.model_id, TYPE60_EXPLODING_RING_MODEL_ID);
        assert_eq!(ring.position, raw_position_to_world(position_raw));
        assert_eq!(ring.associated_entity_id(), RING_ENTITY_ID);
        assert_eq!(
            ring.control_output_1_raw(),
            EXPLODING_RING_INITIAL_CONTROL_RAW
        );

        assert!(!fx.remove_type60_exploding_ring(OTHER_ENTITY_ID));
        assert_eq!(fx.exploding_rings(), &[ring]);
        assert!(fx.remove_type60_exploding_ring(RING_ENTITY_ID));
        assert!(fx.exploding_rings().is_empty());
        assert!(!fx.remove_type60_exploding_ring(RING_ENTITY_ID));
    }

    #[test]
    fn type60_initializer_fallback_ring_preserves_zero_control_output() {
        const FALLBACK_ENTITY_ID: u32 = 0x04be_6102;
        let mut fx = WorldFx::new();

        fx.materialize_type60_exploding_ring_raw(
            FALLBACK_ENTITY_ID,
            [0; 3],
            TYPE60_EXPLODING_RING_MODEL_ID,
            0,
        );

        let ring = fx.exploding_rings()[0];
        assert_eq!(ring.model_id, TYPE60_EXPLODING_RING_MODEL_ID);
        assert_eq!(ring.associated_entity_id(), FALLBACK_ENTITY_ID);
        assert_eq!(ring.control_output_1_raw(), 0);
        assert_eq!(ring.anim_vars().dynamic[1], 0);

        // A real actor owns this presentation. WorldFx must neither promote
        // the failed initializer to 0xffff nor autonomously delete it.
        fx.update(ParticleUpdateRequest::dry(20_000, 1));
        assert_eq!(fx.exploding_rings().len(), 1);
        assert_eq!(fx.exploding_rings()[0].control_output_1_raw(), 0);
    }

    #[test]
    fn type60_control_sync_requires_unique_owner_and_expected_prior_value() {
        const RING_ENTITY_ID: u32 = 0x04be_6103;
        const OTHER_ENTITY_ID: u32 = 0x04be_6104;
        let mut fx = WorldFx::new();
        fx.materialize_type60_exploding_ring_raw(
            RING_ENTITY_ID,
            [0x0200, 0x0300, 0x0400],
            HARD_WATER_ENTRY_SPLASH_MID_MODEL_ID,
            EXPLODING_RING_INITIAL_CONTROL_RAW,
        );

        assert!(!fx.set_type60_exploding_ring_control_raw(
            OTHER_ENTITY_ID,
            EXPLODING_RING_INITIAL_CONTROL_RAW,
            0xf97f,
        ));
        assert!(!fx.set_type60_exploding_ring_control_raw(RING_ENTITY_ID, 0, 0xf97f));
        assert_eq!(
            fx.exploding_rings()[0].control_output_1_raw(),
            EXPLODING_RING_INITIAL_CONTROL_RAW
        );
        assert!(fx.set_type60_exploding_ring_control_raw(
            RING_ENTITY_ID,
            EXPLODING_RING_INITIAL_CONTROL_RAW,
            0xf97f,
        ));

        let duplicate = fx.exploding_rings()[0];
        fx.exploding_rings.push(duplicate);
        assert!(!fx.set_type60_exploding_ring_control_raw(RING_ENTITY_ID, 0xf97f, 0,));
        assert!(!fx.remove_type60_exploding_ring(RING_ENTITY_ID));
        assert!(fx
            .exploding_rings()
            .iter()
            .all(|ring| ring.control_output_1_raw() == 0xf97f));
    }

    #[test]
    fn whole_body_water_entry_selector_seven_is_a_true_no_burst() {
        let mut fx = WorldFx::new();
        fx.emit_whole_body_water_entry_burst_raw([0x0400, 0, 0x0900], -0x0200, 7, 0x04be_0001);

        assert_eq!(fx.particle_count(), 0);
        assert_eq!(fx.direction_cursor, 0);
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn whole_body_water_entry_consumes_four_directions_when_allocation_rejects() {
        let mut fx = full_rate_world_fx();
        fx.particles
            .test_fill_to(MAX_WORLD_PARTICLES, pool_test_particle(16, 0.0));

        fx.emit_whole_body_water_entry_burst_raw([0x0400, 0, 0x0900], -0x0200, 6, 0x04be_0001);

        assert_eq!(fx.particle_count(), MAX_WORLD_PARTICLES);
        assert_eq!(fx.direction_cursor, WHOLE_BODY_WATER_ENTRY_BURST_COUNT);
        assert!(fx
            .particles
            .slots
            .iter()
            .flatten()
            .all(|particle| particle.source_class == 16));
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn mode_one_e230_family_comes_from_exact_descriptor_callbacks() {
        let classes = (0..PARTICLE_DESCRIPTORS.len() as u8)
            .filter(|source_class| uses_mode_one_surface_callback(*source_class))
            .collect::<Vec<_>>();
        assert_eq!(classes, vec![7, 8, 9, 10, 11, 33, 59, 70, 71, 72]);
        assert!(!uses_mode_one_surface_callback(
            CARGO_TRANSFER_PARTICLE_CLASS
        ));
    }

    #[test]
    fn later_world_surface_selectors_use_the_retail_particle_classes() {
        assert_eq!(
            SURFACE_RESPONSE_PARTICLE_CLASS_BY_SELECTOR,
            [7, 8, 9, 10, 7, 11, 0, 0, 59, 10, 71, 70, 72]
        );

        for (selector, expected_class) in [(8, 59), (9, 10), (10, 71), (11, 70), (12, 72)] {
            let mut direction_cursor = 0;
            let burst = default_surface_response_burst(
                [1.0, 2.0, 3.0],
                selector,
                &mut direction_cursor,
                SURFACE_RESPONSE_BURST_COUNT,
            );

            assert_eq!(burst.particles.len(), SURFACE_RESPONSE_BURST_COUNT);
            assert_eq!(direction_cursor, SURFACE_RESPONSE_BURST_COUNT);
            assert!(burst
                .particles
                .iter()
                .all(|particle| particle.source_class == expected_class));
        }
    }

    #[test]
    fn mode_one_non_six_ground_response_snaps_and_bounces_without_deleting() {
        let mut terrain = flat_terrain(0, 5);
        // The current cell remains at raw Y=0, while its other three corners
        // raise the interpolated collision plane. This distinguishes
        // FUN_00440120's bilinear contact test from FUN_0043DB60's later
        // coarse response/snap height.
        terrain_cell_mut(&mut terrain, 5, 5).height = 8;
        terrain_cell_mut(&mut terrain, 4, 6).height = 8;
        terrain_cell_mut(&mut terrain, 5, 6).height = 8;
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [5; 8],
            water_response_selectors: [5; 8],
        };
        let mut fx = WorldFx::new();
        fx.queue(WorldEvent::ParticleBurst(ParticleBurst {
            origin: [4.75, 0.5, 5.75],
            particles: vec![surface_response_particle(7, [0, -5, 0])],
        }));
        fx.process_pending();

        let radius = f32::from(fx.particles[0].collision_radius_raw) / 256.0;
        assert!(terrain.height_at(4.75, 5.75) >= 0.5 - radius);

        fx.update(ParticleUpdateRequest::terrain(0, 0, context));

        let particle = fx
            .particles
            .slots
            .iter()
            .flatten()
            .find(|particle| particle.source_class == 7)
            .copied()
            .expect("non-six E230 response keeps its parent");
        assert_eq!(particle.position[1], 0.0);
        assert_eq!(particle.velocity[1], raw_velocity_to_world([0, 2, 0])[1]);
        assert!(fx.take_positional_sounds().is_empty());
    }

    #[test]
    fn mode_one_non_six_water_response_continues_into_ground_dispatch() {
        let mut terrain = flat_terrain(0, 6);
        // Keep the sea just one raw unit above terrain so the water response
        // snaps the particle sphere into the ground collision band.
        terrain.header[0] = 0x100;
        terrain_cell_mut(&mut terrain, 5, 5).height = 8;
        terrain_cell_mut(&mut terrain, 4, 6).height = 8;
        terrain_cell_mut(&mut terrain, 5, 6).height = 8;
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [6; 8],
            water_response_selectors: [5; 8],
        };
        let x = 4.75;
        let z = 5.75;
        let water_y = displaced_water_surface(context, [x, 0.0, z], 0).unwrap();
        let mut parent = world_particle(
            [x, water_y - 0.01, z],
            surface_response_particle(7, [0, -8, 0]),
        );
        parent.step_start_water_state = 2;
        parent.water_state = 1;
        parent.water_state_initialized = true;

        let mut fx = WorldFx::new();
        fx.particles.test_allocate(parent);
        let slot = fx.particles.test_slot_at(0);
        let mut parent = fx.particles.slots[slot].unwrap();
        let radius = f32::from(parent.collision_radius_raw) / 256.0;
        assert!(water_y - radius <= terrain.height_at(x, z));

        let keep = fx.dispatch_mode_one_surface_collision(
            slot,
            &mut parent,
            context,
            ParticleBirthContext {
                environment: ParticleEnvironment::Terrain(context),
                retail_tick: 0,
            },
        );

        assert!(
            !keep,
            "ground selector six must delete after the water bounce"
        );
        assert_eq!(
            fx.particles
                .slots
                .iter()
                .flatten()
                .filter(|particle| particle.source_class == WATER_SPLASH_PARTICLE_CLASS)
                .count(),
            1
        );
    }

    #[test]
    fn mode_one_selector_six_water_response_short_circuits_ground_dispatch() {
        let mut terrain = flat_terrain(-1, 5);
        terrain.header[0] = 0;
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [5; 8],
            water_response_selectors: [6; 8],
        };
        let x = 4.0;
        let z = 5.0;
        let water_y = displaced_water_surface(context, [x, 0.0, z], 0).unwrap();
        let endpoint_y = water_y - 0.01;
        let mut parent =
            world_particle([x, endpoint_y, z], surface_response_particle(7, [0, -8, 0]));
        parent.step_start_water_state = 2;
        parent.water_state = 1;
        parent.water_state_initialized = true;

        let mut fx = WorldFx::new();
        fx.particles.test_allocate(parent);
        let slot = fx.particles.test_slot_at(0);
        let mut parent = fx.particles.slots[slot].unwrap();
        let seed = (0..u32::MAX)
            .find(|seed| {
                let mut state = *seed;
                retail_random_u16(&mut state) & 3 != 0
            })
            .unwrap();
        fx.rng_state = seed;
        let mut expected_rng = seed;
        assert_ne!(retail_random_u16(&mut expected_rng) & 3, 0);

        let keep = fx.dispatch_mode_one_surface_collision(
            slot,
            &mut parent,
            context,
            ParticleBirthContext {
                environment: ParticleEnvironment::Terrain(context),
                retail_tick: 0,
            },
        );

        assert!(!keep);
        assert_eq!(parent.position[1], endpoint_y);
        assert_eq!(fx.rng_state, expected_rng);
        assert!(fx.take_positional_sounds().is_empty());
        assert_eq!(
            fx.particles
                .slots
                .iter()
                .flatten()
                .filter(|particle| particle.source_class == WATER_SPLASH_PARTICLE_CLASS)
                .count(),
            1
        );
    }

    #[test]
    fn later_slot_mode_one_selector_six_cascades_with_exact_rng_and_audio() {
        let terrain = flat_terrain(0, 6);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [6; 8],
            water_response_selectors: [6; 8],
        };
        let class_seven = world_particle([4.0, 0.0, 5.0], surface_response_particle(7, [0, -8, 0]));
        let mut fx = WorldFx::new();
        fx.particles
            .test_fill_to(MAX_WORLD_PARTICLES - 1, pool_test_particle(31, 0.0));
        fx.particles.test_allocate(class_seven);
        assert_eq!(fx.particles.slots[0].unwrap().source_class, 7);
        assert!(fx.particles.free(100));

        let seed = (0..u32::MAX)
            .find(|seed| {
                let mut state = *seed;
                retail_random_u16(&mut state) & 3 == 0
            })
            .expect("small seed for the E230 one-in-four sound gate");
        fx.rng_state = seed;
        let mut expected_rng = seed;
        assert_eq!(retail_random_u16(&mut expected_rng) & 3, 0);
        let sound_roll = retail_random_u16(&mut expected_rng);
        let expected_sound = if sound_roll & 0x1000 != 0 {
            SPECIAL_SURFACE_SOUND_IDS[0]
        } else {
            SPECIAL_SURFACE_SOUND_IDS[1]
        };
        let expected_rate =
            SPECIAL_SURFACE_RATE_MIN_16_16 + u32::from(retail_random_u16(&mut expected_rng) >> 2);

        fx.update(ParticleUpdateRequest::terrain(0, 0, context));

        assert_eq!(fx.rng_state, expected_rng);
        assert!(fx.particles.slots[0].is_none());
        let cascade = fx.particles.slots[100]
            .expect("selector-six cascade allocates into the later free slot");
        assert_eq!(cascade.source_class, WATER_SPLASH_PARTICLE_CLASS);
        assert_eq!(cascade.position, [4.0, 0.0, 5.0]);
        assert_eq!(cascade.step_start_water_state, cascade.water_state);
        assert!(cascade.water_state_initialized);
        assert_eq!(
            fx.take_positional_sounds(),
            vec![PositionalSoundEvent {
                sound_id: expected_sound,
                position: [4.0, 0.0, 5.0],
                frequency_q16: expected_rate,
            }]
        );
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn inline_trail_uses_birth_tick_water_state_not_previous_tick_wave() {
        let mut terrain = flat_terrain(-128, 0);
        terrain.header[0] = 0;
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [0; 8],
            water_response_selectors: [0; 8],
        };
        let x = 40.0;
        let z = 72.0;
        let radius = f32::from(METEOR_TRAIL_RADIUS_RAW) / 256.0;
        let (previous_tick, retail_tick, y, previous_state, birth_state) = (0..100_u32)
            .find_map(|previous_tick| {
                let retail_tick = previous_tick + 1;
                let previous_surface =
                    displaced_water_surface(context, [x, 0.0, z], previous_tick).unwrap();
                let birth_surface =
                    displaced_water_surface(context, [x, 0.0, z], retail_tick).unwrap();
                let y = (previous_surface + birth_surface) * 0.5 + radius;
                let previous_state = classify_particle_water(
                    Some(context),
                    Some(0.0),
                    [x, y, z],
                    radius,
                    previous_tick,
                );
                let birth_state = classify_particle_water(
                    Some(context),
                    Some(0.0),
                    [x, y, z],
                    radius,
                    retail_tick,
                );
                (y > 0.0 && previous_state != birth_state).then_some((
                    previous_tick,
                    retail_tick,
                    y,
                    previous_state,
                    birth_state,
                ))
            })
            .expect("adjacent retail ticks move this above-flat-sea wave sample");

        let mut parent = world_particle([x, y, z], meteor_impact_burst([x, y, z], 0).particles[0]);
        parent.position = [x, y, z];
        parent.velocity = [0.0; 3];
        let mut fx = WorldFx::new();
        fx.particles
            .test_fill_to(MAX_WORLD_PARTICLES - 1, pool_test_particle(31, 0.0));
        fx.particles.test_allocate(parent);
        assert!(fx.particles.free(100));
        fx.last_retail_tick = Some(previous_tick);

        fx.update(ParticleUpdateRequest::terrain(1, retail_tick, context));

        let trail = fx.particles.slots[100]
            .expect("class-16 callback allocates its trail into the later slot");
        assert_eq!(trail.source_class, METEOR_TRAIL_CLASS);
        assert_eq!(trail.step_start_water_state, birth_state);
        assert_ne!(trail.step_start_water_state, previous_state);
    }

    #[test]
    fn primary_owner_is_ignored_through_tick_50_but_eligible_afterward() {
        let owner = collision_entity(7, [4.0, 2.0, 5.0], 1_000);
        let pool = solid_sphere_pool(1_000);
        let mut fx = WorldFx::new();
        fx.queue_primary_fire_batch(&[primary_event(2.0, [0.0; 3])], None);
        fx.process_pending();
        assert!(fx
            .update(
                ParticleUpdateRequest::dry(0, 0).with_callbacks(ParticleCallbackContext {
                    owner_motions: &[],
                    collision: Some(ParticleCollisionContext {
                        entities: std::slice::from_ref(&owner),
                        model_pool: &pool,
                    }),
                }),
            )
            .entity_impacts
            .is_empty());
        let bullet = fx
            .particles
            .test_iter_mut_in_physical_order()
            .find(|particle| particle.source_class == PRIMARY_BULLET_PARTICLE_CLASS)
            .unwrap();
        bullet.age_ticks = 51.0;
        bullet.step_start_position = bullet.position;
        let entities = [owner];
        let impacts = fx
            .update(
                ParticleUpdateRequest::dry(0, 0).with_callbacks(ParticleCallbackContext {
                    owner_motions: &[],
                    collision: Some(ParticleCollisionContext {
                        entities: &entities,
                        model_pool: &pool,
                    }),
                }),
            )
            .entity_impacts;
        assert!(matches!(
            impacts.as_slice(),
            [ParticleEntityImpact {
                target_entity_id: 7,
                ..
            }]
        ));
    }

    #[test]
    fn exact_model_miss_falls_through_but_unsupported_program_keeps_broad_fallback() {
        let entity = collision_entity(9, [4.0, 2.0, 5.0], 1_000);
        let exact_miss = TestModelPool(vec![collision_model(
            vec![[0, 500, 0, 0]],
            vec![0x8E, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0x88, 0],
            1_000,
        )]);
        assert_eq!(
            retail_swept_model_hit(
                [4.0, 2.0, 5.0],
                [4.0, 2.0, 5.0],
                PRIMARY_BULLET_RADIUS_RAW,
                Some(7),
                0.0,
                std::slice::from_ref(&entity),
                &exact_miss,
            ),
            None
        );

        let unsupported = TestModelPool(vec![collision_model(
            Vec::new(),
            vec![0x87, 0, 0, 0],
            1_000,
        )]);
        assert_eq!(
            retail_swept_model_hit(
                [4.0, 2.0, 5.0],
                [4.0, 2.0, 5.0],
                PRIMARY_BULLET_RADIUS_RAW,
                Some(7),
                0.0,
                &[entity],
                &unsupported,
            ),
            Some(9)
        );
    }

    #[test]
    fn failed_refined_halves_cancel_a_coarse_collision() {
        let pool = TestModelPool(vec![collision_model(
            vec![[0, 0, 0, 0]],
            vec![0x8E, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x88, 0],
            2_000,
        )]);
        let entity = collision_entity(9, [4.0, 2.0, 5.0], 2_000);
        // The radius-zero authored point is inside the initial midpoint probe.
        // Both quarter probes only touch it, and retail rejects a final result
        // with less than one raw unit of penetration.
        assert_eq!(
            retail_swept_model_hit(
                [0.0, 2.0, 5.0],
                [8.0, 2.0, 5.0],
                PRIMARY_BULLET_RADIUS_RAW,
                Some(7),
                0.0,
                &[entity],
                &pool,
            ),
            None
        );
    }

    #[test]
    fn raw_half_vector_truncates_negative_components_toward_zero() {
        assert_eq!(raw_vector_half([-3, 3, -2]), [-1, 1, -1]);
    }

    #[test]
    fn static_tile_lattice_keeps_unaligned_phase_and_x_then_z_order() {
        let probe = [0x1234, 0, 0x5678];
        let mut primary_cells = Vec::new();
        let _: Option<()> = scan_static_tile_lattice(probe, 44, |x, z| {
            primary_cells.push([x, z]);
            None
        });
        assert_eq!(primary_cells, vec![[0x1208, 0x564c]]);

        let mut wider_cells = Vec::new();
        let _: Option<()> = scan_static_tile_lattice(probe, 200, |x, z| {
            wider_cells.push([x, z]);
            None
        });
        assert_eq!(
            wider_cells,
            vec![
                [0x116c, 0x55b0],
                [0x116c, 0x56b0],
                [0x126c, 0x55b0],
                [0x126c, 0x56b0],
            ]
        );
    }

    #[test]
    fn static_tile_center_uses_0x7f_signed_corner_average_and_tick_channel_zero() {
        let mut terrain = flat_terrain(0, 0);
        terrain_cell_mut(&mut terrain, 255, 255).height = (-8_i8) as u8;
        terrain_cell_mut(&mut terrain, 0, 255).height = 4;
        terrain_cell_mut(&mut terrain, 255, 0).height = 8;
        terrain_cell_mut(&mut terrain, 0, 0).height = (-20_i8) as u8;
        // (-8 + 4 + 8 - 20) * 32 / 4 = -128 raw.
        assert_eq!(
            static_tile_center_raw(&terrain, 255, 255),
            [-129, -128, -129]
        );

        let vars = static_collision_anim_vars(65_538);
        assert_eq!(vars.dynamic[0], 2);
        assert!(vars.dynamic[1..].iter().all(|&value| value == 0));
    }

    #[test]
    fn static_tile_refinement_preserves_all_three_retail_branches_and_radius_padding() {
        let endpoint = [1_000, 0, 0];
        let movement = [160, 0, 0];

        let mut backward_miss_calls = Vec::new();
        let backward_miss = refine_static_tile_sweep(
            endpoint,
            movement,
            160,
            PRIMARY_BULLET_RADIUS_RAW as i16,
            |probe, radius| {
                backward_miss_calls.push((probe, radius));
                (backward_miss_calls.len() == 1).then_some(1_u8)
            },
        );
        assert_eq!(backward_miss, Some(([880, 0, 0], 1)));
        assert_eq!(
            backward_miss_calls,
            vec![([920, 0, 0], 84), ([880, 0, 0], 44)]
        );

        let mut both_hit_calls = Vec::new();
        let both_hit = refine_static_tile_sweep(
            endpoint,
            movement,
            160,
            PRIMARY_BULLET_RADIUS_RAW as i16,
            |probe, radius| {
                both_hit_calls.push((probe, radius));
                Some(both_hit_calls.len() as u8)
            },
        );
        assert_eq!(both_hit, Some(([960, 0, 0], 3)));
        assert_eq!(
            both_hit_calls,
            vec![([920, 0, 0], 84), ([880, 0, 0], 44), ([960, 0, 0], 44),]
        );

        let mut forward_miss_count = 0;
        let forward_miss = refine_static_tile_sweep(
            endpoint,
            movement,
            160,
            PRIMARY_BULLET_RADIUS_RAW as i16,
            |_, _| {
                forward_miss_count += 1;
                (forward_miss_count < 3).then_some(forward_miss_count)
            },
        );
        assert_eq!(forward_miss, None);
        assert_eq!(forward_miss_count, 3);
    }

    #[test]
    fn static_tile_refinement_narrows_segment_length_to_signed_word_before_clamping() {
        let mut largest_positive_radius = None;
        assert_eq!(
            refine_static_tile_sweep(
                [0; 3],
                [0; 3],
                i16::MAX,
                PRIMARY_BULLET_RADIUS_RAW as i16,
                |_, radius| {
                    largest_positive_radius = Some(radius);
                    None::<u8>
                },
            ),
            None
        );
        assert_eq!(largest_positive_radius, Some(16_383 + 4));

        let mut wrapped_negative_radius = None;
        assert_eq!(
            refine_static_tile_sweep(
                [0; 3],
                [0; 3],
                32_768_u16 as i16,
                PRIMARY_BULLET_RADIUS_RAW as i16,
                |_, radius| {
                    wrapped_negative_radius = Some(radius);
                    None::<u8>
                },
            ),
            None
        );
        assert_eq!(wrapped_negative_radius, Some(PRIMARY_BULLET_RADIUS_RAW + 4));
    }

    #[test]
    fn primary_static_tile_hit_precedes_heightfield_and_uses_refined_entity_style_burst() {
        let mut terrain = flat_terrain(16, 0);
        *terrain_cell_mut(&mut terrain, 3, 4) = TerrainCell {
            height: 16,
            attribute: 1,
            terrain_type: 0x18,
        };
        let objects = terrain_object_table(1, [0, 0, 0, 0], 29);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: Some(&objects),
            ground_response_selectors: [5; 8],
            water_response_selectors: [7; 8],
        };
        let pool = solid_sphere_pool(200);
        let mut fx = WorldFx::new();
        fx.queue_primary_fire_batch(&[primary_event(2.0, [0.0; 3])], None);
        fx.process_pending();
        fx.take_positional_sounds();

        let impacts = fx
            .update(
                ParticleUpdateRequest::terrain(0, 0, context).with_callbacks(
                    ParticleCallbackContext {
                        owner_motions: &[],
                        collision: Some(ParticleCollisionContext {
                            entities: &[],
                            model_pool: &pool,
                        }),
                    },
                ),
            )
            .primary_impacts;
        assert_eq!(
            impacts,
            vec![PrimaryImpact::StaticTile(ParticleStaticImpact {
                source_particle_class: PRIMARY_BULLET_PARTICLE_CLASS,
                source_owner_id: Some(7),
                position_world: [4.0, 2.0, 5.0],
                cell: [3, 4],
                attribute: 1,
                terrain_type: 0x18,
                model_id: 0,
                kind_index: 29,
                current: Some(CurrentStaticDamageSnapshot {
                    target: crate::static_damage::StaticDamageTarget {
                        cell: [3, 4],
                        state: crate::static_damage::StaticDamageTargetState {
                            kind_index: 29,
                            terrain_type: 0x18,
                            terrain_height_byte: 16,
                            collision_radius_raw: 200,
                            effect_extent_raw: 0,
                        },
                    },
                    attribute: 1,
                    model_id: 0,
                }),
                damage: Some(BallisticDamageRequest {
                    packet: PRIMARY_PROJECTILE_DAMAGE_PACKET,
                    source_entity_type_at_birth: Some(46),
                    source_owner_id: Some(7),
                }),
            })]
        );
        assert!(!fx
            .test_particles_in_virgin_birth_order()
            .iter()
            .any(|particle| particle.source_class == PRIMARY_BULLET_PARTICLE_CLASS));
        assert!(fx
            .test_particles_in_virgin_birth_order()
            .iter()
            .any(|particle| particle.source_class == PRIMARY_ABOVE_WATER_IMPACT_CLASS));
        assert!(!fx
            .test_particles_in_virgin_birth_order()
            .iter()
            .any(|particle| particle.source_class == 11));
        assert!(fx.take_positional_sounds().is_empty());
    }

    #[test]
    fn upgraded_primary_static_f800_uses_its_identical_packet_record_inline() {
        let mut terrain = flat_terrain(16, 0);
        *terrain_cell_mut(&mut terrain, 3, 4) = TerrainCell {
            height: 16,
            attribute: 1,
            terrain_type: 0x10,
        };
        let objects = terrain_object_table(1, [0; 4], KIND_9_STATIC_OBJECT);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: Some(&objects),
            ground_response_selectors: [0; 8],
            water_response_selectors: [0; 8],
        };
        let pool = solid_sphere_pool(200);
        let mut parent = descriptor_test_particle(
            UPGRADED_PRIMARY_PARTICLE_CLASS,
            [4.0, 2.0, 5.0],
            [1_000, 0, 0],
            Some(7),
        );
        parent.source_entity_type_at_birth = Some(46);
        let mut fx = WorldFx::new();
        let parent_slot = fx.particles.allocate(parent).unwrap();
        let mut handled = None;
        let mut handler_saw_parent_and_visual = false;

        let outcome = fx.update_with_event_handlers(
            ParticleUpdateRequest::terrain(0, 0, context).with_callbacks(ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &[],
                    model_pool: &pool,
                }),
            }),
            |_, impact| panic!("unexpected entity impact: {impact:?}"),
            |live_fx, impact| {
                let impact = expect_static_event(impact);
                handler_saw_parent_and_visual =
                    live_fx.particles.slots[parent_slot].is_some_and(|particle| {
                        particle.source_class == UPGRADED_PRIMARY_PARTICLE_CLASS
                    }) && live_fx
                        .particles
                        .slots
                        .iter()
                        .flatten()
                        .any(|particle| particle.source_class == PRIMARY_ABOVE_WATER_IMPACT_CLASS);
                handled = Some(impact);
                ParticleTerrainResponse::Unhandled
            },
        );

        assert!(handler_saw_parent_and_visual);
        let impact = handled.expect("class-3 F800 handler");
        assert_eq!(
            impact.source_particle_class,
            UPGRADED_PRIMARY_PARTICLE_CLASS
        );
        assert_eq!(impact.source_owner_id, Some(7));
        assert_eq!(
            impact.current.unwrap().target.state.kind_index,
            KIND_9_STATIC_OBJECT
        );
        assert_eq!(
            impact.damage,
            Some(BallisticDamageRequest {
                packet: PRIMARY_PROJECTILE_DAMAGE_PACKET,
                source_entity_type_at_birth: Some(46),
                source_owner_id: Some(7),
            })
        );
        assert_eq!(outcome.primary_impacts, [PrimaryImpact::StaticTile(impact)]);
        assert!(fx.particles.slots[parent_slot].is_none());
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn primary_f800_suppression_preserves_visual_and_delete_without_static_damage() {
        let mut terrain = flat_terrain(16, 0);
        *terrain_cell_mut(&mut terrain, 3, 4) = TerrainCell {
            height: 16,
            attribute: 1,
            terrain_type: 0x18,
        };
        let objects = terrain_object_table(1, [0, 0, 0, 0], 29);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: Some(&objects),
            ground_response_selectors: [5; 8],
            water_response_selectors: [7; 8],
        };
        let pool = solid_sphere_pool(200);
        let mut fx = WorldFx::new();
        fx.queue_primary_fire_batch(&[primary_event(2.0, [0.0; 3])], None);
        fx.process_pending();
        fx.take_positional_sounds();
        let parent_slot = fx
            .particles
            .slots
            .iter()
            .position(|particle| {
                particle
                    .is_some_and(|particle| particle.source_class == PRIMARY_BULLET_PARTICLE_CLASS)
            })
            .unwrap();
        fx.particles.slots[parent_slot]
            .as_mut()
            .unwrap()
            .suppresses_impact_damage = true;
        let mut handled = None;
        let mut handler_saw_parent_and_visual = false;

        let outcome = fx.update_with_event_handlers(
            ParticleUpdateRequest::terrain(0, 0, context).with_callbacks(ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &[],
                    model_pool: &pool,
                }),
            }),
            |_, impact| panic!("unexpected entity impact: {impact:?}"),
            |live_fx, impact| {
                let impact = expect_static_event(impact);
                handler_saw_parent_and_visual = live_fx.particles.slots[parent_slot]
                    .is_some_and(|particle| particle.source_class == PRIMARY_BULLET_PARTICLE_CLASS)
                    && live_fx.particles.slots.iter().flatten().any(|particle| {
                        particle.source_class == PRIMARY_ABOVE_WATER_IMPACT_CLASS
                            && particle.suppresses_impact_damage
                    });
                handled = Some(impact);
                ParticleTerrainResponse::Unhandled
            },
        );

        assert!(handler_saw_parent_and_visual);
        assert!(matches!(
            outcome.primary_impacts.as_slice(),
            [PrimaryImpact::StaticTile(ParticleStaticImpact {
                damage: None,
                current: None,
                ..
            })]
        ));
        assert_eq!(
            handled,
            outcome
                .primary_impacts
                .first()
                .and_then(|impact| match impact {
                    PrimaryImpact::StaticTile(impact) => Some(*impact),
                    PrimaryImpact::WaterEntry { .. } | PrimaryImpact::Terrain { .. } => None,
                })
        );
        assert!(fx.particles.slots[parent_slot].is_none());
        let visuals = fx
            .particles
            .slots
            .iter()
            .flatten()
            .filter(|particle| particle.source_class == PRIMARY_ABOVE_WATER_IMPACT_CLASS)
            .collect::<Vec<_>>();
        assert_eq!(visuals.len(), 1);
        assert!(visuals[0].suppresses_impact_damage);
    }

    #[test]
    fn primary_f800_submits_kind9_inline_and_deduplicates_the_later_physical_slot() {
        let mut terrain = flat_terrain(16, 0);
        *terrain_cell_mut(&mut terrain, 3, 4) = TerrainCell {
            height: 16,
            attribute: 1,
            terrain_type: 0x10,
        };
        let objects = terrain_object_table(1, [0; 4], KIND_9_STATIC_OBJECT);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: Some(&objects),
            ground_response_selectors: [0; 8],
            water_response_selectors: [0; 8],
        };
        let pool = solid_sphere_pool(200);
        let mut fx = WorldFx::new();
        let mut later = descriptor_test_particle(
            PRIMARY_BULLET_PARTICLE_CLASS,
            [4.0, 2.0, 5.0],
            [1_000, 0, 0],
            Some(22),
        );
        later.source_entity_type_at_birth = Some(46);
        let later_slot = fx.particles.allocate(later).unwrap();
        let mut earlier = descriptor_test_particle(
            PRIMARY_BULLET_PARTICLE_CLASS,
            [4.0, 2.0, 5.0],
            [1_000, 0, 0],
            Some(11),
        );
        earlier.source_entity_type_at_birth = Some(46);
        let earlier_slot = fx.particles.allocate(earlier).unwrap();
        assert!(earlier_slot < later_slot);

        let mut scheduler = StaticDamageScheduler::new();
        let mut handled_owners = Vec::new();
        let mut submissions = Vec::new();
        let rng_before = fx.rng_state;
        let outcome = fx.update_with_event_handlers(
            ParticleUpdateRequest::terrain(0, 0, context).with_callbacks(ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &[],
                    model_pool: &pool,
                }),
            }),
            |_, impact| panic!("unexpected entity impact: {impact:?}"),
            |live_fx, impact| {
                let impact = expect_static_event(impact);
                let source_owner_id = impact.source_owner_id.unwrap();
                let parent_slot = if source_owner_id == 11 {
                    earlier_slot
                } else {
                    later_slot
                };
                assert!(
                    live_fx.particles.slots[parent_slot].is_some_and(|particle| {
                        particle.source_class == PRIMARY_BULLET_PARTICLE_CLASS
                    })
                );
                assert_eq!(
                    live_fx
                        .particles
                        .slots
                        .iter()
                        .flatten()
                        .filter(|particle| {
                            particle.source_class == PRIMARY_ABOVE_WATER_IMPACT_CLASS
                        })
                        .count(),
                    submissions.len() + 1,
                    "F610 must materialize before the inline F800 handler"
                );
                let current = impact.current.expect("unsuppressed live F800 target");
                assert_eq!(current.target.state.kind_index, KIND_9_STATIC_OBJECT);
                handled_owners.push(source_owner_id);
                submissions.push(scheduler.submit_hit(
                    current.target,
                    impact.damage.unwrap().packet,
                    &mut || live_fx.next_shared_retail_random_u16(),
                ));
                ParticleTerrainResponse::Unhandled
            },
        );

        assert_eq!(handled_owners, [11, 22]);
        assert_eq!(
            submissions,
            [
                StaticDamageOutcome::Started {
                    severity_raw: 2_000,
                    sample: None,
                },
                StaticDamageOutcome::Duplicate {
                    severity_raw: 2_000,
                    sample: None,
                },
            ]
        );
        assert_eq!(scheduler.active_program_count(), 1);
        assert_eq!(outcome.primary_impacts.len(), 2);
        assert_eq!(fx.rng_state, rng_before);
        assert!(fx.particles.slots[earlier_slot].is_none());
        assert!(fx.particles.slots[later_slot].is_none());
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn earlier_cleansing_contact_changes_later_static_model_selection_in_same_pool_pass() {
        let mut terrain = flat_terrain(16, 0);
        *terrain_cell_mut(&mut terrain, 3, 4) = TerrainCell {
            height: 16,
            attribute: 1,
            terrain_type: 0x18,
        };
        let objects = terrain_object_table(1, [0, 0, 1, 1], 29);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: Some(&objects),
            ground_response_selectors: [0; 8],
            water_response_selectors: [0; 8],
        };
        let mut pool = solid_sphere_pool(200);
        let infected_variant = pool.0[0].clone();
        pool.0.push(infected_variant);

        let mut fx = WorldFx::new();
        fx.queue_primary_fire_batch(&[primary_event(2.0, [0.0; 3])], None);
        fx.process_pending();
        fx.take_positional_sounds();
        // The primary pair receives the high virgin slots. This later
        // allocation puts class 6 below them, so its synchronous retail clear
        // must be visible when the traversal reaches the bullet and chooses
        // the static model.
        fx.particles.test_allocate(descriptor_test_particle(
            TerrainContactMode::Cleanse.particle_class(),
            [3.5, 2.0, 4.5],
            [0; 3],
            None,
        ));

        let outcome = fx.update(
            ParticleUpdateRequest::terrain(0, 0, context).with_callbacks(ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &[],
                    model_pool: &pool,
                }),
            }),
        );

        assert_eq!(
            outcome.terrain_type_mutations,
            [ParticleTerrainMutation::Infection {
                cell: [3, 4],
                infected: false,
            }]
        );
        assert!(
            matches!(
                outcome.primary_impacts.as_slice(),
                [PrimaryImpact::StaticTile(ParticleStaticImpact {
                    cell: [3, 4],
                    terrain_type: 0x08,
                    model_id: 0,
                    ..
                })]
            ),
            "unexpected same-pass static impact: {:?}",
            outcome.primary_impacts
        );
        let PrimaryImpact::StaticTile(static_impact) = outcome.primary_impacts[0] else {
            unreachable!()
        };
        assert_eq!(
            static_impact.current.unwrap().target.state.terrain_type,
            0x08,
            "F800 must resolve through the earlier infection-write prefix"
        );
    }

    #[test]
    fn later_cleansing_contact_does_not_retroactively_change_earlier_f800_resolution() {
        let mut terrain = flat_terrain(16, 0);
        *terrain_cell_mut(&mut terrain, 3, 4) = TerrainCell {
            height: 16,
            attribute: 1,
            terrain_type: 0x18,
        };
        let objects = terrain_object_table(1, [0, 0, 1, 1], KIND_9_STATIC_OBJECT);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: Some(&objects),
            ground_response_selectors: [0; 8],
            water_response_selectors: [0; 8],
        };
        let mut pool = solid_sphere_pool(200);
        pool.0.push(pool.0[0].clone());

        let mut fx = WorldFx::new();
        // The first allocation takes the highest physical slot. The primary
        // pair allocated afterward therefore visits F800 before this cleanser
        // publishes its infection-bit clear.
        fx.particles.test_allocate(descriptor_test_particle(
            TerrainContactMode::Cleanse.particle_class(),
            [3.5, 2.0, 4.5],
            [0; 3],
            None,
        ));
        fx.queue_primary_fire_batch(&[primary_event(2.0, [0.0; 3])], None);
        fx.process_pending();
        fx.take_positional_sounds();
        let mut handled = Vec::new();

        let outcome = fx.update_with_event_handlers(
            ParticleUpdateRequest::terrain(0, 0, context).with_callbacks(ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &[],
                    model_pool: &pool,
                }),
            }),
            |_, impact| panic!("unexpected entity impact: {impact:?}"),
            |_, event| {
                handled.push(expect_static_event(event));
                ParticleTerrainResponse::Unhandled
            },
        );

        assert_eq!(
            outcome.terrain_type_mutations,
            [ParticleTerrainMutation::Infection {
                cell: [3, 4],
                infected: false,
            }]
        );
        assert_eq!(handled.len(), 1);
        assert_eq!(handled[0].terrain_type, 0x18);
        assert_eq!(handled[0].model_id, 1);
        assert_eq!(
            handled[0].current.unwrap().target.state.terrain_type,
            0x18,
            "a later physical slot must not affect an earlier F800 re-read"
        );
        assert_eq!(handled[0].current.unwrap().model_id, 1);
    }

    #[test]
    fn entity_hit_at_or_below_sea_spawns_exact_class_42_bubble() {
        let mut fx = WorldFx::new();
        fx.queue_primary_fire_batch(&[primary_event(-4.5, [0.0; 3])], Some(-4.5));
        fx.process_pending();
        let pool = solid_sphere_pool(100);
        let entities = [collision_entity(9, [4.0, -4.5, 5.0], 100)];
        let impacts = fx
            .update(
                ParticleUpdateRequest::flat_water(0, 0, -4.5).with_callbacks(
                    ParticleCallbackContext {
                        owner_motions: &[],
                        collision: Some(ParticleCollisionContext {
                            entities: &entities,
                            model_pool: &pool,
                        }),
                    },
                ),
            )
            .entity_impacts;
        assert_eq!(impacts.len(), 1);
        let bubble = fx
            .test_particles_in_virgin_birth_order()
            .into_iter()
            .find(|particle| particle.source_class == PRIMARY_UNDERWATER_IMPACT_CLASS)
            .unwrap();
        assert_eq!(bubble.current_sprite_id(), 787);
        assert_eq!(bubble.lifetime_ticks, 64);
        assert_eq!(bubble.collision_radius_raw, 80);
        assert_eq!(bubble.draw_scale_raw, 0x0100);
        assert_eq!(
            bubble.velocity,
            raw_velocity_to_world(PRIMARY_UNDERWATER_IMPACT_INITIAL_VELOCITY_RAW)
        );
    }

    #[test]
    fn class_34_random_walk_and_class_42_rise_match_retail_callbacks() {
        let mut fx = WorldFx::new();
        fx.queue(WorldEvent::ParticleBurst(primary_entity_impact_burst(
            [1.0, 2.0, 3.0],
            Some(0.0),
        )));
        fx.queue(WorldEvent::ParticleBurst(primary_entity_impact_burst(
            [1.0, -2.0, 3.0],
            Some(0.0),
        )));
        fx.process_pending();
        let before_above = fx.test_particles_in_virgin_birth_order()[0];
        let before_below = fx.test_particles_in_virgin_birth_order()[1];

        let mut expected_rng = 0;
        let y_raw = i32::from(retail_random_u16(&mut expected_rng) % 10) - 5;
        let x_raw = i32::from(retail_random_u16(&mut expected_rng) & 7) - 4;
        let z_raw = i32::from(retail_random_u16(&mut expected_rng) & 3) - 2;
        fx.update(ParticleUpdateRequest::flat_water(20_000, 1, 0.0));

        let above = fx
            .test_particles_in_virgin_birth_order()
            .into_iter()
            .find(|particle| particle.source_class == PRIMARY_ABOVE_WATER_IMPACT_CLASS)
            .unwrap();
        let increment = raw_velocity_to_world([x_raw, y_raw, z_raw]);
        for axis in 0..3 {
            assert!(
                (above.velocity[axis] - before_above.velocity[axis] - increment[axis]).abs() < 1e-6
            );
        }
        let below = fx
            .test_particles_in_virgin_birth_order()
            .into_iter()
            .find(|particle| particle.source_class == PRIMARY_UNDERWATER_IMPACT_CLASS)
            .unwrap();
        assert!(below.position[1] > before_below.position[1]);
        assert!(below.velocity[1] > before_below.velocity[1]);
    }

    #[test]
    fn primary_heightfield_hit_removes_bullet_and_emits_ordinary_response_event() {
        let terrain = flat_terrain(0, 0b101);
        let mut fx = full_rate_world_fx();
        fx.queue_primary_fire_batch(&[primary_event(0.1, [0.0; 3])], None);
        fx.process_pending();
        fx.take_positional_sounds();
        let position = fx.test_particles_in_virgin_birth_order()[0].position;

        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [5; 8],
            water_response_selectors: [6; 8],
        };
        let impacts = fx
            .update(
                ParticleUpdateRequest::terrain(0, 0, context).with_callbacks(
                    ParticleCallbackContext {
                        owner_motions: &[],
                        collision: Some(ParticleCollisionContext {
                            entities: &[],
                            model_pool: &(),
                        }),
                    },
                ),
            )
            .primary_impacts;
        assert_eq!(
            impacts,
            vec![PrimaryImpact::Terrain {
                projectile_owner_id: 7,
                position_world: position,
                surface_y: 0.0,
                material_code: 5,
                response_selector: 5,
            }]
        );
        assert!(!fx
            .test_particles_in_virgin_birth_order()
            .iter()
            .any(|particle| particle.source_class == PRIMARY_BULLET_PARTICLE_CLASS));
        let debris = fx
            .test_particles_in_virgin_birth_order()
            .into_iter()
            .filter(|particle| particle.source_class == 11)
            .collect::<Vec<_>>();
        assert_eq!(debris.len(), SURFACE_RESPONSE_BURST_COUNT);
        assert_eq!(debris[0].current_sprite_id(), 724);
        assert_eq!(debris[0].lifetime_ticks, SURFACE_RESPONSE_LIFETIME_TICKS);
        assert_eq!(debris[0].collision_radius_raw, SURFACE_RESPONSE_RADIUS_RAW);
        assert_eq!(debris[0].animation_rate_raw, 0x20);
        assert_eq!(debris[0].draw_scale_raw, SURFACE_RESPONSE_DRAW_SCALE_RAW);
        // The global direction cursor increments before lookup, so its first
        // response uses table entry 1: [681,-488,-467].
        assert_eq!(debris[0].velocity, raw_velocity_to_world([340, 122, -234]));
        let mut expected_rng = 0;
        let expected_rate = PRIMARY_TERRAIN_IMPACT_RATE_MIN_16_16
            + u32::from(retail_random_u16(&mut expected_rng) >> 2);
        assert_eq!(
            fx.take_positional_sounds(),
            vec![PositionalSoundEvent {
                sound_id: PRIMARY_TERRAIN_IMPACT_SOUND_ID,
                position,
                frequency_q16: expected_rate,
            }]
        );
    }

    #[test]
    fn cistern_heightfield_hit_uses_selector_twelve_without_panicking() {
        let terrain = flat_terrain(0, 0);
        let mut fx = full_rate_world_fx();
        fx.queue_primary_fire_batch(&[primary_event(0.1, [0.0; 3])], None);
        fx.process_pending();
        fx.take_positional_sounds();

        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [12; 8],
            water_response_selectors: [6; 8],
        };
        let impacts = fx
            .update(
                ParticleUpdateRequest::terrain(0, 0, context).with_callbacks(
                    ParticleCallbackContext {
                        owner_motions: &[],
                        collision: Some(ParticleCollisionContext {
                            entities: &[],
                            model_pool: &(),
                        }),
                    },
                ),
            )
            .primary_impacts;

        assert!(matches!(
            impacts.as_slice(),
            [PrimaryImpact::Terrain {
                response_selector: 12,
                ..
            }]
        ));
        let particles = fx.test_particles_in_virgin_birth_order();
        assert!(!particles
            .iter()
            .any(|particle| particle.source_class == PRIMARY_BULLET_PARTICLE_CLASS));
        assert_eq!(
            particles
                .iter()
                .filter(|particle| particle.source_class == 72)
                .count(),
            SURFACE_RESPONSE_BURST_COUNT
        );
        assert_eq!(
            fx.take_positional_sounds()[0].sound_id,
            PRIMARY_TERRAIN_IMPACT_SOUND_ID
        );
    }

    #[test]
    fn upgraded_primary_ordinary_surface_callback_quarters_and_survives() {
        let terrain = flat_terrain(0, 0b101);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [5; 8],
            water_response_selectors: [6; 8],
        };
        let mut fx = WorldFx::new();
        fx.queue_primary_fire_batch(&[upgraded_primary_event(0.05, [0.0; 3])], None);
        fx.process_pending();
        fx.take_positional_sounds();

        let impacts = fx
            .update(
                ParticleUpdateRequest::terrain(0, 0, context).with_callbacks(
                    ParticleCallbackContext {
                        owner_motions: &[],
                        collision: Some(ParticleCollisionContext {
                            entities: &[],
                            model_pool: &(),
                        }),
                    },
                ),
            )
            .primary_impacts;

        assert_eq!(impacts.len(), 1);
        let projectile = fx
            .test_particles_in_virgin_birth_order()
            .into_iter()
            .find(|particle| particle.source_class == UPGRADED_PRIMARY_PARTICLE_CLASS)
            .unwrap();
        assert_eq!(projectile.position[1], 0.0);
        assert_eq!(projectile.velocity, raw_velocity_to_world([500, 0, 0]));
        assert!(fx.take_positional_sounds().is_empty());
        assert_eq!(fx.particle_count(), 2); // projectile plus class-32 muzzle
    }

    #[test]
    fn upgraded_primary_selector_six_spawns_class_75_and_deletes() {
        let terrain = flat_terrain(0, 0);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [6; 8],
            water_response_selectors: [6; 8],
        };
        let mut fx = WorldFx::new();
        fx.queue_primary_fire_batch(&[upgraded_primary_event(0.05, [0.0; 3])], None);
        fx.process_pending();
        fx.take_positional_sounds();

        let impacts = fx
            .update(
                ParticleUpdateRequest::terrain(0, 0, context).with_callbacks(
                    ParticleCallbackContext {
                        owner_motions: &[],
                        collision: Some(ParticleCollisionContext {
                            entities: &[],
                            model_pool: &(),
                        }),
                    },
                ),
            )
            .primary_impacts;

        assert_eq!(impacts.len(), 1);
        assert!(!fx
            .test_particles_in_virgin_birth_order()
            .iter()
            .any(|particle| particle.source_class == UPGRADED_PRIMARY_PARTICLE_CLASS));
        let burst = fx
            .test_particles_in_virgin_birth_order()
            .into_iter()
            .find(|particle| particle.source_class == UPGRADED_PRIMARY_ABOVE_SURFACE_CLASS)
            .unwrap();
        assert_eq!(
            burst.velocity,
            raw_velocity_to_world([
                0,
                200 + i32::from(
                    particle_descriptor(UPGRADED_PRIMARY_ABOVE_SURFACE_CLASS)
                        .unwrap()
                        .spawn_velocity_y_bias_raw(),
                ),
                0,
            ])
        );
        assert!(fx.take_positional_sounds().is_empty());
    }

    #[test]
    fn primary_wave_entry_selector_six_survives_with_splash_and_exact_sound() {
        let mut terrain = flat_terrain(-128, 3);
        terrain.header[0] = 0;
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [6; 8],
            water_response_selectors: [6; 8],
        };
        let mut fx = WorldFx::new();
        fx.update(ParticleUpdateRequest::terrain(0, 0, context));
        fx.queue_primary_fire_batch(&[primary_event(3.0, [0.0; 3])], Some(0.0));
        fx.process_pending();
        fx.take_positional_sounds();

        let dt = 0.02;
        let x = 40.0;
        let z = 72.0;
        let radius = f32::from(PRIMARY_BULLET_RADIUS_RAW) / 256.0;
        let start_surface = wave_surface_y(x, z, 0.0, 0.0, -16.0);
        let end_surface = wave_surface_y(x, z, 1.0, 0.0, -16.0);
        let start_y = start_surface + radius + 0.5;
        let end_y = end_surface - radius - 0.5;
        let bullet = fx
            .particles
            .test_iter_mut_in_physical_order()
            .find(|particle| particle.source_class == PRIMARY_BULLET_PARTICLE_CLASS)
            .unwrap();
        bullet.position = [x, start_y, z];
        bullet.velocity = [0.0, (end_y - start_y) / dt, 0.0];

        let impacts = fx
            .update(
                ParticleUpdateRequest::terrain(20_000, 1, context).with_callbacks(
                    ParticleCallbackContext {
                        owner_motions: &[],
                        collision: Some(ParticleCollisionContext {
                            entities: &[],
                            model_pool: &(),
                        }),
                    },
                ),
            )
            .primary_impacts;
        let stepped = fx
            .test_particles_in_virgin_birth_order()
            .into_iter()
            .find(|particle| particle.source_class == PRIMARY_BULLET_PARTICLE_CLASS)
            .unwrap();
        assert_eq!(stepped.step_start_water_state, 2);
        assert!(stepped.water_state < 2);
        let impact_position = stepped.position;
        assert_eq!(
            impacts,
            vec![PrimaryImpact::WaterEntry {
                projectile_owner_id: 7,
                position_world: impact_position,
                surface_y: end_surface,
                material_code: 3,
                response_selector: 6,
            }]
        );
        assert!(fx
            .test_particles_in_virgin_birth_order()
            .iter()
            .any(|particle| particle.source_class == PRIMARY_BULLET_PARTICLE_CLASS));
        let splash = fx
            .test_particles_in_virgin_birth_order()
            .into_iter()
            .find(|particle| particle.source_class == WATER_SPLASH_PARTICLE_CLASS)
            .unwrap();
        assert_eq!(splash.position, [x, end_surface, z]);
        assert_eq!(splash.current_sprite_id(), 732);
        assert_eq!(splash.velocity, [0.0; 3]);

        let mut expected_rng = 0;
        let sound_roll = retail_random_u16(&mut expected_rng);
        let sound_id = if sound_roll & 0x1000 != 0 { 26 } else { 27 };
        let rate =
            SPECIAL_SURFACE_RATE_MIN_16_16 + u32::from(retail_random_u16(&mut expected_rng) >> 2);
        assert_eq!(
            fx.take_positional_sounds(),
            vec![PositionalSoundEvent {
                sound_id,
                position: impact_position,
                frequency_q16: rate,
            }]
        );
    }

    #[test]
    fn displaced_water_preserves_retail_tick_bits_above_f32_integer_precision() {
        let mut terrain = flat_terrain(-128, 0);
        terrain.header[0] = 0;
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [0; 8],
            water_response_selectors: [0; 8],
        };
        let position = [40.0, 0.0, 72.0];
        let retail_tick = (1_u32 << 24) + 1;
        let expected_raw = wave_surface_raw(
            (40 * 256) as i16,
            (72 * 256) as i16,
            retail_tick as i32,
            0,
            -128 * 32,
        );
        let float_truncated_raw = wave_surface_raw(
            (40 * 256) as i16,
            (72 * 256) as i16,
            (retail_tick as f32) as i32,
            0,
            -128 * 32,
        );

        assert_ne!(expected_raw, float_truncated_raw);
        assert_eq!(
            displaced_water_surface(context, position, retail_tick),
            Some(f32::from(expected_raw) / 256.0)
        );
    }

    #[test]
    fn primary_wave_entry_selector_seven_deletes_with_class_twelve() {
        let mut terrain = flat_terrain(-128, 4);
        terrain.header[0] = 0;
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [7; 8],
            water_response_selectors: [7; 8],
        };
        let mut fx = WorldFx::new();
        fx.update(ParticleUpdateRequest::terrain(0, 0, context));
        fx.queue_primary_fire_batch(&[primary_event(3.0, [0.0; 3])], Some(0.0));
        fx.process_pending();
        fx.take_positional_sounds();
        let dt = 0.02;
        let x = 40.0;
        let z = 72.0;
        let radius = f32::from(PRIMARY_BULLET_RADIUS_RAW) / 256.0;
        let start_y = wave_surface_y(x, z, 0.0, 0.0, -16.0) + radius + 0.5;
        let end_y = wave_surface_y(x, z, 1.0, 0.0, -16.0) - radius - 0.5;
        let bullet = fx
            .particles
            .test_iter_mut_in_physical_order()
            .find(|particle| particle.source_class == PRIMARY_BULLET_PARTICLE_CLASS)
            .unwrap();
        bullet.position = [x, start_y, z];
        bullet.velocity = [0.0, (end_y - start_y) / dt, 0.0];
        let impacts = fx
            .update(
                ParticleUpdateRequest::terrain(20_000, 1, context).with_callbacks(
                    ParticleCallbackContext {
                        owner_motions: &[],
                        collision: Some(ParticleCollisionContext {
                            entities: &[],
                            model_pool: &(),
                        }),
                    },
                ),
            )
            .primary_impacts;
        assert!(matches!(
            impacts.as_slice(),
            [PrimaryImpact::WaterEntry {
                response_selector: 7,
                ..
            }]
        ));
        assert!(!fx
            .test_particles_in_virgin_birth_order()
            .iter()
            .any(|particle| particle.source_class == PRIMARY_BULLET_PARTICLE_CLASS));
        let delete_effect = fx
            .test_particles_in_virgin_birth_order()
            .into_iter()
            .find(|particle| particle.source_class == WATER_DELETE_PARTICLE_CLASS)
            .unwrap();
        assert_eq!(delete_effect.current_sprite_id(), 824);
        assert_eq!(delete_effect.animation_rate_raw, 0x40);
        assert_eq!(delete_effect.draw_scale_raw, 0x0C00);
    }

    #[test]
    fn primary_ground_selector_six_keeps_bullet_and_emits_class_thirteen() {
        let terrain = flat_terrain(0, 6);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [6; 8],
            water_response_selectors: [6; 8],
        };
        let mut fx = WorldFx::new();
        fx.queue_primary_fire_batch(&[primary_event(0.1, [0.0; 3])], None);
        fx.process_pending();
        fx.take_positional_sounds();
        let impacts = fx
            .update(
                ParticleUpdateRequest::terrain(0, 0, context).with_callbacks(
                    ParticleCallbackContext {
                        owner_motions: &[],
                        collision: Some(ParticleCollisionContext {
                            entities: &[],
                            model_pool: &(),
                        }),
                    },
                ),
            )
            .primary_impacts;
        assert!(matches!(
            impacts.as_slice(),
            [PrimaryImpact::Terrain {
                response_selector: 6,
                ..
            }]
        ));
        assert!(fx
            .test_particles_in_virgin_birth_order()
            .iter()
            .any(|particle| particle.source_class == PRIMARY_BULLET_PARTICLE_CLASS));
        assert!(fx
            .test_particles_in_virgin_birth_order()
            .iter()
            .any(|particle| particle.source_class == WATER_SPLASH_PARTICLE_CLASS));
    }

    #[test]
    fn collision_helpers_round_material_cells_and_preserve_retail_word_seam_rules() {
        let mut terrain = flat_terrain(0, 0);
        terrain.cells[10 * GRID_SIZE + 20].attribute = 0xff;
        terrain.cells[10 * GRID_SIZE + 20].terrain_type = 0xf2;
        terrain.cells[11 * GRID_SIZE + 21].terrain_type = 0xa7;
        terrain.cells[255 * GRID_SIZE + 20].terrain_type = 4;
        terrain.cells[20].terrain_type = 6;
        assert_eq!(
            nearest_material_code_raw(&terrain, 10 * 256 + 0x7f, 20 * 256 + 0x7f),
            2
        );
        assert_eq!(
            nearest_material_code_raw(&terrain, 10 * 256 + 0x80, 20 * 256 + 0x80),
            7
        );
        assert_eq!(nearest_material_code_raw(&terrain, -0x81, 20 * 256), 4);
        assert_eq!(nearest_material_code_raw(&terrain, -0x80, 20 * 256), 6);
        let low = world_position_to_raw([127.9, 0.0, 10.0]);
        let high = world_position_to_raw([128.1, 0.0, 10.0]);
        assert!(raw_spheres_overlap(low, high, 54));
        assert!(!raw_spheres_overlap(low, high, 49));
        assert!(!raw_spheres_overlap_nonwrapping(low, high, 54));
    }

    #[test]
    fn player_wreck_burst_matches_captured_direction_window_owner_and_audio_order() {
        const OWNER: u32 = 0x04BE_0001;
        const ORIGIN_RAW: [i16; 3] = [-32_567, -524, -27_266];
        const EXPECTED_POSITIONS_RAW: [[i16; 3]; 11] = [
            [-32_442, -945, -27_687],
            [32_653, -1_125, -27_867],
            [-32_281, -284, -27_026],
            [-32_764, -1_266, -28_008],
            [-32_299, -979, -27_721],
            [32_302, -863, -27_605],
            [32_340, -355, -27_097],
            [-32_759, 294, -26_448],
            [32_263, -540, -27_282],
            [-32_387, -101, -26_843],
            ORIGIN_RAW,
        ];
        const EXPECTED_VELOCITIES_RAW: [[i32; 3]; 11] = [
            [108, -362, -475],
            [-203, -385, -117],
            [254, 213, 508],
            [-131, -492, 106],
            [301, -511, 420],
            [-477, -243, 66],
            [-206, 55, -99],
            [-178, 754, 82],
            [-881, -20, 471],
            [71, 166, -195],
            [0, 500, 0],
        ];

        let seed = 0x1234_5678;
        let mut expected_rng = seed;
        let expected_random_rate =
            METEOR_IMPACT_RATE_MIN_16_16 + u32::from(retail_random_u16(&mut expected_rng) >> 3);
        let mut fx = full_rate_world_fx();
        fx.rng_state = seed;
        // The retail capture reached direction indices 9..=18, so the shared
        // pre-increment cursor was 8 when FUN_004475F0 ran.
        fx.direction_cursor = 8;
        fx.emit_player_wreck_burst_raw(ORIGIN_RAW, 1_100, Some(-525), OWNER);

        assert_eq!(fx.direction_cursor, 18);
        assert_eq!(fx.rng_state, expected_rng);
        let particles = fx.test_particles_in_virgin_birth_order();
        assert_eq!(particles.len(), 11);
        assert_eq!(
            particles
                .iter()
                .map(|particle| particle.source_class)
                .collect::<Vec<_>>(),
            [vec![METEOR_SCATTER_CLASS; 10], vec![METEOR_SURFACE_CLASS]].concat()
        );
        assert!(particles
            .iter()
            .all(|particle| particle.owner_id == Some(OWNER)));
        assert_eq!(
            particles
                .iter()
                .map(|particle| world_position_to_raw(particle.position))
                .collect::<Vec<_>>(),
            EXPECTED_POSITIONS_RAW
        );
        assert_eq!(
            particles
                .iter()
                .map(|particle| particle.velocity.map(world_velocity_component_to_raw))
                .collect::<Vec<_>>(),
            EXPECTED_VELOCITIES_RAW
        );
        assert_eq!(
            fx.take_positional_sounds(),
            vec![
                PositionalSoundEvent {
                    sound_id: PLAYER_WRECK_SOUND_ID,
                    position: raw_position_to_world(ORIGIN_RAW),
                    frequency_q16: expected_random_rate,
                },
                PositionalSoundEvent {
                    sound_id: PLAYER_WRECK_SOUND_ID,
                    position: raw_position_to_world(ORIGIN_RAW),
                    frequency_q16: PLAYER_WRECK_FIXED_RATE_16_16,
                },
            ]
        );
    }

    #[test]
    fn player_wreck_surface_branch_attempts_underwater_classes_at_and_below_sea() {
        let mut fx = full_rate_world_fx();
        fx.emit_player_wreck_burst_raw([100, -20, 300], 0, Some(-20), 7);

        let particles = fx.test_particles_in_virgin_birth_order();
        assert_eq!(particles.len(), METEOR_IMPACT_SCATTER_COUNT + 2);
        assert_eq!(
            particles[METEOR_IMPACT_SCATTER_COUNT..]
                .iter()
                .map(|particle| particle.source_class)
                .collect::<Vec<_>>(),
            UNDERWATER_IMPACT_CLASSES
        );
        assert_eq!(
            world_position_to_raw(particles[METEOR_IMPACT_SCATTER_COUNT].position),
            [100, -20, 300]
        );
        assert_eq!(
            world_position_to_raw(particles[METEOR_IMPACT_SCATTER_COUNT + 1].position),
            [36, -20, 300]
        );
        assert!(particles
            .iter()
            .all(|particle| particle.owner_id == Some(7)));
    }

    #[test]
    fn type61_explode_with_ring_preserves_scatter_tail_rng_and_logical_owner_provenance() {
        const POSITION_RAW: [i16; 3] = [0x1200, 0x0340, -0x2100];
        const EXTENT_RAW: u16 = 0x0200;
        const OWNER_ID: u32 = 0x04A3_0001;
        const OWNER_TYPE: u8 = 46;

        let seed = 0x1234_5678;
        let mut expected_rng = seed;
        let expected_random_rate =
            METEOR_IMPACT_RATE_MIN_16_16 + u32::from(retail_random_u16(&mut expected_rng) >> 3);
        let mut fx = full_rate_world_fx();
        fx.rng_state = seed;
        fx.direction_cursor = 8;

        fx.emit_explode_with_ring_burst_raw(ExplodeWithRingBurstRequest {
            position_raw: POSITION_RAW,
            source_extent_raw: EXTENT_RAW,
            sea_level_raw: None,
            logical_owner_entity_id: OWNER_ID,
            logical_owner_entity_type: OWNER_TYPE,
            scatter_count: 16,
            scatter_classes: [94, 95],
            suppresses_impact_damage: false,
        });

        assert_eq!(
            fx.direction_cursor,
            8 + TYPE61_EXPLODE_WITH_RING_SCATTER_COUNT
        );
        assert_eq!(fx.rng_state, expected_rng);
        let particles = fx.test_particles_in_virgin_birth_order();
        assert_eq!(particles.len(), TYPE61_EXPLODE_WITH_RING_SCATTER_COUNT + 1);
        let (scatter, surface) = particles.split_at(TYPE61_EXPLODE_WITH_RING_SCATTER_COUNT);
        for (index, particle) in scatter.iter().enumerate() {
            let source_class = TYPE61_EXPLODE_WITH_RING_SCATTER_CLASSES[index & 1];
            let direction = RETAIL_DIRECTION_TABLE_RAW[9 + index];
            let l1_length = direction
                .iter()
                .map(|component| i32::from(*component).abs())
                .sum::<i32>();
            let expected_offset = [
                i32::from(EXTENT_RAW) * i32::from(direction[0]) / l1_length,
                i32::from(EXTENT_RAW) * i32::from(direction[1]) / l1_length,
                i32::from(EXTENT_RAW) * i32::from(direction[1]) / l1_length,
            ];
            let expected_position = std::array::from_fn(|axis| {
                POSITION_RAW[axis].wrapping_add(expected_offset[axis] as i16)
            });
            let mut expected_velocity = direction.map(|component| i32::from(component) >> 1);
            expected_velocity[1] += i32::from(
                particle_descriptor(source_class)
                    .expect("Type-61 scatter descriptor")
                    .spawn_velocity_y_bias_raw(),
            );

            assert_eq!(particle.source_class, source_class);
            assert_eq!(world_position_to_raw(particle.position), expected_position);
            assert_eq!(
                particle.velocity.map(world_velocity_component_to_raw),
                expected_velocity
            );
            assert_eq!(particle.owner_id, Some(OWNER_ID));
            assert_eq!(particle.source_entity_type_at_birth, Some(OWNER_TYPE));
            assert!(!particle.suppresses_impact_damage);
        }
        assert_eq!(surface.len(), 1);
        assert_eq!(surface[0].source_class, METEOR_SURFACE_CLASS);
        assert_eq!(world_position_to_raw(surface[0].position), POSITION_RAW);
        assert_eq!(surface[0].owner_id, Some(OWNER_ID));
        assert_eq!(surface[0].source_entity_type_at_birth, Some(OWNER_TYPE));
        assert_eq!(
            surface[0].velocity.map(world_velocity_component_to_raw),
            [0, 500, 0]
        );
        assert_eq!(
            fx.take_positional_sounds(),
            vec![PositionalSoundEvent {
                sound_id: METEOR_IMPACT_SOUND_ID,
                position: raw_position_to_world(POSITION_RAW),
                frequency_q16: expected_random_rate,
            }]
        );
    }

    #[test]
    fn type61_explode_with_ring_keeps_minimum_one_and_stops_on_first_rejection() {
        const OWNER_ID: u32 = 0x0493_0001;
        const OWNER_TYPE: u8 = 61;

        let mut cold = WorldFx::new();
        cold.emit_explode_with_ring_burst_raw(ExplodeWithRingBurstRequest {
            position_raw: [0; 3],
            source_extent_raw: 0,
            sea_level_raw: None,
            logical_owner_entity_id: OWNER_ID,
            logical_owner_entity_type: OWNER_TYPE,
            scatter_count: 16,
            scatter_classes: [94, 95],
            suppresses_impact_damage: false,
        });
        assert_eq!(cold.direction_cursor, 1);
        assert_eq!(
            cold.test_particles_in_virgin_birth_order()
                .iter()
                .map(|particle| particle.source_class)
                .collect::<Vec<_>>(),
            [
                TYPE61_EXPLODE_WITH_RING_SCATTER_CLASSES[0],
                METEOR_SURFACE_CLASS,
            ]
        );

        let mut saturated = full_rate_world_fx();
        saturated.particles.test_fill_to(
            MAX_WORLD_PARTICLES - 1,
            pool_test_particle(METEOR_SCATTER_CLASS, 0.0),
        );
        let seed = saturated.rng_state;
        let mut expected_rng = seed;
        let _sound_word = retail_random_u16(&mut expected_rng);

        saturated.emit_explode_with_ring_burst_raw(ExplodeWithRingBurstRequest {
            position_raw: [0; 3],
            source_extent_raw: 0x100,
            sea_level_raw: None,
            logical_owner_entity_id: OWNER_ID,
            logical_owner_entity_type: OWNER_TYPE,
            scatter_count: 16,
            scatter_classes: [94, 95],
            suppresses_impact_damage: false,
        });

        assert_eq!(
            saturated.direction_cursor, 2,
            "one accepted scatter and the first rejected attempt consume directions"
        );
        assert_eq!(saturated.particle_count(), MAX_WORLD_PARTICLES);
        assert_eq!(
            saturated
                .test_particles_in_virgin_birth_order()
                .iter()
                .filter(|particle| particle.owner_id == Some(OWNER_ID))
                .map(|particle| (particle.source_class, particle.source_entity_type_at_birth))
                .collect::<Vec<_>>(),
            [(
                TYPE61_EXPLODE_WITH_RING_SCATTER_CLASSES[0],
                Some(OWNER_TYPE)
            )]
        );
        assert_eq!(saturated.rng_state, expected_rng);
        assert_eq!(saturated.take_positional_sounds().len(), 1);
    }

    #[test]
    fn type61_explode_with_ring_underwater_tail_keeps_order_offset_and_provenance() {
        const OWNER_ID: u32 = 0x0493_0001;
        const OWNER_TYPE: u8 = 46;
        let mut fx = full_rate_world_fx();

        fx.emit_explode_with_ring_burst_raw(ExplodeWithRingBurstRequest {
            position_raw: [100, -20, 300],
            source_extent_raw: 0x100,
            sea_level_raw: Some(-20),
            logical_owner_entity_id: OWNER_ID,
            logical_owner_entity_type: OWNER_TYPE,
            scatter_count: 16,
            scatter_classes: [94, 95],
            suppresses_impact_damage: false,
        });

        let particles = fx.test_particles_in_virgin_birth_order();
        let tail = &particles[TYPE61_EXPLODE_WITH_RING_SCATTER_COUNT..];
        assert_eq!(
            tail.iter()
                .map(|particle| particle.source_class)
                .collect::<Vec<_>>(),
            UNDERWATER_IMPACT_CLASSES
        );
        assert_eq!(world_position_to_raw(tail[0].position), [100, -20, 300]);
        assert_eq!(world_position_to_raw(tail[1].position), [36, -20, 300]);
        assert!(tail.iter().all(|particle| {
            particle.owner_id == Some(OWNER_ID)
                && particle.source_entity_type_at_birth == Some(OWNER_TYPE)
        }));
    }

    #[test]
    fn shared_scatter_surface_helper_has_no_player_only_second_sound() {
        let seed = 0x1234_5678;
        let mut expected_rng = seed;
        let expected_random_rate =
            METEOR_IMPACT_RATE_MIN_16_16 + u32::from(retail_random_u16(&mut expected_rng) >> 3);
        let mut fx = full_rate_world_fx();
        fx.rng_state = seed;

        fx.emit_scatter_surface_burst_raw([100, 200, 300], 0x10, None, 0x04A3_0001);

        assert_eq!(fx.rng_state, expected_rng);
        assert_eq!(
            fx.take_positional_sounds(),
            vec![PositionalSoundEvent {
                sound_id: METEOR_IMPACT_SOUND_ID,
                position: raw_position_to_world([100, 200, 300]),
                frequency_q16: expected_random_rate,
            }]
        );
        assert!(fx
            .test_particles_in_virgin_birth_order()
            .iter()
            .all(|particle| particle.owner_id == Some(0x04A3_0001)));
    }

    #[test]
    fn shared_explosion_sound_uses_mutated_underwater_origin_even_when_pool_is_full() {
        for (position_raw, sea_level_raw, expected_position_raw) in [
            ([100, -19, 300], Some(-20), [100, -19, 300]),
            ([100, -20, 300], Some(-20), [36, -20, 300]),
            ([i16::MIN, -21, -1], Some(-20), [32704, -21, -1]),
            ([100, -20, 300], None, [100, -20, 300]),
        ] {
            for saturated in [false, true] {
                let mut fx = full_rate_world_fx();
                if saturated {
                    fx.particles.test_fill_to(
                        MAX_WORLD_PARTICLES,
                        pool_test_particle(METEOR_SCATTER_CLASS, 0.0),
                    );
                }
                let mut expected_rng = fx.rng_state;
                let expected_rate = METEOR_IMPACT_RATE_MIN_16_16
                    + u32::from(retail_random_u16(&mut expected_rng) >> 3);
                fx.emit_scatter_surface_burst_raw(position_raw, 0x100, sea_level_raw, 9);
                assert_eq!(
                    fx.take_positional_sounds(),
                    vec![PositionalSoundEvent {
                        sound_id: METEOR_IMPACT_SOUND_ID,
                        position: raw_position_to_world(expected_position_raw),
                        frequency_q16: expected_rate,
                    }],
                    "position={position_raw:?}, sea={sea_level_raw:?}, saturated={saturated}"
                );
                assert_eq!(fx.rng_state, expected_rng);
            }
        }
    }

    #[test]
    fn underwater_player_wreck_keeps_separate_shared_and_wrapper_sound_origins() {
        let mut fx = full_rate_world_fx();
        let mut expected_rng = fx.rng_state;
        let expected_rate =
            METEOR_IMPACT_RATE_MIN_16_16 + u32::from(retail_random_u16(&mut expected_rng) >> 3);
        fx.emit_player_wreck_burst_raw([100, -20, 300], 0x100, Some(-20), 9);
        assert_eq!(
            fx.take_positional_sounds(),
            vec![
                PositionalSoundEvent {
                    sound_id: PLAYER_WRECK_SOUND_ID,
                    position: raw_position_to_world([36, -20, 300]),
                    frequency_q16: expected_rate,
                },
                PositionalSoundEvent::fixed(
                    PLAYER_WRECK_SOUND_ID,
                    raw_position_to_world([100, -20, 300]),
                ),
            ]
        );
        assert_eq!(fx.rng_state, expected_rng);
    }

    #[test]
    fn player_wreck_scatter_stops_on_rejection_but_surface_and_audio_are_still_attempted() {
        let mut fx = WorldFx::new();
        fx.particles.test_fill_to(
            MAX_WORLD_PARTICLES,
            pool_test_particle(METEOR_SCATTER_CLASS, 0.0),
        );
        fx.direction_cursor = 8;
        let seed = fx.rng_state;
        let mut expected_rng = seed;
        let expected_random_rate =
            METEOR_IMPACT_RATE_MIN_16_16 + u32::from(retail_random_u16(&mut expected_rng) >> 3);

        fx.emit_player_wreck_burst_raw([0; 3], 1_100, None, 9);

        assert_eq!(fx.particle_count(), MAX_WORLD_PARTICLES);
        assert_eq!(
            fx.direction_cursor, 9,
            "only the rejected attempt consumes a direction"
        );
        assert_eq!(fx.rng_state, expected_rng);
        assert_eq!(
            fx.take_positional_sounds(),
            vec![
                PositionalSoundEvent {
                    sound_id: PLAYER_WRECK_SOUND_ID,
                    position: [0.0; 3],
                    frequency_q16: expected_random_rate,
                },
                PositionalSoundEvent::fixed(PLAYER_WRECK_SOUND_ID, [0.0; 3]),
            ]
        );
    }

    #[test]
    fn meteor_impact_has_recovered_particle_and_sound_contract() {
        let position = [12.0, -4.0, 250.0];
        let mut fx = full_rate_world_fx();
        fx.queue_meteor_impact(position, 0, Some(0x0476_0001));

        assert_eq!(fx.pending_event_count(), 2);
        fx.process_pending();
        assert_eq!(
            fx.test_particles_in_virgin_birth_order().len(),
            METEOR_IMPACT_PARTICLE_COUNT
        );
        assert_eq!(
            fx.test_particles_in_virgin_birth_order()
                .iter()
                .map(|particle| particle.sprite_id)
                .collect::<Vec<_>>(),
            vec![699, 699, 699, 699, 699, 699, 699, 699, 699, 699, 873]
        );
        let particles = fx.test_particles_in_virgin_birth_order();
        assert!(particles
            .iter()
            .all(|particle| particle.owner_id == Some(0x0476_0001)));
        let (scatter, surface) = particles.split_at(METEOR_IMPACT_SCATTER_COUNT);
        assert!(scatter.iter().all(|particle| {
            particle.source_class == METEOR_SCATTER_CLASS
                && particle.lifetime_ticks == METEOR_SCATTER_LIFETIME_TICKS
                && particle.collision_radius_raw == METEOR_SCATTER_RADIUS_RAW
                && particle.frame_middle_raw == 0
                && particle.frame_scale_raw == 0x100
                && particle.animation_rate_raw == METEOR_SCATTER_ANIMATION_RATE_RAW
                && particle.draw_scale_raw == METEOR_SCATTER_DRAW_SCALE_RAW
                && particle.size_jitter_divisor_raw == METEOR_SCATTER_SIZE_JITTER_DIVISOR_RAW
                && particle.sprite_id == 699
        }));
        assert_eq!(surface.len(), METEOR_IMPACT_SURFACE_COUNT);
        assert_eq!(surface[0].source_class, METEOR_SURFACE_CLASS);
        assert_eq!(surface[0].lifetime_ticks, METEOR_SURFACE_LIFETIME_TICKS);
        assert_eq!(surface[0].collision_radius_raw, METEOR_SURFACE_RADIUS_RAW);
        assert_eq!(surface[0].frame_middle_raw, 600);
        assert_eq!(surface[0].frame_scale_raw, 0x100);
        assert_eq!(
            surface[0].animation_rate_raw,
            METEOR_SURFACE_ANIMATION_RATE_RAW
        );
        assert_eq!(surface[0].draw_scale_raw, METEOR_SURFACE_DRAW_SCALE_RAW);
        assert_eq!(
            surface[0].size_jitter_divisor_raw,
            METEOR_SURFACE_SIZE_JITTER_DIVISOR_RAW
        );
        assert_eq!(surface[0].velocity, raw_velocity_to_world([0, 500, 0]));

        assert_eq!(
            fx.take_positional_sounds(),
            vec![PositionalSoundEvent {
                sound_id: METEOR_IMPACT_SOUND_ID,
                position,
                frequency_q16: METEOR_IMPACT_RATE_MIN_16_16,
            }]
        );
        assert_eq!(
            PositionalSoundEvent::fixed(7, position),
            PositionalSoundEvent {
                sound_id: 7,
                position,
                frequency_q16: 0x1_0000,
            }
        );
    }

    #[test]
    fn meteor_scatter_failure_still_attempts_surface_particle() {
        let mut burst = meteor_impact_burst([0.0; 3], 0).particles;
        let surface = burst.pop().unwrap();
        let mut attempts = Vec::new();
        materialize_meteor_impact_spawns(burst, surface, |spawn| {
            attempts.push(spawn.source_class);
            (spawn.source_class == METEOR_SURFACE_CLASS).then_some(17)
        });

        assert_eq!(attempts, [METEOR_SCATTER_CLASS, METEOR_SURFACE_CLASS]);
    }

    #[test]
    fn meteor_scatter_is_deterministic_and_index_selectable() {
        let a = meteor_impact_burst([0.0; 3], 3);
        let b = meteor_impact_burst([0.0; 3], 3);
        let c = meteor_impact_burst([0.0; 3], 4);
        assert_eq!(a, b);
        assert_ne!(a.particles[0].velocity, c.particles[0].velocity);
        assert_eq!(a.particles[1].velocity, c.particles[0].velocity);
        assert_eq!(a.particles[0].sprite_id, 699);
        assert_eq!(c.particles[0].sprite_id, 699);
        assert_eq!(a.particles[METEOR_IMPACT_SCATTER_COUNT].sprite_id, 873);
        assert_eq!(c.particles[METEOR_IMPACT_SCATTER_COUNT].sprite_id, 873);
    }

    #[test]
    fn meteor_wake_uses_the_captured_descriptor_and_owner_contract() {
        let descriptor = particle_descriptor(METEOR_WAKE_CLASS).unwrap();
        assert_eq!(descriptor.frame_count(), 8);
        assert_eq!(descriptor.flags(), 6);
        assert_eq!(descriptor.priority_raw(), 2);
        assert_eq!(descriptor.lifetime_ticks(), 15);
        assert_eq!(descriptor.collision_radius_raw(), 0x28);
        assert_eq!(descriptor.animation_rate_raw(), 0x18);
        assert_eq!(descriptor.draw_scale_raw(), 0x0800);
        assert_eq!(descriptor.size_jitter_divisor_raw(), 0x20);
        assert_eq!(descriptor.spawn_velocity_y_bias_raw(), 0);

        let mut fx = WorldFx::new();
        fx.queue_meteor_wake([-110.0, 3.0, -114.0], 0x0476_0001);
        fx.process_pending();
        let particles = fx.test_particles_in_virgin_birth_order();
        assert_eq!(particles.len(), 1);
        let wake = particles[0];
        assert_eq!(wake.source_class, METEOR_WAKE_CLASS);
        assert_eq!(wake.owner_id, Some(0x0476_0001));
        assert_eq!(wake.velocity, [0.0; 3]);
        assert_eq!(wake.lifetime_ticks, METEOR_WAKE_LIFETIME_TICKS);
        assert_eq!(wake.collision_radius_raw, METEOR_WAKE_RADIUS_RAW);
        assert_eq!(wake.animation_rate_raw, METEOR_WAKE_ANIMATION_RATE_RAW);
        assert_eq!(wake.draw_scale_raw, METEOR_WAKE_DRAW_SCALE_RAW);
        assert_eq!(
            wake.size_jitter_divisor_raw,
            METEOR_WAKE_SIZE_JITTER_DIVISOR_RAW
        );
    }

    #[test]
    fn intro2_scatter_windows_match_the_retail_particle_capture() {
        fn assert_window(
            first_direction_index: usize,
            origin_raw: [i32; 3],
            captured_positions_raw: [[i32; 3]; METEOR_IMPACT_SCATTER_COUNT],
            captured_velocities_raw: [[i32; 3]; METEOR_IMPACT_SCATTER_COUNT],
        ) {
            let burst = meteor_impact_burst([0.0; 3], first_direction_index);
            for (index, particle) in burst
                .particles
                .iter()
                .take(METEOR_IMPACT_SCATTER_COUNT)
                .enumerate()
            {
                let offset_raw = particle.offset.map(|value| (value * 256.0) as i32);
                // Y and Z use different origin components even though retail's
                // normalized offset intentionally duplicates direction Y.
                assert_eq!(
                    offset_raw,
                    [
                        captured_positions_raw[index][0] - origin_raw[0],
                        captured_positions_raw[index][1] - origin_raw[1],
                        captured_positions_raw[index][2] - origin_raw[2],
                    ]
                );
                assert_eq!(
                    particle.velocity.map(world_velocity_component_to_raw),
                    captured_velocities_raw[index]
                );
            }
        }

        assert_window(
            25,
            [-27_727, 707, -29_287],
            [
                [-27_745, 757, -29_237],
                [-27_806, 800, -29_194],
                [-27_734, 479, -29_515],
                [-27_703, 834, -29_160],
                [-27_828, 774, -29_220],
                [-27_739, 693, -29_301],
                [-27_690, 853, -29_141],
                [-27_723, 781, -29_213],
                [-27_728, 852, -29_142],
                [-27_613, 621, -29_373],
            ],
            [
                [-91, 247, -912],
                [-323, 376, 334],
                [-20, -606, -54],
                [22, 115, -95],
                [-544, 363, -465],
                [-40, -47, 748],
                [149, 573, 283],
                [16, 272, 644],
                [-9, 759, 570],
                [517, -389, 246],
            ],
        );
        assert_window(
            21,
            [-31_585, -46, -29_752],
            [
                [-31_378, -35, -29_741],
                [-31_604, 166, -29_540],
                [-31_632, -184, -29_890],
                [-31_583, 150, -29_556],
                [-31_603, 4, -29_702],
                [-31_664, 47, -29_659],
                [-31_592, -274, -29_980],
                [-31_561, 81, -29_625],
                [-31_686, 21, -29_685],
                [-31_597, -60, -29_766],
            ],
            [
                [903, 50, 161],
                [-55, 581, -64],
                [-215, -619, 313],
                [9, 714, 205],
                [-91, 247, -912],
                [-323, 376, 334],
                [-20, -606, -54],
                [22, 115, -95],
                [-544, 363, -465],
                [-40, -47, 748],
            ],
        );
        assert_window(
            22,
            [-28_690, 541, -29_395],
            [
                [-28_709, 753, -29_183],
                [-28_737, 403, -29_533],
                [-28_688, 737, -29_199],
                [-28_708, 591, -29_345],
                [-28_769, 634, -29_302],
                [-28_697, 313, -29_623],
                [-28_666, 668, -29_268],
                [-28_791, 608, -29_328],
                [-28_702, 527, -29_409],
                [-28_653, 687, -29_249],
            ],
            [
                [-55, 581, -64],
                [-215, -619, 313],
                [9, 714, 205],
                [-91, 247, -912],
                [-323, 376, 334],
                [-20, -606, -54],
                [22, 115, -95],
                [-544, 363, -465],
                [-40, -47, 748],
                [149, 573, 283],
            ],
        );
        assert_window(
            12,
            [-28_865, 95, 32_395],
            [
                [-28_911, -77, 32_223],
                [-28_803, -11, 32_289],
                [-29_020, 17, 32_317],
                [-29_011, 134, 32_434],
                [-28_909, 285, 32_585],
                [-29_029, 92, 32_392],
                [-28_823, 193, 32_493],
                [-28_814, -41, 32_259],
                [-28_816, 180, 32_480],
                [-28_658, 106, 32_406],
            ],
            [
                [-131, -492, 106],
                [301, -511, 420],
                [-477, -243, 66],
                [-206, 55, -99],
                [-178, 754, 82],
                [-881, -20, 471],
                [71, 166, -195],
                [7, -20, 10],
                [215, 372, 523],
                [903, 50, 161],
            ],
        );
    }

    #[test]
    fn meteor_scatter_preserves_retail_l1_offset_and_z_bug() {
        let burst = meteor_impact_burst([0.0; 3], 0);
        let first = burst.particles[0];
        // Direction [-356,-150,-1], L1 length 507, source extent 0x100.
        // C integer division truncates toward zero: [-179,-75], and retail
        // duplicates the normalized Y offset into Z.
        assert_eq!(first.offset, [-179.0 / 256.0, -75.0 / 256.0, -75.0 / 256.0]);
        assert_eq!(first.velocity, raw_velocity_to_world([-178, -75, -1]));
    }

    #[test]
    fn meteor_frame_tables_preserve_sprite_middle_and_scale_tuples() {
        let scatter_descriptor = particle_descriptor(METEOR_SCATTER_CLASS).unwrap();
        let scatter_frames: Vec<_> = (0..usize::from(scatter_descriptor.frame_count()))
            .map(|index| particle_frame(METEOR_SCATTER_CLASS, index).unwrap())
            .collect();
        assert_eq!(
            scatter_frames
                .iter()
                .map(|frame| frame.sprite_id)
                .collect::<Vec<_>>(),
            [699, 700, 701, 702]
        );
        assert!(scatter_frames
            .iter()
            .all(|frame| frame.middle_raw == 0 && frame.scale_raw == 0x100));

        let surface_descriptor = particle_descriptor(METEOR_SURFACE_CLASS).unwrap();
        let surface_frames: Vec<_> = (0..usize::from(surface_descriptor.frame_count()))
            .map(|index| particle_frame(METEOR_SURFACE_CLASS, index).unwrap())
            .collect();
        assert_eq!(
            surface_frames
                .iter()
                .map(|frame| frame.sprite_id)
                .collect::<Vec<_>>(),
            [873, 874, 875, 876, 877, 878, 879, 880, 881, 882, 883, 884, 885, 886]
        );
        assert_eq!(
            surface_frames
                .iter()
                .map(|frame| frame.middle_raw)
                .collect::<Vec<_>>(),
            [600, 600, 600, 600, 600, 600, 600, 600, 600, 500, 400, 300, 200, 100]
        );
        assert!(surface_frames.iter().all(|frame| frame.scale_raw == 0x100));
    }

    #[test]
    fn meteor_frames_follow_retail_floored_age_formula() {
        let mut fx = full_rate_world_fx();
        fx.queue_meteor_impact([0.0; 3], 0, None);
        fx.process_pending();

        let mut scatter = fx.test_particles_in_virgin_birth_order()[0];
        assert_eq!(scatter.current_sprite_id(), 699);
        scatter.age_ticks = 1.999;
        assert_eq!(scatter.current_sprite_id(), 699);
        scatter.age_ticks = 2.0;
        assert_eq!(scatter.current_sprite_id(), 700);
        scatter.age_ticks = 6.0;
        assert_eq!(scatter.current_sprite_id(), 702);
        scatter.age_ticks = 8.0;
        assert_eq!(scatter.current_sprite_id(), 699);

        let mut surface = fx.test_particles_in_virgin_birth_order()[METEOR_IMPACT_SCATTER_COUNT];
        assert_eq!(surface.current_sprite_id(), 873);
        surface.age_ticks = 3.999;
        assert_eq!(surface.current_sprite_id(), 873);
        surface.age_ticks = 4.0;
        assert_eq!(surface.current_sprite_id(), 874);
        surface.age_ticks = 36.0;
        assert_eq!(surface.current_frame().sprite_id, 882);
        assert_eq!(surface.current_frame().middle_raw, 500);

        // The retail age field is one byte. This synthetic over-lifetime age
        // verifies frame selection narrows after flooring instead of allowing
        // Rust's float-to-u8 saturation to pin it at 255.
        surface.age_ticks = 256.0;
        assert_eq!(surface.current_frame().sprite_id, 873);
        assert_eq!(surface.current_frame().middle_raw, 600);
    }

    #[test]
    fn terrain_light_radius_preserves_retail_height_and_age_boundaries() {
        assert_eq!(particle_terrain_light_radius_raw(0, 0, 0, 0), None);

        // Positive middle values divide their 8.8 radius by height above the
        // coarse terrain sample, clamping that height to one raw cell.
        assert_eq!(
            particle_terrain_light_radius_raw(600, 0, 0x80, 0),
            Some(600)
        );
        assert_eq!(
            particle_terrain_light_radius_raw(600, 0, 0x200, 0),
            Some(300)
        );
        assert_eq!(
            particle_terrain_light_radius_raw(600, 0, 0, 0x80),
            Some(600)
        );

        // Negative middle values encode a full-size radius whose fade begins
        // at age 201, not 200.
        assert_eq!(
            particle_terrain_light_radius_raw(-8, 200, 0, 0),
            Some(0x800)
        );
        assert_eq!(particle_terrain_light_radius_raw(-8, 201, 0, 0), Some(2011));
        assert_eq!(particle_terrain_light_radius_raw(-8, 255, 0, 0), Some(36));
    }

    #[test]
    fn particle_terrain_light_uses_simulation_position_and_signed_coarse_ground() {
        let mut terrain = flat_terrain(4, 0);
        terrain_cell_mut(&mut terrain, 255, 0).height = (-4_i8) as u8;

        let mut fx = full_rate_world_fx();
        fx.queue_meteor_impact([255.75, 2.5, 0.25], 0, None);
        fx.process_pending();
        let mut surface = fx.test_particles_in_virgin_birth_order()[METEOR_IMPACT_SCATTER_COUNT];
        surface.presentation_offset = [20.0, 100.0, 30.0];

        let light = surface.terrain_light_emission(&terrain).unwrap();
        assert_eq!(light.x_raw, -0x40);
        assert_eq!(light.z_raw, 0x40);
        // Cell (255, 0) has ground -4*32=-128. Particle Y is 2.5*256=640,
        // so frame-zero middle 600 produces 600*256/(640-(-128))=200.
        assert_eq!(light.radius_raw, 200);

        let scatter = fx.test_particles_in_virgin_birth_order()[0];
        assert_eq!(scatter.terrain_light_emission(&terrain), None);
    }

    #[test]
    fn meteor_particles_expire_at_the_recovered_lifetime() {
        let mut fx = full_rate_world_fx();
        fx.update(ParticleUpdateRequest::dry(0, 0));
        fx.queue_meteor_impact([0.0; 3], 0, None);
        fx.process_pending();
        fx.update(ParticleUpdateRequest::dry(
            20_000,
            u32::from(METEOR_SURFACE_LIFETIME_TICKS),
        ));
        assert_eq!(
            fx.test_particles_in_virgin_birth_order()
                .iter()
                .filter(|particle| particle.source_class == METEOR_SURFACE_CLASS)
                .count(),
            1
        );
        fx.update(ParticleUpdateRequest::dry(
            20_000,
            u32::from(METEOR_SCATTER_LIFETIME_TICKS),
        ));
        assert_eq!(
            fx.test_particles_in_virgin_birth_order()
                .iter()
                .filter(|particle| particle.source_class == METEOR_SCATTER_CLASS)
                .count(),
            METEOR_IMPACT_SCATTER_COUNT
        );
        assert!(!fx
            .test_particles_in_virgin_birth_order()
            .iter()
            .any(|particle| particle.source_class == METEOR_SURFACE_CLASS));
        fx.update(ParticleUpdateRequest::dry(
            20_000,
            u32::from(METEOR_SCATTER_LIFETIME_TICKS) + 1,
        ));
        assert!(!fx
            .test_particles_in_virgin_birth_order()
            .iter()
            .any(|particle| particle.source_class == METEOR_SCATTER_CLASS));
        assert!(fx
            .test_particles_in_virgin_birth_order()
            .iter()
            .all(|particle| particle.source_class == METEOR_TRAIL_CLASS));
        fx.update(ParticleUpdateRequest::dry(
            0,
            u32::from(METEOR_SCATTER_LIFETIME_TICKS) + 1_000,
        ));
        assert!(fx.test_particles_in_virgin_birth_order().is_empty());
    }

    #[test]
    fn meteor_impact_presentation_clears_without_camera_motion() {
        const VIEWPORT: [u32; 2] = [320, 240];
        const FAR_DEPTH_RAW: i32 = 0x1800;

        let mut fx = full_rate_world_fx();
        fx.update(ParticleUpdateRequest::dry(0, 0));
        fx.queue_meteor_impact([0.0; 3], 0, None);
        fx.process_pending();

        let mut prepared_count = |class, tick| {
            fx.update(ParticleUpdateRequest::dry(20_000, tick));
            fx.prepare_presentation(VIEWPORT, FAR_DEPTH_RAW, |_| {
                center_projection([160, 120], 0x100, 0)
            })
            .particles()
            .filter(|prepared| prepared.particle.source_class == class)
            .count()
        };

        // Keep the projector bit-for-bit constant across every traversal.
        // Retail keeps equality with the lifetime visible for one final frame,
        // then removes the particle on the next tick independently of camera
        // movement.
        assert_eq!(
            prepared_count(
                METEOR_SURFACE_CLASS,
                u32::from(METEOR_SURFACE_LIFETIME_TICKS)
            ),
            1
        );
        assert_eq!(
            prepared_count(
                METEOR_SURFACE_CLASS,
                u32::from(METEOR_SURFACE_LIFETIME_TICKS) + 1
            ),
            0
        );
        assert_eq!(
            prepared_count(
                METEOR_SCATTER_CLASS,
                u32::from(METEOR_SCATTER_LIFETIME_TICKS)
            ),
            METEOR_IMPACT_SCATTER_COUNT
        );
        assert_eq!(
            prepared_count(
                METEOR_SCATTER_CLASS,
                u32::from(METEOR_SCATTER_LIFETIME_TICKS) + 1
            ),
            0
        );
        drop(prepared_count);

        // Class-31 debris trails legitimately outlive their class-16 parents,
        // but they too clear without any projector or camera-state change.
        fx.update(ParticleUpdateRequest::dry(
            0,
            u32::from(METEOR_SCATTER_LIFETIME_TICKS) + 1_000,
        ));
        let frame = fx.prepare_presentation(VIEWPORT, FAR_DEPTH_RAW, |_| {
            center_projection([160, 120], 0x100, 0)
        });
        assert_eq!(frame.particles().len(), 0);
    }

    #[test]
    fn scatter_integrates_then_gravity_applies_while_surface_velocity_is_constant() {
        let mut fx = full_rate_world_fx();
        fx.queue_meteor_impact([10.0, 20.0, 30.0], 0, None);
        fx.process_pending();
        let scatter_before = fx.test_particles_in_virgin_birth_order()[0];
        let surface_before = fx.test_particles_in_virgin_birth_order()[METEOR_IMPACT_SCATTER_COUNT];
        let dt = 0.1;

        fx.update(ParticleUpdateRequest::dry(100_000, 5));
        let scatter_after = fx.test_particles_in_virgin_birth_order()[0];
        let surface_after = fx.test_particles_in_virgin_birth_order()[METEOR_IMPACT_SCATTER_COUNT];

        // Position uses the incoming velocity; gravity is applied by the
        // subsequent class callback and affects the next integration.
        assert!(
            (scatter_after.position[1]
                - (scatter_before.position[1] + scatter_before.velocity[1] * dt))
                .abs()
                < 1e-5
        );
        assert!(
            (scatter_after.velocity[1]
                - (scatter_before.velocity[1] - METEOR_SCATTER_GRAVITY_WORLD_PER_SEC2 * dt))
                .abs()
                < 1e-5
        );

        assert!(
            (surface_after.position[1]
                - (surface_before.position[1] + surface_before.velocity[1] * dt))
                .abs()
                < 1e-5
        );
        assert_eq!(surface_after.velocity, surface_before.velocity);
    }

    #[test]
    fn earlier_physical_trail_slots_wait_until_the_next_update_pass() {
        let mut fx = full_rate_world_fx();
        fx.queue_meteor_impact([10.0, 20.0, 30.0], 0, None);
        fx.process_pending();
        fx.update(ParticleUpdateRequest::dry(20_000, 1));

        // Virgin impact allocation puts scatter parents in slots 199..190.
        // Physical traversal reaches them as 190..199 and allocates their
        // children into 188..179: records already passed by this update.
        for parent_slot in 190..=199 {
            let child_slot = 378 - parent_slot;
            let parent = fx.particles.slots[parent_slot].unwrap();
            let child = fx.particles.slots[child_slot].unwrap();
            assert_eq!(child.position, parent.position);
            assert_eq!(child.sprite_id, 895);
            assert_eq!(child.current_sprite_id(), 895);
            assert_eq!(child.age_ticks, 0.0);
            assert_eq!(child.lifetime_ticks, METEOR_TRAIL_LIFETIME_TICKS);
            assert_eq!(child.collision_radius_raw, METEOR_TRAIL_RADIUS_RAW);
            assert_eq!(child.draw_scale_raw, METEOR_TRAIL_DRAW_SCALE_RAW);
            assert_eq!(
                child.size_jitter_divisor_raw,
                METEOR_TRAIL_SIZE_JITTER_DIVISOR_RAW
            );
            assert_eq!(
                child.velocity,
                raw_velocity_to_world(METEOR_TRAIL_INITIAL_VELOCITY_RAW)
            );
        }

        fx.update(ParticleUpdateRequest::dry(20_000, 2));
        for child_slot in 179..=188 {
            assert_eq!(fx.particles.slots[child_slot].unwrap().age_ticks, 1.0);
        }
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn later_physical_trail_slot_updates_in_the_parent_pass() {
        let mut fx = WorldFx::new();
        let parent = pool_test_particle(METEOR_SCATTER_CLASS, 10.0);
        for _ in 0..MAX_WORLD_PARTICLES {
            fx.particles.allocate(parent).unwrap();
        }
        // Leave only physical slot zero active. Freeing in ascending order
        // makes slot one the free tail and therefore the inline child slot.
        for slot in 1..MAX_WORLD_PARTICLES {
            assert!(fx.particles.free(slot));
        }
        fx.last_retail_tick = Some(0);
        fx.update(ParticleUpdateRequest::dry(20_000, 1));

        let child = fx.particles.slots[1].unwrap();
        assert_eq!(child.source_class, METEOR_TRAIL_CLASS);
        assert_eq!(child.age_ticks, 1.0);
        assert!(child.position[1] > fx.particles.slots[0].unwrap().position[1]);
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn trail_animation_and_post_integration_drag_match_recovered_contract() {
        let mut particle = meteor_trail_particle([1.0, 2.0, 3.0]);
        particle.age_ticks = 1.999;
        assert_eq!(particle.current_sprite_id(), 895);
        particle.age_ticks = 2.0;
        assert_eq!(particle.current_sprite_id(), 896);
        particle.age_ticks = 44.0;
        assert_eq!(particle.current_sprite_id(), 905);
        particle.age_ticks = 46.0;
        assert_eq!(particle.current_sprite_id(), 895);

        particle.age_ticks = 0.0;
        let before = particle;
        let dt = 0.02;
        let mut fx = WorldFx::new();
        fx.particles.test_allocate(particle);
        fx.update(ParticleUpdateRequest::dry(20_000, 1));
        let after = fx.test_particles_in_virgin_birth_order()[0];
        assert!((after.position[1] - (before.position[1] + before.velocity[1] * dt)).abs() < 1e-5);
        let drag_factor = (1.0 - METEOR_TRAIL_DRAG_PER_SECOND * dt).max(0.0);
        assert!((after.velocity[1] - before.velocity[1] * drag_factor).abs() < 1e-5);
    }

    #[test]
    fn world_particle_pool_never_exceeds_retail_200_record_bound() {
        let mut fx = WorldFx::new();
        fx.queue_meteor_impact([0.0; 3], 0, None);
        fx.process_pending();
        for step in 0..150_u32 {
            fx.update(ParticleUpdateRequest::dry(10_000, step.div_ceil(2)));
            assert!(fx.test_particles_in_virgin_birth_order().len() <= MAX_WORLD_PARTICLES);
        }
    }

    #[test]
    fn update_moves_wraps_and_expires_particles() {
        let mut fx = WorldFx::new();
        fx.update(ParticleUpdateRequest::dry(0, 0));
        fx.queue(WorldEvent::ParticleBurst(ParticleBurst {
            origin: [255.5, 1.0, 0.25],
            particles: vec![ParticleSpawn {
                sprite_id: 699,
                source_class: 0x55,
                offset: [0.0; 3],
                presentation_offset: [0.0; 3],
                velocity: [2.0, -1.0, -2.0],
                owner_id: None,
                attached_owner_handle: None,
                attached_offset_raw: [0; 3],
                source_entity_type_at_birth: None,
                suppresses_impact_damage: false,
                lifetime_ticks: 10,
                collision_radius_raw: METEOR_SCATTER_RADIUS_RAW,
                frame_middle_raw: 0,
                frame_scale_raw: 0x100,
                animation_rate_raw: METEOR_SCATTER_ANIMATION_RATE_RAW,
                draw_scale_raw: METEOR_SCATTER_DRAW_SCALE_RAW,
                size_jitter_divisor_raw: METEOR_SCATTER_SIZE_JITTER_DIVISOR_RAW,
            }],
        }));
        fx.process_pending();

        fx.update(ParticleUpdateRequest::dry(100_000, 5));
        let particle = fx.test_particles_in_virgin_birth_order()[0];
        assert!((particle.position[0] - 255.7).abs() < 1e-5);
        assert!((particle.position[1] - 0.9).abs() < 1e-5);
        assert!((particle.position[2] - 0.05).abs() < 1e-5);
        assert!((particle.normalized_age() - 0.5).abs() < 1e-5);

        fx.update(ParticleUpdateRequest::dry(100_000, 10));
        assert_eq!(fx.test_particles_in_virgin_birth_order().len(), 1);
        fx.update(ParticleUpdateRequest::dry(20_000, 11));
        assert!(fx.test_particles_in_virgin_birth_order().is_empty());
    }

    #[test]
    fn pending_particles_materialize_at_age_zero() {
        let mut fx = WorldFx::new();
        fx.queue_meteor_impact([0.0; 3], 0, None);
        fx.update(ParticleUpdateRequest::dry(125_000, 6));
        assert!(fx
            .test_particles_in_virgin_birth_order()
            .iter()
            .all(|particle| particle.age_ticks == 0.0));
    }

    #[test]
    fn same_retail_tick_moves_particles_without_aging_them() {
        let mut fx = WorldFx::new();
        fx.particles
            .test_allocate(meteor_trail_particle([1.0, 2.0, 3.0]));
        fx.update(ParticleUpdateRequest::dry(0, 42));
        let before = fx.test_particles_in_virgin_birth_order()[0];

        fx.update(ParticleUpdateRequest::dry(20_000, 42));

        let after = fx.test_particles_in_virgin_birth_order()[0];
        assert_eq!(after.age_ticks, before.age_ticks);
        assert!(
            (after.position[1] - (before.position[1] + before.velocity[1] * 0.02)).abs() < 1.0e-6
        );
    }

    #[test]
    fn particle_age_uses_wrapping_retail_tick_delta() {
        let mut fx = WorldFx::new();
        fx.particles.test_allocate(meteor_trail_particle([0.0; 3]));
        fx.update(ParticleUpdateRequest::dry(0, u32::MAX - 1));

        fx.update(ParticleUpdateRequest::dry(0, 1));

        assert_eq!(fx.test_particles_in_virgin_birth_order()[0].age_ticks, 3.0);
    }

    #[test]
    fn clear_resets_the_retail_tick_latch() {
        let mut fx = WorldFx::new();
        fx.update(ParticleUpdateRequest::dry(0, 100));
        fx.clear();
        fx.particles.test_allocate(meteor_trail_particle([0.0; 3]));

        fx.update(ParticleUpdateRequest::dry(0, 0));

        assert_eq!(fx.test_particles_in_virgin_birth_order()[0].age_ticks, 0.0);
    }

    #[test]
    fn motion_delta_is_defensively_capped_without_inventing_ticks() {
        let mut fx = WorldFx::new();
        fx.particles.test_allocate(meteor_trail_particle([0.0; 3]));
        fx.update(ParticleUpdateRequest::dry(0, 7));
        let before = fx.test_particles_in_virgin_birth_order()[0];

        fx.update(ParticleUpdateRequest::dry(500_000, 7));

        let after = fx.test_particles_in_virgin_birth_order()[0];
        assert_eq!(after.age_ticks, before.age_ticks);
        assert!(
            (after.position[1] - (before.position[1] + before.velocity[1] * 0.125)).abs() < 1.0e-6
        );
    }

    #[test]
    fn static_kind9_scatter_preserves_exact_class79_direction_table_arguments() {
        let mut fx = full_rate_world_fx();
        let center = [0x1200, 0x0340, -0x2100];
        let rng_before = fx.rng_state;

        fx.emit_static_kind9_scatter_raw(center);

        assert_eq!(fx.direction_cursor, STATIC_KIND_9_SCATTER_COUNT as usize);
        assert_eq!(fx.rng_state, rng_before, "FUN_004407D0 consumes no RNG");
        let particles = fx.test_particles_in_virgin_birth_order();
        assert_eq!(particles.len(), STATIC_KIND_9_SCATTER_COUNT as usize);
        let velocity_y_bias = i32::from(
            particle_descriptor(STATIC_DESTRUCTION_EFFECT_CLASS_79)
                .expect("class 79 descriptor")
                .spawn_velocity_y_bias_raw(),
        );
        for (index, particle) in particles.iter().enumerate() {
            let direction = RETAIL_DIRECTION_TABLE_RAW[index + 1];
            let l1_length = direction
                .iter()
                .map(|component| i32::from(*component).abs())
                .sum::<i32>();
            let expected_offset = [
                0x10 * i32::from(direction[0]) / l1_length,
                0x10 * i32::from(direction[1]) / l1_length,
                0x10 * i32::from(direction[1]) / l1_length,
            ];
            let expected_position =
                std::array::from_fn(|axis| center[axis].wrapping_add(expected_offset[axis] as i16));
            let mut expected_velocity = direction.map(|component| i32::from(component) >> 4);
            expected_velocity[1] += velocity_y_bias;

            assert_eq!(particle.source_class, STATIC_DESTRUCTION_EFFECT_CLASS_79);
            assert_eq!(particle.owner_id, Some(0));
            assert_eq!(particle.source_entity_type_at_birth, Some(0));
            assert_eq!(
                static_ballistic_entity_hit_delivery(particle),
                DamageDeliveryRecord {
                    packet: BALLISTIC_PARTICLE_DAMAGE_PACKET,
                    source_entity_type_raw: 0,
                    owner_handle: 0,
                },
                "static scatter retains its known zero birth provenance"
            );
            assert!(!particle.suppresses_impact_damage);
            assert_eq!(world_position_to_raw(particle.position), expected_position);
            assert_eq!(
                particle.velocity.map(world_velocity_component_to_raw),
                expected_velocity
            );
        }
    }

    fn static_ballistic_entity_hit_delivery(particle: &WorldParticle) -> DamageDeliveryRecord {
        let pool = solid_sphere_pool(100);
        let entities = [collision_entity(9, particle.position, 100)];
        let mut particle = *particle;
        let Some(BallisticSweepImpact::Entity(hit)) = detect_damage_projectile_sweep(
            ProjectileModelSweepProgram::for_class(particle.source_class).unwrap(),
            0,
            &mut particle,
            ProjectileSweepFrame {
                collision: ParticleCollisionContext {
                    entities: &entities,
                    model_pool: &pool,
                },
                terrain_context: None,
                retail_tick: 0,
                terrain_type_mutations: &[],
            },
        ) else {
            panic!("known zero owner must not exclude the live target from the ballistic sweep")
        };
        assert_eq!(hit.target_entity_id, 9);
        hit.damage_delivery_record()
            .expect("retained birth provenance")
    }

    #[test]
    fn static_kind9_scatter_keeps_minimum_one_and_stops_on_first_allocation_failure() {
        let mut cold = WorldFx::new();
        cold.emit_static_kind9_scatter_raw([0; 3]);
        assert_eq!(cold.particle_count(), 1);
        assert_eq!(cold.direction_cursor, 1);

        let mut saturated = full_rate_world_fx();
        let template_burst = single_descriptor_particle_burst(
            [0.0; 3],
            STATIC_DESTRUCTION_EFFECT_CLASS_79,
            [0; 3],
            None,
        );
        saturated.particles.test_fill_to(
            MAX_WORLD_PARTICLES,
            world_particle(template_burst.origin, template_burst.particles[0]),
        );
        let rng_before = saturated.rng_state;

        saturated.emit_static_kind9_scatter_raw([0; 3]);

        assert_eq!(
            saturated.direction_cursor, 1,
            "the cursor advances before the first rejected allocation"
        );
        assert_eq!(saturated.particle_count(), MAX_WORLD_PARTICLES);
        assert_eq!(saturated.rng_state, rng_before);
    }

    #[test]
    fn common_explosion_bundle_preserves_scatter_rng_debris_and_sound_order() {
        let mut fx = full_rate_world_fx();
        let center = [0x1200, 0x0340, -0x2100];
        let light = fx.emit_common_explosion_bundle_raw(center, 0x0200);

        assert_eq!(
            light,
            TerrainExplosionLight {
                x_raw: center[0],
                z_raw: center[2],
                radius_raw: 0x600,
            }
        );
        assert_eq!(fx.direction_cursor, COMMON_EXPLOSION_SCATTER_COUNT);

        let particles = fx.test_particles_in_virgin_birth_order();
        assert_eq!(
            particles.len(),
            COMMON_EXPLOSION_SCATTER_COUNT + COMMON_EXPLOSION_DEBRIS_COUNT
        );
        for (index, particle) in particles[..COMMON_EXPLOSION_SCATTER_COUNT]
            .iter()
            .enumerate()
        {
            let direction = RETAIL_DIRECTION_TABLE_RAW[index + 1];
            let l1_length = direction
                .iter()
                .map(|component| i32::from(*component).abs())
                .sum::<i32>();
            let expected_offset = [
                0x0200 * i32::from(direction[0]) / l1_length,
                0x0200 * i32::from(direction[1]) / l1_length,
                0x0200 * i32::from(direction[1]) / l1_length,
            ];
            let expected_position =
                std::array::from_fn(|axis| center[axis].wrapping_add(expected_offset[axis] as i16));
            assert_eq!(particle.source_class, COMMON_EXPLOSION_SCATTER_CLASS);
            assert_eq!(particle.owner_id, Some(0));
            assert_eq!(particle.source_entity_type_at_birth, Some(0));
            assert_eq!(
                static_ballistic_entity_hit_delivery(particle),
                DamageDeliveryRecord {
                    packet: BALLISTIC_PARTICLE_DAMAGE_PACKET,
                    source_entity_type_raw: 0,
                    owner_handle: 0,
                }
            );
            assert_eq!(world_position_to_raw(particle.position), expected_position);
            let expected_velocity = direction.map(|component| (i32::from(component) * 8) >> 4);
            assert_eq!(
                particle.velocity.map(world_velocity_component_to_raw),
                expected_velocity
            );
        }

        let mut expected_rng = 0;
        let dead_word = retail_random_u16(&mut expected_rng);
        assert_eq!(dead_word, 0x0026);
        let sound_word = retail_random_u16(&mut expected_rng);
        let layouts = [(-0x52, -0x52), (-0x52, 0x12), (0x12, 0x12), (0x12, -0x52)];
        let mut expected_debris = Vec::new();
        for (index, (x_bias, y_bias)) in layouts.into_iter().enumerate() {
            let class_mask = if index == 0 { 1 } else { 7 };
            let source_class = 0x16 + (retail_random_u16(&mut expected_rng) as u8 & class_mask);
            let x_offset = (retail_random_u16(&mut expected_rng) & 0x3f) as i16 + x_bias;
            let y_offset = (retail_random_u16(&mut expected_rng) & 0x3f) as i16 + y_bias;
            let z_offset = (retail_random_u16(&mut expected_rng) & 0x3f) as i16 - 0x20;
            expected_debris.push((
                source_class,
                [
                    center[0].wrapping_add(x_offset),
                    center[1]
                        .wrapping_add(COMMON_EXPLOSION_DEBRIS_Y_LIFT_RAW)
                        .wrapping_add(y_offset),
                    center[2].wrapping_add(z_offset),
                ],
            ));
        }
        assert_eq!(fx.rng_state, expected_rng);
        for (particle, (source_class, position_raw)) in particles[COMMON_EXPLOSION_SCATTER_COUNT..]
            .iter()
            .zip(expected_debris)
        {
            assert_eq!(particle.source_class, source_class);
            assert_eq!(particle.owner_id, Some(0));
            assert_eq!(particle.source_entity_type_at_birth, Some(0));
            assert_eq!(world_position_to_raw(particle.position), position_raw);
        }
        assert_eq!(
            fx.take_positional_sounds(),
            vec![PositionalSoundEvent {
                sound_id: COMMON_EXPLOSION_SOUND_ID,
                position: raw_position_to_world([
                    center[0],
                    center[1].wrapping_add(COMMON_EXPLOSION_DEBRIS_Y_LIFT_RAW),
                    center[2],
                ]),
                frequency_q16: 0xC000 + u32::from(sound_word >> 1),
            }]
        );
    }

    #[test]
    fn common_explosion_scatter_failure_does_not_skip_rng_sound_or_debris_attempts() {
        let mut fx = WorldFx::new();
        fx.particles.test_fill_to(
            MAX_WORLD_PARTICLES,
            pool_test_particle(METEOR_SCATTER_CLASS, 0.0),
        );

        fx.emit_common_explosion_bundle_raw([0; 3], 0x100);

        // Same-priority age-zero records cannot be recycled, so the first
        // class-30 allocation rejects and FUN_004407D0 returns immediately.
        assert_eq!(fx.direction_cursor, 1);
        assert_eq!(fx.particle_count(), MAX_WORLD_PARTICLES);
        let mut expected_rng = 0;
        for _ in 0..(2 + COMMON_EXPLOSION_DEBRIS_COUNT * 4) {
            retail_random_u16(&mut expected_rng);
        }
        assert_eq!(fx.rng_state, expected_rng);
        assert_eq!(fx.take_positional_sounds().len(), 1);
    }

    #[test]
    fn ballistic_damage_delivery_requires_both_birth_provenance_words() {
        let complete = BallisticDamageRequest {
            packet: PRIMARY_PROJECTILE_DAMAGE_PACKET,
            source_entity_type_at_birth: Some(46),
            source_owner_id: Some(0x04aa_0001),
        };
        assert_eq!(
            complete.delivery_record(),
            Some(DamageDeliveryRecord {
                packet: PRIMARY_PROJECTILE_DAMAGE_PACKET,
                source_entity_type_raw: 46,
                owner_handle: 0x04aa_0001,
            })
        );
        assert_eq!(
            BallisticDamageRequest {
                source_entity_type_at_birth: None,
                ..complete
            }
            .delivery_record(),
            None
        );
        assert_eq!(
            BallisticDamageRequest {
                source_owner_id: None,
                ..complete
            }
            .delivery_record(),
            None
        );
    }

    #[test]
    fn ballistic_e4f0_family_and_response_table_come_from_exact_descriptor_data() {
        let classes = (0..PARTICLE_DESCRIPTORS.len() as u8)
            .filter(|source_class| uses_ballistic_surface_callback(*source_class))
            .collect::<Vec<_>>();
        assert_eq!(classes, vec![16, 30, 58, 79, 93, 94, 95]);
        assert_eq!(
            BALLISTIC_SURFACE_RESPONSE_CLASS_BY_SELECTOR,
            [7, 8, 9, 10, 7, 11, 13, 12, 59, 10, 71, 70, 72]
        );
        assert_eq!(
            classes
                .iter()
                .copied()
                .filter(|source_class| uses_ballistic_trail_update_callback(*source_class))
                .collect::<Vec<_>>(),
            vec![16, 30, 58],
            "E4F0 collision dispatch is independent of the update callback slot"
        );
        assert!(classes.into_iter().all(|source_class| {
            let descriptor = particle_descriptor(source_class).unwrap();
            descriptor.raw_byte(0x0e) == 3
                && descriptor.raw_u32(0x18) == BALLISTIC_SURFACE_CALLBACK_VA
                && descriptor.raw_u32(0x1c) == BALLISTIC_ENTITY_CALLBACK_VA
                && descriptor.raw_u32(0x20) == BALLISTIC_DAMAGE_PACKET_VA
                && descriptor.raw_u32(0x24) == BALLISTIC_STATIC_CALLBACK_VA
                && descriptor.flags() & 0x04 != 0
        }));
        assert_eq!(
            (0..PARTICLE_DESCRIPTORS.len() as u8)
                .filter(|source_class| uses_ballistic_sweep_callbacks(*source_class))
                .collect::<Vec<_>>(),
            vec![16, 30, 58, 79, 93, 94, 95],
            "sweep and surface slots coincide in retail data but remain independent policies"
        );
    }

    #[test]
    fn ballistic_entity_sweep_emits_trail_before_hit_visual_and_typed_damage() {
        let mut parent =
            descriptor_test_particle(METEOR_SCATTER_CLASS, [4.0, 2.0, 5.0], [0; 3], None);
        parent.source_entity_type_at_birth = Some(68);
        let mut fx = WorldFx::new();
        let parent_slot = fx.particles.allocate(parent).unwrap();
        let pool = solid_sphere_pool(100);
        let entities = [collision_entity(9, parent.position, 100)];
        let mut terrain = flat_terrain(16, 0);
        *terrain_cell_mut(&mut terrain, 3, 4) = TerrainCell {
            height: 16,
            attribute: 1,
            terrain_type: 0x10,
        };
        let objects = terrain_object_table(1, [0; 4], KIND_9_STATIC_OBJECT);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: Some(&objects),
            ground_response_selectors: [0; 8],
            water_response_selectors: [0; 8],
        };
        let mut entity_handler_calls = 0;
        let mut static_handler_calls = 0;

        let outcome = fx.update_with_event_handlers(
            ParticleUpdateRequest::terrain(0, 0, context).with_callbacks(ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &entities,
                    model_pool: &pool,
                }),
            }),
            |_, _| {
                entity_handler_calls += 1;
                ParticleCollisionCacheRefresh::Unchanged
            },
            |_, event| {
                expect_static_event(event);
                static_handler_calls += 1;
                ParticleTerrainResponse::Unhandled
            },
        );

        assert_eq!(entity_handler_calls, 1);
        assert_eq!(static_handler_calls, 0);
        assert_eq!(
            outcome.entity_impacts,
            vec![ParticleEntityImpact {
                source_particle_class: METEOR_SCATTER_CLASS,
                impact_position_argument_va: retail_particle_impact_position_argument_va(
                    parent_slot,
                ),
                target_entity_id: 9,
                position_world: parent.position,
                velocity_raw: [0; 3],
                damage: Some(BallisticDamageRequest {
                    packet: BALLISTIC_PARTICLE_DAMAGE_PACKET,
                    source_entity_type_at_birth: Some(68),
                    source_owner_id: None,
                }),
            }]
        );
        assert!(fx.particles.slots[parent_slot].is_none());
        let trail_slot = fx
            .particles
            .slots
            .iter()
            .position(|particle| {
                particle.is_some_and(|particle| particle.source_class == METEOR_TRAIL_CLASS)
            })
            .unwrap();
        let visual_slot = fx
            .particles
            .slots
            .iter()
            .position(|particle| {
                particle.is_some_and(|particle| {
                    particle.source_class == PRIMARY_ABOVE_WATER_IMPACT_CLASS
                })
            })
            .unwrap();
        assert!(
            trail_slot > visual_slot,
            "virgin slots descend, so the callback trail must allocate before F610"
        );
        assert_eq!(
            fx.particles.slots[visual_slot].unwrap().position,
            parent.position
        );
        assert!(outcome.primary_impacts.is_empty());
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn ballistic_inert_flag_preserves_visual_and_delete_but_suppresses_damage() {
        let mut parent =
            descriptor_test_particle(METEOR_SCATTER_CLASS, [4.0, 2.0, 5.0], [0; 3], Some(7));
        parent.suppresses_impact_damage = true;
        let mut fx = WorldFx::new();
        fx.particles.test_allocate(parent);
        let pool = solid_sphere_pool(100);
        let entities = [collision_entity(9, parent.position, 100)];

        let outcome = fx.update(ParticleUpdateRequest::dry(0, 0).with_callbacks(
            ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &entities,
                    model_pool: &pool,
                }),
            },
        ));

        assert!(matches!(
            outcome.entity_impacts.as_slice(),
            [ParticleEntityImpact {
                target_entity_id: 9,
                damage: None,
                ..
            }]
        ));
        assert!(!fx
            .particles
            .slots
            .iter()
            .flatten()
            .any(|particle| particle.source_class == METEOR_SCATTER_CLASS));
        let children = fx
            .particles
            .slots
            .iter()
            .flatten()
            .filter(|particle| {
                matches!(
                    particle.source_class,
                    METEOR_TRAIL_CLASS | PRIMARY_ABOVE_WATER_IMPACT_CLASS
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(children.len(), 2);
        assert!(children
            .iter()
            .all(|particle| particle.suppresses_impact_damage));
    }

    #[test]
    fn ballistic_static_sweep_uses_refined_probe_and_short_circuits_surface_tail() {
        let mut terrain = flat_terrain(16, 0);
        *terrain_cell_mut(&mut terrain, 3, 4) = TerrainCell {
            height: 16,
            attribute: 1,
            terrain_type: 0x18,
        };
        let objects = terrain_object_table(1, [0, 0, 0, 0], 29);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: Some(&objects),
            ground_response_selectors: [5; 8],
            water_response_selectors: [7; 8],
        };
        let pool = solid_sphere_pool(200);
        let parent =
            descriptor_test_particle(METEOR_SCATTER_CLASS, [4.0, 2.0, 5.0], [0; 3], Some(66));
        let mut fx = WorldFx::new();
        let parent_slot = fx.particles.allocate(parent).unwrap();
        let mut handled = None;
        let mut handler_saw_parent_trail_and_visual = false;
        let mut scheduler = StaticDamageScheduler::new();
        let mut submission = None;
        let rng_before = fx.rng_state;

        let outcome = fx.update_with_event_handlers(
            ParticleUpdateRequest::terrain(0, 0, context).with_callbacks(ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &[],
                    model_pool: &pool,
                }),
            }),
            |_, impact| panic!("unexpected entity impact: {impact:?}"),
            |live_fx, impact| {
                let impact = expect_static_event(impact);
                handler_saw_parent_trail_and_visual =
                    live_fx.particles.slots[parent_slot]
                        .is_some_and(|particle| particle.source_class == METEOR_SCATTER_CLASS)
                        && live_fx
                            .particles
                            .slots
                            .iter()
                            .flatten()
                            .any(|particle| particle.source_class == METEOR_TRAIL_CLASS)
                        && live_fx.particles.slots.iter().flatten().any(|particle| {
                            particle.source_class == PRIMARY_ABOVE_WATER_IMPACT_CLASS
                        });
                let current = impact.current.expect("unsuppressed ballistic target");
                submission = Some(scheduler.submit_hit(
                    current.target,
                    impact.damage.unwrap().packet,
                    &mut || live_fx.next_shared_retail_random_u16(),
                ));
                handled = Some(impact);
                ParticleTerrainResponse::Unhandled
            },
        );

        assert!(handler_saw_parent_trail_and_visual);
        assert_eq!(
            outcome.ballistic_static_impacts,
            vec![ParticleStaticImpact {
                source_particle_class: METEOR_SCATTER_CLASS,
                source_owner_id: Some(66),
                position_world: [4.0, 2.0, 5.0],
                cell: [3, 4],
                attribute: 1,
                terrain_type: 0x18,
                model_id: 0,
                kind_index: 29,
                current: Some(CurrentStaticDamageSnapshot {
                    target: crate::static_damage::StaticDamageTarget {
                        cell: [3, 4],
                        state: crate::static_damage::StaticDamageTargetState {
                            kind_index: 29,
                            terrain_type: 0x18,
                            terrain_height_byte: 16,
                            collision_radius_raw: 200,
                            effect_extent_raw: 0,
                        },
                    },
                    attribute: 1,
                    model_id: 0,
                }),
                damage: Some(BallisticDamageRequest {
                    packet: BALLISTIC_PARTICLE_DAMAGE_PACKET,
                    source_entity_type_at_birth: None,
                    source_owner_id: Some(66),
                }),
            }]
        );
        assert_eq!(handled, outcome.ballistic_static_impacts.first().copied());
        assert_eq!(
            submission,
            Some(StaticDamageOutcome::NoDamage { severity_raw: 0 })
        );
        assert_eq!(scheduler.active_program_count(), 0);
        assert_eq!(fx.rng_state, rng_before);
        assert!(!fx
            .particles
            .slots
            .iter()
            .flatten()
            .any(|particle| particle.source_class == METEOR_SCATTER_CLASS));
        assert!(fx
            .particles
            .slots
            .iter()
            .flatten()
            .any(|particle| particle.source_class == PRIMARY_ABOVE_WATER_IMPACT_CLASS));
        assert!(!fx
            .particles
            .slots
            .iter()
            .flatten()
            .any(|particle| particle.source_class == 11));
        assert!(fx.take_positional_sounds().is_empty());
    }

    #[test]
    fn ballistic_static_suppression_keeps_trail_visual_and_parent_lifetime_without_delivery() {
        let mut terrain = flat_terrain(16, 0);
        *terrain_cell_mut(&mut terrain, 3, 4) = TerrainCell {
            height: 16,
            attribute: 1,
            terrain_type: 0x18,
        };
        let objects = terrain_object_table(1, [0; 4], 29);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: Some(&objects),
            ground_response_selectors: [0; 8],
            water_response_selectors: [0; 8],
        };
        let pool = solid_sphere_pool(200);
        let mut parent =
            descriptor_test_particle(METEOR_SCATTER_CLASS, [4.0, 2.0, 5.0], [0; 3], Some(66));
        parent.suppresses_impact_damage = true;
        let mut fx = WorldFx::new();
        let parent_slot = fx.particles.allocate(parent).unwrap();
        let mut handled = None;
        let mut handler_saw_parent_trail_and_visual = false;

        let outcome = fx.update_with_event_handlers(
            ParticleUpdateRequest::terrain(0, 0, context).with_callbacks(ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &[],
                    model_pool: &pool,
                }),
            }),
            |_, impact| panic!("unexpected entity impact: {impact:?}"),
            |live_fx, impact| {
                let impact = expect_static_event(impact);
                handler_saw_parent_trail_and_visual = live_fx.particles.slots[parent_slot]
                    .is_some_and(|particle| particle.source_class == METEOR_SCATTER_CLASS)
                    && live_fx.particles.slots.iter().flatten().any(|particle| {
                        particle.source_class == METEOR_TRAIL_CLASS
                            && particle.suppresses_impact_damage
                    })
                    && live_fx.particles.slots.iter().flatten().any(|particle| {
                        particle.source_class == PRIMARY_ABOVE_WATER_IMPACT_CLASS
                            && particle.suppresses_impact_damage
                    });
                handled = Some(impact);
                ParticleTerrainResponse::Unhandled
            },
        );

        assert!(handler_saw_parent_trail_and_visual);
        let impact = handled.expect("suppressed ballistic F800 boundary");
        assert_eq!(impact.source_particle_class, METEOR_SCATTER_CLASS);
        assert_eq!(impact.damage, None);
        assert_eq!(impact.current, None);
        assert_eq!(outcome.ballistic_static_impacts, [impact]);
        assert!(fx.particles.slots[parent_slot].is_none());
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn ballistic_non_six_response_snaps_bounces_and_emits_unowned_table_burst() {
        let terrain = flat_terrain(0, 0);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [7; 8],
            water_response_selectors: [7; 8],
        };
        let mut fx = full_rate_world_fx();
        fx.particles.test_allocate(descriptor_test_particle(
            METEOR_SCATTER_CLASS,
            [4.75, 1.0, 5.75],
            [-7, -5, 7],
            Some(66),
        ));
        let slot = fx
            .particles
            .slots
            .iter()
            .position(|particle| {
                particle.is_some_and(|particle| particle.source_class == METEOR_SCATTER_CLASS)
            })
            .unwrap();
        let mut parent = fx.particles.slots[slot].unwrap();
        parent.suppresses_impact_damage = true;

        let keep = fx.apply_ballistic_surface_response(
            slot,
            &mut parent,
            7,
            0.0,
            context,
            ParticleBirthContext {
                environment: ParticleEnvironment::Terrain(context),
                retail_tick: 0,
            },
        );

        assert!(keep);
        assert_eq!(parent.position[1], 0.0);
        assert_eq!(
            parent.velocity.map(world_velocity_component_to_raw),
            [-7, 2, 7],
            "E4F0 uses -(Vy / 2) with signed C truncation and leaves X/Z unchanged"
        );
        assert_eq!(parent.owner_id, Some(66));
        assert_eq!(fx.particles.slots[slot], Some(parent));
        assert_eq!(fx.direction_cursor, SURFACE_RESPONSE_BURST_COUNT);
        let children = fx
            .particles
            .slots
            .iter()
            .flatten()
            .filter(|particle| particle.source_class == WATER_DELETE_PARTICLE_CLASS)
            .collect::<Vec<_>>();
        assert_eq!(children.len(), SURFACE_RESPONSE_BURST_COUNT);
        assert!(children.iter().all(|particle| particle.owner_id.is_none()));
        assert!(children
            .iter()
            .all(|particle| particle.suppresses_impact_damage));
        assert_eq!(fx.rng_state, 0);
        assert!(fx.take_positional_sounds().is_empty());
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn ballistic_selector_six_emits_eight_wet_attempts_then_damps_parent() {
        let mut terrain = flat_terrain(-128, 6);
        terrain.header[0] = 0;
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [6; 8],
            water_response_selectors: [6; 8],
        };
        let mut fx = full_rate_world_fx();
        fx.particles.test_allocate(descriptor_test_particle(
            COMMON_EXPLOSION_SCATTER_CLASS,
            [4.0, 0.0, 5.0],
            [-7, -5, 7],
            Some(66),
        ));
        let slot = fx
            .particles
            .slots
            .iter()
            .position(|particle| {
                particle
                    .is_some_and(|particle| particle.source_class == COMMON_EXPLOSION_SCATTER_CLASS)
            })
            .unwrap();
        let mut parent = fx.particles.slots[slot].unwrap();
        parent.suppresses_impact_damage = true;
        let position_before = parent.position;

        let keep = fx.apply_ballistic_surface_response(
            slot,
            &mut parent,
            6,
            99.0,
            context,
            ParticleBirthContext {
                environment: ParticleEnvironment::Terrain(context),
                retail_tick: 0,
            },
        );

        assert!(keep);
        assert_eq!(parent.position, position_before);
        assert_eq!(
            parent.velocity.map(world_velocity_component_to_raw),
            [-1, -2, 1]
        );
        assert_eq!(parent.owner_id, Some(66));
        assert_eq!(fx.particles.slots[slot], Some(parent));
        assert_eq!(fx.direction_cursor, 0);
        let splashes = fx
            .particles
            .slots
            .iter()
            .flatten()
            .filter(|particle| particle.source_class == PLAYER_WATER_DOWNWASH_PARTICLE_CLASS)
            .collect::<Vec<_>>();
        assert_eq!(splashes.len(), BALLISTIC_WATER_RESPONSE_COUNT);
        assert!(splashes.iter().all(|particle| particle.owner_id.is_none()));
        assert!(splashes
            .iter()
            .all(|particle| particle.suppresses_impact_damage));
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn ballistic_water_selector_six_continues_into_same_frame_ground_response() {
        let mut terrain = flat_terrain(0, 6);
        // Put the authored sea plane one raw unit above a flat zero-height
        // floor. Every spray probe remains wet, while the live particle sphere
        // is also inside the endpoint terrain collision band.
        terrain.header[0] = 0x100;
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [7; 8],
            water_response_selectors: [6; 8],
        };
        let mut parent = descriptor_test_particle(
            COMMON_EXPLOSION_SCATTER_CLASS,
            [4.0, 0.0, 5.0],
            [-7, -5, 7],
            Some(66),
        );
        parent.step_start_water_state = 2;
        parent.water_state = 1;
        parent.water_state_initialized = true;

        let mut fx = full_rate_world_fx();
        fx.particles.test_allocate(parent);
        let slot = fx
            .particles
            .slots
            .iter()
            .position(|particle| {
                particle
                    .is_some_and(|particle| particle.source_class == COMMON_EXPLOSION_SCATTER_CLASS)
            })
            .unwrap();
        let mut parent = fx.particles.slots[slot].unwrap();

        let keep = fx.dispatch_ballistic_surface_collision(
            slot,
            &mut parent,
            context,
            ParticleBirthContext {
                environment: ParticleEnvironment::Terrain(context),
                retail_tick: 0,
            },
        );

        assert!(keep);
        assert_eq!(parent.position[1], 0.0);
        assert_eq!(
            parent.velocity.map(world_velocity_component_to_raw),
            [-1, 1, 1],
            "water damping must precede the same-frame ground bounce"
        );
        assert_eq!(fx.particles.slots[slot], Some(parent));
        assert_eq!(
            fx.particles
                .slots
                .iter()
                .flatten()
                .filter(|particle| {
                    particle.source_class == PLAYER_WATER_DOWNWASH_PARTICLE_CLASS
                })
                .count(),
            BALLISTIC_WATER_RESPONSE_COUNT
        );
        assert_eq!(
            fx.particles
                .slots
                .iter()
                .flatten()
                .filter(|particle| particle.source_class == WATER_DELETE_PARTICLE_CLASS)
                .count(),
            SURFACE_RESPONSE_BURST_COUNT
        );
        assert_eq!(fx.direction_cursor, SURFACE_RESPONSE_BURST_COUNT);
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn ballistic_non_six_rejection_still_consumes_all_four_directions() {
        let terrain = flat_terrain(0, 0);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [7; 8],
            water_response_selectors: [7; 8],
        };
        let parent = descriptor_test_particle(METEOR_SCATTER_CLASS, [4.0, 0.0, 5.0], [0; 3], None);
        let mut fx = full_rate_world_fx();
        fx.particles.test_fill_to(MAX_WORLD_PARTICLES, parent);
        let mut live_parent = fx.particles.slots[0].unwrap();

        let keep = fx.apply_ballistic_surface_response(
            0,
            &mut live_parent,
            7,
            0.0,
            context,
            ParticleBirthContext {
                environment: ParticleEnvironment::Terrain(context),
                retail_tick: 0,
            },
        );

        assert!(keep);
        assert_eq!(fx.particle_count(), MAX_WORLD_PARTICLES);
        assert_eq!(fx.direction_cursor, SURFACE_RESPONSE_BURST_COUNT);
        assert!(fx
            .particles
            .slots
            .iter()
            .flatten()
            .all(|particle| particle.source_class == METEOR_SCATTER_CLASS));
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn ballistic_trail_allocation_precedes_surface_response_children() {
        let terrain = flat_terrain(0, 0);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: None,
            ground_response_selectors: [7; 8],
            water_response_selectors: [7; 8],
        };
        let parent =
            descriptor_test_particle(METEOR_SCATTER_CLASS, [4.0, 0.0, 5.0], [0; 3], Some(66));
        let mut fx = full_rate_world_fx();
        fx.particles.test_fill_to(MAX_WORLD_PARTICLES, parent);
        for slot in 1..MAX_WORLD_PARTICLES {
            assert!(fx.particles.free(slot));
        }
        fx.last_retail_tick = Some(0);

        fx.update(ParticleUpdateRequest::terrain(20_000, 1, context));

        assert_eq!(
            fx.particles.slots[1].unwrap().source_class,
            METEOR_TRAIL_CLASS
        );
        for slot in 2..=5 {
            let child = fx.particles.slots[slot].unwrap();
            assert_eq!(child.source_class, WATER_DELETE_PARTICLE_CLASS);
            assert_eq!(child.owner_id, None);
        }
        assert_eq!(
            fx.particles.slots[0].unwrap().source_class,
            METEOR_SCATTER_CLASS
        );
        assert_eq!(fx.direction_cursor, SURFACE_RESPONSE_BURST_COUNT);
        fx.particles.assert_valid_topology();
    }

    #[test]
    fn ballistic_trail_callback_applies_to_common_explosion_class() {
        assert!(uses_ballistic_trail_update_callback(METEOR_SCATTER_CLASS));
        assert!(uses_ballistic_trail_update_callback(
            COMMON_EXPLOSION_SCATTER_CLASS
        ));
        assert!(uses_ballistic_trail_update_callback(58));
        assert!(!uses_ballistic_trail_update_callback(METEOR_TRAIL_CLASS));

        let mut fx = WorldFx::new();
        fx.materialize_particle_burst(single_descriptor_particle_burst(
            [1.0, 2.0, 3.0],
            COMMON_EXPLOSION_SCATTER_CLASS,
            [0; 3],
            Some(66),
        ));
        let initial_velocity_y = fx.test_particles_in_virgin_birth_order()[0].velocity[1];

        fx.update(ParticleUpdateRequest::dry(20_000, 0));

        let particles = fx.test_particles_in_virgin_birth_order();
        let parent = particles
            .iter()
            .find(|particle| particle.source_class == COMMON_EXPLOSION_SCATTER_CLASS)
            .unwrap();
        assert_eq!(parent.owner_id, Some(66));
        assert!(
            (parent.velocity[1]
                - (initial_velocity_y - METEOR_SCATTER_GRAVITY_WORLD_PER_SEC2 * 0.02))
                .abs()
                < 1.0e-6
        );
        assert!(particles
            .iter()
            .any(|particle| particle.source_class == METEOR_TRAIL_CLASS));
    }

    #[test]
    fn ballistic_trail_emits_at_zero_delta_and_selects_class_from_flat_sea_plane() {
        for (parent_y, expected_class) in [
            (1.0 / 256.0, METEOR_TRAIL_CLASS),
            (0.0, BALLISTIC_UNDERWATER_TRAIL_CLASS),
            (-1.0 / 256.0, BALLISTIC_UNDERWATER_TRAIL_CLASS),
        ] {
            let mut fx = WorldFx::new();
            let parent = descriptor_test_particle(
                COMMON_EXPLOSION_SCATTER_CLASS,
                [4.0, parent_y, 5.0],
                [0; 3],
                Some(66),
            );
            let parent_velocity = parent.velocity;
            fx.particles.test_allocate(parent);

            fx.update(ParticleUpdateRequest::flat_water(0, 0, 0.0));

            assert_eq!(fx.particle_count(), 2);
            let particles = fx.test_particles_in_virgin_birth_order();
            let parent = particles
                .iter()
                .find(|particle| particle.source_class == COMMON_EXPLOSION_SCATTER_CLASS)
                .unwrap();
            assert_eq!(parent.position, [4.0, parent_y, 5.0]);
            assert_eq!(parent.velocity, parent_velocity);
            let trail = particles
                .iter()
                .find(|particle| particle.source_class == expected_class)
                .unwrap();
            assert_eq!(trail.position, parent.position);
            assert_eq!(trail.owner_id, None);
            assert_eq!(trail.age_ticks, 0.0);
            assert_eq!(
                trail.velocity.map(world_velocity_component_to_raw),
                [
                    0,
                    i32::from(
                        particle_descriptor(expected_class)
                            .unwrap()
                            .spawn_velocity_y_bias_raw()
                    ),
                    0
                ]
            );
            fx.particles.assert_valid_topology();
        }
    }

    #[test]
    fn timeline_crossing_is_frame_step_independent_and_one_shot() {
        let mut timeline = OneShotTimeline::new();
        assert!(!timeline.crossed_once(31, 0.0, 1.9, 3.0));
        assert!(timeline.crossed_once(31, 1.9, 4.5, 3.0));
        assert!(!timeline.crossed_once(31, 4.5, 4.6, 3.0));
        assert!(!timeline.crossed_once(32, 4.5, 2.0, 3.0));

        assert!(timeline.crossed_once(0, 0.0, 0.02, 0.0));
        assert!(timeline.has_fired(0));
        timeline.reset();
        assert!(!timeline.has_fired(0));
    }

    #[test]
    fn meteor_queue_helper_does_not_duplicate_after_large_frame_step() {
        let mut fx = WorldFx::new();
        let mut timeline = OneShotTimeline::new();
        assert!(fx.queue_meteor_impact_once(
            &mut timeline,
            34,
            1.5,
            4.0,
            2.58,
            [10.0, 20.0, 30.0],
            4,
            Some(34),
        ));
        assert!(!fx.queue_meteor_impact_once(
            &mut timeline,
            34,
            4.0,
            4.1,
            2.58,
            [10.0, 20.0, 30.0],
            4,
            Some(34),
        ));
        assert_eq!(fx.pending_event_count(), 2);
    }
}
