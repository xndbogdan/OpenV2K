use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime, actor_task_owner::ActorTaskSlot,
    intro2_type17::capture::tests::fixture,
};

#[v2k_test_support::retail_test]
fn carrying_surface_expiry_releases_child_before_class12_world_tail() {
    let Some(mut f) = fixture() else {
        return;
    };
    let metadata = f.manager.type_runtime_metadata(17).unwrap().clone();
    let sea = f.session.cache.level_terrain().unwrap().sea_level_raw();
    let parent = f.manager.entity_mut(f.parent).unwrap();
    let mut position = parent.position_raw();
    position[1] = sea.saturating_sub(10_000);
    parent.set_motion_raw(position, [0; 3]);
    parent.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(1_990);
    // Controlled E370 entry after the actual carrying task. Overshoot avoids
    // the independent bubble gate while preserving the real 2,000-ms policy.
    parent.collision.state_flags_at_0x08.overwrite(
        crate::entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
        0,
    );
    let entry_state = parent.collision.state_flags_at_0x08.known_value_bits();
    let old_primary = parent.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
    let mut death = None;
    finish(
        &mut f.manager,
        f.parent,
        &metadata,
        &mut Intro2Type17Frame {
            capture_tasks: &mut f.scheduler,
            notifications: &mut f.notifications,
            resources: &f.session.cache,
            world_fx: &mut f.fx,
            elapsed_micros: 20_000,
            retail_tick: 301,
        },
        20_000,
        entry_state,
        &mut death,
    )
    .unwrap();
    assert!(death.is_some(), "E370 must retain its new class12 owner");
    let parent = f.manager.entity_mut(f.parent).unwrap();
    assert_eq!(
        parent.surface_lifetime_timer_ms_at_0x48,
        RetailRuntimeValue::Known(2_010)
    );
    assert_eq!(parent.collision.health_raw, RetailRuntimeValue::Known(0));
    assert!(parent.native_capture_relation.is_none());
    assert_ne!(
        parent.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        old_primary
    );
    assert!(matches!(
        parent.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::CommonDying(_))
    ));
    assert_eq!(f.manager.entity_mut(f.child).unwrap().attached_to, None);
    assert!(
        crate::intro2_type17::capture::CaptureTaskCustody::capture_child_mutation_ready(
            &mut f.scheduler,
            &f.manager,
            f.child
        )
    );
}
