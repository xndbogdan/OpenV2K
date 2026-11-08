use super::*;
use crate::entity_collision_state::EntityTypeRuntimeMetadata;
use crate::hive_birth::HiveEjectionSlot;
use crate::session::GameSession;

/// The lab's world 13 Hive (`0x04A30001`): Sub-N `+4C` set, its mouth at
/// `(BB80, FF00, 8180)` and its model's extent `0x180`.
const MOUTH: [i16; 3] = [-17536, -256, -32384];

fn world13() -> (GameSession, EntityManager, WorldFx) {
    let mut session = GameSession::init(&v2k_test_support::retail_dir()).expect("retail PRELOAD");
    session.load_auxiliary_ovl(3, 1).expect("system overlay");
    let type_metadata = session
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
        .collect::<Vec<_>>();
    session.load_level_by_id(13, 1).expect("world 13");
    let mut world_fx = WorldFx::new();
    let manager = EntityManager::from_fresh_new_game_level_with_type_metadata(
        session.cache.level_desc().expect("world 13 descriptor"),
        &type_metadata,
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects()),
        0,
        &mut world_fx,
    )
    .expect("world 13 actors");
    (session, manager, world_fx)
}

/// Retail's lab moved the bodies `0x80` below the mouth with no velocity.
fn drop_in(manager: &mut EntityManager, id: u32, dx: i16) {
    manager
        .entity_mut(id)
        .unwrap()
        .set_motion_raw([MOUTH[0] + dx, MOUTH[1] - 0x80, MOUTH[2]], [0; 3]);
}

/// One Hive visit of the lab's: 60000 us since the last.
fn visit(
    session: &mut GameSession,
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    hull: &mut PlayerHull,
) -> LiveHiveMouthPass {
    apply_live_hive_mouths(LiveHiveMouthFrame {
        entities: manager,
        resources: &mut session.cache,
        static_damage: &mut StaticDamageScheduler::new(),
        player_hull: hull,
        world_fx,
        scheduler: &mut SpecializedActorTaskScheduler::new(),
        notifications: &mut GameplayNotifications::new(),
        retail_tick: 1697,
        elapsed_us: 60_000,
        extra_lives: 2,
    })
}

fn motion(manager: &EntityManager, id: u32) -> ([u16; 3], [i16; 3], [u16; 3]) {
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    (
        entity.position_raw().map(|word| word as u16),
        entity.velocity_raw(),
        entity
            .rotation_heading_pitch_roll_raw()
            .map(|word| word as u16),
    )
}

fn state(manager: &EntityManager, id: u32, bits: u32) -> RetailRuntimeValue<u32> {
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    entity.collision.state_flags_at_0x08.masked(bits)
}

fn slots(manager: &EntityManager, hive: u32) -> [HiveEjectionSlot; 2] {
    *manager
        .iter_all()
        .find(|entity| entity.id == hive)
        .unwrap()
        .authored_radial_emitter
        .as_ref()
        .unwrap()
        .birth_runtime()
        .unwrap()
        .ejection_slots()
}

/// Retail's lab, world 13 pass 2199: the craft moved below the live Hive's
/// mouth dies on the Hive's visit (`FUN_00410C10`: its burst and death seed
/// take two words) and is thrown out (`FUN_0041C830`: three words, side
/// `0x25FF`), its slot aged by the visit's 60000 us (`FUN_0041CA90`).
#[v2k_test_support::retail_test]
fn a_live_hives_mouth_kills_the_craft_and_throws_it_out() {
    let (mut session, mut manager, mut world_fx) = world13();
    let hive = manager
        .iter_all()
        .find(|entity| entity.entity_type == 67)
        .unwrap()
        .id;
    let craft = manager.player().expect("craft").id;
    drop_in(&mut manager, craft, 0);
    world_fx.set_shared_retail_random_state(0x05A0_76D9);
    let mut hull = PlayerHull::default();
    let pass = visit(&mut session, &mut manager, &mut world_fx, &mut hull);
    assert_eq!(
        pass.events,
        [
            LiveHiveMouthEvent::Killed { hive, body: craft },
            LiveHiveMouthEvent::Ejected {
                hive,
                body: craft,
                outcome: HiveEjectionOutcome::Launched { slot_index: 0 }
            }
        ]
    );
    assert_eq!(world_fx.shared_retail_random_state(), 0x52B5_F07C);
    assert_eq!(
        motion(&manager, craft),
        ([0xBBFF, 0x00B0, 0x8364], [149, 850, 181], [0, 0, 0])
    );
    assert!(hull.dying && hull.health_raw == 0);
    // Retail's state word 0x06078865 -> 0x06074865.
    assert_eq!(
        state(&manager, craft, 0xC000),
        RetailRuntimeValue::Known(DYING_STATE_BIT)
    );
    assert_eq!(
        slots(&manager, hive),
        [
            HiveEjectionSlot {
                child_handle: Some(craft),
                timer_us: 440_000
            },
            HiveEjectionSlot::default()
        ]
    );
    // Thrown out, the craft is no longer in the mouth.
    assert!(visit(&mut session, &mut manager, &mut world_fx, &mut hull)
        .events
        .is_empty());
}

/// Retail's lab, world 13 pass 1900: a Type9 (no capability bit 3) and a
/// Type17 (bit 3) moved below the live mouth. The Type9 dies (one word) and
/// both are thrown out in walk order, each into its own slot; the Type17
/// keeps its health.
#[v2k_test_support::retail_test]
fn a_live_hives_mouth_kills_only_bodies_without_capability_bit_3() {
    let (mut session, mut manager, mut world_fx) = world13();
    let hive = manager
        .iter_all()
        .find(|entity| entity.entity_type == 67)
        .unwrap()
        .id;
    let order: Vec<_> = manager
        .iter_all()
        .map(|entity| (entity.id, entity.entity_type))
        .collect();
    let type9 = order.iter().find(|(_, kind)| *kind == 9).unwrap().0;
    let type17 = order
        .iter()
        .skip_while(|(id, _)| *id != type9)
        .find(|(_, kind)| *kind == 17)
        .expect("a Type17 after the Type9, as in retail's list")
        .0;
    let type17_health = manager
        .iter_all()
        .find(|entity| entity.id == type17)
        .unwrap()
        .collision
        .health_raw;
    drop_in(&mut manager, type9, 0x40);
    drop_in(&mut manager, type17, 0);
    world_fx.set_shared_retail_random_state(0x20CC_DB25);
    let mut hull = PlayerHull::default();
    let pass = visit(&mut session, &mut manager, &mut world_fx, &mut hull);
    let launched = |body, slot_index| LiveHiveMouthEvent::Ejected {
        hive,
        body,
        outcome: HiveEjectionOutcome::Launched { slot_index },
    };
    assert_eq!(
        pass.events,
        [
            LiveHiveMouthEvent::Killed { hive, body: type9 },
            launched(type9, 0),
            launched(type17, 1)
        ]
    );
    assert_eq!(world_fx.shared_retail_random_state(), 0x1E87_5D92);
    assert_eq!(
        motion(&manager, type9),
        ([0xBB32, 0x00B0, 0x8364], [-237, 850, 173], [0x8000, 0, 0])
    );
    assert_eq!(
        motion(&manager, type17),
        ([0xBAFA, 0x00AF, 0x8364], [-97, 850, 135], [0x8000, 0, 0])
    );
    assert_eq!(
        state(&manager, type9, 0xC000),
        RetailRuntimeValue::Known(DYING_STATE_BIT)
    );
    assert_eq!(
        state(&manager, type17, 0xC000),
        RetailRuntimeValue::Known(0)
    );
    assert_eq!(
        manager
            .iter_all()
            .find(|entity| entity.id == type17)
            .unwrap()
            .collision
            .health_raw,
        type17_health
    );
    assert!(!hull.dying);
    assert_eq!(
        slots(&manager, hive).map(|slot| (slot.child_handle, slot.timer_us)),
        [(Some(type9), 440_000), (Some(type17), 440_000)]
    );
}
