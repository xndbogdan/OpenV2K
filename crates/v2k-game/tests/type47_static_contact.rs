//! Static contact for Type47 newants: ordinary level-1 fences and the
//! Intro2 scene.
//!
//! Newants previously walked through static geometry because no Type47
//! static-contact pass existed. Ordinary controls pin the level-1 pen fences;
//! Intro2 controls prove the dispatch runs against authored level-50 statics.

use v2k_game::{
    entity::{
        AuthoredPlayerArrival, AuthoredWorldConstruction, Entity, EntityConstructionResources,
        EntityManager,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    session::GameSession,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
    type47_static_contact::{
        resolve_intro2_type47_static_contact, resolve_ordinary_type47_static_contact,
        Type47StaticContactApplied, Type47StaticContactFrame, Type47StaticContactOutcome,
    },
    world_fx::WorldFx,
};

fn session() -> GameSession {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("PRELOAD");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("normal system tier");
    session
}

fn metadata(session: &GameSession) -> Vec<EntityTypeRuntimeMetadata> {
    session
        .cache
        .global_entity_model_table()
        .iter()
        .copied()
        .enumerate()
        .map(|(entity_type, model_slots)| {
            session
                .cache
                .global_entity_type(entity_type)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots,
                    ..EntityTypeRuntimeMetadata::default()
                })
        })
        .collect()
}

struct OrdinaryFixture {
    session: GameSession,
    manager: EntityManager,
    scheduler: SpecializedActorTaskScheduler,
    metadata: EntityTypeRuntimeMetadata,
    static_damage: StaticDamageScheduler,
    world_fx: WorldFx,
    newant: u32,
}

impl OrdinaryFixture {
    fn load() -> Option<Self> {
        let mut session = session();
        session.load_level_by_id(13, 1).expect("normal Level 1");
        let metadata = metadata(&session);
        let mut fx = WorldFx::new();
        let manager = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                logical_world_index: 1,
                level: session.cache.level_desc().expect("Level-1 spawns"),
                type_metadata: &metadata,
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| {
                        session.cache.global_model(id).map(|model| model.radius)
                    }),
                },
                player_arrival: Some(AuthoredPlayerArrival {
                    position_raw: [19_712, -500, 14_848],
                    heading_raw: 0x4000,
                }),
                retail_tick: 0,
            },
            &mut fx,
        )
        .expect("ordinary Level-1 publication");
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler.adopt_intro2_type47_guards(&manager) > 0);
        let newant = manager
            .iter_all()
            .filter(|entity| entity.entity_type == 47)
            .map(|entity| entity.id)
            .find(|id| scheduler.type47_completed_scheduler_owner(&manager, *id))?;
        Some(Self {
            session,
            manager,
            scheduler,
            metadata: metadata[47].clone(),
            static_damage: StaticDamageScheduler::new(),
            world_fx: WorldFx::new(),
            newant,
        })
    }

    fn entity(&self) -> &Entity {
        self.manager
            .iter_all()
            .find(|entity| entity.id == self.newant)
            .expect("retained newant")
    }

    fn health_raw(&self) -> i32 {
        let RetailRuntimeValue::Known(health) = self.entity().collision.health_raw else {
            panic!("authenticated newant health");
        };
        health
    }

    fn place(&mut self, position_raw: [i16; 3], velocity_raw: [i16; 3]) {
        self.manager
            .entity_mut(self.newant)
            .expect("newant motion owner")
            .set_motion_raw(position_raw, velocity_raw);
    }

    fn resolve(&mut self) -> Type47StaticContactOutcome {
        resolve_ordinary_type47_static_contact(
            &mut self.manager,
            self.newant,
            &self.metadata,
            Type47StaticContactFrame {
                resources: &mut self.session.cache,
                static_damage: &mut self.static_damage,
                world_fx: &mut self.world_fx,
                scheduler: &mut self.scheduler,
                retail_tick: 2,
            },
        )
    }

    fn contact(&mut self) -> Type47StaticContactApplied {
        match self.resolve() {
            Type47StaticContactOutcome::Applied(applied) => applied,
            outcome => panic!("expected authored fence contact, got {outcome:?}"),
        }
    }
}

#[v2k_test_support::retail_test]
fn pen_fence_response_separates_newant_without_damage_on_soft_overlap() {
    let mut fixture = OrdinaryFixture::load().expect("retail fixture");
    // Proven pen-fence pose from the spider/peasant controls (cell [157,151]).
    fixture.place([-24961, 3, -26881], [0; 3]);
    let health = fixture.health_raw();
    let applied = fixture.contact();
    assert_eq!(applied.contact.cell, [157, 151]);
    assert_eq!(applied.contact.kind_index, 9);
    assert!(applied.contact.penetration_raw > 0);
    assert_eq!(applied.impact_raw, 0);
    assert_eq!(applied.static_damage, None);
    assert_eq!(applied.actor_damage, None);
    assert_eq!(fixture.entity().position_raw(), applied.position_after_raw);
    assert_eq!(fixture.health_raw(), health);
}

#[v2k_test_support::retail_test]
fn pen_fence_hard_contact_delivers_static_and_actor_damage() {
    let mut fixture = OrdinaryFixture::load().expect("retail fixture");
    fixture.place([-24961, 3, -26881], [-256, 0, 0]);
    let health = fixture.health_raw();
    let applied = fixture.contact();
    assert_eq!(applied.contact.cell, [157, 151]);
    assert_eq!(applied.contact.model_id, 532);
    assert_eq!(applied.contact.response_raw, -256);
    assert!(applied.position_after_raw[0] > [-24961, 3, -26881][0]);
    assert!(applied.impact_raw > 0);
    assert!(applied.static_damage.is_some());
    assert!(applied.actor_damage.is_some());
    assert_eq!(fixture.health_raw(), health);
}

#[v2k_test_support::retail_test]
fn separated_newant_misses_and_keeps_pose() {
    let mut fixture = OrdinaryFixture::load().expect("retail fixture");
    fixture.place([0, 20_000, 0], [0; 3]);
    assert_eq!(fixture.resolve(), Type47StaticContactOutcome::Miss);
    assert_eq!(fixture.entity().position_raw(), [0, 20_000, 0]);
    assert_eq!(fixture.entity().velocity_raw(), [0; 3]);
}

#[v2k_test_support::retail_test]
fn response_converges_toward_separation() {
    let mut fixture = OrdinaryFixture::load().expect("retail fixture");
    fixture.place([-24961, 3, -26881], [-256, 0, 0]);
    let first = fixture.contact();
    // Retail's exact response can leave a grazing residual (here 1 raw unit
    // for the wider newant envelope); the retail mover clears the rest
    // between passes. A second scan must strictly converge and stay silent.
    match fixture.resolve() {
        Type47StaticContactOutcome::Miss => {}
        Type47StaticContactOutcome::Applied(second) => {
            assert!(
                second.contact.penetration_raw < first.contact.penetration_raw,
                "contact must converge: first={} second={}",
                first.contact.penetration_raw,
                second.contact.penetration_raw
            );
            assert_eq!(second.impact_raw, 0);
            assert_eq!(second.static_damage, None);
            assert_eq!(second.actor_damage, None);
        }
        outcome => panic!("second pass must converge or miss, got {outcome:?}"),
    }
}

#[v2k_test_support::retail_test]
fn non_newant_subject_is_ineligible() {
    let mut fixture = OrdinaryFixture::load().expect("retail fixture");
    let peasant = fixture
        .manager
        .iter_all()
        .find(|entity| entity.entity_type == 9)
        .expect("level-1 peasant")
        .id;
    fixture.newant = peasant;
    assert_eq!(fixture.resolve(), Type47StaticContactOutcome::Ineligible);
}

#[v2k_test_support::retail_test]
fn newant_without_scheduler_custody_is_ineligible() {
    let mut fixture = OrdinaryFixture::load().expect("retail fixture");
    fixture.scheduler = SpecializedActorTaskScheduler::new();
    fixture.place([-24961, 3, -26881], [-256, 0, 0]);
    assert_eq!(fixture.resolve(), Type47StaticContactOutcome::Ineligible);
}

struct IntroFixture {
    session: GameSession,
    manager: EntityManager,
    scheduler: SpecializedActorTaskScheduler,
    metadata: EntityTypeRuntimeMetadata,
    static_damage: StaticDamageScheduler,
    world_fx: WorldFx,
    newant: u32,
}

impl IntroFixture {
    fn load() -> Option<Self> {
        let mut session = session();
        session.load_level_by_id(50, 1).expect("Intro2 level 50");
        let metadata = metadata(&session);
        let mut fx = WorldFx::new();
        let manager = EntityManager::from_native_intro2_frontend(
            session.cache.level_desc().expect("Intro2 spawns"),
            &metadata,
            EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            4793,
            &mut fx,
        )
        .expect("native Intro2 frontend publication");
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler.adopt_intro2_type47_guards(&manager) > 0);
        let newant = manager
            .iter_all()
            .filter(|entity| entity.entity_type == 47)
            .map(|entity| entity.id)
            .find(|id| scheduler.type47_completed_scheduler_owner(&manager, *id))?;
        Some(Self {
            session,
            manager,
            scheduler,
            metadata: metadata[47].clone(),
            static_damage: StaticDamageScheduler::new(),
            world_fx: WorldFx::new(),
            newant,
        })
    }

    fn resolve(&mut self) -> Type47StaticContactOutcome {
        resolve_intro2_type47_static_contact(
            &mut self.manager,
            self.newant,
            &self.metadata,
            Type47StaticContactFrame {
                resources: &mut self.session.cache,
                static_damage: &mut self.static_damage,
                world_fx: &mut self.world_fx,
                scheduler: &mut self.scheduler,
                retail_tick: 4794,
            },
        )
    }

    fn place(&mut self, position: [f32; 3]) {
        let raw = [
            (position[0] * 256.0).round() as i32 as i16,
            (position[1] * 256.0).round() as i32 as i16,
            (position[2] * 256.0).round() as i32 as i16,
        ];
        self.manager
            .entity_mut(self.newant)
            .expect("newant motion owner")
            .set_motion_raw(raw, [0; 3]);
    }
}

#[v2k_test_support::retail_test]
fn intro_newant_misses_empty_space_and_keeps_pose() {
    let mut fixture = IntroFixture::load().expect("retail fixture");
    // Y-only teleport: Intro2 allocation auth pins exact X/Z words.
    let origin = fixture
        .manager
        .iter_all()
        .find(|entity| entity.id == fixture.newant)
        .unwrap()
        .position_raw();
    fixture.place([
        f32::from(origin[0]) / 256.0,
        80.0,
        f32::from(origin[2]) / 256.0,
    ]);
    assert_eq!(fixture.resolve(), Type47StaticContactOutcome::Miss);
    assert_eq!(
        fixture
            .manager
            .iter_all()
            .find(|entity| entity.id == fixture.newant)
            .unwrap()
            .velocity_raw(),
        [0; 3]
    );
}

#[v2k_test_support::retail_test]
fn intro_newant_hits_authored_static_at_natural_pose() {
    let mut fixture = IntroFixture::load().expect("retail fixture");
    // Intro2 allocation auth pins exact X/Z words, so the ant cannot be
    // teleported onto a fence. Instead, borrow one authored static
    // (attribute, terrain_type) pair onto the ant's own cell: the scan must
    // then hit, respond, and either apply or fail closed on the kind.
    let origin = fixture
        .manager
        .iter_all()
        .find(|entity| entity.id == fixture.newant)
        .unwrap()
        .position_raw();
    let terrain = fixture.session.cache.level_terrain().expect("terrain");
    let (donor_attr, donor_type) = terrain
        .cells
        .iter()
        .find_map(|cell| (cell.attribute != 0).then_some((cell.attribute, cell.terrain_type)))
        .expect("level 50 authors static objects");
    let (cell_x, cell_z) = (
        ((origin[0] as u16) >> 8) as usize,
        ((origin[2] as u16) >> 8) as usize,
    );
    assert!(cell_x < 256 && cell_z < 256);
    let terrain_mut = fixture.session.cache.level_terrain_mut().expect("terrain");
    let cell = &mut terrain_mut.cells[cell_x * 256 + cell_z];
    cell.attribute = donor_attr;
    cell.terrain_type = donor_type;
    match fixture.resolve() {
        Type47StaticContactOutcome::Applied(applied) => {
            assert!(applied.contact.penetration_raw > 0);
            eprintln!(
                "intro static: cell={:?} kind={} model={} resp={} pen={}",
                applied.contact.cell,
                applied.contact.kind_index,
                applied.contact.model_id,
                applied.contact.response_raw,
                applied.contact.penetration_raw
            );
            // Retail responds only to inward motion: a stationary overlap is
            // a no-op. Probe cardinal velocities until the response engages.
            let mut moved = false;
            for velocity in [[-256, 0, 0], [256, 0, 0], [0, 0, -256], [0, 0, 256]] {
                fixture
                    .manager
                    .entity_mut(fixture.newant)
                    .unwrap()
                    .set_motion_raw(origin, velocity);
                if let Type47StaticContactOutcome::Applied(retry) = fixture.resolve() {
                    if retry.position_after_raw != origin {
                        moved = true;
                        break;
                    }
                }
            }
            assert!(moved, "inward motion must engage the response");
        }
        Type47StaticContactOutcome::Blocked {
            reason,
            committed_prefix: true,
        } => {
            eprintln!("explicit fail-closed static kind: {reason:?}");
        }
        outcome => panic!("authored static must engage the dispatch, got {outcome:?}"),
    }
}
