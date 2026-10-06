//! Retail gameplay status-orb presentation recovered from `FUN_004292B0`.
//!
//! This module deliberately owns presentation data and frame policy only. It
//! returns the selected weapon model plus ordered sprite layers in the active
//! level-3 variant's authored composition space. The executable maps their
//! positions and clips to the current viewport, then submits them through the
//! shared model/sprite render paths.
//!
//! Weapon presentation is driven by the selected byte-exact inventory
//! descriptor. The default infinite-ammo weapon uses the two inward-revealed
//! status panels; the recovered Level-1 weapon uses its authored sprite and
//! three decimal ammunition digits.

#[cfg(test)]
mod relayout_tests;
mod time_trophy;
pub use time_trophy::{
    GameplayHudTimeTrophyFrame, GameplayHudTimeTrophyLayout, TIME_TROPHY_HUD_MODEL_ID,
};

use v2k_formats::{
    fixed_math::retail_sine_q15,
    models::{AnimVars, ModelEntry},
};
use v2k_render::WorldSpriteBlend;

use crate::hover::RETAIL_FRAME_DELTA_MAX_US;
use crate::model_tree::model_is_camera_facing_actor;
use crate::player::FUEL_FULL_RAW;
use crate::player_hull::{smooth_visible_hull_health_raw, PLAYER_TYPE_46_HULL_PROFILE};
use crate::resource_cache::ResourceCache;
use crate::system_layout::SystemLayoutSource;
use crate::time_trophy::TimeTrophyRuntime;
use crate::weapon_inventory::{Ammunition, WeaponDescriptor, WeaponInventory};

/// System OVL that owns the persistent gameplay HUD assets and layout points.
pub const GAMEPLAY_HUD_SYSTEM_LEVEL: u32 = 3;
/// Authored framebuffer selected by the level-3 system-overlay variant.
///
/// Retail reloads every active OVL when the Display -> Resolution index
/// changes (`FUN_0044E1F0` -> `FUN_00493A40`).  The first digit of
/// `{variant}X3XX.OVL` therefore selects both the HUD artwork and its screen
/// coordinates; variants 1-3 share the high-resolution pixels but retain
/// resolution-specific anchors.
pub const fn gameplay_hud_virtual_size(system_variant: u32) -> Option<(u32, u32)> {
    match system_variant {
        0 => Some((320, 240)),
        1 => Some((640, 480)),
        2 => Some((800, 600)),
        3 => Some((1024, 768)),
        _ => None,
    }
}
/// Retail type-46 hull maximum used by `FUN_004292B0`.
pub const HULL_FULL_RAW: i32 = PLAYER_TYPE_46_HULL_PROFILE.max_health_raw;
/// Captured default Section-2 item descriptor's Section-8 model id.
pub const DEFAULT_WEAPON_MODEL_ID: usize = 115;
/// Retail display-width branch in `FUN_0042A6F0`.
pub const RETAIL_HIGH_RES_WIDTH_THRESHOLD: u32 = 640;

const DEFAULT_WEAPON_FALLBACK_SPRITE_ID: u16 = 518;
const LEVEL_ONE_WEAPON_FALLBACK_SPRITE_ID: u16 = 519;
const DEFAULT_WEAPON_ANGLE_A: u16 = 0x9000;
const DEFAULT_WEAPON_ANGLE_B: u16 = 0x0c00;
const CARGO_EMPTY_SPRITE_ID: u16 = 533;
const CARGO_MODEL_DEPTH_RAW: i32 = 4000;
const CARGO_SPIN_RATE: u64 = 20_000;
const WEAPON_SPIN_RATE: u64 = 58_000;
const STATUS_REVEAL_STEP_PERCENT: u32 = 16;
const STATUS_REVEAL_FULL_PERCENT: u32 = 100;

/// The exact subset of global Section-3 sprites used by the partial status orb.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum GameplayHudSprite {
    FinalFrame = 499,
    OuterFrame = 500,
    InnerFrame = 501,
    Fuel = 502,
    Hull = 503,
    StatusLeft = 504,
    StatusRight = 505,
    CenterShade = 506,
    Digit0 = 508,
    Digit1 = 509,
    Digit2 = 510,
    Digit3 = 511,
    Digit4 = 512,
    Digit5 = 513,
    Digit6 = 514,
    Digit7 = 515,
    Digit8 = 516,
    Digit9 = 517,
    WeaponFallback = DEFAULT_WEAPON_FALLBACK_SPRITE_ID,
    LevelOneWeaponFallback = LEVEL_ONE_WEAPON_FALLBACK_SPRITE_ID,
    CargoEmpty = CARGO_EMPTY_SPRITE_ID,
}

impl GameplayHudSprite {
    pub const fn global_id(self) -> u16 {
        self as u16
    }
}

/// One decoded sprite, retained at its authored size and with its Section-3
/// framebuffer operation intact.
#[derive(Debug, Clone)]
pub struct DecodedGameplayHudSprite {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub blend: WorldSpriteBlend,
}

/// Pre-decoded status-orb sprites from the level-3 common OVL.
#[derive(Debug, Clone)]
pub struct GameplayHudResources {
    final_frame: DecodedGameplayHudSprite,
    outer_frame: DecodedGameplayHudSprite,
    inner_frame: DecodedGameplayHudSprite,
    fuel: DecodedGameplayHudSprite,
    hull: DecodedGameplayHudSprite,
    status_left: DecodedGameplayHudSprite,
    status_right: DecodedGameplayHudSprite,
    center_shade: DecodedGameplayHudSprite,
    digits: [DecodedGameplayHudSprite; 10],
    weapon_fallback: DecodedGameplayHudSprite,
    level_one_weapon_fallback: DecodedGameplayHudSprite,
    cargo_empty: DecodedGameplayHudSprite,
}

impl GameplayHudResources {
    /// Decode every required sprite. Missing or malformed data disables the
    /// compositor as a unit instead of presenting a mixture of fallback art.
    pub fn from_cache(cache: &ResourceCache) -> Option<Self> {
        let digits = (GameplayHudSprite::Digit0.global_id()
            ..=GameplayHudSprite::Digit9.global_id())
            .map(|global_id| decode_sprite_id(cache, global_id))
            .collect::<Option<Vec<_>>>()?
            .try_into()
            .ok()?;
        Some(Self {
            final_frame: decode_sprite(cache, GameplayHudSprite::FinalFrame)?,
            outer_frame: decode_sprite(cache, GameplayHudSprite::OuterFrame)?,
            inner_frame: decode_sprite(cache, GameplayHudSprite::InnerFrame)?,
            fuel: decode_sprite(cache, GameplayHudSprite::Fuel)?,
            hull: decode_sprite(cache, GameplayHudSprite::Hull)?,
            status_left: decode_sprite(cache, GameplayHudSprite::StatusLeft)?,
            status_right: decode_sprite(cache, GameplayHudSprite::StatusRight)?,
            center_shade: decode_sprite(cache, GameplayHudSprite::CenterShade)?,
            digits,
            weapon_fallback: decode_sprite(cache, GameplayHudSprite::WeaponFallback)?,
            level_one_weapon_fallback: decode_sprite(
                cache,
                GameplayHudSprite::LevelOneWeaponFallback,
            )?,
            cargo_empty: decode_sprite(cache, GameplayHudSprite::CargoEmpty)?,
        })
    }

    pub fn sprite(&self, sprite: GameplayHudSprite) -> &DecodedGameplayHudSprite {
        match sprite {
            GameplayHudSprite::FinalFrame => &self.final_frame,
            GameplayHudSprite::OuterFrame => &self.outer_frame,
            GameplayHudSprite::InnerFrame => &self.inner_frame,
            GameplayHudSprite::Fuel => &self.fuel,
            GameplayHudSprite::Hull => &self.hull,
            GameplayHudSprite::StatusLeft => &self.status_left,
            GameplayHudSprite::StatusRight => &self.status_right,
            GameplayHudSprite::CenterShade => &self.center_shade,
            GameplayHudSprite::Digit0
            | GameplayHudSprite::Digit1
            | GameplayHudSprite::Digit2
            | GameplayHudSprite::Digit3
            | GameplayHudSprite::Digit4
            | GameplayHudSprite::Digit5
            | GameplayHudSprite::Digit6
            | GameplayHudSprite::Digit7
            | GameplayHudSprite::Digit8
            | GameplayHudSprite::Digit9 => {
                &self.digits
                    [usize::from(sprite.global_id() - GameplayHudSprite::Digit0.global_id())]
            }
            GameplayHudSprite::WeaponFallback => &self.weapon_fallback,
            GameplayHudSprite::LevelOneWeaponFallback => &self.level_one_weapon_fallback,
            GameplayHudSprite::CargoEmpty => &self.cargo_empty,
        }
    }
}

fn decode_sprite(
    cache: &ResourceCache,
    sprite: GameplayHudSprite,
) -> Option<DecodedGameplayHudSprite> {
    decode_sprite_id(cache, sprite.global_id())
}

pub fn decode_gameplay_hud_sprite(
    cache: &ResourceCache,
    global_id: u16,
) -> Option<DecodedGameplayHudSprite> {
    decode_sprite_id(cache, global_id)
}

fn decode_sprite_id(cache: &ResourceCache, global_id: u16) -> Option<DecodedGameplayHudSprite> {
    let (atlas, entry) = cache.global_sprite(global_id)?;
    let flags = entry.pal_size as u8;
    let decoded = atlas
        .decode_sprite(
            entry,
            usize::from(crate::model_color::sprite_flat_shade_row(flags)),
        )
        .ok()?;
    Some(DecodedGameplayHudSprite {
        rgba: decoded.rgba,
        width: decoded.width as u32,
        height: decoded.height as u32,
        blend: crate::model_color::sprite_blend(flags),
    })
}

/// One exact signed point in the authored composition space.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct GameplayHudPoint {
    pub x: i32,
    pub y: i32,
}

impl GameplayHudPoint {
    const fn offset(self, relative: Self) -> Self {
        Self {
            x: self.x + relative.x,
            y: self.y + relative.y,
        }
    }
}

/// Level-3 Section-1 points consumed by `FUN_004292B0`.
///
/// Point 6 belongs to the deferred finite-ammo digits. Point 7 owns the
/// complementary panels used by the captured infinite-ammo weapon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameplayHudLayout {
    pub virtual_width: u32,
    pub virtual_height: u32,
    pub base: GameplayHudPoint,
    pub outer_frame: GameplayHudPoint,
    pub final_frame: GameplayHudPoint,
    pub inner_frame: GameplayHudPoint,
    pub fuel: GameplayHudPoint,
    pub hull: GameplayHudPoint,
    pub digits: GameplayHudPoint,
    pub status_halves: GameplayHudPoint,
    pub center_shade: GameplayHudPoint,
    /// Local point 11 (`DAT_004FE624 + 0x60`), used by the 2-D fallback.
    pub weapon_fallback: GameplayHudPoint,
    /// Local point 12 (`DAT_004FE624 + 0x64`), used as the model projection centre.
    pub weapon_model: GameplayHudPoint,
    /// Local point 13 (`DAT_004FE624 + 0x68`), selected-item carousel spacing.
    pub weapon_spacing: GameplayHudPoint,
    /// Local point 14 (`DAT_004FE624 + 0x6C`), cargo-slot row origin.
    pub cargo_origin: GameplayHudPoint,
    /// Local point 15 (`DAT_004FE624 + 0x70`), cargo-slot horizontal spacing.
    pub cargo_spacing: GameplayHudPoint,
    pub time_trophy: GameplayHudTimeTrophyLayout,
}

impl GameplayHudLayout {
    /// Load the status-orb points 0..=8, selected-weapon points 11..=13, and
    /// cargo points 14..=15
    /// from level 3's Section-1 table. No coordinate fallback is supplied:
    /// the HUD must follow the selected asset variant.
    pub fn from_cache(
        cache: &(impl SystemLayoutSource + ?Sized),
        system_variant: u32,
    ) -> Option<Self> {
        let (virtual_width, virtual_height) = gameplay_hud_virtual_size(system_variant)?;
        Some(Self {
            virtual_width,
            virtual_height,
            base: layout_point(cache, 0)?,
            outer_frame: layout_point(cache, 1)?,
            final_frame: layout_point(cache, 2)?,
            inner_frame: layout_point(cache, 3)?,
            fuel: layout_point(cache, 4)?,
            hull: layout_point(cache, 5)?,
            digits: layout_point(cache, 6)?,
            status_halves: layout_point(cache, 7)?,
            center_shade: layout_point(cache, 8)?,
            weapon_fallback: layout_point(cache, 11)?,
            weapon_model: layout_point(cache, 12)?,
            weapon_spacing: layout_point(cache, 13)?,
            cargo_origin: layout_point(cache, 14)?,
            cargo_spacing: layout_point(cache, 15)?,
            time_trophy: GameplayHudTimeTrophyLayout {
                model: layout_point(cache, 27)?,
                text: layout_point(cache, 28)?,
            },
        })
    }

    pub const fn absolute(self, relative: GameplayHudPoint) -> GameplayHudPoint {
        self.base.offset(relative)
    }
}

fn layout_point(
    cache: &(impl SystemLayoutSource + ?Sized),
    local_index: usize,
) -> Option<GameplayHudPoint> {
    let (x, y) = cache.system_layout_point(GAMEPLAY_HUD_SYSTEM_LEVEL, local_index)?;
    Some(GameplayHudPoint {
        x: i32::from(x),
        y: i32::from(y),
    })
}

/// Virtual-space clip rectangle. Main maps this through the same uniform 4:3
/// transform as the layer position before calling `Renderer::set_sprite_clip`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameplayHudClip {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Semantic role of a draw layer. Flash/trail roles intentionally remain
/// distinct even when they reuse the same fuel or hull sprite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameplayHudLayerRole {
    OuterFrame,
    FuelFlash,
    FuelFill,
    HullFlash,
    HullTrail,
    HullFill,
    AmmoHundreds,
    AmmoTens,
    AmmoOnes,
    StatusLeft,
    StatusRight,
    InnerFrame,
    CenterShade,
    FinalFrame,
}

/// One ordered sprite submission in the authored composition space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameplayHudLayer {
    pub role: GameplayHudLayerRole,
    pub sprite: GameplayHudSprite,
    pub position: GameplayHudPoint,
    /// `None` means the full virtual viewport; a fill layer clips everything
    /// above the calculated vertical bar boundary.
    pub clip: Option<GameplayHudClip>,
}

/// Descriptor-derived selected-weapon state consumed by the HUD compositor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameplayHudWeaponState {
    pub selector: u32,
    pub global_model_id: Option<usize>,
    /// Descriptor `+0x10` high word. `FUN_004292B0` uses this global sprite
    /// when the 3-D HUD model is absent or fails to resolve.
    pub fallback_sprite: Option<u16>,
    pub ammunition: Ammunition,
}

impl GameplayHudWeaponState {
    /// Preserve the descriptor's independent 3-D and 2-D resources. Retail
    /// prefers the model when present and keeps the sprite as its fallback.
    pub fn from_descriptor(descriptor: &WeaponDescriptor) -> Self {
        Self {
            selector: descriptor.selector(),
            global_model_id: descriptor.hud_model_id().map(usize::from),
            fallback_sprite: descriptor.hud_sprite_id(),
            ammunition: descriptor.ammunition(),
        }
    }
}

/// Compact occupied-weapon list consumed by retail's HUD carousel.
///
/// `FUN_00443260` copies the contiguous occupied controller prefix into
/// selector/count pairs, retaining finite zero counts. `FUN_004292B0` consumes
/// that snapshot and draws the selected weapon plus a temporary carousel
/// neighbor. Selection owns usable-ammunition filtering; the HUD preserves
/// inventory ownership and its selected byte's compact-list index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayHudWeaponRoster {
    weapons: Vec<GameplayHudWeaponState>,
    selected_index: usize,
}

impl GameplayHudWeaponRoster {
    pub fn from_inventory(inventory: &WeaponInventory) -> Self {
        let mut weapons = Vec::with_capacity(inventory.occupied_slot_count());
        let mut selected_index = 0;
        for (slot_index, descriptor) in inventory.slots().iter().enumerate() {
            if descriptor.is_empty() {
                continue;
            }
            if slot_index == inventory.selected_slot() {
                selected_index = weapons.len();
            }
            weapons.push(GameplayHudWeaponState::from_descriptor(descriptor));
        }
        Self {
            weapons,
            selected_index,
        }
    }

    pub fn selected(&self) -> Option<GameplayHudWeaponState> {
        self.weapons.get(self.selected_index).copied()
    }

    pub fn len(&self) -> usize {
        self.weapons.len()
    }

    pub fn is_empty(&self) -> bool {
        self.weapons.is_empty()
    }
}

impl Default for GameplayHudWeaponRoster {
    fn default() -> Self {
        Self::from_inventory(&WeaponInventory::new())
    }
}

/// Evidence-complete presentation of one selected weapon.
///
/// The July 14 fire capture records item descriptor model 115 / fallback 518.
/// `FUN_0042A6F0` renders it at the authored projection point before the orb
/// sprites, whose later submissions mask the model into the aperture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameplayHudWeaponFrame {
    pub selector: u32,
    pub global_model_id: Option<usize>,
    pub fallback_sprite: Option<u16>,
    pub model_position: GameplayHudPoint,
    pub fallback_position: GameplayHudPoint,
    pub carousel_offset_y: i32,
    /// Raw fourth argument to `FUN_0042A570`. Zero is retail's special
    /// full-size path; nonzero values scale the fallback sprite around its
    /// centre by this percentage. Section-8 models ignore this argument.
    pub sprite_size_percent_raw: u32,
    pub spin_raw: u16,
}

impl GameplayHudWeaponFrame {
    /// Retail Section-8 view depth in the menu model transform's raw units.
    pub fn depth_raw(self, display_width: u32) -> i32 {
        retail_weapon_depth_raw(self.carousel_offset_y, display_width)
    }

    /// Convert `FUN_0042A2E0`'s lateral/up/forward Q31 vectors to the shared
    /// renderer's row-major `world = M * local` convention.
    pub fn orientation(self) -> [[f32; 3]; 3] {
        retail_weapon_orientation(self.spin_raw)
    }
}

/// One currently unlocked cargo slot below the left status orb.
///
/// Retail always draws sprite 533 at `marker_position`. An attached entity
/// additionally replaces the empty appearance with its entity type's primary
/// Section-8 model at `model_position`; the marker submission still precedes
/// it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameplayHudCargoFrame {
    pub global_model_id: Option<usize>,
    pub marker: GameplayHudSprite,
    pub marker_position: GameplayHudPoint,
    pub model_position: GameplayHudPoint,
    pub spin_raw: u16,
    pub animation: GameplayHudCargoAnimation,
}

impl GameplayHudCargoFrame {
    /// `FUN_004292B0` uses a fixed raw depth of 4000 for cargo models.
    pub const fn depth_raw(self) -> i32 {
        CARGO_MODEL_DEPTH_RAW
    }

    /// Exact `FUN_0042A2E0(cargo_spin, 0, 0)` orientation.
    pub fn orientation(self) -> [[f32; 3]; 3] {
        retail_orientation(self.spin_raw, 0, 0)
    }

    /// Cargo orientation after applying the model's presentation policy.
    ///
    /// Rigid 3D cargo uses the recovered shared angle. Flat actors consume the
    /// same linked walk callback but remain face-on in the HUD, matching their
    /// camera-facing render semantics rather than visibly yawing edge-on.
    pub fn orientation_for_model(self, model: &ModelEntry) -> [[f32; 3]; 3] {
        if model_is_camera_facing_actor(model) {
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
        } else {
            self.orientation()
        }
    }

    /// Materialize this slot through the retail cargo callback installed at
    /// `0x42977D`. The same callback is used for every attached entity; flat
    /// people select their authored walk frames while ordinary 3D cargo simply
    /// ignores these dynamic inputs. Flat-actor facing is an independent
    /// presentation policy applied by [`Self::orientation_for_model`].
    pub fn animation_vars(self) -> AnimVars {
        self.animation.anim_vars()
    }
}

/// Values returned by retail's cargo-model callback at `0x42A520`.
///
/// Keeping these values on the presentation request makes the distinction
/// between live cargo materialization and the separately recovered static
/// weapon path explicit. It also avoids inferring animation policy from a
/// model name or from planar geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameplayHudCargoAnimation {
    /// Callback index 0: low 16 bits of the 50 Hz retail clock.
    pub dynamic_0: i32,
    /// Callback index 1: four-state walk/mirror phase, changing every 6 ticks.
    pub dynamic_1: i32,
}

impl GameplayHudCargoAnimation {
    pub const fn from_retail_tick(retail_tick: u32) -> Self {
        Self {
            dynamic_0: (retail_tick & 0xffff) as i32,
            dynamic_1: ((retail_tick / 6) & 3) as i32,
        }
    }

    pub fn anim_vars(self) -> AnimVars {
        let mut vars = AnimVars::default();
        vars.dynamic[0] = self.dynamic_0;
        vars.dynamic[1] = self.dynamic_1;
        vars
    }
}

/// Complete partial-orb presentation for one frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayHudFrame {
    pub virtual_width: u32,
    pub virtual_height: u32,
    /// Active challenge, submitted before cargo and status-orb composition.
    pub time_trophy: Option<GameplayHudTimeTrophyFrame>,
    pub cargo: Vec<GameplayHudCargoFrame>,
    /// Neighbor first, selected weapon second during a transition; otherwise
    /// this contains only the selected weapon.
    pub weapons: Vec<GameplayHudWeaponFrame>,
    pub layers: Vec<GameplayHudLayer>,
}

/// Relocate already-evaluated commands between authored composition spaces.
/// High tiers retain the same sprites and local geometry; only anchors and
/// viewport bounds change. This never calls the stateful HUD compositor.
#[derive(Debug, Clone, Copy)]
pub struct GameplayHudRelayout {
    pub previous: GameplayHudLayout,
    pub next: GameplayHudLayout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameplayHudRelayoutError {
    FrameLayoutMismatch,
    IntrinsicGeometryChanged,
    ClipShape(GameplayHudLayerRole),
}

impl GameplayHudFrame {
    /// Preserve order, sprites, reveal/trail decisions, carousel displacement,
    /// spin, animation and formatted clock text. Validate every clip before
    /// publishing any mutation so a rejected relayout leaves the frame intact.
    pub fn relayout(
        &mut self,
        request: GameplayHudRelayout,
    ) -> Result<(), GameplayHudRelayoutError> {
        let GameplayHudRelayout { previous, next } = request;
        if (self.virtual_width, self.virtual_height)
            != (previous.virtual_width, previous.virtual_height)
        {
            return Err(GameplayHudRelayoutError::FrameLayoutMismatch);
        }
        // Removing independently authored anchors exposes intrinsic geometry.
        let mut previous_geometry = previous;
        previous_geometry.virtual_width = next.virtual_width;
        previous_geometry.virtual_height = next.virtual_height;
        previous_geometry.base = next.base;
        previous_geometry.time_trophy = next.time_trophy;
        if previous_geometry != next || next.virtual_width == 0 || next.virtual_height == 0 {
            return Err(GameplayHudRelayoutError::IntrinsicGeometryChanged);
        }
        let delta = GameplayHudPoint {
            x: next.base.x.wrapping_sub(previous.base.x),
            y: next.base.y.wrapping_sub(previous.base.y),
        };
        let clips = self
            .layers
            .iter()
            .map(|layer| relayout_clip(layer.role, layer.clip, previous, next, delta))
            .collect::<Result<Vec<_>, _>>()?;
        for (layer, clip) in self.layers.iter_mut().zip(clips) {
            layer.position = translate_point(layer.position, delta);
            layer.clip = clip;
        }
        for weapon in &mut self.weapons {
            weapon.model_position = translate_point(weapon.model_position, delta);
            weapon.fallback_position = translate_point(weapon.fallback_position, delta);
        }
        for cargo in &mut self.cargo {
            cargo.marker_position = translate_point(cargo.marker_position, delta);
            cargo.model_position = translate_point(cargo.model_position, delta);
        }
        if let Some(trophy) = &mut self.time_trophy {
            trophy.model_position = translate_point(
                trophy.model_position,
                GameplayHudPoint {
                    x: next
                        .time_trophy
                        .model
                        .x
                        .wrapping_sub(previous.time_trophy.model.x),
                    y: next
                        .time_trophy
                        .model
                        .y
                        .wrapping_sub(previous.time_trophy.model.y),
                },
            );
            trophy.text_position = translate_point(
                trophy.text_position,
                GameplayHudPoint {
                    x: next
                        .time_trophy
                        .text
                        .x
                        .wrapping_sub(previous.time_trophy.text.x),
                    y: next
                        .time_trophy
                        .text
                        .y
                        .wrapping_sub(previous.time_trophy.text.y),
                },
            );
        }
        self.virtual_width = next.virtual_width;
        self.virtual_height = next.virtual_height;
        Ok(())
    }
}

fn translate_point(point: GameplayHudPoint, delta: GameplayHudPoint) -> GameplayHudPoint {
    GameplayHudPoint {
        x: point.x.wrapping_add(delta.x),
        y: point.y.wrapping_add(delta.y),
    }
}

fn relayout_clip(
    role: GameplayHudLayerRole,
    clip: Option<GameplayHudClip>,
    previous: GameplayHudLayout,
    next: GameplayHudLayout,
    delta: GameplayHudPoint,
) -> Result<Option<GameplayHudClip>, GameplayHudRelayoutError> {
    let Some(clip) = clip else { return Ok(None) };
    let bad = || GameplayHudRelayoutError::ClipShape(role);
    let result = match role {
        GameplayHudLayerRole::FuelFill
        | GameplayHudLayerRole::HullTrail
        | GameplayHudLayerRole::HullFill => {
            if clip.x != 0
                || clip.width != previous.virtual_width
                || clip.y < 0
                || clip.y as u32 > previous.virtual_height
                || clip.height != previous.virtual_height - clip.y as u32
            {
                return Err(bad());
            }
            vertical_clip(next, clip.y.wrapping_add(delta.y))
        }
        GameplayHudLayerRole::StatusLeft => {
            if clip.x != 0
                || clip.y != 0
                || clip.height != previous.virtual_height
                || clip.width > previous.virtual_width
            {
                return Err(bad());
            }
            GameplayHudClip {
                x: 0,
                y: 0,
                width: (i64::from(clip.width) + i64::from(delta.x))
                    .clamp(0, i64::from(next.virtual_width)) as u32,
                height: next.virtual_height,
            }
        }
        GameplayHudLayerRole::StatusRight => {
            if clip.y != 0
                || clip.height != previous.virtual_height
                || clip.x < 0
                || clip.x as u32 > previous.virtual_width
                || clip.width != previous.virtual_width - clip.x as u32
            {
                return Err(bad());
            }
            let x = clip
                .x
                .wrapping_add(delta.x)
                .clamp(0, next.virtual_width as i32);
            GameplayHudClip {
                x,
                y: 0,
                width: next.virtual_width - x as u32,
                height: next.virtual_height,
            }
        }
        _ => return Err(bad()),
    };
    Ok(Some(result))
}

/// Persistent retail HUD state. The visible hull value deliberately can
/// undershoot the real health for one frame; `smooth_visible_hull_health_raw`
/// reproduces that original damage-lag behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GameplayHud {
    visible_hull_raw: i32,
    cargo_spin_raw: u16,
    weapon_spin_raw: u16,
    status_reveal_percent: u32,
    weapon_selected_index: usize,
    weapon_selected_selector: u32,
    weapon_carousel_offset_y: i32,
}

impl GameplayHud {
    pub const fn visible_hull_raw(&self) -> i32 {
        self.visible_hull_raw
    }

    pub const fn weapon_spin_raw(&self) -> u16 {
        self.weapon_spin_raw
    }

    pub const fn cargo_spin_raw(&self) -> u16 {
        self.cargo_spin_raw
    }

    pub const fn status_reveal_percent(&self) -> u32 {
        self.status_reveal_percent
    }

    pub const fn weapon_carousel_offset_y(&self) -> i32 {
        self.weapon_carousel_offset_y
    }

    /// Reset process-local presentation state. Zero matches the retail BSS
    /// initial value; the first live-health frame snaps it upward.
    pub fn reset(&mut self) {
        self.visible_hull_raw = 0;
    }

    /// Advance `DAT_004DB1B4` for one accepted gameplay update. This remains
    /// independent of HUD visibility, exactly like the retail global.
    pub fn advance_spins(&mut self, frame_delta_micros: u32) {
        self.cargo_spin_raw = advance_retail_cargo_spin(self.cargo_spin_raw, frame_delta_micros);
        self.weapon_spin_raw = advance_retail_weapon_spin(self.weapon_spin_raw, frame_delta_micros);
    }

    /// Build the exact ordered partial-orb layers for one render pass.
    pub fn frame(
        &mut self,
        resources: &GameplayHudResources,
        layout: GameplayHudLayout,
        fuel_raw: i32,
        hull_health_raw: i32,
        retail_tick: u32,
        elapsed_micros: u32,
        weapon_roster: &GameplayHudWeaponRoster,
        cargo_models: &[Option<usize>],
        time_trophy: TimeTrophyRuntime,
    ) -> GameplayHudFrame {
        let fuel_raw = fuel_raw.clamp(0, FUEL_FULL_RAW);
        let hull_health_raw = hull_health_raw.clamp(0, HULL_FULL_RAW);
        let flash_factor = retail_bar_flash_factor(elapsed_micros);
        let mut layers = Vec::with_capacity(11);

        layers.push(layer(
            GameplayHudLayerRole::OuterFrame,
            GameplayHudSprite::OuterFrame,
            layout.absolute(layout.outer_frame),
            None,
        ));

        let fuel_position = layout.absolute(layout.fuel);
        if should_flash_fuel(fuel_raw, flash_factor) {
            layers.push(layer(
                GameplayHudLayerRole::FuelFlash,
                GameplayHudSprite::Fuel,
                fuel_position,
                None,
            ));
        }
        layers.push(layer(
            GameplayHudLayerRole::FuelFill,
            GameplayHudSprite::Fuel,
            fuel_position,
            Some(vertical_clip(
                layout,
                fuel_fill_clip_top(layout, resources.fuel.height, fuel_raw),
            )),
        ));

        let hull_position = layout.absolute(layout.hull);
        let visible_before = self.visible_hull_raw;
        if visible_before == hull_health_raw {
            if should_flash_hull(hull_health_raw, flash_factor) {
                layers.push(layer(
                    GameplayHudLayerRole::HullFlash,
                    GameplayHudSprite::Hull,
                    hull_position,
                    None,
                ));
            }
        } else if hull_health_raw < visible_before {
            layers.push(layer(
                GameplayHudLayerRole::HullTrail,
                GameplayHudSprite::Hull,
                hull_position,
                Some(vertical_clip(
                    layout,
                    hull_fill_clip_top(layout, resources.hull.height, visible_before),
                )),
            ));
            self.visible_hull_raw = smooth_visible_hull_health_raw(visible_before, hull_health_raw);
        } else {
            self.visible_hull_raw = hull_health_raw;
        }
        layers.push(layer(
            GameplayHudLayerRole::HullFill,
            GameplayHudSprite::Hull,
            hull_position,
            Some(vertical_clip(
                layout,
                hull_fill_clip_top(layout, resources.hull.height, hull_health_raw),
            )),
        ));

        match weapon_roster.selected().map(|weapon| weapon.ammunition) {
            Some(Ammunition::Infinite) => {
                // Descriptor byte +0x0C bit zero selects FUN_004292B0's
                // infinite-ammo branch. DAT_004CA6B0 rises by 16 per accepted
                // draw and sprites 504/505 reveal from opposite edges.
                self.status_reveal_percent = self
                    .status_reveal_percent
                    .saturating_add(STATUS_REVEAL_STEP_PERCENT)
                    .min(STATUS_REVEAL_FULL_PERCENT);
            }
            Some(Ammunition::Finite(rounds)) => {
                layers.extend(ammunition_digit_layers(resources, layout, rounds));
                // Finite ammunition retracts the complementary panels by the
                // same 16-point step. They remain visible during the handoff.
                self.status_reveal_percent = self
                    .status_reveal_percent
                    .saturating_sub(STATUS_REVEAL_STEP_PERCENT);
            }
            None => {}
        }
        if self.status_reveal_percent != 0 {
            layers.extend(status_panel_layers(
                resources,
                layout,
                self.status_reveal_percent,
            ));
        }

        layers.push(layer(
            GameplayHudLayerRole::InnerFrame,
            GameplayHudSprite::InnerFrame,
            layout.absolute(layout.inner_frame),
            None,
        ));
        layers.push(layer(
            GameplayHudLayerRole::CenterShade,
            GameplayHudSprite::CenterShade,
            layout.absolute(layout.center_shade),
            None,
        ));
        layers.push(layer(
            GameplayHudLayerRole::FinalFrame,
            GameplayHudSprite::FinalFrame,
            layout.absolute(layout.final_frame),
            None,
        ));

        self.update_weapon_carousel(weapon_roster, layout.weapon_spacing.y);
        let weapons = weapon_carousel_frames(
            layout,
            weapon_roster,
            self.weapon_carousel_offset_y,
            self.weapon_spin_raw,
        );

        GameplayHudFrame {
            virtual_width: layout.virtual_width,
            virtual_height: layout.virtual_height,
            time_trophy: GameplayHudTimeTrophyFrame::from_runtime(
                time_trophy,
                layout.time_trophy,
                self.cargo_spin_raw,
                retail_tick,
            ),
            cargo: cargo_frames(
                resources,
                layout,
                cargo_models,
                self.cargo_spin_raw,
                retail_tick,
            ),
            weapons,
            layers,
        }
    }

    fn update_weapon_carousel(&mut self, roster: &GameplayHudWeaponRoster, spacing_y: i32) {
        let Some(selected) = roster.selected() else {
            self.weapon_carousel_offset_y = 0;
            return;
        };

        if self.weapon_selected_index != roster.selected_index {
            if self.weapon_selected_selector != selected.selector {
                self.weapon_carousel_offset_y = retail_weapon_selection_offset(
                    self.weapon_selected_index,
                    roster.selected_index,
                    spacing_y,
                );
                self.weapon_selected_selector = selected.selector;
            }
            self.weapon_selected_index = roster.selected_index;
        }
        self.weapon_carousel_offset_y =
            advance_retail_weapon_carousel_offset(self.weapon_carousel_offset_y);
    }
}

/// Initial `DAT_004DB1BC` displacement when the compact selected index changes.
///
/// Retail treats wraparound as one neighboring step instead of traversing the
/// whole list: last-to-zero starts at `+spacing`, while zero-to-last starts at
/// `-spacing`. Ordinary adjacent movement retains its signed index delta.
pub fn retail_weapon_selection_offset(
    previous_index: usize,
    selected_index: usize,
    spacing_y: i32,
) -> i32 {
    if selected_index == 0 && previous_index > 1 {
        spacing_y
    } else if previous_index == 0 && selected_index > 1 {
        spacing_y.wrapping_neg()
    } else {
        let selected = i32::try_from(selected_index).unwrap_or(i32::MAX);
        let previous = i32::try_from(previous_index).unwrap_or(i32::MAX);
        selected.wrapping_sub(previous).wrapping_mul(spacing_y)
    }
}

/// One `FUN_004292B0` easing step for `DAT_004DB1BC`.
///
/// Positive and negative values both approach zero by one plus one quarter of
/// their magnitude. This is evaluated immediately after a selection change,
/// before either carousel entry is submitted.
pub fn advance_retail_weapon_carousel_offset(offset_y: i32) -> i32 {
    if offset_y < 0 {
        offset_y.wrapping_add(1_i32.wrapping_sub(offset_y.wrapping_add(3).wrapping_shr(2)))
    } else if offset_y > 0 {
        offset_y.wrapping_sub(1_i32.wrapping_add(offset_y.wrapping_shr(2)))
    } else {
        0
    }
}

fn weapon_carousel_frames(
    layout: GameplayHudLayout,
    roster: &GameplayHudWeaponRoster,
    selected_offset_y: i32,
    spin_raw: u16,
) -> Vec<GameplayHudWeaponFrame> {
    let Some(selected) = roster.selected() else {
        return Vec::new();
    };

    let transition_percent = selected_offset_y.unsigned_abs().saturating_mul(3).min(100);
    let mut frames = Vec::with_capacity(if selected_offset_y == 0 { 1 } else { 2 });
    if roster.weapons.len() > 1 {
        let neighbor = if selected_offset_y < 0 {
            Some((
                (roster.selected_index + 1) % roster.weapons.len(),
                selected_offset_y.wrapping_add(layout.weapon_spacing.y),
            ))
        } else if selected_offset_y > 0 {
            Some((
                if roster.selected_index == 0 {
                    roster.weapons.len() - 1
                } else {
                    roster.selected_index - 1
                },
                selected_offset_y.wrapping_sub(layout.weapon_spacing.y),
            ))
        } else {
            None
        };
        if let Some((index, offset_y)) = neighbor {
            frames.push(weapon_frame(
                layout,
                roster.weapons[index],
                offset_y,
                transition_percent,
                spin_raw,
            ));
        }
    }
    frames.push(weapon_frame(
        layout,
        selected,
        selected_offset_y,
        if selected_offset_y == 0 {
            0
        } else {
            100 - transition_percent
        },
        spin_raw,
    ));
    frames
}

fn weapon_frame(
    layout: GameplayHudLayout,
    weapon: GameplayHudWeaponState,
    offset_y: i32,
    sprite_size_percent_raw: u32,
    spin_raw: u16,
) -> GameplayHudWeaponFrame {
    let offset = GameplayHudPoint { x: 0, y: offset_y };
    GameplayHudWeaponFrame {
        selector: weapon.selector,
        global_model_id: weapon.global_model_id,
        fallback_sprite: weapon.fallback_sprite,
        model_position: layout.absolute(layout.weapon_model).offset(offset),
        fallback_position: layout.absolute(layout.weapon_fallback).offset(offset),
        carousel_offset_y: offset_y,
        sprite_size_percent_raw,
        spin_raw,
    }
}

/// Rectangle produced by `FUN_0042A570` for a carousel fallback sprite.
///
/// The raw zero argument bypasses scaling and retains the complete sprite.
/// Every nonzero percentage removes half the unused width/height from each
/// side using retail's truncating integer arithmetic.
pub fn retail_weapon_sprite_rect(
    position: GameplayHudPoint,
    width: u32,
    height: u32,
    size_percent_raw: u32,
) -> (GameplayHudPoint, u32, u32) {
    if size_percent_raw == 0 {
        return (position, width, height);
    }
    let scaled_width = width.saturating_mul(size_percent_raw) / 100;
    let scaled_height = height.saturating_mul(size_percent_raw) / 100;
    let margin_x = width.saturating_sub(scaled_width) / 2;
    let margin_y = height.saturating_sub(scaled_height) / 2;
    (
        GameplayHudPoint {
            x: position
                .x
                .saturating_add(i32::try_from(margin_x).unwrap_or(i32::MAX)),
            y: position
                .y
                .saturating_add(i32::try_from(margin_y).unwrap_or(i32::MAX)),
        },
        width.saturating_sub(margin_x.saturating_mul(2)),
        height.saturating_sub(margin_y.saturating_mul(2)),
    )
}

fn ammunition_digit_layers(
    resources: &GameplayHudResources,
    layout: GameplayHudLayout,
    rounds: u32,
) -> [GameplayHudLayer; 3] {
    let digits = [
        ((rounds / 100) % 10) as u8,
        ((rounds / 10) % 10) as u8,
        (rounds % 10) as u8,
    ];
    let roles = [
        GameplayHudLayerRole::AmmoHundreds,
        GameplayHudLayerRole::AmmoTens,
        GameplayHudLayerRole::AmmoOnes,
    ];
    let mut position = layout.absolute(layout.digits);
    std::array::from_fn(|index| {
        let sprite = gameplay_hud_digit_sprite(digits[index]);
        let layer = layer(roles[index], sprite, position, None);
        position.x = position.x.saturating_add(
            i32::try_from(resources.sprite(sprite).width)
                .unwrap_or(i32::MAX)
                .saturating_add(1),
        );
        layer
    })
}

fn gameplay_hud_digit_sprite(digit: u8) -> GameplayHudSprite {
    match digit {
        0 => GameplayHudSprite::Digit0,
        1 => GameplayHudSprite::Digit1,
        2 => GameplayHudSprite::Digit2,
        3 => GameplayHudSprite::Digit3,
        4 => GameplayHudSprite::Digit4,
        5 => GameplayHudSprite::Digit5,
        6 => GameplayHudSprite::Digit6,
        7 => GameplayHudSprite::Digit7,
        8 => GameplayHudSprite::Digit8,
        9 => GameplayHudSprite::Digit9,
        _ => unreachable!("decimal digit"),
    }
}

fn status_panel_layers(
    resources: &GameplayHudResources,
    layout: GameplayHudLayout,
    reveal_percent: u32,
) -> [GameplayHudLayer; 2] {
    let left = resources.sprite(GameplayHudSprite::StatusLeft);
    let position = layout.absolute(layout.status_halves);
    let visible_width = left.width.saturating_mul(reveal_percent.min(100)) / 100;
    let visible_width_i32 = i32::try_from(visible_width).unwrap_or(i32::MAX);
    let left_width = position.x.saturating_add(visible_width_i32).max(0) as u32;

    let right_position = GameplayHudPoint {
        x: position
            .x
            .saturating_add(i32::try_from(left.width).unwrap_or(i32::MAX))
            .saturating_add(1),
        y: position.y,
    };
    // Retail uses sprite 504's width for both complementary clip boundaries.
    // The authored 504/505 pair has equal widths in every system variant.
    let right_clip_x = right_position
        .x
        .saturating_add(i32::try_from(left.width).unwrap_or(i32::MAX))
        .saturating_sub(visible_width_i32)
        .clamp(0, layout.virtual_width as i32);

    [
        layer(
            GameplayHudLayerRole::StatusLeft,
            GameplayHudSprite::StatusLeft,
            position,
            Some(GameplayHudClip {
                x: 0,
                y: 0,
                width: left_width.min(layout.virtual_width),
                height: layout.virtual_height,
            }),
        ),
        layer(
            GameplayHudLayerRole::StatusRight,
            GameplayHudSprite::StatusRight,
            right_position,
            Some(GameplayHudClip {
                x: right_clip_x,
                y: 0,
                width: layout.virtual_width.saturating_sub(right_clip_x as u32),
                height: layout.virtual_height,
            }),
        ),
    ]
}

const fn layer(
    role: GameplayHudLayerRole,
    sprite: GameplayHudSprite,
    position: GameplayHudPoint,
    clip: Option<GameplayHudClip>,
) -> GameplayHudLayer {
    GameplayHudLayer {
        role,
        sprite,
        position,
        clip,
    }
}

fn cargo_frames(
    resources: &GameplayHudResources,
    layout: GameplayHudLayout,
    cargo_models: &[Option<usize>],
    spin_raw: u16,
    retail_tick: u32,
) -> Vec<GameplayHudCargoFrame> {
    let slot_count = i32::try_from(cargo_models.len()).unwrap_or(i32::MAX);
    let row_origin = layout.absolute(layout.cargo_origin);
    let centre_offset = layout.cargo_spacing.x.wrapping_mul(slot_count) / 2;
    let marker = resources.sprite(GameplayHudSprite::CargoEmpty);
    let marker_half_width = i32::try_from(marker.width / 2).unwrap_or(i32::MAX);
    let marker_height = i32::try_from(marker.height).unwrap_or(i32::MAX);

    cargo_models
        .iter()
        .enumerate()
        .map(|(slot, &global_model_id)| {
            let slot = i32::try_from(slot).unwrap_or(i32::MAX);
            let x = row_origin
                .x
                .wrapping_add(layout.cargo_spacing.x.wrapping_mul(slot))
                .wrapping_sub(centre_offset);
            GameplayHudCargoFrame {
                global_model_id,
                marker: GameplayHudSprite::CargoEmpty,
                marker_position: GameplayHudPoint {
                    x: x.wrapping_sub(marker_half_width),
                    y: row_origin.y.wrapping_sub(marker_height),
                },
                model_position: GameplayHudPoint { x, y: row_origin.y },
                spin_raw,
                animation: GameplayHudCargoAnimation::from_retail_tick(retail_tick),
            }
        })
        .collect()
}

fn vertical_clip(layout: GameplayHudLayout, top: i32) -> GameplayHudClip {
    let top = top.clamp(0, layout.virtual_height as i32);
    GameplayHudClip {
        x: 0,
        y: top,
        width: layout.virtual_width,
        height: (layout.virtual_height as i32 - top) as u32,
    }
}

/// Top of sprite 502's visible fuel fill in authored pixels.
pub fn fuel_fill_clip_top(layout: GameplayHudLayout, sprite_height: u32, fuel_raw: i32) -> i32 {
    let height = sprite_height as i32;
    let active_height = height * 75 / 100;
    let fuel_raw = fuel_raw.clamp(0, FUEL_FULL_RAW);
    let unfilled = (active_height - fuel_raw * active_height / FUEL_FULL_RAW).max(0);
    layout.absolute(layout.fuel).y + height * 10 / 100 + unfilled
}

/// Top of sprite 503's visible hull fill in authored pixels.
pub fn hull_fill_clip_top(
    layout: GameplayHudLayout,
    sprite_height: u32,
    hull_value_raw: i32,
) -> i32 {
    let height = sprite_height as i32 + 1;
    let active_height = height * 75 / 100;
    let hull_value_raw = hull_value_raw.clamp(0, HULL_FULL_RAW);
    layout.absolute(layout.hull).y - 1 + height * 10 / 100 + active_height
        - hull_value_raw * active_height / HULL_FULL_RAW
}

/// Advance `DAT_004DB1B8` for one accepted `FUN_0044FFA0` gameplay update.
///
/// Retail caps the frame delta before calling `FUN_00429240`. Menu/cinematic
/// time and the early mode-transition return must not call this helper; the
/// accumulator persists and intentionally wraps.
pub fn advance_retail_bar_clock(clock: u32, frame_delta_micros: u32) -> u32 {
    clock.wrapping_add(frame_delta_micros.min(RETAIL_FRAME_DELTA_MAX_US))
}

/// Advance `DAT_004DB1B4` exactly as `FUN_00429240` does. The decompiled
/// high-word/low-sign-bit expression is an arithmetic shift by 20 after the
/// `58_000 * dt` product.
pub fn advance_retail_weapon_spin(spin: u16, frame_delta_micros: u32) -> u16 {
    advance_retail_hud_spin(spin, frame_delta_micros, WEAPON_SPIN_RATE)
}

/// Advance `DAT_004DB1A8`, the slower cargo-model angle accumulator.
pub fn advance_retail_cargo_spin(spin: u16, frame_delta_micros: u32) -> u16 {
    advance_retail_hud_spin(spin, frame_delta_micros, CARGO_SPIN_RATE)
}

fn advance_retail_hud_spin(spin: u16, frame_delta_micros: u32, rate: u64) -> u16 {
    let elapsed = u64::from(frame_delta_micros.min(RETAIL_FRAME_DELTA_MAX_US));
    let delta = ((elapsed * rate) >> 20) as u16;
    spin.wrapping_add(delta)
}

/// `FUN_0042A6F0`'s selected-item depth policy. The width override is odd at
/// the 640-pixel boundary, but it is authored behavior rather than a smooth
/// approximation and must use the renderer's logical display width.
/// `display_width` is the selected retail presentation tier's logical width
/// (320/640/800/1024), never the final host-window viewport width.
pub fn retail_weapon_depth_raw(carousel_offset_y: i32, display_width: u32) -> i32 {
    if display_width > RETAIL_HIGH_RES_WIDTH_THRESHOLD {
        let width = i32::try_from(display_width).unwrap_or(i32::MAX);
        width
            .wrapping_sub(RETAIL_HIGH_RES_WIDTH_THRESHOLD as i32)
            .wrapping_mul(0x10)
    } else {
        carousel_offset_y
            .wrapping_abs()
            .wrapping_mul(0x104)
            .wrapping_add(3000)
    }
}

/// Exact `FUN_0042A2E0(0x9000, 0x0C00, spin)` orientation, transposed from
/// retail's contiguous lateral/up/forward vectors into renderer rows.
pub fn retail_weapon_orientation(spin: u16) -> [[f32; 3]; 3] {
    retail_orientation(DEFAULT_WEAPON_ANGLE_A, DEFAULT_WEAPON_ANGLE_B, spin)
}

fn retail_orientation(angle_a: u16, angle_b: u16, angle_c: u16) -> [[f32; 3]; 3] {
    let raw = retail_euler_basis_q31(angle_a, angle_b, angle_c);
    std::array::from_fn(|row| {
        std::array::from_fn(|column| raw[column][row] as f32 / 2_147_483_648.0)
    })
}

fn retail_euler_basis_q31(angle_a: u16, angle_b: u16, angle_c: u16) -> [[i32; 3]; 3] {
    let sin_b = duplicated_sine_q31(angle_b);
    let cos_b = duplicated_sine_q31(angle_b.wrapping_add(0x4000));
    let sin_a = duplicated_sine_q31(angle_a);
    let cos_a = duplicated_sine_q31(angle_a.wrapping_add(0x4000));
    let sin_c = duplicated_sine_q31(angle_c);
    let cos_c = duplicated_sine_q31(angle_c.wrapping_add(0x4000));

    let cos_c_sin_a = q31_mul(cos_c, sin_a);
    let cos_c_cos_a = q31_mul(cos_c, cos_a);
    let sin_c_sin_a = q31_mul(sin_c, sin_a);
    let sin_c_cos_a = q31_mul(sin_c, cos_a);

    [
        [
            q31_mul(sin_c_cos_a, sin_b).wrapping_add(cos_c_sin_a),
            q31_mul(sin_c, cos_b),
            q31_mul(sin_c_sin_a, sin_b).wrapping_sub(cos_c_cos_a),
        ],
        [
            q31_mul(cos_c_cos_a, sin_b).wrapping_sub(sin_c_sin_a),
            q31_mul(cos_c, cos_b),
            q31_mul(cos_c_sin_a, sin_b).wrapping_add(sin_c_cos_a),
        ],
        [
            q31_mul(cos_b, cos_a),
            sin_b.wrapping_neg(),
            q31_mul(cos_b, sin_a),
        ],
    ]
}

/// Signed Q31 sine from `FUN_0042A2E0`. Retail duplicates the positive table
/// word first, then negates that full 32-bit magnitude in negative quadrants;
/// equivalently this is signed `word * 0x10001`.
fn duplicated_sine_q31(angle: u16) -> i32 {
    retail_sine_q15(u32::from(angle)).wrapping_mul(0x1_0001)
}

fn q31_mul(lhs: i32, rhs: i32) -> i32 {
    ((i64::from(lhs) * i64::from(rhs)) >> 31) as i32
}

/// Retail low-bar flash multiplier from `(DAT_004DB1B8 >> 18) & 7`.
pub fn retail_bar_flash_factor(retail_flash_clock: u32) -> i32 {
    match (retail_flash_clock >> 18) & 7 {
        0 => 5,
        2 | 6 => 7,
        4 => 3,
        _ => 0,
    }
}

pub fn should_flash_fuel(fuel_raw: i32, flash_factor: i32) -> bool {
    fuel_raw != 0
        && flash_factor != 0
        && fuel_raw.clamp(0, FUEL_FULL_RAW) * flash_factor < FUEL_FULL_RAW + 1
}

pub fn should_flash_hull(hull_health_raw: i32, flash_factor: i32) -> bool {
    hull_health_raw != 0
        && flash_factor != 0
        && hull_health_raw.clamp(0, HULL_FULL_RAW) * flash_factor < HULL_FULL_RAW + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon_inventory::WEAPON_MASTER_TABLE_RAW;

    #[test]
    fn weapon_fallback_sprite_uses_descriptor_high_word() {
        let gatling = GameplayHudWeaponState::from_descriptor(&WeaponDescriptor::from_raw(
            crate::weapon_inventory::DEFAULT_WEAPON_DESCRIPTOR_RAW,
        ));
        assert_eq!(
            gatling.fallback_sprite,
            Some(GameplayHudSprite::WeaponFallback.global_id())
        );
        let selector5 = GameplayHudWeaponState::from_descriptor(&WeaponDescriptor::from_raw(
            WEAPON_MASTER_TABLE_RAW[6],
        ));
        assert_eq!(selector5.selector, 5);
        assert_eq!(selector5.fallback_sprite, Some(520));
    }

    fn variant_zero_layout() -> GameplayHudLayout {
        GameplayHudLayout {
            virtual_width: 320,
            virtual_height: 240,
            base: GameplayHudPoint { x: 7, y: 165 },
            outer_frame: GameplayHudPoint { x: 0, y: 0 },
            final_frame: GameplayHudPoint { x: 0, y: 0 },
            inner_frame: GameplayHudPoint { x: 0, y: 0 },
            fuel: GameplayHudPoint { x: 0, y: 1 },
            hull: GameplayHudPoint { x: 40, y: 2 },
            digits: GameplayHudPoint { x: 22, y: 8 },
            status_halves: GameplayHudPoint { x: 20, y: 8 },
            center_shade: GameplayHudPoint { x: 0, y: 0 },
            weapon_fallback: GameplayHudPoint { x: 22, y: 22 },
            weapon_model: GameplayHudPoint { x: 37, y: 33 },
            weapon_spacing: GameplayHudPoint { x: 0, y: 20 },
            cargo_origin: GameplayHudPoint { x: 37, y: 72 },
            cargo_spacing: GameplayHudPoint { x: 10, y: 0 },
            time_trophy: GameplayHudTimeTrophyLayout {
                model: GameplayHudPoint { x: 30, y: 15 },
                text: GameplayHudPoint { x: 45, y: 12 },
            },
        }
    }

    fn fake_sprite(width: u32, height: u32) -> DecodedGameplayHudSprite {
        DecodedGameplayHudSprite {
            rgba: vec![255; (width * height * 4) as usize],
            width,
            height,
            blend: WorldSpriteBlend::Masked,
        }
    }

    fn resources() -> GameplayHudResources {
        GameplayHudResources {
            final_frame: fake_sprite(62, 61),
            outer_frame: fake_sprite(62, 61),
            inner_frame: fake_sprite(62, 61),
            fuel: fake_sprite(23, 58),
            hull: fake_sprite(23, 58),
            status_left: fake_sprite(11, 11),
            status_right: fake_sprite(11, 11),
            center_shade: fake_sprite(64, 64),
            digits: std::array::from_fn(|_| fake_sprite(4, 7)),
            weapon_fallback: fake_sprite(32, 32),
            level_one_weapon_fallback: fake_sprite(24, 24),
            cargo_empty: fake_sprite(6, 6),
        }
    }

    fn default_weapon_roster() -> GameplayHudWeaponRoster {
        GameplayHudWeaponRoster::default()
    }

    #[test]
    fn variant_zero_fill_boundaries_match_retail_integer_math() {
        let layout = variant_zero_layout();

        assert_eq!(fuel_fill_clip_top(layout, 58, FUEL_FULL_RAW), 171);
        assert_eq!(fuel_fill_clip_top(layout, 58, FUEL_FULL_RAW / 2), 193);
        assert_eq!(fuel_fill_clip_top(layout, 58, 0), 214);

        assert_eq!(hull_fill_clip_top(layout, 58, HULL_FULL_RAW), 171);
        assert_eq!(hull_fill_clip_top(layout, 58, HULL_FULL_RAW / 2), 193);
        assert_eq!(hull_fill_clip_top(layout, 58, 0), 215);
    }

    #[test]
    fn flash_phase_and_strict_thresholds_match_retail() {
        assert_eq!(retail_bar_flash_factor(0), 5);
        assert_eq!(retail_bar_flash_factor(1 << 18), 0);
        assert_eq!(retail_bar_flash_factor(2 << 18), 7);
        assert_eq!(retail_bar_flash_factor(4 << 18), 3);
        assert_eq!(retail_bar_flash_factor(6 << 18), 7);

        assert!(should_flash_fuel(40_000, 5));
        assert!(!should_flash_fuel(40_001, 5));
        assert!(!should_flash_fuel(0, 5));
        assert!(should_flash_hull(8_000, 5));
        assert!(!should_flash_hull(8_001, 5));
        assert!(!should_flash_hull(1, 0));
    }

    #[test]
    fn retail_flash_clock_caps_each_accepted_gameplay_delta_and_wraps() {
        assert_eq!(advance_retail_bar_clock(10, 20), 30);
        assert_eq!(advance_retail_bar_clock(10, 200_000), 125_010);
        assert_eq!(advance_retail_bar_clock(u32::MAX - 4, 10), 5);
    }

    #[test]
    fn frame_layers_follow_retail_order_and_virtual_positions() {
        let resources = resources();
        let layout = variant_zero_layout();
        let mut hud = GameplayHud::default();
        let weapons = default_weapon_roster();

        let frame = hud.frame(
            &resources,
            layout,
            FUEL_FULL_RAW,
            HULL_FULL_RAW,
            0,
            1 << 18,
            &weapons,
            &[],
            TimeTrophyRuntime::default(),
        );
        let roles: Vec<_> = frame.layers.iter().map(|layer| layer.role).collect();
        assert_eq!(
            roles,
            [
                GameplayHudLayerRole::OuterFrame,
                GameplayHudLayerRole::FuelFill,
                GameplayHudLayerRole::HullFill,
                GameplayHudLayerRole::StatusLeft,
                GameplayHudLayerRole::StatusRight,
                GameplayHudLayerRole::InnerFrame,
                GameplayHudLayerRole::CenterShade,
                GameplayHudLayerRole::FinalFrame,
            ]
        );
        assert_eq!(frame.layers[0].position, GameplayHudPoint { x: 7, y: 165 });
        assert_eq!(frame.layers[1].position, GameplayHudPoint { x: 7, y: 166 });
        assert_eq!(frame.layers[2].position, GameplayHudPoint { x: 47, y: 167 });
        assert_eq!(frame.layers[3].position, GameplayHudPoint { x: 27, y: 173 });
        assert_eq!(frame.layers[3].clip.unwrap().width, 28);
        assert_eq!(frame.layers[4].position, GameplayHudPoint { x: 39, y: 173 });
        assert_eq!(frame.layers[4].clip.unwrap().x, 49);
        assert_eq!(hud.status_reveal_percent(), 16);
        assert_eq!(frame.weapons.len(), 1);
        assert_eq!(
            frame.weapons[0].global_model_id,
            Some(DEFAULT_WEAPON_MODEL_ID)
        );
        assert_eq!(
            frame.weapons[0].fallback_sprite,
            Some(GameplayHudSprite::WeaponFallback.global_id())
        );
        assert_eq!(
            frame.weapons[0].model_position,
            GameplayHudPoint { x: 44, y: 198 }
        );
        assert_eq!(
            frame.weapons[0].fallback_position,
            GameplayHudPoint { x: 29, y: 187 }
        );
        assert_eq!(frame.weapons[0].sprite_size_percent_raw, 0);
        assert_eq!(hud.visible_hull_raw(), HULL_FULL_RAW);
    }

    #[test]
    fn low_bars_flash_before_their_clipped_fills() {
        let resources = resources();
        let layout = variant_zero_layout();
        let mut hud = GameplayHud {
            visible_hull_raw: 8_000,
            ..GameplayHud::default()
        };
        let weapons = default_weapon_roster();

        let frame = hud.frame(
            &resources,
            layout,
            40_000,
            8_000,
            0,
            0,
            &weapons,
            &[],
            TimeTrophyRuntime::default(),
        );
        let roles: Vec<_> = frame.layers.iter().map(|layer| layer.role).collect();
        assert_eq!(
            roles,
            [
                GameplayHudLayerRole::OuterFrame,
                GameplayHudLayerRole::FuelFlash,
                GameplayHudLayerRole::FuelFill,
                GameplayHudLayerRole::HullFlash,
                GameplayHudLayerRole::HullFill,
                GameplayHudLayerRole::StatusLeft,
                GameplayHudLayerRole::StatusRight,
                GameplayHudLayerRole::InnerFrame,
                GameplayHudLayerRole::CenterShade,
                GameplayHudLayerRole::FinalFrame,
            ]
        );
        assert_eq!(frame.layers[1].clip, None);
        assert_eq!(frame.layers[2].clip.unwrap().y, 206);
        assert_eq!(frame.layers[3].clip, None);
        assert_eq!(frame.layers[4].clip.unwrap().y, 207);
    }

    #[test]
    fn infinite_ammo_panels_reveal_from_opposite_edges_and_cap_at_100() {
        let resources = resources();
        let layout = variant_zero_layout();
        let mut hud = GameplayHud::default();
        let weapons = default_weapon_roster();
        let mut frame = None;

        for _ in 0..7 {
            frame = Some(hud.frame(
                &resources,
                layout,
                FUEL_FULL_RAW,
                HULL_FULL_RAW,
                0,
                1 << 18,
                &weapons,
                &[],
                TimeTrophyRuntime::default(),
            ));
        }

        let frame = frame.expect("seventh HUD frame");
        assert_eq!(hud.status_reveal_percent(), 100);
        let left = frame
            .layers
            .iter()
            .find(|layer| layer.role == GameplayHudLayerRole::StatusLeft)
            .expect("left status panel");
        let right = frame
            .layers
            .iter()
            .find(|layer| layer.role == GameplayHudLayerRole::StatusRight)
            .expect("right status panel");
        assert_eq!(left.position, GameplayHudPoint { x: 27, y: 173 });
        assert_eq!(left.clip.unwrap().width, 38);
        assert_eq!(right.position, GameplayHudPoint { x: 39, y: 173 });
        assert_eq!(right.clip.unwrap().x, 39);

        let _ = hud.frame(
            &resources,
            layout,
            FUEL_FULL_RAW,
            HULL_FULL_RAW,
            0,
            1 << 18,
            &weapons,
            &[],
            TimeTrophyRuntime::default(),
        );
        assert_eq!(hud.status_reveal_percent(), 100);
    }

    #[test]
    fn finite_weapon_uses_authored_sprite_and_three_decimal_digits() {
        use crate::weapon_inventory::{PowerUpPayload, WeaponInventory};

        let resources = resources();
        let layout = variant_zero_layout();
        let mut inventory = WeaponInventory::new();
        inventory.acquire_weapon(PowerUpPayload::from_runtime_word(0x0000_c802));
        let weapon_roster = GameplayHudWeaponRoster::from_inventory(&inventory);
        let mut hud = GameplayHud::default();

        let frame = hud.frame(
            &resources,
            layout,
            FUEL_FULL_RAW,
            HULL_FULL_RAW,
            0,
            1 << 18,
            &weapon_roster,
            &[],
            TimeTrophyRuntime::default(),
        );

        assert_eq!(frame.weapons.len(), 2);
        assert_eq!(frame.weapons[1].global_model_id, None);
        assert_eq!(
            frame.weapons[1].fallback_sprite,
            Some(GameplayHudSprite::LevelOneWeaponFallback.global_id())
        );
        let ammo_layers: Vec<_> = frame
            .layers
            .iter()
            .filter(|layer| {
                matches!(
                    layer.role,
                    GameplayHudLayerRole::AmmoHundreds
                        | GameplayHudLayerRole::AmmoTens
                        | GameplayHudLayerRole::AmmoOnes
                )
            })
            .collect();
        assert_eq!(ammo_layers.len(), 3);
        assert_eq!(ammo_layers[0].sprite, GameplayHudSprite::Digit2);
        assert_eq!(ammo_layers[1].sprite, GameplayHudSprite::Digit0);
        assert_eq!(ammo_layers[2].sprite, GameplayHudSprite::Digit0);
        assert_eq!(ammo_layers[0].position, GameplayHudPoint { x: 29, y: 173 });
        assert_eq!(ammo_layers[1].position, GameplayHudPoint { x: 34, y: 173 });
        assert_eq!(ammo_layers[2].position, GameplayHudPoint { x: 39, y: 173 });
        assert_eq!(hud.status_reveal_percent(), 0);
    }

    #[test]
    fn selected_weapon_change_uses_retail_neighbor_order_offsets_and_easing() {
        use crate::weapon_inventory::{PowerUpPayload, WeaponCycleDirection, WeaponInventory};

        let resources = resources();
        let layout = variant_zero_layout();
        let mut inventory = WeaponInventory::new();
        let initial_roster = GameplayHudWeaponRoster::from_inventory(&inventory);
        let mut hud = GameplayHud::default();

        let initial = hud.frame(
            &resources,
            layout,
            FUEL_FULL_RAW,
            HULL_FULL_RAW,
            0,
            0,
            &initial_roster,
            &[],
            TimeTrophyRuntime::default(),
        );
        assert_eq!(
            initial
                .weapons
                .iter()
                .map(|weapon| weapon.selector)
                .collect::<Vec<_>>(),
            [1]
        );

        inventory.acquire_weapon(PowerUpPayload::from_runtime_word(0x0000_c802));
        let upgraded_roster = GameplayHudWeaponRoster::from_inventory(&inventory);
        assert_eq!(upgraded_roster.len(), 2);
        assert_eq!(upgraded_roster.selected_index, 1);

        let forward = hud.frame(
            &resources,
            layout,
            FUEL_FULL_RAW,
            HULL_FULL_RAW,
            0,
            0,
            &upgraded_roster,
            &[],
            TimeTrophyRuntime::default(),
        );
        assert_eq!(hud.weapon_carousel_offset_y(), 14);
        assert_eq!(
            forward
                .weapons
                .iter()
                .map(|weapon| (
                    weapon.selector,
                    weapon.carousel_offset_y,
                    weapon.sprite_size_percent_raw,
                ))
                .collect::<Vec<_>>(),
            [(1, -6, 42), (2, 14, 58)]
        );
        assert_eq!(
            forward.weapons[1].fallback_position,
            GameplayHudPoint { x: 29, y: 201 }
        );

        inventory.cycle(WeaponCycleDirection::Backward);
        let default_roster = GameplayHudWeaponRoster::from_inventory(&inventory);
        let backward = hud.frame(
            &resources,
            layout,
            FUEL_FULL_RAW,
            HULL_FULL_RAW,
            0,
            0,
            &default_roster,
            &[],
            TimeTrophyRuntime::default(),
        );
        assert_eq!(hud.weapon_carousel_offset_y(), -14);
        assert_eq!(
            backward
                .weapons
                .iter()
                .map(|weapon| (
                    weapon.selector,
                    weapon.carousel_offset_y,
                    weapon.sprite_size_percent_raw,
                ))
                .collect::<Vec<_>>(),
            [(2, 6, 42), (1, -14, 58)]
        );
    }

    #[test]
    fn retail_carousel_wrap_and_decay_match_signed_integer_branches() {
        assert_eq!(retail_weapon_selection_offset(3, 0, 20), 20);
        assert_eq!(retail_weapon_selection_offset(0, 3, 20), -20);
        assert_eq!(retail_weapon_selection_offset(1, 2, 20), 20);
        assert_eq!(retail_weapon_selection_offset(2, 1, 20), -20);

        let positive: Vec<_> = std::iter::successors(Some(20), |&offset| {
            (offset != 0).then(|| advance_retail_weapon_carousel_offset(offset))
        })
        .collect();
        let negative: Vec<_> = std::iter::successors(Some(-20), |&offset| {
            (offset != 0).then(|| advance_retail_weapon_carousel_offset(offset))
        })
        .collect();
        assert_eq!(positive, [20, 14, 10, 7, 5, 3, 2, 1, 0]);
        assert_eq!(negative, [-20, -14, -10, -7, -5, -3, -2, -1, 0]);
    }

    #[test]
    fn retail_fallback_scaling_keeps_zero_full_size_and_truncates_symmetrically() {
        let origin = GameplayHudPoint { x: 10, y: 20 };
        assert_eq!(
            retail_weapon_sprite_rect(origin, 24, 24, 0),
            (origin, 24, 24)
        );
        assert_eq!(
            retail_weapon_sprite_rect(origin, 24, 24, 42),
            (GameplayHudPoint { x: 17, y: 27 }, 10, 10)
        );
        assert_eq!(
            retail_weapon_sprite_rect(origin, 5, 5, 1),
            (GameplayHudPoint { x: 12, y: 22 }, 1, 1)
        );
        assert_eq!(
            retail_weapon_sprite_rect(origin, 24, 24, 100),
            (origin, 24, 24)
        );
    }

    #[test]
    fn hull_damage_draws_old_trail_then_updates_persistent_value() {
        let resources = resources();
        let layout = variant_zero_layout();
        let mut hud = GameplayHud {
            visible_hull_raw: HULL_FULL_RAW,
            ..GameplayHud::default()
        };
        let weapons = default_weapon_roster();

        let frame = hud.frame(
            &resources,
            layout,
            FUEL_FULL_RAW,
            36_772,
            0,
            1 << 18,
            &weapons,
            &[],
            TimeTrophyRuntime::default(),
        );
        let roles: Vec<_> = frame.layers.iter().map(|layer| layer.role).collect();
        assert_eq!(
            roles,
            [
                GameplayHudLayerRole::OuterFrame,
                GameplayHudLayerRole::FuelFill,
                GameplayHudLayerRole::HullTrail,
                GameplayHudLayerRole::HullFill,
                GameplayHudLayerRole::StatusLeft,
                GameplayHudLayerRole::StatusRight,
                GameplayHudLayerRole::InnerFrame,
                GameplayHudLayerRole::CenterShade,
                GameplayHudLayerRole::FinalFrame,
            ]
        );
        assert_eq!(frame.layers[2].clip.unwrap().y, 171);
        assert_eq!(frame.layers[3].clip.unwrap().y, 175);
        assert_eq!(hud.visible_hull_raw(), 39_200);
    }

    #[test]
    fn retail_hud_spins_use_distinct_rates_and_share_the_delta_cap() {
        assert_eq!(advance_retail_cargo_spin(0, 20_000), 381);
        assert_eq!(advance_retail_cargo_spin(0, 200_000), 2_384);
        assert_eq!(advance_retail_weapon_spin(0, 20_000), 1_106);
        assert_eq!(advance_retail_weapon_spin(0, 200_000), 6_914);
        assert_eq!(advance_retail_weapon_spin(u16::MAX - 5, 20_000), 1_100);

        let mut hud = GameplayHud::default();
        hud.advance_spins(20_000);
        assert_eq!(hud.cargo_spin_raw(), 381);
        assert_eq!(hud.weapon_spin_raw(), 1_106);
        hud.reset();
        assert_eq!(
            hud.weapon_spin_raw(),
            1_106,
            "level reset preserves the retail global"
        );
        assert_eq!(hud.cargo_spin_raw(), 381);
    }

    #[test]
    fn cargo_slots_center_the_unlocked_row_and_replace_only_occupied_markers() {
        let resources = resources();
        let layout = variant_zero_layout();
        let mut hud = GameplayHud::default();
        hud.advance_spins(20_000);
        let weapons = default_weapon_roster();
        let frame = hud.frame(
            &resources,
            layout,
            FUEL_FULL_RAW,
            HULL_FULL_RAW,
            23,
            0,
            &weapons,
            &[Some(81), None, Some(42)],
            TimeTrophyRuntime::default(),
        );

        assert_eq!(frame.cargo.len(), 3);
        assert_eq!(
            frame
                .cargo
                .iter()
                .map(|slot| slot.global_model_id)
                .collect::<Vec<_>>(),
            [Some(81), None, Some(42)]
        );
        assert_eq!(
            frame
                .cargo
                .iter()
                .map(|slot| slot.model_position)
                .collect::<Vec<_>>(),
            [
                GameplayHudPoint { x: 29, y: 237 },
                GameplayHudPoint { x: 39, y: 237 },
                GameplayHudPoint { x: 49, y: 237 },
            ]
        );
        assert_eq!(
            frame.cargo[0].marker_position,
            GameplayHudPoint { x: 26, y: 231 }
        );
        assert_eq!(frame.cargo[0].marker, GameplayHudSprite::CargoEmpty);
        assert_eq!(frame.cargo[0].depth_raw(), 4_000);
        assert_eq!(frame.cargo[0].spin_raw, 381);
        assert_eq!(frame.cargo[0].animation.dynamic_0, 23);
        assert_eq!(frame.cargo[0].animation.dynamic_1, 3);
    }

    #[test]
    fn cargo_animation_callback_changes_on_exact_six_tick_boundaries() {
        for (retail_tick, expected_phase) in [
            (0, 0),
            (5, 0),
            (6, 1),
            (11, 1),
            (12, 2),
            (17, 2),
            (18, 3),
            (23, 3),
            (24, 0),
        ] {
            let animation = GameplayHudCargoAnimation::from_retail_tick(retail_tick);
            assert_eq!(animation.dynamic_0, retail_tick as i32);
            assert_eq!(animation.dynamic_1, expected_phase);

            let vars = animation.anim_vars();
            assert_eq!(vars.dynamic[0], retail_tick as i32);
            assert_eq!(vars.dynamic[1], expected_phase);
            assert!(vars.dynamic[2..].iter().all(|&value| value == 0));
        }
    }

    #[test]
    fn retail_weapon_depth_preserves_both_executable_branches() {
        assert_eq!(retail_weapon_depth_raw(0, 320), 3_000);
        assert_eq!(retail_weapon_depth_raw(20, 640), 8_200);
        assert_eq!(retail_weapon_depth_raw(-20, 640), 8_200);
        assert_eq!(retail_weapon_depth_raw(0, 800), 2_560);
    }

    #[test]
    fn retail_weapon_matrix_uses_the_common_transposed_basis_contract() {
        let negative_angle = 0x9000;
        let sine_q15 = retail_sine_q15(u32::from(negative_angle));
        assert!(sine_q15 < 0);
        assert_eq!(
            duplicated_sine_q31(negative_angle),
            sine_q15.wrapping_mul(0x1_0001)
        );

        let raw = retail_euler_basis_q31(0, 0, 0);
        let orientation = std::array::from_fn::<_, 3, _>(|row| {
            std::array::from_fn::<_, 3, _>(|column| raw[column][row] as f32 / 2_147_483_648.0)
        });

        // FUN_0042A2E0 stores lateral/up/forward consecutively. At zero
        // angles those vectors describe Ry(-pi/2); transposition therefore
        // gives the renderer Ry(+pi/2), the same conversion used by entities.
        assert!(orientation[0][2] > 0.999);
        assert!(orientation[1][1] > 0.999);
        assert!(orientation[2][0] < -0.999);
        for row in orientation {
            let length = row.iter().map(|value| value * value).sum::<f32>().sqrt();
            assert!((length - 1.0).abs() < 0.001);
        }

        let actual = retail_weapon_orientation(0x1234);
        for row in actual {
            let length = row.iter().map(|value| value * value).sum::<f32>().sqrt();
            assert!((length - 1.0).abs() < 0.001);
        }
    }
}
