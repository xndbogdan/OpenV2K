use super::*;
use crate::{
    entity::EntityManager,
    entity_behavior::{behavior_program, BehaviorContextRuntime, BehaviorSelection},
    gameplay_notifications::GameplayNotifications,
    intro2_type47_live::world::native_intro2_fixture,
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskFamily, SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
    world_fx::WorldFx,
};

fn fixture() -> Option<(
    GameSession,
    EntityManager,
    SpecializedActorTaskScheduler,
    u32,
)> {
    let (session, mut manager, _) = native_intro2_fixture()?;
    let id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(40))
        .unwrap()
        .id;
    let metadata = manager.type_runtime_metadata(58).unwrap().clone();
    let entity = manager.entity_mut(id).unwrap();
    assert!(intro2_type58_allocation_authenticates(entity));
    let selection = BehaviorSelection {
        choice_index: 1,
        program: behavior_program(26).unwrap(),
    };
    assert!(super::super::native::publish_initial_style(
        entity,
        &metadata,
        selection,
        BehaviorContextRuntime::from_fresh_weighted_selection(selection).unwrap(),
        &mut || 0
    ));
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(u32::MAX, 0x0406_8000);
    entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
    entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
    entity.set_velocity_raw([0; 3]);
    let mut scheduler = SpecializedActorTaskScheduler::default();
    scheduler.register_intro2_type58(Intro2Type58Owner::adopt(&manager, id).unwrap());
    assert_eq!(
        session.cache.global_entity_type(58).unwrap().raw_header[0x8a..0x8c],
        [0, 0],
        "canonical Type58 has no static-contact cue"
    );
    Some((session, manager, scheduler, id))
}

fn frame<'a>(
    session: &'a mut GameSession,
    manager: &'a mut EntityManager,
    scheduler: &'a mut SpecializedActorTaskScheduler,
    fx: &'a mut WorldFx,
    damage: &'a mut StaticDamageScheduler,
    notifications: &'a mut GameplayNotifications,
) -> Intro2ContactFrame<'a> {
    Intro2ContactFrame {
        entities: manager,
        resources: &mut session.cache,
        world_fx: fx,
        static_damage: damage,
        notifications,
        retail_tick: 27,
        actor_tasks: scheduler,
    }
}

/// Callback tests begin with a selected contact from an actual terrain record;
/// their synthetic normal isolates 11760 arithmetic from collision geometry.
fn selected_contact(session: &mut GameSession, kind: u32, burned: bool) -> StaticModelContact {
    let table = session.cache.terrain_objects().unwrap();
    let attribute = table
        .records
        .iter()
        .enumerate()
        .find(|(index, entry)| *index != 0 && entry.kind_index == kind)
        .map(|(index, _)| index)
        .unwrap() as u8;
    assert_ne!(attribute, 0);
    let descriptor = table.records[usize::from(attribute)].clone();
    let cell = [0xbb, 0x0c];
    let terrain_type = if burned { 8 } else { 0 };
    let target = &mut session.cache.level_terrain_mut().unwrap().cells
        [usize::from(cell[0]) * 256 + usize::from(cell[1])];
    target.attribute = attribute;
    target.terrain_type = terrain_type;
    StaticModelContact {
        cell,
        attribute,
        terrain_type,
        model_id: descriptor.model_id_for(terrain_type),
        kind_index: kind,
        normal_q12: [4096, 0, 0],
        penetration_raw: 37,
        response_raw: 0,
    }
}

fn apply_selected(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    contact: StaticModelContact,
) -> Result<Intro2Type58ContactApplied, Intro2Type58ContactBlock> {
    assert!(frame
        .actor_tasks
        .intro2_type58_completed_owner(frame.entities, id));
    let mut committed = false;
    let result = apply_selected_contact(frame, id, contact, &mut committed);
    if result.is_err() {
        retain_failed_prefix(frame, id, committed);
    }
    result
}

#[v2k_test_support::retail_test]
fn native_type58_empty_static_delivery_still_reselects_without_hit_stamp_or_impulse() {
    let Some((mut session, mut manager, mut scheduler, id)) = fixture() else {
        return;
    };
    let contact = selected_contact(&mut session, 0, false);
    session.cache.level_terrain_mut().unwrap().cells[0xbb * 256 + 0x0c].attribute = 0;
    let entity = manager.entity_mut(id).unwrap();
    let old = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    let before = (
        entity.position_raw(),
        entity.velocity_raw(),
        entity.rotation_heading_pitch_roll_raw(),
        entity.intro2_type58_runtime,
    );
    let health = entity.collision.health_raw;
    let mut fx = WorldFx::new();
    let mut damage = StaticDamageScheduler::default();
    let mut notifications = GameplayNotifications::default();
    let applied = apply_selected(
        &mut frame(
            &mut session,
            &mut manager,
            &mut scheduler,
            &mut fx,
            &mut damage,
            &mut notifications,
        ),
        id,
        contact,
    )
    .unwrap();
    assert_eq!(applied.furniture_damage, None);
    assert_eq!(applied.collision_impact_raw, 0);
    assert_eq!(applied.actor_damage, None);
    let entity = manager.entity_mut(id).unwrap();
    assert_ne!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        Some(old)
    );
    assert!(entity.actor_tasks.wrapper_flags(old).is_none());
    assert_eq!(
        (
            entity.position_raw(),
            entity.velocity_raw(),
            entity.rotation_heading_pitch_roll_raw(),
            entity.intro2_type58_runtime
        ),
        before
    );
    assert_eq!(entity.collision.health_raw, health);
    assert_eq!(
        entity.collision.last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(17)
    );
    assert!(scheduler.intro2_type58_completed_owner(&manager, id));
    assert_eq!(damage.active_program_count(), 0);
    assert!(fx.take_positional_sounds().is_empty());
    assert_eq!(fx.particle_count(), 0);
    assert_ne!(
        fx.next_shared_retail_random_u16(),
        WorldFx::new().next_shared_retail_random_u16(),
        "C690 consumes its own selector/initializer words even after an empty27950 call"
    );
}

#[v2k_test_support::retail_test]
fn native_type58_burned_static_callback_keeps_original_plane_after_cell_change() {
    let Some((mut session, mut manager, mut scheduler, id)) = fixture() else {
        return;
    };
    let mut contact = selected_contact(&mut session, 10, true);
    contact.response_raw = -2048;
    manager
        .entity_mut(id)
        .unwrap()
        .set_velocity_raw([-256, 0, 0]);
    let before = manager.entity_mut(id).unwrap().position_raw();
    let mut fx = WorldFx::new();
    let mut damage = StaticDamageScheduler::default();
    let mut notifications = GameplayNotifications::default();
    let applied = apply_selected(
        &mut frame(
            &mut session,
            &mut manager,
            &mut scheduler,
            &mut fx,
            &mut damage,
            &mut notifications,
        ),
        id,
        contact,
    )
    .unwrap();
    assert!(matches!(
        applied.furniture_damage,
        Some(StaticDamageOutcome::BurnedKind10Transition { .. })
    ));
    let cell = session
        .cache
        .level_terrain()
        .unwrap()
        .cell(0xbb, 0x0c)
        .unwrap();
    assert_eq!(cell.attribute, contact.attribute.wrapping_add(1));
    assert_eq!(cell.terrain_type & 8, 0);
    assert_eq!(
        applied.contact, contact,
        "C690/27950 cell changes cannot replace the selected plane"
    );
    assert_eq!(applied.velocity_before_response_raw, [-256, 0, 0]);
    assert_eq!(applied.velocity_after_response_raw, [256, 0, 0]);
    assert_eq!(
        applied.position_after_response_raw,
        [before[0].wrapping_add(37), before[1], before[2]]
    );
    assert_eq!(applied.collision_impact_raw, 200);
    assert!(scheduler.intro2_type58_completed_owner(&manager, id));
}

#[v2k_test_support::retail_test]
fn native_type58_collision_damage_adopts_common12_without_primary_particle_suffix() {
    let Some((mut session, mut manager, mut scheduler, id)) = fixture() else {
        return;
    };
    let contact = selected_contact(&mut session, 0, false);
    manager
        .entity_mut(id)
        .unwrap()
        .set_velocity_raw([-8192, 0, 0]);
    let mut fx = WorldFx::new();
    let mut damage = StaticDamageScheduler::default();
    let mut notifications = GameplayNotifications::default();
    let applied = apply_selected(
        &mut frame(
            &mut session,
            &mut manager,
            &mut scheduler,
            &mut fx,
            &mut damage,
            &mut notifications,
        ),
        id,
        contact,
    )
    .unwrap();
    assert!(matches!(
        applied.furniture_damage,
        Some(StaticDamageOutcome::Started { .. })
    ));
    assert!(
        matches!(
            applied.collision_static_damage,
            Some(StaticDamageOutcome::Duplicate { .. })
        ),
        "C890 static program is registered before11760 submits its collision packet"
    );
    assert!(applied.actor_damage.unwrap().death_publication.is_some());
    assert_eq!(
        scheduler.family_for(id),
        Some(SpecializedActorTaskFamily::Intro2CommonDying)
    );
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
    assert_eq!(
        entity.collision.last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(17)
    );
    assert!(
        matches!(entity.actor_task_state(ActorTaskSlot::Primary),Some(ActorTaskRuntime::CommonDying(task)) if task.elapsed_ms()==0)
    );
    assert_eq!(
        fx.particle_count(),
        0,
        "11760 has no10EB0 capability8 particle suffix"
    );
    assert!(
        fx.take_positional_sounds()
            .iter()
            .all(|sound| sound.sound_id != 84),
        "no accepted primary cue"
    );
}

#[v2k_test_support::retail_test]
fn native_type58_later_damage_failure_retains_response_and_pending_current_graph() {
    let Some((mut session, mut manager, mut scheduler, id)) = fixture() else {
        return;
    };
    let contact = selected_contact(&mut session, 0, false);
    let entity = manager.entity_mut(id).unwrap();
    entity.set_velocity_raw([-8192, 0, 0]);
    entity.collision.pair_callbacks.damage_modifier_address = RetailRuntimeValue::Unresolved;
    let health = entity.collision.health_raw;
    let old = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    let mut fx = WorldFx::new();
    let mut damage = StaticDamageScheduler::default();
    let mut notifications = GameplayNotifications::default();
    assert!(matches!(
        apply_selected(
            &mut frame(
                &mut session,
                &mut manager,
                &mut scheduler,
                &mut fx,
                &mut damage,
                &mut notifications
            ),
            id,
            contact
        ),
        Err(Intro2Type58ContactBlock::Damage(_))
    ));
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.velocity_raw(), [0; 3]);
    assert_eq!(entity.collision.health_raw, health);
    assert_ne!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        Some(old)
    );
    assert!(damage.contains_cell(contact.cell));
    assert!(scheduler.intro2_type58_has_pending_prefix(id));
}

#[v2k_test_support::retail_test]
fn native_type58_pending_contact_owner_rejects_before_motion_rng_or_static_program() {
    let Some((mut session, mut manager, mut scheduler, id)) = fixture() else {
        return;
    };
    let owner = Intro2Type58Owner::adopt_blocked_prefix(&manager, id).unwrap();
    scheduler.register_intro2_type58(owner);
    let entity = manager.entity_mut(id).unwrap();
    let before = (
        entity.current_behavior_context,
        entity.position_raw(),
        entity.velocity_raw(),
        entity.collision.health_raw,
    );
    let mut fx = WorldFx::new();
    let mut damage = StaticDamageScheduler::default();
    let mut notifications = GameplayNotifications::default();
    let result = resolve_intro2_type58_static_contact(
        &mut frame(
            &mut session,
            &mut manager,
            &mut scheduler,
            &mut fx,
            &mut damage,
            &mut notifications,
        ),
        id,
    );
    assert!(matches!(
        result,
        Intro2Type58ContactOutcome::Blocked {
            committed_prefix: false,
            ..
        }
    ));
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        (
            entity.current_behavior_context,
            entity.position_raw(),
            entity.velocity_raw(),
            entity.collision.health_raw
        ),
        before
    );
    assert_eq!(damage.active_program_count(), 0);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        WorldFx::new().next_shared_retail_random_u16()
    );
    assert!(scheduler.intro2_type58_has_pending_prefix(id));
}

#[v2k_test_support::retail_test]
fn native_type58_ineligible_subject_does_not_require_a_completed_contact_owner() {
    let Some((mut session, mut manager, mut scheduler, id)) = fixture() else {
        return;
    };
    scheduler
        .register_intro2_type58(Intro2Type58Owner::adopt_blocked_prefix(&manager, id).unwrap());
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(1);
    let mut fx = WorldFx::new();
    let mut damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::default();
    assert_eq!(
        resolve_intro2_type58_static_contact(
            &mut frame(
                &mut session,
                &mut manager,
                &mut scheduler,
                &mut fx,
                &mut damage,
                &mut notifications
            ),
            id
        ),
        Intro2Type58ContactOutcome::Ineligible
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        WorldFx::new().next_shared_retail_random_u16()
    );
    assert_eq!(damage.active_program_count(), 0);
}
