//! Release world evidence must close before18500 changes parent/child custody.
use super::*;
use crate::{
    entity_relation_release::relation_attach_state_word_after,
    native_actor_attachment::NativeActorAttachOutcome,
    native_type122::construction_tests::native_fixture_with_player,
    ordinary_type9_cargo::Type9CargoReleasePosition,
};

fn actor(manager: &EntityManager, id: u32) -> &Entity {
    manager.iter_all().find(|entity| entity.id == id).unwrap()
}

#[v2k_test_support::retail_test]
fn type7_release_preflights_job_capacity_at_the_actual_release_position_without_mutation() {
    for materialiser in [false, true] {
        let (_, mut manager, mut fx) = native_fixture_with_player(34);
        manager.cleanup_pending_actor_deferred_destroys();
        let id = manager.iter_all().find(|e| e.entity_type == 7).unwrap().id;
        let parent = manager.player().unwrap().id;
        let factory = manager.iter_all().find(|e| e.entity_type == 66).unwrap().id;
        let attach = cargo::prepare_attach(&manager, id, parent).unwrap();
        let RetailRuntimeValue::Known(Some(rows)) =
            &mut manager.entity_mut(parent).unwrap().sub_j_attachment_runtime
        else {
            panic!("native player Sub-J")
        };
        rows.append(id).unwrap();
        let entity = manager.entity_mut(id).unwrap();
        entity.collision.state_flags_at_0x08 =
            relation_attach_state_word_after(entity.collision.state_flags_at_0x08);
        entity.attached_to = Some(parent);
        assert_eq!(
            cargo::commit_attach(&mut manager, id, attach, &mut fx),
            NativeActorAttachOutcome::RetainedGraph
        );

        let [x, y, z] = actor(&manager, id).position_raw();
        let release_raw = [x.wrapping_add(if materialiser { 8000 } else { 0 }), y, z];
        let release = if materialiser {
            Type9CargoReleasePosition::Materialiser(release_raw)
        } else {
            Type9CargoReleasePosition::Retained
        };
        let others = manager
            .iter_all()
            .filter(|e| e.id != id)
            .map(|e| e.id)
            .collect::<Vec<_>>();
        for other in others {
            manager.entity_mut(other).unwrap().set_position_raw([
                release_raw[0].wrapping_add(16000),
                y,
                z.wrapping_add(16000),
            ]);
        }
        manager.entity_mut(factory).unwrap().set_position_raw([
            release_raw[0].wrapping_add(512),
            y,
            z,
        ]);
        manager.entity_mut(factory).unwrap().base_factory_runtime = RetailRuntimeValue::Unresolved;
        let entity = actor(&manager, id);
        let body = (
            entity.collision.clone(),
            entity.position_raw(),
            entity.velocity_raw(),
            entity.rotation_heading_pitch_roll_raw(),
            entity.physical_body_basis_q31,
            entity.attached_to,
            entity.native_type86_anchor_raw_at_0x90,
        );
        let components = (
            entity.sub_a_propulsion_runtime,
            entity.type8_sub_d_runtime,
            entity.type8_sub_d_frame_owner,
            entity.actor_animation_runtime,
        );
        let graph = Type86Owner::published_graph(entity);
        let task_storage = format!("{:?}", entity.actor_tasks);
        let receipt = entity.native_type86_runtime;
        let parent_rows = actor(&manager, parent).sub_j_attachment_runtime.clone();
        let live_order = manager.retail_live_order_ids().collect::<Vec<_>>();
        let pending = manager
            .pending_fresh_level1_type9_resource_text_receipts()
            .to_vec();
        let mut oracle = fx.fork_for_main_base_abort_transaction();
        assert!(matches!(
            cargo::prepare_release(&manager, id, parent, release),
            Err(Type86Block::Runtime("Job Nearby evidence"))
        ));
        let entity = actor(&manager, id);
        assert_eq!(
            (
                entity.collision.clone(),
                entity.position_raw(),
                entity.velocity_raw(),
                entity.rotation_heading_pitch_roll_raw(),
                entity.physical_body_basis_q31,
                entity.attached_to,
                entity.native_type86_anchor_raw_at_0x90,
            ),
            body
        );
        assert_eq!(
            (
                entity.sub_a_propulsion_runtime,
                entity.type8_sub_d_runtime,
                entity.type8_sub_d_frame_owner,
                entity.actor_animation_runtime,
            ),
            components
        );
        assert_eq!(Type86Owner::published_graph(entity), graph);
        assert_eq!(format!("{:?}", entity.actor_tasks), task_storage);
        assert_eq!(entity.native_type86_runtime, receipt);
        assert_eq!(
            actor(&manager, parent).sub_j_attachment_runtime,
            parent_rows
        );
        assert_eq!(
            manager.retail_live_order_ids().collect::<Vec<_>>(),
            live_order
        );
        assert_eq!(
            manager.pending_fresh_level1_type9_resource_text_receipts(),
            pending
        );
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );

        // The same unresolved component is irrelevant outside the exact
        // strict3072 cube; materialiser preflight uses its future pose.
        manager.entity_mut(factory).unwrap().set_position_raw([
            release_raw[0].wrapping_add(3072),
            y,
            z,
        ]);
        assert!(cargo::prepare_release(&manager, id, parent, release).is_ok());
    }
}
