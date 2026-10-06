//! Opt-in OpenGL presentation smoke for the native weapon draw integration.
//!
//! This exports framed authored models, not a retail timeline comparison. The
//! hidden window never pumps input, presents a frame, or writes runtime config.
//! Captures and the generated receipt stay beneath the repository's `.tmp`.

use super::*;
use sdl2::pixels::PixelFormatEnum;
use v2k_formats::models::{ModelNativeInstanceFrame, ModelNativeMountCommand};
use v2k_game::entity::{
    AuthoredPlayerArrival, AuthoredWorldConstruction, EntityConstructionResources,
};
use v2k_game::model_tree::{linked_model_bounds, ModelTreeSubmissionBuffer};
use v2k_game::native_model_frame::{NativeModelFrame, NativeWorldViewport};
use v2k_game::session::GameSession;
use v2k_render::{GlRenderer, Renderer};

const OUTPUT_ENV: &str = "V2K_NATIVE_WEAPON_VISUAL_SMOKE_OUTPUT";
const SIZE: (u32, u32) = (640, 480);
const TICK: u32 = 73;

#[test]
#[ignore = "requires retail data, hidden OpenGL, and an explicit .tmp output path"]
fn export_native_weapon_visual_smoke() {
    run().expect("native weapon visual smoke");
}

fn visual_paths(output_env: &str) -> Result<(PathBuf, PathBuf), Box<dyn std::error::Error>> {
    let data = v2k_test_support::retail_dir().canonicalize()?;
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let tmp = repo.join(".tmp").canonicalize()?;
    let requested = PathBuf::from(
        std::env::var_os(output_env)
            .ok_or_else(|| format!("set {output_env} to a directory under repository .tmp"))?,
    );
    let output = if requested.is_absolute() {
        requested
    } else {
        repo.join(requested)
    };
    if output
        .components()
        .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err("visual smoke output must stay beneath repository .tmp".into());
    }
    // Windows canonical paths carry a verbatim prefix. Normalize the existing
    // parent before comparing an as-yet-uncreated output directory to `.tmp`.
    let output = output
        .parent()
        .ok_or("visual smoke output needs an existing parent")?
        .canonicalize()?
        .join(
            output
                .file_name()
                .ok_or("visual smoke output needs a name")?,
        );
    if !output.starts_with(&tmp) {
        return Err("visual smoke output must stay beneath repository .tmp".into());
    }
    std::fs::create_dir_all(&output)?;
    let output = output.canonicalize()?;
    if !output.starts_with(&tmp) {
        return Err("resolved visual smoke output escaped repository .tmp".into());
    }
    Ok((data, output))
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let (data, output) = visual_paths(OUTPUT_ENV)?;
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let attrs = video.gl_attr();
    attrs.set_context_profile(sdl2::video::GLProfile::Compatibility);
    attrs.set_context_version(2, 1);
    attrs.set_double_buffer(true);
    let window = video
        .window("V2K native weapon visual smoke", SIZE.0, SIZE.1)
        .hidden()
        .opengl()
        .build()?;
    let mut renderer = GlRenderer::new(window, SIZE.0, SIZE.1)?;
    renderer.set_scaling_mode(v2k_render::ScalingMode::Native, SIZE.0, SIZE.1);

    let mut session = GameSession::init(&data)?;
    for level in [2, 3, 5] {
        session.load_auxiliary_ovl(level, 1)?;
    }
    let colors = v2k_game::model_color::ModelMaterialCache::new();
    let mut receipts = Vec::new();

    // Enter menu, world, then player scenes on one persistent renderer. This
    // catches state carried across the same production scene boundaries.
    renderer.begin_scene(RenderScene::Menu);
    renderer.clear(0.0, 0.0, 0.0);
    renderer.set_camera(&menu_camera(SIZE.0 as f32 / SIZE.1 as f32));
    draw_menu_backdrop(
        &mut renderer,
        &session.cache,
        &colors,
        MenuBackdropFrame {
            presentation: MenuBackdropPresentation::Frontend,
            retail_tick: TICK,
            anim_state: KlausBackdropAnimState {
                morph_progress: 0,
                sway_amplitude_raw: 0,
                primary_phase: 0,
                twitch_phase: 0,
                twitch_strength: 0,
                model_state: 0,
                visible: true,
            },
            view_depth_raw: MENU_BACKDROP_VIEW_DEPTH_RAW,
            projection_y_offset: 0.0,
            depth_fade: ModelDepthFade::Disabled,
        },
        MenuSceneLight::NEUTRAL,
    );
    receipts.push(save_scene(&mut renderer, &output, "menu-klaus", None)?);

    // These authored hierarchies cover the normal-tier common flat-lit
    // material family used by cinematic and gameplay world submissions.
    session.load_level_by_id(13, 1)?;
    for name in ["college", "factory2", "lifter"] {
        let (id, _) = session
            .cache
            .global_model_by_name(name)
            .ok_or_else(|| format!("required world model {name} is absent"))?;
        let vars = AnimVars::default();
        let bounds = linked_model_bounds(&session.cache, id, GAMEPLAY_MODEL_SCALE, 8, &vars)
            .ok_or_else(|| format!("required world model {name} has no geometry"))?;
        let distance = (bounds.radius * 3.0).max(4.0);
        let focus = bounds.center.map(|v| (v * 256.0).round() as i16);
        let eye = [
            focus[0].wrapping_add((distance * 0.5 * 256.0) as i16),
            focus[1].wrapping_add((distance * 0.4 * 256.0) as i16),
            focus[2].wrapping_sub((distance * 256.0) as i16),
        ];
        let (camera, _) = world_camera(&session.cache, eye, focus);
        begin_world(&mut renderer, &session.cache, &camera);
        let mut buffer = ModelTreeSubmissionBuffer::default();
        ModelTreeRenderer::new_world(
            &mut renderer,
            &session.cache,
            &colors,
            GAMEPLAY_MODEL_SCALE,
            Some(MenuSceneLight::NEUTRAL),
            TICK as i32,
        )
        .with_submission_buffer(&mut buffer)
        .with_view((&camera).into())
        .draw_linked(
            id,
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            [0.0; 3],
            8,
            None,
            &vars,
        );
        assert!(
            !buffer.is_empty(),
            "{name} must enqueue real model geometry"
        );
        buffer.flush(&mut renderer, None);
        receipts.push(save_scene(
            &mut renderer,
            &output,
            &format!("cinematic-world-{name}"),
            None,
        )?);
    }

    session.load_level_by_id(17, 1)?;
    let (mut entities, mut actor_fx) = authored_player(&session);
    let player = entities.player().expect("authored player");
    let origin = player.position_raw();
    let known_basis = match player.physical_body_basis_q31() {
        RetailRuntimeValue::Known(basis) => basis,
        RetailRuntimeValue::Unresolved => {
            // Fresh player publication leaves the subsequent controlled-body
            // 13F70 phase to gameplay. This presentation fixture supplies that
            // explicit phase from the actual constructor's three angle words,
            // using the public shared source math without mutating the actor.
            let [heading, pitch, roll] = player.rotation_heading_pitch_roll_raw();
            Type9BodyBasis::from_angle_words(heading, pitch, roll)
        }
    };
    let basis = RetailRuntimeValue::Known(known_basis);
    let live_model = player.model_index.expect("authored live player model");
    assert_eq!(live_model, 41);
    let dying_model = session.cache.global_entity_model_table()[46][1] as usize;
    let (camera, viewport) = world_camera(
        &session.cache,
        [
            origin[0].wrapping_add(700),
            origin[1].wrapping_add(400),
            origin[2].wrapping_sub(1_200),
        ],
        origin,
    );
    let mut craft = PlayerCraft::new();
    craft.gun_barrel = f32::from(0x1000_u16);
    craft.set_primary_joint_pulses([u16::MAX; 2]);
    for (name, selector, dying) in [
        ("player-grenade-callback4", 4, false),
        ("player-rocket-callback5", 5, false),
        ("player-dying-center-fallback", 5, true),
    ] {
        craft.select_weapon_callback(selector);
        begin_world(&mut renderer, &session.cache, &camera);
        let mut buffer = ModelTreeSubmissionBuffer::default();
        let origins = draw_player_craft(
            &mut renderer,
            PlayerCraftDrawFrame {
                cache: &session.cache,
                colors: &colors,
                camera: &camera,
                model_id: if dying { dying_model } else { live_model },
                animation: if dying {
                    PlayerCraftDrawAnimation::Dying {
                        control_output_7_raw: 0x1000,
                    }
                } else {
                    PlayerCraftDrawAnimation::Live(&craft)
                },
                position: origin.map(|v| f32::from(v) / 256.0),
                body: known_basis.orientation_world_from_model(),
                shade_shift: 0,
                retail_tick: TICK,
                native_viewport: Some(viewport),
                actor_origin_raw: origin,
                body_basis: basis,
                submissions: &mut buffer,
            },
        );
        assert!(
            origins.admitted && !origins.blocked,
            "{name}: native Sub-E custody"
        );
        if selector == 4 && !dying {
            assert_eq!(
                origins.points,
                grenade_origins(&session, &craft, viewport, origin, known_basis),
                "current child mounted slot18/20 must supply each grenade origin",
            );
            assert!(origins.points.iter().all(Option::is_some));
        } else {
            assert_eq!(
                origins.points, [None; 2],
                "dormant tf14 retains center fallback"
            );
        }
        assert!(
            !buffer.is_empty(),
            "{name} must enqueue real player geometry"
        );
        buffer.flush(&mut renderer, None);
        receipts.push(save_scene(
            &mut renderer,
            &output,
            name,
            Some(origins.points),
        )?);
    }
    let split_parent = entities
        .iter_all()
        .find(|entity| entity.entity_type == 40)
        .expect("authored Type40");
    entities
        .construct_native_type56(
            v2k_game::split_and_explode::SplitChildRequest {
                requested_entity_handle_raw: 0,
                entity_type: 56,
                position_raw: split_parent.position_raw(),
                objective: false,
                velocity_raw: [0; 3],
                rotation_heading_pitch_roll_raw: [0; 3],
            },
            &session.cache,
            &mut actor_fx,
            TICK,
        )
        .expect("real dynamic Type56 presentation fixture");
    // Frame actual constructor-issued model/animation inputs. These views
    // do not execute actor Sub-H external-frame callbacks or establish a
    // retail animation match.
    for entity_type in [30, 40, 56] {
        let entity = entities
            .iter_all()
            .find(|entity| entity.entity_type == entity_type)
            .expect("native Alpine actor");
        let model = entity.model_index.expect("native model");
        let vars = entity.presentation_anim_vars(TICK);
        let bounds = linked_model_bounds(&session.cache, model, GAMEPLAY_MODEL_SCALE, 8, &vars)
            .expect("native actor geometry");
        let distance = (bounds.radius * 3.0).max(4.0);
        let focus = entity.position_raw();
        let eye = [
            focus[0].wrapping_add((distance * 0.5 * 256.0) as i16),
            focus[1].wrapping_add((distance * 0.4 * 256.0) as i16),
            focus[2].wrapping_sub((distance * 256.0) as i16),
        ];
        let (camera, _) = world_camera(&session.cache, eye, focus);
        let draw_pos = camera_relative(&camera, entity.position);
        let orientation =
            if model_is_camera_facing_actor(session.cache.global_model(model).unwrap()) {
                camera_facing_entity_orientation(camera.position, draw_pos, camera.left_handed)
            } else {
                non_player_entity_orientation(entity.heading, entity.physical_body_basis_q31())
            };
        begin_world(&mut renderer, &session.cache, &camera);
        let mut buffer = ModelTreeSubmissionBuffer::default();
        ModelTreeRenderer::new_world(
            &mut renderer,
            &session.cache,
            &colors,
            GAMEPLAY_MODEL_SCALE,
            Some(MenuSceneLight::NEUTRAL),
            TICK as i32,
        )
        .with_submission_buffer(&mut buffer)
        .with_view((&camera).into())
        .draw_linked(model, orientation, draw_pos, 8, None, &vars);
        assert!(!buffer.is_empty(), "native Type{entity_type} geometry");
        buffer.flush(&mut renderer, None);
        receipts.push(save_scene(
            &mut renderer,
            &output,
            &format!("native-alpine-type{entity_type}"),
            None,
        )?);
    }
    std::fs::write(
        output.join("receipt.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "scope": "framed authored geometry, constructor-issued actor banks and native player Sub-E; actor Sub-H external-frame callbacks omitted; no retail timeline acceptance",
            "system_variant": 1,
            "viewport": [SIZE.0, SIZE.1],
            "capture_source": "CurrentScene",
            "frames": receipts,
        }))?,
    )?;
    eprintln!(
        "Native weapon visual smoke exported to {}",
        output.display()
    );
    Ok(())
}

/// Controlled production ground views distinguish a depth-buffer failure from
/// a callback/geometry failure. These constructed poses do not claim a retail
/// camera or timeline match. No window is shown and no frame is presented.
#[test]
#[ignore = "requires retail data, hidden OpenGL, and an explicit .tmp output path"]
fn export_ground_overlay_visual_smoke() {
    use v2k_render::OverlayDepthPolicy;

    let (data, output) = visual_paths("V2K_GROUND_OVERLAY_VISUAL_SMOKE_OUTPUT").unwrap();
    let sdl = sdl2::init().unwrap();
    let video = sdl.video().unwrap();
    let attrs = video.gl_attr();
    attrs.set_context_profile(sdl2::video::GLProfile::Compatibility);
    attrs.set_context_version(2, 1);
    attrs.set_double_buffer(true);
    let window = video
        .window("V2K ground overlay visual smoke", SIZE.0, SIZE.1)
        .hidden()
        .opengl()
        .build()
        .unwrap();
    let mut renderer = GlRenderer::new(window, SIZE.0, SIZE.1).unwrap();
    renderer.set_scaling_mode(v2k_render::ScalingMode::Native, SIZE.0, SIZE.1);
    let mut session = GameSession::init(&data).unwrap();
    for level in [2, 3, 5] {
        session.load_auxiliary_ovl(level, 1).unwrap();
    }
    session.load_level_by_id(13, 1).unwrap();
    let colors = v2k_game::model_color::ModelMaterialCache::new();
    let frames = v2k_game::terrain_render::build_terrain_frames(&session.cache, &mut renderer)
        .expect("authored terrain textures");
    let terrain = session.cache.terrain().unwrap();
    let palette = session.cache.color_palettes().unwrap();
    let (model_id, model) = session.cache.global_model_by_name("player4").unwrap();
    assert_eq!(model_id, 41);
    assert!(model.vertex_type_flags.contains(&13));
    let root_footprint_triangles = model
        .triangles
        .iter()
        .filter(|tri| {
            tri.iter()
                .all(|&i| model.vertex_type_flags[i as usize] == 13)
        })
        .count();
    assert_eq!(root_footprint_triangles, 8);
    let craft = PlayerCraft::new();
    let basis = Type9BodyBasis::from_angle_words(0x4000, 0, 0);
    let mut receipts = Vec::new();
    // draw_terrain consumes elapsed duration, rather than an absolute tick.
    // Advance the infection motion once, then freeze it for paired captures.
    let mut terrain_elapsed_micros = TICK * 20_000;
    for (pose, x, z) in [
        ("cargo", 75.25_f32, 60.25_f32),
        ("near-cargo", 80.5, 60.5),
        ("spawn-slope", 181.25, 126.25),
        ("spawn-fraction", 183.75, 128.5),
    ] {
        let x_raw = (x * 256.0) as i32 as i16;
        let z_raw = (z * 256.0) as i32 as i16;
        let floor = terrain.bilinear_height_raw(x_raw, z_raw);
        assert!(
            floor >= terrain.sea_level_raw(),
            "{pose} must be dry ground"
        );
        let origin = [x_raw, floor.wrapping_add(256), z_raw];
        let eye = [
            origin[0].wrapping_add(900),
            origin[1].wrapping_add(850),
            origin[2].wrapping_sub(1800),
        ];
        let (camera, viewport) = world_camera(&session.cache, eye, origin);
        for (policy_name, policy) in [
            ("terrain-recede", OverlayDepthPolicy::TerrainRecede),
            ("decal-offset", OverlayDepthPolicy::DecalOffset),
            ("overlay-always", OverlayDepthPolicy::OverlayAlways),
        ] {
            for content in [GroundDrawContent::Complete, GroundDrawContent::SurfaceOnly] {
                renderer.set_overlay_depth_policy(policy);
                begin_world(&mut renderer, &session.cache, &camera);
                renderer.draw_terrain(
                    terrain,
                    palette,
                    Some(&frames),
                    None,
                    terrain_elapsed_micros,
                );
                terrain_elapsed_micros = 0;
                if content == GroundDrawContent::Complete {
                    receipts.push(
                        save_scene(
                            &mut renderer,
                            &output,
                            &format!("{pose}-{policy_name}-ground"),
                            None,
                        )
                        .unwrap(),
                    );
                }
                let mut submissions = ModelTreeSubmissionBuffer::default();
                let origins = draw_player_craft(
                    &mut renderer,
                    PlayerCraftDrawFrame {
                        cache: &session.cache,
                        colors: &colors,
                        camera: &camera,
                        model_id,
                        animation: PlayerCraftDrawAnimation::Live(&craft),
                        position: origin.map(|axis| f32::from(axis) / 256.0),
                        body: basis.orientation_world_from_model(),
                        shade_shift: 0,
                        retail_tick: TICK,
                        native_viewport: Some(viewport),
                        actor_origin_raw: origin,
                        body_basis: RetailRuntimeValue::Known(basis),
                        submissions: &mut submissions,
                    },
                );
                assert!(origins.admitted && !origins.blocked);
                assert!(!submissions.is_empty());
                let surface = v2k_render::WorldSurfaceProjection::new(terrain, TICK as i32)
                    .with_waves_enabled(
                        session
                            .cache
                            .level_desc()
                            .unwrap()
                            .raw_u32(0x84)
                            .unwrap_or(0)
                            != 0,
                    );
                let mut audit = GroundDrawAudit {
                    renderer: &mut renderer,
                    camera: camera.position,
                    content,
                    bodies: Vec::new(),
                };
                submissions.flush(&mut audit, Some(surface));
                let bodies = std::mem::take(&mut audit.bodies);
                drop(audit);
                assert_eq!(
                    bodies
                        .iter()
                        .filter(|body| {
                            body["radius_raw"].as_u64() == Some(u64::from(model.radius))
                        })
                        .count(),
                    root_footprint_triangles,
                    "{pose}/{policy_name}/{content:?}: main footprint must reach GL"
                );
                receipts.push(serde_json::json!({
                "pose": pose, "policy": policy_name, "content": format!("{content:?}"), "surface_draws": bodies,
            }));
                receipts.push(
                    save_scene(
                        &mut renderer,
                        &output,
                        &format!(
                            "{pose}-{policy_name}-{}",
                            match content {
                                GroundDrawContent::Complete => "craft",
                                GroundDrawContent::SurfaceOnly => "footprint",
                            }
                        ),
                        None,
                    )
                    .unwrap(),
                );
            }
        }
        receipts.push(serde_json::json!({
            "pose": pose, "origin_raw": origin, "eye_raw": eye,
            "surface_raw": floor, "sea_raw": terrain.sea_level_raw(),
        }));
    }
    std::fs::write(output.join("receipt.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "scope": "constructed real-player/terrain views; depth discriminator, not retail acceptance",
        "level": 13, "system_variant": 1, "viewport": [SIZE.0, SIZE.1],
        "model_records": model.records,
        "model_surface_materials": model.triangles.iter().enumerate().filter(|(_, tri)| {
            tri.iter().all(|&i| model.vertex_type_flags[i as usize] == 13)
        }).map(|(i, _)| model.face_materials[i]).collect::<Vec<_>>(),
        "capture_source": "CurrentScene", "frames": receipts,
    })).unwrap()).unwrap();
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GroundDrawContent {
    Complete,
    SurfaceOnly,
}

/// The image receipt carries the actual submitted footprint coordinates and
/// clip/cull decisions, so a missing raster can be traced to its owned phase.
struct GroundDrawAudit<'a> {
    renderer: &'a mut GlRenderer,
    camera: [f32; 3],
    content: GroundDrawContent,
    bodies: Vec<serde_json::Value>,
}

impl Renderer for GroundDrawAudit<'_> {
    fn backend_name(&self) -> &str {
        self.renderer.backend_name()
    }
    fn clear(&mut self, r: f32, g: f32, b: f32) {
        self.renderer.clear(r, g, b);
    }
    fn present(&mut self) {
        self.renderer.present();
    }
    fn resize(&mut self, w: u32, h: u32) {
        self.renderer.resize(w, h);
    }
    fn set_camera(&mut self, camera: &Camera) {
        self.renderer.set_camera(camera);
    }
    fn set_fog(&mut self, enabled: bool, near: f32, far: f32, color: [f32; 3]) {
        self.renderer.set_fog(enabled, near, far, color);
    }
    fn draw_terrain(
        &mut self,
        terrain: &v2k_formats::terrain::TerrainGrid,
        palette: &[v2k_formats::system::PaletteEntry],
        frames: Option<&v2k_render::TerrainFrames>,
        lights: Option<&v2k_render::TerrainLightWindow>,
        elapsed: u32,
    ) {
        self.renderer
            .draw_terrain(terrain, palette, frames, lights, elapsed);
    }
    fn draw_model_body(&mut self, draw: v2k_render::ModelDraw<'_>) {
        let mesh = draw.mesh;
        for (index, tri) in mesh.triangles.iter().enumerate() {
            if !tri
                .iter()
                .all(|&i| mesh.vertex_type_flags[i as usize] == 13)
            {
                continue;
            }
            let cull = mesh.face_cull.get(index).copied();
            let visible = match cull {
                Some(v2k_formats::models::ModelFaceCull::Plane(plane)) => {
                    v2k_render::gl_backend::authored_face_plane_visible(
                        plane,
                        draw.transform.orientation,
                        draw.transform.position,
                        draw.transform.scale,
                        self.camera,
                    )
                }
                _ => true,
            };
            self.bodies.push(serde_json::json!({
                "radius_raw": mesh.radius_raw,
                "triangle": tri, "raw": tri.map(|i| mesh.vertices[i as usize]),
                "projection": tri.map(|i| format!("{:?}", mesh.vertex_projection[i as usize])),
                "clip": tri.map(|i| format!("{:?}", mesh.vertex_clip[i as usize])),
                "surface_origin": tri.map(|i| format!("{:?}", mesh.vertex_surface_origin[i as usize])),
                "plane": format!("{cull:?}"), "plane_visible": visible,
                "material": format!("{:?}", mesh.materials.get(index)),
                "depth_policy": format!("{:?}", draw.depth_policy),
                "view_pin": format!("{:?}", draw.view_pin),
                "transform": { "position": draw.transform.position,
                    "orientation": draw.transform.orientation, "scale": draw.transform.scale },
            }));
            if self.content == GroundDrawContent::SurfaceOnly {
                let mut face = mesh;
                face.triangles = &mesh.triangles[index..index + 1];
                face.face_vertices = mesh.face_vertices.get(index..index + 1).unwrap_or(&[]);
                face.normals = mesh.normals.get(index..index + 1).unwrap_or(&[]);
                face.face_cull = mesh.face_cull.get(index..index + 1).unwrap_or(&[]);
                face.face_uvs = mesh.face_uvs.get(index..index + 1).unwrap_or(&[]);
                face.face_corner_normals = mesh
                    .face_corner_normals
                    .get(index..index + 1)
                    .unwrap_or(&[]);
                face.face_shading = mesh.face_shading.get(index..index + 1).unwrap_or(&[]);
                face.materials = mesh.materials.get(index..index + 1).unwrap_or(&[]);
                face.edges = &[];
                face.edge_materials = &[];
                face.edge_widths = &[];
                self.renderer
                    .draw_model_body(v2k_render::ModelDraw { mesh: face, ..draw });
            }
        }
        if self.content == GroundDrawContent::Complete {
            self.renderer.draw_model_body(draw);
        }
    }
    fn draw_model_billboards(&mut self, draw: v2k_render::ModelBillboardDraw<'_>) {
        if self.content == GroundDrawContent::Complete {
            self.renderer.draw_model_billboards(draw);
        }
    }
    fn draw_sprite(&mut self, rgba: &[u8], w: u32, h: u32, x: i32, y: i32) {
        self.renderer.draw_sprite(rgba, w, h, x, y);
    }
    fn draw_material_sprite(
        &mut self,
        rgba: &[u8],
        w: u32,
        h: u32,
        x: i32,
        y: i32,
        blend: v2k_render::WorldSpriteBlend,
    ) {
        self.renderer.draw_material_sprite(rgba, w, h, x, y, blend);
    }
    fn draw_fullscreen(&mut self, rgba: &[u8], w: u32, h: u32) {
        self.renderer.draw_fullscreen(rgba, w, h);
    }
    fn draw_color_overlay(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.renderer.draw_color_overlay(r, g, b, a);
    }
    fn viewport_size(&self) -> (u32, u32) {
        self.renderer.viewport_size()
    }
}

fn authored_player(session: &GameSession) -> (EntityManager, WorldFx) {
    let metadata = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, _)| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect::<Vec<_>>();
    let mut fx = WorldFx::new();
    let entities = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level: session.cache.level_desc().unwrap(),
            logical_world_index: 5,
            type_metadata: &metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.level_terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            player_arrival: Some(AuthoredPlayerArrival {
                position_raw: [1_000, 1_024, 2_000],
                heading_raw: 0x4000,
            }),
            retail_tick: TICK,
        },
        &mut fx,
    )
    .expect("authored Alpine player construction");
    (entities, fx)
}

fn world_camera(
    cache: &v2k_game::resource_cache::ResourceCache,
    eye: [i16; 3],
    focus: [i16; 3],
) -> (Camera, NativeWorldViewport) {
    let delta = [
        f32::from(focus[0].wrapping_sub(eye[0])),
        f32::from(focus[1].wrapping_sub(eye[1]).min(500)),
        f32::from(focus[2].wrapping_sub(eye[2])),
    ];
    let length = delta.iter().map(|value| value * value).sum::<f32>().sqrt();
    let mut camera = Camera::new(SIZE.0 as f32 / SIZE.1 as f32);
    camera.position = eye.map(|v| f32::from(v) / 256.0);
    camera.yaw = delta[0].atan2(-delta[2]);
    camera.pitch = (delta[1] / length).asin();
    camera.left_handed = true;
    WorldProjection::from_cache(cache)
        .expect("selected normal-tier native lens")
        .apply_to(&mut camera, SIZE);
    (
        camera,
        v2k_game::chase_camera::native_viewport_from_spring_points(eye, focus),
    )
}

fn begin_world(
    renderer: &mut dyn Renderer,
    cache: &v2k_game::resource_cache::ResourceCache,
    camera: &Camera,
) {
    renderer.begin_scene(RenderScene::World);
    renderer.clear(0.0, 0.0, 0.0);
    renderer.set_camera(camera);
    renderer.set_scene_projection_authority(
        v2k_game::world_projection::scene_projection_authority(
            WorldProjection::from_cache(cache),
            true,
            true,
            SIZE,
            v2k_render::ProjectionEffect::None,
        ),
    );
}

fn grenade_origins(
    session: &GameSession,
    craft: &PlayerCraft,
    viewport: NativeWorldViewport,
    origin: [i16; 3],
    basis: Type9BodyBasis,
) -> [Option<[i16; 3]>; 2] {
    let root = session.cache.global_model(41).unwrap();
    let tube = session.cache.global_model(56).unwrap();
    let vars = craft.anim_vars();
    let frame =
        NativeModelFrame::from_actor(viewport, origin, [basis.lateral, basis.up, basis.forward]);
    let mut result = [None; 2];
    for gun in root
        .materialize(&vars)
        .instances
        .iter()
        .filter(|gun| gun.model_id == 56)
    {
        let Some(ModelNativeInstanceFrame::Mount(commands)) = gun.native_frame.as_ref() else {
            panic!("PLAYER4 side gun must retain an authored mount");
        };
        assert!(commands
            .iter()
            .any(|command| matches!(command, ModelNativeMountCommand::Parent(16 | 48))));
        let child = frame.child(root, gun, &vars).unwrap();
        let index = (1 - gun.registers[3]) as usize;
        let slot = if index == 0 { 18 } else { 20 };
        assert_eq!(tube.records[usize::from(slot >> 1)][0], 0);
        let point = child.resolve_model_slot(tube, &vars, slot).unwrap();
        result[index] = Some(viewport.view_point_to_world(point).map(|v| v as i16));
    }
    result
}

fn save_scene(
    renderer: &mut dyn Renderer,
    output: &Path,
    name: &str,
    origins: Option<[Option<[i16; 3]>; 2]>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let mut frame = renderer
        .capture_frame(FrameCaptureSource::CurrentScene)
        .ok_or("OpenGL CurrentScene capture unavailable")?;
    assert_eq!((frame.width, frame.height), SIZE);
    let colored = frame
        .rgba
        .chunks_exact(4)
        .filter(|pixel| pixel[..3] != [0; 3])
        .count();
    assert!(
        colored > 64,
        "{name} must contain authored model pixels, found {colored}"
    );
    let filename = format!("{name}.bmp");
    sdl2::surface::Surface::from_data(
        &mut frame.rgba,
        frame.width,
        frame.height,
        frame.width * 4,
        PixelFormatEnum::RGBA32,
    )?
    .save_bmp(output.join(&filename))?;
    eprintln!("{filename}: {colored} colored pixels; emitter origins {origins:?}");
    Ok(serde_json::json!({
        "file": filename,
        "colored_pixels": colored,
        "emitter_origins_raw": origins,
    }))
}
