use super::*;
use crate::intro2_type17::intro2_type17_allocation_authenticates;
use crate::{
    actor_task_owner::ActorTaskSlot,
    damage::{DamagePacket, FUN_0043F780_DAMAGE_DELIVERY},
    entity::EntityManager,
    entity_behavior::{
        audited_behavior_style, behavior_program, BehaviorChoiceListSource, BehaviorContextRuntime,
    },
    gameplay_notifications::GameplayNotifications,
    intro2_type47_live::world::native_intro2_fixture,
    live_actor_checked_damage::LiveActorDamagePhase,
    specialized_actor_task_production::{
        SpecializedActorTaskFamily, SpecializedActorTaskScheduler,
    },
    world_fx::{BallisticDamageRequest, WorldFx},
};

fn impact(id: u32, infected: bool, amount: i32) -> ParticleEntityImpact {
    ParticleEntityImpact {
        source_particle_class: if infected { 5 } else { 16 },
        impact_position_argument_va: if infected { 0x004D_CF48 } else { 0 },
        target_entity_id: id,
        position_world: [0.0; 3],
        velocity_raw: [0, 0, 8192],
        damage: Some(BallisticDamageRequest {
            packet: if infected {
                FUN_0043F780_DAMAGE_DELIVERY.packet
            } else {
                DamagePacket {
                    channels: [1, 0],
                    amounts_raw: [amount, 0],
                }
            },
            // F780 owns a static zero/zero delivery independently of these.
            source_entity_type_at_birth: Some(34),
            source_owner_id: Some(35),
        }),
    }
}

fn spawn_id(manager: &EntityManager, spawn: usize) -> u32 {
    manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(spawn))
        .unwrap()
        .id
}

fn prepare_target(manager: &mut EntityManager, spawn: usize) -> u32 {
    let id = spawn_id(manager, spawn);
    let entity = manager.entity_mut(id).unwrap();
    assert!(intro2_type17_allocation_authenticates(entity));
    // Controlled local reaction/admission flags; native tasks, metadata and
    // component receipts come from the actual authored birth.
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(u32::MAX, 0x0406_8000);
    entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
    entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
    id
}

fn set_style(entity: &mut Entity, class: u32, variant: u8) {
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(
        BehaviorContextRuntime::named_audited(
            behavior_program(class).unwrap(),
            u32::from(variant),
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            RetailRuntimeValue::Known(None),
            RetailRuntimeValue::Known(0),
            *audited_behavior_style(class, variant).unwrap(),
        )
        .unwrap(),
    ));
}

fn full_rate_fx() -> WorldFx {
    let mut fx = WorldFx::new();
    for _ in 0..8 {
        fx.advance_frame_pacing(20_000);
    }
    fx
}

#[test]
fn native_type17_authenticates_primary_and_infected_style_asymmetries() {
    for (class, variant, infected, primary) in [
        (
            9,
            0,
            ImpactCallbackPolicy::ReselectBehavior,
            ImpactCallbackPolicy::ReselectBehavior,
        ),
        (
            9,
            1,
            ImpactCallbackPolicy::ReselectBehavior,
            ImpactCallbackPolicy::ReselectBehavior,
        ),
        (
            9,
            2,
            ImpactCallbackPolicy::None,
            ImpactCallbackPolicy::CapturePeopleCleanup,
        ),
        (
            9,
            3,
            ImpactCallbackPolicy::None,
            ImpactCallbackPolicy::CapturePeopleCleanup,
        ),
        (
            9,
            4,
            ImpactCallbackPolicy::None,
            ImpactCallbackPolicy::CapturePeopleCleanup,
        ),
        (
            9,
            5,
            ImpactCallbackPolicy::None,
            ImpactCallbackPolicy::CapturePeopleCleanup,
        ),
        (
            10,
            0,
            ImpactCallbackPolicy::ReselectBehavior,
            ImpactCallbackPolicy::None,
        ),
        (
            10,
            1,
            ImpactCallbackPolicy::ReselectBehavior,
            ImpactCallbackPolicy::None,
        ),
        (
            10,
            2,
            ImpactCallbackPolicy::None,
            ImpactCallbackPolicy::None,
        ),
        (
            33,
            0,
            ImpactCallbackPolicy::ReselectBehavior,
            ImpactCallbackPolicy::ReselectBehavior,
        ),
        (
            33,
            1,
            ImpactCallbackPolicy::ReselectBehavior,
            ImpactCallbackPolicy::ReselectBehavior,
        ),
        (
            12,
            0,
            ImpactCallbackPolicy::None,
            ImpactCallbackPolicy::None,
        ),
        (
            12,
            1,
            ImpactCallbackPolicy::None,
            ImpactCallbackPolicy::None,
        ),
    ] {
        let style = ActiveBehaviorStyle::Audited(*audited_behavior_style(class, variant).unwrap());
        assert_eq!(callback_policy(style, HitEntry::Infected), Some(infected));
        assert_eq!(callback_policy(style, HitEntry::Primary), Some(primary));
        assert_eq!(
            callback_policy(style, HitEntry::Cured),
            Some(if class == 10 { infected } else { primary })
        );
    }
    for entry in [HitEntry::Primary, HitEntry::Infected] {
        assert_eq!(
            callback_policy(ActiveBehaviorStyle::InitializerFailureFallback, entry),
            Some(ImpactCallbackPolicy::None)
        );
        assert_eq!(
            callback_policy(
                ActiveBehaviorStyle::Audited(*audited_behavior_style(7, 0).unwrap()),
                entry
            ),
            None
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type17_antidote_runs_c690_and_cure_cue_before_checked_damage_without_primary_suffix() {
    for spawn in [4, 30] {
        let Some((session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = prepare_target(&mut manager, spawn);
        let metadata = manager.type_runtime_metadata(17).unwrap().clone();
        let RetailRuntimeValue::Known(cue) = metadata.cured_model_presentation_sound_id else {
            panic!()
        };
        let filtered = metadata
            .damage_profile
            .unwrap()
            .filter(crate::damage::FUN_0043F7C0_DAMAGE_PACKET);
        let entity = manager.entity_mut(id).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x2000, 0x2000);
        let primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.adopt_intro2_type17(&manager);
        let mut fx = full_rate_fx();
        let mut delivery = impact(id, false, 0);
        delivery.source_particle_class = 6;
        delivery.damage.as_mut().unwrap().packet = crate::damage::FUN_0043F7C0_DAMAGE_PACKET;
        let result = apply_intro2_type17_particle_hit(
            crate::shared_actor_impact::SharedActorImpactFrame {
                entities: &mut manager,
                resources: &session.cache,
                world_fx: &mut fx,
                scheduler: &mut scheduler,
                notifications: &mut GameplayNotifications::new(),
                retail_tick: 251,
            },
            delivery,
        );
        let Intro2Type17ImpactOutcome::Applied(result) = result else {
            panic!("{result:?}")
        };
        assert_eq!(result.filtered_damage_raw, filtered);
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(
            entity.collision.health_raw,
            RetailRuntimeValue::Known(5000 - filtered)
        );
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(0x2000),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            entity.collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(17)
        );
        assert_ne!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            primary
        );
        fx.process_pending();
        assert_eq!(
            fx.take_positional_sounds()
                .iter()
                .map(|sound| sound.sound_id)
                .collect::<Vec<_>>(),
            cue.into_iter().map(usize::from).collect::<Vec<_>>()
        );
        assert_eq!(fx.particle_count(), 0);
    }
}

#[v2k_test_support::retail_test]
fn native_type17_antidote_requires_capture_cleanup_custody_before_impulse_or_damage() {
    for variant in 2..=5 {
        let Some((session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = prepare_target(&mut manager, 30);
        let entity = manager.entity_mut(id).unwrap();
        set_style(entity, 9, variant);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x2000, 0x2000);
        let before_pose = (
            entity.velocity_raw(),
            entity.rotation_heading_pitch_roll_raw(),
        );
        let mut delivery = impact(id, false, 0);
        delivery.source_particle_class = 6;
        delivery.damage.as_mut().unwrap().packet = crate::damage::FUN_0043F7C0_DAMAGE_PACKET;
        let mut fx = WorldFx::new();
        let result = apply_intro2_type17_particle_hit(
            crate::shared_actor_impact::SharedActorImpactFrame {
                entities: &mut manager,
                resources: &session.cache,
                world_fx: &mut fx,
                scheduler: &mut SpecializedActorTaskScheduler::new(),
                notifications: &mut GameplayNotifications::new(),
                retail_tick: 251,
            },
            delivery,
        );
        assert!(
            matches!(
                result,
                Intro2Type17ImpactOutcome::Blocked {
                    reason: Intro2Type17ImpactBlock::Capture(super::super::capture::CaptureBlock {
                        reason: "capture row callback custody",
                        ..
                    }),
                    committed_prefix: true,
                }
            ),
            "{result:?}"
        );
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(0x2000),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(5000));
        assert_eq!(
            (
                entity.velocity_raw(),
                entity.rotation_heading_pitch_roll_raw()
            ),
            before_pose
        );
        assert_eq!(
            entity.collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(17)
        );
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            38,
            "D040 custody failure precedes11030 RNG"
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type17_both_births_reselect_before_primary_or_filtered_zero_infected_damage() {
    for spawn in [4, 30] {
        for (infected, amount, filtered) in [(false, 2500, 500), (false, 2000, 0), (true, 2000, 0)]
        {
            let Some((session, mut manager, _)) = native_intro2_fixture() else {
                return;
            };
            let id = prepare_target(&mut manager, spawn);
            let entity = manager.entity_mut(id).unwrap();
            let primary_before = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
            let position = entity.position_raw();
            let mut scheduler = SpecializedActorTaskScheduler::new();
            assert_eq!(scheduler.adopt_intro2_type17(&manager), 2);
            let mut fx = full_rate_fx();
            let delivery = impact(id, infected, amount);
            if infected {
                assert_eq!(
                    delivery.damage_delivery_record(),
                    Some(FUN_0043F780_DAMAGE_DELIVERY)
                );
            }
            let result = apply_intro2_type17_particle_hit(
                crate::shared_actor_impact::SharedActorImpactFrame {
                    entities: &mut manager,
                    resources: &session.cache,
                    world_fx: &mut fx,
                    scheduler: &mut scheduler,
                    notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                    retail_tick: 251,
                },
                delivery,
            );
            let Intro2Type17ImpactOutcome::Applied(result) = result else {
                panic!("spawn{spawn} infected{infected}: {result:?}")
            };
            assert_eq!(result.filtered_damage_raw, filtered);
            assert!(result.death_publication.is_none());
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(
                entity.collision.health_raw,
                RetailRuntimeValue::Known(5000 - filtered)
            );
            assert_eq!(
                entity.collision.last_hit_presentation_tick_at_0x34,
                RetailRuntimeValue::Known(if infected { 17 } else { 251 })
            );
            assert_ne!(
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                primary_before
            );
            assert_eq!(
                state_bits(entity, 0x2000).unwrap(),
                if infected { 0x2000 } else { 0 }
            );
            assert_eq!(
                scheduler.adopt_intro2_type17(&manager),
                0,
                "C690 already transferred its replacement owner"
            );
            fx.process_pending();
            let sounds = fx.take_positional_sounds();
            assert_eq!(
                sounds
                    .iter()
                    .map(|sound| sound.sound_id)
                    .collect::<Vec<_>>(),
                if infected {
                    vec![92]
                } else if filtered != 0 {
                    vec![84]
                } else {
                    vec![]
                }
            );
            assert!(sounds.iter().all(|sound| sound.frequency_q16 == 0x10000));
            let particles = fx.prepare_presentation([640, 480], 0x3000, |_| {
                v2k_render::ParticleCenterProjection {
                    screen: [320, 240],
                    depth_raw: 1000,
                    clip: 0,
                }
            });
            assert_eq!(
                particles.particles().len(),
                if !infected && filtered != 0 { 2 } else { 0 }
            );
            let extent = session.cache.global_model(256).unwrap().radius;
            for particle in particles.particles() {
                assert_eq!(particle.particle.source_class, 5);
                assert_eq!(particle.particle.owner_id, Some(id));
                assert_eq!(
                    particle.particle.position,
                    [
                        f32::from(position[0] as u16) / 256.0,
                        f32::from(position[1]) / 256.0,
                        f32::from(position[2].wrapping_sub(extent as i16) as u16) / 256.0
                    ]
                );
            }
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type17_capture_styles_without_row_custody_block_primary_but_allow_zero_infected_reaction()
{
    for variant in 2..=5 {
        for infected in [false, true] {
            let Some((session, mut manager, _)) = native_intro2_fixture() else {
                return;
            };
            let id = prepare_target(&mut manager, 30);
            let entity = manager.entity_mut(id).unwrap();
            // Only the raw style is controlled. Infected entry has no hook, so
            // it must not inspect or invent Capture's attached task state.
            set_style(entity, 9, variant);
            let before_context = entity.current_behavior_context;
            let primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
            let angles = entity.rotation_heading_pitch_roll_raw();
            let velocity = entity.velocity_raw();
            let mut fx = WorldFx::new();
            let result = apply_intro2_type17_particle_hit(
                crate::shared_actor_impact::SharedActorImpactFrame {
                    entities: &mut manager,
                    resources: &session.cache,
                    world_fx: &mut fx,
                    scheduler: &mut SpecializedActorTaskScheduler::new(),
                    notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                    retail_tick: 251,
                },
                impact(id, infected, 2500),
            );
            if infected {
                let Intro2Type17ImpactOutcome::Applied(result) = result else {
                    panic!("{result:?}")
                };
                assert_eq!(result.filtered_damage_raw, 0);
                assert_eq!(
                    fx.next_shared_retail_random_u16(),
                    2437,
                    "zero-strength11030 still consumes three words"
                );
            } else {
                assert!(
                    matches!(
                        result,
                        Intro2Type17ImpactOutcome::Blocked {
                            reason: Intro2Type17ImpactBlock::Capture(
                                super::super::capture::CaptureBlock {
                                    reason: "capture row callback custody",
                                    ..
                                }
                            ),
                            committed_prefix: true
                        }
                    ),
                    "{result:?}"
                );
                assert_eq!(fx.next_shared_retail_random_u16(), 38);
            }
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(5000));
            assert_eq!(entity.current_behavior_context, before_context);
            assert_eq!(
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                primary
            );
            assert_eq!(entity.velocity_raw(), velocity);
            assert_eq!(
                entity.rotation_heading_pitch_roll_raw(),
                if infected {
                    angles.map(|a| a.wrapping_sub(0x400))
                } else {
                    angles
                }
            );
            assert_eq!(
                entity.collision.last_hit_presentation_tick_at_0x34,
                RetailRuntimeValue::Known(if infected { 17 } else { 251 })
            );
            fx.process_pending();
            assert_eq!(
                fx.take_positional_sounds()
                    .iter()
                    .map(|sound| sound.sound_id)
                    .collect::<Vec<_>>(),
                if infected { vec![92] } else { vec![] }
            );
            assert_eq!(fx.particle_count(), 0);
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type17_lethal_primary_publishes_class12_before_later_hits() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = prepare_target(&mut manager, 4);
    manager.entity_mut(id).unwrap().collision.health_raw = RetailRuntimeValue::Known(1);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_intro2_type17(&manager);
    let mut fx = full_rate_fx();
    let first = apply_intro2_type17_particle_hit(
        crate::shared_actor_impact::SharedActorImpactFrame {
            entities: &mut manager,
            resources: &session.cache,
            world_fx: &mut fx,
            scheduler: &mut scheduler,
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            retail_tick: 251,
        },
        impact(id, false, 2500),
    );
    let Intro2Type17ImpactOutcome::Applied(first) = first else {
        panic!("{first:?}")
    };
    assert!(first.death_publication.is_some());
    assert_eq!(
        scheduler.family_for(id),
        Some(SpecializedActorTaskFamily::Intro2CommonDying)
    );
    assert!(Intro2CommonDyingOwner::adopt(&manager, id).is_ok());
    assert_eq!(
        fx.particle_count(),
        2,
        "lethal delivery retains capability8 suffix"
    );
    fx.process_pending();
    assert_eq!(
        fx.take_positional_sounds()
            .iter()
            .map(|sound| sound.sound_id)
            .collect::<Vec<_>>(),
        [94]
    );
    let primary = manager
        .entity_mut(id)
        .unwrap()
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary);
    let second = apply_intro2_type17_particle_hit(
        crate::shared_actor_impact::SharedActorImpactFrame {
            entities: &mut manager,
            resources: &session.cache,
            world_fx: &mut fx,
            scheduler: &mut scheduler,
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            retail_tick: 252,
        },
        impact(id, false, 2500),
    );
    let Intro2Type17ImpactOutcome::Applied(second) = second else {
        panic!("{second:?}")
    };
    assert!(second.death_publication.is_none());
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
    assert_eq!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        primary
    );
    assert_eq!(fx.particle_count(), 4);
    fx.process_pending();
    assert!(fx.take_positional_sounds().is_empty());
}

#[v2k_test_support::retail_test]
fn native_type17_living_lethal_and_corpse_blood_infects_the_live_world_terrain() {
    use crate::{
        playing_particle_host::PlayingParticleHost,
        resource_cache::ResourceCache,
        shared_actor_impact::{apply_shared_actor_particle_hit, SharedActorImpactOutcome},
        static_damage::StaticDamageScheduler,
        world_fx::{
            ParticleTerrainEvent, ParticleTerrainMutation, ParticleTerrainResponse,
            ParticleTraversalTiming, TerrainCollisionContext,
        },
    };

    #[derive(Clone, Copy)]
    enum WorldHost {
        Playing,
        Intro2,
    }

    for host in [WorldHost::Playing, WorldHost::Intro2] {
        let (mut session, mut manager, _) =
            native_intro2_fixture().expect("native blood regression requires the retail corpus");
        let material = TerrainCollisionContext::from_current_level_cache(&session.cache)
            .unwrap()
            .ground_response_selectors
            .iter()
            .position(|&selector| selector != 6)
            .expect("an ordinary infectable ground material") as u8;
        // Dry, flat ground isolates E180's infection writer from water response
        // and static objects. Blood is born exactly on this surface, so the first
        // zero-delta traversal reaches its contact tail without a timing guess.
        let terrain = session.cache.level_terrain_mut().unwrap();
        terrain.header[0] = -4096 << 8;
        for cell in &mut terrain.cells {
            cell.height = 0;
            cell.attribute = 0;
            cell.terrain_type = material;
        }
        session.cache.take_level_terrain_presentation_dirty();

        let id = prepare_target(&mut manager, 4);
        manager.entity_mut(id).unwrap().collision.health_raw = RetailRuntimeValue::Known(10_000);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.adopt_intro2_type17(&manager);
        let mut fx = full_rate_fx();
        let mut notifications = crate::gameplay_notifications::GameplayNotifications::new();
        let mut static_damage = StaticDamageScheduler::new();
        let mut corpse_task = None;

        for (stage, amount, position) in [
            ("living", 2500, [1024, 0, 2048]),
            ("lethal", 100_000, [2048, 0, 2048]),
            ("corpse", 2500, [3072, 0, 2048]),
        ] {
            manager.entity_mut(id).unwrap().set_position_raw(position);
            let outcome = apply_shared_actor_particle_hit(
                crate::shared_actor_impact::SharedActorImpactFrame {
                    entities: &mut manager,
                    resources: &session.cache,
                    world_fx: &mut fx,
                    scheduler: &mut scheduler,
                    notifications: &mut notifications,
                    retail_tick: 251,
                },
                impact(id, false, amount),
            );
            let Some(SharedActorImpactOutcome::Spider(Intro2Type17ImpactOutcome::Applied(damage))) =
                outcome
            else {
                panic!("{stage}: {outcome:?}");
            };
            assert_ne!(damage.filtered_damage_raw, 0, "{stage}");
            assert_eq!(damage.death_publication.is_some(), stage == "lethal");
            let entity = manager.entity_mut(id).unwrap();
            if stage == "living" {
                assert!(
                    matches!(entity.collision.health_raw, RetailRuntimeValue::Known(health) if health > 0)
                );
            } else {
                assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
                let task = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
                if stage == "lethal" {
                    corpse_task = Some(task);
                } else {
                    assert_eq!(Some(task), corpse_task, "re-hit preserves the death task");
                }
            }
            let extent = session
                .cache
                .global_model(entity.model_index.unwrap())
                .unwrap()
                .radius;
            let emission = [
                position[0],
                position[1],
                position[2].wrapping_sub(extent as i16),
            ];
            let cell = [
                (emission[0] as u16 >> 8) as u8,
                (emission[2] as u16 >> 8) as u8,
            ];
            assert_eq!(
                fx.particle_count(),
                2,
                "{stage} keeps the full-rate class5 suffix"
            );
            assert_eq!(
                session
                    .cache
                    .level_terrain()
                    .unwrap()
                    .cell(cell[0] as usize, cell[1] as usize)
                    .unwrap()
                    .terrain_type,
                material
            );
            assert!(!session.cache.take_level_terrain_presentation_dirty());
            let mut unchanged_rng = fx.fork_for_main_base_abort_transaction();
            let update = match host {
                WorldHost::Playing => fx.update_with_traversal_host(
                    ParticleTraversalTiming {
                        elapsed_micros: 0,
                        retail_tick: 251,
                    },
                    &mut PlayingParticleHost {
                        cache: &mut session.cache,
                        static_damage: &mut static_damage,
                        owner_motions: vec![],
                        collision_models: vec![],
                        attachment_owners: vec![],
                        on_actor_event: |_: &mut ResourceCache,
                                           _: &mut StaticDamageScheduler,
                                           _: &mut WorldFx,
                                           _: crate::playing_particle_host::PlayingParticleActorEvent|
                         -> crate::playing_particle_host::PlayingParticleActorResponse {
                            panic!("the isolated blood fixture has no entity collision candidates")
                        },
                        on_terrain_event: |_: &mut StaticDamageScheduler,
                                           _: &mut WorldFx,
                                           _: ParticleTerrainEvent|
                         -> ParticleTerrainResponse {
                            panic!("class5 infection must use its own terrain writer")
                        },
                    },
                ),
                WorldHost::Intro2 => {
                    let report = crate::intro2_effects::update_intro2_effects(
                        crate::intro2_effects::Intro2EffectsFrame {
                            cache: &mut session.cache,
                            entities: &mut manager,
                            world_fx: &mut fx,
                            static_damage: &mut static_damage,
                            scheduler: &mut scheduler,
                            notifications: &mut notifications,
                            elapsed_micros: 0,
                            retail_tick: 251,
                        },
                    );
                    assert!(report.entity_deliveries.is_empty());
                    assert!(report.static_deliveries.is_empty());
                    report.particles
                }
            };
            assert_eq!(
                update.terrain_type_mutations,
                [ParticleTerrainMutation::Infection {
                    cell,
                    infected: true
                }],
                "{stage}: both carriers contact the same clean cell"
            );
            assert_eq!(
                fx.particle_count(),
                0,
                "{stage}: contact retires both carriers"
            );
            assert_eq!(
                session
                    .cache
                    .level_terrain()
                    .unwrap()
                    .cell(cell[0] as usize, cell[1] as usize)
                    .unwrap()
                    .terrain_type,
                material | 0x10,
                "{stage}: the production host commits the live grid"
            );
            assert!(
                session.cache.take_level_terrain_presentation_dirty(),
                "{stage}: infection invalidates terrain presentation"
            );
            assert_eq!(
                fx.fork_for_main_base_abort_transaction()
                    .next_shared_retail_random_u16(),
                unchanged_rng.next_shared_retail_random_u16(),
                "blood surface contact consumes no RNG"
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type17_late_checked_block_preserves_reselected_scheduler_owner() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = prepare_target(&mut manager, 30);
    let entity = manager.entity_mut(id).unwrap();
    let primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
    entity.collision.health_raw = RetailRuntimeValue::Unresolved;
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_intro2_type17(&manager);
    let result = apply_intro2_type17_particle_hit(
        crate::shared_actor_impact::SharedActorImpactFrame {
            entities: &mut manager,
            resources: &session.cache,
            world_fx: &mut WorldFx::new(),
            scheduler: &mut scheduler,
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            retail_tick: 251,
        },
        impact(id, false, 2500),
    );
    assert!(
        matches!(
            result,
            Intro2Type17ImpactOutcome::Blocked {
                reason: Intro2Type17ImpactBlock::Damage(LiveActorDamageError {
                    phase: LiveActorDamagePhase::Health,
                    ..
                }),
                committed_prefix: true,
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
        primary
    );
    assert_eq!(scheduler.adopt_intro2_type17(&manager), 0);
}

#[v2k_test_support::retail_test]
fn native_type17_pending_actor_prefix_rejects_both_hits_without_mutation() {
    for spawn in [4, 30] {
        for infected in [false, true] {
            let Some((session, mut manager, _)) = native_intro2_fixture() else {
                return;
            };
            let id = prepare_target(&mut manager, spawn);
            let entity = manager.entity_mut(id).unwrap();
            let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
                panic!("native birth must publish its current style")
            };
            assert_eq!(
                callback_policy(context.active_style(), HitEntry::Primary),
                Some(ImpactCallbackPolicy::ReselectBehavior),
                "the pending gate must precede an otherwise valid C690 callback"
            );
            let collision = entity.collision.clone();
            let body = (
                entity.position_raw(),
                entity.velocity_raw(),
                entity.rotation_heading_pitch_roll_raw(),
                entity.mass_raw,
            );
            let tasks = ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
                (
                    entity.actor_tasks.task_in_slot(slot),
                    entity.actor_task_state(slot).copied(),
                )
            });
            let a = entity.sub_a_propulsion_runtime;
            let h = entity.sub_h_external_frame_runtime.clone();
            let axis = entity.actor_common_axis_descriptor;
            let d = (entity.type17_sub_d_frame_owner, entity.type17_sub_d_runtime);
            let identity = entity.intro2_type17_runtime;
            let owner = Intro2Type17Owner::adopt_blocked_prefix(&manager, id).unwrap();
            let mut scheduler = SpecializedActorTaskScheduler::new();
            assert_eq!(scheduler.adopt_intro2_type17(&manager), 2);
            scheduler.register_intro2_type17(owner);
            assert!(scheduler.intro2_type17_has_pending_prefix(id));
            let mut fx = WorldFx::new();
            let result = apply_intro2_type17_particle_hit(
                crate::shared_actor_impact::SharedActorImpactFrame {
                    entities: &mut manager,
                    resources: &session.cache,
                    world_fx: &mut fx,
                    scheduler: &mut scheduler,
                    notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                    retail_tick: 251,
                },
                impact(id, infected, 2500),
            );
            assert_eq!(
                result,
                Intro2Type17ImpactOutcome::Blocked {
                    reason: Intro2Type17ImpactBlock::Runtime("pending actor prefix"),
                    committed_prefix: false,
                }
            );
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(entity.collision, collision);
            assert_eq!(
                entity.current_behavior_context,
                RetailRuntimeValue::Known(Some(context))
            );
            assert_eq!(
                (
                    entity.position_raw(),
                    entity.velocity_raw(),
                    entity.rotation_heading_pitch_roll_raw(),
                    entity.mass_raw,
                ),
                body
            );
            assert_eq!(
                ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
                    (
                        entity.actor_tasks.task_in_slot(slot),
                        entity.actor_task_state(slot).copied(),
                    )
                }),
                tasks
            );
            assert_eq!(entity.sub_a_propulsion_runtime, a);
            assert_eq!(entity.sub_h_external_frame_runtime, h);
            assert_eq!(entity.actor_common_axis_descriptor, axis);
            assert_eq!(
                (entity.type17_sub_d_frame_owner, entity.type17_sub_d_runtime),
                d
            );
            assert_eq!(entity.intro2_type17_runtime, identity);
            assert!(scheduler.intro2_type17_has_pending_prefix(id));
            assert_eq!(
                scheduler.family_for(id),
                Some(SpecializedActorTaskFamily::Intro2Type17)
            );
            assert_eq!(scheduler.adopt_intro2_type17(&manager), 0);
            assert!(scheduler.intro2_type17_has_pending_prefix(id));
            assert_eq!(fx.next_shared_retail_random_u16(), 38);
            assert_eq!(fx.pending_event_count(), 0);
            assert!(fx.take_positional_sounds().is_empty());
            assert_eq!(fx.particle_count(), 0);
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type17_wrong_allocation_cannot_commit_either_hit_prefix() {
    for infected in [false, true] {
        let Some((session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = prepare_target(&mut manager, 4);
        let entity = manager.entity_mut(id).unwrap();
        entity.intro2_type17_runtime = None;
        let flags = entity.collision.state_flags_at_0x08;
        let mut fx = WorldFx::new();
        let result = apply_intro2_type17_particle_hit(
            crate::shared_actor_impact::SharedActorImpactFrame {
                entities: &mut manager,
                resources: &session.cache,
                world_fx: &mut fx,
                scheduler: &mut SpecializedActorTaskScheduler::new(),
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                retail_tick: 251,
            },
            impact(id, infected, 2500),
        );
        assert!(
            matches!(
                result,
                Intro2Type17ImpactOutcome::Blocked {
                    reason: Intro2Type17ImpactBlock::Runtime("native allocation"),
                    committed_prefix: false
                }
            ),
            "{result:?}"
        );
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(
            entity.collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(17)
        );
        assert_eq!(entity.collision.state_flags_at_0x08, flags);
        assert_eq!(fx.next_shared_retail_random_u16(), 38);
        assert_eq!(fx.pending_event_count(), 0);
    }
}
