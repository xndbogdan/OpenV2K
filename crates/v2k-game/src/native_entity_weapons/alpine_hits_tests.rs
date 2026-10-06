//! Enclosing Class22 contact -> Class1 radial -> Class18 inline split.

use super::{
    contact::{resolve_entity_weapon_surface_contact, EntityWeaponSurfaceOutcome},
    EntityWeaponConstructionRequest, EntityWeaponKind,
};
use crate::{
    class49_terminal::{Class49TerminalFrame, Class49WorldContext},
    damage::DamagePacket,
    entity_collision_state::RetailRuntimeValue,
    gameplay_notifications::GameplayNotifications,
    native_type40::death::{finished_terminal_authenticates, Type40SplitStatus},
    player_hull::{HullDamageProfile, PlayerHull},
    radial_damage::{scale_radial_damage, RadialDamageTemplate},
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    split_and_explode::{SplitAndExplodeCompletion, SplitChildRequest},
    static_damage::StaticDamageScheduler,
};

#[v2k_test_support::retail_test]
fn real_rocket_terrain_terminal_visits_inline_type40_children_in_the_same_radial_pass() {
    for objective in [false, true] {
        let (mut session, mut manager, mut fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(19);
        let parent_id = manager.iter_all().find(|e| e.entity_type == 40).unwrap().id;
        let player_id = manager.player().unwrap().id;
        let origin = [4096, 0, 4096];
        // Control current test terrain, without replacing any canonical type,
        // model, native constructor or allocation receipt. Attribute zero also
        // keeps the static radial prefix independent of object-hit RNG.
        let terrain = session.cache.level_terrain_mut().unwrap();
        terrain.header[0] = 0;
        for cell in &mut terrain.cells {
            cell.height = 0;
            cell.attribute = 0;
            cell.terrain_type = 0;
        }
        let initial_ids = manager.retail_live_order_ids().collect::<Vec<_>>();
        for &id in &initial_ids {
            manager
                .entity_mut(id)
                .unwrap()
                .set_position_raw([16000, 12000, 16000]);
        }
        let parent = manager.entity_mut(parent_id).unwrap();
        parent.set_position_raw(origin);
        parent
            .collision
            .state_flags_at_0x08
            .overwrite(0x0100_0000, if objective { 0x0100_0000 } else { 0 });
        let heading = parent.rotation_heading_pitch_roll_raw()[0] as u16;
        assert_eq!(parent.collision.health_raw, RetailRuntimeValue::Known(8000));
        assert!(crate::native_type40::manager_allocation_authenticates(
            &manager, parent_id
        ));
        let weapon = manager
            .construct_entity_weapon(
                EntityWeaponConstructionRequest {
                    kind: EntityWeaponKind::Rocket,
                    source_actor_id: player_id,
                    position_raw: origin,
                    velocity_raw: [0; 3],
                    rotation_raw: [0; 3],
                },
                &session.cache,
                &mut fx,
                4793,
            )
            .unwrap();
        let before_ids = manager.retail_live_order_ids().collect::<Vec<_>>();
        assert!(before_ids.len() + 8 <= 80);
        let record = session.cache.global_entity_type(42).unwrap();
        let header = &record.raw_header;
        let word = |at: usize| i16::from_le_bytes(header[at..at + 2].try_into().unwrap());
        let dword = |at: usize| i32::from_le_bytes(header[at..at + 4].try_into().unwrap());
        let blast = RadialDamageTemplate {
            inner_radius_raw: word(0x50),
            outer_radius_raw: word(0x52),
            impulse_raw: dword(0x54),
            packet: DamagePacket {
                channels: [dword(0x58), dword(0x5c)],
                amounts_raw: [dword(0x60), dword(0x64)],
            },
            trailing_raw: [46, player_id as i32],
        };
        assert_eq!(blast.packet.channels, [2, 3]);
        assert!(
            manager
                .type_runtime_metadata(40)
                .unwrap()
                .damage_profile
                .unwrap()
                .filter(blast.packet)
                >= 8000
        );

        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler.adopt_type40(&manager) > 0);
        scheduler.register_native_weapon(&manager, weapon).unwrap();
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let mut hull = PlayerHull::new(HullDamageProfile::from_type_record(
            session.cache.global_entity_type(46).unwrap(),
        ));
        hull.readback_entity_damage_state(&manager.player().unwrap().collision);
        for _ in 0..8 {
            fx.advance_frame_pacing(20_000);
        }
        let first_sub_d_seed = fx.next_sub_d_allocation_seed();
        let mut expected_rng = fx.fork_for_main_base_abort_transaction();
        // Class1 BAF0's sound precedes C080's sound; direction-table scatter
        // takes no RNG. Each child's four constructor draws must separate the
        // next nine-word launch packet, rather than merely match an endpoint.
        expected_rng.next_shared_retail_random_u16();
        expected_rng.next_shared_retail_random_u16();
        let expected_births = std::array::from_fn::<_, 8, _>(|_| {
            let words =
                std::array::from_fn::<_, 9, _>(|_| expected_rng.next_shared_retail_random_u16());
            let birth = SplitChildRequest {
                requested_entity_handle_raw: 0,
                entity_type: 56,
                position_raw: std::array::from_fn(|axis| {
                    origin[axis].wrapping_add(((words[axis] >> 6) as i16) - 512)
                }),
                objective,
                velocity_raw: [
                    ((words[3] >> 5) as i16) - 1024,
                    (words[4] >> 6) as i16,
                    ((words[5] >> 5) as i16) - 1024,
                ],
                rotation_heading_pitch_roll_raw: [
                    heading.wrapping_add((words[6] >> 4).wrapping_sub(2048)),
                    (words[7] >> 4).wrapping_sub(2048),
                    (words[8] >> 4).wrapping_sub(2048),
                ],
            };
            for _ in 0..4 {
                expected_rng.next_shared_retail_random_u16();
            }
            birth
        });

        let outcome = resolve_entity_weapon_surface_contact(
            Class49TerminalFrame {
                entities: &mut manager,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut static_damage,
                notifications: &mut notifications,
                retail_tick: 4794,
                world: Class49WorldContext::Playing {
                    scheduler: &mut scheduler,
                    player_hull: &mut hull,
                    extra_lives: RetailRuntimeValue::Known(3),
                    active_terminal_calls: Vec::new(),
                },
            },
            weapon,
        )
        .expect("actual Class22 terrain callback completes its enclosing Playing radial pass");
        assert!(
            matches!(
                outcome,
                EntityWeaponSurfaceOutcome::Applied {
                    solid_contact: true,
                    collision_damage_raw: 0,
                    water_entry: false,
                    ..
                }
            ),
            "{outcome:?}"
        );
        assert!(crate::class49_death::finished_terminal_hit_authenticates(
            &manager,
            weapon.entity_id()
        ));
        assert!(finished_terminal_authenticates(&manager, parent_id));
        let scatter = fx
            .test_particles_in_virgin_birth_order()
            .into_iter()
            .filter(|particle| particle.source_class == 37)
            .collect::<Vec<_>>();
        assert_eq!(scatter.len(), 12);
        assert!(scatter[..10].iter().all(|particle| {
            particle.owner_id == Some(player_id) && particle.source_entity_type_at_birth == Some(46)
        }));
        assert!(scatter[10..].iter().all(|particle| {
            particle.owner_id == Some(parent_id) && particle.source_entity_type_at_birth == Some(40)
        }));
        let split = manager
            .iter_all()
            .find(|e| e.id == parent_id)
            .unwrap()
            .native_type40_runtime
            .unwrap()
            .split_terminal
            .unwrap();
        assert_eq!(
            split.status,
            Type40SplitStatus::Completed(SplitAndExplodeCompletion::ChildrenComplete)
        );
        assert_eq!(
            split.progress.native_entity_list_count_raw,
            Some(before_ids.len() as i32)
        );
        assert_eq!(
            (
                split.progress.attempted_children,
                split.progress.created_children,
                split.progress.launch_rng_words
            ),
            (8, 8, 72)
        );
        let children = manager
            .retail_live_order_ids()
            .filter(|id| !before_ids.contains(id))
            .collect::<Vec<_>>();
        assert_eq!(children.len(), 8);
        let child_damage = manager
            .type_runtime_metadata(56)
            .unwrap()
            .damage_profile
            .unwrap();
        for (ordinal, (&id, birth)) in children.iter().zip(expected_births).enumerate() {
            let child = manager.iter_all().find(|e| e.id == id).unwrap();
            let runtime = child.native_type56_runtime.unwrap();
            assert_eq!(child.entity_type, 56);
            assert!(child.authored_spawn_index.is_none());
            assert_eq!(
                runtime.birth(),
                birth,
                "constructor interleave at child {ordinal}"
            );
            assert_eq!(
                runtime.allocation(),
                manager.main_base_abort_actor_observation(id).unwrap().lease
            );
            assert!(crate::native_type56::manager_allocation_authenticates(
                &manager, id
            ));
            assert_eq!(
                runtime.sub_d_owner.classifier_cache().stagger_counter(),
                first_sub_d_seed.wrapping_add(ordinal as u8)
            );
            assert_eq!(
                child.collision.state_flags_at_0x08.masked(0x0100_0000),
                RetailRuntimeValue::Known(if objective { 0x0100_0000 } else { 0 })
            );
            let scaled = scale_radial_damage(blast, origin, runtime.anchor_raw(), true)
                .expect("genuine anchored newborn is inside its enclosing rocket blast");
            assert!(child_damage.filter(scaled.packet) >= 6000);
            // A constructor alone leaves 6000 health and living tasks. Reaching
            // this authenticated quiet terminal requires the outer walker to
            // reload its appended tail and find the inline scheduler owner.
            assert_eq!(child.collision.health_raw, RetailRuntimeValue::Known(0));
            assert!(crate::native_type56::death::finished_terminal_authenticates(&manager, id));
            assert!(scheduler.family_for(id).is_none());
        }
        assert_eq!(
            fx.next_sub_d_allocation_seed(),
            first_sub_d_seed.wrapping_add(8)
        );
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            expected_rng.next_shared_retail_random_u16()
        );
        // No test-only adoption is allowed to repair inline publication.
        assert_eq!(scheduler.adopt_type56(&manager), 0);
        assert_eq!(
            notifications.save_tail_seen_mask() & (1 << 4),
            if objective { 1 << 4 } else { 0 },
            "14E90 retains player source and reads the post-death objective bit"
        );
    }
}

#[v2k_test_support::retail_test]
fn real_grenade_active_pair_kills_type30_and_publishes_class49_ring_and_class12() {
    use crate::{
        actor_task_dispatcher::ActorTaskRuntime,
        actor_task_owner::ActorTaskSlot,
        intro2_contacts::Intro2ContactFrame,
        intro2_radial::Intro2RadialTaskCustody,
        native_actor_capture::pair::{
            resolve_native_captor_active_contacts_with_playing, CaptureFeedbackPolicy,
            NativeCaptorPairOutcome, NativeCaptorPairStage, PlayingPlayerContact,
        },
        native_type30::contact::{resolve_type30_static_contact, Type30ContactOutcome},
        player_active_contact::{
            active_pair_body_from_entity, classify_oriented_active_pair_contact,
            OrientedActivePairContactRequest,
        },
        specialized_actor_task_production::SpecializedActorTaskFamily,
        type60_exploding_ring::Type60ConstructionProvenance,
    };
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(19);
    let target_id = manager.iter_all().find(|e| e.entity_type == 30).unwrap().id;
    let player_id = manager.player().unwrap().id;
    let center = [4096, 6000, 4096];
    let terrain = session.cache.level_terrain_mut().unwrap();
    terrain.header[0] = 0;
    for cell in &mut terrain.cells {
        cell.height = 0;
        cell.attribute = 0;
        cell.terrain_type = 0;
    }
    let initial_ids = manager.retail_live_order_ids().collect::<Vec<_>>();
    for id in initial_ids {
        manager.entity_mut(id).unwrap().set_motion_raw(
            if id == target_id {
                center
            } else {
                [16000, 16000, 16000]
            },
            [0; 3],
        );
    }
    let native_before = manager
        .entity_mut(target_id)
        .unwrap()
        .native_type30_runtime
        .clone()
        .unwrap();
    let models_before = manager.entity_mut(target_id).unwrap().model_slots;
    let objective_before = manager
        .entity_mut(target_id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .masked(0x0100_0000);
    assert!(matches!(objective_before, RetailRuntimeValue::Known(_)));
    let grenade = manager
        .construct_entity_weapon(
            EntityWeaponConstructionRequest {
                kind: EntityWeaponKind::Grenade,
                source_actor_id: player_id,
                position_raw: center,
                velocity_raw: [0; 3],
                rotation_raw: [0; 3],
            },
            &session.cache,
            &mut fx,
            4793,
        )
        .unwrap();
    let before_ids = manager.retail_live_order_ids().collect::<Vec<_>>();
    assert!(
        before_ids.iter().position(|id| *id == target_id)
            < before_ids.iter().position(|id| *id == grenade.entity_id())
    );
    let mut contact = None;
    // Query only the genuine source model programs. These are controlled test
    // positions; no recorded shot position is used to force a contact.
    'search: for dy in [-128i16, 0, 128] {
        for dx in (-512..=512).step_by(64) {
            for dz in (-512..=512).step_by(64) {
                manager
                    .entity_mut(grenade.entity_id())
                    .unwrap()
                    .set_position_raw([
                        center[0].wrapping_add(dx as i16),
                        center[1].wrapping_add(dy),
                        center[2].wrapping_add(dz as i16),
                    ]);
                let target = manager.iter_all().find(|e| e.id == target_id).unwrap();
                let entry = active_pair_body_from_entity(target, &session.cache);
                let found = classify_oriented_active_pair_contact(
                    OrientedActivePairContactRequest {
                        subject: target,
                        candidate: manager
                            .iter_all()
                            .find(|e| e.id == grenade.entity_id())
                            .unwrap(),
                        subject_entry: &entry,
                        retail_tick: 4794,
                    },
                    &session.cache,
                )
                .expect("canonical Type30/grenade oriented model program");
                if let Some(found) = found.filter(|found| found.penetration_raw >= 32) {
                    contact = Some(found);
                    break 'search;
                }
            }
        }
    }
    let contact = contact.expect("genuine Type30 and grenade narrow solids overlap");
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert!(scheduler.adopt_type30(&manager) > 0);
    scheduler.register_native_weapon(&manager, grenade).unwrap();
    assert!(scheduler.prepare_native_actor_mutation(&manager, target_id));
    let mut hull = PlayerHull::new(HullDamageProfile::from_type_record(
        session.cache.global_entity_type(46).unwrap(),
    ));
    hull.readback_entity_damage_state(&manager.player().unwrap().collision);
    let mut notifications = GameplayNotifications::new();
    let mut static_damage = StaticDamageScheduler::new();
    for _ in 0..8 {
        fx.advance_frame_pacing(20_000);
    }
    // The authored Type30 precedes the fresh grenade in the intrusive list,
    // so its actual11AD0 visit owns this unordered pair. Keep the production
    // static-before-active order and provide the real Playing terminal host.
    let mut frame = Intro2ContactFrame {
        entities: &mut manager,
        resources: &mut session.cache,
        world_fx: &mut fx,
        static_damage: &mut static_damage,
        notifications: &mut notifications,
        retail_tick: 4794,
        actor_tasks: &mut scheduler,
    };
    let static_contact = resolve_type30_static_contact(&mut frame, target_id);
    assert!(
        matches!(
            static_contact,
            Type30ContactOutcome::Miss | Type30ContactOutcome::Ineligible
        ),
        "{static_contact:?}"
    );
    let result = resolve_native_captor_active_contacts_with_playing(
        &mut frame,
        target_id,
        CaptureFeedbackPolicy::Gameplay,
        Some(PlayingPlayerContact {
            hull: &mut hull,
            extra_lives: RetailRuntimeValue::Known(3),
        }),
    );
    let NativeCaptorPairOutcome::Resolved { visits } = result else {
        panic!("{result:?}")
    };
    let visit = visits
        .iter()
        .find(|visit| visit.candidate_id == grenade.entity_id())
        .expect("actual Type30 subject reaches the appended grenade");
    assert_eq!(visit.contact, contact);
    assert!(!visit.physical_suppressed);
    assert_eq!(
        visit
            .stages
            .iter()
            .filter(|stage| matches!(stage, NativeCaptorPairStage::Behavior { .. }))
            .count(),
        2
    );
    assert_eq!(
        visit
            .stages
            .iter()
            .filter(|stage| matches!(stage, NativeCaptorPairStage::Component { .. }))
            .count(),
        6
    );
    assert!(matches!(
        visit.stages.last(),
        Some(NativeCaptorPairStage::Physical { .. })
    ));
    assert!(crate::class49_death::finished_terminal_hit_authenticates(
        &manager,
        grenade.entity_id()
    ));
    assert!(manager
        .pending_actor_deferred_destroy_ids()
        .contains(&grenade.entity_id()));
    assert!(scheduler.family_for(grenade.entity_id()).is_none());
    let target = manager.iter_all().find(|e| e.id == target_id).unwrap();
    assert_eq!(target.collision.health_raw, RetailRuntimeValue::Known(0));
    assert!(crate::native_type30::manager_allocation_authenticates(
        &manager, target_id
    ));
    assert!(crate::intro2_common_dying::Intro2CommonDyingOwner::adopt(&manager, target_id).is_ok());
    assert_eq!(
        scheduler.family_for(target_id),
        Some(SpecializedActorTaskFamily::Intro2CommonDying)
    );
    let native_after = target.native_type30_runtime.as_ref().unwrap();
    assert_eq!(native_after.allocation(), native_before.allocation());
    assert_eq!(
        native_after.allocation(),
        manager
            .main_base_abort_actor_observation(target_id)
            .unwrap()
            .lease
    );
    assert_eq!(native_after.kl_components, native_before.kl_components);
    assert_eq!(native_after.sub_e_runtime, native_before.sub_e_runtime);
    assert_eq!(target.model_slots, models_before);
    let newborns = manager
        .iter_all()
        .filter(|e| !before_ids.contains(&e.id))
        .collect::<Vec<_>>();
    assert_eq!(newborns.len(), 1);
    let ring = newborns[0];
    assert_eq!(ring.entity_type, 60);
    assert_eq!(
        ring.type60_construction_provenance(),
        Some(Type60ConstructionProvenance::Class49ExplosionTail)
    );
    assert!(matches!(
        ring.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::ExplodingRing(_))
    ));
    assert_eq!(
        scheduler.family_for(ring.id),
        Some(SpecializedActorTaskFamily::Type60ExplodingRing)
    );
    let scatter = fx
        .test_particles_in_virgin_birth_order()
        .into_iter()
        .filter(|particle| matches!(particle.source_class, 94 | 95))
        .collect::<Vec<_>>();
    assert_eq!(scatter.len(), 16);
    assert!(scatter
        .iter()
        .all(|particle| particle.owner_id == Some(player_id)
            && particle.source_entity_type_at_birth == Some(46)));
    assert_eq!(
        notifications.save_tail_seen_mask() & (1 << 4),
        if objective_before == RetailRuntimeValue::Known(0) {
            0
        } else {
            1 << 4
        }
    );
}
