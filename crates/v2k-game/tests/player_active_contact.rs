//! Real-data coverage for the bounded player/active-solid adapter.

use v2k_formats::models::CollisionModelPool;
use v2k_game::active_pair::{ActivePairUnresolved, PairCandidateDisposition};
use v2k_game::actor_task_dispatcher::ActorTaskRuntime;
use v2k_game::actor_task_owner::ActorTaskSlot;
use v2k_game::damage::{DamageDeliveryRecord, EntityHitEntry, PRIMARY_PROJECTILE_DAMAGE_PACKET};
use v2k_game::entity::{
    AuthoredPlayerArrival, AuthoredWorldConstruction, BeamCommand, BeamOutcome,
    CheckedProjectileDamageOutcome, CheckedProjectileDamageRequest,
    CheckedProjectileDamageUnresolved, Entity, EntityConstructionResources, EntityManager,
};
use v2k_game::entity_behavior::BehaviorChoiceListSource;
use v2k_game::entity_collision_state::{
    EntityCollisionRuntimeState, EntityPairCallbackRuntimeState, EntityTypeRuntimeMetadata,
    PairComponentContactPolicy, PairOrientationPolicy, RetailRuntimeValue,
    FULLY_ABOVE_SURFACE_STATE_BIT, SURFACE_STATE_MASK,
};
use v2k_game::gameplay_notifications::GameplayNotifications;
use v2k_game::player::PlayerCraft;
use v2k_game::player_active_contact::{
    resolve_player_active_contacts, PlayerActivePairError, PlayerActivePairFrame,
};
use v2k_game::player_hull::PlayerHull;
use v2k_game::session::GameSession;
use v2k_game::specialized_actor_task_production::SpecializedActorTaskScheduler;
use v2k_game::world_fx::WorldFx;

fn level_one_with_provenance(fresh_new_game: bool) -> Option<(GameSession, EntityManager)> {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("retail fixture must load");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("retail fixture must load");
    let type_models = session.cache.global_entity_model_table();
    let type_metadata = type_models
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
    session
        .load_level_by_id(13, 1)
        .expect("retail fixture must load");
    let manager = if fresh_new_game {
        let mut world_fx = WorldFx::new();
        EntityManager::from_fresh_new_game_level_with_type_metadata(
            session.cache.level_desc()?,
            &type_metadata,
            Some(session.cache.terrain()?),
            0,
            &mut world_fx,
        )
        .expect("fresh type-17 birth publication")
    } else {
        EntityManager::from_level_with_type_metadata(
            session.cache.level_desc()?,
            &type_metadata,
            Some(session.cache.terrain()?),
        )
    };
    Some((session, manager))
}

fn level_one() -> Option<(GameSession, EntityManager)> {
    level_one_with_provenance(false)
}

fn fresh_level_one() -> Option<(GameSession, EntityManager)> {
    level_one_with_provenance(true)
}

fn authored_ordinary_world(level_id: u32) -> Option<(GameSession, EntityManager)> {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("retail fixture must load");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("retail fixture must load");
    session
        .load_level_by_id(level_id, 1)
        .expect("retail fixture must load");
    let type_models = session.cache.global_entity_model_table();
    let type_metadata = type_models
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
    let mut world_fx = WorldFx::new();
    let manager = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            logical_world_index: (level_id - 12) as i32,
            level: session.cache.level_desc()?,
            type_metadata: &type_metadata,
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
            retail_tick: 4793,
        },
        &mut world_fx,
    )
    .expect("retail fixture must load");
    Some((session, manager))
}

fn later_campaign_world() -> Option<(GameSession, EntityManager)> {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("retail fixture must load");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("retail fixture must load");
    let type_models = session.cache.global_entity_model_table();
    let type_metadata = type_models
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
    session
        .load_level_by_id(14, 1)
        .expect("retail fixture must load");
    let mut manager = EntityManager::from_level_with_type_metadata(
        session.cache.level_desc()?,
        &type_metadata,
        Some(session.cache.terrain()?),
    );
    manager.place_or_spawn_player_at_campaign_arrival(
        type_metadata.get(46),
        [0, 2_560, 0],
        0x4000,
        session.cache.terrain(),
    );
    Some((session, manager))
}

fn entity_at_spawn(manager: &EntityManager, spawn_index: usize) -> &Entity {
    manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(spawn_index))
        .unwrap_or_else(|| panic!("Level-1 spawn {spawn_index}"))
}

fn audited_local_callbacks(pitch_raw: u16, roll_raw: u16) -> EntityPairCallbackRuntimeState {
    EntityPairCallbackRuntimeState {
        component_contact: RetailRuntimeValue::Known([
            PairComponentContactPolicy::None,
            PairComponentContactPolicy::None,
            PairComponentContactPolicy::None,
        ]),
        damage_modifier_address: RetailRuntimeValue::Known(None),
        damage_modifier_identity_context_empty: RetailRuntimeValue::Known(true),
        type_hit_callback_address: RetailRuntimeValue::Known(None),
        orientation_policy: RetailRuntimeValue::Known(
            PairOrientationPolicy::LiveHeadingWithAuthoredPitchRoll {
                pitch_raw,
                roll_raw,
            },
        ),
    }
}

fn audited_type9_callbacks() -> EntityPairCallbackRuntimeState {
    EntityPairCallbackRuntimeState {
        component_contact: RetailRuntimeValue::Known([
            PairComponentContactPolicy::DescriptorContact,
            PairComponentContactPolicy::None,
            PairComponentContactPolicy::None,
        ]),
        damage_modifier_address: RetailRuntimeValue::Known(None),
        damage_modifier_identity_context_empty: RetailRuntimeValue::Known(true),
        type_hit_callback_address: RetailRuntimeValue::Known(None),
        orientation_policy: RetailRuntimeValue::Known(
            PairOrientationPolicy::LiveHeadingWithAuthoredPitchRoll {
                pitch_raw: 0,
                roll_raw: 0,
            },
        ),
    }
}

fn audited_type17_callbacks() -> EntityPairCallbackRuntimeState {
    EntityPairCallbackRuntimeState {
        component_contact: RetailRuntimeValue::Known([
            PairComponentContactPolicy::DescriptorContact,
            PairComponentContactPolicy::None,
            PairComponentContactPolicy::None,
        ]),
        damage_modifier_address: RetailRuntimeValue::Known(None),
        damage_modifier_identity_context_empty: RetailRuntimeValue::Known(true),
        type_hit_callback_address: RetailRuntimeValue::Known(None),
        orientation_policy: RetailRuntimeValue::Unresolved,
    }
}

fn install_base_damage_impact(manager: &mut EntityManager) -> PlayerCraft {
    let base_position = entity_at_spawn(manager, 6).position_raw();
    let player = manager.player_mut().expect("persistent type-46 player");
    player.heading = std::f32::consts::FRAC_PI_2;
    player.set_motion_raw(base_position, [0, -4_096, 0]);
    PlayerCraft::new()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EntityCommitSnapshot {
    id: u32,
    position_bits: [u32; 3],
    velocity_bits: [u32; 3],
    collision: EntityCollisionRuntimeState,
}

fn commit_snapshot(manager: &EntityManager) -> Vec<EntityCommitSnapshot> {
    manager
        .iter_all()
        .map(|entity| EntityCommitSnapshot {
            id: entity.id,
            position_bits: entity.position.map(f32::to_bits),
            velocity_bits: entity.velocity.map(f32::to_bits),
            collision: entity.collision.clone(),
        })
        .collect()
}

#[v2k_test_support::retail_test]
fn level_one_seeds_only_the_authenticated_pair_allocations() {
    let (session, manager) = level_one().expect("retail fixture");

    let player = manager.player().expect("persistent type-46 player");
    assert_eq!(player.entity_type, 46);
    assert_eq!(player.model_in_slot(0), Some(41));
    assert_eq!(player.mass_raw, 100);
    let RetailRuntimeValue::Known(Some(context)) = player.current_behavior_context else {
        panic!("owned local controlled constructor must publish its selected class24");
    };
    assert!(
        v2k_game::player_contact_style::is_player_style(context, 24),
        "4438A0 local controller/component success applies to native level entry too"
    );
    assert_eq!(
        player.collision.pair_callbacks,
        EntityPairCallbackRuntimeState {
            component_contact: RetailRuntimeValue::Known([
                PairComponentContactPolicy::None,
                PairComponentContactPolicy::None,
                PairComponentContactPolicy::None,
            ]),
            damage_modifier_address: RetailRuntimeValue::Known(Some(0x0044_84A0)),
            damage_modifier_identity_context_empty: RetailRuntimeValue::Unresolved,
            type_hit_callback_address: RetailRuntimeValue::Known(None),
            orientation_policy: RetailRuntimeValue::Unresolved,
        }
    );

    for (spawn_index, entity_type, model_id, pitch_raw, roll_raw) in [
        (5, 68, 81, 0, 0),
        (6, 6, 286, 0, 0),
        (8, 54, 560, 0, 0),
        (23, 66, 227, 0, 0),
        (24, 67, 341, 0, 0),
    ] {
        let entity = entity_at_spawn(&manager, spawn_index);
        assert_eq!(entity.entity_type, entity_type);
        assert_eq!(entity.model_in_slot(0), Some(model_id));
        assert_eq!(
            entity.collision.pair_callbacks,
            audited_local_callbacks(pitch_raw, roll_raw)
        );
        assert!(
            session.cache.collision_model(model_id).is_some(),
            "model {model_id} collision program"
        );
    }
    for spawn_index in [9, 10, 14, 15, 16, 22] {
        let entity = entity_at_spawn(&manager, spawn_index);
        assert_eq!(entity.entity_type, 9);
        assert_eq!(entity.model_in_slot(0), Some(558));
        assert_eq!(entity.collision.pair_callbacks, audited_type9_callbacks());
    }
    for spawn_index in 17..=20 {
        let entity = entity_at_spawn(&manager, spawn_index);
        assert_eq!(entity.entity_type, 17);
        assert_eq!(entity.model_in_slot(0), Some(256));
        assert_eq!(entity.collision.pair_callbacks, audited_type17_callbacks());
    }
    for spawn_index in 11..=13 {
        let entity = entity_at_spawn(&manager, spawn_index);
        assert_eq!(entity.entity_type, 47);
        assert_eq!(
            entity.collision.pair_callbacks.damage_modifier_address,
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(
            entity.collision.pair_callbacks.type_hit_callback_address,
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(
            entity.collision.pair_callbacks.component_contact,
            RetailRuntimeValue::Unresolved
        );
        assert_eq!(
            entity.collision.pair_callbacks.orientation_policy,
            RetailRuntimeValue::Unresolved
        );
    }
    for entity in manager
        .iter_all()
        .filter(|entity| entity.authored_spawn_index.is_some())
        .filter(|entity| {
            !matches!(
                entity.authored_spawn_index,
                Some(
                    5 | 6
                        | 8
                        | 9
                        | 10
                        | 11
                        | 12
                        | 13
                        | 14
                        | 15
                        | 16
                        | 17
                        | 18
                        | 19
                        | 20
                        | 22
                        | 23
                        | 24
                )
            )
        })
    {
        assert_eq!(
            entity.collision.pair_callbacks,
            EntityPairCallbackRuntimeState::unresolved_with_constructor_null_modifier(),
            "uncaptured spawn {:?} must remain fail-closed",
            entity.authored_spawn_index
        );
    }
    assert_eq!(
        entity_at_spawn(&manager, 6).heading.to_bits(),
        (11_468_f32 * std::f32::consts::TAU / 65_536.0).to_bits(),
        "Main Base retains the captured first Euler word as live heading"
    );
    assert_eq!(
        entity_at_spawn(&manager, 6).current_behavior_context,
        RetailRuntimeValue::Unresolved,
        "generic construction cannot infer the fallible Main Base initializer result"
    );

    let power_up_spawns = manager
        .iter_all()
        .filter(|entity| entity.entity_type == 61)
        .map(|entity| entity.authored_spawn_index.expect("authored Power Up"))
        .collect::<Vec<_>>();
    assert_eq!(power_up_spawns, vec![32, 33, 34]);
    assert!(power_up_spawns.iter().all(|index| *index > 23));
    for spawn_index in power_up_spawns {
        let RetailRuntimeValue::Known(Some(context)) =
            entity_at_spawn(&manager, spawn_index).current_behavior_context
        else {
            panic!("Power Up's infallible initializer publishes its exact context");
        };
        assert_eq!(context.descriptor_address(), 0x004C_9738);
        assert_eq!(context.active_style().style_address(), 0x004C_96C0);
        assert_eq!(
            context.choice_list_source(),
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        );
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(0)
        );
    }
}

#[v2k_test_support::retail_test]
fn fresh_level_one_type17_constructor_surface_state_is_exact_and_route_scoped() {
    let (_, fresh) = fresh_level_one().expect("retail fixture");
    let (_, direct) = level_one().expect("retail fixture");

    for spawn_index in 17..=20 {
        let fresh_state = entity_at_spawn(&fresh, spawn_index)
            .collision
            .state_flags_at_0x08;
        assert_eq!(fresh_state.known_mask(), u32::MAX);
        assert_eq!(fresh_state.known_value_bits(), 0x0746_8805);
        assert_eq!(
            fresh_state.masked(SURFACE_STATE_MASK),
            RetailRuntimeValue::Known(FULLY_ABOVE_SURFACE_STATE_BIT)
        );

        let direct_state = entity_at_spawn(&direct, spawn_index)
            .collision
            .state_flags_at_0x08;
        assert_eq!(direct_state.known_mask(), !SURFACE_STATE_MASK);
    }
}

#[v2k_test_support::retail_test]
fn ordinary_type17_overlap_does_not_unresolved_the_solid_pass() {
    let (session, mut manager) = authored_ordinary_world(13).expect("retail fixture");
    let spider = entity_at_spawn(&manager, 17);
    assert_eq!(spider.entity_type, 17);
    assert!(spider.intro2_type17_runtime.is_some());
    let spider_id = spider.id;
    let spider_position = spider.position_raw();
    assert!(
        spider
            .model_index
            .and_then(|model_id| session.cache.collision_model(model_id))
            .is_some(),
        "ordinary Type17 collision program"
    );
    let type17_ids: Vec<u32> = manager
        .iter_all()
        .filter(|entity| entity.entity_type == 17)
        .map(|entity| entity.id)
        .collect();
    for id in type17_ids {
        manager.entity_mut(id).unwrap().collision.health_raw = RetailRuntimeValue::Known(50_000);
    }

    let player = manager.player_mut().expect("persistent type-46 player");
    player.heading = std::f32::consts::FRAC_PI_2;
    player.set_motion_raw(spider_position, [0, -4_096, 0]);
    let craft = PlayerCraft::new();
    let mut hull = PlayerHull::default();
    let pass = resolve_player_active_contacts(
        &mut manager,
        &session.cache,
        PlayerActivePairFrame {
            resources: &session.cache,
            retail_tick: 12_010,
            player_craft: &craft,
            player_hull: &mut hull,
            scheduler: &mut SpecializedActorTaskScheduler::new(),
            world_fx: &mut WorldFx::new(),
            notifications: &mut GameplayNotifications::new(),
            extra_lives: 0,
        },
    )
    .expect("Type17 player overlap must not Unresolved the solid pass");
    assert!(
        pass.core.dispositions.iter().any(|disposition| matches!(
            disposition,
            PairCandidateDisposition::Resolved { candidate_id, .. } if *candidate_id == spider_id
        )),
        "{:?}",
        pass.core.dispositions
    );
}

#[v2k_test_support::retail_test]
fn lethal_player_ram_kills_native_spider_through_capture_path_death() {
    let (session, mut manager) = authored_ordinary_world(13).expect("retail fixture");
    let spider = entity_at_spawn(&manager, 17);
    assert_eq!(spider.entity_type, 17);
    assert!(spider.intro2_type17_runtime.is_some());
    let spider_id = spider.id;
    // The commit-time dispatch authenticates the issuing allocation and the
    // scheduler's completed owner, exactly like the captor lane.
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert!(
        scheduler.adopt_intro2_type17(&manager) > 0,
        "native spider owner"
    );
    // Pre-wound the spider so the player's bounded ram is lethal: retail
    // spiders reach lethal player-pair delivery after surviving earlier hits,
    // not only from full health. Only health is written; the damage profile,
    // tasks and allocation remain authored.
    manager.entity_mut(spider_id).unwrap().collision.health_raw = RetailRuntimeValue::Known(200);
    let mut hull = PlayerHull::default();
    let mut killed = false;
    // The exact lethal closing speed is an arithmetic outcome of the shared
    // cap, not a fixture constant: escalate the downward ram until the
    // staged dispatch fires.
    for speed in [4_096_i16, 8_192, 12_000, 16_000] {
        let spider_position = manager
            .iter_all()
            .find(|entity| entity.id == spider_id)
            .unwrap()
            .position_raw();
        let player = manager.player_mut().expect("persistent type-46 player");
        player.heading = std::f32::consts::FRAC_PI_2;
        player.set_motion_raw(spider_position, [0, -speed, 0]);
        let mut notifications = GameplayNotifications::new();
        let mut world_fx = WorldFx::new();
        let pass = resolve_player_active_contacts(
            &mut manager,
            &session.cache,
            PlayerActivePairFrame {
                resources: &session.cache,
                retail_tick: 12_010,
                player_craft: &PlayerCraft::new(),
                player_hull: &mut hull,
                scheduler: &mut scheduler,
                world_fx: &mut world_fx,
                notifications: &mut notifications,
                extra_lives: 0,
            },
        )
        .expect("lethal spider packet must dispatch, not fail the solid pass");
        assert!(
            pass.core.dispositions.iter().any(|disposition| matches!(
                disposition,
                PairCandidateDisposition::Resolved { candidate_id, .. } if *candidate_id == spider_id
            )),
            "{:?}",
            pass.core.dispositions
        );
        let spider = manager
            .iter_all()
            .find(|entity| entity.id == spider_id)
            .unwrap();
        if spider
            .collision
            .state_flags_at_0x08
            .masked(v2k_game::entity_collision_state::DYING_STATE_BIT)
            == RetailRuntimeValue::Known(v2k_game::entity_collision_state::DYING_STATE_BIT)
        {
            killed = true;
            break;
        }
    }
    assert!(
        killed,
        "bounded player ram never reached the spider's lethal cap"
    );
    // The capture path owns health, the dying bit and the CommonDying task;
    // the staged pre-damage collision must not clobber them at commit.
    let spider = manager
        .iter_all()
        .find(|entity| entity.id == spider_id)
        .unwrap();
    let RetailRuntimeValue::Known(health) = spider.collision.health_raw else {
        panic!("spider health");
    };
    assert!(health <= 0, "lethal health prefix commits: {health}");
    assert!(
        matches!(
            spider.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::CommonDying(_))
        ),
        "lethal player-pair delivery publishes standard death, not a live graph"
    );
    // Physical response still moved the dying body before the 15040 entry.
    assert_ne!(
        spider.velocity_raw(),
        [0; 3],
        "retail moves both participants before each directional damage call"
    );
    // The player survives its own nonlethal share and stays hull-synced.
    assert!(!hull.dying);
    assert!(hull.health_raw > 0);
    assert_eq!(
        manager.player().unwrap().collision.health_raw,
        RetailRuntimeValue::Known(hull.health_raw)
    );
}

#[v2k_test_support::retail_test]
fn lethal_player_ram_kills_player_through_hull_terminal_without_failing_pass() {
    let (session, mut manager) = authored_ordinary_world(13).expect("retail fixture");
    let spider = entity_at_spawn(&manager, 17);
    assert_eq!(spider.entity_type, 17);
    assert!(spider.intro2_type17_runtime.is_some());
    let spider_id = spider.id;
    // The spider survives with deflecting health so only the subject direction
    // is lethal: the crafted player wreck, not a spider death, is under test.
    for id in manager
        .iter_all()
        .filter(|entity| entity.entity_type == 17)
        .map(|entity| entity.id)
        .collect::<Vec<_>>()
    {
        manager.entity_mut(id).unwrap().collision.health_raw = RetailRuntimeValue::Known(50_000);
    }
    // Pre-wound the player symmetrically on both hull and entity: retail rams
    // kill wounded players, not only full-health ones. Buffers stay default
    // so only health is preconditioned.
    let mut hull = PlayerHull::default();
    hull.health_raw = 200;
    manager
        .player_mut()
        .expect("persistent type-46 player")
        .collision
        .health_raw = RetailRuntimeValue::Known(200);
    let player_start = manager.player().unwrap().position_raw();
    let player_stamp = manager
        .player()
        .unwrap()
        .collision
        .last_hit_presentation_tick_at_0x34;
    let mut world_fx = WorldFx::new();
    let mut died = false;
    // The exact lethal closing speed is an arithmetic outcome of the shared
    // cap, not a fixture constant: escalate the downward ram until the staged
    // subject dispatch fires.
    for speed in [4_096_i16, 8_192, 12_000, 16_000] {
        let spider_position = manager
            .iter_all()
            .find(|entity| entity.id == spider_id)
            .unwrap()
            .position_raw();
        let player = manager.player_mut().expect("persistent type-46 player");
        player.heading = std::f32::consts::FRAC_PI_2;
        player.set_motion_raw(spider_position, [0, -speed, 0]);
        let mut notifications = GameplayNotifications::new();
        let pass = resolve_player_active_contacts(
            &mut manager,
            &session.cache,
            PlayerActivePairFrame {
                resources: &session.cache,
                retail_tick: 12_010,
                player_craft: &PlayerCraft::new(),
                player_hull: &mut hull,
                scheduler: &mut SpecializedActorTaskScheduler::new(),
                world_fx: &mut world_fx,
                notifications: &mut notifications,
                extra_lives: 0,
            },
        )
        .expect("lethal player packet must dispatch, not fail the solid pass");
        assert!(
            pass.core.dispositions.iter().any(|disposition| matches!(
                disposition,
                PairCandidateDisposition::Resolved { candidate_id, .. } if *candidate_id == spider_id
            )),
            "{:?}",
            pass.core.dispositions
        );
        if hull.dying {
            died = true;
            break;
        }
    }
    assert!(
        died,
        "bounded spider ram never reached the player's lethal cap"
    );
    // The hull terminal owns the player death state; the staged pre-damage
    // values must not resurrect it at commit.
    assert_eq!(hull.health_raw, 0);
    let player = manager.player().unwrap();
    assert_eq!(player.collision.health_raw, RetailRuntimeValue::Known(0));
    assert_eq!(player.model_index, player.model_slots[1]);
    assert_eq!(
        player.collision.last_hit_presentation_tick_at_0x34,
        player_stamp
    );
    assert!(manager.player_dying_contact_runtime().is_some());
    assert_eq!(
        world_fx.particle_count(),
        2,
        "cold governor: synchronous native scatter plus surface before pair return"
    );
    assert_eq!(
        world_fx
            .take_positional_sounds()
            .iter()
            .filter(|sound| sound.sound_id == 62)
            .count(),
        2,
        "475F0 random then fixed cue must finish before pair return"
    );
    assert_eq!(
        player
            .collision
            .state_flags_at_0x08
            .masked(v2k_game::entity_collision_state::DYING_STATE_BIT),
        RetailRuntimeValue::Known(v2k_game::entity_collision_state::DYING_STATE_BIT)
    );
    // Physical response still moved the dying player before the 15040 entry.
    assert_ne!(
        player.position_raw(),
        player_start,
        "retail moves both participants before each directional damage call"
    );
    // The high-health spider survives its nonlethal share on the same pass.
    let spider = manager
        .iter_all()
        .find(|entity| entity.id == spider_id)
        .unwrap();
    let RetailRuntimeValue::Known(spider_health) = spider.collision.health_raw else {
        panic!("spider health");
    };
    assert!(spider_health > 0 && spider_health <= 50_000);
    assert_eq!(
        spider
            .collision
            .state_flags_at_0x08
            .masked(v2k_game::entity_collision_state::DYING_STATE_BIT),
        RetailRuntimeValue::Known(0)
    );
}

#[v2k_test_support::retail_test]
fn ordinary_type17_forward_player_contact_applies_native_02da0() {
    let (session, mut manager) = authored_ordinary_world(13).expect("retail fixture");
    let spider = entity_at_spawn(&manager, 17);
    assert_eq!(spider.entity_type, 17);
    assert!(spider.intro2_type17_runtime.is_some());
    assert!(
        matches!(
            spider.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(_))
        ),
        "native Type17 birth publishes Primary SharedRetarget with 02DA0"
    );
    let spider_id = spider.id;
    let RetailRuntimeValue::Known(basis) = spider.physical_body_basis_q31() else {
        panic!("native Type17 publishes a physical body basis");
    };
    let heading_before = spider.heading_raw();
    let sub_a_before = spider.sub_a_propulsion_runtime;
    let primary_before = spider
        .actor_task_state(ActorTaskSlot::Primary)
        .cloned()
        .expect("Primary SharedRetarget");
    let last_yaw_before = spider
        .type17_sub_d_runtime
        .as_ref()
        .map(|runtime| runtime.last_yaw_step_raw);
    let position = spider.position;
    let forward = basis
        .forward
        .map(|component| component as f32 / 2_147_483_648.0);
    for id in manager
        .iter_all()
        .filter(|entity| entity.entity_type == 17)
        .map(|entity| entity.id)
        .collect::<Vec<_>>()
    {
        manager.entity_mut(id).unwrap().collision.health_raw = RetailRuntimeValue::Known(50_000);
    }

    let player = manager.player_mut().expect("persistent type-46 player");
    player.heading = std::f32::consts::FRAC_PI_2;
    player.position = std::array::from_fn(|axis| position[axis] + 0.25 * forward[axis]);
    let pass = resolve_player_active_contacts(
        &mut manager,
        &session.cache,
        PlayerActivePairFrame {
            resources: &session.cache,
            retail_tick: 12_010,
            player_craft: &PlayerCraft::new(),
            player_hull: &mut PlayerHull::default(),
            scheduler: &mut SpecializedActorTaskScheduler::new(),
            world_fx: &mut WorldFx::new(),
            notifications: &mut GameplayNotifications::new(),
            extra_lives: 0,
        },
    )
    .expect("authenticated Type17 02DA0 must not Unresolved the solid pass");
    assert!(
        pass.core.dispositions.iter().any(|disposition| matches!(
            disposition,
            PairCandidateDisposition::Resolved { candidate_id, .. } if *candidate_id == spider_id
        )),
        "{:?}",
        pass.core.dispositions
    );

    let spider = entity_at_spawn(&manager, 17);
    let heading_after = spider.heading_raw();
    let sub_a_after = spider.sub_a_propulsion_runtime;
    let primary_after = spider
        .actor_task_state(ActorTaskSlot::Primary)
        .cloned()
        .expect("Primary SharedRetarget after 02DA0");
    let last_yaw_after = spider
        .type17_sub_d_runtime
        .as_ref()
        .map(|runtime| runtime.last_yaw_step_raw);
    assert!(
        heading_after != heading_before
            || sub_a_after != sub_a_before
            || primary_after != primary_before
            || last_yaw_after != last_yaw_before,
        "forward player contact must apply native no-Sub-I 02DA0 writes; heading {heading_before:#06x} -> {heading_after:#06x}, sub_a {sub_a_before:?} -> {sub_a_after:?}, yaw {last_yaw_before:?} -> {last_yaw_after:?}"
    );
    assert!(
        matches!(
            spider.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(_))
        ),
        "02DA0 must not replace the Primary graph"
    );
}

#[v2k_test_support::retail_test]
fn unauthenticated_type17_player_contact_fails_closed_with_diagnostics() {
    let (session, mut manager) = authored_ordinary_world(13).expect("retail fixture");
    let spider = entity_at_spawn(&manager, 17);
    assert_eq!(spider.entity_type, 17);
    assert!(spider.intro2_type17_runtime.is_some());
    let spider_id = spider.id;
    let spider_spawn = spider.authored_spawn_index;
    let spider_position = spider.position_raw();
    manager.entity_mut(spider_id).unwrap().type17_sub_d_runtime = None;

    let player = manager.player_mut().expect("persistent type-46 player");
    player.heading = std::f32::consts::FRAC_PI_2;
    player.set_motion_raw(spider_position, [0, -4_096, 0]);
    let error = resolve_player_active_contacts(
        &mut manager,
        &session.cache,
        PlayerActivePairFrame {
            resources: &session.cache,
            retail_tick: 12_010,
            player_craft: &PlayerCraft::new(),
            player_hull: &mut PlayerHull::default(),
            scheduler: &mut SpecializedActorTaskScheduler::new(),
            world_fx: &mut WorldFx::new(),
            notifications: &mut GameplayNotifications::new(),
            extra_lives: 0,
        },
    )
    .expect_err("unauthenticated Type17 component contact must fail closed");
    assert_eq!(
        error,
        PlayerActivePairError::Type17DescriptorContact {
            entity_id: spider_id,
            entity_type: 17,
            spawn_index: spider_spawn,
        }
    );
}

#[v2k_test_support::retail_test]
fn no_contact_pass_preserves_noncanonical_float_bits_exactly() {
    let (session, mut manager) = level_one().expect("retail fixture");
    let player = manager.player_mut().expect("persistent type-46 player");
    player.position = [1.234_567, 100.123_45, 2.345_678];
    player.velocity = [0.012_345_67, -0.076_543_21, 0.034_567_89];
    let before = commit_snapshot(&manager);
    let mut hull = PlayerHull::default();

    let pass = resolve_player_active_contacts(
        &mut manager,
        &session.cache,
        PlayerActivePairFrame {
            resources: &session.cache,
            retail_tick: 0,
            player_craft: &PlayerCraft::new(),
            player_hull: &mut hull,
            scheduler: &mut SpecializedActorTaskScheduler::new(),
            world_fx: &mut WorldFx::new(),
            notifications: &mut GameplayNotifications::new(),
            extra_lives: 0,
        },
    )
    .expect("high-altitude no-contact pass");

    assert!(pass
        .core
        .dispositions
        .iter()
        .all(|disposition| !matches!(disposition, PairCandidateDisposition::Resolved { .. })));
    assert_eq!(commit_snapshot(&manager), before);
}

#[v2k_test_support::retail_test]
fn unresolved_overlapping_allocation_is_omitted_without_rolling_back_the_solid_pass() {
    let (session, mut manager) = level_one().expect("retail fixture");
    let type47 = entity_at_spawn(&manager, 11);
    let type47_id = type47.id;
    let unresolved_position = type47.position;
    manager
        .player_mut()
        .expect("persistent type-46 player")
        .position = unresolved_position;
    let before = commit_snapshot(&manager);
    let mut hull = PlayerHull::default();
    let hull_before = hull;

    let pass = resolve_player_active_contacts(
        &mut manager,
        &session.cache,
        PlayerActivePairFrame {
            resources: &session.cache,
            retail_tick: 0,
            player_craft: &PlayerCraft::new(),
            player_hull: &mut hull,
            scheduler: &mut SpecializedActorTaskScheduler::new(),
            world_fx: &mut WorldFx::new(),
            notifications: &mut GameplayNotifications::new(),
            extra_lives: 0,
        },
    )
    .expect("uncaptured type-47 must leave constructor-closed solids runnable");

    assert!(
        pass.core.dispositions.iter().all(|disposition| {
            let candidate_id = match disposition {
                PairCandidateDisposition::Skipped { candidate_id, .. }
                | PairCandidateDisposition::CancelledByCallbacks { candidate_id, .. }
                | PairCandidateDisposition::Resolved { candidate_id, .. } => *candidate_id,
            };
            candidate_id != type47_id
        }),
        "uncaptured type-47 must stay out of the visit list: {:?}",
        pass.core.dispositions
    );
    if pass
        .core
        .dispositions
        .iter()
        .all(|disposition| !matches!(disposition, PairCandidateDisposition::Resolved { .. }))
    {
        assert_eq!(commit_snapshot(&manager), before);
        assert_eq!(hull, hull_before);
    }
}

#[v2k_test_support::retail_test]
fn authored_main_base_program_resolves_a_real_level_one_overlap() {
    let (session, mut manager) = fresh_level_one().expect("retail fixture");
    let base_id = entity_at_spawn(&manager, 6).id;
    let RetailRuntimeValue::Known(Some(player_context)) = manager
        .player()
        .expect("persistent type-46 player")
        .current_behavior_context
    else {
        panic!("fresh post-Intro capture publishes the player context");
    };
    assert_eq!(player_context.descriptor_address(), 0x004C_DB38);
    assert_eq!(player_context.active_style().style_address(), 0x004C_D940);
    let RetailRuntimeValue::Known(Some(base_context)) =
        entity_at_spawn(&manager, 6).current_behavior_context
    else {
        panic!("fresh post-Intro capture publishes the Main Base context");
    };
    assert_eq!(base_context.descriptor_address(), 0x004C_9718);
    assert_eq!(base_context.active_style().style_address(), 0x004C_9480);
    let base = entity_at_spawn(&manager, 6);
    let base_position_before = base.position_raw();
    let base_velocity_before = base.velocity_raw();
    let craft = install_base_damage_impact(&mut manager);
    let player_position_before = manager.player().expect("persistent player").position_raw();
    let mut hull = PlayerHull::default();

    let pass = resolve_player_active_contacts(
        &mut manager,
        &session.cache,
        PlayerActivePairFrame {
            resources: &session.cache,
            retail_tick: 12_010,
            player_craft: &craft,
            player_hull: &mut hull,
            scheduler: &mut SpecializedActorTaskScheduler::new(),
            world_fx: &mut WorldFx::new(),
            notifications: &mut GameplayNotifications::new(),
            extra_lives: 0,
        },
    )
    .expect("authored Main Base overlap");

    assert!(
        pass.core.dispositions.iter().any(|disposition| matches!(
            disposition,
            PairCandidateDisposition::Resolved { candidate_id, .. } if *candidate_id == base_id
        )),
        "{:?}",
        pass.core.dispositions
    );
    let base = entity_at_spawn(&manager, 6);
    assert_eq!(base.position_raw(), base_position_before);
    assert_eq!(base.velocity_raw(), base_velocity_before);
    let player = manager.player().expect("persistent player");
    assert_ne!(player.position_raw(), player_position_before);
    assert_ne!(player.velocity_raw(), [0, -4_096, 0]);
    assert_eq!(
        player.collision.health_raw,
        RetailRuntimeValue::Known(hull.health_raw)
    );
    assert_eq!(
        player.collision.pre_health_damage_buffer_raw,
        RetailRuntimeValue::Known(hull.pre_health_damage_buffer_raw)
    );
    assert!(hull.health_raw > 0);
    assert!(hull.health_raw < hull.profile().max_health_raw);
    // Main Base sound 7 is the independently ordered accepted-projectile
    // presentation field. Its generic active-collision hit sound is null.
    assert!(pass.sounds.is_empty());
}

#[v2k_test_support::retail_test]
fn authored_hive_resolves_a_real_level_one_overlap() {
    let (session, mut manager) = level_one().expect("retail fixture");
    let hive = entity_at_spawn(&manager, 24);
    assert_eq!(hive.entity_type, 67);
    assert_eq!(hive.model_index, Some(341));
    assert_eq!(
        hive.collision.pair_callbacks,
        audited_local_callbacks(0, 0),
        "type-67 constructor/census pair identity must be closed"
    );
    assert!(
        session.cache.collision_model(341).is_some(),
        "hive1xa collision program"
    );
    let hive_id = hive.id;
    let hive_position_before = hive.position_raw();
    let hive_velocity_before = hive.velocity_raw();
    let player = manager.player_mut().expect("persistent player");
    player.heading = std::f32::consts::FRAC_PI_2;
    player.set_motion_raw(hive_position_before, [0, -4_096, 0]);
    let player_position_before = player.position_raw();
    let craft = PlayerCraft::new();
    let mut hull = PlayerHull::default();

    let pass = resolve_player_active_contacts(
        &mut manager,
        &session.cache,
        PlayerActivePairFrame {
            resources: &session.cache,
            retail_tick: 12_010,
            player_craft: &craft,
            player_hull: &mut hull,
            scheduler: &mut SpecializedActorTaskScheduler::new(),
            world_fx: &mut WorldFx::new(),
            notifications: &mut GameplayNotifications::new(),
            extra_lives: 0,
        },
    )
    .expect("authored Hive overlap");

    assert!(
        pass.core.dispositions.iter().any(|disposition| matches!(
            disposition,
            PairCandidateDisposition::Resolved { candidate_id, .. } if *candidate_id == hive_id
        )),
        "{:?}",
        pass.core.dispositions
    );
    let hive = entity_at_spawn(&manager, 24);
    assert_eq!(hive.position_raw(), hive_position_before);
    assert_eq!(hive.velocity_raw(), hive_velocity_before);
    let player = manager.player().expect("persistent player");
    assert_ne!(
        player.position_raw(),
        player_position_before,
        "fixed hive body must displace the overlapping player"
    );
}

#[v2k_test_support::retail_test]
fn authored_hive_accepts_a_surviving_primary_hit() {
    let (_, mut manager) = level_one().expect("retail fixture");
    let hive = entity_at_spawn(&manager, 24);
    assert_eq!(
        hive.current_behavior_context
            .map(|context| { context.map(|context| context.active_style().style_address()) }),
        RetailRuntimeValue::Known(Some(0x004C_94C8)),
        "installed hive emitter must publish class-46 live style"
    );
    let hive_id = hive.id;
    let health_before = hive.collision.health_raw;
    let CheckedProjectileDamageOutcome::Applied(applied) = manager
        .apply_audited_base_factory_projectile_damage(
            hive_id,
            CheckedProjectileDamageRequest {
                delivery: DamageDeliveryRecord {
                    packet: PRIMARY_PROJECTILE_DAMAGE_PACKET,
                    source_entity_type_raw: 46,
                    owner_handle: manager.player().expect("player projectile owner").id,
                },
                entry: EntityHitEntry::PrimaryProjectile,
                retail_tick: 11,
            },
        )
    else {
        panic!("live Level-1 hive must accept a surviving primary hit");
    };
    assert_eq!(applied.transition.accepted_damage_raw, 1_800);
    let health_after = entity_at_spawn(&manager, 24).collision.health_raw;
    assert_ne!(health_after, health_before);
}

#[v2k_test_support::retail_test]
fn authored_hive_lethal_hit_fails_closed_while_the_level_one_cohort_lives() {
    let (_, mut manager) = level_one().expect("retail fixture");
    let hive_id = entity_at_spawn(&manager, 24).id;
    let first = manager.apply_audited_base_factory_projectile_damage(
        hive_id,
        CheckedProjectileDamageRequest {
            delivery: DamageDeliveryRecord {
                packet: PRIMARY_PROJECTILE_DAMAGE_PACKET,
                source_entity_type_raw: 46,
                owner_handle: manager.player().expect("player projectile owner").id,
            },
            entry: EntityHitEntry::PrimaryProjectile,
            retail_tick: 96_800,
        },
    );
    assert!(
        matches!(first, CheckedProjectileDamageOutcome::Applied(_)),
        "first primary hit must leave the hive alive: {first:?}"
    );
    let second = manager.apply_audited_base_factory_projectile_damage(
        hive_id,
        CheckedProjectileDamageRequest {
            delivery: DamageDeliveryRecord {
                packet: PRIMARY_PROJECTILE_DAMAGE_PACKET,
                source_entity_type_raw: 46,
                owner_handle: manager.player().expect("player projectile owner").id,
            },
            entry: EntityHitEntry::PrimaryProjectile,
            retail_tick: 97_200,
        },
    );
    assert_eq!(
        second,
        CheckedProjectileDamageOutcome::Unresolved {
            target_id: hive_id,
            reason: CheckedProjectileDamageUnresolved::HiveObjectiveHostilesRemain,
        }
    );
    let hive = entity_at_spawn(&manager, 24);
    assert_eq!(hive.model_index, Some(341));
    assert!(manager
        .iter_all()
        .filter(|entity| entity.entity_type == 67)
        .all(|entity| {
            entity
                .authored_radial_emitter
                .as_ref()
                .is_none_or(|emitter| !emitter.dying_slot0())
        }));
    assert!(
        !manager.iter().any(|entity| entity.entity_type == 111),
        "a live cohort must not spawn a type-111 inside the hive"
    );
}

#[v2k_test_support::retail_test]
fn live_cargo_makes_the_player_damage_modifier_fail_closed() {
    let (session, mut manager) = level_one().expect("retail fixture");
    let weight = entity_at_spawn(&manager, 5);
    let weight_id = weight.id;
    let weight_position = weight.position;
    manager.player_mut().expect("persistent player").position = weight_position;
    manager.queue_beam(BeamCommand::Collect);
    assert_eq!(
        manager.tick_beam(81_000),
        Some(BeamOutcome::Collected {
            entity_id: weight_id
        })
    );
    assert!(matches!(
        manager.attached_cargo_mass_state(),
        RetailRuntimeValue::Known(mass) if mass > 0
    ));

    let craft = install_base_damage_impact(&mut manager);
    let before = commit_snapshot(&manager);
    let mut hull = PlayerHull::default();
    let hull_before = hull;
    let error = resolve_player_active_contacts(
        &mut manager,
        &session.cache,
        PlayerActivePairFrame {
            resources: &session.cache,
            retail_tick: 12_010,
            player_craft: &craft,
            player_hull: &mut hull,
            scheduler: &mut SpecializedActorTaskScheduler::new(),
            world_fx: &mut WorldFx::new(),
            notifications: &mut GameplayNotifications::new(),
            extra_lives: 0,
        },
    )
    .expect_err("nonempty modifier context must not be treated as identity");

    assert!(matches!(
        error,
        PlayerActivePairError::Core(ActivePairUnresolved::Callback { .. })
    ));
    assert_eq!(commit_snapshot(&manager), before);
    assert_eq!(hull, hull_before);
}

#[v2k_test_support::retail_test]
fn later_campaign_main_base_is_collidable_and_takes_projectile_damage() {
    let (session, mut manager) = later_campaign_world().expect("retail fixture");

    let player = manager.player().expect("campaign arrival type-46 player");
    assert_eq!(
        player.collision.pair_callbacks,
        EntityPairCallbackRuntimeState {
            component_contact: RetailRuntimeValue::Known([
                PairComponentContactPolicy::None,
                PairComponentContactPolicy::None,
                PairComponentContactPolicy::None,
            ]),
            damage_modifier_address: RetailRuntimeValue::Known(Some(0x0044_84A0)),
            damage_modifier_identity_context_empty: RetailRuntimeValue::Unresolved,
            type_hit_callback_address: RetailRuntimeValue::Known(None),
            orientation_policy: RetailRuntimeValue::Unresolved,
        }
    );

    for entity_type in [6_u32, 66] {
        let entity = manager
            .iter()
            .find(|entity| entity.entity_type == entity_type)
            .unwrap_or_else(|| panic!("Medaeval type-{entity_type} spawn"));
        assert!(matches!(
            entity.collision.pair_callbacks.component_contact,
            RetailRuntimeValue::Known(callbacks)
                if callbacks
                    .into_iter()
                    .all(|callback| callback == PairComponentContactPolicy::None)
        ));
        assert_eq!(
            entity.collision.pair_callbacks.damage_modifier_address,
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(
            entity.collision.pair_callbacks.type_hit_callback_address,
            RetailRuntimeValue::Known(None)
        );
        assert!(matches!(
            entity.collision.pair_callbacks.orientation_policy,
            RetailRuntimeValue::Known(
                PairOrientationPolicy::LiveHeadingWithAuthoredPitchRoll { .. }
            )
        ));
        assert!(
            entity
                .model_index
                .and_then(|model_id| session.cache.collision_model(model_id))
                .is_some(),
            "type {entity_type} collision program"
        );
    }

    let base_id = manager
        .iter()
        .find(|entity| entity.entity_type == 6)
        .expect("Medaeval Main Base")
        .id;
    let health_before = manager
        .iter()
        .find(|entity| entity.id == base_id)
        .expect("Main Base remains live")
        .collision
        .health_raw;
    let CheckedProjectileDamageOutcome::Applied(applied) = manager
        .apply_audited_base_factory_projectile_damage(
            base_id,
            CheckedProjectileDamageRequest {
                delivery: DamageDeliveryRecord {
                    packet: PRIMARY_PROJECTILE_DAMAGE_PACKET,
                    source_entity_type_raw: 46,
                    owner_handle: manager.player().expect("player projectile owner").id,
                },
                entry: EntityHitEntry::PrimaryProjectile,
                retail_tick: 11,
            },
        )
    else {
        panic!("later-world Main Base must accept a surviving primary hit");
    };
    assert_eq!(applied.transition.accepted_damage_raw, 1_800);
    let health_after = manager
        .iter()
        .find(|entity| entity.id == base_id)
        .expect("surviving Main Base remains live")
        .collision
        .health_raw;
    assert_ne!(health_after, health_before);

    let base_position = manager
        .iter()
        .find(|entity| entity.id == base_id)
        .expect("Main Base remains live")
        .position_raw();
    let player = manager.player_mut().expect("persistent type-46 player");
    player.heading = std::f32::consts::FRAC_PI_2;
    player.set_motion_raw(base_position, [0, -4_096, 0]);
    let player_position_before = player.position_raw();
    let craft = PlayerCraft::new();
    let mut hull = PlayerHull::default();

    let pass = resolve_player_active_contacts(
        &mut manager,
        &session.cache,
        PlayerActivePairFrame {
            resources: &session.cache,
            retail_tick: 12_010,
            player_craft: &craft,
            player_hull: &mut hull,
            scheduler: &mut SpecializedActorTaskScheduler::new(),
            world_fx: &mut WorldFx::new(),
            notifications: &mut GameplayNotifications::new(),
            extra_lives: 0,
        },
    )
    .expect("later-world Main Base overlap");
    assert!(
        pass.core.dispositions.iter().any(|disposition| matches!(
            disposition,
            PairCandidateDisposition::Resolved { candidate_id, .. } if *candidate_id == base_id
        )),
        "{:?}",
        pass.core.dispositions
    );
    let player = manager.player().expect("persistent player");
    assert_ne!(player.position_raw(), player_position_before);
    assert_ne!(player.velocity_raw(), [0, -4_096, 0]);
}

#[v2k_test_support::retail_test]
fn later_campaign_hive_uses_constructor_pair_identity() {
    let (session, mut manager) = later_campaign_world().expect("retail fixture");
    let hive = manager
        .iter()
        .find(|entity| entity.entity_type == 67)
        .expect("retail fixture");
    assert!(matches!(
        hive.collision.pair_callbacks.component_contact,
        RetailRuntimeValue::Known(callbacks)
            if callbacks
                .into_iter()
                .all(|callback| callback == PairComponentContactPolicy::None)
    ));
    assert_eq!(
        hive.collision.pair_callbacks.damage_modifier_address,
        RetailRuntimeValue::Known(None)
    );
    assert_eq!(
        hive.collision.pair_callbacks.type_hit_callback_address,
        RetailRuntimeValue::Known(None)
    );
    assert!(matches!(
        hive.collision.pair_callbacks.orientation_policy,
        RetailRuntimeValue::Known(PairOrientationPolicy::LiveHeadingWithAuthoredPitchRoll { .. })
    ));
    let hive_id = hive.id;
    let hive_position = hive.position_raw();
    assert!(
        hive.model_index
            .and_then(|model_id| session.cache.collision_model(model_id))
            .is_some(),
        "later-world hive collision program"
    );

    let player = manager.player_mut().expect("persistent type-46 player");
    player.heading = std::f32::consts::FRAC_PI_2;
    player.set_motion_raw(hive_position, [0, -4_096, 0]);
    let player_position_before = player.position_raw();
    let craft = PlayerCraft::new();
    let mut hull = PlayerHull::default();
    let pass = resolve_player_active_contacts(
        &mut manager,
        &session.cache,
        PlayerActivePairFrame {
            resources: &session.cache,
            retail_tick: 12_010,
            player_craft: &craft,
            player_hull: &mut hull,
            scheduler: &mut SpecializedActorTaskScheduler::new(),
            world_fx: &mut WorldFx::new(),
            notifications: &mut GameplayNotifications::new(),
            extra_lives: 0,
        },
    )
    .expect("later-world Hive overlap");
    assert!(
        pass.core.dispositions.iter().any(|disposition| matches!(
            disposition,
            PairCandidateDisposition::Resolved { candidate_id, .. } if *candidate_id == hive_id
        )),
        "{:?}",
        pass.core.dispositions
    );
    let player = manager.player().expect("persistent player");
    assert_ne!(
        player.position_raw(),
        player_position_before,
        "later-world hive must displace the overlapping player"
    );
}

#[v2k_test_support::retail_test]
fn overlapping_fish_are_omitted_from_the_player_solid_pass_untouched() {
    // Fish keep unresolved pair orientation/component contact: the adapter
    // admits only closed-identity candidates, so overlapping fish are
    // omitted rather than contacted. Their task recovery needs the same
    // style-table evidence as the Type9 02CA0 matrix.
    let (session, mut manager) = authored_ordinary_world(22).expect("retail fixture");
    let fish = manager
        .iter_all()
        .find(|entity| matches!(entity.entity_type, 22 | 24 | 124))
        .expect("world22 fish");
    assert_eq!(fish.capability_flags & 0x1000, 0);
    let fish_id = fish.id;
    let fish_position = fish.position_raw();
    let before = (
        fish.position_raw(),
        fish.velocity_raw(),
        fish.collision.clone(),
    );
    manager
        .player_mut()
        .expect("persistent player")
        .set_motion_raw(fish_position, [0, -4_096, 0]);
    let mut hull = PlayerHull::default();
    let mut notifications = GameplayNotifications::new();
    let pass = resolve_player_active_contacts(
        &mut manager,
        &session.cache,
        PlayerActivePairFrame {
            resources: &session.cache,
            retail_tick: 12_010,
            player_craft: &PlayerCraft::new(),
            player_hull: &mut hull,
            scheduler: &mut SpecializedActorTaskScheduler::new(),
            world_fx: &mut WorldFx::new(),
            notifications: &mut notifications,
            extra_lives: 0,
        },
    )
    .expect("fish omission must not fail the solid pass");
    assert!(
        !pass.core.dispositions.iter().any(|disposition| matches!(
            disposition,
            PairCandidateDisposition::Resolved { candidate_id, .. } if *candidate_id == fish_id
        )),
        "{:?}",
        pass.core.dispositions
    );
    let fish = manager
        .iter_all()
        .find(|entity| entity.id == fish_id)
        .unwrap();
    assert_eq!(
        (
            fish.position_raw(),
            fish.velocity_raw(),
            fish.collision.clone()
        ),
        before
    );
}

#[v2k_test_support::retail_test]
fn beam_collect_never_targets_fish_without_capability() {
    let (_, mut manager) = authored_ordinary_world(22).expect("retail fixture");
    let fish_ids: Vec<u32> = manager
        .iter_all()
        .filter(|entity| matches!(entity.entity_type, 22 | 24 | 124))
        .map(|entity| entity.id)
        .collect();
    assert!(!fish_ids.is_empty());
    let fish_position = manager
        .iter_all()
        .find(|entity| entity.id == fish_ids[0])
        .unwrap()
        .position;
    manager.player_mut().unwrap().position = fish_position;
    manager.queue_beam(BeamCommand::Collect);
    for _ in 0..30 {
        if let Some(BeamOutcome::Collected { entity_id }) = manager.tick_beam(81_000) {
            assert!(
                !fish_ids.contains(&entity_id),
                "beam collected fish {entity_id}"
            );
        }
    }
    for id in fish_ids {
        assert_eq!(
            manager
                .iter_all()
                .find(|entity| entity.id == id)
                .unwrap()
                .attached_to,
            None
        );
    }
}
