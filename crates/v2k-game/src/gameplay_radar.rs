//! Retail terrain-radar and fullscreen-map presentation.
//!
//! `FUN_0044AB20`/`FUN_0044A5C0` build one persistent 256x256, north-up
//! terrain raster. `FUN_004494A0` projects that raster through the three
//! system-level-3 Section-14 resources for the right HUD globe, while
//! `FUN_0044C1F0`/`FUN_0044BCF0` scale the same raster for the modal M map.
//! Entity markers retain intrusive-list order and the original capability and
//! live-state gates. Craft yaw is deliberately absent from every API here.
//!
//! One boundary remains explicit: partial globe-mask pixels read and average
//! the existing RGB565 framebuffer in retail. A conventional RGBA sprite
//! cannot reproduce that destination-dependent integer operation exactly, so
//! those edge pixels use the equivalent source-coverage alpha. Mask 0 (opaque)
//! and mask 15 (preserve destination) remain exact.

use std::collections::HashMap;

#[cfg(test)]
mod relayout_tests;

use v2k_formats::anim_frames::TerrainObjectTable;
use v2k_formats::fixed_math::retail_sine_q15;
use v2k_formats::radar_projection::RadarProjectionTables;
use v2k_formats::system::PaletteEntry;
use v2k_formats::terrain::{TerrainCell, TerrainGrid};

use crate::entity::{Entity, EntityManager};
use crate::entity_collision_state::{RetailRuntimeValue, RetailStateWord};
use crate::gameplay_hud::{gameplay_hud_virtual_size, GAMEPLAY_HUD_SYSTEM_LEVEL};
use crate::resource_cache::ResourceCache;
use crate::system_layout::SystemLayoutSource;

const WORLD_SIZE: usize = 256;
const WORLD_PIXELS: usize = WORLD_SIZE * WORLD_SIZE;
const COVERAGE_BYTES: usize = WORLD_PIXELS / 2;
const TERRAIN_PALETTE_LEN: usize = 35;
const SHADE_PALETTE_LEN: usize = 16;
const MARKER_PALETTE_LEN: usize = 7;
const PACKED_HALF_MASK_RGB565: u16 = 0x7bef;
const PLAYER_ENTITY_TYPE: u32 = 46;
const MAP_BLINK_INTERVAL_US: u32 = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadarRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadarImage {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadarHudFrame {
    pub image: RadarImage,
    pub origin: [i32; 2],
    pub virtual_size: [u32; 2],
}

/// Layout-only system-3 inputs, independently readable from a prepared tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameplayRadarLayout {
    pub globe_size: [usize; 2],
    pub hud_origin: [i32; 2],
    pub virtual_size: [u32; 2],
    pub map_rect: RadarRect,
}

impl GameplayRadarLayout {
    pub fn from_cache(
        cache: &(impl SystemLayoutSource + ?Sized),
        system_variant: u32,
    ) -> Option<Self> {
        let (width, height) = gameplay_hud_virtual_size(system_variant)?;
        let (x, y) = cache.system_layout_point(GAMEPLAY_HUD_SYSTEM_LEVEL, 10)?;
        let layout = Self {
            globe_size: [
                usize::try_from(cache.system_data_value(GAMEPLAY_HUD_SYSTEM_LEVEL, 0)?).ok()?,
                usize::try_from(cache.system_data_value(GAMEPLAY_HUD_SYSTEM_LEVEL, 1)?).ok()?,
            ],
            hud_origin: [i32::from(x), i32::from(y)],
            virtual_size: [width, height],
            map_rect: RadarRect {
                x: i32::try_from(cache.system_data_value(GAMEPLAY_HUD_SYSTEM_LEVEL, 6)?).ok()?,
                y: i32::try_from(cache.system_data_value(GAMEPLAY_HUD_SYSTEM_LEVEL, 7)?).ok()?,
                width: cache.system_data_value(GAMEPLAY_HUD_SYSTEM_LEVEL, 8)?,
                height: cache.system_data_value(GAMEPLAY_HUD_SYSTEM_LEVEL, 9)?,
            },
        };
        layout.valid().then_some(layout)
    }

    fn valid(self) -> bool {
        self.globe_size.iter().all(|&size| size != 0)
            && self.virtual_size.iter().all(|&size| size != 0)
            && self.map_rect.width >= 2
            && self.map_rect.height >= 2
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameplayRadarRelayoutError {
    InvalidLayout,
    GlobeGeometryChanged,
    FrameLayoutMismatch,
}

/// Successful presentation compatibility check, published without fallible work.
#[derive(Debug, Clone, Copy)]
pub struct PreparedGameplayRadarLayout {
    layout: GameplayRadarLayout,
}

impl RadarHudFrame {
    /// Keep the already-projected pixels and accepted marker/RNG decisions.
    pub fn relayout(
        &mut self,
        previous: GameplayRadarLayout,
        next: GameplayRadarLayout,
    ) -> Result<(), GameplayRadarRelayoutError> {
        if !next.valid() {
            return Err(GameplayRadarRelayoutError::InvalidLayout);
        }
        if previous.globe_size != next.globe_size {
            return Err(GameplayRadarRelayoutError::GlobeGeometryChanged);
        }
        if self.origin != previous.hud_origin
            || self.virtual_size != previous.virtual_size
            || [self.image.width as usize, self.image.height as usize] != previous.globe_size
        {
            return Err(GameplayRadarRelayoutError::FrameLayoutMismatch);
        }
        self.origin = next.hud_origin;
        self.virtual_size = next.virtual_size;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedMapIcon {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapIconPlacement {
    pub entity_type: u32,
    pub center_x: i32,
    pub center_y: i32,
}

#[derive(Debug, Clone, Copy)]
struct RadarConfig {
    width: usize,
    height: usize,
    map_rect: RadarRect,
}

#[derive(Debug, Clone)]
struct RadarPalette565 {
    terrain: [u16; TERRAIN_PALETTE_LEN],
    shade: [u16; SHADE_PALETTE_LEN],
    markers: [u16; MARKER_PALETTE_LEN],
}

#[derive(Debug, Clone)]
pub(crate) struct TerrainRadar {
    coverage: Vec<u8>,
    indices: Vec<u8>,
    config: TerrainRadarConfig,
    revision: u64,
}

/// Immutable access to the level's simulation-owned terrain radar allocation.
/// Presentation borrows these buffers and never rebuilds coverage or draws RNG.
#[derive(Debug, Clone, Copy)]
pub struct TerrainRadarView<'a> {
    coverage: &'a [u8],
    indices: &'a [u8],
    revision: u64,
}

impl<'a> TerrainRadarView<'a> {
    pub fn packed_coverage(&self) -> &'a [u8] {
        self.coverage
    }

    pub fn indices(&self) -> &'a [u8] {
        self.indices
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    fn coverage_at_entity(&self, entity: &Entity) -> u8 {
        let [x, _, z] = entity.position_raw();
        coverage_nibble(
            self.coverage,
            usize::from((x as u16) >> 8),
            usize::from((z as u16) >> 8),
        )
    }
}

#[derive(Debug, Clone, Copy)]
struct TerrainRadarConfig {
    color_bands: u8,
    color_stride_minus_one: u8,
    coverage_max: u8,
    footprint_radius: i32,
    underwater_override: bool,
}

/// The signed contribution passed by the Portable Radar task to `44A8D0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RadarCoverageChange {
    Add,
    Remove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainRadarError {
    Uninitialized,
    ResourceUnavailable(&'static str),
    InvalidConfig(&'static str),
}

impl std::fmt::Display for TerrainRadarError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for TerrainRadarError {}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct MapLifecycle {
    open: bool,
    pass: u8,
    blink_phase: u8,
    blink_elapsed_us: u32,
}

struct FullscreenRasterImages {
    revision: u64,
    nearest: RadarImage,
    bilinear: RadarImage,
}

/// Per-level presentation resources and images derived from the shared raster.
pub struct GameplayRadar {
    projection: RadarProjectionTables,
    config: RadarConfig,
    palette: RadarPalette565,
    hud_origin: [i32; 2],
    virtual_size: [u32; 2],
    fullscreen_raster: Option<FullscreenRasterImages>,
    map_icons: HashMap<u32, DecodedMapIcon>,
    lifecycle: MapLifecycle,
}

impl GameplayRadar {
    /// Decode presentation resources after the simulation initializes its
    /// shared raster. Opening or recreating the presentation consumes no RNG.
    pub fn from_cache(cache: &ResourceCache, system_variant: u32) -> Option<Self> {
        cache.level_terrain_radar()?;
        let projection = cache
            .system_radar_projection_tables(GAMEPLAY_HUD_SYSTEM_LEVEL)
            .ok()?;
        let layout = GameplayRadarLayout::from_cache(cache, system_variant)?;
        if layout.globe_size != [projection.width, projection.height] {
            return None;
        }
        let config = RadarConfig {
            width: layout.globe_size[0],
            height: layout.globe_size[1],
            map_rect: layout.map_rect,
        };
        if config.width == 0
            || config.height == 0
            || config.map_rect.width < 2
            || config.map_rect.height < 2
        {
            return None;
        }

        let palette = RadarPalette565::from_cache(cache)?;
        let map_icons = decode_map_icons(cache);

        Some(Self {
            projection,
            config,
            palette,
            hud_origin: layout.hud_origin,
            virtual_size: layout.virtual_size,
            fullscreen_raster: None,
            map_icons,
            lifecycle: MapLifecycle::default(),
        })
    }

    pub fn layout(&self) -> GameplayRadarLayout {
        GameplayRadarLayout {
            globe_size: [self.config.width, self.config.height],
            hud_origin: self.hud_origin,
            virtual_size: self.virtual_size,
            map_rect: self.config.map_rect,
        }
    }

    pub fn prepare_layout_refresh(
        &self,
        next: GameplayRadarLayout,
    ) -> Result<PreparedGameplayRadarLayout, GameplayRadarRelayoutError> {
        if !next.valid() {
            return Err(GameplayRadarRelayoutError::InvalidLayout);
        }
        if next.globe_size != [self.projection.width, self.projection.height] {
            return Err(GameplayRadarRelayoutError::GlobeGeometryChanged);
        }
        Ok(PreparedGameplayRadarLayout { layout: next })
    }

    /// Preserve map pass/blink, icons, palette, projection and simulation raster.
    /// Only the derived fullscreen images depend on the changed map dimensions.
    pub fn refresh_layout(&mut self, prepared: PreparedGameplayRadarLayout) {
        let next = prepared.layout;
        if (self.config.map_rect.width, self.config.map_rect.height)
            != (next.map_rect.width, next.map_rect.height)
        {
            self.fullscreen_raster = None;
        }
        self.config.map_rect = next.map_rect;
        self.hud_origin = next.hud_origin;
        self.virtual_size = next.virtual_size;
    }

    pub fn hud_frame(
        &self,
        terrain: TerrainRadarView<'_>,
        entities: &EntityManager,
        retail_tick: u32,
        next_random: &mut impl FnMut() -> u16,
    ) -> Option<RadarHudFrame> {
        let player = entities.player()?;
        let mut image = project_globe(
            &self.projection,
            &self.palette,
            terrain.indices,
            player.position_raw(),
        );
        for entity in entities.iter() {
            if let Some(marker) = self.hud_marker(terrain, player, entity, retail_tick, next_random)
            {
                plot_marker(&mut image, marker);
            }
        }
        Some(RadarHudFrame {
            image,
            origin: self.hud_origin,
            virtual_size: self.virtual_size,
        })
    }

    pub fn map_rect(&self) -> RadarRect {
        self.config.map_rect
    }

    pub fn virtual_size(&self) -> [u32; 2] {
        self.virtual_size
    }

    pub fn is_fullscreen_open(&self) -> bool {
        self.lifecycle.open
    }

    pub fn enter_fullscreen(&mut self) {
        self.lifecycle = MapLifecycle {
            open: true,
            ..MapLifecycle::default()
        };
    }

    pub fn leave_fullscreen(&mut self) {
        self.lifecycle = MapLifecycle::default();
    }

    /// Advance only the map's presentation clock. Gameplay and the retail
    /// process tick stay frozen while this modal is open.
    pub fn advance_fullscreen(&mut self, elapsed_micros: u32) {
        if !self.lifecycle.open {
            return;
        }
        if self.lifecycle.pass < 4 {
            self.lifecycle.pass += 1;
            if self.lifecycle.pass == 3 {
                self.lifecycle.blink_phase = 1;
                self.lifecycle.blink_elapsed_us = 0;
            }
            return;
        }
        self.lifecycle.blink_elapsed_us = self
            .lifecycle
            .blink_elapsed_us
            .saturating_add(elapsed_micros);
        while self.lifecycle.blink_elapsed_us > MAP_BLINK_INTERVAL_US {
            self.lifecycle.blink_elapsed_us -= MAP_BLINK_INTERVAL_US;
            self.lifecycle.blink_phase = (self.lifecycle.blink_phase + 1) % 3;
        }
    }

    /// The first two retail UI passes are blank setup frames. Pass three uses
    /// nearest sampling; subsequent passes use the recovered RGB interpolation.
    pub fn fullscreen_image(&mut self, terrain: TerrainRadarView<'_>) -> Option<&RadarImage> {
        if !self.lifecycle.open || self.lifecycle.pass < 3 {
            return None;
        }
        if self
            .fullscreen_raster
            .as_ref()
            .is_none_or(|images| images.revision != terrain.revision)
        {
            self.fullscreen_raster = Some(FullscreenRasterImages {
                revision: terrain.revision,
                nearest: build_fullscreen_map(
                    terrain.indices,
                    &self.palette.terrain,
                    self.config.map_rect,
                    false,
                ),
                bilinear: build_fullscreen_map(
                    terrain.indices,
                    &self.palette.terrain,
                    self.config.map_rect,
                    true,
                ),
            });
        }
        let images = self.fullscreen_raster.as_ref()?;
        match self.lifecycle.pass {
            0..=2 => None,
            3 => Some(&images.nearest),
            _ => Some(&images.bilinear),
        }
    }

    pub fn fullscreen_icon(&self, entity_type: u32) -> Option<&DecodedMapIcon> {
        self.map_icons.get(&entity_type)
    }

    pub fn fullscreen_icon_placements(
        &self,
        terrain: TerrainRadarView<'_>,
        entities: &EntityManager,
        next_random: &mut impl FnMut() -> u16,
    ) -> Vec<MapIconPlacement> {
        if !self.lifecycle.open || self.lifecycle.pass < 3 || self.lifecycle.blink_phase == 0 {
            return Vec::new();
        }
        let rect = self.config.map_rect;
        let mut placements = Vec::new();
        for entity in entities.iter() {
            if !self.map_icons.contains_key(&entity.entity_type)
                || !map_state_is_visible(entity.collision.state_flags_at_0x08)
            {
                continue;
            }
            if terrain.coverage_at_entity(entity) == 0
                && entity.entity_type != PLAYER_ENTITY_TYPE
                && next_random() & 3 != 0
            {
                continue;
            }
            let [x_raw, _, z_raw] = entity.position_raw();
            let cell_x = u32::from((x_raw as u16) >> 8);
            let cell_z = u32::from((z_raw as u16) >> 8);
            placements.push(MapIconPlacement {
                entity_type: entity.entity_type,
                center_x: rect.x + ((cell_x * rect.width) / WORLD_SIZE as u32) as i32,
                center_y: rect.y
                    + (((WORLD_SIZE as u32 - cell_z) * rect.height) / WORLD_SIZE as u32) as i32,
            });
        }
        placements
    }

    fn hud_marker(
        &self,
        terrain: TerrainRadarView<'_>,
        player: &Entity,
        entity: &Entity,
        retail_tick: u32,
        next_random: &mut impl FnMut() -> u16,
    ) -> Option<Marker> {
        let capabilities = entity.capability_flags;
        if capabilities & 0x10 == 0 {
            let state = entity.collision.state_flags_at_0x08;
            match state.masked(0x4000) {
                RetailRuntimeValue::Known(0) => {}
                RetailRuntimeValue::Known(_) | RetailRuntimeValue::Unresolved => return None,
            }
            if state_is_exact_zero(state) || !state_proves_nonzero(state) {
                return None;
            }
        }
        if terrain.coverage_at_entity(entity) == 0
            && entity.entity_type != PLAYER_ENTITY_TYPE
            && next_random() & 3 != 0
        {
            return None;
        }
        if capabilities & 0x8ebd == 0 {
            return None;
        }

        let player_raw = player.position_raw();
        let entity_raw = entity.position_raw();
        let dx = entity_raw[0].wrapping_sub(player_raw[0]);
        let dz = entity_raw[2].wrapping_sub(player_raw[2]);
        let dx_i32 = i32::from(dx);
        let dz_i32 = i32::from(dz);
        let distance_input = ((i64::from(dx_i32) * i64::from(dx_i32)) >> 2)
            + ((i64::from(dz_i32) * i64::from(dz_i32)) >> 2);
        let distance = integer_sqrt(distance_input.max(0) as u64).saturating_mul(2) as i32;
        let center_x = self.config.width as i32 / 2;
        let center_y = self.config.height as i32 / 2;
        let variant = if distance < 0x4000 {
            ((retail_tick / 20) & 1) as u8
        } else {
            0
        };
        let (x, y) = if distance == 0 {
            (center_x, center_y)
        } else {
            let sine_q31 = if distance < 0x4000 {
                i32::from(retail_sine_q15(distance as u32)).wrapping_mul(0x1_0001)
            } else {
                i32::MAX
            };
            let projected_x =
                q31_mul(sine_q31, dx_i32.wrapping_mul(self.config.width as i32) / 2) / distance;
            let projected_y =
                q31_mul(sine_q31, dz_i32.wrapping_mul(self.config.height as i32) / 2) / distance;
            (center_x + projected_x, center_y - projected_y)
        };

        // The high word of retail's allocation handle is not retained by
        // Entity yet. `variant` therefore uses only the proven process-time
        // half of the parity.

        let (palette_index, shape) =
            marker_style(capabilities, entity.collision.state_flags_at_0x08, variant)?;
        Some(Marker {
            x,
            y,
            color: rgb565_to_rgba(self.palette.markers[palette_index]),
            shape,
        })
    }
}

impl RadarPalette565 {
    fn from_cache(cache: &ResourceCache) -> Option<Self> {
        let base = usize::try_from(cache.level_desc()?.raw_u32(0x50)?).ok()?;
        let mut terrain = [0_u16; TERRAIN_PALETTE_LEN];
        for (index, color) in terrain.iter_mut().take(32).enumerate() {
            *color = radar_palette_rgb565(cache.global_palette_entry(base + index)?);
        }
        terrain[32] = radar_palette_rgb565(cache.global_palette_entry(31)?);
        terrain[33] = radar_palette_rgb565(cache.global_palette_entry(34)?);
        terrain[34] = radar_palette_rgb565(cache.global_palette_entry(33)?);

        let mut shade = [0_u16; SHADE_PALETTE_LEN];
        for (index, color) in shade.iter_mut().enumerate() {
            *color = radar_palette_rgb565(cache.global_palette_entry(base + 32 + index)?);
        }
        let marker_offsets = [24_usize, 25, 26, 28, 29, 30, 31];
        let mut markers = [0_u16; MARKER_PALETTE_LEN];
        for (index, color) in markers.iter_mut().enumerate() {
            *color =
                radar_palette_rgb565(cache.global_palette_entry(base + marker_offsets[index])?);
        }
        Some(Self {
            terrain,
            shade,
            markers,
        })
    }
}

impl TerrainRadarConfig {
    fn from_cache(cache: &ResourceCache) -> Result<Self, TerrainRadarError> {
        let scalar = |index| {
            cache
                .system_data_value(GAMEPLAY_HUD_SYSTEM_LEVEL, index)
                .ok_or(TerrainRadarError::ResourceUnavailable("radar scalar"))
        };
        let byte = |index| {
            u8::try_from(scalar(index)?)
                .map_err(|_| TerrainRadarError::InvalidConfig("radar byte scalar"))
        };
        Ok(Self {
            color_bands: byte(2)?,
            color_stride_minus_one: byte(3)?,
            coverage_max: byte(4)?,
            footprint_radius: i32::try_from(scalar(5)?)
                .map_err(|_| TerrainRadarError::InvalidConfig("coverage radius"))?,
            underwater_override: cache
                .level_desc()
                .and_then(|level| level.raw_u32(0x80))
                .ok_or(TerrainRadarError::ResourceUnavailable("radar sea policy"))?
                != 0,
        })
    }
}

impl TerrainRadar {
    pub(crate) fn view(&self) -> TerrainRadarView<'_> {
        TerrainRadarView {
            coverage: &self.coverage,
            indices: &self.indices,
            revision: self.revision,
        }
    }

    /// `2E570` calls `F390`, then `4AFB0 -> 4AB20(0)` after the authored
    /// constructors. Every covered cell consumes a process RNG word, including
    /// cells subsequently overwritten by the map grid. Display resources are
    /// independent: Intro2 performs this load even without a visible HUD.
    pub(crate) fn from_cache(
        cache: &ResourceCache,
        next_random: &mut impl FnMut() -> u16,
    ) -> Result<Self, TerrainRadarError> {
        let config = TerrainRadarConfig::from_cache(cache)?;
        let (grid, objects) = terrain_radar_resources(cache)?;
        let mut coverage = vec![0_u8; COVERAGE_BYTES];

        // Section 10 is X-major, unlike both radar buffers below.
        for x in 0..WORLD_SIZE {
            for z in 0..WORLD_SIZE {
                let cell = grid.cell(x, z).expect("validated complete terrain");
                if cell.attribute == 0 || cell.terrain_type & 0x18 != 0 {
                    continue;
                }
                let descriptor = objects.records.get(usize::from(cell.attribute)).ok_or(
                    TerrainRadarError::ResourceUnavailable("radar coverage object"),
                )?;
                if descriptor.kind_index == 8 {
                    stamp_coverage(
                        &mut coverage,
                        x,
                        z,
                        config.footprint_radius,
                        config.coverage_max,
                    );
                }
            }
        }

        let mut radar = Self {
            coverage,
            indices: vec![0_u8; WORLD_PIXELS],
            config,
            revision: 1,
        };
        for x in 0..WORLD_SIZE {
            for z in 0..WORLD_SIZE {
                radar.refresh_cell(
                    grid,
                    objects,
                    [(x << 8) as i16, (z << 8) as i16],
                    next_random,
                );
            }
        }
        Ok(radar)
    }

    /// `4A890` refreshes the raster only; coverage is retained. Preserve raw
    /// positions and input order, including duplicates, because each covered
    /// call consumes a fresh word. The material sample rounds by `+0x80` while
    /// object, height and layer remain in the unrounded cell (`4A5C0`).
    pub(crate) fn refresh_cells(
        &mut self,
        cache: &ResourceCache,
        positions: &[[i16; 2]],
        next_random: &mut impl FnMut() -> u16,
    ) -> Result<usize, TerrainRadarError> {
        let (grid, objects) = terrain_radar_resources(cache)?;
        let mut draws = 0;
        for &position in positions {
            draws += usize::from(self.refresh_cell(grid, objects, position, next_random));
        }
        if !positions.is_empty() {
            self.revision = self.revision.wrapping_add(1);
        }
        Ok(draws)
    }

    /// `44A8D0` changes packed coverage and immediately runs `44A5C0` for
    /// every cell in the square, including zero-contribution edge cells.
    /// Whole-byte carries/borrows and X-outer/Z-inner traversal are observable
    /// through both neighboring coverage and the shared random stream.
    pub(crate) fn mutate_coverage(
        &mut self,
        cache: &ResourceCache,
        position: [i16; 2],
        change: RadarCoverageChange,
        next_random: &mut impl FnMut() -> u16,
    ) -> Result<usize, TerrainRadarError> {
        let (grid, objects) = terrain_radar_resources(cache)?;
        let radius = self.config.footprint_radius;
        if radius <= 0 {
            return Err(TerrainRadarError::InvalidConfig("positive coverage radius"));
        }
        let [center_x, center_z] = position.map(|raw| i64::from((raw as u16) >> 8));
        let mut draws = 0;
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                let distance = integer_sqrt(
                    (i64::from(dx) * i64::from(dx) + i64::from(dz) * i64::from(dz)) as u64,
                )
                .min(radius as u64);
                let maximum = u64::from(self.config.coverage_max);
                let contribution = (maximum - maximum * distance / radius as u64) as u8;
                let x = (center_x + i64::from(dx)).rem_euclid(WORLD_SIZE as i64) as usize;
                let z = (center_z + i64::from(dz)).rem_euclid(WORLD_SIZE as i64) as usize;
                let linear = z * WORLD_SIZE + x;
                let contribution = if linear & 1 == 0 {
                    contribution
                } else {
                    contribution.wrapping_mul(16)
                };
                let packed = &mut self.coverage[linear >> 1];
                *packed = match change {
                    RadarCoverageChange::Add => packed.wrapping_add(contribution),
                    RadarCoverageChange::Remove => packed.wrapping_sub(contribution),
                };
                draws += usize::from(self.refresh_cell(
                    grid,
                    objects,
                    [(x << 8) as i16, (z << 8) as i16],
                    next_random,
                ));
            }
        }
        self.revision = self.revision.wrapping_add(1);
        Ok(draws)
    }

    fn refresh_cell(
        &mut self,
        grid: &TerrainGrid,
        objects: &TerrainObjectTable,
        position: [i16; 2],
        next_random: &mut impl FnMut() -> u16,
    ) -> bool {
        let [x, z] = position.map(|raw| usize::from((raw as u16) >> 8));
        let coverage = coverage_nibble(&self.coverage, x, z);
        let mut color_index = 32;
        if coverage != 0 {
            let cell = grid.cell(x, z).expect("validated complete terrain");
            let [material_x, material_z] =
                position.map(|raw| usize::from((raw.wrapping_add(0x80) as u16) >> 8));
            let material = grid
                .cell(material_x, material_z)
                .expect("validated complete terrain")
                .terrain_type
                & 7;
            let descriptor_kind = (cell.attribute != 0)
                .then(|| objects.records.get(usize::from(cell.attribute)))
                .flatten()
                .map(|descriptor| descriptor.kind_index);
            color_index = terrain_radar_color(
                self.config,
                *cell,
                material,
                descriptor_kind,
                grid.sea_level_raw(),
                coverage,
                next_random(),
            );
        }
        if is_world_map_grid_cell(x, z) {
            color_index = 33;
        }
        self.indices[z * WORLD_SIZE + x] = color_index.min(34);
        coverage != 0
    }
}

fn terrain_radar_resources(
    cache: &ResourceCache,
) -> Result<(&TerrainGrid, &TerrainObjectTable), TerrainRadarError> {
    let grid = cache
        .terrain()
        .ok_or(TerrainRadarError::ResourceUnavailable("radar terrain"))?;
    if grid.cells.len() != WORLD_PIXELS {
        return Err(TerrainRadarError::ResourceUnavailable(
            "complete radar terrain",
        ));
    }
    let objects = cache
        .terrain_objects()
        .ok_or(TerrainRadarError::ResourceUnavailable(
            "radar terrain objects",
        ))?;
    Ok((grid, objects))
}

fn terrain_radar_color(
    config: TerrainRadarConfig,
    cell: TerrainCell,
    material: u8,
    descriptor_kind: Option<u32>,
    sea_level_raw: i16,
    coverage: u8,
    random_word: u16,
) -> u8 {
    let layer = cell.terrain_type >> 5;
    let random_offset = layer / 2 + ((random_word as u8 & 1) & layer);
    let offset = random_offset.min(coverage);
    let band = config.color_bands.saturating_sub(offset).saturating_sub(1);
    let stride = config.color_stride_minus_one.saturating_add(1);
    // `4A7A3` jumps directly to the grid overlay: burned cells do not take
    // the later underwater override, even when their terrain height is low.
    if cell.terrain_type & 0x10 != 0 {
        return stride.saturating_mul(band).saturating_add(5);
    }
    if config.underwater_override && i16::from(cell.height as i8) * 32 < sea_level_raw {
        return stride.saturating_mul(band);
    }
    if cell.terrain_type & 0x08 == 0 && descriptor_kind == Some(8) {
        24
    } else {
        stride.saturating_mul(band).saturating_add(material)
    }
}

/// `FUN_0044A5C0` writes the authored 32-cell grid directly into the shared
/// terrain raster as palette index `0x21`. The first row is deliberately not a
/// complete horizontal line: only the already-covered vertical grid cells are
/// overwritten there.
fn is_world_map_grid_cell(x: usize, z: usize) -> bool {
    x % 32 == 0 || (z % 32 == 0 && x != 0 && z != 0)
}

fn stamp_coverage(coverage: &mut [u8], center_x: usize, center_z: usize, radius: i32, maximum: u8) {
    if radius <= 0 || maximum == 0 {
        return;
    }
    for dx in -radius..=radius {
        for dz in -radius..=radius {
            let distance = integer_sqrt(
                (i64::from(dx) * i64::from(dx) + i64::from(dz) * i64::from(dz)) as u64,
            )
            .min(radius as u64) as i32;
            if distance > radius {
                continue;
            }
            let contribution = i32::from(maximum) - (i32::from(maximum) * distance / radius);
            let x = (center_x as i32 + dx).rem_euclid(WORLD_SIZE as i32) as usize;
            let z = (center_z as i32 + dz).rem_euclid(WORLD_SIZE as i32) as usize;
            let linear = z * WORLD_SIZE + x;
            let byte = &mut coverage[linear >> 1];
            let add = if linear & 1 == 0 {
                contribution as u8
            } else {
                (contribution as u8).wrapping_mul(16)
            };
            *byte = byte.wrapping_add(add);
        }
    }
}

fn coverage_nibble(coverage: &[u8], x: usize, z: usize) -> u8 {
    let linear = z * WORLD_SIZE + x;
    let packed = coverage[linear >> 1];
    if linear & 1 == 0 {
        packed & 0x0f
    } else {
        packed >> 4
    }
}

fn project_globe(
    projection: &RadarProjectionTables,
    palette: &RadarPalette565,
    terrain: &[u8],
    player_raw: [i16; 3],
) -> RadarImage {
    let mut rgba = vec![0_u8; projection.width * projection.height * 4];
    let half_width = projection.half_width();
    let half_height = projection.half_height();
    for sy in 0..projection.height {
        let top = sy < half_height;
        let hy = if top { sy } else { projection.height - 1 - sy };
        for sx in 0..projection.width {
            let left = sx < half_width;
            let hx = if left { sx } else { projection.width - 1 - sx };
            let tuple_index = hy * half_width + hx;
            let tuple = projection.coordinate_tuples[tuple_index];
            let mask = projection.mask_grid[tuple_index] & 0x0f;
            if mask == 0x0f {
                continue;
            }
            let samples = tuple.samples.map(|sample| {
                let x = if left {
                    player_raw[0].wrapping_add(sample.x)
                } else {
                    player_raw[0].wrapping_sub(sample.x)
                };
                let z = if top {
                    player_raw[2].wrapping_add(sample.z)
                } else {
                    player_raw[2].wrapping_sub(sample.z)
                };
                let cell_x = usize::from((x as u16) >> 8);
                let cell_z = usize::from((z as u16) >> 8);
                palette.terrain[usize::from(terrain[cell_z * WORLD_SIZE + cell_x])]
            });
            let pixel = sy * projection.width + sx;
            let out = &mut rgba[pixel * 4..pixel * 4 + 4];
            if mask == 0 {
                let packed = packed_mean_four(samples).wrapping_add(
                    palette.shade
                        [usize::from(projection.shade_grid[pixel]).min(SHADE_PALETTE_LEN - 1)],
                );
                out.copy_from_slice(&rgb565_to_rgba(packed));
            } else {
                let mut count = 0_u32;
                let mut sum = [0_u32; 3];
                for (index, packed) in samples.into_iter().enumerate() {
                    if mask & (1 << index) == 0 {
                        let color = rgb565_to_rgba(packed);
                        sum[0] += u32::from(color[0]);
                        sum[1] += u32::from(color[1]);
                        sum[2] += u32::from(color[2]);
                        count += 1;
                    }
                }
                if count != 0 {
                    out[0] = (sum[0] / count) as u8;
                    out[1] = (sum[1] / count) as u8;
                    out[2] = (sum[2] / count) as u8;
                    out[3] = (count * 255 / 4) as u8;
                }
            }
        }
    }
    RadarImage {
        rgba,
        width: projection.width as u32,
        height: projection.height as u32,
    }
}

fn packed_mean_four(colors: [u16; 4]) -> u16 {
    let pair_a = colors[0].wrapping_add(colors[1]) >> 1 & PACKED_HALF_MASK_RGB565;
    let pair_b = colors[2].wrapping_add(colors[3]) >> 1 & PACKED_HALF_MASK_RGB565;
    pair_a.wrapping_add(pair_b) >> 1 & PACKED_HALF_MASK_RGB565
}

fn build_fullscreen_map(
    terrain: &[u8],
    palette: &[u16; TERRAIN_PALETTE_LEN],
    rect: RadarRect,
    bilinear: bool,
) -> RadarImage {
    let width = rect.width as usize;
    let height = rect.height as usize;
    let x_step = 0x00fe_0000_i64 / (i64::from(rect.width) - 1);
    let z_step = -0x00fe_0000_i64 / (i64::from(rect.height) - 1);
    let mut rgba = vec![0_u8; width * height * 4];
    for y in 0..height {
        let z_q16 = 0x00fe_8000_i64 + z_step * y as i64;
        for x in 0..width {
            let x_q16 = 0x0000_8000_i64 + x_step * x as i64;
            let color = if bilinear {
                bilinear_raster_color(terrain, palette, x_q16, z_q16)
            } else {
                let cell_x = ((x_q16 >> 16) as usize).min(WORLD_SIZE - 1);
                let cell_z = ((z_q16 >> 16) as usize).min(WORLD_SIZE - 1);
                rgb565_to_rgba(palette[usize::from(terrain[cell_z * WORLD_SIZE + cell_x])])
            };
            rgba[(y * width + x) * 4..(y * width + x) * 4 + 4].copy_from_slice(&color);
        }
    }
    RadarImage {
        rgba,
        width: rect.width,
        height: rect.height,
    }
}

fn bilinear_raster_color(
    terrain: &[u8],
    palette: &[u16; TERRAIN_PALETTE_LEN],
    x_q16: i64,
    z_q16: i64,
) -> [u8; 4] {
    let x0 = ((x_q16 >> 16) as usize).min(WORLD_SIZE - 1);
    let z0 = ((z_q16 >> 16) as usize).min(WORLD_SIZE - 1);
    let x1 = (x0 + 1).min(WORLD_SIZE - 1);
    let z1 = (z0 + 1).min(WORLD_SIZE - 1);
    let fx = (x_q16 & 0xffff) as u64;
    let fz = (z_q16 & 0xffff) as u64;
    let colors = [
        rgb565_to_rgba(palette[usize::from(terrain[z0 * WORLD_SIZE + x0])]),
        rgb565_to_rgba(palette[usize::from(terrain[z0 * WORLD_SIZE + x1])]),
        rgb565_to_rgba(palette[usize::from(terrain[z1 * WORLD_SIZE + x0])]),
        rgb565_to_rgba(palette[usize::from(terrain[z1 * WORLD_SIZE + x1])]),
    ];
    let mut out = [0_u8; 4];
    for channel in 0..3 {
        let top =
            u64::from(colors[0][channel]) * (0x1_0000 - fx) + u64::from(colors[1][channel]) * fx;
        let bottom =
            u64::from(colors[2][channel]) * (0x1_0000 - fx) + u64::from(colors[3][channel]) * fx;
        out[channel] = ((top * (0x1_0000 - fz) + bottom * fz) >> 32) as u8;
    }
    out[3] = 255;
    out
}

fn decode_map_icons(cache: &ResourceCache) -> HashMap<u32, DecodedMapIcon> {
    let mut icons = HashMap::new();
    for entity_type in 0..cache.global_entity_model_table().len() {
        let Some(sprite_id) = cache
            .global_entity_type(entity_type)
            .and_then(|record| record.fullscreen_map_icon_sprite_id())
        else {
            continue;
        };
        let Some((atlas, entry)) = cache.global_sprite(sprite_id) else {
            continue;
        };
        let flags = entry.pal_size as u8;
        let Ok(decoded) = atlas.decode_sprite(
            entry,
            usize::from(crate::model_color::sprite_flat_shade_row(flags)),
        ) else {
            continue;
        };
        icons.insert(
            entity_type as u32,
            DecodedMapIcon {
                rgba: decoded.rgba,
                width: decoded.width as u32,
                height: decoded.height as u32,
            },
        );
    }
    icons
}

#[derive(Debug, Clone, Copy)]
struct Marker {
    x: i32,
    y: i32,
    color: [u8; 4],
    shape: u8,
}

fn marker_style(capabilities: u32, state: RetailStateWord, variant: u8) -> Option<(usize, u8)> {
    if capabilities & 0x0c00 != 0 {
        return Some((6, variant));
    }
    if capabilities & 1 != 0 {
        return Some((0, variant));
    }
    if capabilities & 0x00a0 != 0 {
        return Some((3, variant));
    }
    if capabilities & 0x0200 != 0 {
        return Some((2, variant));
    }
    if capabilities & 4 != 0 {
        return Some((5, variant));
    }
    if capabilities & 0x10 != 0 {
        let inactive = match state.masked(0x4000) {
            RetailRuntimeValue::Known(value) if value != 0 => true,
            RetailRuntimeValue::Known(0) => state_is_exact_zero(state),
            RetailRuntimeValue::Unresolved => return None,
            RetailRuntimeValue::Known(_) => unreachable!(),
        };
        if inactive {
            return (variant != 0).then_some((4, 3));
        }
        return Some((4, variant));
    }
    if capabilities & 8 != 0 {
        return Some((1, variant));
    }
    (capabilities & 0x8000 != 0).then_some((5, 3))
}

fn plot_marker(image: &mut RadarImage, marker: Marker) {
    if marker.shape & 2 == 0 {
        blend_image_pixel(image, marker.x, marker.y, marker.color, 255);
    }
    if marker.shape & 1 == 0 {
        return;
    }
    for (dx, dy, alpha) in [
        (1, 0, 128),
        (-1, 0, 128),
        (0, 1, 128),
        (0, -1, 128),
        (1, 1, 64),
        (1, -1, 64),
        (-1, 1, 64),
        (-1, -1, 64),
    ] {
        blend_image_pixel(image, marker.x + dx, marker.y + dy, marker.color, alpha);
    }
}

fn blend_image_pixel(image: &mut RadarImage, x: i32, y: i32, color: [u8; 4], alpha: u8) {
    if x < 0 || y < 0 || x >= image.width as i32 || y >= image.height as i32 {
        return;
    }
    let offset = (y as usize * image.width as usize + x as usize) * 4;
    let source_alpha = u32::from(alpha);
    let destination_alpha = u32::from(image.rgba[offset + 3]);
    let out_alpha = source_alpha + destination_alpha * (255 - source_alpha) / 255;
    if out_alpha == 0 {
        return;
    }
    for channel in 0..3 {
        let source = u32::from(color[channel]) * source_alpha;
        let destination =
            u32::from(image.rgba[offset + channel]) * destination_alpha * (255 - source_alpha)
                / 255;
        image.rgba[offset + channel] = ((source + destination) / out_alpha) as u8;
    }
    image.rgba[offset + 3] = out_alpha as u8;
}

fn map_state_is_visible(state: RetailStateWord) -> bool {
    if !state_proves_nonzero(state) {
        return false;
    }
    matches!(state.masked(0x5000), RetailRuntimeValue::Known(0))
}

fn state_is_exact_zero(state: RetailStateWord) -> bool {
    state.known_mask() == u32::MAX && state.known_value_bits() == 0
}

fn state_proves_nonzero(state: RetailStateWord) -> bool {
    state.known_value_bits() != 0
}

fn radar_palette_rgb565(entry: &PaletteEntry) -> u16 {
    // Section 7 stores RGB555 on disk. Retail expands those channels into the
    // RGB565 framebuffer table captured at DAT_004FE63C by shifting the red
    // and green lanes one place. It does not replicate green's high bit into
    // the new low bit: disk 0x7fff deliberately becomes 0xffdf, not 0xffff.
    ((entry.rgb555 & 0x7c00) << 1) | ((entry.rgb555 & 0x03e0) << 1) | (entry.rgb555 & 0x001f)
}

fn rgb565_to_rgba(color: u16) -> [u8; 4] {
    let red = ((color >> 11) & 0x1f) as u8;
    let green = ((color >> 5) & 0x3f) as u8;
    let blue = (color & 0x1f) as u8;
    [
        (red << 3) | (red >> 2),
        (green << 2) | (green >> 4),
        (blue << 3) | (blue >> 2),
        255,
    ]
}

fn q31_mul(lhs: i32, rhs: i32) -> i32 {
    ((i64::from(lhs) * i64::from(rhs)) >> 31) as i32
}

fn integer_sqrt(value: u64) -> u64 {
    if value == 0 {
        return 0;
    }
    let mut remainder = value;
    let mut root = 0_u64;
    let mut bit = 1_u64 << 62;
    while bit > remainder {
        bit >>= 2;
    }
    while bit != 0 {
        if remainder >= root + bit {
            remainder -= root + bit;
            root = (root >> 1) + bit;
        } else {
            root >>= 1;
        }
        bit >>= 2;
    }
    root
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::retail_rng::retail_random_u16;

    fn terrain_session(level: u32) -> crate::session::GameSession {
        let data = v2k_test_support::retail_dir();
        assert!(
            data.join("PRELOAD.DAT").is_file(),
            "canonical retail corpus required"
        );
        let mut session = crate::session::GameSession::init(&data).expect("PRELOAD");
        session
            .load_auxiliary_ovl(3, 1)
            .expect("normal system radar");
        session.load_level_by_id(level, 1).expect("normal world");
        session
    }

    fn raster_config() -> TerrainRadarConfig {
        TerrainRadarConfig {
            color_bands: 4,
            color_stride_minus_one: 5,
            coverage_max: 3,
            footprint_radius: 50,
            underwater_override: true,
        }
    }

    fn portable_coverage_fixture(
        radius: i32,
        maximum: u8,
        packed_coverage: u8,
    ) -> (crate::session::GameSession, TerrainRadar) {
        let mut session = terrain_session(30);
        session
            .cache
            .level_terrain_mut()
            .unwrap()
            .cells
            .fill(TerrainCell {
                height: 127,
                attribute: 0,
                terrain_type: 0x21,
            });
        let radar = TerrainRadar {
            config: TerrainRadarConfig {
                footprint_radius: radius,
                coverage_max: maximum,
                ..raster_config()
            },
            revision: 1,
            coverage: vec![packed_coverage; COVERAGE_BYTES],
            indices: vec![34; WORLD_PIXELS],
        };
        (session, radar)
    }

    #[v2k_test_support::retail_test]
    fn portable_coverage_carries_and_borrows_across_whole_packed_bytes() {
        for (x, initial, added, add_draws) in [(10, 0x0f, 0x10, 1), (11, 0xf0, 0x00, 0)] {
            let (session, mut radar) = portable_coverage_fixture(1, 1, 0);
            let packed_index = (10 * WORLD_SIZE + x) >> 1;
            radar.coverage[packed_index] = initial;
            let position = [(x << 8) as i16, 10 << 8];
            assert_eq!(
                radar.mutate_coverage(
                    &session.cache,
                    position,
                    RadarCoverageChange::Add,
                    &mut || 0,
                ),
                Ok(add_draws)
            );
            assert_eq!(radar.coverage[packed_index], added);
            assert_eq!(radar.indices[10 * WORLD_SIZE + x], 32);
            if x == 10 {
                assert_eq!(
                    radar.indices[10 * WORLD_SIZE + 11],
                    19,
                    "the later odd-X refresh sees the low-nibble carry"
                );
            }
            assert_eq!(
                radar.mutate_coverage(
                    &session.cache,
                    position,
                    RadarCoverageChange::Remove,
                    &mut || 0,
                ),
                Ok(1)
            );
            assert_eq!(radar.coverage[packed_index], initial);
            assert_eq!(radar.indices[10 * WORLD_SIZE + x], 19);
            if x == 10 {
                assert_eq!(radar.indices[10 * WORLD_SIZE + 11], 32);
            }
            assert_eq!(radar.revision, 3);
        }
    }

    #[v2k_test_support::retail_test]
    fn portable_coverage_refreshes_zero_contributions_in_x_outer_z_inner_rng_order() {
        // Radius one only changes the center. All eight zero-contribution
        // neighbors still consume words because the existing map covers them.
        for maximum in [0, 1] {
            let (session, mut radar) = portable_coverage_fixture(1, maximum, 0x11);
            let mut words = [0, 0, 1, 1, 0, 0, 1, 1, 0].into_iter();
            assert_eq!(
                radar.mutate_coverage(
                    &session.cache,
                    [10 << 8, 10 << 8],
                    RadarCoverageChange::Add,
                    &mut || words.next().expect("exact nine covered-cell draws"),
                ),
                Ok(9)
            );
            assert!(words.next().is_none());
            for (x, z, expected) in [
                (9, 9, 19),
                (9, 10, 19),
                (9, 11, 13),
                (10, 9, 13),
                (10, 10, 19),
                (10, 11, 19),
                (11, 9, 13),
                (11, 10, 13),
                (11, 11, 19),
            ] {
                assert_eq!(radar.indices[z * WORLD_SIZE + x], expected, "{x},{z}");
            }
            assert_eq!(radar.indices[8 * WORLD_SIZE + 9], 34);
            assert_eq!(radar.indices[12 * WORLD_SIZE + 9], 34);
        }
    }

    #[v2k_test_support::retail_test]
    fn portable_coverage_wraps_world_and_refreshes_cell_aligned_material_including_grid() {
        let (mut session, mut radar) = portable_coverage_fixture(1, 1, 0x11);
        // The original raw position would round its material sample to 0,0.
        // 44A8D0 instead forwards each visited cell's aligned raw coordinate.
        session.cache.level_terrain_mut().unwrap().cells[0].terrain_type = 6;
        let mut draws = 0;
        assert_eq!(
            radar.mutate_coverage(
                &session.cache,
                [0xff80_u16 as i16, 0xff80_u16 as i16],
                RadarCoverageChange::Add,
                &mut || {
                    draws += 1;
                    0
                },
            ),
            Ok(9)
        );
        assert_eq!(draws, 9, "grid cells draw before their color is replaced");
        assert_eq!(coverage_nibble(&radar.coverage, 255, 255), 2);
        assert_eq!(radar.indices[255 * WORLD_SIZE + 255], 19);
        assert_eq!(radar.indices[0 * WORLD_SIZE + 255], 19);
        assert_eq!(radar.indices[255 * WORLD_SIZE + 0], 33);
        assert_eq!(radar.indices[0], 33);
        assert_eq!(radar.indices[1 * WORLD_SIZE + 255], 34);
    }

    #[v2k_test_support::retail_test]
    fn portable_coverage_overlaps_and_removes_the_current_position_footprint() {
        let (session, mut radar) = portable_coverage_fixture(2, 3, 0);
        let empty = radar.coverage.clone();
        let position = [10 << 8, 10 << 8];
        radar
            .mutate_coverage(
                &session.cache,
                position,
                RadarCoverageChange::Add,
                &mut || 0,
            )
            .unwrap();
        let once = radar.coverage.clone();
        assert_eq!(coverage_nibble(&once, 10, 10), 3);
        assert_eq!(coverage_nibble(&once, 9, 9), 2);
        assert_eq!(coverage_nibble(&once, 8, 10), 0);
        radar
            .mutate_coverage(
                &session.cache,
                position,
                RadarCoverageChange::Add,
                &mut || 0,
            )
            .unwrap();
        assert_eq!(coverage_nibble(&radar.coverage, 10, 10), 6);
        assert_eq!(coverage_nibble(&radar.coverage, 9, 9), 4);
        radar
            .mutate_coverage(
                &session.cache,
                position,
                RadarCoverageChange::Remove,
                &mut || 0,
            )
            .unwrap();
        assert_eq!(radar.coverage, once);
        radar
            .mutate_coverage(
                &session.cache,
                position,
                RadarCoverageChange::Remove,
                &mut || {
                    panic!("removing the sole small footprint leaves every visited cell hidden")
                },
            )
            .unwrap();
        assert_eq!(radar.coverage, empty);
    }

    #[v2k_test_support::retail_test]
    fn portable_coverage_rejects_missing_resources_or_zero_radius_before_mutation() {
        let (session, mut radar) = portable_coverage_fixture(0, 3, 0x11);
        let before = radar.clone();
        for cache in [&ResourceCache::new(vec![]), &session.cache] {
            assert!(radar
                .mutate_coverage(cache, [0, 0], RadarCoverageChange::Add, &mut || panic!(
                    "invalid coverage operation drew RNG"
                ))
                .is_err());
            assert_eq!(radar.coverage, before.coverage);
            assert_eq!(radar.indices, before.indices);
            assert_eq!(radar.revision, before.revision);
        }
    }

    fn presentation_fixture() -> (crate::session::GameSession, GameplayRadar) {
        let mut session = terrain_session(13);
        session
            .cache
            .initialize_level_terrain_radar(&mut || 0)
            .unwrap();
        let presenter = GameplayRadar::from_cache(&session.cache, 1).unwrap();
        (session, presenter)
    }

    #[v2k_test_support::retail_test]
    fn hud_marker_preserves_state_coverage_random_then_capability_gate_order() {
        let (_session, presenter) = presentation_fixture();
        let player =
            Entity::unresolved_port_entity(1, crate::entity::EntityKind::from_type(46), 46);
        let indices = vec![0; WORLD_PIXELS];
        for (entity_type, capability, state, covered, word, expected_draws, visible) in [
            (2, 4, RetailStateWord::exact(0x80), false, 0, 1, true),
            (2, 4, RetailStateWord::exact(0x80), false, 1, 1, false),
            (2, 4, RetailStateWord::exact(0), false, 0, 0, false),
            (2, 4, RetailStateWord::unknown(), false, 0, 0, false),
            (2, 4, RetailStateWord::exact(0x4080), false, 0, 0, false),
            (2, 4, RetailStateWord::exact(0x1080), false, 0, 1, true),
            (2, 2, RetailStateWord::exact(0x80), false, 0, 1, false),
            (2, 4, RetailStateWord::exact(0x80), true, 1, 0, true),
            (46, 1, RetailStateWord::exact(0x80), false, 1, 0, true),
            (2, 0x10, RetailStateWord::exact(0x4000), false, 0, 1, true),
            (2, 0x10, RetailStateWord::unknown(), false, 0, 1, false),
        ] {
            let coverage = vec![if covered { 0x11 } else { 0 }; COVERAGE_BYTES];
            let terrain = TerrainRadarView {
                coverage: &coverage,
                indices: &indices,
                revision: 1,
            };
            let mut entity = Entity::unresolved_port_entity(
                2,
                crate::entity::EntityKind::from_type(entity_type),
                entity_type,
            );
            entity.capability_flags = capability;
            entity.collision.state_flags_at_0x08 = state;
            let mut draws = 0;
            let marker = presenter.hud_marker(terrain, &player, &entity, 20, &mut || {
                draws += 1;
                word
            });
            assert_eq!(
                draws, expected_draws,
                "type={entity_type} cap={capability:#x} state={state:?}"
            );
            assert_eq!(
                marker.is_some(),
                visible,
                "type={entity_type} cap={capability:#x} state={state:?}"
            );
        }
    }

    #[v2k_test_support::retail_test]
    fn fullscreen_markers_preserve_icon_state_coverage_random_and_modal_gates() {
        let (session, mut presenter) = presentation_fixture();
        let mut entities = EntityManager::from_level(
            session.cache.level_desc().unwrap(),
            &session.cache.global_entity_model_table(),
            session.cache.terrain(),
        );
        let ids = entities.iter().map(|entity| entity.id).collect::<Vec<_>>();
        for &id in &ids {
            entities
                .entity_mut(id)
                .unwrap()
                .collision
                .state_flags_at_0x08 = RetailStateWord::exact(0);
        }
        let id = ids[0];
        let indices = vec![0; WORLD_PIXELS];
        presenter.enter_fullscreen();
        for _ in 0..3 {
            presenter.advance_fullscreen(0);
        }
        for (entity_type, state, covered, word, expected_draws, visible) in [
            (2, RetailStateWord::exact(0x80), false, 0, 1, true),
            (2, RetailStateWord::exact(0x80), false, 1, 1, false),
            (54, RetailStateWord::exact(0x80), false, 0, 0, false),
            (2, RetailStateWord::exact(0), false, 0, 0, false),
            (2, RetailStateWord::unknown(), false, 0, 0, false),
            (2, RetailStateWord::exact(0x4080), false, 0, 0, false),
            (2, RetailStateWord::exact(0x1080), false, 0, 0, false),
            (2, RetailStateWord::exact(0x80), true, 1, 0, true),
            (46, RetailStateWord::exact(0x80), false, 1, 0, true),
        ] {
            let entity = entities.entity_mut(id).unwrap();
            entity.entity_type = entity_type;
            entity.collision.state_flags_at_0x08 = state;
            let coverage = vec![if covered { 0x11 } else { 0 }; COVERAGE_BYTES];
            let terrain = TerrainRadarView {
                coverage: &coverage,
                indices: &indices,
                revision: 1,
            };
            let mut draws = 0;
            let placements = presenter.fullscreen_icon_placements(terrain, &entities, &mut || {
                draws += 1;
                word
            });
            assert_eq!(draws, expected_draws, "type={entity_type} state={state:?}");
            assert_eq!(
                placements.len(),
                usize::from(visible),
                "type={entity_type} state={state:?}"
            );
        }
        let entity = entities.entity_mut(id).unwrap();
        entity.entity_type = 2;
        let terrain = session.cache.level_terrain_radar().unwrap();
        presenter.leave_fullscreen();
        assert!(presenter
            .fullscreen_icon_placements(terrain, &entities, &mut || panic!("closed map drew RNG"))
            .is_empty());
        presenter.enter_fullscreen();
        for _ in 0..3 {
            assert!(presenter
                .fullscreen_icon_placements(terrain, &entities, &mut || panic!(
                    "setup map drew RNG"
                ))
                .is_empty());
            presenter.advance_fullscreen(0);
        }
        presenter.advance_fullscreen(0);
        presenter.advance_fullscreen(MAP_BLINK_INTERVAL_US * 2 + 1);
        assert_eq!(presenter.lifecycle.blink_phase, 0);
        assert!(presenter
            .fullscreen_icon_placements(terrain, &entities, &mut || panic!("blank blink drew RNG"))
            .is_empty());
    }

    #[v2k_test_support::retail_test]
    fn fullscreen_raster_retains_setup_sampling_and_cached_images_until_refresh() {
        let (session, mut presenter) = presentation_fixture();
        let terrain = session.cache.level_terrain_radar().unwrap();
        assert!(presenter.fullscreen_image(terrain).is_none());
        presenter.enter_fullscreen();
        for _ in 0..3 {
            assert!(presenter.fullscreen_image(terrain).is_none());
            assert!(presenter.fullscreen_raster.is_none());
            presenter.advance_fullscreen(0);
        }
        let nearest = build_fullscreen_map(
            terrain.indices,
            &presenter.palette.terrain,
            presenter.config.map_rect,
            false,
        );
        assert_eq!(presenter.fullscreen_image(terrain), Some(&nearest));
        let cached_pointer = presenter.fullscreen_image(terrain).unwrap().rgba.as_ptr();
        assert_eq!(
            presenter.fullscreen_image(terrain).unwrap().rgba.as_ptr(),
            cached_pointer
        );
        presenter.advance_fullscreen(0);
        let bilinear = build_fullscreen_map(
            terrain.indices,
            &presenter.palette.terrain,
            presenter.config.map_rect,
            true,
        );
        assert_eq!(presenter.fullscreen_image(terrain), Some(&bilinear));
        assert_eq!(
            presenter.fullscreen_raster.as_ref().unwrap().revision,
            terrain.revision()
        );
        presenter.leave_fullscreen();
        presenter.enter_fullscreen();
        for _ in 0..3 {
            presenter.advance_fullscreen(0);
        }
        assert_eq!(
            presenter.fullscreen_image(terrain).unwrap().rgba.as_ptr(),
            cached_pointer,
            "resume retains derived raster without reconstruction"
        );
    }

    #[v2k_test_support::retail_test]
    fn terrain_radar_native_load_consumes_one_word_per_covered_cell_and_presentation_borrows_it() {
        for level in [13, 50] {
            let mut session = terrain_session(level);
            let mut words = 0;
            let mut state = 0;
            let radar = TerrainRadar::from_cache(&session.cache, &mut || {
                words += 1;
                retail_random_u16(&mut state)
            })
            .expect("native terrain radar");
            let covered = (0..WORLD_SIZE)
                .flat_map(|z| (0..WORLD_SIZE).map(move |x| [x, z]))
                .filter(|&[x, z]| coverage_nibble(&radar.coverage, x, z) != 0)
                .count();
            assert!(covered > 0);
            assert_eq!(words, covered);
            assert!(GameplayRadar::from_cache(&session.cache, 1).is_none());
            let mut shared_state = 0;
            assert_eq!(
                session
                    .cache
                    .initialize_level_terrain_radar(&mut || retail_random_u16(&mut shared_state)),
                Ok(covered)
            );
            let _gui = GameplayRadar::from_cache(&session.cache, 1).expect("authored GUI");
            let shared = session.cache.level_terrain_radar().unwrap();
            assert_eq!(radar.coverage, shared.packed_coverage());
            assert_eq!(radar.indices, shared.indices());
            assert_eq!(state, shared_state);
            assert_eq!(
                session.cache.initialize_level_terrain_radar(&mut || panic!(
                    "presentation repeated raster RNG"
                )),
                Ok(0)
            );
        }
    }

    #[v2k_test_support::retail_test]
    fn terrain_radar_refresh_retains_coverage_and_consumes_duplicate_calls_in_order() {
        let mut session = terrain_session(50);
        let mut radar = TerrainRadar::from_cache(&session.cache, &mut || 0).unwrap();
        let [x, z] = (1..WORLD_SIZE)
            .flat_map(|x| (1..WORLD_SIZE).map(move |z| [x, z]))
            .find(|&[x, z]| {
                !is_world_map_grid_cell(x, z) && coverage_nibble(&radar.coverage, x, z) >= 1
            })
            .expect("covered nongrid cell");
        let coverage = radar.coverage.clone();
        let before = radar.indices.clone();
        let cell = &mut session.cache.level_terrain_mut().unwrap().cells[x * WORLD_SIZE + z];
        *cell = TerrainCell {
            height: 127,
            attribute: 0,
            terrain_type: 0x21,
        };
        let position = [(x << 8) as i16, (z << 8) as i16];
        let mut words = [0, 1].into_iter();
        assert_eq!(
            radar.refresh_cells(&session.cache, &[position, position], &mut || words
                .next()
                .expect("exact two calls")),
            Ok(2)
        );
        assert!(words.next().is_none());
        assert_eq!(radar.coverage, coverage);
        assert_eq!(
            radar.indices[z * WORLD_SIZE + x],
            13,
            "second word chooses band two plus material one"
        );
        for (index, (&old, &new)) in before.iter().zip(&radar.indices).enumerate() {
            if index != z * WORLD_SIZE + x {
                assert_eq!(old, new);
            }
        }
    }

    #[test]
    fn terrain_radar_refresh_uses_raw_material_rounding_and_grid_draws() {
        let mut grid = TerrainGrid {
            header: [0; 5],
            cells: vec![
                TerrainCell {
                    height: 1,
                    attribute: 0,
                    terrain_type: 1
                };
                WORLD_PIXELS
            ],
        };
        grid.cells[0 * WORLD_SIZE + 2].terrain_type = 6;
        let objects = TerrainObjectTable { records: vec![] };
        let mut radar = TerrainRadar {
            config: raster_config(),
            revision: 1,
            coverage: vec![0x11; COVERAGE_BYTES],
            indices: vec![0; WORLD_PIXELS],
        };
        let mut draws = 0;
        let mut random = || {
            draws += 1;
            0
        };
        assert!(radar.refresh_cell(&grid, &objects, [0xff80_u16 as i16, 0x180], &mut random));
        assert_eq!(
            radar.indices[1 * WORLD_SIZE + 255],
            24,
            "material wraps to cell0,2; raster stays255,1"
        );
        assert!(radar.refresh_cell(&grid, &objects, [0, 0x100], &mut random));
        assert_eq!(
            radar.indices[WORLD_SIZE], 33,
            "grid replaces color after consuming word"
        );
        radar.coverage[WORLD_SIZE / 2] = 0;
        assert!(!radar.refresh_cell(&grid, &objects, [0, 0x100], &mut random));
        assert_eq!(radar.indices[WORLD_SIZE], 33);
        assert_eq!(draws, 2, "uncovered grid cell consumes no word");
    }

    #[test]
    fn terrain_radar_burned_color_bypasses_underwater_override() {
        let submerged = TerrainCell {
            height: 0xff,
            attribute: 1,
            terrain_type: 0x11,
        };
        assert_eq!(
            terrain_radar_color(raster_config(), submerged, 1, Some(8), 0, 3, 0),
            23
        );
        assert_eq!(
            terrain_radar_color(
                raster_config(),
                TerrainCell {
                    terrain_type: 1,
                    ..submerged
                },
                1,
                Some(8),
                0,
                3,
                0
            ),
            18
        );
        assert_eq!(
            terrain_radar_color(
                raster_config(),
                TerrainCell {
                    height: 1,
                    terrain_type: 1,
                    ..submerged
                },
                1,
                Some(8),
                0,
                3,
                0
            ),
            24
        );
    }

    #[test]
    fn terrain_radar_missing_refresh_resources_preserve_raster_and_random_stream() {
        let mut radar = TerrainRadar {
            config: raster_config(),
            revision: 1,
            coverage: vec![0x11; COVERAGE_BYTES],
            indices: vec![19; WORLD_PIXELS],
        };
        let cache = ResourceCache::new(vec![]);
        let before = radar.clone();
        let mut random = || panic!("missing dependencies precede RNG");
        assert!(TerrainRadar::from_cache(&cache, &mut random).is_err());
        assert!(radar.refresh_cells(&cache, &[[0, 0]], &mut random).is_err());
        assert_eq!(radar.coverage, before.coverage);
        assert_eq!(radar.indices, before.indices);
        assert_eq!(radar.revision, before.revision);
    }

    #[test]
    fn packed_mean_matches_retail_two_stage_masking() {
        assert_eq!(packed_mean_four([0xffff; 4]), 0x7bef);
        assert_eq!(packed_mean_four([0, 0, 0, 0]), 0);
        assert_eq!(
            packed_mean_four([0xf800, 0x07e0, 0x001f, 0xffff]),
            (((0xf800_u16.wrapping_add(0x07e0) >> 1) & 0x7bef)
                .wrapping_add((0x001f_u16.wrapping_add(0xffff) >> 1) & 0x7bef)
                >> 1)
                & 0x7bef
        );
    }

    #[test]
    fn radar_palette_matches_the_captured_rgb555_to_rgb565_expansion() {
        let entry = PaletteEntry {
            rgb555: 0x08cc,
            r: 16,
            g: 49,
            b: 98,
        };
        assert_eq!(radar_palette_rgb565(&entry), 0x118c);

        let white = PaletteEntry {
            rgb555: 0x7fff,
            r: 255,
            g: 255,
            b: 255,
        };
        assert_eq!(radar_palette_rgb565(&white), 0xffdf);
    }

    #[test]
    fn packed_coverage_uses_z_major_alternating_nibbles() {
        let mut coverage = vec![0_u8; COVERAGE_BYTES];
        coverage[0] = 0xa3;
        coverage[WORLD_SIZE / 2] = 0x74;
        assert_eq!(coverage_nibble(&coverage, 0, 0), 3);
        assert_eq!(coverage_nibble(&coverage, 1, 0), 10);
        assert_eq!(coverage_nibble(&coverage, 0, 1), 4);
        assert_eq!(coverage_nibble(&coverage, 1, 1), 7);
    }

    #[test]
    fn world_map_grid_matches_the_retail_asymmetric_origin_rule() {
        for z in 0..WORLD_SIZE {
            assert!(is_world_map_grid_cell(0, z));
            assert!(is_world_map_grid_cell(32, z));
        }

        assert!(!is_world_map_grid_cell(1, 0));
        assert!(!is_world_map_grid_cell(31, 0));
        assert!(!is_world_map_grid_cell(255, 0));
        assert!(!is_world_map_grid_cell(31, 31));

        assert!(is_world_map_grid_cell(1, 32));
        assert!(is_world_map_grid_cell(31, 32));
        assert!(is_world_map_grid_cell(255, 32));
        assert!(is_world_map_grid_cell(255, 224));
    }

    #[test]
    fn map_position_is_fixed_north_up_and_inverts_z() {
        let rect = RadarRect {
            x: 2,
            y: 7,
            width: 464,
            height: 464,
        };
        let project = |x: u32, z: u32| {
            (
                rect.x + ((x * rect.width) / 256) as i32,
                rect.y + (((256 - z) * rect.height) / 256) as i32,
            )
        };
        assert_eq!(project(0, 255), (2, 8));
        assert_eq!(project(255, 0), (464, 471));
        assert!(project(200, 100).0 > project(100, 100).0);
        assert!(project(100, 200).1 < project(100, 100).1);
    }

    #[test]
    fn partial_state_zero_is_not_treated_as_exact_zero() {
        let partial = RetailStateWord::from_known_bits(0, 0x5000);
        assert!(!state_is_exact_zero(partial));
        assert!(!state_proves_nonzero(partial));
        assert!(!map_state_is_visible(partial));

        let visible = RetailStateWord::from_known_bits(0x80, 0x5080);
        assert!(state_proves_nonzero(visible));
        assert!(map_state_is_visible(visible));
    }

    #[test]
    fn marker_shape_bits_match_center_and_halo_contract() {
        let mut image = RadarImage {
            rgba: vec![0; 3 * 3 * 4],
            width: 3,
            height: 3,
        };
        plot_marker(
            &mut image,
            Marker {
                x: 1,
                y: 1,
                color: [255, 0, 0, 255],
                shape: 3,
            },
        );
        assert_eq!(image.rgba[(1 * 3 + 1) * 4 + 3], 0);
        assert_eq!(image.rgba[(1 * 3 + 2) * 4 + 3], 128);
        assert_eq!(image.rgba[3], 64);
    }
}
