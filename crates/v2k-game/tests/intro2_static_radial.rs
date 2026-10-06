use v2k_game::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    damage::DamagePacket,
    entity::{
        DynamicRadialDeathPublication, DynamicRadialLivePhase, EntityConstructionResources,
        EntityManager, Intro2BirthSelection,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue, RetailStateWord},
    gameplay_notifications::GameplayNotifications,
    intro2_radial::{apply_intro2_radial_damage, Intro2RadialFrame, Intro2RadialReport},
    session::GameSession,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::{
        StaticDamageAction, StaticDamageOutcome, StaticDamageScheduler, KIND_0_RADIAL_TEMPLATE,
        KIND_27_RADIAL_TEMPLATE,
    },
    static_damage_live::resolve_current_static_damage_target,
    world_fx::WorldFx,
};

fn fixture() -> (GameSession, EntityManager) {
    let path = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&path).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    let metadata = session
        .cache
        .global_entity_model_table()
        .iter()
        .copied()
        .enumerate()
        .map(|(kind, model_slots)| {
            session
                .cache
                .global_entity_type(kind)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots,
                    ..Default::default()
                })
        })
        .collect::<Vec<_>>();
    let manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&|id| session.cache.global_model(id).map(|model| model.radius)),
        },
        Intro2BirthSelection::default(),
        &mut || 1,
    )
    .unwrap();
    (session, manager)
}

fn isolate_target(manager: &mut EntityManager, spawn: usize, position: [i16; 3]) -> u32 {
    let id = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(spawn))
        .unwrap()
        .id;
    let ids = manager
        .iter_all()
        .map(|entity| entity.id)
        .collect::<Vec<_>>();
    for other in ids {
        manager
            .entity_mut(other)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(0x8000, 0);
    }
    let entity = manager.entity_mut(id).unwrap();
    // Controlled eligibility and pose, preserving the real allocation receipt,
    // authored metadata, current task graph and native component custody.
    entity.collision.state_flags_at_0x08 = RetailStateWord::exact(0x68000);
    entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
    entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(321);
    entity.set_motion_raw(position, [0; 3]);
    id
}

#[v2k_test_support::retail_test]
fn actual_static_template_kills_native_type17_and26_births_without_particle_hit_callbacks() {
    for (spawn, kind) in [(4, 17), (30, 17), (10, 26), (25, 26)] {
        let (mut session, mut entities) = fixture();
        let origin = [100, 10_000, 300];
        let id = isolate_target(&mut entities, spawn, origin);
        let metadata = EntityTypeRuntimeMetadata::from_section12(
            session.cache.global_entity_type(kind).unwrap(),
        );
        assert_eq!(KIND_27_RADIAL_TEMPLATE.trailing_raw, [-1, 0]);
        assert!(
            metadata
                .damage_profile
                .unwrap()
                .filter(KIND_27_RADIAL_TEMPLATE.packet)
                > metadata.initial_health_raw.unwrap()
        );
        let before_angles = entities
            .entity_mut(id)
            .unwrap()
            .rotation_heading_pitch_roll_raw();
        let mut scheduler = SpecializedActorTaskScheduler::default();
        assert_eq!(
            if kind == 17 {
                scheduler.adopt_intro2_type17(&entities)
            } else {
                scheduler.adopt_intro2_type26(&entities)
            },
            2
        );
        let registered_before = scheduler.registered_len();
        let mut fx = WorldFx::new();
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let report = apply_intro2_radial_damage(
            &mut Intro2RadialFrame {
                active_terminal_calls: Vec::new(),
                entities: &mut entities,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut static_damage,
                notifications: &mut notifications,
                retail_tick: 999,
                actor_tasks: &mut scheduler,
            },
            origin,
            KIND_27_RADIAL_TEMPLATE,
        );
        let Intro2RadialReport::Applied {
            static_deliveries,
            dynamic,
        } = report
        else {
            panic!("{report:?}")
        };
        assert!(
            static_deliveries.is_empty(),
            "high source Y isolates actor delivery"
        );
        assert!(dynamic.completed(), "{dynamic:?}");
        assert_eq!(dynamic.completed_target_ids, [id]);
        assert!(matches!(dynamic.death_publications.as_slice(),
            [DynamicRadialDeathPublication::Intro2Class12(owner)] if owner.entity_id() == id));
        assert_eq!(
            scheduler.registered_len(),
            registered_before,
            "class12 replaces its living owner"
        );
        let entity = entities.entity_mut(id).unwrap();
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            entity.collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(321)
        );
        assert_eq!(
            entity.rotation_heading_pitch_roll_raw(),
            before_angles,
            "14AE0 does not call11030"
        );
        let Some(ActorTaskRuntime::CommonDying(task)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("native class12 publication")
        };
        assert_eq!(
            task.elapsed_ms(),
            0,
            "static phase follows this frame's actor cursor"
        );
        assert_eq!(
            fx.particle_count(),
            0,
            "radial does not run primary-hit capability8 class5"
        );
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            7719,
            "only class12 constructor word38"
        );
        fx.process_pending();
        let sounds = fx.take_positional_sounds();
        assert_eq!(
            sounds
                .iter()
                .map(|sound| sound.sound_id)
                .collect::<Vec<_>>(),
            [if kind == 17 { 94 } else { 75 }]
        );
    }
}

#[v2k_test_support::retail_test]
fn actual_kind0_radial_preserves_native_type9_style_tasks_stamp_and_angular_pose() {
    let (mut session, mut entities) = fixture();
    let origin = [100, 10_000, 300];
    let position = [484, 10_000, 300];
    let id = isolate_target(&mut entities, 2, position);
    let metadata =
        EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(9).unwrap());
    let scaled = v2k_game::radial_damage::scale_radial_damage(
        KIND_0_RADIAL_TEMPLATE,
        origin,
        position,
        true,
    )
    .unwrap();
    assert_eq!(
        scaled.packet.amounts_raw,
        [500, 500],
        "384 is halfway through256..512 falloff"
    );
    let filtered = metadata.damage_profile.unwrap().filter(scaled.packet);
    let health = metadata.initial_health_raw.unwrap();
    assert!(
        filtered > 0 && filtered < health,
        "actual source packet must be nonlethal"
    );
    assert_eq!(KIND_0_RADIAL_TEMPLATE.trailing_raw, [-1, 0]);
    let mut scheduler = SpecializedActorTaskScheduler::default();
    assert_eq!(scheduler.adopt_intro2_type9(&mut entities), 13);
    let entity = entities.entity_mut(id).unwrap();
    let context = entity.current_behavior_context;
    let tasks =
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).copied());
    let angles = entity.rotation_heading_pitch_roll_raw();
    let mut fx = WorldFx::new();
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let report = apply_intro2_radial_damage(
        &mut Intro2RadialFrame {
            active_terminal_calls: Vec::new(),
            entities: &mut entities,
            resources: &mut session.cache,
            world_fx: &mut fx,
            static_damage: &mut static_damage,
            notifications: &mut notifications,
            retail_tick: 999,
            actor_tasks: &mut scheduler,
        },
        origin,
        KIND_0_RADIAL_TEMPLATE,
    );
    let Intro2RadialReport::Applied {
        static_deliveries,
        dynamic,
    } = report
    else {
        panic!("{report:?}")
    };
    assert!(static_deliveries.is_empty());
    assert!(dynamic.completed(), "{dynamic:?}");
    assert_eq!(dynamic.completed_target_ids, [id]);
    assert!(dynamic.death_publications.is_empty());
    let entity = entities.entity_mut(id).unwrap();
    assert_eq!(
        entity.collision.health_raw,
        RetailRuntimeValue::Known(health - filtered)
    );
    assert_eq!(
        entity.velocity_raw(),
        [1000, 0, 0],
        "14AE0 applies its radial impulse only"
    );
    assert_eq!(entity.rotation_heading_pitch_roll_raw(), angles);
    assert_eq!(entity.current_behavior_context, context);
    assert_eq!(
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).copied()),
        tasks
    );
    assert_eq!(
        entity.collision.last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(321)
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        38,
        "no particle selector, impulse jitter or feedback RNG"
    );
    assert_eq!(fx.pending_event_count(), 0);
}

#[v2k_test_support::retail_test]
fn consecutive_static_radials_use_the_immediately_adopted_native_type9_death_owner() {
    let (mut session, mut entities) = fixture();
    let origin = [100, 10_000, 300];
    let id = isolate_target(&mut entities, 2, origin);
    let mut scheduler = SpecializedActorTaskScheduler::default();
    assert_eq!(scheduler.adopt_intro2_type9(&mut entities), 13);
    let mut fx = WorldFx::new();
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut death_tasks = None;
    let mut pending_after_death = 0;
    for visit in 0..2 {
        let report = apply_intro2_radial_damage(
            &mut Intro2RadialFrame {
                active_terminal_calls: Vec::new(),
                entities: &mut entities,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut static_damage,
                notifications: &mut notifications,
                retail_tick: 999,
                actor_tasks: &mut scheduler,
            },
            origin,
            KIND_0_RADIAL_TEMPLATE,
        );
        let Intro2RadialReport::Applied {
            static_deliveries,
            dynamic,
        } = report
        else {
            panic!("{report:?}")
        };
        assert!(static_deliveries.is_empty());
        assert!(dynamic.completed(), "radial visit{visit}: {dynamic:?}");
        if visit == 0 {
            assert_eq!(dynamic.completed_target_ids, [id]);
        } else {
            assert!(
                dynamic.completed_target_ids.is_empty(),
                "class14 cleared damage eligibility"
            );
        }
        assert_eq!(scheduler.registered_len(), 13);
        assert!(
            scheduler.prepare_native_type9_external_mutation(&entities, id),
            "the native death owner must already be in scheduler custody"
        );
        let entity = entities.entity_mut(id).unwrap();
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            entity.collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(321)
        );
        let tasks =
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).copied());
        if visit == 0 {
            assert!(matches!(
                dynamic.death_publications.as_slice(),
                [DynamicRadialDeathPublication::Intro2Type9Class14(_)]
            ));
            death_tasks = Some(tasks);
            pending_after_death = fx.pending_event_count();
        } else {
            assert!(dynamic.death_publications.is_empty());
            assert_eq!(Some(tasks), death_tasks);
            assert_eq!(fx.pending_event_count(), pending_after_death);
        }
    }
}

#[v2k_test_support::retail_test]
fn real_static_parent_stays_registered_through_dynamic_block_then_retires_once() {
    let (mut session, mut entities) = fixture();
    let parent = (0..=u8::MAX)
        .flat_map(|x| (0..=u8::MAX).map(move |z| [x, z]))
        .find_map(|cell| {
            resolve_current_static_damage_target(&session.cache, cell)
                .unwrap()
                .filter(|target| target.state.kind_index == 0 && target.state.terrain_type & 8 == 0)
        })
        .expect("Intro2 has authored unburned kind0 scenery");
    let mut static_damage = StaticDamageScheduler::new();
    assert!(matches!(
        static_damage.submit_hit(parent, DamagePacket::collision(30_000), &mut || {
            panic!("large source hit is deterministic")
        }),
        StaticDamageOutcome::Started { .. }
    ));
    static_damage.begin_advance(600_000);
    let batch = static_damage
        .next_node_batch(|cell| {
            resolve_current_static_damage_target(&session.cache, cell)
                .unwrap()
                .map(|target| target.state)
        })
        .expect("source kind0 program reaches radial opcode12");
    let (token, actions) = batch.into_parts();
    let mut radial = None;
    for action in actions {
        match action {
            StaticDamageAction::SetBurned { cell } => {
                session.cache.or_level_terrain_type_bits(cell, 0x08);
            }
            StaticDamageAction::Radial {
                source_cell,
                origin_raw,
                template,
            } => {
                assert_eq!(source_cell, parent.cell);
                radial = Some((origin_raw, template));
            }
            _ => {} // Other emitted actions do not own actor or node custody.
        }
    }
    let (origin, template) = radial.unwrap();
    assert_eq!(template, KIND_0_RADIAL_TEMPLATE);
    assert!(static_damage.contains_cell(parent.cell));
    let id = isolate_target(&mut entities, 2, origin);
    let mut scheduler = SpecializedActorTaskScheduler::default();
    assert_eq!(scheduler.adopt_intro2_type9(&mut entities), 13);
    let entity = entities.entity_mut(id).unwrap();
    let health_before = entity.collision.health_raw;
    entity.collision.pair_callbacks.damage_modifier_address =
        RetailRuntimeValue::Known(Some(0x1234));
    let mut fx = WorldFx::new();
    let mut notifications = GameplayNotifications::new();
    let report = apply_intro2_radial_damage(
        &mut Intro2RadialFrame {
            active_terminal_calls: Vec::new(),
            entities: &mut entities,
            resources: &mut session.cache,
            world_fx: &mut fx,
            static_damage: &mut static_damage,
            notifications: &mut notifications,
            retail_tick: 999,
            actor_tasks: &mut scheduler,
        },
        origin,
        template,
    );
    let Intro2RadialReport::Applied {
        static_deliveries,
        dynamic,
    } = report
    else {
        panic!("{report:?}")
    };
    assert!(
        !static_deliveries.is_empty(),
        "the actual scenery static scan completed first"
    );
    let blocked = dynamic
        .blocked
        .expect("controlled unresolved modifier stops only the dynamic suffix");
    assert_eq!(blocked.target_id, id);
    assert_eq!(blocked.phase, DynamicRadialLivePhase::Modifier);
    assert!(dynamic.death_publications.is_empty());
    assert_eq!(
        entities.entity_mut(id).unwrap().collision.health_raw,
        health_before
    );
    assert!(
        static_damage.contains_cell(parent.cell),
        "radial return does not retire its caller"
    );
    assert!(
        static_damage.complete_node_batch(token),
        "opcode0 retires the parent even after a blocked suffix"
    );
    assert!(
        !static_damage.complete_node_batch(token),
        "batch receipt cannot be completed twice"
    );
    assert!(!static_damage.contains_cell(parent.cell));
    assert!(static_damage
        .next_node_batch(|_| panic!("radial children wait until next advance"))
        .is_none());
}
