//! Canonical distinct person records, allocation custody and the shared graph.
use super::*;
use crate::{
    gameplay_notifications::GameplayNotifications,
    main_base_type9_abort::MainBaseType9ResultScreenState,
    native_actor_capture::{
        attach_capture_child, execute_capture_root, CaptureContext, CaptureRootCallback,
    },
    specialized_actor_task_production::SpecializedActorTaskScheduler,
};

fn actor(manager: &EntityManager, id: u32) -> &Entity {
    manager.iter_all().find(|entity| entity.id == id).unwrap()
}

#[v2k_test_support::retail_test]
fn native_person_profiles_validate_exact_records_and_reject_cross_profile_metadata() {
    let Some((session, _)) = tests::load(24, &mut WorldFx::new()) else {
        return;
    };
    let profiles = [
        NativePersonProfile::Type78,
        NativePersonProfile::Type86,
        NativePersonProfile::Type95,
    ];
    for profile in profiles {
        let metadata = EntityTypeRuntimeMetadata::from_section12(
            session
                .cache
                .global_entity_type(profile.entity_type() as usize)
                .unwrap(),
        );
        birth::validate_metadata(NativeFourChoiceProfile::Person(profile), &metadata).unwrap();
        for foreign in profiles.into_iter().filter(|other| *other != profile) {
            assert!(
                birth::validate_metadata(NativeFourChoiceProfile::Person(foreign), &metadata)
                    .is_err()
            );
        }
        let mut changed = metadata.clone();
        changed
            .initializer
            .as_mut()
            .unwrap()
            .behavior_choices
            .swap(0, 1);
        assert!(
            birth::validate_metadata(NativeFourChoiceProfile::Person(profile), &changed).is_err()
        );
        let mut changed = metadata.clone();
        let RetailRuntimeValue::Known(ref mut effects) = changed.common_world_effects else {
            panic!("canonical effects");
        };
        effects.surface_lifetime_ms += 1;
        assert!(
            birth::validate_metadata(NativeFourChoiceProfile::Person(profile), &changed).is_err()
        );
        let mut changed = metadata;
        let RetailRuntimeValue::Known(Some(ref mut animation)) = changed.actor_animation_descriptor
        else {
            panic!("canonical Sub-I");
        };
        animation.variable_binding = 0;
        assert!(
            birth::validate_metadata(NativeFourChoiceProfile::Person(profile), &changed).is_err()
        );
    }
}

#[v2k_test_support::retail_test]
fn native_person_all_authored_78_95_births_retain_profiles_and_owners_run() {
    let cohorts = [
        (16, 95, 10),
        (17, 78, 6),
        (19, 78, 6),
        (27, 95, 7),
        (31, 95, 10),
        (32, 78, 3),
        (35, 95, 8),
        (36, 95, 16),
        (38, 78, 6),
        (40, 95, 6),
    ];
    for (level, entity_type, count) in cohorts {
        let mut fx = WorldFx::new();
        let Some((session, mut manager)) = tests::load(level, &mut fx) else {
            return;
        };
        manager.cleanup_pending_actor_deferred_destroys();
        let profile = NativePersonProfile::from_entity_type(entity_type).unwrap();
        let ids: Vec<_> = manager
            .iter_all()
            .filter(|e| e.entity_type == entity_type)
            .map(|e| e.id)
            .collect();
        assert_eq!(ids.len(), count, "world{level}");
        let mut owners = Vec::new();
        for id in ids {
            let entity = actor(&manager, id);
            assert!(native_type86_manager_allocation_authenticates(&manager, id));
            assert_eq!(
                entity.native_type86_runtime.unwrap().profile,
                NativeFourChoiceProfile::Person(profile)
            );
            assert_eq!(entity.model_slots, [Some(profile.model_id()); 4]);
            assert_eq!(
                entity.actor_common_axis_descriptor,
                RetailRuntimeValue::Known(
                    manager
                        .type_runtime_metadata(entity_type)
                        .unwrap()
                        .initializer
                        .as_ref()
                        .unwrap()
                        .common_axis_descriptor
                )
            );
            assert!(entity.type8_sub_d_frame_owner.is_some());
            assert!(entity.type8_sub_d_runtime.is_some());
            owners.push(Type86Owner::take_birth(manager.entity_mut(id).unwrap()).unwrap());
        }
        for step in 1..=30 {
            for owner in &mut owners {
                let id = owner.entity_id();
                let tick = tick_type86(
                    &mut manager,
                    *owner,
                    Type86Frame {
                        resources: &session.cache,
                        world_fx: &mut fx,
                        elapsed_micros: 100_000,
                        global_elapsed_micros: 100_000,
                        retail_tick: step,
                    },
                );
                assert!(
                    matches!(
                        tick.outcome,
                        Type86Outcome::Waiting { .. }
                            | Type86Outcome::Advanced {
                                terminal: false,
                                ..
                            }
                    ),
                    "world{level} type{entity_type} entity{id} step{step}: {:?}",
                    tick.outcome
                );
                *owner = tick.retained_owner.unwrap();
            }
        }
    }
}

#[v2k_test_support::retail_test]
fn native_person_death_uses_own_cue_and_rejects_foreign_or_retyped_receipts_before_writes() {
    for (level, entity_type, death_cue) in [(17, 78, 74), (24, 86, 74), (16, 95, 35)] {
        let mut fx = WorldFx::new();
        let Some((_session, mut manager)) = tests::load(level, &mut fx) else {
            return;
        };
        let id = manager
            .iter_all()
            .find(|e| e.entity_type == entity_type)
            .unwrap()
            .id;
        let receipt = actor(&manager, id).native_type86_runtime;
        let Some((_session, mut foreign)) = tests::load(level, &mut WorldFx::new()) else {
            return;
        };
        foreign.entity_mut(id).unwrap().native_type86_runtime = receipt;
        // Constructors and standard death enqueue logical sound events; the
        // normal FX drain moves them into the presentation queue.
        fx.process_pending();
        fx.take_positional_sounds();
        let mut oracle = fx.fork_for_main_base_abort_transaction();
        assert!(impact::run_native_type86_standard_death(&mut foreign, id, &mut fx).is_err());
        assert_eq!(
            actor(&foreign, id).collision.health_raw,
            RetailRuntimeValue::Known(HEALTH)
        );
        assert_eq!(fx.pending_event_count(), 0);
        assert!(fx.take_positional_sounds().is_empty());
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );

        let other = if entity_type == 95 {
            NativePersonProfile::Type78
        } else {
            NativePersonProfile::Type95
        };
        let entity = foreign.entity_mut(id).unwrap();
        entity.entity_type = other.entity_type();
        entity.model_slots = [Some(other.model_id()); 4];
        entity.model_index = Some(other.model_id());
        assert!(!native_type86_allocation_authenticates(entity));

        let death = impact::run_native_type86_standard_death(&mut manager, id, &mut fx).unwrap();
        assert!(death.returned_nonzero);
        assert_eq!(death.publication.unwrap().kind, TaskKind::Exploding);
        assert_eq!(fx.pending_event_count(), 1);
        fx.process_pending();
        assert_eq!(
            fx.take_positional_sounds()
                .iter()
                .map(|sound| sound.sound_id)
                .collect::<Vec<_>>(),
            [death_cue]
        );
        assert_eq!(actor(&manager, id).native_type86_runtime, receipt);
        assert_eq!(
            Type86Owner::adopt_published(actor(&manager, id))
                .unwrap()
                .kind,
            TaskKind::Exploding
        );
    }
}

#[v2k_test_support::retail_test]
fn native_person_runaway_4c76a8_real_capture_attach_and_cleanup_release_keep_own_components() {
    let mut fx = WorldFx::new();
    let Some((_session, mut manager)) = tests::load(24, &mut fx) else {
        return;
    };
    manager.cleanup_pending_actor_deferred_destroys();
    let parent = manager
        .iter_all()
        .find(|e| e.entity_type == 122)
        .unwrap()
        .id;
    let child = manager.iter_all().find(|e| e.entity_type == 86).unwrap().id;
    let [x, y, z] = actor(&manager, parent).position_raw();
    manager
        .entity_mut(child)
        .unwrap()
        .set_position_raw([x.wrapping_add(32), y, z]);
    let mut tasks = SpecializedActorTaskScheduler::default();
    tasks.adopt_type122(&manager);
    tasks.adopt_native_type86(&mut manager);
    let mut chosen = None;
    for _ in 0..256 {
        let owner = birth::reselect(&mut manager, child, &mut fx).unwrap();
        if owner.kind == TaskKind::RunAwayAcquiring {
            chosen = Some(owner);
            break;
        }
    }
    tasks.register_native_type86(chosen.expect("actual nearby baddie must select RunAway"));
    let before = (
        actor(&manager, child).native_type86_runtime.unwrap(),
        actor(&manager, child).type8_sub_d_frame_owner,
        actor(&manager, child).type8_sub_d_runtime,
    );
    let mut notifications = GameplayNotifications::new();
    let mut context = CaptureContext {
        tasks: &mut tasks,
        world_fx: &mut fx,
        notifications: &mut notifications,
        retail_tick: 4793,
        result_screen: MainBaseType9ResultScreenState::NotShown,
        hive_dying: Default::default(),
    };
    attach_capture_child(&mut manager, parent, child, &mut context).unwrap();
    let carried = Type86Owner::adopt_published(actor(&manager, child)).unwrap();
    assert_eq!(carried.kind, TaskKind::Carried);
    let ActiveBehaviorStyle::Audited(style) = carried.context.active_style() else {
        panic!();
    };
    assert_eq!(style.frame_address, 0x004c76a8);
    assert_eq!(actor(&manager, child).attached_to, Some(parent));
    execute_capture_root(
        &mut manager,
        parent,
        CaptureRootCallback::Cleanup,
        &mut context,
    )
    .unwrap();
    let entity = actor(&manager, child);
    assert_eq!(entity.attached_to, None);
    assert_ne!(
        Type86Owner::adopt_published(entity).unwrap().kind,
        TaskKind::Carried
    );
    assert!(entity
        .native_type86_runtime
        .unwrap()
        .same_allocation(before.0));
    assert_eq!(entity.type8_sub_d_frame_owner, before.1);
    assert_eq!(entity.type8_sub_d_runtime, before.2);
    assert!(native_type86_manager_allocation_authenticates(
        &manager, child
    ));
}
