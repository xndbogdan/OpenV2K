//! Type124's alternate class63: BAF0, then BC90's Type61 drop at the fish.

use crate::{
    actor_task_owner::ActorTaskSlot,
    class49_death::{
        begin_class49_standard_death, claim_class49_terminal, finish_class49_terminal,
        finished_terminal_hit_authenticates,
    },
    damage::DamagePacket,
    entity::EntityManager,
    entity_behavior::AUTO_PILOT_BEHAVIOR_PROGRAM,
    entity_collision_state::{
        RetailRuntimeValue, DEFERRED_DESTROY_PENDING_STATE_BIT, DYING_STATE_BIT, SURFACE_STATE_MASK,
    },
    gameplay_notifications::GameplayNotifications,
    player_hull::PlayerHull,
    session::GameSession,
    shared_actor_impact::{
        apply_playing_actor_particle_hit, PlayingActorImpactFrame, SharedActorImpactOutcome,
    },
    shared_fish::impact::SharedFishImpactOutcome,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
    world_fx::{BallisticDamageRequest, ParticleEntityImpact, WorldFx},
};

/// Every authored Type124: Reef (23), Coral (30) and Lagoon (34).
const CARRIER_WORLDS: [u32; 3] = [23, 30, 34];

fn fixture(world: u32) -> (GameSession, EntityManager, WorldFx, u32) {
    let (session, manager, fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(world);
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 124)
        .unwrap()
        .id;
    (session, manager, fx, id)
}

fn authored_payload(session: &GameSession, manager: &EntityManager, id: u32) -> u32 {
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    let spawn = &session.cache.level_desc().unwrap().entities[entity.authored_spawn_index.unwrap()];
    u32::from_le_bytes(spawn.extra[8..12].try_into().unwrap())
}

fn tasks(manager: &EntityManager, id: u32) -> Vec<Option<crate::actor_task_owner::ActorTaskId>> {
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .into_iter()
        .map(|slot| entity.actor_tasks.task_in_slot(slot))
        .collect()
}

/// The dropped Type61 is native: its receipt names the carrier, and its
/// constructor compares the carrier position with this tick's surface.
fn assert_dropped_power_up(
    session: &GameSession,
    manager: &EntityManager,
    carrier: u32,
    power_up: u32,
    payload: u32,
    position_raw: [i16; 3],
    retail_tick: u32,
) {
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == power_up)
        .unwrap();
    assert_eq!(entity.entity_type, 61);
    assert_eq!(entity.authored_spawn_index, None);
    assert_eq!(entity.power_up_payload_packed, Some(payload));
    assert_eq!(entity.position_raw(), position_raw);
    assert_eq!(entity.rotation_heading_pitch_roll_raw(), [0; 3]);
    assert!(crate::native_type61::allocation_authenticates(
        manager, power_up
    ));
    let birth = entity
        .native_type61_allocation
        .and_then(|allocation| allocation.auto_pilot_birth())
        .expect("class63 origin");
    assert_eq!(birth.carrier.entity_id, carrier);
    assert_eq!(birth.carrier_type, 124);
    assert_eq!(birth.payload_packed, payload);
    assert!(
        matches!(entity.initial_behavior, RetailRuntimeValue::Known(Some(selection)) if selection.program.class_id == 23)
    );
    let surface = crate::entity_initializer::constructor_surface_bits_at_tick(
        position_raw,
        session.cache.level_terrain().unwrap(),
        retail_tick,
        manager.common_environment_physics().waves_enabled,
    )
    .unwrap();
    assert_eq!(
        entity
            .collision
            .state_flags_at_0x08
            .masked(SURFACE_STATE_MASK),
        RetailRuntimeValue::Known(surface)
    );
}

fn assert_finished_carrier(manager: &EntityManager, id: u32) {
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
    assert_eq!(
        entity
            .collision
            .state_flags_at_0x08
            .masked(DYING_STATE_BIT | DEFERRED_DESTROY_PENDING_STATE_BIT | 0x0006_0000),
        RetailRuntimeValue::Known(DYING_STATE_BIT | DEFERRED_DESTROY_PENDING_STATE_BIT)
    );
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        panic!("class63 context")
    };
    assert_eq!(
        context.active_style().style_address(),
        AUTO_PILOT_BEHAVIOR_PROGRAM.initial_style.frame_address
    );
    assert!(finished_terminal_hit_authenticates(manager, id));
    assert!(manager.pending_actor_deferred_destroy_ids().contains(&id));
}

fn lethal_hit(id: u32) -> ParticleEntityImpact {
    ParticleEntityImpact {
        source_particle_class: 16,
        impact_position_argument_va: 0,
        target_entity_id: id,
        position_world: [0.0; 3],
        velocity_raw: [0, 0, 8192],
        damage: Some(BallisticDamageRequest {
            packet: DamagePacket {
                channels: [1, 0],
                amounts_raw: [1_000_000, 0],
            },
            source_entity_type_at_birth: Some(34),
            source_owner_id: Some(35),
        }),
    }
}

/// BC90 keeps the fish tasks (no A860), draws only the Type61 selector after
/// BAF0, appends the authored payload at the fish and stages removal.
#[v2k_test_support::retail_test]
fn class63_keeps_the_tasks_and_drops_the_authored_power_up() {
    for world in CARRIER_WORLDS {
        let (session, mut manager, mut fx, id) = fixture(world);
        let payload = authored_payload(&session, &manager, id);
        let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
        assert_eq!(entity.auto_pilot_payload_packed, Some(payload));
        let position = entity.position_raw();
        let tasks_before = tasks(&manager, id);
        assert!(tasks_before.iter().any(Option::is_some));
        let receipt = begin_class49_standard_death(&mut manager, id, &session.cache, &mut fx, 700)
            .unwrap()
            .expect("live fish");
        assert!(claim_class49_terminal(&mut manager, &receipt));
        let mut expected = fx.fork_for_main_base_abort_transaction();
        expected.next_shared_retail_random_u16(); // Type61's singleton selector.
        let completion =
            finish_class49_terminal(&mut manager, receipt, &session.cache, &mut fx).unwrap();
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16(),
            "world{world}: BC90 draws one constructor word"
        );
        assert!(completion.ring.is_none() && completion.ring_owner.is_none());
        let power_up = completion.power_up.expect("Type61 drop");
        assert_eq!(tasks(&manager, id), tasks_before, "BC90 never calls A860");
        assert_finished_carrier(&manager, id);
        assert_dropped_power_up(&session, &manager, id, power_up, payload, position, 700);
    }
}

/// A Playing particle kill runs the fish hit prefix, then the class63
/// terminal with the lent player; the owner retires and a re-hit before the
/// sweep reaches only the finished receipt.
#[v2k_test_support::retail_test]
fn a_lethal_playing_hit_drops_the_power_up_and_a_rehit_finds_the_receipt() {
    for world in CARRIER_WORLDS {
        let (mut session, mut manager, mut fx, id) = fixture(world);
        manager.cleanup_pending_actor_deferred_destroys();
        let payload = authored_payload(&session, &manager, id);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler.adopt_shared_fish(&manager) > 0);
        assert!(scheduler.shared_fish_completed_owner(&manager, id));
        let power_ups_before: Vec<u32> = manager
            .iter_all()
            .filter(|entity| entity.entity_type == 61)
            .map(|entity| entity.id)
            .collect();
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let mut hull = PlayerHull::default();
        let mut hit = |session: &mut GameSession,
                       manager: &mut EntityManager,
                       fx: &mut WorldFx,
                       scheduler: &mut SpecializedActorTaskScheduler| {
            apply_playing_actor_particle_hit(
                PlayingActorImpactFrame {
                    extra_lives: RetailRuntimeValue::Known(2),
                    resources: &mut session.cache,
                    entities: manager,
                    world_fx: fx,
                    scheduler,
                    notifications: &mut notifications,
                    static_damage: &mut static_damage,
                    player_hull: &mut hull,
                    retail_tick: 900,
                },
                lethal_hit(id),
            )
        };
        let position = manager
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap()
            .position_raw();
        let outcome = hit(&mut session, &mut manager, &mut fx, &mut scheduler);
        assert!(
            matches!(
                outcome,
                Some(SharedActorImpactOutcome::Fish(
                    SharedFishImpactOutcome::Applied(_)
                ))
            ),
            "world{world}: {outcome:?}"
        );
        assert_finished_carrier(&manager, id);
        assert!(!scheduler.shared_fish_completed_owner(&manager, id));
        let dropped: Vec<u32> = manager
            .iter_all()
            .filter(|entity| entity.entity_type == 61 && !power_ups_before.contains(&entity.id))
            .map(|entity| entity.id)
            .collect();
        assert_eq!(dropped.len(), 1, "world{world}: one Type61 drop");
        assert_dropped_power_up(&session, &manager, id, dropped[0], payload, position, 900);

        // Before 14990: the class63 style has no +28 hook and the fish is
        // already dying, so a re-hit is a bare reaction with no new drop.
        let outcome = hit(&mut session, &mut manager, &mut fx, &mut scheduler);
        let Some(SharedActorImpactOutcome::Fish(SharedFishImpactOutcome::Applied(applied))) =
            outcome
        else {
            panic!("world{world}: {outcome:?}")
        };
        assert!(applied.death_publication.is_none());
        assert_eq!(
            manager
                .iter_all()
                .filter(|entity| entity.entity_type == 61)
                .count(),
            power_ups_before.len() + 1
        );
        assert_finished_carrier(&manager, id);
        // The blast may also retire neighbours; the carrier is among the swept.
        assert!(manager
            .cleanup_pending_actor_deferred_destroys()
            .contains(&id));
        assert!(crate::native_type61::allocation_authenticates(
            &manager, dropped[0]
        ));
    }
}

/// An attached particle lends its own Playing world: a lethal burn takes
/// Type124 through class63 and drops its Type61 exactly as a particle kill.
#[v2k_test_support::retail_test]
fn attached_particle_damage_takes_type124_through_class63() {
    let (mut session, mut manager, mut fx, id) = fixture(34);
    manager.cleanup_pending_actor_deferred_destroys();
    let payload = authored_payload(&session, &manager, id);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert!(scheduler.adopt_shared_fish(&manager) > 0);
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.health_raw = RetailRuntimeValue::Known(1);
    let position = entity.position_raw();
    let newest = manager.iter_all().map(|entity| entity.id).max().unwrap();
    let mut hull = PlayerHull::default();
    let result = crate::attached_particle_damage::apply_attached_particle_damage(
        crate::attached_particle_damage::AttachedParticleDamageFrame {
            entities: &mut manager,
            resources: &mut session.cache,
            world_fx: &mut fx,
            static_damage: &mut StaticDamageScheduler::new(),
            scheduler: &mut scheduler,
            notifications: &mut GameplayNotifications::new(),
            retail_tick: 81,
            world: crate::attached_particle_damage::AttachedParticleDamageWorld::Playing {
                player_hull: &mut hull,
                extra_lives: RetailRuntimeValue::Known(2),
            },
        },
        crate::world_fx::AttachedParticleDamageRequest {
            target_handle: id,
            packet_va: 0x004c_c048,
            source_entity_type_raw: (-5_i32) as u32,
            current_emitter: None,
            ratio_numerator: 255,
            ratio_denominator: 255,
        },
    );
    assert!(matches!(result, Ok(damage) if damage > 0), "{result:?}");
    assert_finished_carrier(&manager, id);
    assert!(!scheduler.shared_fish_completed_owner(&manager, id));
    let drop = manager
        .iter_all()
        .find(|entity| entity.id > newest && entity.entity_type == 61)
        .expect("Type61 drop")
        .id;
    assert_dropped_power_up(&session, &manager, id, drop, payload, position, 81);
}
