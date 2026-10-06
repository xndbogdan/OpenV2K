//! Retail plasma sweeps must reach player and ordinary turret presentation.
//!
//! PLAYER4 selects models 46/47/48 through callback word 4. Their authored
//! register programs shift callback zero left ten bits and move opposite-phase
//! glow anchors along the otherwise stationary coloured tubes. Tests do not
//! supply an animation clock directly: the player draw and entity presentation
//! callback own publication, independently of native turret task custody.

use super::*;
use v2k_formats::system::PaletteEntry;
use v2k_formats::terrain::TerrainGrid;
use v2k_game::entity::{Entity, EntityKind};
use v2k_game::model_color::ModelMaterialCache;
use v2k_game::session::GameSession;
use v2k_game::weapon_inventory::PowerUpPayload;
use v2k_render::{
    FaceMaterial, ModelBillboardDraw, ModelDraw, Renderer, TerrainFrames, TerrainLightWindow,
    TextureId, WorldSpriteBlend,
};

#[derive(Debug, PartialEq)]
struct RecordedGlow {
    slot: u16,
    anchor_raw: [f64; 3],
    anchor_world: [f64; 3],
    size: u16,
    angle: u16,
    material: FaceMaterial,
}

#[derive(Debug, PartialEq)]
struct RecordedPlasmaBody {
    core_triangles_raw: Vec<[[f64; 3]; 3]>,
    materials: Vec<FaceMaterial>,
}

#[derive(Debug, PartialEq)]
struct RecordedFrame {
    glows: Vec<RecordedGlow>,
    bodies: Vec<RecordedPlasmaBody>,
}

struct RecordingRenderer {
    glow_sprite: u16,
    core_texture: Option<TextureId>,
    next_texture: u32,
    glows: Vec<RecordedGlow>,
    bodies: Vec<RecordedPlasmaBody>,
}

impl RecordingRenderer {
    fn new(glow_sprite: u16) -> Self {
        Self {
            glow_sprite,
            core_texture: None,
            next_texture: 1,
            glows: Vec::new(),
            bodies: Vec::new(),
        }
    }

    fn take_frame(&mut self) -> RecordedFrame {
        RecordedFrame {
            glows: std::mem::take(&mut self.glows),
            bodies: std::mem::take(&mut self.bodies),
        }
    }
}

impl Renderer for RecordingRenderer {
    fn backend_name(&self) -> &str {
        "plasma presentation recorder"
    }

    fn clear(&mut self, _r: f32, _g: f32, _b: f32) {}
    fn present(&mut self) {}
    fn resize(&mut self, _width: u32, _height: u32) {}
    fn set_camera(&mut self, _camera: &Camera) {}
    fn set_fog(&mut self, _enabled: bool, _near: f32, _far: f32, _color: [f32; 3]) {}

    fn draw_terrain(
        &mut self,
        _terrain: &TerrainGrid,
        _colors: &[PaletteEntry],
        _frames: Option<&TerrainFrames>,
        _lights: Option<&TerrainLightWindow>,
        _elapsed_micros: u32,
    ) {
    }

    fn create_texture(&mut self, _rgba: &[u8], _width: u32, _height: u32) -> Option<TextureId> {
        let texture = TextureId(self.next_texture);
        self.next_texture += 1;
        Some(texture)
    }

    fn draw_model_body(&mut self, draw: ModelDraw<'_>) {
        let Some(core_texture) = self.core_texture else {
            return;
        };
        let core_triangles_raw: Vec<_> = draw
            .mesh
            .triangles
            .iter()
            .zip(draw.mesh.materials)
            .filter(|(_, material)| material.texture == Some(core_texture))
            .map(|(triangle, _)| triangle.map(|vertex| draw.mesh.vertices[usize::from(vertex)]))
            .collect();
        if !core_triangles_raw.is_empty() {
            self.bodies.push(RecordedPlasmaBody {
                core_triangles_raw,
                materials: draw.mesh.materials.to_vec(),
            });
        }
    }

    fn draw_model_billboards(&mut self, draw: ModelBillboardDraw<'_>) {
        for (billboard, material) in draw.billboards.iter().zip(draw.materials) {
            if !billboard.textured || billboard.id != self.glow_sprite {
                continue;
            }
            assert!(billboard.size > 0);
            assert_eq!(
                draw.vertex_clip[usize::from(billboard.vertex)],
                v2k_formats::models::ModelSlotClip::Clear
            );
            assert_eq!(material.blend, WorldSpriteBlend::Additive);
            assert!(material.face.texture.is_some());
            let anchor_raw = draw.vertices[usize::from(billboard.vertex)];
            let anchor_world = std::array::from_fn(|axis| {
                f64::from(draw.transform.position[axis])
                    + (0..3)
                        .map(|local_axis| {
                            f64::from(draw.transform.orientation[axis][local_axis])
                                * anchor_raw[local_axis]
                                * f64::from(draw.transform.scale)
                                / 100.0
                        })
                        .sum::<f64>()
            });
            self.glows.push(RecordedGlow {
                slot: billboard.slot,
                anchor_raw,
                anchor_world,
                size: billboard.size,
                angle: billboard.angle,
                material: material.face,
            });
        }
    }

    fn draw_sprite(&mut self, _rgba: &[u8], _width: u32, _height: u32, _x: i32, _y: i32) {}

    fn draw_material_sprite(
        &mut self,
        _rgba: &[u8],
        _width: u32,
        _height: u32,
        _x: i32,
        _y: i32,
        _blend: WorldSpriteBlend,
    ) {
    }

    fn draw_fullscreen(&mut self, _rgba: &[u8], _width: u32, _height: u32) {}
    fn draw_color_overlay(&mut self, _r: f32, _g: f32, _b: f32, _a: f32) {}

    fn viewport_size(&self) -> (u32, u32) {
        (640, 480)
    }
}

fn normal_tier_session() -> Option<GameSession> {
    let data_dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&data_dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    Some(session)
}

#[v2k_test_support::retail_test]
fn acquired_plasma_player_draw_sweeps_all_colours_and_wraps_the_retail_clock() {
    let Some(session) = normal_tier_session() else {
        return;
    };
    let (player_model, _) = session.cache.global_model_by_name("player4").unwrap();
    assert_eq!(player_model, 41);

    let mut camera = Camera::new(640.0 / 480.0);
    camera.position = [128.0, 20.0, 136.0];
    camera.yaw = 0.0;
    camera.pitch = 0.0;
    camera.left_handed = true;
    let position = [128.0, 20.0, 128.0];

    for (selector, callback, model, name, core_sprite, glow_sprite) in [
        (0x0e, 6, 46, "pl4plasmared", 599, 603),
        (0x0d, 7, 47, "pl4plasmagreen", 598, 602),
        (0x0c, 8, 48, "pl4plasmablue", 597, 601),
    ] {
        assert_eq!(
            session.cache.global_model(model).unwrap().name.as_deref(),
            Some(name)
        );
        let mut inventory = WeaponInventory::new();
        assert!(matches!(
            inventory.acquire_weapon(PowerUpPayload {
                selector,
                amount: 100,
            }),
            WeaponAcquisition::Accepted {
                auto_selected: true,
                ..
            }
        ));
        let mut craft = PlayerCraft::new();
        sync_player_weapon_callback(&mut craft, &inventory);
        assert_eq!(craft.weapon_selector(), callback);

        let colors = ModelMaterialCache::new();
        let mut renderer = RecordingRenderer::new(glow_sprite);
        let core = colors.materials_for(&session.cache, &mut renderer, &[0x8000 | core_sprite]);
        renderer.core_texture = core[0].texture;
        assert!(renderer.core_texture.is_some());

        let ticks = [
            0, 16, 32, 48, 64, 63, 65_535, 65_536, 65_552, 65_568, 65_584, 65_600,
        ];
        let frames: Vec<_> = ticks
            .into_iter()
            .map(|tick| {
                let mut submissions = v2k_game::model_tree::ModelTreeSubmissionBuffer::default();
                draw_player_craft(
                    &mut renderer,
                    PlayerCraftDrawFrame {
                        cache: &session.cache,
                        colors: &colors,
                        camera: &camera,
                        model_id: player_model,
                        animation: PlayerCraftDrawAnimation::Live(&craft),
                        position,
                        body: craft.model_orientation(std::f32::consts::FRAC_PI_2),
                        shade_shift: 0,
                        retail_tick: tick,
                        native_viewport: None,
                        actor_origin_raw: world_position_raw(position),
                        body_basis: RetailRuntimeValue::Unresolved,
                        submissions: &mut submissions,
                    },
                );
                submissions.flush(
                    &mut renderer,
                    session.cache.terrain().map(|terrain| {
                        v2k_render::WorldSurfaceProjection::new(terrain, tick as i32)
                    }),
                );
                let frame = renderer.take_frame();
                assert_eq!(
                    frame.glows.len(),
                    2,
                    "{name}, tick {tick}: both plasma mounts"
                );
                assert_eq!(
                    frame.bodies.len(),
                    2,
                    "{name}, tick {tick}: both tube bodies"
                );
                assert_ne!(frame.glows[0].anchor_world, frame.glows[1].anchor_world);
                frame
            })
            .collect();

        for frame in &frames {
            assert_eq!(
                frame.bodies, frames[0].bodies,
                "{name}: stationary core geometry/materials"
            );
        }
        for pair in frames[..5].windows(2) {
            for mount in 0..2 {
                assert_ne!(
                    pair[0].glows[mount].anchor_raw, pair[1].glows[mount].anchor_raw,
                    "{name}, mount {mount}: each quarter-cycle must move the authored glow"
                );
                assert_ne!(
                    pair[0].glows[mount].anchor_world,
                    pair[1].glows[mount].anchor_world
                );
            }
        }
        assert_eq!(
            frames[0], frames[4],
            "{name}: 64 ticks complete the authored cycle"
        );
        assert_eq!(frames[5], frames[6], "{name}: final tick before u16 wrap");
        for phase in 0..5 {
            assert_eq!(
                frames[phase],
                frames[7 + phase],
                "{name}: u16 wrap at phase {phase}"
            );
        }
        assert_ne!(
            frames[6].glows, frames[7].glows,
            "{name}: wrap advances the glow"
        );
    }
}

#[v2k_test_support::retail_test]
fn ordinary_plasma_turrets_sweep_all_six_models_without_native_task_receipts() {
    let Some(session) = normal_tier_session() else {
        return;
    };
    let model_table = session.cache.global_entity_model_table();
    let mut camera = Camera::new(640.0 / 480.0);
    camera.position = [128.0, 20.0, 136.0];
    camera.yaw = 0.0;
    camera.pitch = 0.0;
    camera.left_handed = true;
    let orientation = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

    for (model, expected_type, name, core_sprite, glow_sprite, mounts) in [
        (162, 102, "turetred", 599, 603, 2),
        (165, 99, "turetblu", 597, 601, 2),
        (168, 103, "turetgre", 598, 602, 2),
        (171, 92, "tulazred", 599, 603, 1),
        (173, 97, "tulazblu", 597, 601, 1),
        (175, 96, "tulazgre", 598, 602, 1),
    ] {
        assert_eq!(
            session.cache.global_model(model).unwrap().name.as_deref(),
            Some(name)
        );
        let types: Vec<_> = model_table
            .iter()
            .enumerate()
            .filter(|(_, slots)| **slots == [model as u16; 4])
            .map(|(entity_type, _)| entity_type)
            .collect();
        assert_eq!(
            types,
            [expected_type],
            "{name}: original Section-12 model slots"
        );

        for entity_type in types {
            // This is only an ordinary presentation fixture. It deliberately
            // has no authored-spawn or native Intro2 constructor receipt.
            let mut entity = Entity::unresolved_port_entity(
                entity_type as u32 + 1,
                EntityKind::Unknown(entity_type as u32),
                entity_type as u32,
            );
            entity.model_slots = model_table[entity_type].map(|id| Some(usize::from(id)));
            entity.model_index = entity.model_slots[0];
            entity.position = [128.0, 20.0, 128.0];
            assert!(entity.intro2_gun_turret_runtime.is_none());
            assert!(entity.authored_spawn_index.is_none());

            let colors = ModelMaterialCache::new();
            let mut renderer = RecordingRenderer::new(glow_sprite);
            let core = colors.materials_for(&session.cache, &mut renderer, &[0x8000 | core_sprite]);
            renderer.core_texture = core[0].texture;
            assert!(renderer.core_texture.is_some());

            let frames: Vec<_> = [0, 16, 32, 48, 64, 65_536, 65_552]
                .into_iter()
                .map(|tick| {
                    let vars = entity.presentation_anim_vars(tick);
                    ModelTreeRenderer::new_world(
                        &mut renderer,
                        &session.cache,
                        &colors,
                        GAMEPLAY_MODEL_SCALE,
                        None,
                        tick as i32,
                    )
                    .with_view((&camera).into())
                    .draw_linked(
                        entity.model_index.unwrap(),
                        orientation,
                        entity.position,
                        8,
                        None,
                        &vars,
                    );
                    let frame = renderer.take_frame();
                    assert_eq!(
                        frame.glows.len(),
                        mounts,
                        "type {entity_type}, tick {tick}: glows"
                    );
                    assert_eq!(
                        frame.bodies.len(),
                        mounts,
                        "type {entity_type}, tick {tick}: bodies"
                    );
                    frame
                })
                .collect();

            for frame in &frames {
                assert_eq!(
                    frame.bodies, frames[0].bodies,
                    "type {entity_type}: stationary core geometry/materials"
                );
            }
            for pair in frames[..5].windows(2) {
                for mount in 0..mounts {
                    assert_ne!(
                        pair[0].glows[mount].anchor_raw, pair[1].glows[mount].anchor_raw,
                        "type {entity_type}, mount {mount}: quarter-cycle movement"
                    );
                    assert_ne!(
                        pair[0].glows[mount].anchor_world,
                        pair[1].glows[mount].anchor_world
                    );
                }
            }
            assert_eq!(frames[0], frames[4], "type {entity_type}: full cycle");
            assert_eq!(frames[0], frames[5], "type {entity_type}: u16 wrap");
            assert_eq!(
                frames[1], frames[6],
                "type {entity_type}: movement after u16 wrap"
            );
            assert!(entity.intro2_gun_turret_runtime.is_none());
        }
    }
}
