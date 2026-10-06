//! Actual native insects versus authored live factories/huts in both list seats.
use super::*;
use crate::{
    entity::{AuthoredWorldConstruction, EntityConstructionResources, Intro2BirthSelection},
    entity_collision_state::EntityTypeRuntimeMetadata,
    gameplay_notifications::GameplayNotifications,
    session::GameSession,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
    world_fx::WorldFx,
};

fn scene(level: u32) -> (GameSession, EntityManager, WorldFx) {
    let data = v2k_test_support::retail_dir();
    assert!(data.join("PRELOAD.DAT").is_file(), "retail corpus required");
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(level, 1).unwrap();
    let metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(kind, _)| {
            EntityTypeRuntimeMetadata::from_section12(
                session.cache.global_entity_type(kind).unwrap(),
            )
        })
        .collect();
    let extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&extent),
    };
    let mut fx = WorldFx::new();
    let manager = if level == 50 {
        EntityManager::from_captured_intro2_frontend_with_type_metadata(
            session.cache.level_desc().unwrap(),
            &metadata,
            resources,
            Intro2BirthSelection::default(),
            &mut || 1,
        )
        .unwrap()
    } else {
        EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                level: session.cache.level_desc().unwrap(),
                logical_world_index: level as i32 - 12,
                type_metadata: &metadata,
                resources,
                player_arrival: None,
                retail_tick: 1300,
            },
            &mut fx,
        )
        .unwrap()
    };
    (session, manager, fx)
}

fn overlap(
    session: &GameSession,
    manager: &mut EntityManager,
    insect: u32,
    building: u32,
) -> (u32, u32, ActivePairContact) {
    let order: Vec<_> = manager.retail_live_order_ids().collect();
    let insect_first = order.iter().position(|id| *id == insect).unwrap()
        < order.iter().position(|id| *id == building).unwrap();
    let (subject, candidate) = if insect_first {
        (insect, building)
    } else {
        (building, insect)
    };
    let center = actor(manager, building).unwrap().position_raw();
    let subject_model = actor(manager, subject).unwrap().model_index.unwrap();
    let candidate_model = actor(manager, candidate).unwrap().model_index.unwrap();
    eprintln!("authored pair subject{subject}/model{subject_model}/{:?} candidate{candidate}/model{candidate_model}/{:?}",
        session.cache.global_model(subject_model).unwrap().name,
        session.cache.global_model(candidate_model).unwrap().name);
    let mut geometry_failures = std::collections::BTreeMap::<String, usize>::new();
    for dy in [64i16, 192, 320, 448, 576, 704] {
        for dx in (-1024..=1024).step_by(64) {
            for dz in (-1024..=1024).step_by(64) {
                manager.entity_mut(insect).unwrap().set_position_raw([
                    center[0].wrapping_add(dx as i16),
                    center[1].wrapping_add(dy),
                    center[2].wrapping_add(dz as i16),
                ]);
                let subject_entity = actor(manager, subject).unwrap();
                let candidate_entity = actor(manager, candidate).unwrap();
                let subject_entry = active_pair_body_from_entity(subject_entity, &session.cache);
                match classify_oriented_active_pair_contact(
                    crate::player_active_contact::OrientedActivePairContactRequest {
                        subject: subject_entity,
                        candidate: candidate_entity,
                        subject_entry: &subject_entry,
                        retail_tick: 1300,
                    },
                    &session.cache,
                ) {
                    // A depth1 dragon contact rounds native fixed-point body
                    // deltas to zero. A meaningful overlap must expose response.
                    Ok(Some(contact)) if contact.penetration_raw >= 32 => {
                        assert!(geometry_failures.is_empty(), "authored subject{subject}/model{subject_model} candidate{candidate}/model{candidate_model} had unsupported geometry before a valid hit: {geometry_failures:?}");
                        return (subject, candidate, contact);
                    }
                    Ok(_) => {}
                    Err(error) => {
                        let key = format!("{error:?}");
                        *geometry_failures.entry(key).or_default() += 1;
                    }
                }
            }
        }
    }
    panic!("no authored Type{} versus Type66 contact: subject{subject}/model{subject_model}, candidate{candidate}/model{candidate_model}; geometry failures={geometry_failures:?}", actor(manager, insect).unwrap().entity_type);
}

fn exercise(session: GameSession, manager: EntityManager, fx: WorldFx, insect: u32, building: u32) {
    exercise_with_type13_lethal(session, manager, fx, insect, building, false);
}

fn exercise_with_type13_lethal(
    mut session: GameSession,
    mut manager: EntityManager,
    mut fx: WorldFx,
    insect: u32,
    building: u32,
    lethal_type13: bool,
) {
    let ids: Vec<_> = manager.iter_all().map(|entity| entity.id).collect();
    for id in ids {
        let entity = manager.entity_mut(id).unwrap();
        entity.collision.state_flags_at_0x08.overwrite(
            0x8000,
            if id == insect || id == building {
                0x8000
            } else {
                0
            },
        );
        if id == insect || id == building {
            entity.collision.state_flags_at_0x08.overwrite(0x1000, 0);
            entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        }
    }
    let (subject, candidate, contact) = overlap(&session, &mut manager, insect, building);
    let actor_type = actor(&manager, insect).unwrap().entity_type;
    if lethal_type13 {
        assert_eq!(actor_type, 13);
        let entity = manager.entity_mut(insect).unwrap();
        entity.collision.health_raw = RetailRuntimeValue::Known(1);
        entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
    }
    let building_model = actor(&manager, building).unwrap().model_index.unwrap();
    let building_pose = actor(&manager, building).unwrap().position_raw();
    let insect_pose = actor(&manager, insect).unwrap().position_raw();
    let basis = actor(&manager, insect).unwrap().physical_body_basis_q31();
    let tasks_before = ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
        actor(&manager, insect)
            .unwrap()
            .actor_task_state(slot)
            .copied()
    });
    let sub_g_before = actor(&manager, insect).unwrap().sub_g_06070_runtime;
    let sub_h_before = actor(&manager, insect)
        .unwrap()
        .sub_h_external_frame_runtime
        .clone();
    let heading_before = actor(&manager, insect)
        .unwrap()
        .rotation_heading_pitch_roll_raw();
    let RetailRuntimeValue::Known(body_basis) = basis else {
        panic!("native insect basis");
    };
    let displacement =
        std::array::from_fn(|axis| building_pose[axis].wrapping_sub(insect_pose[axis]));
    let front_contact = crate::hover::dot_q31(body_basis.forward, displacement) >= 0;
    let speed = if lethal_type13 { 12_000 } else { 256 };
    let velocity = contact
        .normal_q12
        .map(|n| (i32::from(n) * if subject == insect { -speed } else { speed } >> 12) as i16);
    manager
        .entity_mut(insect)
        .unwrap()
        .set_velocity_raw(velocity);
    let expected = plan_active_pair_response_and_damage_cap(
        active_pair_body_from_entity(actor(&manager, subject).unwrap(), &session.cache),
        active_pair_body_from_entity(actor(&manager, candidate).unwrap(), &session.cache),
        contact,
    )
    .unwrap();
    if lethal_type13 {
        let RetailRuntimeValue::Known(profile) =
            actor(&manager, insect).unwrap().collision.damage_profile
        else {
            panic!("authored Type13 filter");
        };
        assert!(profile.filter(crate::damage::DamagePacket::collision(expected.capped_pair_damage_raw)) > 0,
            "injured Type13 must receive nonzero damage after native14D30 cap: plan{expected:?}, profile{profile:?}");
    }
    let mut tasks = SpecializedActorTaskScheduler::new();
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut resolve = |manager: &mut EntityManager, tasks: &mut SpecializedActorTaskScheduler| {
        resolve_native_captor_active_contacts(
            &mut Intro2ContactFrame {
                entities: manager,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut static_damage,
                notifications: &mut notifications,
                retail_tick: 1300,
                actor_tasks: tasks,
            },
            subject,
            CaptureFeedbackPolicy::Cinematic,
        )
    };
    let rejected = resolve(&mut manager, &mut tasks);
    assert!(
        matches!(
            rejected,
            NativeCaptorPairOutcome::Blocked {
                committed_prefix: false,
                ..
            }
        ),
        "{rejected:?}"
    );
    assert_eq!(actor(&manager, insect).unwrap().position_raw(), insect_pose);
    assert_eq!(
        actor(&manager, building).unwrap().position_raw(),
        building_pose
    );
    assert!(tasks.adopt_intro2_type66(&manager) > 0);
    assert!(
        match actor_type {
            16 => tasks.adopt_intro2_type16(&manager),
            26 => tasks.adopt_intro2_type26(&manager),
            94 => tasks.adopt_intro2_type94(&manager),
            13 => tasks.adopt_intro2_type13_search_attack(&manager),
            10 => tasks.adopt_intro2_type10(&manager),
            57 => tasks.adopt_intro2_type57(&manager),
            15 | 87 => tasks.adopt_intro2_flyers(&manager),
            _ => unreachable!(),
        } > 0
    );
    let result = resolve(&mut manager, &mut tasks);
    let NativeCaptorPairOutcome::Resolved { visits } = result else {
        panic!("Type{actor_type} vs model{building_model}: {result:?}");
    };
    let visit = visits
        .iter()
        .find(|visit| visit.candidate_id == candidate)
        .expect("real dynamic building pair must be dispatched");
    assert_eq!(visit.contact, contact);
    assert!(!visit.physical_suppressed);
    assert!(visit
        .stages
        .iter()
        .any(|stage| matches!(stage, NativeCaptorPairStage::Physical { .. })));
    let components: Vec<_> = visit
        .stages
        .iter()
        .filter_map(|stage| match stage {
            NativeCaptorPairStage::Component {
                owner,
                slot,
                result,
            } => Some((*owner, *slot, *result)),
            _ => None,
        })
        .collect();
    let source_order: Vec<_> = [subject, candidate]
        .into_iter()
        .flat_map(|owner| {
            ActorTaskSlot::IN_RETAIL_TICK_ORDER
                .into_iter()
                .map(move |slot| (owner, slot))
        })
        .collect();
    assert_eq!(
        components
            .iter()
            .map(|(owner, slot, _)| (*owner, *slot))
            .collect::<Vec<_>>(),
        source_order
    );
    for (owner, slot, result) in &components {
        if *owner == building {
            assert_eq!(
                *result,
                NativeCaptorPairComponentResult::Null,
                "25BD0 does not install task+18"
            );
            continue;
        }
        let before = tasks_before[ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .iter()
            .position(|value| value == slot)
            .unwrap()];
        let descriptor_hook = before
            .as_ref()
            .is_some_and(crate::native_actor_descriptor_contact::task_has_descriptor_contact);
        if !descriptor_hook {
            assert_eq!(*result, NativeCaptorPairComponentResult::Null);
        } else if !front_contact {
            assert_eq!(*result, NativeCaptorPairComponentResult::Behind);
        } else if matches!(actor_type, 16 | 26 | 94) {
            assert!(
                matches!(
                    result,
                    NativeCaptorPairComponentResult::Applied { rng_draws: 3 | 4 }
                ),
                "A/D consumes its conditional Sub-A draws, then X/Z retarget draws: {result:?}"
            );
        } else {
            assert_eq!(
                *result,
                NativeCaptorPairComponentResult::Applied { rng_draws: 2 },
                "G/D consumes only X/Z retarget draws"
            );
        }
        if let Some(before_private) = before
            .as_ref()
            .and_then(crate::native_actor_descriptor_contact::private_state)
        {
            if let Some(after_private) = actor(&manager, insect)
                .unwrap()
                .actor_task_state(*slot)
                .and_then(crate::native_actor_descriptor_contact::private_state)
            {
                assert_eq!(
                    after_private.direction,
                    if front_contact {
                        -before_private.direction
                    } else {
                        before_private.direction
                    }
                );
                assert_eq!(
                    after_private.reversal_timer_ms,
                    if front_contact {
                        1500
                    } else {
                        before_private.reversal_timer_ms
                    }
                );
            } else {
                assert!(
                    matches!(actor(&manager, insect).unwrap().collision.health_raw, RetailRuntimeValue::Known(health) if health <= 0),
                    "only native death may replace the contacted task"
                );
            }
        }
    }
    assert_eq!(
        actor(&manager, subject).unwrap().position_raw(),
        expected.subject.position_raw
    );
    assert_eq!(
        actor(&manager, candidate).unwrap().position_raw(),
        expected.candidate.position_raw
    );
    assert_ne!(actor(&manager, insect).unwrap().position_raw(), insect_pose);
    assert_eq!(
        actor(&manager, insect).unwrap().physical_body_basis_q31(),
        basis
    );
    if let (RetailRuntimeValue::Known(Some(before)), RetailRuntimeValue::Known(Some(after))) = (
        sub_g_before,
        actor(&manager, insect).unwrap().sub_g_06070_runtime,
    ) {
        assert_eq!(
            after.animation_outputs_raw(),
            before.animation_outputs_raw()
        );
        assert_eq!(after.rate_raw_at_0x34(), before.rate_raw_at_0x34());
        assert_eq!(
            after.randomized_target_raw_at_0x38(),
            before.randomized_target_raw_at_0x38()
        );
        assert_eq!(after.mode_at_0x3f(), before.mode_at_0x3f());
        assert_eq!(after.mode_at_0x40(), before.mode_at_0x40());
    }
    if actor_type == 94 {
        let entity = actor(&manager, insect).unwrap();
        let RetailRuntimeValue::Known(Some(h)) = &entity.sub_h_external_frame_runtime else {
            panic!("native water H");
        };
        assert_eq!(h.records().len(), 6);
        assert_eq!(
            h.surface_policy(),
            crate::sub_h_external_frame::SubHSurfacePolicy::TerrainAndWater
        );
        assert_eq!(
            entity.sub_h_external_frame_runtime, sub_h_before,
            "ordinary pair response does not advance or disable the living water feet"
        );
    }
    if !front_contact {
        assert_eq!(
            actor(&manager, insect)
                .unwrap()
                .rotation_heading_pitch_roll_raw(),
            heading_before,
            "02DA0's behind gate must precede Sub-D yaw writes"
        );
    }
    if lethal_type13 {
        assert!(crate::class49_death::finished_terminal_hit_authenticates(&manager, insect),
            "pair collision must finish authentic BAC0/BAF0/radial/A860 before returning; plan{expected:?}, visits{visits:?}, health{:?}, buffer{:?}",
            actor(&manager, insect).unwrap().collision.health_raw,
            actor(&manager, insect).unwrap().collision.pre_health_damage_buffer_raw);
        assert_eq!(manager.pending_actor_deferred_destroy_ids(), [insect]);
        assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| actor(&manager, insect)
                .unwrap()
                .actor_task_state(slot)
                .is_none()));
        assert!(
            !manager.iter_all().any(|entity| entity.entity_type == 60),
            "Class1 has no BD20 ring suffix"
        );
        assert!(
            fx.particle_count() + fx.pending_event_count() > 0,
            "BAF0 effects are immediate"
        );
    }
    eprintln!("Type{actor_type} versus live Type66 model{building_model}: source subject{subject:08x}, {insect_pose:?}->{:?}", actor(&manager, insect).unwrap().position_raw());
}

#[v2k_test_support::retail_test]
fn intro2_native_insects_contact_authored_hut_and_factory_bodies() {
    for spawn in [0, 1, 5, 10, 43, 44, 46, 55, 56] {
        for building_spawn in [51, 36] {
            let (session, manager, fx) = scene(50);
            let insect = manager
                .iter_all()
                .find(|entity| entity.authored_spawn_index == Some(spawn))
                .unwrap()
                .id;
            let building = manager
                .iter_all()
                .find(|entity| entity.authored_spawn_index == Some(building_spawn))
                .unwrap()
                .id;
            exercise(session, manager, fx, insect, building);
        }
    }
}

#[v2k_test_support::retail_test]
fn ordinary_stag_contacts_independently_authored_native_factories() {
    let mut checked = 0;
    for level in 13..=49 {
        let (session, manager, fx) = scene(level);
        let Some(insect) = manager
            .iter_all()
            .find(|entity| entity.entity_type == 26)
            .map(|entity| entity.id)
        else {
            continue;
        };
        let Some(building) = manager
            .iter_all()
            .find(|entity| entity.entity_type == 66 && entity.intro2_type66_runtime.is_some())
            .map(|entity| entity.id)
        else {
            continue;
        };
        eprintln!("ordinary authored factory world level{level}");
        exercise(session, manager, fx, insect, building);
        checked += 1;
        if checked == 2 {
            break;
        }
    }
    assert_eq!(checked, 2, "two ordinary native factory worlds required");
}

#[v2k_test_support::retail_test]
fn lethal_type13_factory_pair_finishes_native_class1() {
    let (session, manager, fx) = scene(50);
    let insect = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(0))
        .unwrap()
        .id;
    let building = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(51))
        .unwrap()
        .id;
    exercise_with_type13_lethal(session, manager, fx, insect, building, true);
}
