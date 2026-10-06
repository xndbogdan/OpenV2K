use super::*;
use crate::{
    actor_task_owner::PreparedActorTask,
    defecate_virus::DefecateVirusTerrainTaskState,
    entity_behavior::{
        audited_behavior_style, behavior_program, BehaviorChoiceListSource, BehaviorContextRuntime,
    },
    infection_evolution::{InfectionCellWrite, INFECTION_TERRAIN_TYPE_BIT},
    intro2_type47_live::world::native_intro2_fixture,
};

fn prepare(manager: &mut EntityManager) -> u32 {
    let id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(5))
        .unwrap()
        .id;
    let entity = manager.entity_mut(id).unwrap();
    assert!(intro2_type16_allocation_authenticates(entity));
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(
        BehaviorContextRuntime::named_audited(
            behavior_program(4).unwrap(),
            0,
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            RetailRuntimeValue::Known(None),
            RetailRuntimeValue::Known(0),
            *audited_behavior_style(4, 0).unwrap(),
        )
        .unwrap(),
    ));
    entity.actor_tasks.replace_prepared(
        ActorTaskSlot::Tertiary,
        PreparedActorTask::new(ActorTaskRuntime::DefecateVirusTerrain(
            DefecateVirusTerrainTaskState::new(0),
        )),
    );
    entity.collision.state_flags_at_0x08.overwrite(u32::MAX, 0);
    entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    id
}

fn age(manager: &EntityManager, id: u32) -> u32 {
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    let Some(ActorTaskRuntime::DefecateVirusTerrain(task)) =
        entity.actor_task_state(ActorTaskSlot::Tertiary)
    else {
        panic!()
    };
    assert_eq!(task.lifetime_ms(), 0);
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Tertiary)
        .unwrap();
    assert!(
        !entity
            .actor_tasks
            .wrapper_flags(task_id)
            .unwrap()
            .in_callback
    );
    task.elapsed_ms()
}

#[v2k_test_support::retail_test]
fn native_type16_defecate_zero_lifetime_and_suppression_preserve_rng() {
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = prepare(&mut manager);
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(0x1000, 0x1000);
    let mut fx = WorldFx::new();
    let mut expected = fx.fork_for_main_base_abort_transaction();
    for mode in [
        CommonMoverDispatchMode::Normal,
        CommonMoverDispatchMode::Restricted,
    ] {
        tick_intro2_type16_defecate(
            &mut manager,
            id,
            &mut session.cache,
            &mut fx,
            mode,
            30_000_000,
        )
        .unwrap();
    }
    assert_eq!(age(&manager, id), 60_000);
    assert_eq!(fx.particle_count(), 0);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_type16_defecate_rejected_gate_needs_no_basis_but_pass_preserves_late_prefix() {
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = prepare(&mut manager);
    let mut fx = WorldFx::new();
    let mut expected = fx.fork_for_main_base_abort_transaction();
    // dt0 always rejects. The native allocation deliberately has an unknown basis.
    expected.next_shared_retail_random_u16();
    tick_intro2_type16_defecate(
        &mut manager,
        id,
        &mut session.cache,
        &mut fx,
        CommonMoverDispatchMode::Normal,
        0,
    )
    .unwrap();
    // dt/4=65536 always passes a u16 gate; the subsequent basis read blocks.
    expected.next_shared_retail_random_u16();
    assert_eq!(
        tick_intro2_type16_defecate(
            &mut manager,
            id,
            &mut session.cache,
            &mut fx,
            CommonMoverDispatchMode::Normal,
            262_144
        ),
        Err(Block::Runtime("Defecate body basis"))
    );
    assert_eq!(age(&manager, id), 262);
    assert_eq!(fx.particle_count(), 0);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_type16_defecate_coarse_writes_wrapped_cell_z_then_x_without_geometry() {
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = prepare(&mut manager);
    manager
        .entity_mut(id)
        .unwrap()
        .set_position_raw([i16::MAX, 0, i16::MIN]);
    let mut fx = WorldFx::new();
    let mut expected = fx.fork_for_main_base_abort_transaction();
    let z = i16::MIN
        .wrapping_add((expected.next_shared_retail_random_u16() >> 7).wrapping_sub(0x100) as i16);
    let x = i16::MAX
        .wrapping_add((expected.next_shared_retail_random_u16() >> 7).wrapping_sub(0x100) as i16);
    let cell = [(x as u16 >> 8) as u8, (z as u16 >> 8) as u8];
    session
        .cache
        .apply_level_infection_writes(&[InfectionCellWrite {
            cell,
            infected: false,
        }])
        .unwrap();
    tick_intro2_type16_defecate(
        &mut manager,
        id,
        &mut session.cache,
        &mut fx,
        CommonMoverDispatchMode::Restricted,
        20_000,
    )
    .unwrap();
    let index = usize::from(cell[0]) * v2k_formats::terrain::GRID_SIZE + usize::from(cell[1]);
    assert_ne!(
        session.cache.terrain().unwrap().cells[index].terrain_type & INFECTION_TERRAIN_TYPE_BIT,
        0
    );
    assert_eq!(age(&manager, id), 20);
    assert_eq!(fx.particle_count(), 0);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}
