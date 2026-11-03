//! A person touching a living Type74: `FUN_0040CAF0` beams it to the Main Base.

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
fn a_person_touching_a_remote_beamer_is_beamed_to_the_main_base() {
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
    let main_base = entities.iter_all().find(|e| e.entity_type == 6).unwrap().id;
    let center = [4096, 6000, 4096];
    let ids = entities.retail_live_order_ids().collect::<Vec<_>>();
    for &id in ids.iter().filter(|&&id| id != main_base) {
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
    let base_position = actor(&entities, main_base).unwrap().position_raw();
    let person_state = actor(&entities, person)
        .unwrap()
        .collision
        .state_flags_at_0x08;
    let newest = entities.iter_all().map(|e| e.id).max().unwrap();
    let mut tasks = SpecializedActorTaskScheduler::new();
    tasks.adopt_class0_actors(&entities);
    tasks.adopt_native_type86(&mut entities);
    let mut notifications = GameplayNotifications::new();
    let outcome = resolve_native_captor_active_contacts(
        &mut Intro2ContactFrame {
            entities: &mut entities,
            resources: &mut session.cache,
            world_fx: &mut fx,
            static_damage: &mut StaticDamageScheduler::new(),
            notifications: &mut notifications,
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
    let stages = &visits[0].stages;
    assert!(
        stages.contains(&NativeCaptorPairStage::DispatchRemoteBeam),
        "{stages:?}"
    );
    // The A300 cancels 12760's bounce.
    assert!(visits[0].physical_suppressed);
    assert!(!stages
        .iter()
        .any(|stage| matches!(stage, NativeCaptorPairStage::Physical { .. })));
    // The Type93 sits at the Main Base's X, Z - 0x100 on the terrain, raised
    // one unit by 08F00, and holds the person.
    let proxy = entities
        .iter_all()
        .find(|e| e.id > newest && e.entity_type == 93)
        .unwrap()
        .id;
    let z = base_position[2].wrapping_sub(0x100);
    let ground = session
        .cache
        .level_terrain()
        .unwrap()
        .bilinear_height_raw(base_position[0], z);
    assert_eq!(
        actor(&entities, proxy).unwrap().position_raw(),
        [base_position[0], ground.wrapping_add(1), z]
    );
    let held = actor(&entities, person).unwrap();
    assert_eq!(held.attached_to, Some(proxy));
    // Retail's beamed native: 0x00C68825 -> 0x00C21025.
    let before = person_state.masked(0x0004_9800);
    let after = held.collision.state_flags_at_0x08.masked(0x0004_9800);
    assert_eq!(
        (before, after),
        (
            RetailRuntimeValue::Known(0x0004_8800),
            RetailRuntimeValue::Known(0x1000)
        )
    );
    // 40CAF0's walk sets the Type6's +60 to its own handle.
    assert_eq!(
        actor(&entities, main_base)
            .unwrap()
            .collision
            .recent_relation_id_at_0x60,
        RetailRuntimeValue::Known(Some(main_base))
    );
    // Type93's settled release lets the person go where it stands.
    let mut released = None;
    let mut last = [0; 3];
    for t in 5001..5300 {
        let terrain = session.cache.level_terrain().unwrap();
        let blocked =
            entities.update_late_tail_materialisers(crate::entity::LateTailMaterialiserFrame {
                elapsed_micros: 20_000,
                terrain,
                world_fx: &mut fx,
                scheduler: &mut tasks,
                notifications: &mut notifications,
                retail_tick: t,
            });
        assert!(blocked.is_empty(), "{blocked:?}");
        // The Main Base's own pass then pushes the Type93 out of its body.
        if entities.iter_all().any(|e| e.id == proxy) {
            crate::main_base_conversion_live::resolve_main_base_materialiser_contacts(
                &mut entities,
                &session.cache,
                t,
            )
            .unwrap();
            last = actor(&entities, proxy).unwrap().position_raw();
        }
        if actor(&entities, person).unwrap().attached_to.is_none() {
            released = Some(t);
            break;
        }
    }
    released.expect("the Type93 releases its person");
    assert!(!entities.iter_all().any(|e| e.id == proxy));
    // Retail's world-21 lab beam: the Base pushed its Type93 from Z -31744
    // through -31931, -32052, -32090 and -32098 to -32100 (Y 3228 at the
    // first push), and the person was let go there, at Y 3009.
    assert_eq!(last, [base_position[0], 3009, -32101]);
    assert_eq!(actor(&entities, person).unwrap().position_raw(), last);
}
