use super::*;
use std::hash::{Hash, Hasher};
use v2k_game::{
    entity::{EntityConstructionResources, EntityManager},
    entity_collision_state::EntityTypeRuntimeMetadata,
    frozen_world_presentation::{FrozenWorldPresentationSnapshot, FrozenWorldPresentationSource},
    gameplay_hud::{GameplayHud, GameplayHudLayerRole, GameplayHudWeaponRoster, HULL_FULL_RAW},
    resource_cache::ResourceCache,
    system_layout::{SystemLayoutOrigin, SystemLayoutSource},
    time_trophy::TimeTrophyRuntime,
};

fn fingerprint(value: &impl Hash) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

struct DisplayFixture {
    session: GameSession,
    fonts: Option<MenuFonts>,
    variant: Option<u32>,
    world_variant: u32,
    reference_size: (u32, u32),
    hud_layout: Option<GameplayHudLayout>,
    radar: Option<GameplayRadar>,
    map_status: Option<FullscreenMapStatusResources>,
    overlay: GameplayOverlayFrame,
    projection: WorldProjection,
    frozen_world: FrozenWorldPresentationSnapshot,
}

impl DisplayFixture {
    fn new() -> Self {
        let root = v2k_test_support::retail_dir();
        assert!(root.join("PRELOAD.DAT").is_file(), "retail corpus required");
        let mut session = GameSession::init(&root).unwrap();
        for system in [2, 3, 5, 51] {
            session.load_auxiliary_ovl(system, 1).unwrap();
        }
        session.load_level_by_id(13, 1).unwrap();
        let metadata = session
            .cache
            .global_entity_model_table()
            .iter()
            .enumerate()
            .map(|(kind, _)| {
                EntityTypeRuntimeMetadata::from_section12(
                    session.cache.global_entity_type(kind).unwrap(),
                )
            })
            .collect::<Vec<_>>();
        let mut fx = WorldFx::new();
        let entities = EntityManager::from_fresh_new_game_level_with_type_metadata(
            session.cache.level_desc().unwrap(),
            &metadata,
            EntityConstructionResources::new(
                session.cache.terrain(),
                session.cache.terrain_objects(),
            ),
            123,
            &mut fx,
        )
        .unwrap();
        session
            .cache
            .initialize_level_terrain_radar(&mut || fx.next_shared_retail_random_u16())
            .unwrap();
        let mut radar = GameplayRadar::from_cache(&session.cache, 1).unwrap();
        let radar_frame = radar
            .hud_frame(
                session.cache.level_terrain_radar().unwrap(),
                &entities,
                123,
                &mut || fx.next_shared_retail_random_u16(),
            )
            .unwrap();
        radar.enter_fullscreen();
        for _ in 0..4 {
            radar.advance_fullscreen(20_000);
        }
        assert!(radar
            .fullscreen_image(session.cache.level_terrain_radar().unwrap())
            .is_some());

        let hud_layout = GameplayHudLayout::from_cache(&session.cache, 1).unwrap();
        let resources = GameplayHudResources::from_cache(&session.cache).unwrap();
        let mut hud = GameplayHud::default();
        hud.advance_spins(12_345);
        let roster = GameplayHudWeaponRoster::default();
        hud.frame(
            &resources,
            hud_layout,
            v2k_game::player::FUEL_FULL_RAW,
            HULL_FULL_RAW,
            123,
            20_000,
            &roster,
            &[],
            TimeTrophyRuntime::default(),
        );
        let hud_frame = hud.frame(
            &resources,
            hud_layout,
            v2k_game::player::FUEL_FULL_RAW / 2,
            10_000,
            123,
            20_000,
            &roster,
            &[Some(9)],
            TimeTrophyRuntime::default(),
        );
        assert!(hud_frame
            .layers
            .iter()
            .any(|layer| layer.role == GameplayHudLayerRole::HullTrail));
        fx.emit_player_wreck_burst_raw([0; 3], 128, None, 46);
        let particles = fx.prepare_presentation([640, 480], 0x1800, |_| {
            v2k_render::ParticleCenterProjection {
                screen: [50, 50],
                depth_raw: 0x100,
                clip: 0,
            }
        });
        assert!(particles.particles().len() > 0);
        // Fresh Level1 construction has no Class0 adoption targets. The
        // presentation snapshot retains its paired scheduler as constructed.
        let tasks = SpecializedActorTaskScheduler::default();
        let frozen_world =
            FrozenWorldPresentationSnapshot::capture(FrozenWorldPresentationSource {
                entities: &entities,
                world_fx: &fx,
                specialized_actor_tasks: &tasks,
            });
        let fonts = MenuFonts::from_cache(&session.cache).unwrap();
        let map_status = FullscreenMapStatusResources::from_cache(&session.cache).unwrap();
        let projection = WorldProjection::from_cache(&session.cache).unwrap();
        Self {
            session,
            fonts: Some(fonts),
            variant: Some(1),
            world_variant: 1,
            reference_size: (640, 480),
            hud_layout: Some(hud_layout),
            radar: Some(radar),
            map_status: Some(map_status),
            overlay: GameplayOverlayFrame {
                full_frame_sprite: Some(FullFrameSpriteFrame {
                    table_index: 1,
                    global_sprite_id: 0x227,
                }),
                hud: Some(hud_frame),
                radar: Some(radar_frame),
                notifications: vec![GameplayNotificationLine {
                    string_id: 0xe6,
                    text: "Factory".into(),
                    x_percent: 20,
                    baseline_percent: 70,
                    width_percent: 60,
                    center_x: false,
                }],
                explosion_lights: vec![TerrainExplosionLight {
                    x_raw: 17,
                    z_raw: 19,
                    radius_raw: 400,
                }],
                static_explosion_lights: vec![TerrainExplosionLight {
                    x_raw: -17,
                    z_raw: -19,
                    radius_raw: 500,
                }],
                presentation: Some(GameplayWorldPresentation {
                    shield: Some(v2k_game::player_shield::PlayerShieldFrame {
                        spin_raw: 0x1234,
                        retail_tick: 123,
                        damage_flash_level: 3,
                    }),
                    particles,
                }),
            },
            projection,
            frozen_world,
        }
    }

    fn refresh(&mut self, tier: HighSystemLayoutTier) -> Result<bool, DisplayRefreshError> {
        ResidentDisplay {
            session: &mut self.session,
            fonts: &mut self.fonts,
            variant: &mut self.variant,
            world_variant: &mut self.world_variant,
            reference_size: &mut self.reference_size,
            hud_layout: &mut self.hud_layout,
            radar: &mut self.radar,
            map_status: &mut self.map_status,
            overlay: &mut self.overlay,
            projection: &mut self.projection,
        }
        .refresh(tier)
    }

    /// Intrinsic allocations and evaluated world commands have separate custody
    /// from layout tables. No glyph decoding or frozen-world capture may recur.
    fn retained_owners(&self) -> Vec<usize> {
        let cache = &self.session.cache;
        let mut addresses = vec![
            cache.level().unwrap() as *const _ as usize,
            cache.menu_ovl().unwrap() as *const _ as usize,
            &self.frozen_world as *const _ as usize,
            self.overlay.notifications.as_ptr() as usize,
            self.overlay.explosion_lights.as_ptr() as usize,
            self.overlay.static_explosion_lights.as_ptr() as usize,
            self.overlay
                .presentation
                .as_ref()
                .unwrap()
                .particles
                .particles()
                .next()
                .unwrap() as *const _ as usize,
        ];
        for id in cache.global_model_ids() {
            addresses.push(cache.global_model(id).unwrap() as *const _ as usize);
        }
        for id in cache.global_sprite_ids() {
            let (atlas, entry) = cache.global_sprite(id).unwrap();
            addresses.extend([atlas as *const _ as usize, entry as *const _ as usize]);
        }
        addresses.extend(
            cache
                .global_strings()
                .iter()
                .map(|text| text.as_ptr() as usize),
        );
        for selected in [false, true] {
            let font = self.fonts.as_ref().unwrap().font(selected);
            for code in 0..=u8::MAX {
                let glyph = font.glyph(char::from(code)).unwrap();
                addresses.extend([glyph as *const _ as usize, glyph.rgba.as_ptr() as usize]);
            }
        }
        addresses
    }

    fn snapshot(&mut self) -> DisplaySnapshot {
        let fonts = self.fonts.as_ref().unwrap();
        let glyph_pixels = [false, true]
            .into_iter()
            .flat_map(|selected| {
                let font = fonts.font(selected);
                (0..=u8::MAX).map(move |code| {
                    let glyph = font.glyph(char::from(code)).unwrap();
                    (
                        fingerprint(&glyph.rgba),
                        glyph.width,
                        glyph.height,
                        [glyph.advance, glyph.kern, glyph.xoff, glyph.yoff].map(f32::to_bits),
                    )
                })
            })
            .collect();
        let radar = self.radar.as_mut().unwrap();
        let image = radar
            .fullscreen_image(self.session.cache.level_terrain_radar().unwrap())
            .unwrap();
        let fullscreen = (
            image.width,
            image.height,
            image.rgba.as_ptr() as usize,
            fingerprint(&image.rgba),
        );
        DisplaySnapshot {
            layouts: layout_snapshot(&self.session.cache),
            variant: self.variant,
            world_variant: self.world_variant,
            reference_size: self.reference_size,
            font_size: [fonts.virtual_w.to_bits(), fonts.virtual_h.to_bits()],
            font_points: (0..13).map(|index| fonts.layout_point(index)).collect(),
            glyph_pixels,
            hud_layout: self.hud_layout,
            radar_layout: radar.layout(),
            fullscreen,
            map_status: self.map_status.clone(),
            hud: self.overlay.hud.clone(),
            radar: self.overlay.radar.clone(),
            projection: self.projection,
            retained_owners: self.retained_owners(),
        }
    }
}

#[derive(Debug, PartialEq)]
struct SystemLayoutSnapshot {
    origin: Option<SystemLayoutOrigin>,
    scalars: Vec<Option<u32>>,
    points: Vec<Option<(i16, i16)>>,
}

fn layout_snapshot(cache: &ResourceCache) -> [SystemLayoutSnapshot; 2] {
    [(2, 6, 13), (3, 18, 35)].map(|(system, scalars, points)| SystemLayoutSnapshot {
        origin: cache.system_layout_origin(system).cloned(),
        scalars: (0..scalars)
            .map(|index| cache.system_data_value(system, index))
            .collect(),
        points: (0..points)
            .map(|index| cache.system_layout_point(system, index))
            .collect(),
    })
}

#[derive(Debug, PartialEq)]
struct DisplaySnapshot {
    layouts: [SystemLayoutSnapshot; 2],
    variant: Option<u32>,
    world_variant: u32,
    reference_size: (u32, u32),
    font_size: [u32; 2],
    font_points: Vec<Option<(f32, f32)>>,
    glyph_pixels: Vec<(u64, u32, u32, [u32; 4])>,
    hud_layout: Option<GameplayHudLayout>,
    radar_layout: GameplayRadarLayout,
    fullscreen: (u32, u32, usize, u64),
    map_status: Option<FullscreenMapStatusResources>,
    hud: Option<GameplayHudFrame>,
    radar: Option<RadarHudFrame>,
    projection: WorldProjection,
    retained_owners: Vec<usize>,
}

#[v2k_test_support::retail_test]
fn resident_display_composes_all_authored_tiers_and_retains_intrinsic_and_frozen_owners() {
    let mut fixture = DisplayFixture::new();
    let original = fixture.snapshot();
    let notifications = fixture.overlay.notifications.clone();
    let lights = fixture.overlay.explosion_lights.clone();
    let static_lights = fixture.overlay.static_explosion_lights.clone();
    let sprite = fixture.overlay.full_frame_sprite;
    let presentation = fixture.overlay.presentation.as_ref().unwrap();
    let shield = presentation.shield;
    let particles = presentation.particles.clone();
    let intrinsic_path = fixture
        .session
        .cache
        .menu_ovl()
        .unwrap()
        .source_path
        .clone();
    let world_path = fixture.session.cache.level().unwrap().source_path.clone();
    for tier in [
        HighSystemLayoutTier::High800,
        HighSystemLayoutTier::High1024,
        HighSystemLayoutTier::High640,
    ] {
        // Resolve an independent expected composition while the old cache is
        // still resident. The production bridge then stages and publishes it.
        let stage = fixture
            .session
            .prepare_high_system_layout_refresh(tier)
            .unwrap();
        let expected_hud = GameplayHudLayout::from_cache(&stage, tier.variant()).unwrap();
        let expected_radar = GameplayRadarLayout::from_cache(&stage, tier.variant()).unwrap();
        let expected_projection = WorldProjection::from_cache(&stage).unwrap();
        let mut expected_status = fixture.map_status.clone().unwrap();
        expected_status.refresh_layout(&stage).unwrap();
        let expected_menu_points = (0..13)
            .map(|index| {
                stage
                    .system_layout_point(2, index)
                    .map(|(x, y)| (f32::from(x), f32::from(y)))
            })
            .collect::<Vec<_>>();
        assert!(fixture.refresh(tier).unwrap());
        assert_eq!(fixture.variant, Some(tier.variant()));
        assert_eq!(fixture.world_variant, tier.variant());
        assert_eq!(fixture.reference_size, tier.size());
        assert_eq!(fixture.hud_layout, Some(expected_hud));
        assert_eq!(fixture.radar.as_ref().unwrap().layout(), expected_radar);
        assert_eq!(fixture.projection, expected_projection);
        assert_eq!(fixture.map_status, Some(expected_status));
        let fonts = fixture.fonts.as_ref().unwrap();
        assert_eq!(
            (fonts.virtual_w, fonts.virtual_h),
            (tier.size().0 as f32, tier.size().1 as f32)
        );
        assert_eq!(
            (0..13)
                .map(|index| fonts.layout_point(index))
                .collect::<Vec<_>>(),
            expected_menu_points
        );
        for system in [2, 3] {
            assert_eq!(
                fixture
                    .session
                    .cache
                    .system_layout_origin(system)
                    .unwrap()
                    .tier,
                tier
            );
        }
        let hud = fixture.overlay.hud.as_ref().unwrap();
        assert_eq!((hud.virtual_width, hud.virtual_height), tier.size());
        let cached_radar = fixture.overlay.radar.as_ref().unwrap();
        assert_eq!(cached_radar.origin, expected_radar.hud_origin);
        assert_eq!(cached_radar.virtual_size, expected_radar.virtual_size);
        assert_eq!(cached_radar.image, original.radar.as_ref().unwrap().image);
        assert!(fixture.radar.as_ref().unwrap().is_fullscreen_open());
        let image = fixture
            .radar
            .as_mut()
            .unwrap()
            .fullscreen_image(fixture.session.cache.level_terrain_radar().unwrap())
            .unwrap();
        assert_eq!(
            (image.width, image.height),
            (
                expected_radar.map_rect.width,
                expected_radar.map_rect.height
            )
        );
        assert_eq!(fixture.retained_owners(), original.retained_owners);
        assert_eq!(fixture.overlay.notifications, notifications);
        assert_eq!(fixture.overlay.explosion_lights, lights);
        assert_eq!(fixture.overlay.static_explosion_lights, static_lights);
        assert_eq!(fixture.overlay.full_frame_sprite, sprite);
        let presentation = fixture.overlay.presentation.as_ref().unwrap();
        assert_eq!(presentation.shield, shield);
        assert_eq!(presentation.particles, particles);
        assert_eq!(
            fixture.session.cache.menu_ovl().unwrap().source_path,
            intrinsic_path
        );
        assert_eq!(
            fixture.session.cache.level().unwrap().source_path,
            world_path
        );
        assert_eq!(fixture.snapshot().glyph_pixels, original.glyph_pixels);
    }
    let final_state = fixture.snapshot();
    assert_eq!(final_state.layouts, original.layouts);
    assert_eq!(final_state.font_points, original.font_points);
    assert_eq!(
        final_state.hud, original.hud,
        "evaluated HUD roundtrip is lossless"
    );
    assert_eq!(final_state.radar, original.radar);
    assert_eq!(final_state.projection, original.projection);
    assert_eq!(final_state.map_status, original.map_status);
    let before_noop = fixture.snapshot();
    assert!(!fixture.refresh(HighSystemLayoutTier::High640).unwrap());
    assert_eq!(
        fixture.snapshot(),
        before_noop,
        "same-tier request publishes nothing"
    );
}

#[v2k_test_support::retail_test]
fn mismatched_cached_hud_or_radar_rejects_before_any_display_publication() {
    let mut fixture = DisplayFixture::new();
    for bad_hud in [true, false] {
        let valid_hud = fixture.overlay.hud.clone();
        let valid_radar = fixture.overlay.radar.clone();
        if bad_hud {
            fixture.overlay.hud.as_mut().unwrap().virtual_width += 1;
        } else {
            fixture.overlay.radar.as_mut().unwrap().origin[0] += 1;
        }
        let before = fixture.snapshot();
        let result = fixture.refresh(HighSystemLayoutTier::High1024);
        if bad_hud {
            assert!(matches!(
                result,
                Err(DisplayRefreshError::Overlay(
                    paused_gameplay::GameplayOverlayRelayoutError::Hud(
                        v2k_game::gameplay_hud::GameplayHudRelayoutError::FrameLayoutMismatch
                    )
                ))
            ));
        } else {
            assert!(matches!(
                result,
                Err(DisplayRefreshError::Overlay(
                    paused_gameplay::GameplayOverlayRelayoutError::Radar(
                        v2k_game::gameplay_radar::GameplayRadarRelayoutError::FrameLayoutMismatch
                    )
                ))
            ));
        }
        assert_eq!(
            fixture.snapshot(),
            before,
            "no cache/font/layout prefix on rejection"
        );
        fixture.overlay.hud = valid_hud;
        fixture.overlay.radar = valid_radar;
    }
    assert!(
        fixture.refresh(HighSystemLayoutTier::High1024).unwrap(),
        "rejection leaves the resident tier usable"
    );
}

#[test]
fn ui_submission_policy_follows_resident_art_and_explicit_native_or_classic_choice() {
    use v2k_render::config::GraphicsDetail;
    use v2k_render::{ScalingMode, UiMapping, UiMappingRequest, UiSubmissionPolicy};
    let mut config = GameConfig::default();
    config.width = 1920;
    config.height = 1080;
    config.detail = GraphicsDetail::High;
    config.scaling = ScalingMode::Native;
    for (tier, expected_scale) in [
        (HighSystemLayoutTier::High640, 1.8),
        (HighSystemLayoutTier::High800, 1.8),
        (HighSystemLayoutTier::High1024, 1.40625),
    ] {
        let policy = ui_policy(&config, Some(tier.variant()));
        assert_eq!(policy, UiSubmissionPolicy::NativeCanvas);
        assert_eq!(
            UiMapping::new(UiMappingRequest {
                viewport: [config.width, config.height],
                authored_canvas: [tier.size().0, tier.size().1],
                policy,
            })
            .scale,
            expected_scale
        );
    }
    config.detail = GraphicsDetail::Low;
    assert_eq!(
        ui_policy(&config, Some(0)),
        UiSubmissionPolicy::FitAuthoredCanvas
    );
    assert_eq!(
        ui_policy(&config, None),
        UiSubmissionPolicy::FitAuthoredCanvas
    );
    config.detail = GraphicsDetail::High;
    assert_eq!(
        ui_policy(&config, Some(0)),
        UiSubmissionPolicy::FitAuthoredCanvas,
        "labelled low fallback follows actual pixels"
    );
    for scaling in [ScalingMode::FourThree, ScalingMode::Stretched] {
        config.scaling = scaling;
        assert_eq!(
            ui_policy(&config, Some(3)),
            UiSubmissionPolicy::FitAuthoredCanvas
        );
    }
}
