//! Interactive, no-argument asset workbench.
//!
//! Models and sprites are resolved through the same [`GameSession`] resource
//! cache and OpenGL submission path as the game. Section-10 map images remain
//! deliberately raw diagnostic views; they are not reconstructions of the
//! retail world renderer.

mod input;
mod maps;
mod ui;

use super::ViewerMode;
use input::{
    parse_catalog_position, CatalogPositionError, NavigationDirection, NavigationRepeat,
    TextInputState,
};
use maps::{MapAsset, MapLayer, MAP_SIZE};
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::{Keycode, Mod};
use sdl2::mouse::{MouseButton, MouseWheelDirection};
use sdl2::rect::{Point, Rect};
use std::path::{Path, PathBuf};
use std::time::Instant;
use ui::{fit_rect, ToolbarTab, ToolbarView, UiAction, UiRects, UiSurface, TOOLBAR_HEIGHT};
use v2k_formats::models::{AnimVars, ModelEntry};
use v2k_formats::sprites::{DecodedSprite, SpriteEntry};
use v2k_game::model_color::ModelMaterialCache;
use v2k_game::model_tree::{
    linked_model_bounds, ModelSceneLight, ModelTreeBounds, ModelTreeRenderer, ModelTreeStyle,
};
use v2k_game::session::{world_resource_level, GameSession};
use v2k_render::gl_backend::GlRenderer;
use v2k_render::{
    apply_app_window_icon, orientation_from_ypr, Camera, RenderScene, Renderer, ViewPinMode,
};

const BACKGROUND: [u8; 4] = [18, 20, 28, 255];
const PREVIEW_MODEL_SCALE: f32 = 1.0;
// Levels 2 and 5 are embedded in PRELOAD.DAT, but those resident copies carry
// the low-tier atlases. Overlay the selected disk tier as well so newest-first
// sprite lookup cannot mix low menu/font art into a high-tier workbench.
const GLOBAL_ASSET_LEVELS: [u32; 10] = [2, 3, 5, 6, 7, 8, 9, 10, 11, 51];
/// Variant 0 is the authored 320x240 presentation tier, not a generic common
/// pool. The workbench defaults to retail's first high-resolution tier so its
/// sprite and textured-model previews do not silently use low-resolution art.
const DEFAULT_SYSTEM_VARIANT: u32 = 1;
const ORBIT_RADIANS_PER_PIXEL: f32 = 0.008;
const MIN_CAMERA_PITCH: f32 = -1.45;
const MAX_CAMERA_PITCH: f32 = 1.45;
const MIN_CAMERA_DISTANCE: f32 = 0.05;
const ENTITY_MODEL_SLOT_COUNT: usize = 4;

pub struct Options {
    pub data_dir: PathBuf,
    pub initial_source: Option<PathBuf>,
    pub initial_mode: ViewerMode,
    pub shade: usize,
    pub model_query: Option<String>,
    pub variant: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssetKind {
    Models,
    Assemblies,
    EntityTypes,
    Sprites,
    Maps,
}

impl AssetKind {
    const COUNT: usize = 5;

    fn next(self) -> Self {
        match self {
            Self::Models => Self::Assemblies,
            Self::Assemblies => Self::EntityTypes,
            Self::EntityTypes => Self::Sprites,
            Self::Sprites => Self::Maps,
            Self::Maps => Self::Models,
        }
    }

    const fn ordinal(self) -> usize {
        match self {
            Self::Models => 0,
            Self::Assemblies => 1,
            Self::EntityTypes => 2,
            Self::Sprites => 3,
            Self::Maps => 4,
        }
    }

    const fn is_model_preview(self) -> bool {
        matches!(self, Self::Models | Self::Assemblies | Self::EntityTypes)
    }

    const fn is_model_catalog(self) -> bool {
        matches!(self, Self::Models | Self::Assemblies)
    }

    const fn hierarchy_depth(self) -> Option<u8> {
        match self {
            Self::Models => Some(0),
            Self::Assemblies | Self::EntityTypes => Some(8),
            Self::Sprites | Self::Maps => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Models => "MODELS",
            Self::Assemblies => "ASSEMBLIES",
            Self::EntityTypes => "ENTITY TYPES",
            Self::Sprites => "SPRITES",
            Self::Maps => "MAPS",
        }
    }

    fn position_label(self) -> &'static str {
        match self {
            Self::Models => "MODEL",
            Self::Assemblies => "ASSEMBLY",
            Self::EntityTypes => "TYPE",
            Self::Sprites => "SPRITE",
            Self::Maps => "MAP",
        }
    }

    fn toolbar_tab(self) -> ToolbarTab {
        match self {
            Self::Models => ToolbarTab::Models,
            Self::Assemblies => ToolbarTab::Assemblies,
            Self::EntityTypes => ToolbarTab::EntityTypes,
            Self::Sprites => ToolbarTab::Sprites,
            Self::Maps => ToolbarTab::Maps,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UnsupportedWorkbenchMode;

impl std::fmt::Display for UnsupportedWorkbenchMode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("audio and save modes are export-only; the workbench supports models, assemblies, entity types, sprites, and terrain")
    }
}

impl std::error::Error for UnsupportedWorkbenchMode {}

impl TryFrom<ViewerMode> for AssetKind {
    type Error = UnsupportedWorkbenchMode;

    fn try_from(mode: ViewerMode) -> Result<Self, Self::Error> {
        match mode {
            ViewerMode::Models => Ok(Self::Models),
            ViewerMode::Sprites => Ok(Self::Sprites),
            ViewerMode::Terrain => Ok(Self::Maps),
            ViewerMode::Audio | ViewerMode::Saves => Err(UnsupportedWorkbenchMode),
        }
    }
}

#[derive(Clone, Copy)]
struct PreviewCamera {
    pitch: f32,
    orbit_yaw: f32,
    distance: f32,
    screen_pan_pixels: [i32; 2],
}

impl Default for PreviewCamera {
    fn default() -> Self {
        Self {
            pitch: -0.28,
            orbit_yaw: 0.45,
            distance: 5.0,
            screen_pan_pixels: [0; 2],
        }
    }
}

impl PreviewCamera {
    fn frame_model(&mut self, radius: Option<f32>) {
        self.reset_pan();
        if let Some(radius) = radius {
            self.distance = (radius * 2.8).max(0.5);
            self.pitch = -0.28;
            self.orbit_yaw = 0.45;
        }
    }

    fn orbit_by_pixels(&mut self, xrel: i32, yrel: i32) {
        self.orbit_yaw += xrel as f32 * ORBIT_RADIANS_PER_PIXEL;
        self.pitch = (self.pitch + yrel as f32 * ORBIT_RADIANS_PER_PIXEL)
            .clamp(MIN_CAMERA_PITCH, MAX_CAMERA_PITCH);
    }

    fn pan_by_pixels(&mut self, xrel: i32, yrel: i32) {
        self.screen_pan_pixels[0] = self.screen_pan_pixels[0].saturating_add(xrel);
        self.screen_pan_pixels[1] = self.screen_pan_pixels[1].saturating_add(yrel);
    }

    fn reset_pan(&mut self) {
        self.screen_pan_pixels = [0; 2];
    }

    fn is_panned(&self) -> bool {
        self.screen_pan_pixels != [0; 2]
    }

    fn zoom_in(&mut self) {
        self.distance = (self.distance * 0.82).max(MIN_CAMERA_DISTANCE);
    }

    fn zoom_out(&mut self) {
        self.distance *= 1.22;
    }

    fn render_camera(self, width: u32, height: u32) -> Camera {
        let width = width.max(1);
        let height = height.max(1);
        let horizontal = self.distance * self.pitch.cos();
        let position = [
            horizontal * self.orbit_yaw.sin(),
            -self.distance * self.pitch.sin(),
            horizontal * self.orbit_yaw.cos(),
        ];
        let mut camera = Camera::new(width as f32 / height as f32);
        camera.position = position;
        camera.yaw = (-position[0]).atan2(position[2]);
        camera.pitch = (-position[1]).atan2(horizontal.max(0.001));
        camera.fov = 50.4_f32.to_radians();
        camera.near = 0.01;
        camera.far = (self.distance * 20.0).max(100.0);
        // Keep the optical centre below the toolbar, then apply diagnostic
        // panning in screen pixels without translating the model hierarchy.
        // Projection offsets move rendered geometry in the opposite NDC X
        // direction, while positive screen Y points down.
        camera.projection_offset = [
            -2.0 * self.screen_pan_pixels[0] as f32 / width as f32,
            TOOLBAR_HEIGHT as f32 / height as f32
                + 2.0 * self.screen_pan_pixels[1] as f32 / height as f32,
        ];
        camera
    }
}

fn normalized_wheel_y(y: i32, direction: MouseWheelDirection) -> i32 {
    match direction {
        MouseWheelDirection::Flipped => y.saturating_neg(),
        MouseWheelDirection::Normal | MouseWheelDirection::Unknown(_) => y,
    }
}

struct State {
    session: GameSession,
    materials: ModelMaterialCache,
    model_ids: Vec<usize>,
    entity_model_slots: Vec<[u16; 4]>,
    sprite_ids: Vec<u16>,
    maps: Vec<MapAsset>,
    kind: AssetKind,
    item_indices: [usize; AssetKind::COUNT],
    entity_model_slot: usize,
    shade: usize,
    show_textures: bool,
    show_shadows: bool,
    map_layer: MapLayer,
    camera: PreviewCamera,
    model_bounds: Option<ModelTreeBounds>,
    preview_palette_level: u32,
    query: String,
    text_input: TextInputState,
    status: String,
}

impl State {
    fn new(options: Options) -> Result<Self, Box<dyn std::error::Error>> {
        let initial_source =
            resolve_initial_source(&options.data_dir, options.initial_source.as_deref());
        let system_variant = selected_system_variant(options.variant, initial_source.as_deref())
            .map_err(|message| std::io::Error::new(std::io::ErrorKind::InvalidInput, message))?;
        let mut session = GameSession::init(&options.data_dir)?;
        let loaded_layers = load_global_asset_layers(&mut session, system_variant)?;
        if let Some(path) = initial_source.as_deref() {
            if !path.is_file() {
                return Err(format!("initial OVL does not exist: {}", path.display()).into());
            }
            session.load_level_from_path(path)?;
        }
        let initial_sprite_id = session
            .cache
            .level()
            .and_then(|level| level.sprites.as_ref())
            .and_then(|atlas| atlas.entries.first())
            .map(|entry| entry.index);
        let preview_palette_level = session
            .cache
            .level()
            .and_then(|level| level.level.as_ref())
            .and_then(|descriptor| world_resource_level(descriptor.world_style))
            .or_else(|| {
                initial_source
                    .as_deref()
                    .and_then(system_level_from_ovl_path)
                    .filter(|level| (6..=11).contains(level))
            })
            .unwrap_or(6);
        let model_ids = session.cache.global_model_ids();
        let entity_model_slots = session.cache.global_entity_model_table();
        let sprite_ids = session.cache.global_sprite_ids();
        let maps = maps::discover(&options.data_dir, initial_source.as_deref())?;

        let requested_kind = if options.model_query.is_some() {
            AssetKind::Models
        } else {
            AssetKind::try_from(options.initial_mode)?
        };
        let kind = first_available_kind(
            requested_kind,
            model_ids.len(),
            entity_model_slots.len(),
            sprite_ids.len(),
            maps.len(),
        )
        .ok_or("no models, sprites, or terrain maps were found")?;

        let mut state = Self {
            session,
            materials: ModelMaterialCache::new(),
            model_ids,
            entity_model_slots,
            sprite_ids,
            maps,
            kind,
            item_indices: [0; AssetKind::COUNT],
            entity_model_slot: 0,
            shade: options.shade,
            show_textures: true,
            show_shadows: true,
            map_layer: MapLayer::Height,
            camera: PreviewCamera::default(),
            model_bounds: None,
            preview_palette_level,
            query: options.model_query.unwrap_or_default(),
            text_input: TextInputState::default(),
            status: if initial_source.is_some() {
                format!("LOADED {loaded_layers} {system_variant}X GLOBAL PACKS + INITIAL OVL")
            } else {
                format!("LOADED {loaded_layers} {system_variant}X DISK GLOBAL PACKS")
            },
        };

        if let Some(initial) = initial_source.as_deref() {
            match state.kind {
                AssetKind::Models => {
                    if let Some(level) = system_level_from_ovl_path(initial) {
                        if let Some(index) = state.model_ids.iter().position(|&id| {
                            state.session.cache.global_model_system_level(id) == Some(level)
                        }) {
                            state.set_item_index(index);
                        }
                    }
                }
                AssetKind::Assemblies | AssetKind::EntityTypes => {}
                AssetKind::Sprites => {
                    if let Some(index) =
                        initial_sprite_id.and_then(|id| state.sprite_ids.binary_search(&id).ok())
                    {
                        state.set_item_index(index);
                    }
                }
                AssetKind::Maps => {
                    if let Some(index) = state
                        .maps
                        .iter()
                        .position(|map| maps::same_path(map.path(), initial))
                    {
                        state.set_item_index(index);
                    }
                }
            }
        }
        state.fit_current_model();
        if !state.query.is_empty() {
            state.find_next();
        }
        Ok(state)
    }

    fn item_count(&self, kind: AssetKind) -> usize {
        match kind {
            AssetKind::Models | AssetKind::Assemblies => self.model_ids.len(),
            AssetKind::EntityTypes => self.entity_model_slots.len(),
            AssetKind::Sprites => self.sprite_ids.len(),
            AssetKind::Maps => self.maps.len(),
        }
    }

    fn item_index(&self) -> usize {
        self.item_indices[self.kind.ordinal()]
    }

    fn set_item_index(&mut self, index: usize) {
        self.item_indices[self.kind.ordinal()] = index;
    }

    fn current_model_id(&self) -> Option<usize> {
        matches!(self.kind, AssetKind::Models | AssetKind::Assemblies)
            .then(|| self.model_ids.get(self.item_index()).copied())
            .flatten()
    }

    fn current_entity_type(&self) -> Option<usize> {
        (self.kind == AssetKind::EntityTypes).then_some(self.item_index())
    }

    fn current_preview_model_id(&self) -> Option<usize> {
        match self.kind {
            AssetKind::Models | AssetKind::Assemblies => self.current_model_id(),
            AssetKind::EntityTypes => selected_entity_model(
                &self.entity_model_slots,
                self.item_index(),
                self.entity_model_slot,
            ),
            AssetKind::Sprites | AssetKind::Maps => None,
        }
    }

    fn current_model(&self) -> Option<&ModelEntry> {
        self.current_preview_model_id()
            .and_then(|id| self.session.cache.global_model(id))
    }

    fn model_palette_level(&self, model_id: usize) -> Option<u32> {
        self.session
            .cache
            .global_model_system_level(model_id)
            .filter(|&level| self.session.cache.system_color_palette(level).is_some())
            .or_else(|| {
                self.session
                    .cache
                    .system_color_palette(self.preview_palette_level)
                    .map(|_| self.preview_palette_level)
            })
    }

    fn current_sprite_id(&self) -> Option<u16> {
        (self.kind == AssetKind::Sprites)
            .then(|| self.sprite_ids.get(self.item_index()).copied())
            .flatten()
    }

    fn current_sprite(&self) -> Option<(&v2k_formats::sprites::SpriteAtlas, &SpriteEntry)> {
        self.current_sprite_id()
            .and_then(|id| self.session.cache.global_sprite(id))
    }

    fn current_map(&self) -> Option<&MapAsset> {
        (self.kind == AssetKind::Maps)
            .then(|| self.maps.get(self.item_index()))
            .flatten()
    }

    fn set_kind(&mut self, kind: AssetKind) {
        self.text_input.cancel();
        if self.kind == kind {
            return;
        }
        if self.item_count(kind) == 0 {
            self.status = format!("NO {} FOUND", kind.label());
            return;
        }
        let shared_model_index =
            (self.kind.is_model_catalog() && kind.is_model_catalog()).then_some(self.item_index());
        self.kind = kind;
        if let Some(index) = shared_model_index {
            self.set_item_index(index);
        }
        self.status.clear();
        self.fit_current_model();
    }

    fn next(&mut self) {
        let count = self.item_count(self.kind);
        if count != 0 {
            self.set_item_index((self.item_index() + 1) % count);
            self.status.clear();
            self.fit_current_model();
        }
    }

    fn previous(&mut self) {
        let count = self.item_count(self.kind);
        if count != 0 {
            self.set_item_index((self.item_index() + count - 1) % count);
            self.status.clear();
            self.fit_current_model();
        }
    }

    fn home(&mut self) {
        self.set_item_index(0);
        self.status.clear();
        self.fit_current_model();
    }

    fn fit_current_model(&mut self) {
        let depth = self.kind.hierarchy_depth();
        self.model_bounds = self.current_preview_model_id().and_then(|model_id| {
            linked_model_bounds(
                &self.session.cache,
                model_id,
                PREVIEW_MODEL_SCALE,
                depth.unwrap_or_default(),
                &AnimVars::default(),
            )
        });
        if self.kind.is_model_preview() {
            // Every model selection starts centred, including entries whose
            // linked hierarchy has no measurable geometry.
            self.camera
                .frame_model(self.model_bounds.map(|bounds| bounds.radius));
        }
    }

    fn can_reset_position(&self) -> bool {
        self.kind.is_model_preview() && self.camera.is_panned()
    }

    fn reset_position(&mut self) {
        if self.can_reset_position() {
            self.camera.reset_pan();
        }
    }

    fn begin_search(&mut self) {
        self.text_input.activate_search();
        self.status.clear();
    }

    fn begin_catalog_position_edit(&mut self) {
        let count = self.item_count(self.kind);
        if count == 0 {
            return;
        }
        self.text_input.activate_catalog_position();
        self.status = format!("TYPE {} POSITION 1 TO {count}", self.kind.position_label());
    }

    fn cancel_text_input(&mut self) {
        let was_active = self.text_input.is_active();
        self.text_input.cancel();
        if was_active {
            self.status.clear();
        }
    }

    fn commit_catalog_position(&mut self) {
        let count = self.item_count(self.kind);
        match parse_catalog_position(self.text_input.catalog_position(), count) {
            Ok(index) => {
                self.set_item_index(index);
                self.text_input.cancel();
                self.fit_current_model();
                self.status = format!(
                    "JUMPED TO {} POSITION {}",
                    self.kind.position_label(),
                    index + 1
                );
            }
            Err(CatalogPositionError::Empty) => {
                self.status = format!(
                    "ENTER A {} POSITION FROM 1 TO {count}",
                    self.kind.position_label()
                );
            }
            Err(CatalogPositionError::Invalid) => {
                self.status = format!(
                    "{} POSITION MUST USE ASCII DIGITS",
                    self.kind.position_label()
                );
            }
            Err(CatalogPositionError::OutOfRange) => {
                self.status = format!(
                    "{} POSITION MUST BE 1 TO {count}",
                    self.kind.position_label()
                );
            }
        }
    }

    fn cycle_entity_model_slot(&mut self, direction: NavigationDirection) {
        if self.kind != AssetKind::EntityTypes {
            return;
        }
        self.entity_model_slot = cycled_entity_model_slot(self.entity_model_slot, direction);
        self.status = format!(
            "SELECTED AUTHORED MODEL SLOT {}",
            self.entity_model_slot + 1
        );
        self.fit_current_model();
    }

    fn find_next(&mut self) {
        let needle = self.query.trim().to_ascii_lowercase();
        if needle.is_empty() {
            self.status = "TYPE A GLOBAL ID OR NAME".to_string();
            return;
        }
        match self.kind {
            AssetKind::Models | AssetKind::Assemblies => self.find_model(&needle),
            AssetKind::EntityTypes => self.find_entity_type(&needle),
            AssetKind::Sprites => self.find_sprite(&needle),
            AssetKind::Maps => self.find_map(&needle),
        }
    }

    fn find_model(&mut self, needle: &str) {
        let numeric = needle.parse::<usize>().ok();
        let count = self.model_ids.len();
        let found = (1..=count).find_map(|offset| {
            let index = (self.item_index() + offset) % count;
            let id = self.model_ids[index];
            let model = self.session.cache.global_model(id)?;
            let name_matches = model
                .name
                .as_deref()
                .is_some_and(|name| name.to_ascii_lowercase().contains(needle));
            (numeric == Some(id) || name_matches).then_some(index)
        });
        if let Some(index) = found {
            self.set_item_index(index);
            self.fit_current_model();
            self.status = "FOUND GLOBAL MODEL".to_string();
        } else {
            self.status = "NO GLOBAL MODEL MATCH".to_string();
        }
    }

    fn find_sprite(&mut self, needle: &str) {
        let Some(id) = needle.parse::<u16>().ok() else {
            self.status = "SPRITE SEARCH NEEDS A GLOBAL ID".to_string();
            return;
        };
        if let Ok(index) = self.sprite_ids.binary_search(&id) {
            self.set_item_index(index);
            self.status = "FOUND GLOBAL SPRITE".to_string();
        } else {
            self.status = "NO GLOBAL SPRITE MATCH".to_string();
        }
    }

    fn find_map(&mut self, needle: &str) {
        let numeric = needle.parse::<usize>().ok();
        let count = self.maps.len();
        let found = (1..=count).find_map(|offset| {
            let index = (self.item_index() + offset) % count;
            let name_matches = self.maps[index]
                .source_name()
                .to_ascii_lowercase()
                .contains(needle);
            (numeric == Some(index) || name_matches).then_some(index)
        });
        if let Some(index) = found {
            self.set_item_index(index);
            self.status = "FOUND DIAGNOSTIC MAP".to_string();
        } else {
            self.status = "NO DIAGNOSTIC MAP MATCH".to_string();
        }
    }

    fn find_entity_type(&mut self, needle: &str) {
        let numeric = needle.parse::<usize>().ok();
        let count = self.entity_model_slots.len();
        let found = (1..=count).find_map(|offset| {
            let entity_type = (self.item_index() + offset) % count;
            let name_matches = self.entity_model_slots[entity_type]
                .iter()
                .any(|&model_id| {
                    self.session
                        .cache
                        .global_model(usize::from(model_id))
                        .and_then(|model| model.name.as_deref())
                        .is_some_and(|name| name.to_ascii_lowercase().contains(needle))
                });
            (numeric == Some(entity_type) || name_matches).then_some(entity_type)
        });
        if let Some(entity_type) = found {
            self.set_item_index(entity_type);
            self.fit_current_model();
            self.status = "FOUND ENTITY TYPE".to_string();
        } else {
            self.status = "NO ENTITY TYPE OR SLOT MODEL MATCH".to_string();
        }
    }

    fn info_line(&self) -> String {
        match self.kind {
            AssetKind::Models | AssetKind::Assemblies => {
                let Some(id) = self.current_model_id() else {
                    return "NO GLOBAL MODELS".to_string();
                };
                let Some(model) = self.current_model() else {
                    return format!("UNRESOLVED GLOBAL MODEL {id}");
                };
                format!(
                    "{} {id}  {}/{}  {}  V {} T {} LINKS {}  PAL L{}",
                    if self.kind == AssetKind::Models {
                        "ROOT MODEL"
                    } else {
                        "ASSEMBLY ROOT"
                    },
                    self.item_index() + 1,
                    self.model_ids.len(),
                    model.name.as_deref().unwrap_or("UNNAMED"),
                    model.vertices.len(),
                    model.triangles.len(),
                    model.instances.len(),
                    self.model_palette_level(id).unwrap_or_default(),
                )
            }
            AssetKind::EntityTypes => {
                let Some(entity_type) = self.current_entity_type() else {
                    return "NO GLOBAL ENTITY TYPES".to_string();
                };
                let slots = self.entity_model_slots[entity_type];
                let model_id = usize::from(slots[self.entity_model_slot]);
                let model_name = self
                    .session
                    .cache
                    .global_model(model_id)
                    .and_then(|model| model.name.as_deref())
                    .unwrap_or("UNRESOLVED");
                let slot_list = slots
                    .iter()
                    .enumerate()
                    .map(|(slot, model)| {
                        if slot == self.entity_model_slot {
                            format!("{model}*")
                        } else {
                            model.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                let metadata = self
                    .session
                    .cache
                    .global_entity_type(entity_type)
                    .map(|record| {
                        format!(
                            "TAG {} SCALE {} ID {:08X} FLAGS {:08X} SUBS {}",
                            record.type_tag,
                            record.scale,
                            record.id_field,
                            record.flags,
                            record.active_subs.join("")
                        )
                    })
                    .unwrap_or_else(|| "RECORD UNRESOLVED".to_string());
                format!(
                    "TYPE {entity_type}  {}/{}  SLOTS [{slot_list}]  MODEL {model_id} {model_name}  {metadata}",
                    self.item_index() + 1,
                    self.entity_model_slots.len(),
                )
            }
            AssetKind::Sprites => {
                let Some(id) = self.current_sprite_id() else {
                    return "NO GLOBAL SPRITES".to_string();
                };
                let entry = self.current_sprite().map(|(_, entry)| entry);
                format!(
                    "GLOBAL SPRITE {id}  {}/{}  ENTRY {}  FLAGS {:02X}",
                    self.item_index() + 1,
                    self.sprite_ids.len(),
                    entry.map(|entry| entry.entry_idx).unwrap_or_default(),
                    entry.map(|entry| entry.pal_size as u8).unwrap_or_default(),
                )
            }
            AssetKind::Maps => {
                let Some(map) = self.current_map() else {
                    return "NO DIAGNOSTIC MAPS".to_string();
                };
                let height_range = map.height_range();
                format!(
                    "DIAGNOSTIC SECTION 10  {}  {}/{}  HEIGHT {} TO {}",
                    map.source_name(),
                    self.item_index() + 1,
                    self.maps.len(),
                    height_range.0,
                    height_range.1,
                )
            }
        }
    }
}

fn selected_entity_model(
    entity_model_slots: &[[u16; ENTITY_MODEL_SLOT_COUNT]],
    entity_type: usize,
    slot: usize,
) -> Option<usize> {
    entity_model_slots
        .get(entity_type)
        .and_then(|models| models.get(slot))
        .copied()
        .map(usize::from)
}

fn cycled_entity_model_slot(slot: usize, direction: NavigationDirection) -> usize {
    match direction {
        NavigationDirection::Previous => {
            (slot + ENTITY_MODEL_SLOT_COUNT - 1) % ENTITY_MODEL_SLOT_COUNT
        }
        NavigationDirection::Next => (slot + 1) % ENTITY_MODEL_SLOT_COUNT,
    }
}

pub fn run(options: Options) -> Result<(), Box<dyn std::error::Error>> {
    let mut state = State::new(options)?;
    let sdl = sdl2::init().map_err(|error| format!("SDL2 init: {error}"))?;
    let video = sdl
        .video()
        .map_err(|error| format!("SDL2 video: {error}"))?;
    let gl_attributes = video.gl_attr();
    gl_attributes.set_context_profile(sdl2::video::GLProfile::Compatibility);
    gl_attributes.set_context_version(2, 1);
    gl_attributes.set_double_buffer(true);
    let mut window = video
        .window("V2K Asset Workbench", 1200, 800)
        .position_centered()
        .resizable()
        .opengl()
        .build()
        .map_err(|error| error.to_string())?;
    apply_app_window_icon(&mut window);
    let mut renderer = GlRenderer::new(window, 1200, 800)
        .map_err(|error| format!("OpenGL workbench init: {error}"))?;
    let mut events = sdl
        .event_pump()
        .map_err(|error| format!("SDL2 events: {error}"))?;
    video.text_input().start();

    render(&mut renderer, &mut state);
    let mut navigation_repeat = NavigationRepeat::default();
    let mut running = true;
    while running {
        let mut dirty = false;
        let timeout_ms = navigation_repeat.wait_timeout_ms(Instant::now());
        if let Some(event) = events.wait_event_timeout(timeout_ms) {
            running = handle_event(
                &mut state,
                &mut renderer,
                &mut navigation_repeat,
                Instant::now(),
                event,
            );
            dirty = true;
        }
        for event in events.poll_iter() {
            dirty = true;
            if !handle_event(
                &mut state,
                &mut renderer,
                &mut navigation_repeat,
                Instant::now(),
                event,
            ) {
                running = false;
                break;
            }
        }
        if state.text_input.is_active() {
            navigation_repeat.clear();
        } else if running {
            if let Some(steps) = navigation_repeat.take_due(Instant::now()) {
                for _ in 0..steps.count {
                    navigate(&mut state, steps.direction);
                }
                dirty = true;
            }
        }
        if running && dirty {
            // The workbench currently displays a fixed AnimVars frame, so it
            // only needs to rebuild and upload diagnostic surfaces after an
            // input/window event rather than spinning at ~125 FPS.
            render(&mut renderer, &mut state);
        }
    }
    Ok(())
}

fn navigate(state: &mut State, direction: NavigationDirection) {
    match direction {
        NavigationDirection::Previous => state.previous(),
        NavigationDirection::Next => state.next(),
    }
}

fn arrow_navigation_direction(key: Keycode) -> Option<NavigationDirection> {
    match key {
        Keycode::Left => Some(NavigationDirection::Previous),
        Keycode::Right => Some(NavigationDirection::Next),
        _ => None,
    }
}

fn handle_event(
    state: &mut State,
    renderer: &mut dyn Renderer,
    navigation_repeat: &mut NavigationRepeat,
    now: Instant,
    event: Event,
) -> bool {
    match event {
        Event::Quit { .. }
        | Event::Window {
            win_event: WindowEvent::Close,
            ..
        } => return false,
        Event::Window {
            win_event: WindowEvent::Resized(width, height) | WindowEvent::SizeChanged(width, height),
            ..
        } => renderer.resize(width.max(1) as u32, height.max(1) as u32),
        Event::Window {
            win_event: WindowEvent::FocusLost,
            ..
        } => {
            navigation_repeat.clear();
            state.cancel_text_input();
        }
        Event::TextInput { text, .. } if state.text_input.is_search_active() => {
            state.query.push_str(&text);
        }
        Event::TextInput { text, .. } if state.text_input.is_catalog_position_active() => {
            state.text_input.push_catalog_position_text(&text);
        }
        Event::KeyDown {
            keycode: Some(key),
            keymod,
            repeat: false,
            ..
        } => {
            if state.text_input.is_catalog_position_active() {
                match key {
                    Keycode::Escape => state.cancel_text_input(),
                    Keycode::Backspace => state.text_input.backspace_catalog_position(),
                    Keycode::Return | Keycode::KpEnter => state.commit_catalog_position(),
                    Keycode::Tab => {
                        state.cancel_text_input();
                        state.set_kind(state.kind.next());
                    }
                    _ => {}
                }
                return true;
            }
            if state.text_input.is_search_active() {
                match key {
                    Keycode::Escape => state.cancel_text_input(),
                    Keycode::Backspace => {
                        state.query.pop();
                    }
                    Keycode::Return | Keycode::KpEnter => {
                        state.find_next();
                        state.text_input.cancel();
                    }
                    Keycode::Tab => {
                        state.cancel_text_input();
                        state.set_kind(state.kind.next());
                    }
                    _ => {}
                }
                return true;
            }
            match key {
                Keycode::Escape | Keycode::Q => return false,
                Keycode::Tab => state.set_kind(state.kind.next()),
                Keycode::F1 => state.set_kind(AssetKind::Models),
                Keycode::F2 => state.set_kind(AssetKind::Assemblies),
                Keycode::F3 => state.set_kind(AssetKind::EntityTypes),
                Keycode::F4 => state.set_kind(AssetKind::Sprites),
                Keycode::F5 => state.set_kind(AssetKind::Maps),
                Keycode::Right => {
                    if navigation_repeat.press(NavigationDirection::Next, now) {
                        navigate(state, NavigationDirection::Next);
                    }
                }
                Keycode::Left => {
                    if navigation_repeat.press(NavigationDirection::Previous, now) {
                        navigate(state, NavigationDirection::Previous);
                    }
                }
                Keycode::PageDown | Keycode::Space => state.next(),
                Keycode::PageUp => state.previous(),
                Keycode::Home => state.home(),
                Keycode::T if state.kind.is_model_preview() => {
                    state.show_textures = !state.show_textures
                }
                Keycode::H if state.kind.is_model_preview() => {
                    state.show_shadows = !state.show_shadows
                }
                Keycode::L if state.kind == AssetKind::Maps => {
                    state.map_layer = state.map_layer.next()
                }
                Keycode::A if state.kind.is_model_preview() => state.camera.orbit_yaw -= 0.12,
                Keycode::D if state.kind.is_model_preview() => state.camera.orbit_yaw += 0.12,
                Keycode::W if state.kind.is_model_preview() => {
                    state.camera.pitch = (state.camera.pitch - 0.12).max(MIN_CAMERA_PITCH)
                }
                Keycode::S if state.kind.is_model_preview() => {
                    state.camera.pitch = (state.camera.pitch + 0.12).min(MAX_CAMERA_PITCH)
                }
                Keycode::Equals | Keycode::Plus | Keycode::KpPlus
                    if state.kind.is_model_preview() =>
                {
                    state.camera.zoom_in()
                }
                Keycode::Minus | Keycode::KpMinus if state.kind.is_model_preview() => {
                    state.camera.zoom_out()
                }
                Keycode::Slash => {
                    state.begin_search();
                    navigation_repeat.clear();
                }
                Keycode::F if keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD) => {
                    state.begin_search();
                    navigation_repeat.clear();
                }
                Keycode::G => {
                    state.begin_catalog_position_edit();
                    navigation_repeat.clear();
                }
                Keycode::V | Keycode::RightBracket => {
                    state.cycle_entity_model_slot(NavigationDirection::Next)
                }
                Keycode::LeftBracket => {
                    state.cycle_entity_model_slot(NavigationDirection::Previous)
                }
                Keycode::Return | Keycode::KpEnter if !state.query.is_empty() => state.find_next(),
                _ => {}
            }
        }
        Event::KeyUp {
            keycode: Some(key), ..
        } => {
            if let Some(direction) = arrow_navigation_direction(key) {
                navigation_repeat.release(direction, now);
            }
        }
        Event::MouseWheel { y, direction, .. } if state.kind.is_model_preview() => {
            let y = normalized_wheel_y(y, direction);
            if y > 0 {
                state.camera.zoom_in();
            } else if y < 0 {
                state.camera.zoom_out();
            }
        }
        Event::MouseMotion {
            xrel,
            yrel,
            mousestate,
            ..
        } if state.kind.is_model_preview() => {
            if mousestate.right() {
                state.camera.orbit_by_pixels(xrel, yrel);
            }
            if mousestate.middle() {
                state.camera.pan_by_pixels(xrel, yrel);
            }
        }
        Event::MouseButtonDown {
            mouse_btn: MouseButton::Left,
            x,
            y,
            ..
        } => handle_click(state, navigation_repeat, Point::new(x, y)),
        _ => {}
    }
    true
}

fn handle_click(state: &mut State, navigation_repeat: &mut NavigationRepeat, point: Point) {
    let ui = UiRects::new();
    let action = ui.action_at(point).filter(|action| {
        action.is_enabled(
            state.kind.is_model_preview(),
            state.kind == AssetKind::EntityTypes,
            state.kind == AssetKind::Maps,
            state.can_reset_position(),
        )
    });
    if !matches!(action, Some(UiAction::Search | UiAction::CatalogPosition)) {
        state.cancel_text_input();
    }
    match action {
        Some(UiAction::SelectModels) => state.set_kind(AssetKind::Models),
        Some(UiAction::SelectAssemblies) => state.set_kind(AssetKind::Assemblies),
        Some(UiAction::SelectEntityTypes) => state.set_kind(AssetKind::EntityTypes),
        Some(UiAction::SelectSprites) => state.set_kind(AssetKind::Sprites),
        Some(UiAction::SelectMaps) => state.set_kind(AssetKind::Maps),
        Some(UiAction::Previous) => state.previous(),
        Some(UiAction::Next) => state.next(),
        Some(UiAction::Search) => {
            state.begin_search();
            navigation_repeat.clear();
        }
        Some(UiAction::CatalogPosition) => {
            state.begin_catalog_position_edit();
            navigation_repeat.clear();
        }
        Some(UiAction::ToggleTextures) => state.show_textures = !state.show_textures,
        Some(UiAction::ToggleShadows) => state.show_shadows = !state.show_shadows,
        Some(UiAction::NextMapLayer) => state.map_layer = state.map_layer.next(),
        Some(UiAction::NextEntityModelSlot) => {
            state.cycle_entity_model_slot(NavigationDirection::Next)
        }
        Some(UiAction::ResetPosition) => state.reset_position(),
        None => {}
    }
}

fn render(renderer: &mut dyn Renderer, state: &mut State) {
    let (width, height) = renderer.viewport_size();
    renderer.begin_scene(RenderScene::Menu);
    renderer.clear(
        BACKGROUND[0] as f32 / 255.0,
        BACKGROUND[1] as f32 / 255.0,
        BACKGROUND[2] as f32 / 255.0,
    );
    let content = Rect::new(
        0,
        TOOLBAR_HEIGHT,
        width,
        height.saturating_sub(TOOLBAR_HEIGHT as u32),
    );
    match state.kind {
        AssetKind::Models | AssetKind::Assemblies | AssetKind::EntityTypes => {
            render_model_preview(renderer, state, width, height)
        }
        AssetKind::Sprites => {
            if let Err(error) = render_sprite(renderer, state, content) {
                state.status = format!("SPRITE ERROR: {error}");
            }
        }
        AssetKind::Maps => render_map(renderer, state, content),
    }
    let info = state.info_line();
    ui::render_toolbar(
        renderer,
        width,
        ToolbarView {
            active_tab: state.kind.toolbar_tab(),
            query: &state.query,
            search_active: state.text_input.is_search_active(),
            catalog_label: state.kind.position_label(),
            catalog_position: Some(state.item_index() + 1),
            catalog_count: state.item_count(state.kind),
            catalog_position_input: state.text_input.catalog_position(),
            catalog_position_active: state.text_input.is_catalog_position_active(),
            model_preview_active: state.kind.is_model_preview(),
            entity_types_active: state.kind == AssetKind::EntityTypes,
            entity_model_slot: state.entity_model_slot,
            show_textures: state.show_textures,
            show_shadows: state.show_shadows,
            can_reset_position: state.can_reset_position(),
            map_layer: state.map_layer.label(),
            info: &info,
            status: &state.status,
        },
    );
    renderer.present();
}

fn render_model_preview(renderer: &mut dyn Renderer, state: &State, width: u32, height: u32) {
    let Some(model_id) = state.current_preview_model_id() else {
        return;
    };
    let vars = AnimVars::default();
    let center = state
        .model_bounds
        .map(|bounds| bounds.center)
        .unwrap_or([0.0; 3]);
    let position = [-center[0], -center[1], -center[2]];
    let camera = state.camera.render_camera(width, height);
    renderer.set_fog(false, 0.0, camera.far, [0.0; 3]);
    renderer.set_camera(&camera);
    let mut tree = ModelTreeRenderer::new(
        renderer,
        &state.session.cache,
        &state.materials,
        PREVIEW_MODEL_SCALE,
        Some(ModelSceneLight::NEUTRAL),
        ViewPinMode::CameraFacing,
    )
    .with_view((&camera).into())
    .with_style(ModelTreeStyle {
        textures: state.show_textures,
        shadows: state.show_shadows,
    });
    let source_palette = state
        .model_palette_level(model_id)
        .and_then(|level| state.session.cache.system_color_palette(level));
    if let Some(palette) = source_palette {
        tree = tree.with_palette_override(palette);
    }
    tree.draw_linked(
        model_id,
        orientation_from_ypr(0.0, 0.0, 0.0),
        position,
        state.kind.hierarchy_depth().unwrap_or_default(),
        None,
        &vars,
    );
}

fn render_sprite(renderer: &mut dyn Renderer, state: &State, content: Rect) -> Result<(), String> {
    let id = state
        .current_sprite_id()
        .ok_or_else(|| "no selected global sprite".to_string())?;
    let (atlas, entry) = state
        .current_sprite()
        .ok_or_else(|| format!("global sprite {id} is unresolved"))?;
    let sprite = atlas
        .decode_sprite(entry, state.shade)
        .map_err(|error| format!("global sprite {id}: {error}"))?;
    draw_sprite_preview(renderer, content, &sprite);
    Ok(())
}

fn draw_sprite_preview(renderer: &mut dyn Renderer, content: Rect, sprite: &DecodedSprite) {
    let destination = fit_rect(sprite.width as u32, sprite.height as u32, content, 12.0);
    let mut surface = UiSurface::new(destination.width(), destination.height(), [0, 0, 0, 255]);
    surface.draw_checkerboard(Rect::new(0, 0, destination.width(), destination.height()));
    surface.blit_scaled(
        &sprite.rgba,
        sprite.width as u32,
        sprite.height as u32,
        Rect::new(0, 0, destination.width(), destination.height()),
    );
    surface.draw(renderer, destination.x(), destination.y());
}

fn render_map(renderer: &mut dyn Renderer, state: &State, content: Rect) {
    let Some(map) = state.current_map() else {
        return;
    };
    let rgba = map.diagnostic_rgba(state.map_layer);
    let destination = fit_rect(MAP_SIZE, MAP_SIZE, content, 3.0);
    let mut surface = UiSurface::new(destination.width(), destination.height(), [0, 0, 0, 255]);
    surface.blit_scaled(
        &rgba,
        MAP_SIZE,
        MAP_SIZE,
        Rect::new(0, 0, destination.width(), destination.height()),
    );
    surface.draw(renderer, destination.x(), destination.y());
}

fn load_global_asset_layers(
    session: &mut GameSession,
    variant: u32,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut loaded = 0;
    for level in GLOBAL_ASSET_LEVELS {
        let path = session
            .data_dir
            .join("Overlay")
            .join(format!("{variant}X{level}XX.OVL"));
        if path.is_file() {
            session.load_auxiliary_ovl(level, variant)?;
            loaded += 1;
        }
    }
    Ok(loaded)
}

fn resolve_initial_source(data_dir: &Path, initial: Option<&Path>) -> Option<PathBuf> {
    initial.map(|path| {
        if path.is_absolute() || path.is_file() {
            path.to_path_buf()
        } else {
            data_dir.join(path)
        }
    })
}

fn ovl_variant_and_system_level(path: &Path) -> Option<(u32, u32)> {
    let stem = path.file_stem()?.to_str()?.to_ascii_uppercase();
    let stem = stem.strip_suffix("XX")?;
    let (variant, level) = stem.split_once('X')?;
    let variant = variant.parse().ok()?;
    let level = level.parse().ok()?;
    (variant <= 3).then_some((variant, level))
}

fn system_level_from_ovl_path(path: &Path) -> Option<u32> {
    ovl_variant_and_system_level(path).map(|(_, level)| level)
}

fn selected_system_variant(
    explicit: Option<u32>,
    initial_source: Option<&Path>,
) -> Result<u32, String> {
    let inferred = initial_source
        .and_then(|path| ovl_variant_and_system_level(path).map(|(variant, _)| variant));
    if let (Some(explicit), Some(inferred)) = (explicit, inferred) {
        if explicit != inferred {
            return Err(format!(
                "--variant {explicit} conflicts with explicit {inferred}X OVL source"
            ));
        }
    }
    Ok(explicit.or(inferred).unwrap_or(DEFAULT_SYSTEM_VARIANT))
}

fn first_available_kind(
    requested: AssetKind,
    models: usize,
    entity_types: usize,
    sprites: usize,
    maps: usize,
) -> Option<AssetKind> {
    let count = |kind| match kind {
        AssetKind::Models | AssetKind::Assemblies => models,
        AssetKind::EntityTypes => entity_types,
        AssetKind::Sprites => sprites,
        AssetKind::Maps => maps,
    };
    let mut candidate = requested;
    for _ in 0..AssetKind::COUNT {
        if count(candidate) != 0 {
            return Some(candidate);
        }
        candidate = candidate.next();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_kind_cycles_through_workbench_tabs() {
        assert_eq!(AssetKind::Models.next(), AssetKind::Assemblies);
        assert_eq!(AssetKind::Assemblies.next(), AssetKind::EntityTypes);
        assert_eq!(AssetKind::EntityTypes.next(), AssetKind::Sprites);
        assert_eq!(AssetKind::Sprites.next(), AssetKind::Maps);
        assert_eq!(AssetKind::Maps.next(), AssetKind::Models);
    }

    #[test]
    fn preview_tabs_keep_root_and_hierarchy_semantics_distinct() {
        assert_eq!(AssetKind::Models.hierarchy_depth(), Some(0));
        assert_eq!(AssetKind::Assemblies.hierarchy_depth(), Some(8));
        assert_eq!(AssetKind::EntityTypes.hierarchy_depth(), Some(8));
        assert_eq!(AssetKind::Sprites.hierarchy_depth(), None);
    }

    #[test]
    fn entity_type_slot_selection_returns_one_alternative_at_a_time() {
        let slots = [[41, 67, 81, 225]];
        assert_eq!(selected_entity_model(&slots, 0, 0), Some(41));
        assert_eq!(selected_entity_model(&slots, 0, 1), Some(67));
        assert_eq!(selected_entity_model(&slots, 0, 3), Some(225));
        assert_eq!(selected_entity_model(&slots, 1, 0), None);
        assert_eq!(selected_entity_model(&slots, 0, 4), None);

        assert_eq!(
            cycled_entity_model_slot(0, NavigationDirection::Previous),
            3
        );
        assert_eq!(cycled_entity_model_slot(3, NavigationDirection::Next), 0);
    }

    #[v2k_test_support::retail_test]
    fn entity_type_catalog_reads_the_cumulative_section12_table() {
        let data_dir = v2k_test_support::retail_dir();
        let mut session = GameSession::init(&data_dir).expect("initialize V2000 data");
        load_global_asset_layers(&mut session, DEFAULT_SYSTEM_VARIANT)
            .expect("load global asset layers");

        let slots = session.cache.global_entity_model_table();
        assert_eq!(slots.len(), 130);
        assert_eq!(slots[46], [41, 67, 41, 67]);
        assert_eq!(
            session
                .cache
                .global_model(selected_entity_model(&slots, 46, 0).unwrap())
                .and_then(|model| model.name.as_deref()),
            Some("player4")
        );
    }

    #[v2k_test_support::retail_test]
    fn selected_tier_overlays_preload_low_sprite_atlases() {
        let data_dir = v2k_test_support::retail_dir();
        let mut session = GameSession::init(&data_dir).expect("initialize V2000 data");
        load_global_asset_layers(&mut session, DEFAULT_SYSTEM_VARIANT)
            .expect("load high-tier global asset layers");

        for level in [2, 5] {
            let load_atlas = |variant| {
                let path = data_dir
                    .join("Overlay")
                    .join(format!("{variant}X{level}XX.OVL"));
                let bytes = std::fs::read(path).expect("read system OVL");
                let ovl = v2k_formats::ovl::OvlFile::parse(&bytes).expect("parse system OVL");
                v2k_formats::sections::parse_sprites(&ovl).expect("parse system sprite atlas")
            };
            let low = load_atlas(0);
            let high = load_atlas(DEFAULT_SYSTEM_VARIANT);
            assert_ne!(
                low.height, high.height,
                "level {level} must distinguish tiers"
            );
            let global_id = high.entries[0].index;
            let (cached, _) = session
                .cache
                .global_sprite(global_id)
                .expect("selected-tier global sprite");
            assert_eq!(cached.height, high.height, "level {level} stayed low-tier");
            assert_eq!(cached.entries.len(), high.entries.len());
        }
    }

    #[test]
    fn viewer_modes_only_convert_when_the_workbench_supports_them() {
        assert_eq!(
            AssetKind::try_from(ViewerMode::Models),
            Ok(AssetKind::Models)
        );
        assert_eq!(
            AssetKind::try_from(ViewerMode::Sprites),
            Ok(AssetKind::Sprites)
        );
        assert_eq!(
            AssetKind::try_from(ViewerMode::Terrain),
            Ok(AssetKind::Maps)
        );
        assert_eq!(
            AssetKind::try_from(ViewerMode::Audio),
            Err(UnsupportedWorkbenchMode)
        );
        assert_eq!(
            AssetKind::try_from(ViewerMode::Saves),
            Err(UnsupportedWorkbenchMode)
        );
    }

    #[test]
    fn requested_empty_tab_falls_back_without_losing_global_order() {
        assert_eq!(
            first_available_kind(AssetKind::Models, 0, 0, 12, 3),
            Some(AssetKind::Sprites)
        );
        assert_eq!(
            first_available_kind(AssetKind::Sprites, 0, 0, 0, 3),
            Some(AssetKind::Maps)
        );
        assert_eq!(
            first_available_kind(AssetKind::Assemblies, 0, 130, 12, 3),
            Some(AssetKind::EntityTypes)
        );
    }

    #[test]
    fn ovl_filename_recovers_system_level_for_initial_model_selection() {
        assert_eq!(
            system_level_from_ovl_path(Path::new("Overlay/0X6XX.OVL")),
            Some(6)
        );
        assert_eq!(
            system_level_from_ovl_path(Path::new("Overlay/3X11XX.ovl")),
            Some(11)
        );
        assert_eq!(system_level_from_ovl_path(Path::new("PRELOAD.DAT")), None);
    }

    #[test]
    fn workbench_variant_defaults_high_and_follows_explicit_ovl_tier() {
        assert_eq!(selected_system_variant(None, None), Ok(1));
        assert_eq!(
            selected_system_variant(None, Some(Path::new("Overlay/0X3XX.OVL"))),
            Ok(0)
        );
        assert_eq!(
            selected_system_variant(None, Some(Path::new("Overlay/3X11XX.ovl"))),
            Ok(3)
        );
        assert_eq!(
            selected_system_variant(Some(2), Some(Path::new("Overlay/2X3XX.OVL"))),
            Ok(2)
        );
        assert_eq!(
            selected_system_variant(None, Some(Path::new("PRELOAD.DAT"))),
            Ok(1)
        );
        assert_eq!(
            selected_system_variant(Some(2), Some(Path::new("PRELOAD.DAT"))),
            Ok(2)
        );
    }

    #[test]
    fn explicit_variant_cannot_conflict_with_the_initial_ovl_tier() {
        assert_eq!(
            selected_system_variant(Some(2), Some(Path::new("Overlay/0X3XX.OVL"))),
            Err("--variant 2 conflicts with explicit 0X OVL source".to_owned())
        );
    }

    #[test]
    fn ovl_filename_rejects_an_unsupported_system_variant() {
        assert_eq!(
            ovl_variant_and_system_level(Path::new("Overlay/4X3XX.OVL")),
            None
        );
        assert_eq!(
            ovl_variant_and_system_level(Path::new("Overlay/10X3XX.OVL")),
            None
        );
    }

    #[test]
    fn mouse_orbit_uses_sdl_right_and_down_signs_and_clamps_pitch() {
        let mut camera = PreviewCamera::default();
        let initial_yaw = camera.orbit_yaw;
        let initial_pitch = camera.pitch;

        camera.orbit_by_pixels(15, 15);
        assert!((camera.orbit_yaw - (initial_yaw + 0.12)).abs() < 1e-6);
        assert!((camera.pitch - (initial_pitch + 0.12)).abs() < 1e-6);

        camera.orbit_by_pixels(0, i32::MAX);
        assert_eq!(camera.pitch, MAX_CAMERA_PITCH);
        camera.orbit_by_pixels(0, i32::MIN);
        assert_eq!(camera.pitch, MIN_CAMERA_PITCH);
    }

    #[test]
    fn wheel_direction_normalization_respects_natural_scrolling() {
        assert_eq!(normalized_wheel_y(1, MouseWheelDirection::Normal), 1);
        assert_eq!(normalized_wheel_y(-2, MouseWheelDirection::Normal), -2);
        assert_eq!(normalized_wheel_y(3, MouseWheelDirection::Flipped), -3);
        assert_eq!(normalized_wheel_y(4, MouseWheelDirection::Unknown(99)), 4);
    }

    #[test]
    fn screen_pan_maps_one_for_one_into_projection_pixels() {
        let mut preview = PreviewCamera::default();
        preview.pan_by_pixels(20, 10);

        let camera = preview.render_camera(800, 600);
        assert!((camera.projection_offset[0] - -0.05).abs() < 1e-6);
        let expected_y = TOOLBAR_HEIGHT as f32 / 600.0 + 20.0 / 600.0;
        assert!((camera.projection_offset[1] - expected_y).abs() < 1e-6);
    }

    #[test]
    fn reset_position_only_clears_screen_pan() {
        let mut camera = PreviewCamera::default();
        camera.orbit_by_pixels(30, -20);
        camera.zoom_in();
        camera.pan_by_pixels(12, -9);
        let pose = (camera.pitch, camera.orbit_yaw, camera.distance);

        camera.reset_pan();

        assert!(!camera.is_panned());
        assert_eq!((camera.pitch, camera.orbit_yaw, camera.distance), pose);
    }

    #[test]
    fn framing_a_new_model_resets_pan_even_without_geometry() {
        let mut camera = PreviewCamera::default();
        camera.pan_by_pixels(12, -9);
        camera.frame_model(Some(3.0));
        assert!(!camera.is_panned());
        assert!((camera.distance - 8.4).abs() < 1e-6);

        camera.pan_by_pixels(-5, 7);
        camera.frame_model(None);
        assert!(!camera.is_panned());
    }
}
