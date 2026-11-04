//! Type53 capture transport on the shared captor lane (world 15).

use super::*;
use crate::{
    gameplay_notifications::GameplayNotifications,
    intro2_contacts::Intro2ContactFrame,
    native_actor_capture::{
        carry_tasks::carrying_variant,
        pair::{
            resolve_native_captor_active_contacts, CaptureFeedbackPolicy, NativeCaptorPairOutcome,
        },
    },
    native_type28::tests::{activate, adopt_world, class, entity, fixture, tick},
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskProductionOutcome, SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
    world_fx::WorldFx,
};

struct World {
    session: GameSession,
    manager: EntityManager,
    fx: WorldFx,
    tasks: SpecializedActorTaskScheduler,
    static_damage: StaticDamageScheduler,
    notifications: GameplayNotifications,
}

impl World {
    /// One scheduler pass, then 11AD0's walk in live order. Neither may stop
    /// on a Type53 or on a pair naming one.
    fn step(&mut self, t: u32) {
        let pass = tick(
            &mut self.tasks,
            &mut self.manager,
            &mut self.session,
            &mut self.fx,
            &mut self.static_damage,
            &mut self.notifications,
            t,
        );
        assert!(pass.block.is_none(), "tick{t}: {:?}", pass.block);

        for outcome in &pass.outcomes {
            if let SpecializedActorTaskProductionOutcome::Intro2Type53(outcome) = outcome {
                assert!(
                    !matches!(outcome, Intro2Type53Outcome::Blocked { .. }),
                    "tick{t}: {outcome:?}"
                );
            }
        }
        let live: Vec<u32> = self.manager.retail_live_order_ids().collect();
        for id in live {
            if !self.manager.iter_all().any(|e| e.id == id && e.active) {
                continue;
            }
            let pairs = resolve_native_captor_active_contacts(
                &mut Intro2ContactFrame {
                    entities: &mut self.manager,
                    resources: &mut self.session.cache,
                    world_fx: &mut self.fx,
                    static_damage: &mut self.static_damage,
                    notifications: &mut self.notifications,
                    retail_tick: t,
                    actor_tasks: &mut self.tasks,
                },
                id,
                CaptureFeedbackPolicy::Gameplay,
            );
            if let NativeCaptorPairOutcome::Blocked { reason, .. } = &pairs {
                let subject = entity(&self.manager, id).entity_type;
                let text = format!("{reason:?}");
                let named_type53 = self
                    .manager
                    .iter_all()
                    .any(|e| e.entity_type == 53 && text.contains(&format!("entity_id: {}", e.id)));
                assert!(
                    subject != 53 && !named_type53,
                    "tick{t} subject{id}: {pairs:?} {:?}",
                    self.manager
                        .iter_all()
                        .filter(|e| text.contains(&format!("entity_id: {} ", e.id)))
                        .map(|e| (e.id, e.entity_type))
                        .collect::<Vec<_>>()
                );
            }
        }
    }
}

/// Runs the world, people placed beside the captors, until C910 attaches one.
/// (Moving a pursued peasant would part its retained outer tail.)
fn capture_a_person(level: u32) -> (World, u32, u32, u32) {
    let (session, mut manager, fx) = fixture(level);
    let captors: Vec<u32> = manager
        .iter_all()
        .filter(|e| e.entity_type == 53)
        .map(|e| e.id)
        .collect();
    activate(&mut manager, &captors);
    let people: Vec<u32> = manager
        .iter_all()
        .filter(|e| e.active && e.capability_flags & 0x0c00 != 0)
        .map(|e| e.id)
        .collect();
    assert!(!people.is_empty() && !captors.is_empty(), "world{level}");
    // People beside the captors, so People Nearby can weigh Capture9.
    for (index, &person) in people.iter().enumerate() {
        let [x, y, z] = entity(&manager, captors[index % captors.len()]).position_raw();
        manager.entity_mut(person).unwrap().set_position_raw([
            x.wrapping_add(0x180),
            y,
            z.wrapping_add(0x180),
        ]);
    }
    let mut tasks = adopt_world(&mut manager);
    // The peasants' selected owners, as a production load adopts them.
    tasks
        .adopt_fresh_level1_type9_selected(&mut manager)
        .unwrap();
    let mut world = World {
        session,
        manager,
        fx,
        tasks,
        static_damage: StaticDamageScheduler::new(),
        notifications: GameplayNotifications::new(),
    };
    for t in 1..=6000u32 {
        world.step(t);
        for &captor in &captors {
            let relation = world
                .manager
                .iter_all()
                .find(|e| e.id == captor && e.active)
                .and_then(|e| e.native_capture_relation);
            if let Some(relation) = relation {
                let person = relation.child.entity_id;
                assert_eq!(entity(&world.manager, person).attached_to, Some(captor));
                assert!(carrying_variant(entity(&world.manager, captor)).is_some());
                return (world, captor, person, t);
            }
        }
    }
    panic!("world{level}: no Type53 ever attached a person");
}

/// C910 attaches a pursued person to a Type53's J row; the carrying variants
/// run on Type53's mover, and 18640 keeps the person at the captor.
#[v2k_test_support::retail_test]
fn a_type53_captures_and_carries_its_pursued_person() {
    let (mut world, captor, person, at) = capture_a_person(15);
    let mut variants = std::collections::BTreeSet::new();
    for t in at + 1..=at + 500 {
        world.step(t);
        let held = entity(&world.manager, captor).native_capture_relation;
        if held.is_some_and(|relation| relation.child.entity_id == person) {
            variants.extend(carrying_variant(entity(&world.manager, captor)));
            // 18640 places the person at the slot [0,0,120] turned by the
            // captor's basis, before the captor's own master motion.
            let [a, b] = [captor, person].map(|id| entity(&world.manager, id).position_raw());
            let r2: i32 = (0..3)
                .map(|i| i32::from(a[i].wrapping_sub(b[i])).pow(2))
                .sum();
            assert!((60 * 60..=180 * 180).contains(&r2), "tick{t}: {a:?} {b:?}");
        }
    }
    assert!(!variants.is_empty());
}

/// A Type53 killed while carrying: DB80's D040 pops the person, released
/// alive, then class12 replaces the captor's graph.
#[v2k_test_support::retail_test]
fn a_type53_killed_while_carrying_lets_its_captive_go() {
    use crate::shared_actor_impact::{
        apply_shared_actor_particle_hit, SharedActorImpactFrame, SharedActorImpactOutcome,
    };
    let (mut world, captor, person, at) = capture_a_person(15);
    let outcome = apply_shared_actor_particle_hit(
        SharedActorImpactFrame {
            resources: &world.session.cache,
            entities: &mut world.manager,
            world_fx: &mut world.fx,
            scheduler: &mut world.tasks,
            notifications: &mut world.notifications,
            retail_tick: at + 1,
        },
        crate::native_type28::tests::lethal_hit(captor),
    )
    .unwrap();
    let SharedActorImpactOutcome::Type53(impact::Intro2Type53ImpactOutcome::Applied(applied)) =
        outcome
    else {
        panic!("{outcome:?}")
    };
    assert!(applied.death_publication.is_some());
    let dead = entity(&world.manager, captor);
    assert_eq!(dead.collision.health_raw, RetailRuntimeValue::Known(0));
    assert!(dead.native_capture_relation.is_none());
    assert_eq!(class(&world.manager, captor), 12);
    let released = entity(&world.manager, person);
    assert_eq!(released.attached_to, None);
    assert_ne!(released.collision.health_raw, RetailRuntimeValue::Known(0));
    for t in at + 2..=at + 250 {
        world.step(t);
    }
}
