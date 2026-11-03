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

/// DB80 on a Type128 killed directly while carrying (a radial, static
/// contact, a pair, the abort or an infected hit all enter here).
fn kill_carrier(
    session: &mut GameSession,
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    tasks: &mut SpecializedActorTaskScheduler,
    static_damage: &mut StaticDamageScheduler,
    notifications: &mut GameplayNotifications,
    captor: u32,
    tick: u32,
) -> Vec<u32> {
    let newest = manager.iter_all().map(|entity| entity.id).max().unwrap();
    let mut hull = crate::player_hull::PlayerHull::default();
    let result = crate::class49_terminal::run_class49_standard_death(
        crate::class49_terminal::Class49TerminalFrame {
            entities: manager,
            resources: &mut session.cache,
            world_fx: fx,
            static_damage,
            notifications,
            retail_tick: tick,
            world: crate::class49_terminal::Class49WorldContext::Playing {
                scheduler: tasks,
                player_hull: &mut hull,
                extra_lives: RetailRuntimeValue::Known(3),
                active_terminal_calls: Vec::new(),
            },
        },
        captor,
    );
    assert!(
        result.as_ref().is_ok_and(|result| result.returned_nonzero),
        "{result:?}"
    );
    let carrier = entity(manager, captor);
    assert_eq!(carrier.collision.health_raw, RetailRuntimeValue::Known(0));
    // An empty row's relation stays the bookkeeping of a popped row.
    assert!(carrier
        .native_capture_relation
        .is_none_or(|relation| !relation.row_present));
    assert!(crate::class49_death::finished_terminal_hit_authenticates(
        manager, captor
    ));
    assert_eq!(
        manager
            .pending_actor_deferred_destroy_ids()
            .iter()
            .filter(|&&id| id == captor)
            .count(),
        1,
        "10B70 is idempotent"
    );
    let position = carrier.position_raw();
    let drops: Vec<u32> = manager
        .iter_all()
        .filter(|entity| entity.id > newest && entity.entity_type == 61)
        .map(|entity| {
            assert_eq!(entity.position_raw(), position);
            entity.id
        })
        .collect();
    drops
}

/// Type128 shares the transport. Killed directly while carrying, its D040
/// releases the person and C690 runs BC90 once; DB80's reread runs it again:
/// two Type61 drops.
#[v2k_test_support::retail_test]
fn a_type128_killed_while_carrying_drops_twice() {
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
    ) = capture_a_person(28, Type16Row::Type128);
    assert!(carrying_variant(entity(&manager, captor)).is_some());
    let drops = kill_carrier(
        &mut session,
        &mut manager,
        &mut fx,
        &mut tasks,
        &mut static_damage,
        &mut notifications,
        captor,
        at + 1,
    );
    assert_eq!(drops.len(), 2);
    assert_eq!(entity(&manager, person).attached_to, None);
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
    }
}

/// With its row already compacted by 18640 (the person died in its grip),
/// 18500 returns BE5B0: D040 skips C690 and only DB80's reread runs BC90.
#[v2k_test_support::retail_test]
fn a_type128_carrying_an_empty_row_drops_once() {
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
    ) = capture_a_person(28, Type16Row::Type128);
    let child = manager.entity_mut(person).unwrap();
    child.collision.health_raw = RetailRuntimeValue::Known(0);
    child.collision.state_flags_at_0x08.overwrite(
        crate::entity_collision_state::DYING_STATE_BIT,
        crate::entity_collision_state::DYING_STATE_BIT,
    );
    crate::native_actor_capture::update_carried_pose(&mut manager, captor).unwrap();
    assert!(carrying_variant(entity(&manager, captor)).is_some());
    let drops = kill_carrier(
        &mut session,
        &mut manager,
        &mut fx,
        &mut tasks,
        &mut static_damage,
        &mut notifications,
        captor,
        at + 1,
    );
    assert_eq!(drops.len(), 1);
}

/// The scheduler's Playing pass, which lends a Type128 its world.
fn playing_tick(
    tasks: &mut SpecializedActorTaskScheduler,
    manager: &mut EntityManager,
    session: &mut GameSession,
    fx: &mut WorldFx,
    static_damage: &mut StaticDamageScheduler,
    notifications: &mut GameplayNotifications,
    hull: &mut crate::player_hull::PlayerHull,
    t: u32,
) -> crate::specialized_actor_task_production::SpecializedActorTaskProductionPass {
    tasks.tick(
        manager,
        crate::specialized_actor_task_production::SpecializedActorTaskProductionFrame {
            world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Playing {
                player_hull: hull,
                extra_lives: RetailRuntimeValue::Known(3),
            },
            hive_components: None,
            notification_phase: crate::gameplay_notifications::GameplayNotificationPhase::Playing,
            resources: &mut session.cache,
            world_fx: fx,
            static_damage,
            elapsed_micros: 20_000,
            global_elapsed_micros: t * 20_000,
            retail_tick: t,
            main_base_abort_active: false,
        },
        notifications,
    )
}

/// Hold a Type128 deep under the sea with its 20-second lifetime spent until
/// 162B0's lifecycle runs, and return the Type61s it dropped.
#[allow(clippy::too_many_arguments)]
fn drown_carrier(
    session: &mut GameSession,
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    tasks: &mut SpecializedActorTaskScheduler,
    static_damage: &mut StaticDamageScheduler,
    notifications: &mut GameplayNotifications,
    captor: u32,
    from: u32,
) -> Vec<u32> {
    let newest = manager.iter_all().map(|entity| entity.id).max().unwrap();
    // Flood the carrier's spot 0x400 deep, as a rising sea would.
    let [x, _, z] = entity(manager, captor).position_raw();
    let terrain = session.cache.level_terrain_mut().unwrap();
    let ground = terrain.bilinear_height_raw(x, z);
    terrain.header[0] = i32::from(ground.wrapping_add(0x400)) << 8;
    let depth = [x, ground.wrapping_add(0x200), z];
    let mut hull = crate::player_hull::PlayerHull::default();
    for t in from..from + 50 {
        if entity(manager, captor).collision.health_raw == RetailRuntimeValue::Known(0) {
            break;
        }
        let body = manager.entity_mut(captor).unwrap();
        body.set_position_raw(depth);
        body.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(20_000);
        let pass = playing_tick(
            tasks,
            manager,
            session,
            fx,
            static_damage,
            notifications,
            &mut hull,
            t,
        );
        assert_pass_in(Some(manager), &pass, &format!("tick{t}"));
    }
    let carrier = entity(manager, captor);
    assert_eq!(carrier.collision.health_raw, RetailRuntimeValue::Known(0));
    assert_eq!(class(manager, captor), 63);
    assert!(crate::class49_death::finished_terminal_hit_authenticates(
        manager, captor
    ));
    assert_eq!(
        manager
            .pending_actor_deferred_destroy_ids()
            .iter()
            .filter(|&&id| id == captor)
            .count(),
        1,
        "10B70 is idempotent"
    );
    // The corpse has no owner left: the next pass does not visit it.
    let pass = playing_tick(
        tasks,
        manager,
        session,
        fx,
        static_damage,
        notifications,
        &mut hull,
        from + 50,
    );
    assert!(!pass.outcomes.iter().any(|outcome| matches!(outcome,
        SpecializedActorTaskProductionOutcome::Intro2Type16(outcome)
            if outcome.entity_id() == captor)));
    manager
        .iter_all()
        .filter(|entity| entity.id > newest && entity.entity_type == 61)
        .map(|entity| entity.id)
        .collect()
}

/// A drowning Type128's 162B0 runs 10C10 in its surface step: class63's BC90
/// bursts, blasts and drops one Type61, as its other deaths do.
#[v2k_test_support::retail_test]
fn a_drowning_type128_runs_class63() {
    let (mut session, mut manager, mut fx) = fixture(28);
    let ids = captor_ids(&manager);
    activate(&mut manager, &ids);
    let mut tasks = adopt_world(&mut manager);
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let captor = ids[0];
    assert_eq!(
        type16_row(entity(&manager, captor)),
        Some(Type16Row::Type128)
    );
    let drops = drown_carrier(
        &mut session,
        &mut manager,
        &mut fx,
        &mut tasks,
        &mut static_damage,
        &mut notifications,
        captor,
        1,
    );
    assert_eq!(drops.len(), 1);
}

/// Drowning while carrying enters DB80's D040 as a direct death does: the
/// person is released alive and class63 runs twice.
#[v2k_test_support::retail_test]
fn a_type128_drowning_with_a_captive_drops_twice() {
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
    ) = capture_a_person(28, Type16Row::Type128);
    assert!(carrying_variant(entity(&manager, captor)).is_some());
    let drops = drown_carrier(
        &mut session,
        &mut manager,
        &mut fx,
        &mut tasks,
        &mut static_damage,
        &mut notifications,
        captor,
        at + 1,
    );
    assert_eq!(drops.len(), 2);
    assert_eq!(entity(&manager, person).attached_to, None);
}
