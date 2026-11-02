//! A person touching a living Type74: the held `FUN_0040CAF0` beam.

use super::{
    pair::{actor, *},
    *,
};
use crate::{
    intro2_contacts::Intro2ContactFrame,
    player_active_contact::{active_pair_body_from_entity, classify_oriented_active_pair_contact},
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
};

#[v2k_test_support::retail_test]
fn a_person_touching_a_remote_beamer_is_held_whole() {
    let (mut session, mut entities, mut fx) =
        crate::native_type122::construction_tests::native_fixture(21);
    let beamer = entities
        .iter_all()
        .find(|e| e.entity_type == 74)
        .unwrap()
        .id;
    let person = entities
        .iter_all()
        .find(|e| e.entity_type == 86)
        .unwrap()
        .id;
    let center = [4096, 6000, 4096];
    let ids = entities.retail_live_order_ids().collect::<Vec<_>>();
    for &id in &ids {
        let e = entities.entity_mut(id).unwrap();
        e.set_motion_raw(
            if [beamer, person].contains(&id) {
                center
            } else {
                [16000, 16000, 16000]
            },
            [0; 3],
        );
        if [beamer, person].contains(&id) {
            e.collision.state_flags_at_0x08.overwrite(0x8000, 0x8000);
            e.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        }
    }
    let (subject, candidate) =
        if ids.iter().position(|id| *id == beamer) < ids.iter().position(|id| *id == person) {
            (beamer, person)
        } else {
            (person, beamer)
        };
    let mut found = false;
    'search: for dy in [-150i16, -75, 0, 75, 150] {
        for dx in [-240i16, -160, -80, 0, 80, 160, 240] {
            for dz in [-240i16, -160, -80, 0, 80, 160, 240] {
                entities.entity_mut(person).unwrap().set_position_raw([
                    center[0].wrapping_add(dx),
                    center[1].wrapping_add(dy),
                    center[2].wrapping_add(dz),
                ]);
                let subject_actor = actor(&entities, subject).unwrap();
                let entry = active_pair_body_from_entity(subject_actor, &session.cache);
                if classify_oriented_active_pair_contact(
                    crate::player_active_contact::OrientedActivePairContactRequest {
                        subject: subject_actor,
                        candidate: actor(&entities, candidate).unwrap(),
                        subject_entry: &entry,
                        retail_tick: 5000,
                    },
                    &session.cache,
                )
                .unwrap()
                .is_some()
                {
                    found = true;
                    break 'search;
                }
            }
        }
    }
    assert!(found, "beamer/person narrow solids overlap");
    let before = [beamer, person].map(|id| {
        let e = actor(&entities, id).unwrap();
        (e.position_raw(), e.velocity_raw(), e.collision.clone())
    });
    let count = entities.iter_all().count();
    let mut tasks = SpecializedActorTaskScheduler::new();
    tasks.adopt_class0_actors(&entities);
    tasks.adopt_native_type86(&mut entities);
    let outcome = resolve_native_captor_active_contacts(
        &mut Intro2ContactFrame {
            entities: &mut entities,
            resources: &mut session.cache,
            world_fx: &mut fx,
            static_damage: &mut StaticDamageScheduler::new(),
            notifications: &mut GameplayNotifications::new(),
            retail_tick: 5000,
            actor_tasks: &mut tasks,
        },
        subject,
        CaptureFeedbackPolicy::Gameplay,
    );
    let NativeCaptorPairOutcome::Resolved { visits } = &outcome else {
        panic!("{outcome:?}")
    };
    assert_eq!(visits.len(), 1);
    assert_eq!(
        visits[0].stages,
        [NativeCaptorPairStage::RemoteBeamHeld { beamer, person }]
    );
    assert_eq!(held_remote_beams(&outcome), [(beamer, person)]);
    // Nothing of 11AD0 ran: no Type93, callback, bounce or damage.
    assert_eq!(entities.iter_all().count(), count);
    assert_eq!(
        [beamer, person].map(|id| {
            let e = actor(&entities, id).unwrap();
            (e.position_raw(), e.velocity_raw(), e.collision.clone())
        }),
        before
    );
}
