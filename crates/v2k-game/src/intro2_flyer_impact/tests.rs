use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    damage::{DamagePacket, FUN_0043F780_DAMAGE_DELIVERY},
    entity::EntityManager,
    gameplay_notifications::GameplayNotifications,
    intro2_type47_live::world::native_intro2_fixture,
    shared_actor_impact::{
        apply_shared_actor_particle_hit, SharedActorImpactFrame, SharedActorImpactOutcome,
    },
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    world_fx::{BallisticDamageRequest, WorldFx},
};

fn fixture() -> (crate::session::GameSession, EntityManager) {
    let data = v2k_test_support::retail_dir();
    assert!(
        data.join("PRELOAD.DAT").is_file(),
        "retail PRELOAD is required"
    );
    assert!(
        data.join("Overlay/1X3XX.OVL").is_file(),
        "retail Intro2 is required"
    );
    let (session, manager, _) = native_intro2_fixture().expect("native Intro2 construction");
    (session, manager)
}

fn setup(manager: &mut EntityManager, spawn: usize) -> (u32, SpecializedActorTaskScheduler) {
    let id = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(spawn))
        .unwrap()
        .id;
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.entity_type, if spawn == 44 { 15 } else { 87 });
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(u32::MAX, 0x0606_8000);
    entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
    entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
    let owner = Intro2FlyerSchedulerOwner::adopt_published(entity).unwrap();
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.register_intro2_flyer(owner);
    assert!(scheduler.intro2_flyer_completed_owner(manager, id));
    (id, scheduler)
}

fn impact(id: u32, infected: bool, packet: DamagePacket) -> ParticleEntityImpact {
    ParticleEntityImpact {
        source_particle_class: if infected { 5 } else { 16 },
        impact_position_argument_va: if infected { 0x004d_cf48 } else { 0 },
        target_entity_id: id,
        position_world: [0.; 3],
        velocity_raw: [0, 0, 8192],
        damage: Some(BallisticDamageRequest {
            packet: if infected {
                FUN_0043F780_DAMAGE_DELIVERY.packet
            } else {
                packet
            },
            source_entity_type_at_birth: Some(34),
            source_owner_id: Some(25),
        }),
    }
}

fn packet(amount: i32) -> DamagePacket {
    DamagePacket {
        channels: [3, 0],
        amounts_raw: [amount, 0],
    }
}

#[v2k_test_support::retail_test]
fn native_flyer_class5_zero_filter_reselects_both_actual_births_without_primary_suffix() {
    for spawn in [44, 46] {
        let (session, mut manager) = fixture();
        let (id, mut scheduler) = setup(&mut manager, spawn);
        let before_health = manager.entity_mut(id).unwrap().collision.health_raw;
        let profile = match manager.entity_mut(id).unwrap().collision.damage_profile {
            RetailRuntimeValue::Known(profile) => profile,
            _ => panic!("actual damage profile must be owned"),
        };
        assert_eq!(
            FUN_0043F780_DAMAGE_DELIVERY
                .packet
                .filtered_raw(Some(&profile)),
            0
        );
        let mut fx = WorldFx::new();
        let mut oracle = WorldFx::new();
        let mut notifications = GameplayNotifications::new();
        for tick in [918, 922, 926] {
            let old_primary = manager
                .entity_mut(id)
                .unwrap()
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary);
            let old_rotation = manager
                .entity_mut(id)
                .unwrap()
                .rotation_heading_pitch_roll_raw();
            let outcome = apply_shared_actor_particle_hit(
                SharedActorImpactFrame {
                    resources: &session.cache,
                    entities: &mut manager,
                    world_fx: &mut fx,
                    scheduler: &mut scheduler,
                    notifications: &mut notifications,
                    retail_tick: tick,
                },
                impact(id, true, packet(100)),
            )
            .unwrap();
            let SharedActorImpactOutcome::Flyer(NativeFlyerImpactOutcome::Applied(result)) =
                outcome
            else {
                panic!("{outcome:?}")
            };
            assert_eq!(result.filtered_damage_raw, 0);
            assert_eq!(result.death_publication, None);
            let entity = manager.entity_mut(id).unwrap();
            assert_ne!(
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                old_primary
            );
            assert_eq!(entity.collision.health_raw, before_health);
            assert_eq!(
                entity.collision.last_hit_presentation_tick_at_0x34,
                RetailRuntimeValue::Known(17)
            );
            assert_eq!(
                entity.collision.state_flags_at_0x08.masked(0x2000),
                RetailRuntimeValue::Known(0x2000)
            );
            assert_eq!(
                entity.rotation_heading_pitch_roll_raw(),
                old_rotation.map(|word| word.wrapping_sub(1024))
            );
            assert!(scheduler.intro2_flyer_completed_owner(&manager, id));
            assert_eq!(
                fx.particle_count(),
                0,
                "11250 has no primary capability suffix"
            );
            assert_eq!(
                fx.pending_event_count(),
                0,
                "both authored +82 cues are null"
            );
            // C690 singleton selector + two6030 constructors, then11030's
            // three angle draws even though425590 returns zero force.
            for _ in 0..6 {
                oracle.next_shared_retail_random_u16();
            }
            assert_eq!(
                fx.next_shared_retail_random_u16(),
                oracle.next_shared_retail_random_u16()
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn native_flyer_primary_nonlethal_and_lethal_keep_authored_profiles_and_quiet_death() {
    for spawn in [44, 46] {
        for lethal in [false, true] {
            let (session, mut manager) = fixture();
            let (id, mut scheduler) = setup(&mut manager, spawn);
            let before_health = match manager.entity_mut(id).unwrap().collision.health_raw {
                RetailRuntimeValue::Known(health) => health,
                _ => panic!("actual birth health"),
            };
            assert!(before_health > 200 && before_health < 20000);
            let mut fx = WorldFx::new();
            let result = apply_native_flyer_particle_hit(
                crate::shared_actor_impact::SharedActorImpactFrame {
                    entities: &mut manager,
                    resources: &session.cache,
                    world_fx: &mut fx,
                    scheduler: &mut scheduler,
                    notifications: &mut GameplayNotifications::new(),
                    retail_tick: 1011,
                },
                impact(id, false, packet(if lethal { 10000 } else { 100 })),
            );
            let NativeFlyerImpactOutcome::Applied(result) = result else {
                panic!("{result:?}")
            };
            let expected = if lethal { 20000 } else { 200 };
            assert_eq!(result.filtered_damage_raw, expected);
            assert_eq!(result.death_publication.is_some(), lethal);
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(
                entity.collision.health_raw,
                RetailRuntimeValue::Known(if lethal { 0 } else { before_health - expected })
            );
            assert_eq!(
                entity.collision.last_hit_presentation_tick_at_0x34,
                RetailRuntimeValue::Known(1011)
            );
            assert!(
                fx.particle_count() > 0,
                "10EB0 capability8 suffix remains valid after quiet death"
            );
            if lethal {
                assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
                    .into_iter()
                    .all(|slot| entity.actor_tasks.task_in_slot(slot).is_none()));
                assert!(native_flyer_quiet_death_authenticates(&manager, id));
                assert!(!scheduler.intro2_flyer_completed_owner(&manager, id));
                assert!(matches!(
                    result.death_publication,
                    Some(NativeFlyingSurfaceDeathPublication::QuietDeath(_))
                ));
                let again = apply_native_flyer_particle_hit(
                    crate::shared_actor_impact::SharedActorImpactFrame {
                        entities: &mut manager,
                        resources: &session.cache,
                        world_fx: &mut fx,
                        scheduler: &mut scheduler,
                        notifications: &mut GameplayNotifications::new(),
                        retail_tick: 1012,
                    },
                    impact(id, true, packet(100)),
                );
                assert!(
                    matches!(again, NativeFlyerImpactOutcome::Applied(_)),
                    "{again:?}"
                );
                assert!(native_flyer_quiet_death_authenticates(&manager, id));
            } else {
                assert!(scheduler.intro2_flyer_completed_owner(&manager, id));
            }
        }
    }
}

#[v2k_test_support::retail_test]
fn native_flyer_primary_filtered_zero_stamps_but_has_no_accepted_suffix() {
    for spawn in [44, 46] {
        let (session, mut manager) = fixture();
        let (id, mut scheduler) = setup(&mut manager, spawn);
        let before_health = manager.entity_mut(id).unwrap().collision.health_raw;
        let mut fx = WorldFx::new();
        let result = apply_native_flyer_particle_hit(
            crate::shared_actor_impact::SharedActorImpactFrame {
                entities: &mut manager,
                resources: &session.cache,
                world_fx: &mut fx,
                scheduler: &mut scheduler,
                notifications: &mut GameplayNotifications::new(),
                retail_tick: 1200,
            },
            impact(id, false, packet(0)),
        );
        assert!(
            matches!(
                result,
                NativeFlyerImpactOutcome::Applied(LiveActorDamageOutcome {
                    filtered_damage_raw: 0,
                    ..
                })
            ),
            "{result:?}"
        );
        assert_eq!(
            manager
                .entity_mut(id)
                .unwrap()
                .collision
                .last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(1200)
        );
        assert_eq!(
            manager.entity_mut(id).unwrap().collision.health_raw,
            before_health
        );
        assert_eq!(fx.particle_count(), 0);
        assert_eq!(fx.pending_event_count(), 0);
        assert!(scheduler.intro2_flyer_completed_owner(&manager, id));
    }
}

#[v2k_test_support::retail_test]
fn native_flyer_committed_hit_prefix_parks_replacement_without_rng_replay() {
    for spawn in [44, 46] {
        let (session, mut manager) = fixture();
        let (id, mut scheduler) = setup(&mut manager, spawn);
        let old_primary = manager
            .entity_mut(id)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary);
        let before_health = manager.entity_mut(id).unwrap().collision.health_raw;
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .pair_callbacks
            .damage_modifier_address = RetailRuntimeValue::Known(Some(0x0040_1234));
        let mut fx = WorldFx::new();
        let mut oracle = WorldFx::new();
        let result = apply_native_flyer_particle_hit(
            crate::shared_actor_impact::SharedActorImpactFrame {
                entities: &mut manager,
                resources: &session.cache,
                world_fx: &mut fx,
                scheduler: &mut scheduler,
                notifications: &mut GameplayNotifications::new(),
                retail_tick: 555,
            },
            impact(id, false, packet(100)),
        );
        assert!(
            matches!(
                result,
                NativeFlyerImpactOutcome::Blocked {
                    committed_prefix: true,
                    reason: NativeFlyerImpactBlock::Damage(_),
                    ..
                }
            ),
            "{result:?}"
        );
        assert_ne!(
            manager
                .entity_mut(id)
                .unwrap()
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary),
            old_primary
        );
        assert_eq!(
            manager.entity_mut(id).unwrap().collision.health_raw,
            before_health
        );
        assert!(!scheduler.intro2_flyer_completed_owner(&manager, id));
        for _ in 0..6 {
            oracle.next_shared_retail_random_u16();
        }
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
        let result = apply_native_flyer_particle_hit(
            crate::shared_actor_impact::SharedActorImpactFrame {
                entities: &mut manager,
                resources: &session.cache,
                world_fx: &mut fx,
                scheduler: &mut scheduler,
                notifications: &mut GameplayNotifications::new(),
                retail_tick: 556,
            },
            impact(id, false, packet(100)),
        );
        assert!(
            matches!(
                result,
                NativeFlyerImpactOutcome::Blocked {
                    committed_prefix: false,
                    ..
                }
            ),
            "{result:?}"
        );
        assert_eq!(
            manager
                .entity_mut(id)
                .unwrap()
                .collision
                .last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(555)
        );
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
        assert_eq!(fx.particle_count(), 0);
    }
}

#[v2k_test_support::retail_test]
fn native_flyer_foreign_custody_and_static_route_reject_before_any_prefix() {
    for spawn in [44, 46] {
        let (session, mut manager) = fixture();
        let (id, mut scheduler) = setup(&mut manager, spawn);
        let mut fx = WorldFx::new();
        let mut oracle = WorldFx::new();
        let mut notifications = GameplayNotifications::new();
        let mut static_hit = impact(id, false, packet(100));
        static_hit.source_particle_class = 52;
        let result = apply_shared_actor_particle_hit(
            SharedActorImpactFrame {
                resources: &session.cache,
                entities: &mut manager,
                world_fx: &mut fx,
                scheduler: &mut scheduler,
                notifications: &mut notifications,
                retail_tick: 700,
            },
            static_hit,
        );
        assert!(matches!(
            result,
            Some(SharedActorImpactOutcome::UnsupportedStaticRouteTarget {
                entity_type: 15 | 87
            })
        ));
        manager.entity_mut(id).unwrap().intro2_flyer_frame_owner = None;
        let result = apply_native_flyer_particle_hit(
            crate::shared_actor_impact::SharedActorImpactFrame {
                entities: &mut manager,
                resources: &session.cache,
                world_fx: &mut fx,
                scheduler: &mut scheduler,
                notifications: &mut GameplayNotifications::new(),
                retail_tick: 701,
            },
            impact(id, true, packet(100)),
        );
        assert!(matches!(
            result,
            NativeFlyerImpactOutcome::Blocked {
                committed_prefix: false,
                ..
            }
        ));
        assert_eq!(
            manager
                .entity_mut(id)
                .unwrap()
                .collision
                .last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(17)
        );
        assert_eq!(
            manager
                .entity_mut(id)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .masked(0x2000),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
        assert_eq!(fx.particle_count(), 0);
    }
}
