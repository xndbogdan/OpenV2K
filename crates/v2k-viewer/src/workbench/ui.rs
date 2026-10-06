//! Small software-rendered controls used by the OpenGL asset workbench.

use sdl2::rect::{Point, Rect};
use v2k_render::Renderer;

pub(super) const TOOLBAR_HEIGHT: i32 = 88;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ToolbarTab {
    Models,
    Assemblies,
    EntityTypes,
    Sprites,
    Maps,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UiAction {
    SelectModels,
    SelectAssemblies,
    SelectEntityTypes,
    SelectSprites,
    SelectMaps,
    Previous,
    Next,
    CatalogPosition,
    Search,
    ToggleTextures,
    ToggleShadows,
    NextMapLayer,
    NextEntityModelSlot,
    ResetPosition,
}

impl UiAction {
    pub(super) fn is_enabled(
        self,
        model_preview_active: bool,
        entity_types_active: bool,
        maps_active: bool,
        can_reset_position: bool,
    ) -> bool {
        match self {
            Self::ToggleTextures | Self::ToggleShadows => model_preview_active,
            Self::NextEntityModelSlot => entity_types_active,
            Self::NextMapLayer => maps_active,
            Self::ResetPosition => can_reset_position,
            _ => true,
        }
    }
}

pub(super) struct UiRects {
    models: Rect,
    assemblies: Rect,
    entity_types: Rect,
    sprites: Rect,
    maps: Rect,
    previous: Rect,
    next: Rect,
    catalog_position: Rect,
    search: Rect,
    textures: Rect,
    shadows: Rect,
    layer: Rect,
    entity_model_slot: Rect,
    reset_position: Rect,
}

impl UiRects {
    pub(super) fn new() -> Self {
        Self {
            models: Rect::new(8, 7, 72, 25),
            assemblies: Rect::new(86, 7, 96, 25),
            entity_types: Rect::new(188, 7, 96, 25),
            sprites: Rect::new(290, 7, 78, 25),
            maps: Rect::new(374, 7, 66, 25),
            previous: Rect::new(452, 7, 68, 25),
            next: Rect::new(526, 7, 58, 25),
            catalog_position: Rect::new(596, 7, 142, 25),
            search: Rect::new(746, 7, 260, 25),
            textures: Rect::new(8, 40, 110, 24),
            shadows: Rect::new(124, 40, 106, 24),
            entity_model_slot: Rect::new(236, 40, 120, 24),
            layer: Rect::new(362, 40, 156, 24),
            reset_position: Rect::new(1014, 7, 116, 25),
        }
    }

    fn controls(&self) -> [(Rect, UiAction); 14] {
        [
            (self.models, UiAction::SelectModels),
            (self.assemblies, UiAction::SelectAssemblies),
            (self.entity_types, UiAction::SelectEntityTypes),
            (self.sprites, UiAction::SelectSprites),
            (self.maps, UiAction::SelectMaps),
            (self.previous, UiAction::Previous),
            (self.next, UiAction::Next),
            (self.catalog_position, UiAction::CatalogPosition),
            (self.search, UiAction::Search),
            (self.textures, UiAction::ToggleTextures),
            (self.shadows, UiAction::ToggleShadows),
            (self.layer, UiAction::NextMapLayer),
            (self.entity_model_slot, UiAction::NextEntityModelSlot),
            (self.reset_position, UiAction::ResetPosition),
        ]
    }

    pub(super) fn action_at(&self, point: Point) -> Option<UiAction> {
        self.controls()
            .into_iter()
            .find_map(|(rect, action)| rect.contains_point(point).then_some(action))
    }
}

pub(super) struct ToolbarView<'a> {
    pub(super) active_tab: ToolbarTab,
    pub(super) query: &'a str,
    pub(super) search_active: bool,
    pub(super) catalog_label: &'a str,
    pub(super) catalog_position: Option<usize>,
    pub(super) catalog_count: usize,
    pub(super) catalog_position_input: &'a str,
    pub(super) catalog_position_active: bool,
    pub(super) model_preview_active: bool,
    pub(super) entity_types_active: bool,
    pub(super) entity_model_slot: usize,
    pub(super) show_textures: bool,
    pub(super) show_shadows: bool,
    pub(super) can_reset_position: bool,
    pub(super) map_layer: &'a str,
    pub(super) info: &'a str,
    pub(super) status: &'a str,
}

pub(super) fn render_toolbar(renderer: &mut dyn Renderer, width: u32, view: ToolbarView<'_>) {
    let mut surface = UiSurface::new(width, TOOLBAR_HEIGHT as u32, [34, 38, 50, 255]);
    let ui = UiRects::new();
    draw_button(
        &mut surface,
        ui.models,
        "F1 MODELS",
        ButtonState::active(view.active_tab == ToolbarTab::Models),
    );
    draw_button(
        &mut surface,
        ui.assemblies,
        "F2 ASSEMBLY",
        ButtonState::active(view.active_tab == ToolbarTab::Assemblies),
    );
    draw_button(
        &mut surface,
        ui.entity_types,
        "F3 TYPES",
        ButtonState::active(view.active_tab == ToolbarTab::EntityTypes),
    );
    draw_button(
        &mut surface,
        ui.sprites,
        "F4 SPRITES",
        ButtonState::active(view.active_tab == ToolbarTab::Sprites),
    );
    draw_button(
        &mut surface,
        ui.maps,
        "F5 MAPS",
        ButtonState::active(view.active_tab == ToolbarTab::Maps),
    );
    draw_button(&mut surface, ui.previous, "< PREV", ButtonState::Normal);
    draw_button(&mut surface, ui.next, "NEXT >", ButtonState::Normal);
    draw_button(
        &mut surface,
        ui.catalog_position,
        &catalog_position_label(&view),
        if view.catalog_position.is_none() {
            ButtonState::Disabled
        } else {
            ButtonState::active(view.catalog_position_active)
        },
    );
    draw_button(
        &mut surface,
        ui.search,
        &format!(
            "FIND: {}{}",
            view.query,
            if view.search_active { "_" } else { "" }
        ),
        ButtonState::active(view.search_active),
    );
    draw_button(
        &mut surface,
        ui.reset_position,
        "RESET POSITION",
        ButtonState::enabled(view.can_reset_position),
    );
    draw_checkbox(
        &mut surface,
        ui.textures,
        "TEXTURES",
        view.show_textures,
        view.model_preview_active,
    );
    draw_checkbox(
        &mut surface,
        ui.shadows,
        "SHADOWS",
        view.show_shadows,
        view.model_preview_active,
    );
    draw_button(
        &mut surface,
        ui.entity_model_slot,
        &format!("SLOT {}/4  V", view.entity_model_slot + 1),
        if view.entity_types_active {
            ButtonState::Active
        } else {
            ButtonState::Disabled
        },
    );
    draw_button(
        &mut surface,
        ui.layer,
        &format!("LAYER: {}", view.map_layer),
        if view.active_tab == ToolbarTab::Maps {
            ButtonState::Active
        } else {
            ButtonState::Disabled
        },
    );
    draw_text(
        &mut surface,
        8,
        74,
        1,
        &format!("{}  {}", view.info, view.status),
        [220, 225, 235, 255],
    );
    surface.draw(renderer, 0, 0);
}

fn catalog_position_label(view: &ToolbarView<'_>) -> String {
    if view.catalog_position_active {
        format!(
            "{} {}_/{count}",
            view.catalog_label,
            view.catalog_position_input,
            count = view.catalog_count
        )
    } else if let Some(position) = view.catalog_position {
        format!("{} {position}/{}", view.catalog_label, view.catalog_count)
    } else {
        format!("{} -/{}", view.catalog_label, view.catalog_count)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ButtonState {
    Normal,
    Active,
    Disabled,
}

impl ButtonState {
    fn active(active: bool) -> Self {
        if active {
            Self::Active
        } else {
            Self::Normal
        }
    }

    fn enabled(enabled: bool) -> Self {
        if enabled {
            Self::Normal
        } else {
            Self::Disabled
        }
    }
}

fn draw_button(surface: &mut UiSurface, rect: Rect, label: &str, state: ButtonState) {
    let (fill, border, text) = match state {
        ButtonState::Normal => (
            [54, 59, 74, 255],
            [105, 112, 132, 255],
            [235, 238, 245, 255],
        ),
        ButtonState::Active => (
            [62, 108, 128, 255],
            [160, 225, 240, 255],
            [235, 238, 245, 255],
        ),
        ButtonState::Disabled => ([40, 43, 52, 255], [68, 72, 84, 255], [105, 110, 124, 255]),
    };
    surface.fill_rect(rect, fill);
    surface.draw_border(rect, border);
    draw_text(surface, rect.x() + 6, rect.y() + 7, 1, label, text);
}

fn draw_checkbox(surface: &mut UiSurface, rect: Rect, label: &str, checked: bool, enabled: bool) {
    let box_rect = Rect::new(rect.x(), rect.y() + 4, 15, 15);
    surface.fill_rect(box_rect, [24, 27, 36, 255]);
    surface.draw_border(
        box_rect,
        if enabled {
            [130, 142, 166, 255]
        } else {
            [68, 72, 84, 255]
        },
    );
    if checked && enabled {
        draw_text(
            surface,
            box_rect.x() + 5,
            box_rect.y() + 4,
            1,
            "X",
            [100, 220, 150, 255],
        );
    }
    draw_text(
        surface,
        rect.x() + 21,
        rect.y() + 8,
        1,
        label,
        if enabled {
            [220, 225, 235, 255]
        } else {
            [105, 110, 124, 255]
        },
    );
}

pub(super) struct UiSurface {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl UiSurface {
    pub(super) fn new(width: u32, height: u32, color: [u8; 4]) -> Self {
        let mut pixels = vec![0; width as usize * height as usize * 4];
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.copy_from_slice(&color);
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    pub(super) fn draw(&self, renderer: &mut dyn Renderer, x: i32, y: i32) {
        renderer.draw_sprite(&self.pixels, self.width, self.height, x, y);
    }

    fn fill_rect(&mut self, rect: Rect, color: [u8; 4]) {
        let left = rect.x().clamp(0, self.width as i32);
        let top = rect.y().clamp(0, self.height as i32);
        let right = rect.right().clamp(left, self.width as i32);
        let bottom = rect.bottom().clamp(top, self.height as i32);
        for y in top..bottom {
            for x in left..right {
                self.set_pixel(x as u32, y as u32, color);
            }
        }
    }

    fn draw_border(&mut self, rect: Rect, color: [u8; 4]) {
        if rect.width() == 0 || rect.height() == 0 {
            return;
        }
        self.fill_rect(Rect::new(rect.x(), rect.y(), rect.width(), 1), color);
        self.fill_rect(
            Rect::new(rect.x(), rect.bottom() - 1, rect.width(), 1),
            color,
        );
        self.fill_rect(Rect::new(rect.x(), rect.y(), 1, rect.height()), color);
        self.fill_rect(
            Rect::new(rect.right() - 1, rect.y(), 1, rect.height()),
            color,
        );
    }

    pub(super) fn draw_checkerboard(&mut self, rect: Rect) {
        const CELL: i32 = 12;
        for y in (rect.y()..rect.bottom()).step_by(CELL as usize) {
            for x in (rect.x()..rect.right()).step_by(CELL as usize) {
                let alternate = ((x - rect.x()) / CELL + (y - rect.y()) / CELL) & 1 != 0;
                let width = CELL.min(rect.right() - x).max(0) as u32;
                let height = CELL.min(rect.bottom() - y).max(0) as u32;
                self.fill_rect(
                    Rect::new(x, y, width, height),
                    if alternate {
                        [75, 75, 82, 255]
                    } else {
                        [105, 105, 112, 255]
                    },
                );
            }
        }
    }

    pub(super) fn blit_scaled(
        &mut self,
        source: &[u8],
        source_width: u32,
        source_height: u32,
        dest: Rect,
    ) {
        if source_width == 0
            || source_height == 0
            || dest.width() == 0
            || dest.height() == 0
            || source.len() < source_width as usize * source_height as usize * 4
        {
            return;
        }
        for destination_y in 0..dest.height() {
            let source_y = destination_y as usize * source_height as usize / dest.height() as usize;
            for destination_x in 0..dest.width() {
                let source_x =
                    destination_x as usize * source_width as usize / dest.width() as usize;
                let source_offset = (source_y * source_width as usize + source_x) * 4;
                let target_x = dest.x() + destination_x as i32;
                let target_y = dest.y() + destination_y as i32;
                if target_x < 0
                    || target_y < 0
                    || target_x >= self.width as i32
                    || target_y >= self.height as i32
                {
                    continue;
                }
                self.blend_pixel(
                    target_x as u32,
                    target_y as u32,
                    source[source_offset..source_offset + 4]
                        .try_into()
                        .expect("four-byte RGBA pixel"),
                );
            }
        }
    }

    fn set_pixel(&mut self, x: u32, y: u32, color: [u8; 4]) {
        if x >= self.width || y >= self.height {
            return;
        }
        let offset = (y as usize * self.width as usize + x as usize) * 4;
        self.pixels[offset..offset + 4].copy_from_slice(&color);
    }

    fn blend_pixel(&mut self, x: u32, y: u32, source: [u8; 4]) {
        if x >= self.width || y >= self.height {
            return;
        }
        let offset = (y as usize * self.width as usize + x as usize) * 4;
        let destination = &mut self.pixels[offset..offset + 4];
        let source_alpha = u32::from(source[3]);
        let destination_alpha = u32::from(destination[3]);
        let inverse = 255 - source_alpha;
        let output_alpha = source_alpha + destination_alpha * inverse / 255;
        if output_alpha == 0 {
            destination.copy_from_slice(&[0; 4]);
            return;
        }
        for channel in 0..3 {
            let premultiplied = u32::from(source[channel]) * source_alpha
                + u32::from(destination[channel]) * destination_alpha * inverse / 255;
            destination[channel] = (premultiplied / output_alpha) as u8;
        }
        destination[3] = output_alpha as u8;
    }
}

pub(super) fn fit_rect(
    source_width: u32,
    source_height: u32,
    bounds: Rect,
    max_scale: f32,
) -> Rect {
    let scale = (bounds.width() as f32 / source_width.max(1) as f32)
        .min(bounds.height() as f32 / source_height.max(1) as f32)
        .min(max_scale);
    let width = (source_width as f32 * scale).max(1.0) as u32;
    let height = (source_height as f32 * scale).max(1.0) as u32;
    Rect::new(
        bounds.x() + (bounds.width().saturating_sub(width) / 2) as i32,
        bounds.y() + (bounds.height().saturating_sub(height) / 2) as i32,
        width,
        height,
    )
}

fn draw_text(surface: &mut UiSurface, x: i32, y: i32, scale: i32, text: &str, color: [u8; 4]) {
    let mut cursor = x;
    for character in text.chars() {
        let glyph = glyph(character);
        for (row, bits) in glyph.iter().enumerate() {
            for column in 0..5 {
                if bits & (1 << (4 - column)) != 0 {
                    surface.fill_rect(
                        Rect::new(
                            cursor + column * scale,
                            y + row as i32 * scale,
                            scale as u32,
                            scale as u32,
                        ),
                        color,
                    );
                }
            }
        }
        cursor += 6 * scale;
    }
}

fn glyph(character: char) -> [u8; 7] {
    match character.to_ascii_uppercase() {
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [31, 4, 4, 4, 4, 4, 31],
        'J' => [7, 2, 2, 2, 18, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 21, 19, 17, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        '<' => [2, 4, 8, 16, 8, 4, 2],
        '>' => [8, 4, 2, 1, 2, 4, 8],
        ':' => [0, 4, 4, 0, 4, 4, 0],
        '/' => [1, 2, 2, 4, 8, 8, 16],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        '_' => [0, 0, 0, 0, 0, 0, 31],
        '.' => [0, 0, 0, 0, 0, 6, 6],
        '[' => [14, 8, 8, 8, 8, 8, 14],
        ']' => [14, 2, 2, 2, 2, 2, 14],
        _ => [0; 7],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolbar_hit_testing_returns_semantic_actions() {
        let ui = UiRects::new();
        assert_eq!(
            ui.action_at(Point::new(10, 10)),
            Some(UiAction::SelectModels)
        );
        assert_eq!(
            ui.action_at(Point::new(100, 10)),
            Some(UiAction::SelectAssemblies)
        );
        assert_eq!(
            ui.action_at(Point::new(200, 10)),
            Some(UiAction::SelectEntityTypes)
        );
        assert_eq!(ui.action_at(Point::new(530, 10)), Some(UiAction::Next));
        assert_eq!(
            ui.action_at(Point::new(600, 10)),
            Some(UiAction::CatalogPosition)
        );
        assert_eq!(
            ui.action_at(Point::new(140, 45)),
            Some(UiAction::ToggleShadows)
        );
        assert_eq!(
            ui.action_at(Point::new(1020, 10)),
            Some(UiAction::ResetPosition)
        );
        assert_eq!(
            ui.action_at(Point::new(250, 45)),
            Some(UiAction::NextEntityModelSlot)
        );
        assert_eq!(ui.action_at(Point::new(800, 70)), None);
    }

    #[test]
    fn toolbar_controls_fit_without_overlap_at_the_default_width() {
        let controls = UiRects::new().controls();
        for (index, (rect, _)) in controls.iter().enumerate() {
            assert!(rect.x() >= 0);
            assert!(rect.y() >= 0);
            assert!(rect.right() <= 1200);
            assert!(rect.bottom() <= TOOLBAR_HEIGHT);
            for (other, _) in &controls[index + 1..] {
                let separated = rect.right() <= other.left()
                    || other.right() <= rect.left()
                    || rect.bottom() <= other.top()
                    || other.bottom() <= rect.top();
                assert!(
                    separated,
                    "toolbar controls overlap: {rect:?} and {other:?}"
                );
            }
        }
    }

    #[test]
    fn reset_position_action_is_disabled_until_the_model_is_panned() {
        assert!(!UiAction::ResetPosition.is_enabled(true, false, false, false));
        assert!(UiAction::ResetPosition.is_enabled(true, false, false, true));
        assert!(UiAction::Next.is_enabled(false, false, false, false));
        assert!(UiAction::CatalogPosition.is_enabled(false, false, false, false));
        assert!(!UiAction::NextEntityModelSlot.is_enabled(true, false, false, false));
        assert!(UiAction::NextEntityModelSlot.is_enabled(true, true, false, false));
        assert!(!UiAction::NextMapLayer.is_enabled(false, false, false, false));
        assert!(UiAction::NextMapLayer.is_enabled(false, false, true, false));
        assert_eq!(ButtonState::enabled(false), ButtonState::Disabled);
        assert_eq!(ButtonState::enabled(true), ButtonState::Normal);
    }

    #[test]
    fn catalog_position_field_uses_the_active_catalog_label() {
        let mut view = ToolbarView {
            active_tab: ToolbarTab::Models,
            query: "",
            search_active: false,
            catalog_label: "MODEL",
            catalog_position: Some(37),
            catalog_count: 760,
            catalog_position_input: "",
            catalog_position_active: false,
            model_preview_active: true,
            entity_types_active: false,
            entity_model_slot: 0,
            show_textures: true,
            show_shadows: true,
            can_reset_position: false,
            map_layer: "HEIGHT",
            info: "",
            status: "",
        };
        assert_eq!(catalog_position_label(&view), "MODEL 37/760");

        view.catalog_position_input = "500";
        view.catalog_position_active = true;
        assert_eq!(catalog_position_label(&view), "MODEL 500_/760");

        view.active_tab = ToolbarTab::EntityTypes;
        view.catalog_label = "TYPE";
        view.catalog_position = None;
        view.catalog_position_active = false;
        assert_eq!(catalog_position_label(&view), "TYPE -/760");
    }

    #[test]
    fn scaled_rgba_blit_preserves_opaque_source_color() {
        let mut surface = UiSurface::new(2, 2, [0, 0, 0, 255]);
        surface.blit_scaled(&[10, 20, 30, 255], 1, 1, Rect::new(0, 0, 2, 2));
        assert!(surface
            .pixels
            .chunks_exact(4)
            .all(|pixel| pixel == [10, 20, 30, 255]));
    }

    #[test]
    fn fit_rect_centres_without_exceeding_scale_limit() {
        assert_eq!(
            fit_rect(10, 5, Rect::new(20, 30, 100, 100), 2.0),
            Rect::new(60, 75, 20, 10)
        );
    }

    #[test]
    fn glyphs_have_seven_rows() {
        assert_eq!(glyph('A').len(), 7);
        assert_ne!(glyph('A'), [0; 7]);
    }
}
