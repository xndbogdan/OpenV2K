use super::*;
use crate::{
    common_mover::type9_owner::{
        OrdinaryType9OwnerExternalBlock, OrdinaryType9OwnerPoll, OrdinaryType9OwnerResume,
    },
    entity_view_detail::{RetailViewDetail, RetailViewDetailContext, VIEW_DETAIL_STATE_MASK},
    ordinary_type9_outer_tail::start_outer_tail_transaction,
};

fn completed_tail_fixture() -> (
    EntityManager,
    OrdinaryType9WanderProductionOwner,
    ResourceCache,
    WorldFx,
    u32,
) {
    let (mut manager, owner, id) = production_owner_fixture();
    let entity = manager.entity_mut_for_test(id).unwrap();
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    let position = entity.position_raw();
    let eye = [
        i32::from(position[0]),
        i32::from(position[1]),
        i32::from(position[2]) - 256,
    ];
    assert_eq!(
        RetailViewDetailContext::from_raw(eye, 0, (52, 30))
            .publish(position, &mut entity.collision.state_flags_at_0x08),
        RetailRuntimeValue::Known(Some(RetailViewDetail::Full))
    );
    let resources = terrain_resources();
    let mut world_fx = WorldFx::new();
    let tick = tick_ordinary_type9_wander_owner_with_random(
        &mut manager,
        owner,
        frame(&resources),
        &mut world_fx,
        |_| 0,
    );
    let owner = tick
        .retained_owner
        .expect("the actor completed its first callback");
    assert!(
        matches!(
            owner.outer_tail,
            Some(OrdinaryType9OuterTailCustody::Complete { .. })
        ),
        "{:?}",
        tick.outcome
    );
    (manager, owner, resources, world_fx, id)
}

#[test]
fn completed_tail_accepts_presented_detail_before_next_tick_and_cargo() {
    for (x_offset, detail) in [
        (0x4000, RetailViewDetail::Coarse),
        (26 * 256 + 1, RetailViewDetail::Broader),
    ] {
        let (mut manager, owner, resources, mut world_fx, id) = completed_tail_fixture();
        let lease = owner.actor_lease();
        assert_eq!(owner.completed_visit_lease(&manager), Some(lease));
        let entity = manager.entity_mut_for_test(id).unwrap();
        let animation_before_presentation = entity.actor_animation_runtime;
        let position = entity.position_raw();
        let eye = [
            i32::from(position[0]) - x_offset,
            i32::from(position[1]),
            i32::from(position[2]),
        ];
        assert_eq!(
            RetailViewDetailContext::from_raw(eye, 0, (52, 30))
                .publish(position, &mut entity.collision.state_flags_at_0x08),
            RetailRuntimeValue::Known(Some(detail))
        );
        assert_eq!(
            entity.actor_animation_runtime,
            animation_before_presentation
        );
        assert_eq!(
            owner.completed_visit_lease(&manager),
            Some(lease),
            "a presentation-only write must not prevent the next cargo attach"
        );
        let tick = tick_ordinary_type9_wander_owner_with_random(
            &mut manager,
            owner,
            frame(&resources),
            &mut world_fx,
            |_| 0,
        );
        assert!(
            !matches!(
                tick.outcome,
                OrdinaryType9WanderProductionOutcome::Dropped { .. }
            ),
            "{:?}",
            tick.outcome
        );
        assert!(tick.retained_owner.is_some(), "{:?}", tick.outcome);
        let entity = manager.entity_mut_for_test(id).unwrap();
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(VIEW_DETAIL_STATE_MASK),
            RetailRuntimeValue::Known(detail.state_bits()),
            "the old completed receipt must not restore its previous Full classification"
        );
        if detail == RetailViewDetail::Coarse {
            assert_eq!(entity.actor_animation_runtime, animation_before_presentation,
                "the fresh coarse callback preserves Sub-I instead of replaying the old detailed one");
        }
    }
}

#[test]
fn completed_tail_rejects_unrelated_state_motion_and_b2_changes() {
    for change in 0..4 {
        let (mut manager, owner, resources, mut world_fx, id) = completed_tail_fixture();
        let entity = manager.entity_mut_for_test(id).unwrap();
        match change {
            0 => {
                let RetailRuntimeValue::Known(before) =
                    entity.collision.state_flags_at_0x08.masked(0x400)
                else {
                    panic!("completed fixture has exact flags")
                };
                entity
                    .collision
                    .state_flags_at_0x08
                    .overwrite(0x400, before ^ 0x400);
            }
            1 => entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(1),
            2 => {
                let mut velocity = entity.velocity_raw();
                velocity[0] = velocity[0].wrapping_add(1);
                entity.set_motion_raw(entity.position_raw(), velocity);
            }
            _ => entity
                .collision
                .state_flags_at_0x08
                .invalidate(VIEW_DETAIL_STATE_MASK),
        }
        assert_eq!(
            owner.completed_visit_lease(&manager),
            None,
            "tamper {change}"
        );
        assert!(!outer_tail_observation_authenticates(
            &manager,
            owner.actor_lease(),
            owner.outer_tail.as_ref().unwrap()
        ));
        let mut random_calls = 0;
        let tick = tick_ordinary_type9_wander_owner_with_random(
            &mut manager,
            owner,
            frame(&resources),
            &mut world_fx,
            |_| {
                random_calls += 1;
                0
            },
        );
        assert!(
            matches!(
                tick.outcome,
                OrdinaryType9WanderProductionOutcome::Dropped { .. }
            ),
            "tamper {change}: {:?}",
            tick.outcome
        );
        assert_eq!(random_calls, 0);
    }
}

#[test]
fn pending_outer_transaction_keeps_frozen_detail_and_cannot_admit_presentation() {
    let (mut manager, mut owner, resources, _world_fx, id) = completed_tail_fixture();
    let expected_state = snapshot_outer_tail_state(&manager, owner.actor_lease()).unwrap();
    let transaction_id = take_outer_transaction_id(&mut owner.next_transaction_id);
    let (mut transaction, b2) =
        start_outer_tail_transaction(&manager, owner.actor_lease(), transaction_id, &resources, 1)
            .unwrap();
    let OrdinaryType9OwnerPoll::Action(issued) = transaction.poll() else {
        panic!("the transaction must issue its first state commit")
    };
    transaction
        .resume(
            issued.receipt,
            OrdinaryType9OwnerResume::Blocked {
                reason: OrdinaryType9OwnerExternalBlock::StateCommitUnavailable,
            },
        )
        .unwrap();
    let OrdinaryType9OwnerPoll::Blocked(block) = transaction.poll() else {
        panic!("the unexecuted commit must remain blocked")
    };
    owner.outer_tail = Some(OrdinaryType9OuterTailCustody::Transaction {
        transaction,
        block,
        origin_retail_tick: 0,
        expected_state,
        expected_animation_offset_at_0xb2: b2,
    });
    assert!(outer_tail_observation_authenticates(
        &manager,
        owner.actor_lease(),
        owner.outer_tail.as_ref().unwrap()
    ));
    assert_eq!(owner.completed_visit_lease(&manager), None);
    let parked_before = format!("{owner:?}");
    assert!(!owner.consume_completed_visit_for_player_contact(&manager));
    assert_eq!(format!("{owner:?}"), parked_before);
    // This write is deliberately outside the completed-visit admission gate.
    // A pending plan cannot accept it and later overwrite the new detail bits.
    manager
        .entity_mut_for_test(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(VIEW_DETAIL_STATE_MASK, 0);
    assert!(!outer_tail_observation_authenticates(
        &manager,
        owner.actor_lease(),
        owner.outer_tail.as_ref().unwrap()
    ));
}
