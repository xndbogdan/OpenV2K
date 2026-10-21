//! Type16/Type128 capture transport on the shared captor lane.

use super::*;
use crate::{
    entity::EntityManager,
    gameplay_notifications::GameplayNotifications,
    intro2_contacts::Intro2ContactFrame,
    native_actor_capture::{
        carry_tasks::carrying_variant,
        pair::{
            resolve_native_captor_active_contacts, CaptureFeedbackPolicy, NativeCaptorPairOutcome,
        },
    },
    native_type28::tests::{activate, adopt_world, class, context_target, entity, fixture, tick},
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskProductionOutcome, SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
    world_fx::WorldFx,
};

/// Every authored Type16 (18/20/21/24/31/35/37) and Type128 (25/28) world.
const WORLDS: [u32; 9] = [18, 20, 21, 24, 31, 35, 37, 25, 28];

fn captor_ids(manager: &EntityManager) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|entity| matches!(entity.entity_type, 16 | 128))
        .map(|entity| entity.id)
        .collect()
}

/// 11AD0's pair walk in live order; no pair naming a Type16/128 may block.
fn walk_pairs(
    manager: &mut EntityManager,
    session: &mut GameSession,
    fx: &mut WorldFx,
    static_damage: &mut StaticDamageScheduler,
    notifications: &mut GameplayNotifications,
    tasks: &mut SpecializedActorTaskScheduler,
    t: u32,
) {
    let live: Vec<u32> = manager.retail_live_order_ids().collect();
    for id in live {
        if !manager
            .iter_all()
            .any(|entity| entity.id == id && entity.active)
        {
            continue;
        }
        let pairs = resolve_native_captor_active_contacts(
            &mut Intro2ContactFrame {
                entities: manager,
                resources: &mut session.cache,
                world_fx: fx,
                static_damage,
                notifications,
                retail_tick: t,
                actor_tasks: tasks,
            },
            id,
            CaptureFeedbackPolicy::Gameplay,
        );
        if let NativeCaptorPairOutcome::Blocked { reason, .. } = &pairs {
            // Other owners' boundaries (unported types, ordinary peasants that
            // production walks through their own path) are not this lane's.
            let text = format!("{reason:?}");
            let named: Vec<(u32, u32)> = manager
                .iter_all()
                .filter(|e| {
                    e.id == id
                        || text.contains(&format!("entity_id: {},", e.id))
                        || text.contains(&format!("entity_id: {} ", e.id))
                })
                .map(|e| (e.id, e.entity_type))
                .collect();
            assert!(
                !named.iter().any(|&(_, kind)| matches!(kind, 16 | 128))
                    && !matches!(
                        reason,
                        crate::native_actor_capture::pair::NativeCaptorPairBlock::UnresolvedBehavior {
                            entity_type: 16 | 128,
                            ..
                        }
                    ),
                "tick{t} subject{id} {named:?}: {pairs:?}"
            );
        }
    }
}

fn assert_pass(
    pass: &crate::specialized_actor_task_production::SpecializedActorTaskProductionPass,
    label: &str,
) {
    assert_pass_in(None, pass, label)
}

fn assert_pass_in(
    manager: Option<&EntityManager>,
    pass: &crate::specialized_actor_task_production::SpecializedActorTaskProductionPass,
    label: &str,
) {
    if let Some(manager) = manager {
        for outcome in &pass.outcomes {
            if let SpecializedActorTaskProductionOutcome::Intro2Type16(
                Intro2Type16Outcome::Blocked { entity_id, .. },
            ) = outcome
            {
                let e = entity(manager, *entity_id);
                eprintln!(
                    "{label} blocked id{entity_id} variant {:?} style {:?} P {:?} S {:?} T {:?} rel {:?}",
                    carrying_variant(e),
                    match e.current_behavior_context {
                        RetailRuntimeValue::Known(Some(c)) => Some((c.active_style().style_address(), c.style_table_index_raw_at_0x10())),
                        _ => None,
                    },
                    e.actor_task_state(crate::actor_task_owner::ActorTaskSlot::Primary),
                    e.actor_task_state(crate::actor_task_owner::ActorTaskSlot::Secondary),
                    e.actor_task_state(crate::actor_task_owner::ActorTaskSlot::Tertiary),
                    e.native_capture_relation.is_some(),
                );
            }
        }
    }
    assert!(pass.block.is_none(), "{label}: {:?}", pass.block);
    for outcome in &pass.outcomes {
        if let SpecializedActorTaskProductionOutcome::Intro2Type16(outcome) = outcome {
            assert!(
                !matches!(outcome, Intro2Type16Outcome::Blocked { .. }),
                "{label}: {outcome:?}"
            );
        }
    }
}

/// A minute of every Type16/128 world with its pair walks: no owner or pair
/// blocks. Reports the classes and captures each world reaches.
#[v2k_test_support::retail_test]
fn type16_worlds_run_a_minute_with_pairs() {
    for level in WORLDS {
        let (mut session, mut manager, mut fx) = fixture(level);
        let ids = captor_ids(&manager);
        activate(&mut manager, &ids);
        let mut tasks = adopt_world(&mut manager);
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let mut classes = std::collections::BTreeSet::new();
        let mut carried_ticks = 0;
        for t in 1..=3000u32 {
            let pass = tick(
                &mut tasks,
                &mut manager,
                &mut session,
                &mut fx,
                &mut static_damage,
                &mut notifications,
                t,
            );
            assert_pass(&pass, &format!("world{level} tick{t}"));
            walk_pairs(
                &mut manager,
                &mut session,
                &mut fx,
                &mut static_damage,
                &mut notifications,
                &mut tasks,
                t,
            );
            for &id in &ids {
                let Some(captor) = manager
                    .iter_all()
                    .find(|entity| entity.id == id && entity.active)
                else {
                    continue;
                };
                if captor.native_capture_relation.is_some() {
                    carried_ticks += 1;
                }
                if matches!(
                    captor.current_behavior_context,
                    RetailRuntimeValue::Known(Some(_))
                ) {
                    classes.insert(class(&manager, id));
                }
            }
        }
        println!("world{level} classes {classes:?} carried-ticks {carried_ticks}");
    }
}

/// A person beside a captor that pursues it: bring the bodies into contact
/// and return the tick the person attached.
#[allow(clippy::type_complexity)]
fn capture_a_person(
    level: u32,
    row: Type16Row,
) -> (
    GameSession,
    EntityManager,
    WorldFx,
    SpecializedActorTaskScheduler,
    StaticDamageScheduler,
    GameplayNotifications,
    u32,
    u32,
    u32,
) {
    let (mut session, mut manager, mut fx) = fixture(level);
    let ids = captor_ids(&manager);
    activate(&mut manager, &ids);
    let people: Vec<u32> = manager
        .iter_all()
        .filter(|entity| entity.active && entity.capability_flags & 0x0c00 != 0)
        .map(|entity| entity.id)
        .collect();
    assert!(!people.is_empty(), "world{level} authors people");
    let mut tasks = adopt_world(&mut manager);
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    // Put every person beside a captor so PeopleNearby can weigh Capture9.
    let captors: Vec<u32> = ids
        .iter()
        .copied()
        .filter(|&id| type16_row(entity(&manager, id)) == Some(row))
        .collect();
    for (index, &person) in people.iter().enumerate() {
        let [x, y, z] = entity(&manager, captors[index % captors.len()]).position_raw();
        manager.entity_mut(person).unwrap().set_position_raw([
            x.wrapping_add(0x180),
            y,
            z.wrapping_add(0x180),
        ]);
    }
    for t in 1..=6000u32 {
        let pass = tick(
            &mut tasks,
            &mut manager,
            &mut session,
            &mut fx,
            &mut static_damage,
            &mut notifications,
            t,
        );
        assert_pass(&pass, &format!("world{level} tick{t}"));
        // Pursuit 4C8038: bring the pursued person into contact.
        for &captor in &captors {
            if !manager
                .iter_all()
                .any(|entity| entity.id == captor && entity.active)
            {
                continue;
            }
            let style = match entity(&manager, captor).current_behavior_context {
                RetailRuntimeValue::Known(Some(context)) => context.active_style().style_address(),
                _ => continue,
            };
            if style != 0x004c_8038 {
                continue;
            }
            if let Some(person) = context_target(&manager, captor).filter(|p| people.contains(p)) {
                let [x, y, z] = entity(&manager, captor).position_raw();
                manager
                    .entity_mut(person)
                    .unwrap()
                    .set_position_raw([x.wrapping_add(0x20), y, z]);
            }
        }
        walk_pairs(
            &mut manager,
            &mut session,
            &mut fx,
            &mut static_damage,
            &mut notifications,
            &mut tasks,
            t,
        );
        for &captor in &captors {
            let Some(relation) = manager
                .iter_all()
                .find(|entity| entity.id == captor && entity.active)
                .and_then(|entity| entity.native_capture_relation)
            else {
                continue;
            };
            let person = relation.child.entity_id;
            assert_eq!(entity(&manager, person).attached_to, Some(captor));
            return (
                session,
                manager,
                fx,
                tasks,
                static_damage,
                notifications,
                captor,
                person,
                t,
            );
        }
    }
    panic!("world{level}: no {row:?} ever attached a person");
}

/// C910 attaches a pursued person to a Type16's J row; the carrying variants
/// then run for ten seconds, with 18640 keeping the person at the captor.
#[v2k_test_support::retail_test]
fn a_type16_captures_and_carries_its_pursued_person() {
    let (
        mut session,
        mut manager,
        mut fx,
        mut tasks,
        mut static_damage,
        mut notifications,
        captor,
        person,
        at,
    ) = capture_a_person(35, Type16Row::Type16);
    assert!(carrying_variant(entity(&manager, captor)).is_some());
    let mut variants = std::collections::BTreeSet::new();
    for t in at + 1..=at + 500 {
        let pass = tick(
            &mut tasks,
            &mut manager,
            &mut session,
            &mut fx,
            &mut static_damage,
            &mut notifications,
            t,
        );
        assert_pass_in(Some(&manager), &pass, &format!("tick{t}"));
        walk_pairs(
            &mut manager,
            &mut session,
            &mut fx,
            &mut static_damage,
            &mut notifications,
            &mut tasks,
            t,
        );
        let held = entity(&manager, captor).native_capture_relation;
        if held.is_some_and(|relation| relation.child.entity_id == person) {
            if let Some(variant) = carrying_variant(entity(&manager, captor)) {
                variants.insert(variant);
            }
            // 18640 copies the captor's position (plus the slot offset).
            let captor_position = entity(&manager, captor).position_raw();
            let person_position = entity(&manager, person).position_raw();
            let near = captor_position
                .iter()
                .zip(person_position)
                .all(|(a, b)| a.wrapping_sub(b).unsigned_abs() < 0x200);
            assert!(near, "tick{t}: {captor_position:?} {person_position:?}");
        }
    }
    println!("carrying variants {variants:?}");
    assert!(!variants.is_empty());
}

fn lethal_hit(id: u32) -> crate::world_fx::ParticleEntityImpact {
    crate::native_type28::tests::lethal_hit(id)
}

/// A Type16 killed while carrying: DB80's D040 pops the person, whose own
/// release reselects it alive, then class12 replaces the captor's graph.
#[v2k_test_support::retail_test]
fn a_type16_killed_while_carrying_lets_its_captive_go() {
    use crate::shared_actor_impact::{
        apply_shared_actor_particle_hit, SharedActorImpactFrame, SharedActorImpactOutcome,
    };
    let (
        mut session,
        mut manager,
        mut fx,
        mut tasks,
        mut static_damage,
        mut notifications,
        captor,
        person,
        at,
    ) = capture_a_person(35, Type16Row::Type16);
    let outcome = apply_shared_actor_particle_hit(
        SharedActorImpactFrame {
            resources: &session.cache,
            entities: &mut manager,
            world_fx: &mut fx,
            scheduler: &mut tasks,
            notifications: &mut notifications,
            retail_tick: at + 1,
        },
        lethal_hit(captor),
    )
    .unwrap();
    let SharedActorImpactOutcome::Type16(impact::Intro2Type16ImpactOutcome::Applied(applied)) =
        outcome
    else {
        panic!("{outcome:?}")
    };
    assert!(applied.death_publication.is_some());
    let dead = entity(&manager, captor);
    assert_eq!(dead.collision.health_raw, RetailRuntimeValue::Known(0));
    assert!(dead.native_capture_relation.is_none());
    assert_eq!(class(&manager, captor), 12);
    let released = entity(&manager, person);
    assert_eq!(released.attached_to, None);
    assert_ne!(released.collision.health_raw, RetailRuntimeValue::Known(0));
    for t in at + 2..=at + 250 {
        let pass = tick(
            &mut tasks,
            &mut manager,
            &mut session,
            &mut fx,
            &mut static_damage,
            &mut notifications,
            t,
        );
        assert_pass_in(Some(&manager), &pass, &format!("tick{t}"));
        walk_pairs(
            &mut manager,
            &mut session,
            &mut fx,
            &mut static_damage,
            &mut notifications,
            &mut tasks,
            t,
        );
    }
}

/// Type128 shares the transport: C910 attaches a person to a carrier. Its
/// class63 death while carrying stays held before its first write.
#[v2k_test_support::retail_test]
fn a_type128_captures_its_pursued_person() {
    let (session, mut manager, mut fx, _, _, _, captor, person, at) =
        capture_a_person(25, Type16Row::Type128);
    assert!(carrying_variant(entity(&manager, captor)).is_some());
    let before = entity(&manager, captor).collision.health_raw;
    assert_eq!(
        crate::class49_death::begin_class49_standard_death(
            &mut manager,
            captor,
            &session.cache,
            &mut fx,
            at + 1,
        ),
        Err(crate::class49_death::Class49DeathBlock::Runtime(
            "unaudited death style"
        ))
    );
    assert_eq!(entity(&manager, person).attached_to, Some(captor));
    assert_eq!(entity(&manager, captor).collision.health_raw, before);
}
