//! Native spider instances use actual construction history in every authored world.

use v2k_formats::models::{ModelEdgeStyle, ModelVertexProjection};
use v2k_formats::system::PaletteEntry;
use v2k_formats::terrain::TerrainGrid;
use v2k_game::{
    actor_task_owner::ActorTaskSlot,
    common_mover::type9_attitude::Type9BodyBasis,
    damage::DamagePacket,
    entity::{
        world_position_raw, AuthoredPlayerArrival, AuthoredWorldConstruction,
        EntityConstructionResources, EntityManager,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    gameplay_notifications::GameplayNotifications,
    intro2_common_dying::{
        tick_intro2_common_dying, Intro2CommonDyingFrame, Intro2CommonDyingOutcome,
        Intro2CommonDyingOwner,
    },
    intro2_type17::impact::Intro2Type17ImpactOutcome,
    intro2_type17::{
        tick_intro2_type17, Intro2Type17Frame, Intro2Type17Outcome, Intro2Type17Owner,
    },
    model_color::ModelMaterialCache,
    model_tree::{ModelTreeRenderer, ModelTreeView},
    session::GameSession,
    shared_actor_impact::{
        apply_shared_actor_particle_hit, SharedActorImpactFrame, SharedActorImpactOutcome,
    },
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    world_fx::{BallisticDamageRequest, ParticleEntityImpact, WorldFx},
};
use v2k_render::renderer::ModelSurfaceResolution;
use v2k_render::{
    Camera, ModelBillboardDraw, ModelDraw, Renderer, TerrainFrames, TerrainLightWindow,
    WorldSpriteBlend,
};

fn session() -> GameSession {
    let data = v2k_test_support::retail_dir();
    assert!(
        data.join("PRELOAD.DAT").is_file(),
        "normal-tier retail corpus required"
    );
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session
}

fn rows(session: &GameSession) -> Vec<EntityTypeRuntimeMetadata> {
    session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, _)| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect()
}

fn ordinary(session: &GameSession, world: u32, fx: &mut WorldFx) -> EntityManager {
    EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            logical_world_index: (world - 12) as i32,
            level: session.cache.level_desc().unwrap(),
            type_metadata: &rows(session),
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            player_arrival: Some(AuthoredPlayerArrival {
                position_raw: [19712, -500, 14848],
                heading_raw: 0x4000,
            }),
            retail_tick: 4793,
        },
        fx,
    )
    .unwrap()
}

fn check_births(
    session: &GameSession,
    manager: &EntityManager,
    mut seed: u8,
    player: bool,
) -> usize {
    let metadata = rows(session);
    if player
        && matches!(
            metadata[46].sub_d_steering_descriptor,
            RetailRuntimeValue::Known(Some(_))
        )
    {
        seed = seed.wrapping_add(1);
    }
    let mut count = 0;
    for spawn in &session.cache.level_desc().unwrap().entities {
        if spawn.entity_type == 17 {
            let entity = manager
                .iter_all()
                .find(|entity| entity.authored_spawn_index == Some(spawn.index))
                .unwrap();
            let cache = entity.type17_sub_d_frame_owner.unwrap();
            assert_eq!(
                cache.classifier_cache().stagger_counter(),
                seed,
                "spawn{} uses actual process allocation",
                spawn.index
            );
            assert_eq!(
                cache.classifier_cache().origin(),
                RetailRuntimeValue::Unresolved
            );
            assert_eq!(cache.classifier_cache().rows(), [0; 8]);
            assert!(Intro2Type17Owner::adopt(manager, entity.id).is_ok());
            assert_eq!(entity.model_slots, [Some(256); 4]);
            assert_eq!(
                entity.rotation_heading_pitch_roll_raw(),
                spawn.rotation.map(|word| word as i16)
            );
            let angles = entity.rotation_heading_pitch_roll_raw();
            assert_eq!(
                entity.physical_body_basis_q31(),
                RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(
                    angles[0], angles[1], angles[2]
                ))
            );
            assert_eq!(
                entity.position_raw()[1],
                session
                    .cache
                    .terrain()
                    .unwrap()
                    .bilinear_height_raw(spawn.position_raw()[0], spawn.position_raw()[2])
                    .wrapping_add(75)
            );
            assert_eq!(
                entity.collision.animation_offset_at_0xb2,
                RetailRuntimeValue::Known(0)
            );
            assert!(entity.actor_task_state(ActorTaskSlot::Primary).is_some());
            assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_some());
            count += 1;
        }
        if matches!(
            metadata[spawn.entity_type as usize].sub_d_steering_descriptor,
            RetailRuntimeValue::Known(Some(_))
        ) {
            seed = seed.wrapping_add(1);
        }
    }
    count
}

#[v2k_test_support::retail_test]
fn every_ordinary_spider_uses_shared_birth_and_its_own_process_sub_d() {
    let mut session = session();
    let mut fx = WorldFx::new();
    let mut counts = Vec::new();
    for world in 13..=49 {
        session.load_level_by_id(world, 1).unwrap();
        let initial_seed = fx.next_sub_d_allocation_seed();
        let manager = ordinary(&session, world, &mut fx);
        let count = check_births(&session, &manager, initial_seed, true);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(
            scheduler.adopt_fresh_level1_type17_follow_beacons(&manager),
            0,
            "replay adoption must not take a shared native task"
        );
        assert_eq!(scheduler.adopt_intro2_type17(&manager), count);
        assert_eq!(scheduler.adopt_intro2_type17(&manager), 0);
        if count != 0 {
            counts.push((world, count));
        }
        fx.clear();
    }
    assert_eq!(counts, [(13, 4), (14, 1), (20, 1), (35, 1)]);
}

#[v2k_test_support::retail_test]
fn native_intro_spiders_retain_process_history_across_an_earlier_world() {
    let mut session = session();
    let mut fx = WorldFx::new();
    session.load_level_by_id(14, 1).unwrap();
    let _previous = ordinary(&session, 14, &mut fx);
    fx.clear();
    let seed = fx.next_sub_d_allocation_seed();
    assert_ne!(seed, 0);
    session.load_level_by_id(50, 1).unwrap();
    let manager = EntityManager::from_native_intro2_frontend(
        session.cache.level_desc().unwrap(),
        &rows(&session),
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&|id| session.cache.global_model(id).map(|model| model.radius)),
        },
        4793,
        &mut fx,
    )
    .unwrap();
    assert_eq!(check_births(&session, &manager, seed, false), 2);
}

#[v2k_test_support::retail_test]
fn later_spiders_move_take_shared_primary_hits_and_run_class12() {
    for world in [14, 20, 35] {
        let mut session = session();
        session.load_level_by_id(world, 1).unwrap();
        let mut fx = WorldFx::new();
        let mut manager = ordinary(&session, world, &mut fx);
        let id = manager
            .iter_all()
            .find(|entity| entity.entity_type == 17)
            .unwrap()
            .id;
        let initial = manager.entity_mut(id).unwrap().position_raw();
        let stamp = manager.entity_mut(id).unwrap().construction_stamp_at_0xb4;
        let mut owner = Intro2Type17Owner::adopt(&manager, id).unwrap();
        let mut tick = 4793;
        for _ in 0..100 {
            tick += 1;
            // Controlled presentation enables this native actor's detailed pass.
            manager
                .entity_mut(id)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(0x68000, 0x68000);
            let pass = tick_intro2_type17(
                &mut manager,
                owner,
                Intro2Type17Frame {
                    capture_tasks: &mut v2k_game::specialized_actor_task_production::SpecializedActorTaskScheduler::default(),
                    notifications: &mut v2k_game::gameplay_notifications::GameplayNotifications::new(),
                    resources: &mut session.cache,
                    world_fx: &mut fx,
                    elapsed_micros: 20_000,
                    retail_tick: tick,
                },
            );
            assert!(
                matches!(
                    pass.outcome,
                    Intro2Type17Outcome::Waiting { .. } | Intro2Type17Outcome::Advanced { .. }
                ),
                "world{world}: {:?}",
                pass.outcome
            );
            assert!(pass.replacement_common_dying_owner.is_none());
            owner = pass.retained_owner.unwrap();
        }
        assert_ne!(
            manager.entity_mut(id).unwrap().position_raw(),
            initial,
            "world{world}"
        );
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(scheduler.adopt_intro2_type17(&manager), 1);
        let mut notifications = GameplayNotifications::new();
        let result = apply_shared_actor_particle_hit(
            SharedActorImpactFrame {
                resources: &session.cache,
                entities: &mut manager,
                world_fx: &mut fx,
                scheduler: &mut scheduler,
                notifications: &mut notifications,
                retail_tick: tick,
            },
            ParticleEntityImpact {
                source_particle_class: 16,
                impact_position_argument_va: 0,
                target_entity_id: id,
                position_world: [0.; 3],
                velocity_raw: [0, 0, 8192],
                damage: Some(BallisticDamageRequest {
                    packet: DamagePacket {
                        channels: [2, 0],
                        amounts_raw: [50_000, 0],
                    },
                    source_entity_type_at_birth: Some(46),
                    source_owner_id: Some(1),
                }),
            },
        );
        assert!(
            matches!(
                result,
                Some(SharedActorImpactOutcome::Spider(
                    Intro2Type17ImpactOutcome::Applied(_)
                ))
            ),
            "world{world}: {result:?}"
        );
        assert_eq!(
            manager.entity_mut(id).unwrap().collision.health_raw,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            manager.entity_mut(id).unwrap().construction_stamp_at_0xb4,
            stamp
        );
        let mut dying = Intro2CommonDyingOwner::adopt(&manager, id).unwrap();
        let mut terminal = false;
        for _ in 0..800 {
            tick += 1;
            manager
                .entity_mut(id)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(0x68000, 0x68000);
            let pass = tick_intro2_common_dying(
                &mut manager,
                dying,
                Intro2CommonDyingFrame {
                    resources: &session.cache,
                    world_fx: &mut fx,
                    elapsed_micros: 20_000,
                    retail_tick: tick,
                },
            );
            assert!(
                matches!(
                    pass.outcome,
                    Intro2CommonDyingOutcome::Waiting { .. }
                        | Intro2CommonDyingOutcome::Advanced { .. }
                ),
                "world{world}: {:?}",
                pass.outcome
            );
            if matches!(
                pass.outcome,
                Intro2CommonDyingOutcome::Advanced { terminal: true, .. }
            ) {
                terminal = true;
                break;
            }
            dying = pass.retained_owner.unwrap();
        }
        assert!(terminal, "world{world} native class12 must complete");
    }
}

#[derive(Default)]
struct RecordingRenderer {
    bodies: Vec<RecordedBody>,
}

struct RecordedBody {
    vertex_type_flags: Vec<i16>,
    vertex_projection: Vec<ModelVertexProjection>,
    surface_resolution: ModelSurfaceResolution,
    edges: Vec<v2k_formats::models::ModelEdge>,
    transform: v2k_render::ModelTransform,
}

impl Renderer for RecordingRenderer {
    fn backend_name(&self) -> &str {
        "spider-gait"
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
    fn draw_model_body(&mut self, draw: ModelDraw<'_>) {
        self.bodies.push(RecordedBody {
            vertex_type_flags: draw.mesh.vertex_type_flags.to_vec(),
            vertex_projection: draw.mesh.vertex_projection.to_vec(),
            surface_resolution: draw.surface_resolution,
            edges: draw.mesh.edges.to_vec(),
            transform: draw.transform,
        });
    }
    fn draw_model_billboards(&mut self, _draw: ModelBillboardDraw<'_>) {}
    fn draw_sprite(&mut self, _rgba: &[u8], _width: u32, _height: u32, _x: i32, _y: i32) {}
    fn draw_fullscreen(&mut self, _rgba: &[u8], _width: u32, _height: u32) {}
    fn draw_color_overlay(&mut self, _r: f32, _g: f32, _b: f32, _a: f32) {}
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
    fn viewport_size(&self) -> (u32, u32) {
        (640, 480)
    }
}

struct SpiderWorldDraw {
    context_resolved_external_points: usize,
    type14_offsets: Vec<[f32; 3]>,
}

fn submit_spider_world_draw(
    session: &v2k_game::session::GameSession,
    manager: &mut EntityManager,
    id: u32,
    retail_tick: u32,
) -> SpiderWorldDraw {
    let colors = ModelMaterialCache::new();
    let mut renderer = RecordingRenderer::default();
    let entity = manager.entity_mut(id).unwrap();
    let model_id = entity.model_index.expect("spider model");
    let draw_pos = entity.position;
    let orientation = match entity.physical_body_basis_q31() {
        RetailRuntimeValue::Known(basis) => basis.orientation_world_from_model(),
        RetailRuntimeValue::Unresolved => panic!("Type17 must retain FUN_00413F70"),
    };
    let vars = entity.presentation_anim_vars(retail_tick);
    let descriptor = session
        .cache
        .global_entity_type(17)
        .and_then(|record| record.sub_h_external_frame_descriptor())
        .expect("Type17 Sub-H");
    let body_axes_q31 = match entity.physical_body_basis_q31() {
        RetailRuntimeValue::Known(basis) => Some([basis.lateral, basis.up, basis.forward]),
        RetailRuntimeValue::Unresolved => None,
    };
    let origin_raw = world_position_raw(entity.position);
    let mut tree = ModelTreeRenderer::new_world(
        &mut renderer,
        &session.cache,
        &colors,
        100.0 / 256.0,
        None,
        retail_tick as i32,
    )
    .with_view(ModelTreeView {
        position: [draw_pos[0], draw_pos[1] + 8.0, draw_pos[2] - 15.0],
        forward: [0.0, -0.4, -1.0],
    });
    if let RetailRuntimeValue::Known(Some(runtime)) = &mut entity.sub_h_external_frame_runtime {
        tree = tree.with_sub_h_presentation(v2k_game::sub_h_external_frame::SubHPresentation {
            runtime,
            retail_tick,
            descriptor: &descriptor,
            model_records: &session.cache.global_model(model_id).unwrap().records,
            origin_raw,
            origin_world: draw_pos,
            body_axes_q31,
            native_context: None,
            fallback: None,
            emitter: None,
        });
    }
    tree.draw_linked(model_id, orientation, draw_pos, 8, None, &vars);
    drop(tree);
    // The shared dependency resolver invokes D360 while resolving authored
    // tf14 slots, before tf13 aliases consume them. The resulting body carries
    // semantic WorldPoint endpoints and deliberately uses Raw externally so
    // the backend does not invoke the old selector adapter a second time.
    let context_resolved_external_points = renderer
        .bodies
        .iter()
        .filter(|body| body.surface_resolution == ModelSurfaceResolution::ContextResolved)
        .map(|body| {
            body.vertex_type_flags
                .iter()
                .zip(&body.vertex_projection)
                .filter(|(flag, projection)| {
                    **flag == 14 && matches!(projection, ModelVertexProjection::WorldPoint(_))
                })
                .count()
        })
        .sum();
    SpiderWorldDraw {
        context_resolved_external_points,
        type14_offsets: renderer
            .bodies
            .iter()
            .flat_map(type14_sprite_world_offsets)
            .collect(),
    }
}

fn type14_offsets_changed(before: &[[f32; 3]], after: &[[f32; 3]]) -> bool {
    before.len() == after.len()
        && before
            .iter()
            .zip(after)
            .any(|(a, b)| (0..3).any(|axis| (a[axis] - b[axis]).abs() > 1.0 / 256.0))
}

fn sub_h_phase_started(manager: &EntityManager, id: u32) -> bool {
    manager
        .iter_all()
        .find(|entity| entity.id == id)
        .and_then(|entity| match &entity.sub_h_external_frame_runtime {
            RetailRuntimeValue::Known(Some(runtime)) => Some(runtime),
            _ => None,
        })
        .is_some_and(|runtime| runtime.records().iter().any(|record| record.phase_raw != 0))
}

fn type14_sprite_world_offsets(body: &RecordedBody) -> Vec<[f32; 3]> {
    let mut offsets = Vec::new();
    for edge in &body.edges {
        if !matches!(edge.style, ModelEdgeStyle::Sprite { .. }) {
            continue;
        }
        for &index in &edge.vertices {
            if body.vertex_type_flags.get(usize::from(index)) != Some(&14) {
                continue;
            }
            assert_eq!(
                body.surface_resolution,
                ModelSurfaceResolution::ContextResolved
            );
            let Some(ModelVertexProjection::WorldPoint(world)) =
                body.vertex_projection.get(usize::from(index))
            else {
                panic!("submitted tf14 ribbon endpoint must retain its D360 world point");
            };
            let world = world.map(|component| component as f32);
            offsets.push([
                world[0] - body.transform.position[0],
                world[1] - body.transform.position[1],
                world[2] - body.transform.position[2],
            ]);
        }
    }
    offsets
}

#[v2k_test_support::retail_test]
fn ordinary_spider_type14_legs_stride_after_world_sub_h_draw() {
    let mut session = session();
    session.load_level_by_id(14, 1).unwrap();
    let mut fx = WorldFx::new();
    let mut manager = ordinary(&session, 14, &mut fx);
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 17)
        .unwrap()
        .id;
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(0x68000, 0x68000);
    let mut owner = Intro2Type17Owner::adopt(&manager, id).unwrap();
    let mut tick = 4793u32;
    let pass = tick_intro2_type17(
        &mut manager,
        owner,
        Intro2Type17Frame {
            capture_tasks: &mut SpecializedActorTaskScheduler::default(),
            notifications: &mut GameplayNotifications::new(),
            resources: &mut session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            retail_tick: tick,
        },
    );
    assert!(matches!(
        pass.outcome,
        Intro2Type17Outcome::Waiting { .. } | Intro2Type17Outcome::Advanced { .. }
    ));
    owner = pass.retained_owner.unwrap();
    let first = submit_spider_world_draw(&session, &mut manager, id, tick);
    assert!(
        first.context_resolved_external_points >= 16,
        "world draw must resolve FUN_0041D360 into semantic tf14 points, got {}",
        first.context_resolved_external_points
    );
    let before = first.type14_offsets;
    assert!(
        before.len() >= 16,
        "spider authors sixteen type-14 0x22 ribbons, got {}",
        before.len()
    );
    for offset in &before {
        let span = (offset[0] * offset[0] + offset[1] * offset[1] + offset[2] * offset[2]).sqrt();
        assert!(
            span < 4.0,
            "type-14 0x22 must stay insect-scale, offset={offset:?} span={span}"
        );
    }
    // D360 is a draw callback. Constructor flags stay 0 until the first
    // writeback, so D0A0 cannot start a phase on the opening tick. A later
    // settled D360 visit sets WAIT_FOR_DEPS (0x08); the next D0A0 starts
    // the stride. Loop the shipped tick+draw pair rather than inventing a
    // gait or sampling Sub-H without presentation.
    let mut after = before.clone();
    let mut saw_phase = false;
    let mut endpoints_moved = false;
    for _ in 0..64 {
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(0x68000, 0x68000);
        tick += 1;
        let pass = tick_intro2_type17(
            &mut manager,
            owner,
            Intro2Type17Frame {
                capture_tasks: &mut SpecializedActorTaskScheduler::default(),
                notifications: &mut GameplayNotifications::new(),
                resources: &mut session.cache,
                world_fx: &mut fx,
                elapsed_micros: 20_000,
                retail_tick: tick,
            },
        );
        assert!(matches!(
            pass.outcome,
            Intro2Type17Outcome::Waiting { .. } | Intro2Type17Outcome::Advanced { .. }
        ));
        owner = pass.retained_owner.unwrap();
        let second = submit_spider_world_draw(&session, &mut manager, id, tick);
        after = second.type14_offsets;
        saw_phase = sub_h_phase_started(&manager, id);
        endpoints_moved = type14_offsets_changed(&before, &after);
        if saw_phase && endpoints_moved {
            break;
        }
    }
    assert_eq!(before.len(), after.len());
    for offset in &after {
        let span = (offset[0] * offset[0] + offset[1] * offset[1] + offset[2] * offset[2]).sqrt();
        assert!(
            span < 4.0,
            "walking type-14 0x22 must stay insect-scale, offset={offset:?} span={span}"
        );
    }
    assert!(
        saw_phase,
        "D360 writeback must let D0A0 leave phase 0, before={before:?} after={after:?}"
    );
    assert!(
        endpoints_moved,
        "type-14 0x22 endpoints must leave rest pose after D360 writeback + D0A0, before={before:?} after={after:?}"
    );
}
