use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    entity::EntityManager,
    gameplay_notifications::GameplayNotifications,
    intro2_contacts::Intro2ContactFrame,
    native_actor_descriptor_contact::{
        resolve_native_actor_descriptor_contact, NativeDescriptorContactError,
        NativeDescriptorContactOutcome,
    },
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
    world_fx::WorldFx,
};

#[v2k_test_support::retail_test]
fn native_defecate_wander_pair_uses_shared_descriptor_without_advancing_task() {
    let Some((mut session, metadata)) = native::tests::fixture() else {
        return;
    };
    let mut entities = native::tests::generic(&session, &metadata);
    let id = 6;
    let opposite = 1;
    let mut words = [0x1700, 0xffff].into_iter();
    let publication = publish_intro2_type16(
        entities.entity_mut(id).unwrap(),
        &metadata[16],
        &[],
        session.cache.terrain().unwrap(),
        &mut || words.next().unwrap(),
    )
    .unwrap();
    assert_eq!(publication.selection.program.class_id, 4);
    let entity = entities.entity_mut(id).unwrap();
    let position = entity.position_raw();
    let basis = entity.physical_body_basis_q31();
    let heading = entity.rotation_heading_pitch_roll_raw()[0];
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    let Some(ActorTaskRuntime::DefecateVirusWander(task)) =
        entity.actor_tasks.task_state_mut(task_id)
    else {
        panic!()
    };
    task.before_callback(123_000);
    let before = *task;
    entities
        .entity_mut(opposite)
        .unwrap()
        .set_position_raw(position);
    let mut fx = WorldFx::new();
    let mut scheduler = SpecializedActorTaskScheduler::default();
    let mut notifications = GameplayNotifications::new();
    let mut static_damage = StaticDamageScheduler::new();
    let invoke = |entities: &mut EntityManager,
                  scheduler: &mut SpecializedActorTaskScheduler,
                  fx: &mut WorldFx,
                  resources: &mut crate::resource_cache::ResourceCache,
                  notifications: &mut GameplayNotifications,
                  static_damage: &mut StaticDamageScheduler,
                  slot| {
        resolve_native_actor_descriptor_contact(
            &mut Intro2ContactFrame {
                entities,
                resources,
                world_fx: fx,
                actor_tasks: scheduler,
                notifications,
                static_damage,
                retail_tick: 1340,
            },
            id,
            opposite,
            slot,
        )
    };
    // The same native body/task graph without its completed owner cannot mutate.
    let rejected = invoke(
        &mut entities,
        &mut scheduler,
        &mut fx,
        &mut session.cache,
        &mut notifications,
        &mut static_damage,
        ActorTaskSlot::Primary,
    )
    .unwrap_err();
    assert_eq!(
        rejected.reason,
        NativeDescriptorContactError::Runtime("completed descriptor owner")
    );
    assert!(!rejected.committed_prefix);
    assert_eq!(scheduler.adopt_intro2_type16(&entities), 1);
    let applied = invoke(
        &mut entities,
        &mut scheduler,
        &mut fx,
        &mut session.cache,
        &mut notifications,
        &mut static_damage,
        ActorTaskSlot::Primary,
    )
    .unwrap();
    assert_eq!(
        applied,
        NativeDescriptorContactOutcome::Applied { rng_draws: 4 }
    );
    let entity = entities.entity_mut(id).unwrap();
    let Some(ActorTaskRuntime::DefecateVirusWander(after)) = entity.actor_tasks.task_state(task_id)
    else {
        panic!()
    };
    assert_eq!(after.elapsed_ms(), before.elapsed_ms());
    assert_eq!(
        after.private_state().direction,
        -before.private_state().direction
    );
    assert_eq!(after.private_state().reversal_timer_ms, 1500);
    assert_eq!(entity.physical_body_basis_q31(), basis);
    assert_eq!(entity.position_raw(), position);
    let step = entity
        .intro2_type16_runtime
        .unwrap()
        .sub_d_runtime
        .last_yaw_step_raw;
    assert_ne!(step, 0);
    assert_eq!(
        entity.rotation_heading_pitch_roll_raw()[0],
        heading.wrapping_sub(step)
    );
    // The simultaneously installed02820 emitter has no descriptor callback.
    assert_eq!(
        invoke(
            &mut entities,
            &mut scheduler,
            &mut fx,
            &mut session.cache,
            &mut notifications,
            &mut static_damage,
            ActorTaskSlot::Tertiary
        )
        .unwrap(),
        NativeDescriptorContactOutcome::Null
    );
    assert!(scheduler.park_native_contact_prefix(&entities, id));
    let rejected = invoke(
        &mut entities,
        &mut scheduler,
        &mut fx,
        &mut session.cache,
        &mut notifications,
        &mut static_damage,
        ActorTaskSlot::Primary,
    )
    .unwrap_err();
    assert_eq!(
        rejected.reason,
        NativeDescriptorContactError::Runtime("completed descriptor owner")
    );
}
