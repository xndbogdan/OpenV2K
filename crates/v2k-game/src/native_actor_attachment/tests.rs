use super::*;
use crate::{
    entity::Entity,
    entity_behavior::ActiveBehaviorStyle,
    entity_relation_release::relation_attach_state_word_after,
    intro2_type8::{self, NativeWorkerProfile},
    native_type123,
    native_type86::{self, NativePersonProfile},
};

enum Plan {
    Worker(intro2_type8::cargo::Type8AttachPlan),
    Person(native_type86::cargo::Type86AttachPlan),
    Type123(native_type123::cargo::Type123AttachPlan),
}

fn actor(manager: &EntityManager, id: u32) -> &Entity {
    manager.iter_all().find(|e| e.id == id).unwrap()
}

fn prepare(manager: &EntityManager, id: u32, parent: u32) -> Result<Plan, String> {
    let kind = actor(manager, id).entity_type;
    if NativeWorkerProfile::from_entity_type(kind).is_some() {
        intro2_type8::cargo::prepare_attach(manager, id, parent)
            .map(Plan::Worker)
            .map_err(|e| format!("{e:?}"))
    } else if NativePersonProfile::from_entity_type(kind).is_some() {
        native_type86::cargo::prepare_attach(manager, id, parent)
            .map(Plan::Person)
            .map_err(|e| format!("{e:?}"))
    } else {
        assert_eq!(kind, 123);
        native_type123::cargo::prepare_attach(manager, id, parent)
            .map(Plan::Type123)
            .map_err(|e| format!("{e:?}"))
    }
}

fn commit(
    manager: &mut EntityManager,
    id: u32,
    plan: Plan,
    fx: &mut WorldFx,
) -> NativeActorAttachOutcome {
    match plan {
        Plan::Worker(plan) => intro2_type8::cargo::commit_attach(manager, id, plan, fx),
        Plan::Person(plan) => native_type86::cargo::commit_attach(manager, id, plan, fx),
        Plan::Type123(plan) => native_type123::cargo::commit_attach(manager, id, plan, fx),
    }
}

fn standard_death(manager: &mut EntityManager, id: u32, fx: &mut WorldFx) {
    let kind = actor(manager, id).entity_type;
    if NativeWorkerProfile::from_entity_type(kind).is_some() {
        assert!(
            intro2_type8::impact::run_intro2_type8_standard_death(manager, id, fx)
                .unwrap()
                .publication
                .is_some()
        );
    } else if NativePersonProfile::from_entity_type(kind).is_some() {
        assert!(
            native_type86::impact::run_native_type86_standard_death(manager, id, fx)
                .unwrap()
                .publication
                .is_some()
        );
    } else {
        assert!(
            native_type123::impact::run_native_type123_standard_death(manager, id, fx)
                .unwrap()
                .publication
                .is_some()
        );
    }
}

fn relation_prefix(manager: &mut EntityManager, id: u32, parent: u32) {
    let RetailRuntimeValue::Known(Some(rows)) =
        &mut manager.entity_mut(parent).unwrap().sub_j_attachment_runtime
    else {
        panic!();
    };
    rows.append(id).unwrap();
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.state_flags_at_0x08 =
        relation_attach_state_word_after(entity.collision.state_flags_at_0x08);
    entity.attached_to = Some(parent);
}

fn fixture(level: u32, kind: u32) -> (EntityManager, WorldFx, u32, u32) {
    let (_, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(level);
    manager.cleanup_pending_actor_deferred_destroys();
    fx.process_pending();
    fx.take_positional_sounds();
    let id = manager
        .iter_all()
        .find(|e| e.entity_type == kind)
        .unwrap()
        .id;
    let parent = manager.iter_all().find(|e| e.id != id && e.active
        && matches!(&e.sub_j_attachment_runtime, RetailRuntimeValue::Known(Some(rows)) if rows.is_empty() && rows.capacity() > 0)).unwrap().id;
    (manager, fx, id, parent)
}

#[v2k_test_support::retail_test]
fn dying_native_people_and_workers_attach_to_terminal_c470_without_rng_or_another_death() {
    for (level, kind) in [
        (17, 78),
        (24, 86),
        (16, 95),
        (49, 123),
        (25, 8),
        (19, 79),
        (24, 90),
        (40, 91),
        (42, 116),
    ] {
        let (mut manager, mut fx, id, parent) = fixture(level, kind);
        standard_death(&mut manager, id, &mut fx);
        fx.process_pending();
        fx.take_positional_sounds();
        let before = actor(&manager, id);
        let task = before
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let components = (
            before.model_slots,
            before.type8_sub_d_frame_owner,
            before.type8_sub_d_runtime,
            before.sub_a_propulsion_runtime,
            before.physical_body_basis_q31,
        );
        let RetailRuntimeValue::Known(Some(mut expected_animation)) =
            before.actor_animation_runtime
        else {
            panic!();
        };
        assert!(expected_animation.special_mode());
        let parent_actor = actor(&manager, parent);
        let sound = expected_animation.relation_attach_sound_id(parent_actor.capability_flags);
        let parent_position = parent_actor.position_raw();
        expected_animation.apply_relation_attach(parent);
        let mut oracle = fx.fork_for_main_base_abort_transaction();
        let plan = prepare(&manager, id, parent).unwrap();
        relation_prefix(&mut manager, id, parent);
        assert_eq!(
            commit(&mut manager, id, plan, &mut fx),
            NativeActorAttachOutcome::DeferredDestroy
        );
        let after = actor(&manager, id);
        assert_eq!(after.attached_to, Some(parent));
        assert!(after.active, "C470 defers physical removal");
        assert_eq!(after.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            after
                .collision
                .state_flags_at_0x08
                .masked(DEFERRED_DESTROY_PENDING_STATE_BIT),
            RetailRuntimeValue::Known(DEFERRED_DESTROY_PENDING_STATE_BIT)
        );
        assert_eq!(
            after.collision.state_flags_at_0x08.masked(0x8000),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(after.actor_tasks.wrapper_flags(task), None);
        for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
            assert_eq!(after.actor_tasks.task_in_slot(slot), None);
        }
        let RetailRuntimeValue::Known(Some(context)) = after.current_behavior_context else {
            panic!();
        };
        let ActiveBehaviorStyle::Audited(style) = context.active_style() else {
            panic!();
        };
        assert_eq!(style.frame_address, 0x004c7108);
        assert_eq!(
            after.actor_animation_runtime,
            RetailRuntimeValue::Known(Some(expected_animation))
        );
        assert_eq!(
            (
                after.model_slots,
                after.type8_sub_d_frame_owner,
                after.type8_sub_d_runtime,
                after.sub_a_propulsion_runtime,
                after.physical_body_basis_q31
            ),
            components
        );
        assert_eq!(manager.pending_actor_deferred_destroy_ids(), [id]);
        assert!(prepare(&manager, id, parent).is_err());
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
        fx.process_pending();
        let sounds = fx.take_positional_sounds();
        assert_eq!(
            sounds.iter().map(|s| s.sound_id).collect::<Vec<_>>(),
            sound.into_iter().map(usize::from).collect::<Vec<_>>()
        );
        if let Some(sound) = sounds.first() {
            assert_eq!(
                sound.position,
                [
                    (parent_position[0] as u16) as f32 / 256.0,
                    f32::from(parent_position[1]) / 256.0,
                    (parent_position[2] as u16) as f32 / 256.0
                ]
            );
        }
        assert!(matches!(&actor(&manager,parent).sub_j_attachment_runtime,
            RetailRuntimeValue::Known(Some(rows)) if rows.contains(id)));
        assert_eq!(manager.cleanup_pending_actor_deferred_destroys(), [id]);
        assert!(!manager.iter_all().any(|e| e.id == id));
    }
}

#[v2k_test_support::retail_test]
fn native_attach_420760_uses_parent_cues_for_living_and_dying_people() {
    for dying in [false, true] {
        let (mut manager, mut fx, id, _) = fixture(24, 86);
        let parent = manager
            .iter_all()
            .find(|e| e.entity_type == 122)
            .unwrap()
            .id;
        assert_eq!(actor(&manager, parent).capability_flags & 0x209, 8);
        assert_eq!(actor(&manager, id).capability_flags & 0x209, 0);
        if dying {
            standard_death(&mut manager, id, &mut fx);
        }
        fx.process_pending();
        fx.take_positional_sounds();
        let plan = prepare(&manager, id, parent).unwrap();
        relation_prefix(&mut manager, id, parent);
        let outcome = commit(&mut manager, id, plan, &mut fx);
        assert_eq!(
            outcome,
            if dying {
                NativeActorAttachOutcome::DeferredDestroy
            } else {
                NativeActorAttachOutcome::RetainedGraph
            }
        );
        fx.process_pending();
        assert_eq!(
            fx.take_positional_sounds()
                .iter()
                .map(|s| s.sound_id)
                .collect::<Vec<_>>(),
            [85]
        );
    }
}

#[v2k_test_support::retail_test]
fn terminal_attachment_rejects_foreign_remote_pending_and_changed_parent_evidence() {
    for (level, kind) in [(17, 78), (24, 86), (16, 95), (49, 123), (19, 79)] {
        let (mut manager, mut fx, id, parent) = fixture(level, kind);
        standard_death(&mut manager, id, &mut fx);
        fx.process_pending();
        fx.take_positional_sounds();
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(REMOTE_OWNED_STATE_BIT, REMOTE_OWNED_STATE_BIT);
        assert!(prepare(&manager, id, parent).is_err());
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(REMOTE_OWNED_STATE_BIT, 0);
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(
                DEFERRED_DESTROY_PENDING_STATE_BIT,
                DEFERRED_DESTROY_PENDING_STATE_BIT,
            );
        assert!(prepare(&manager, id, parent).is_err());
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(DEFERRED_DESTROY_PENDING_STATE_BIT, 0);
        let (mut foreign, _, foreign_id, foreign_parent) = fixture(level, kind);
        let source = actor(&manager, id);
        let destination = foreign.entity_mut(foreign_id).unwrap();
        destination.intro2_type8_runtime = source.intro2_type8_runtime;
        destination.native_type86_runtime = source.native_type86_runtime;
        destination.native_type123_runtime = source.native_type123_runtime;
        assert!(prepare(&foreign, foreign_id, foreign_parent).is_err());

        let plan = prepare(&manager, id, parent).unwrap();
        relation_prefix(&mut manager, id, parent);
        let mut position = actor(&manager, parent).position_raw();
        position[0] = position[0].wrapping_add(1);
        manager
            .entity_mut(parent)
            .unwrap()
            .set_position_raw(position);
        let before = actor(&manager, id).current_behavior_context;
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| commit(
                &mut manager,
                id,
                plan,
                &mut fx
            )))
            .is_err()
        );
        assert_eq!(actor(&manager, id).current_behavior_context, before);
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
        assert_eq!(fx.pending_event_count(), 0);
    }
}
