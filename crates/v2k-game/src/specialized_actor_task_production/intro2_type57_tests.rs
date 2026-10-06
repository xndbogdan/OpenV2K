//! Native Type57 bat custody at the hit, death, and contact boundaries.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    damage::DamagePacket,
    entity_collision_state::{RetailRuntimeValue, DYING_STATE_BIT},
    intro2_contacts::{resolve_intro2_contacts, Intro2ContactFrame, Intro2ContactReport},
    intro2_radial::{apply_intro2_radial_damage, Intro2RadialFrame, Intro2RadialReport},
    intro2_type47_live::world::native_intro2_fixture,
    intro2_type57::{impact::apply_intro2_type57_particle_hit, Intro2Type57ContactOutcome},
    radial_damage::RadialDamageTemplate,
    static_damage::StaticDamageScheduler,
    world_fx::{BallisticDamageRequest, ParticleEntityImpact, WorldFx},
};

fn frame<'a>(
    resources: &'a mut ResourceCache,
    fx: &'a mut WorldFx,
    static_damage: &'a mut StaticDamageScheduler,
) -> SpecializedActorTaskProductionFrame<'a> {
    SpecializedActorTaskProductionFrame {
        world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
        hive_components: None,
        notification_phase: crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
        resources,
        world_fx: fx,
        static_damage,
        elapsed_micros: 20000,
        global_elapsed_micros: 20000,
        retail_tick: 2202,
        main_base_abort_active: false,
    }
}

fn prepare(manager: &mut EntityManager) -> u32 {
    let id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(1))
        .expect("Intro2 spawn 1 is Type57")
        .id;
    let entity = manager.entity_mut(id).unwrap();
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(u32::MAX, 0x0206_8000);
    entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    entity.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    id
}

#[v2k_test_support::retail_test]
fn type57_lethal_hit_transitions_to_alternate11_tumble() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_intro2_type57(&manager);
    let id = prepare(&mut manager);

    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    assert_eq!(entity.entity_type, 57);
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(5000));

    let mut fx = WorldFx::new();
    let impact = ParticleEntityImpact {
        source_particle_class: 38, // Intro2 tick 2201 lethal fireball
        impact_position_argument_va: 0,
        target_entity_id: id,
        position_world: [0.0, 1000.0, 0.0],
        velocity_raw: [0, 0, 8192],
        damage: Some(BallisticDamageRequest {
            packet: DamagePacket {
                channels: [1, 0],
                amounts_raw: [24_000, 0],
            },
            source_entity_type_at_birth: Some(13),
            source_owner_id: Some(1),
        }),
    };

    let result = apply_intro2_type57_particle_hit(
        &mut manager,
        &session.cache,
        &mut fx,
        &mut scheduler,
        impact,
        2201,
    );

    let outcome = match result {
        crate::intro2_type57::impact::Intro2Type57ImpactOutcome::Applied(outcome) => outcome,
        other => panic!("expected applied lethal outcome: {other:?}"),
    };
    assert!(outcome.death_publication.is_some());

    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
    assert_eq!(
        entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
        RetailRuntimeValue::Known(DYING_STATE_BIT)
    );
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::Intro2Type57Tumble(_))
    ));
    assert!(scheduler.intro2_type57_completed_owner(&manager, id));
    assert!(scheduler.intro2_type57_tumble_completed_owner(&manager, id));
}

#[v2k_test_support::retail_test]
fn type57_tumble_ticks_and_advances_kinematics() {
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_intro2_type57(&manager);
    let id = prepare(&mut manager);

    let mut fx = WorldFx::new();
    let owner = crate::intro2_type57::death::publish_intro2_type57_standard_death(
        &mut manager,
        id,
        &mut fx,
    )
    .expect("standard death publishes")
    .expect("owner returned");
    scheduler.register_intro2_type57_tumble(owner);

    let initial_ypr = manager
        .iter_all()
        .find(|e| e.id == id)
        .unwrap()
        .rotation_heading_pitch_roll_raw();

    let mut notifications = GameplayNotifications::new();
    let mut static_damage = StaticDamageScheduler::new();
    let pass = scheduler.tick(
        &mut manager,
        frame(&mut session.cache, &mut fx, &mut static_damage),
        &mut notifications,
    );

    assert!(pass.outcomes.iter().any(|o| matches!(
        o,
        SpecializedActorTaskProductionOutcome::Intro2Type57Tumble(
            crate::intro2_type57::Intro2Type57TumbleOutcome::Advanced { entity_id, .. }
        ) if *entity_id == id
    )));

    let updated_ypr = manager
        .iter_all()
        .find(|e| e.id == id)
        .unwrap()
        .rotation_heading_pitch_roll_raw();
    assert_ne!(initial_ypr, updated_ypr, "tumble advances rotation");
}

#[v2k_test_support::retail_test]
fn type57_tumble_terrain_contact_triggers_terminal_explosion() {
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_intro2_type57(&manager);
    let id = prepare(&mut manager);

    let mut fx = WorldFx::new();
    let owner = crate::intro2_type57::death::publish_intro2_type57_standard_death(
        &mut manager,
        id,
        &mut fx,
    )
    .unwrap()
    .unwrap();
    scheduler.register_intro2_type57_tumble(owner);

    // Place the entity on the ground to trigger terrain contact
    let terrain = session.cache.level_terrain().unwrap();
    let p = manager
        .iter_all()
        .find(|e| e.id == id)
        .unwrap()
        .position_raw();
    let ground_y = terrain.bilinear_height_raw(p[0], p[2]);
    manager
        .entity_mut(id)
        .unwrap()
        .set_motion_raw([p[0], ground_y - 10, p[2]], [0, -100, 0]);

    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();

    let reports = resolve_intro2_contacts(Intro2ContactFrame {
        entities: &mut manager,
        resources: &mut session.cache,
        world_fx: &mut fx,
        static_damage: &mut static_damage,
        notifications: &mut notifications,
        retail_tick: 2205,
        actor_tasks: &mut scheduler,
    });

    let type57_report = reports.into_iter().find_map(|r| match r {
        Intro2ContactReport::Type57 { entity_id, result } if entity_id == id => Some(result),
        _ => None,
    });

    match type57_report {
        Some(Intro2Type57ContactOutcome::Applied(report)) => {
            assert!(report.terrain_contact);
            assert!(report.terminal.is_some());
            let terminal = report.terminal.unwrap();
            assert!(terminal.finalized);
        }
        other => panic!("expected applied terrain terminal: {other:?}"),
    }

    assert!(manager.pending_actor_deferred_destroy_ids().contains(&id));
}

#[v2k_test_support::retail_test]
fn type57_dynamic_radial_damage_triggers_tumble() {
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_intro2_type57(&manager);
    let id = prepare(&mut manager);

    let pos = manager
        .iter_all()
        .find(|e| e.id == id)
        .unwrap()
        .position_raw();
    let mut fx = WorldFx::new();
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();

    let report = apply_intro2_radial_damage(
        &mut Intro2RadialFrame {
            active_terminal_calls: Vec::new(),
            entities: &mut manager,
            resources: &mut session.cache,
            world_fx: &mut fx,
            static_damage: &mut static_damage,
            notifications: &mut notifications,
            retail_tick: 2201,
            actor_tasks: &mut scheduler,
        },
        pos,
        RadialDamageTemplate {
            inner_radius_raw: 500,
            outer_radius_raw: 1000,
            impulse_raw: 2000,
            packet: DamagePacket {
                channels: [1, 3],
                amounts_raw: [35_000, 35_000],
            },
            trailing_raw: [-1, 0],
        },
    );

    match report {
        Intro2RadialReport::Applied { dynamic, .. } => {
            assert!(dynamic.death_publications.iter().any(|publ| matches!(
                publ,
                crate::entity::DynamicRadialDeathPublication::Intro2Type57Tumble(owner)
                    if owner.entity_id() == id
            )));
        }
        other => panic!("expected radial damage applied: {other:?}"),
    }
}
