//! Corpus-backed native entity-weapon construction and retained scheduler flow.

use std::{collections::BTreeMap, time::Duration};
use v2k_game::{
    common_mover::type9_attitude::Type9BodyBasis,
    entity::{
        AuthoredPlayerArrival, AuthoredWorldConstruction, EntityConstructionResources,
        EntityManager,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue, RetailStateWord},
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    native_entity_weapons::{
        launch::{entity_weapon_direction_q31, entity_weapon_launch_velocity_raw},
        EntityWeaponBlock, EntityWeaponConstructionRequest, EntityWeaponKind,
        NativeEntityWeaponOwner,
    },
    primary_weapon::{
        fire_profile_from_descriptor, PrimaryFireGeometry, PrimaryGunChannel, PrimaryGunMounts,
        PrimaryLaunchBasis, PrimaryShotBudget, PrimaryTriggerInput, PrimaryWeapon,
    },
    session::GameSession,
    specialized_actor_task_production::{
        NativeWeaponProductionOutcome, SpecializedActorTaskProductionFrame,
        SpecializedActorTaskProductionOutcome, SpecializedActorTaskScheduler,
        SpecializedActorTaskWorld,
    },
    static_damage::StaticDamageScheduler,
    weapon_inventory::{AmmoCommit, Ammunition, PowerUpPayload, WeaponInventory},
    world_fx::WorldFx,
};

const HELD: PrimaryTriggerInput = PrimaryTriggerInput {
    source_a: true,
    source_b: false,
};

fn world() -> (GameSession, EntityManager) {
    let data = v2k_test_support::retail_dir();
    assert!(
        data.join("PRELOAD.DAT").is_file(),
        "normal-tier corpus required"
    );
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(2, 1).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(17, 1).unwrap();
    let metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, _)| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect();
    let mut manager = EntityManager::from_authored_world(
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
                position_raw: [1000, 1024, 2000],
                heading_raw: 0,
            }),
            retail_tick: 0,
        },
        &mut WorldFx::new(),
    )
    .unwrap();
    manager.cleanup_pending_actor_deferred_destroys();
    (session, manager)
}

fn geometry(manager: &EntityManager) -> PrimaryFireGeometry {
    let player = manager.player().unwrap();
    PrimaryFireGeometry {
        launch_basis: PrimaryLaunchBasis {
            origin_world: [0.0, 60.0, 0.0],
            direction_unit: [0.0, 0.0, 1.0],
        },
        gun_mounts: PrimaryGunMounts::Unresolved,
        shooter_origin_world: [0.0, 60.0, 0.0],
        shooter_id: player.id,
        shooter_entity_type_at_birth: Some(player.entity_type as u8),
        shooter_velocity_world: [0.0; 3],
        sound_origin_world: [0.0, 60.0, 0.0],
    }
}

fn request(
    manager: &EntityManager,
    kind: EntityWeaponKind,
    y: i16,
) -> EntityWeaponConstructionRequest {
    let source = manager.player().unwrap();
    let rotation_raw = source.rotation_heading_pitch_roll_raw();
    let basis = Type9BodyBasis::from_angle_words(rotation_raw[0], rotation_raw[1], rotation_raw[2]);
    let direction = entity_weapon_direction_q31(kind, basis, 0);
    EntityWeaponConstructionRequest {
        kind,
        source_actor_id: source.id,
        position_raw: [0, y, 0],
        velocity_raw: entity_weapon_launch_velocity_raw(kind, direction, source.velocity_raw()),
        rotation_raw,
    }
}

#[v2k_test_support::retail_test]
fn finite_held_batch_keeps_last_round_descriptor_for_native_constructor_after_selection() {
    for (selector, model, cue) in [(3, 128, 61), (4, 128, 88), (5, 240, 79)] {
        let (session, mut manager) = world();
        let mut inventory = WeaponInventory::new();
        inventory.acquire_weapon(PowerUpPayload {
            selector,
            amount: 1,
        });
        let descriptor = *inventory.selected_descriptor();
        let profile = fire_profile_from_descriptor(&descriptor).unwrap();
        let mut fire = PrimaryWeapon::new();
        assert!(fire
            .update_with_profile(
                Duration::ZERO,
                HELD,
                geometry(&manager),
                profile,
                PrimaryShotBudget::Limited(0)
            )
            .is_empty());
        let events = fire.update_with_profile(
            Duration::from_secs(3),
            HELD,
            geometry(&manager),
            profile,
            PrimaryShotBudget::Limited(1),
        );
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].fired_at, Duration::from_secs(1));
        assert_eq!(events[0].gun_channel, PrimaryGunChannel::A);
        assert_eq!(events[0].sound.sound_id, cue);
        assert!(
            matches!(inventory.commit_selected_round(), AmmoCommit::Fired { selector: fired, remaining: Ammunition::Finite(0), automatic_successor_slot: Some(_), .. } if fired == u32::from(selector))
        );
        assert_eq!(inventory.selected_descriptor().selector(), 1);
        let kind = EntityWeaponKind::from_selector(descriptor.selector()).unwrap();
        let launch = request(&manager, kind, 60 * 256);
        let owner = manager
            .construct_entity_weapon(launch, &session.cache, &mut WorldFx::new(), 0)
            .unwrap();
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == owner.entity_id())
            .unwrap();
        assert_eq!(entity.model_index, Some(model));
        assert_eq!(entity.entity_type, kind.entity_type() as u32);
        assert_eq!(entity.position_raw(), launch.position_raw);
        assert_eq!(entity.velocity_raw(), launch.velocity_raw);
        let mut tasks = SpecializedActorTaskScheduler::default();
        tasks.register_native_weapon(&manager, owner).unwrap();
        assert_eq!(
            tasks.native_weapon_owner(&manager, owner.entity_id()),
            Some(owner)
        );
    }
}

#[v2k_test_support::retail_test]
fn rejected_actual_source_allocates_nothing_and_consumes_no_random_word() {
    for kind in [
        EntityWeaponKind::Rocket,
        EntityWeaponKind::Grenade,
        EntityWeaponKind::DepthCharge,
    ] {
        for flags in [
            RetailStateWord::exact(0x8000_0000),
            RetailStateWord::from_known_bits(0, 0x4000),
        ] {
            let (session, mut manager) = world();
            let launch = request(&manager, kind, 60 * 256);
            manager.player_mut().unwrap().collision.state_flags_at_0x08 = flags;
            let ordinal = manager.next_common_body_ordinal();
            let count = manager.iter_all().count();
            let mut actual = WorldFx::new();
            let mut expected = WorldFx::new();
            assert_eq!(
                manager.construct_entity_weapon(launch, &session.cache, &mut actual, 0),
                Err(EntityWeaponBlock::SourceAllocation)
            );
            assert_eq!(manager.next_common_body_ordinal(), ordinal);
            assert_eq!(manager.iter_all().count(), count);
            assert_eq!(
                actual.next_shared_retail_random_u16(),
                expected.next_shared_retail_random_u16()
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn native_entity_deliveries_keep_birth_at_and_below_retail_sea_word() {
    for kind in [
        EntityWeaponKind::Rocket,
        EntityWeaponKind::Grenade,
        EntityWeaponKind::DepthCharge,
    ] {
        let (session, mut manager) = world();
        let sea = session.cache.terrain().unwrap().sea_level_raw();
        for y in [sea, sea.wrapping_sub(256)] {
            let launch = request(&manager, kind, y);
            let owner = manager
                .construct_entity_weapon(launch, &session.cache, &mut WorldFx::new(), 0)
                .unwrap();
            let entity = manager
                .iter_all()
                .find(|entity| entity.id == owner.entity_id())
                .unwrap();
            assert_eq!(entity.position_raw()[1], y);
        }
    }
}

fn run_terminal(
    session: &mut GameSession,
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    owner: NativeEntityWeaponOwner,
) {
    let mut tasks = SpecializedActorTaskScheduler::default();
    tasks.register_native_weapon(manager, owner).unwrap();
    for id in manager.retail_live_order_ids().collect::<Vec<_>>() {
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(
                u32::MAX,
                if id == owner.entity_id() {
                    0x0206_0005
                } else {
                    0
                },
            );
    }
    let mut hull = v2k_game::player_hull::PlayerHull::default();
    manager.sync_player_hull_collision_state(&hull);
    let mut statics = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    for tick in 1..=110 {
        fx.advance_frame_pacing(20000);
        // Displacement keeps the grenade's independent stillness tag out of
        // this timeout test; all task time, forces and terminal effects run.
        let entity = manager.entity_mut(owner.entity_id()).unwrap();
        let mut position = entity.position_raw();
        position[0] = tick as i16;
        entity.set_motion_raw(position, entity.velocity_raw());
        let pass = tasks.tick(
            manager,
            SpecializedActorTaskProductionFrame {
                world: SpecializedActorTaskWorld::Playing {
                    player_hull: &mut hull,
                    extra_lives: RetailRuntimeValue::Known(1),
                },
                hive_components: None,
                resources: &mut session.cache,
                world_fx: fx,
                static_damage: &mut statics,
                elapsed_micros: 20000,
                global_elapsed_micros: 20000,
                retail_tick: tick,
                notification_phase: GameplayNotificationPhase::Playing,
                main_base_abort_active: false,
            },
            &mut notifications,
        );
        let outcome = pass
            .outcomes
            .iter()
            .find(|outcome| outcome.entity_id() == owner.entity_id())
            .unwrap();
        if matches!(
            outcome,
            SpecializedActorTaskProductionOutcome::NativeWeapon(
                NativeWeaponProductionOutcome::Terminal { .. }
            )
        ) {
            assert_eq!(tick, 101, "strict native >2000-ms timer");
            assert!(tasks
                .native_weapon_owner(manager, owner.entity_id())
                .is_none());
            return;
        }
        assert!(
            matches!(
                outcome,
                SpecializedActorTaskProductionOutcome::NativeWeapon(
                    NativeWeaponProductionOutcome::Advanced { .. }
                )
            ),
            "{outcome:?}"
        );
    }
    panic!("native terminal did not run");
}

#[v2k_test_support::retail_test]
fn native_terminal_keeps_scatter_logical_owner_and_ordered_death_cues() {
    for kind in [
        EntityWeaponKind::Rocket,
        EntityWeaponKind::Grenade,
        EntityWeaponKind::DepthCharge,
    ] {
        let (mut session, mut manager) = world();
        let source = manager.player().unwrap().id;
        let source_type = manager.player().unwrap().entity_type as u8;
        let launch = request(&manager, kind, 60 * 256);
        let mut fx = WorldFx::new();
        let owner = manager
            .construct_entity_weapon(launch, &session.cache, &mut fx, 0)
            .unwrap();
        run_terminal(&mut session, &mut manager, &mut fx, owner);
        assert_eq!(
            manager.pending_actor_deferred_destroy_ids(),
            &[owner.entity_id()]
        );
        fx.process_pending();
        let sounds = fx.take_positional_sounds();
        assert_eq!(sounds.len(), 2);
        assert_eq!(sounds[0].sound_id, 62);
        assert_eq!(sounds[0].frequency_q16, 0x10000);
        assert_eq!(sounds[1].sound_id, 62);
        assert!((0x10000..=0x11fff).contains(&sounds[1].frequency_q16));
        assert_eq!(sounds[1].position, sounds[0].position);
        let frame = fx.prepare_presentation([640, 480], 0x1800, |_| {
            v2k_render::ParticleCenterProjection {
                screen: [320, 240],
                depth_raw: 512,
                clip: 0,
            }
        });
        let mut scatter = BTreeMap::<u8, usize>::new();
        for prepared in frame.particles() {
            let particle = &prepared.particle;
            if matches!(particle.source_class, 37 | 94 | 95) {
                *scatter.entry(particle.source_class).or_default() += 1;
                assert_eq!(particle.owner_id, Some(source));
                assert_eq!(particle.source_entity_type_at_birth, Some(source_type));
            }
        }
        if kind == EntityWeaponKind::Rocket {
            assert_eq!(scatter, BTreeMap::from([(37, 10)]));
        } else {
            assert_eq!(scatter, BTreeMap::from([(94, 8), (95, 8)]));
        }
        assert_eq!(
            manager
                .iter_all()
                .filter(|entity| entity.entity_type == 60)
                .count(),
            usize::from(kind != EntityWeaponKind::Rocket)
        );
    }
}
