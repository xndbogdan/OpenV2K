//! Exercise the production HUD submission, including face-free model roots.

use super::*;
use v2k_formats::{system::PaletteEntry, terrain::TerrainGrid};
use v2k_game::model_color::ModelMaterialCache;
use v2k_render::{Camera, ModelDraw, Renderer, UiSubmissionPolicy, WorldSpriteBlend};

#[derive(Debug, Clone, Copy, PartialEq)]
struct RecordedCamera {
    origin: [f32; 2],
    focal_pixels: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RecordedSprite {
    rect: (i32, i32, u32, u32),
    clip: Option<(i32, i32, u32, u32)>,
}

struct ModelRecorder {
    viewport: [u32; 2],
    policy: UiSubmissionPolicy,
    visible_bodies: Vec<(usize, usize)>,
    cameras: Vec<RecordedCamera>,
    model_bounds: Vec<[f32; 4]>,
    sprites: Vec<RecordedSprite>,
    material_sprites: Vec<RecordedSprite>,
    clip: Option<(i32, i32, u32, u32)>,
}

impl Default for ModelRecorder {
    fn default() -> Self {
        Self {
            viewport: [640, 480],
            policy: UiSubmissionPolicy::FitAuthoredCanvas,
            visible_bodies: Vec::new(),
            cameras: Vec::new(),
            model_bounds: Vec::new(),
            sprites: Vec::new(),
            material_sprites: Vec::new(),
            clip: None,
        }
    }
}

impl Renderer for ModelRecorder {
    fn backend_name(&self) -> &str {
        "HUD submission recorder"
    }
    fn clear(&mut self, _: f32, _: f32, _: f32) {}
    fn present(&mut self) {}
    fn resize(&mut self, width: u32, height: u32) {
        self.viewport = [width, height];
    }
    fn set_camera(&mut self, camera: &Camera) {
        let [width, height] = self.viewport.map(|value| value as f32);
        let projection = camera.projection_matrix();
        self.cameras.push(RecordedCamera {
            origin: [
                (1.0 - camera.projection_offset[0]) * width * 0.5,
                (1.0 + camera.projection_offset[1]) * height * 0.5,
            ],
            focal_pixels: [projection[0] * width * 0.5, projection[5] * height * 0.5],
        });
    }
    fn set_fog(&mut self, _: bool, _: f32, _: f32, _: [f32; 3]) {}
    fn draw_terrain(
        &mut self,
        _: &TerrainGrid,
        _: &[PaletteEntry],
        _: Option<&v2k_render::terrain_tiles::TerrainFrames>,
        _: Option<&v2k_render::terrain_light::TerrainLightWindow>,
        _: u32,
    ) {
    }
    fn draw_model_body(&mut self, draw: ModelDraw<'_>) {
        if !draw.mesh.triangles.is_empty() {
            self.visible_bodies
                .push((draw.mesh.vertices.len(), draw.mesh.triangles.len()));
            let camera = *self.cameras.last().unwrap();
            let mut bounds = [
                f32::INFINITY,
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::NEG_INFINITY,
            ];
            for &index in draw.mesh.triangles.iter().flatten() {
                // HUD cameras look down -Z from the origin. Convert the
                // submitted raw-local mesh through its model transform before
                // applying this camera's pixel lens; do not project locals.
                let local = draw.mesh.vertices[usize::from(index)]
                    .map(|value| value as f32 * draw.transform.scale / 100.0);
                let view = mat3_apply(draw.transform.orientation, local);
                let view = std::array::from_fn::<_, 3, _>(|axis| {
                    view[axis] + draw.transform.position[axis]
                });
                let depth = -view[2];
                assert!(depth > 0.0);
                let pixel = [
                    camera.origin[0] + camera.focal_pixels[0] * view[0] / depth,
                    camera.origin[1] - camera.focal_pixels[1] * view[1] / depth,
                ];
                for axis in 0..2 {
                    bounds[axis] = bounds[axis].min(pixel[axis]);
                    bounds[axis + 2] = bounds[axis + 2].max(pixel[axis]);
                }
            }
            self.model_bounds.push(bounds);
        }
    }
    fn set_sprite_clip(&mut self, clip: Option<(i32, i32, u32, u32)>) {
        self.clip = clip;
    }
    fn draw_sprite(&mut self, _: &[u8], width: u32, height: u32, x: i32, y: i32) {
        self.sprites.push(RecordedSprite {
            rect: (x, y, width, height),
            clip: self.clip,
        });
    }
    fn draw_material_sprite(
        &mut self,
        _: &[u8],
        width: u32,
        height: u32,
        x: i32,
        y: i32,
        _: WorldSpriteBlend,
    ) {
        self.material_sprites.push(RecordedSprite {
            rect: (x, y, width, height),
            clip: self.clip,
        });
    }
    fn draw_fullscreen(&mut self, _: &[u8], _: u32, _: u32) {}
    fn draw_color_overlay(&mut self, _: f32, _: f32, _: f32, _: f32) {}
    fn viewport_size(&self) -> (u32, u32) {
        (self.viewport[0], self.viewport[1])
    }
    fn ui_submission_policy(&self) -> UiSubmissionPolicy {
        self.policy
    }
}

#[v2k_test_support::retail_test]
fn active_trophy_submits_its_visible_child_and_expiry_removes_the_whole_hierarchy() {
    let root = v2k_test_support::retail_dir();
    assert!(root.join("PRELOAD.DAT").exists(), "retail corpus required");
    let mut session = v2k_game::session::GameSession::init(&root).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(15, 1).unwrap();
    let resources = GameplayHudResources::from_cache(&session.cache).unwrap();
    let layout = GameplayHudLayout::from_cache(&session.cache, 1).unwrap();
    let colors = ModelMaterialCache::new();
    let mut hud = GameplayHud::default();
    let roster = GameplayHudWeaponRoster::default();
    assert_eq!(roster.selected().unwrap().global_model_id, Some(115));
    let weapon = session.cache.global_model(115).unwrap();
    assert_eq!(weapon.name.as_deref(), Some("hudweap"));
    assert_eq!((weapon.vertices.len(), weapon.triangles.len()), (16, 28));
    let mut progress = PlayerCampaignProgress::new();
    progress.set_current_control_slot(Some(3));
    let (mut timer, _) =
        v2k_game::time_trophy::TimeTrophyRuntime::from_loaded_world(300, false, &mut progress)
            .unwrap();
    for active in [true, false] {
        if !active {
            timer.advance(300_500_000);
        }
        let frame = hud.frame(
            &resources,
            layout,
            200_000,
            40_000,
            222,
            0,
            &roster,
            &[],
            timer,
        );
        let mut renderer = ModelRecorder::default();
        draw_gameplay_hud(
            &mut renderer,
            &session.cache,
            &colors,
            &resources,
            &frame,
            None,
        );
        // Model138 itself has no faces. The 52-vertex/91-triangle submission
        // is its actual authored child139, reached through the HUD callback.
        // Default inventory also draws model115's independent static weapon;
        // that body remains present when the trophy hierarchy expires.
        assert_eq!(
            renderer.visible_bodies,
            if active {
                vec![(52, 91), (16, 28)]
            } else {
                vec![(16, 28)]
            }
        );
    }
}

/// Check physical edge distances rather than reconstructing the mapping policy.
fn assert_pixel_distance(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 2.1,
        "pixel distance {actual} differs from {expected}"
    );
}

#[v2k_test_support::retail_test]
fn modern_time_trophy_keeps_normal_high_art_bounds_visible_through_a_full_spin() {
    use v2k_game::gameplay_hud::GameplayHudTimeTrophyFrame;

    let root = v2k_test_support::retail_dir();
    assert!(root.join("PRELOAD.DAT").exists(), "retail corpus required");
    let mut session = v2k_game::session::GameSession::init(&root).unwrap();
    let colors = ModelMaterialCache::new();
    let mut progress = PlayerCampaignProgress::new();
    progress.set_current_control_slot(Some(2));
    let (timer, _) =
        v2k_game::time_trophy::TimeTrophyRuntime::from_loaded_world(270, false, &mut progress)
            .unwrap();
    let mut normal_art_bounds = Vec::new();
    for variant in 1..=3 {
        session.load_auxiliary_ovl(3, variant).unwrap();
        let layout = GameplayHudLayout::from_cache(&session.cache, variant).unwrap();
        for spin in (0..65536).step_by(4096) {
            let mut trophy =
                GameplayHudTimeTrophyFrame::from_runtime(timer, layout.time_trophy, 0, 2).unwrap();
            trophy.spin_raw = spin as u16;
            let vars = trophy.animation_vars();
            for viewport in [[640, 480], [1920, 1080], [3840, 2160]] {
                // Only tier 1 supplies the original high-art reference. The
                // original tier-2/3 lenses are covered by the submission test.
                if viewport == [640, 480] && variant != 1 {
                    continue;
                }
                let mut renderer = ModelRecorder {
                    viewport,
                    ..ModelRecorder::default()
                };
                assert!(draw_gameplay_hud_model(
                    &mut renderer,
                    &session.cache,
                    &colors,
                    trophy.global_model_id,
                    trophy.model_position,
                    trophy.orientation(),
                    trophy.depth_raw(),
                    GameplayHudModelMaterialization::Linked(&vars),
                    GameplayHudModelView::time_trophy(UiMappingRequest {
                        viewport,
                        authored_canvas: [layout.virtual_width, layout.virtual_height],
                        policy: UiSubmissionPolicy::NativeHud {
                            anchor: UiAnchor::TopLeft,
                        },
                    }),
                ));
                assert_eq!(renderer.visible_bodies, [(52, 91)]);
                let bounds = renderer.model_bounds[0];
                assert!(
                    bounds[1] >= 0.0,
                    "tier {variant}, {viewport:?}, spin {spin}: {bounds:?}"
                );
                let camera = renderer.cameras[0];
                let relative = [
                    bounds[0] - camera.origin[0],
                    bounds[1] - camera.origin[1],
                    bounds[2] - camera.origin[0],
                    bounds[3] - camera.origin[1],
                ];
                if viewport == [640, 480] {
                    normal_art_bounds.push(relative);
                } else {
                    let scale = if viewport == [1920, 1080] { 1.8 } else { 3.6 };
                    let expected = normal_art_bounds[spin / 4096];
                    for axis in 0..4 {
                        assert!((relative[axis] / scale - expected[axis]).abs() < 0.002);
                    }
                }
            }
        }
    }
}

#[v2k_test_support::retail_test]
fn native_hud_submission_keeps_corner_groups_and_clips_aligned_after_fullscreen() {
    use v2k_game::{
        gameplay_hud::GameplayHudSprite,
        gameplay_radar::{GameplayRadarLayout, RadarImage},
        menu_text::MenuFonts,
    };

    let root = v2k_test_support::retail_dir();
    assert!(root.join("PRELOAD.DAT").exists(), "retail corpus required");
    let mut session = v2k_game::session::GameSession::init(&root).unwrap();
    let colors = ModelMaterialCache::new();
    let roster = GameplayHudWeaponRoster::default();
    for variant in 1..=3 {
        session.load_auxiliary_ovl(3, variant).unwrap();
        let resources = GameplayHudResources::from_cache(&session.cache).unwrap();
        let layout = GameplayHudLayout::from_cache(&session.cache, variant).unwrap();
        let radar = GameplayRadarLayout::from_cache(&session.cache, variant).unwrap();
        let menu_level = session.load_ovl_by_id(2, variant).unwrap();
        let fonts = MenuFonts::from_level(&menu_level).unwrap();
        let canvas = [layout.virtual_width, layout.virtual_height];
        let mut progress = PlayerCampaignProgress::new();
        progress.set_current_control_slot(Some(3));
        let (timer, _) =
            v2k_game::time_trophy::TimeTrophyRuntime::from_loaded_world(300, false, &mut progress)
                .unwrap();
        // An occupied slot uses the known resident weapon mesh solely to test
        // the linked cargo submission camera; actor behavior is not exercised.
        let frame = GameplayHud::default().frame(
            &resources,
            layout,
            100_000,
            20_000,
            222,
            0,
            &roster,
            &[Some(115)],
            timer,
        );
        let trophy = frame.time_trophy.as_ref().unwrap();
        let mut original: Option<(
            Vec<RecordedCamera>,
            Vec<RecordedSprite>,
            Vec<RecordedSprite>,
        )> = None;
        for (viewport, scale) in [
            (canvas, 1.0),
            ([1920, 1080], 1.8),
            ([3840, 2160], 3.6),
            (canvas, 1.0),
        ] {
            let mut renderer = ModelRecorder {
                viewport,
                policy: UiSubmissionPolicy::NativeCanvas,
                ..ModelRecorder::default()
            };
            draw_gameplay_hud(
                &mut renderer,
                &session.cache,
                &colors,
                &resources,
                &frame,
                Some(&fonts.normal),
            );
            assert_eq!(
                renderer.cameras.len(),
                3,
                "trophy, cargo and weapon cameras"
            );
            assert_eq!(renderer.material_sprites.len(), 1 + frame.layers.len());
            assert_eq!(renderer.clip, None, "HUD drawing restores unclipped output");

            // The trophy and first clock glyph retain their top-left relation.
            let trophy_camera = renderer.cameras[0];
            assert_pixel_distance(
                trophy_camera.origin[0],
                trophy.model_position.x as f32 * scale,
            );
            assert_pixel_distance(
                trophy_camera.origin[1],
                trophy.model_position.y as f32 * scale,
            );
            let glyph = fonts
                .normal
                .glyph(trophy.text.chars().next().unwrap())
                .unwrap();
            let clock = renderer.sprites.first().unwrap();
            assert_pixel_distance(
                clock.rect.0 as f32,
                (trophy.text_position.x as f32 + glyph.xoff) * scale,
            );
            assert_pixel_distance(
                clock.rect.1 as f32,
                (trophy.text_position.y as f32 + glyph.yoff - glyph.height as f32 + 1.0) * scale,
            );

            // Both foreground models preserve the same left/bottom insets as
            // the cargo marker and the later status-orb sprite composition.
            for (camera, point) in [
                (renderer.cameras[1], frame.cargo[0].model_position),
                (renderer.cameras[2], frame.weapons[0].model_position),
            ] {
                assert_pixel_distance(camera.origin[0], point.x as f32 * scale);
                assert_pixel_distance(
                    viewport[1] as f32 - camera.origin[1],
                    (canvas[1] as i32 - point.y) as f32 * scale,
                );
            }
            let marker = resources.sprite(GameplayHudSprite::CargoEmpty);
            let marker_draw = renderer.material_sprites[0];
            assert_pixel_distance(
                marker_draw.rect.0 as f32,
                frame.cargo[0].marker_position.x as f32 * scale,
            );
            assert_pixel_distance(
                viewport[1] as f32 - marker_draw.rect.1 as f32 - marker_draw.rect.3 as f32,
                (canvas[1] as i32 - frame.cargo[0].marker_position.y - marker.height as i32) as f32
                    * scale,
            );
            for (layer, draw) in frame.layers.iter().zip(&renderer.material_sprites[1..]) {
                let sprite = resources.sprite(layer.sprite);
                assert_pixel_distance(draw.rect.0 as f32, layer.position.x as f32 * scale);
                assert_pixel_distance(
                    viewport[1] as f32 - draw.rect.1 as f32 - draw.rect.3 as f32,
                    (canvas[1] as i32 - layer.position.y - sprite.height as i32) as f32 * scale,
                );
                match (layer.clip, draw.clip) {
                    (None, None) => {}
                    (Some(authored), Some((x, y, width, height))) => {
                        assert_pixel_distance(x as f32, authored.x as f32 * scale);
                        assert_pixel_distance(
                            viewport[1] as f32 - y as f32 - height as f32,
                            (canvas[1] as i32 - authored.y - authored.height as i32) as f32 * scale,
                        );
                        assert_pixel_distance(width as f32, authored.width as f32 * scale);
                    }
                    _ => panic!("sprite clip lost its authored layer association"),
                }
            }
            if let Some((cameras, sprites, materials)) = &original {
                for (index, (camera, baseline)) in renderer.cameras.iter().zip(cameras).enumerate()
                {
                    // The modern trophy uses normal high-art pixel metrics;
                    // cargo and weapons retain the selected layout's lens.
                    let lens_scale = if index == 0 && viewport != canvas {
                        scale * 480.0 / canvas[1] as f32
                    } else {
                        scale
                    };
                    assert_pixel_distance(
                        camera.focal_pixels[0],
                        baseline.focal_pixels[0] * lens_scale,
                    );
                    assert_pixel_distance(
                        camera.focal_pixels[1],
                        baseline.focal_pixels[1] * lens_scale,
                    );
                }
                if viewport == canvas {
                    assert_eq!(
                        &renderer.cameras, cameras,
                        "return restores original model projection"
                    );
                    assert_eq!(&renderer.sprites, sprites);
                    assert_eq!(&renderer.material_sprites, materials);
                }
            } else {
                original = Some((
                    renderer.cameras.clone(),
                    renderer.sprites.clone(),
                    renderer.material_sprites.clone(),
                ));
            }

            // Radar uses the canonical tier's origin and globe size, while its
            // pixels are irrelevant to this production placement regression.
            let [width, height] = radar.globe_size.map(|value| value as u32);
            let radar_frame = RadarHudFrame {
                image: RadarImage {
                    rgba: vec![255; (width * height * 4) as usize],
                    width,
                    height,
                },
                origin: radar.hud_origin,
                virtual_size: radar.virtual_size,
            };
            let mut radar_renderer = ModelRecorder {
                viewport,
                policy: UiSubmissionPolicy::NativeCanvas,
                ..ModelRecorder::default()
            };
            draw_gameplay_radar(&mut radar_renderer, &radar_frame);
            assert_eq!(radar_renderer.sprites.len(), 1);
            let (x, y, drawn_width, drawn_height) = radar_renderer.sprites[0].rect;
            assert_pixel_distance(
                viewport[0] as f32 - x as f32 - drawn_width as f32,
                (canvas[0] as i32 - radar.hud_origin[0] - width as i32) as f32 * scale,
            );
            assert_pixel_distance(
                viewport[1] as f32 - y as f32 - drawn_height as f32,
                (canvas[1] as i32 - radar.hud_origin[1] - height as i32) as f32 * scale,
            );
            assert_pixel_distance(drawn_width as f32, width as f32 * scale);
            assert_pixel_distance(drawn_height as f32, height as f32 * scale);
        }
    }
}
