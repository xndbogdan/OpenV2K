use super::*;
use crate::{
    entity_behavior::behavior_program, entity_collision_state::RetailStateWord,
    job_nearby::JobCapacityState,
    ordinary_type9_initial_production::exact_wander_link_ready_fixture,
};

#[test]
fn attaching_help_cue_retires_its_sub_i_link_after_relation_attach() {
    for odd_parity in [false, true] {
        let (link, metadata) =
            crate::ordinary_type9_initial_production::exact_attract_attention_link_ready_fixture(
                odd_parity,
            );
        let (mut entity, _, _) = link.into_parts();
        let RetailRuntimeValue::Known(Some(animation)) = &mut entity.actor_animation_runtime else {
            panic!()
        };
        animation.advance(1_000, 0, false);
        animation.advance(81_000, 0, false);
        let before = *animation;
        assert!(before.forced_stop());
        let parent = Type9RelationOwner {
            id: 0x7000,
            capability_flags: 1,
            position_raw: position_raw(&entity),
        };
        let plan = plan_attach(&entity, &metadata, parent).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x1000, 0x1000);
        entity.attached_to = Some(parent.id);
        let actor = MainBaseAbortActorLease {
            entity_id: entity.id,
            allocation_identity: 1,
        };
        let carried = commit_attach(&mut entity, actor, plan).unwrap();
        assert!(carried.authenticates(&entity));
        assert_eq!(entity.attached_to, Some(parent.id));
        let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
            panic!()
        };
        assert_eq!(
            animation.linked_handle(),
            None,
            "ADB0 deletes cue after CD70 writes the parent link"
        );
        assert_eq!(animation.phase(), 0);
        assert!(!animation.forced_stop());
        assert_eq!(animation.countdown_millis(), before.countdown_millis());
        assert_eq!(animation.output(), before.output());
    }
}

fn attached_fixture() -> (Entity, EntityTypeRuntimeMetadata, Type9CarriedOwner) {
    let (link, metadata) = exact_wander_link_ready_fixture();
    let (mut entity, _birth_owner, _) = link.into_parts();
    let relation_owner = Type9RelationOwner {
        id: 0x7000,
        capability_flags: 1,
        position_raw: position_raw(&entity),
    };
    let plan = plan_attach(&entity, &metadata, relation_owner).unwrap();
    assert_eq!(
        plan.sound(),
        None,
        "man2 player-attachment sound is authored zero"
    );
    let actor = MainBaseAbortActorLease {
        entity_id: entity.id,
        allocation_identity: 1,
    };
    // This unit fixture represents the caller's already-authenticated 16700
    // prefix. The production high-level test obtains custody from the manager.
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x1000, 0x1000);
    entity.attached_to = Some(relation_owner.id);
    let carried = commit_attach(&mut entity, actor, plan).unwrap();
    (entity, metadata, carried)
}

fn nearby_inputs(
    entity: &Entity,
    position: [i16; 3],
) -> (
    Vec<OrdinaryType9RootEntityRef>,
    Vec<OrdinaryType9GoToJobCandidateEvidence>,
) {
    let mut candidates = vec![OrdinaryType9RootEntityRef {
        id: entity.id,
        entity_type: 9,
        position_raw: position_raw(entity),
        state_flags_raw: entity.collision.state_flags_at_0x08,
        capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        attached_entity_handle: RetailRuntimeValue::Known(None),
    }];
    for (id, entity_type, capability) in [(0x7100, 17, 8), (0x7200, 46, 1), (0x7300, 99, 0x20)] {
        candidates.push(OrdinaryType9RootEntityRef {
            id,
            entity_type,
            position_raw: position,
            state_flags_raw: RetailStateWord::exact(1),
            capability_flags: RetailRuntimeValue::Known(capability),
            attached_entity_handle: RetailRuntimeValue::Known(None),
        });
    }
    let evidence = candidates
        .iter()
        .map(|candidate| OrdinaryType9GoToJobCandidateEvidence {
            candidate_id: candidate.id,
            state_flags: candidate.state_flags_raw,
            capability_flags: candidate.capability_flags,
            capacity: RetailRuntimeValue::Known(if candidate.id == 0x7300 {
                Some(JobCapacityState {
                    current_jobs_raw: 0,
                    capacity_raw: 1,
                })
            } else {
                None
            }),
        })
        .collect();
    (candidates, evidence)
}

#[test]
fn release_preflight_rejects_unresolved_sub_a_without_relation_or_task_mutation() {
    let (mut entity, metadata, carried) = attached_fixture();
    let state = entity.collision.state_flags_at_0x08;
    let context = entity.current_behavior_context;
    let animation = entity.actor_animation_runtime;
    let tasks = visits(&entity);
    let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
        panic!()
    };
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
        crate::common_mover::SubAPropulsionRuntime::from_retail_words(
            RetailRuntimeValue::Unresolved,
            sub_a.direction_multiplier(),
            sub_a.drive_scale_percent(),
        ),
    ));
    let request = Type9CargoReleaseRequest {
        metadata: &metadata,
        candidates: &[],
        job_evidence: &[],
        position: Type9CargoReleasePosition::Materialiser([100, 200, 300]),
    };
    let position = position_raw(&entity);
    assert_eq!(
        prepare_release(&entity, &carried, &request).unwrap_err(),
        Type9CargoBlock::ComponentUnavailable
    );
    assert_eq!(position_raw(&entity), position);
    assert_eq!(entity.collision.state_flags_at_0x08, state);
    assert_eq!(entity.current_behavior_context, context);
    assert_eq!(entity.actor_animation_runtime, animation);
    assert_eq!(visits(&entity), tasks);
    assert!(carried.authenticates(&entity));
}

#[test]
fn attach_switches_each_reachable_style_to_the_audited_carrying_variant() {
    for (class, before_variant, after_variant, address) in [
        (6, 0, 1u8, 0x004c7a08),
        (54, 0, 1, 0x004c87d0),
        (10, 0, 2, 0x004c76a8),
        (10, 1, 2, 0x004c76a8),
        (45, 0, 2, 0x004c8740),
        (45, 1, 2, 0x004c8740),
    ] {
        let context = BehaviorContextRuntime::named_audited(
            behavior_program(class).unwrap(),
            u32::from(before_variant),
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            RetailRuntimeValue::Known(Some(123)),
            RetailRuntimeValue::Known(0x12345678),
            *audited_behavior_style(class, before_variant).unwrap(),
        )
        .unwrap();
        let carrying = carrying_context(context).unwrap();
        assert_eq!(
            carrying.style_table_index_raw_at_0x10(),
            u32::from(after_variant)
        );
        let ActiveBehaviorStyle::Audited(style) = carrying.active_style() else {
            panic!()
        };
        assert_eq!(style.frame_address, address);
        assert_eq!(style.release_callback_address, Some(0x0040ce90));
        assert_eq!(
            carrying.target_handle_at_0x08(),
            context.target_handle_at_0x08()
        );
        assert_eq!(
            carrying.auxiliary_word_at_0x0c(),
            context.auxiliary_word_at_0x0c()
        );
        assert!(
            carrying_context(carrying).is_none(),
            "carrying style has null attach callback"
        );
    }
}

#[test]
fn attach_owns_real_none_wrapper_and_retains_mover_components() {
    let (link, metadata) = exact_wander_link_ready_fixture();
    let (mut entity, _, _) = link.into_parts();
    let before = entity
        .ordinary_type9_selected_component_runtime
        .unwrap()
        .components();
    let sub_a = entity.sub_a_propulsion_runtime;
    let old_primary = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    let plan = plan_attach(
        &entity,
        &metadata,
        Type9RelationOwner {
            id: 123,
            capability_flags: 1,
            position_raw: [0; 3],
        },
    )
    .unwrap();
    let actor = MainBaseAbortActorLease {
        entity_id: entity.id,
        allocation_identity: 1,
    };
    let carried = commit_attach(&mut entity, actor, plan).unwrap();
    assert!(carried.authenticates(&entity));
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::None)
    ));
    assert_ne!(carried.visit.task_id, old_primary);
    assert!(entity.actor_tasks.wrapper_flags(old_primary).is_none());
    assert_eq!(
        entity
            .ordinary_type9_selected_component_runtime
            .unwrap()
            .components(),
        before
    );
    assert_eq!(entity.sub_a_propulsion_runtime, sub_a);
}

#[test]
fn stale_attach_plan_preserves_an_entered_task_and_controller() {
    let (link, metadata) = exact_wander_link_ready_fixture();
    let (mut entity, _, _) = link.into_parts();
    let plan = plan_attach(
        &entity,
        &metadata,
        Type9RelationOwner {
            id: 123,
            capability_flags: 1,
            position_raw: [0; 3],
        },
    )
    .unwrap();
    let actor = MainBaseAbortActorLease {
        entity_id: entity.id,
        allocation_identity: 1,
    };
    let id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    let Some(ActorTaskRuntime::OrdinaryType9Wander(task)) = entity.actor_tasks.task_state_mut(id)
    else {
        panic!()
    };
    task.before_callback(20_000);
    let before = entity.actor_animation_runtime;
    assert!(matches!(
        commit_attach(&mut entity, actor, plan),
        Err(Type9CargoBlock::StateChanged)
    ));
    assert_eq!(entity.actor_animation_runtime, before);
    assert_eq!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        Some(id)
    );
}

#[test]
fn release_reselects_all_four_classes_at_landing_without_replaying_birth() {
    for (selector, class, random_count) in [(0, 10, 3), (3063, 45, 4), (3982, 54, 2), (65230, 6, 2)]
    {
        let (mut entity, metadata, carried) = attached_fixture();
        let initial = entity.initial_behavior;
        let original_position = position_raw(&entity);
        let before_context = carried.context;
        let landing = [12000, 96, -9000];
        let (candidates, job_evidence) = nearby_inputs(&entity, landing);
        let request = Type9CargoReleaseRequest {
            metadata: &metadata,
            candidates: &candidates,
            job_evidence: &job_evidence,
            position: Type9CargoReleasePosition::Materialiser(landing),
        };
        let prepared = prepare_release(&entity, &carried, &request).unwrap();
        assert_eq!(
            position_raw(&entity),
            original_position,
            "preparation leaves live pose intact"
        );
        let mut words = 0;
        let publication = commit_release(
            &mut entity,
            carried,
            prepared,
            request,
            || {
                words += 1;
                if words == 1 {
                    selector
                } else {
                    0x4040
                }
            },
            |_| {},
            |_| {},
        )
        .unwrap();
        assert_eq!(
            words, random_count,
            "selector + exact class-{class} constructor RNG"
        );
        assert!(publication.authenticates(&entity));
        assert_eq!(position_raw(&entity), landing);
        assert_eq!(
            entity
                .ordinary_type9_selected_component_runtime
                .unwrap()
                .components()
                .immutable_anchor_raw_at_0x90(),
            RetailRuntimeValue::Known(landing)
        );
        assert_eq!(
            entity.initial_behavior, initial,
            "birth evidence is not replaced by release"
        );
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!()
        };
        assert!(
            matches!(context.descriptor(), BehaviorDescriptorIdentity::Named(program) if u32::from(program.class_id) == class)
        );
        assert_eq!(
            context.target_handle_at_0x08(),
            before_context.target_handle_at_0x08()
        );
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            before_context.auxiliary_word_at_0x0c()
        );
        assert_eq!(entity.attached_to, None);
        assert_eq!(
            entity.collision.default_state_flags_at_0xc8,
            RetailRuntimeValue::Known(0x2f)
        );
    }
}

#[test]
fn drop_start_preserves_old_anchor_and_prepared_drift_consumes_no_rng() {
    let (mut entity, metadata, carried) = attached_fixture();
    let old_position = position_raw(&entity);
    let anchor = carried.selected.components().immutable_anchor_raw_at_0x90();
    let request = Type9CargoReleaseRequest {
        metadata: &metadata,
        candidates: &[],
        job_evidence: &[],
        position: Type9CargoReleasePosition::Retained,
    };
    let prepared = prepare_release(&entity, &carried, &request).unwrap();
    entity.position[0] += 1.0;
    let before_state = entity.collision.state_flags_at_0x08;
    let mut words = 0;
    let failure = commit_release(
        &mut entity,
        carried,
        prepared,
        request,
        || {
            words += 1;
            0
        },
        |_| {},
        |_| {},
    )
    .unwrap_err();
    assert_eq!(words, 0);
    assert_eq!(entity.collision.state_flags_at_0x08, before_state);
    let carried = failure;
    assert!(carried.authenticates(&entity));
    entity.position = old_position.map(|word| f32::from(word) / 256.0);
    let request = Type9CargoReleaseRequest {
        metadata: &metadata,
        candidates: &[],
        job_evidence: &[],
        position: Type9CargoReleasePosition::Retained,
    };
    let prepared = prepare_release(&entity, &carried, &request).unwrap();
    let publication = commit_release(
        &mut entity,
        carried,
        prepared,
        request,
        || {
            words += 1;
            0xffff
        },
        |_| {},
        |_| {},
    )
    .unwrap();
    assert!(publication.authenticates(&entity));
    assert_eq!(position_raw(&entity), old_position);
    assert_eq!(
        entity
            .ordinary_type9_selected_component_runtime
            .unwrap()
            .components()
            .immutable_anchor_raw_at_0x90(),
        anchor
    );
    assert_eq!(words, 2);
}
