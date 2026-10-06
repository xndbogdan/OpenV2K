//! Real Level-1 actors; the controlled blast isolates the inter-visit writer.
//! This is not evidence that an autonomous blast occurs at the windmill.

use super::*;
use crate::damage::DamagePacket;
use crate::entity::{
    DynamicRadialDamageOutcome, DynamicRadialPlayerContext, Entity, EntityConstructionResources,
};
use crate::entity_collision_state::{
    EntityTypeRuntimeMetadata, RetailRuntimeValue, CHECKED_DAMAGE_ENABLED_STATE_BIT,
    PAIR_COLLISION_FIXED_STATE_BIT,
};
use crate::gameplay_notifications::GameplayNotifications;
use crate::session::GameSession;
use crate::specialized_actor_task_production::{
    SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
};
use crate::static_damage::StaticDamageScheduler;
use crate::world_fx::WorldFx;

struct Fixture {
    session: GameSession,
    manager: EntityManager,
    scheduler: SpecializedActorTaskScheduler,
    fx: WorldFx,
    static_damage: StaticDamageScheduler,
    notifications: GameplayNotifications,
    hull: PlayerHull,
    tick: u32,
    peasant_id: u32,
}

impl Fixture {
    fn new() -> Self {
        let data = v2k_test_support::retail_dir();
        assert!(data.join("PRELOAD.DAT").is_file(), "retail corpus required");
        let mut session = GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(13, 1).unwrap();
        let metadata = session
            .cache
            .global_entity_model_table()
            .iter()
            .copied()
            .enumerate()
            .map(|(index, model_slots)| {
                session
                    .cache
                    .global_entity_type(index)
                    .map(EntityTypeRuntimeMetadata::from_section12)
                    .unwrap_or(EntityTypeRuntimeMetadata {
                        model_slots,
                        ..Default::default()
                    })
            })
            .collect::<Vec<_>>();
        let mut fx = WorldFx::new();
        let mut manager = EntityManager::from_fresh_new_game_level_with_type_metadata(
            session.cache.level_desc().unwrap(),
            &metadata,
            EntityConstructionResources::new(
                session.cache.terrain(),
                session.cache.terrain_objects(),
            ),
            0,
            &mut fx,
        )
        .unwrap();
        let peasant = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(15))
            .unwrap();
        assert_eq!((peasant.id, peasant.entity_type), (17, 9));
        let peasant_id = peasant.id;
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(
            scheduler
                .adopt_fresh_level1_type9_selected(&mut manager)
                .unwrap(),
            6
        );
        let mut notifications = GameplayNotifications::new();
        notifications
            .drain_fresh_level1_type9_attract_attention_receipts(&mut manager)
            .unwrap();
        let mut fixture = Self {
            session,
            manager,
            scheduler,
            fx,
            notifications,
            peasant_id,
            static_damage: StaticDamageScheduler::new(),
            hull: PlayerHull::default(),
            tick: 0,
        };
        // Wait for a real completed mover/world tail, not the no-tail newly
        // adopted state. A cold RNG startup can WAIT before its first mover.
        for _ in 0..200 {
            fixture.advance();
            let owner = fixture
                .scheduler
                .owners
                .iter()
                .find(|owner| owner.entity_id() == peasant_id)
                .unwrap();
            if has_completed_tail(owner, &fixture.manager) {
                return fixture;
            }
        }
        panic!("windmill peasant never completed a real actor visit");
    }

    fn entity(&self, id: u32) -> &Entity {
        self.manager
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap()
    }

    fn advance(&mut self) -> Vec<SpecializedActorTaskProductionOutcome> {
        let claims = self.scheduler.actor_animation_claims().collect::<Vec<_>>();
        let pass = self.scheduler.tick(
            &mut self.manager,
            SpecializedActorTaskProductionFrame {
                world:
                    crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut self.session.cache,
                world_fx: &mut self.fx,
                static_damage: &mut self.static_damage,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: self.tick,
                main_base_abort_active: false,
            },
            &mut self.notifications,
        );
        self.manager
            .advance_unclaimed_actor_animations(20_000, &claims);
        self.tick += 1;
        pass.outcomes
    }

    fn origin(&self) -> [i16; 3] {
        let mut origin = self.entity(self.peasant_id).position_raw();
        origin[0] = origin[0].wrapping_sub(8);
        origin
    }

    fn deliver(
        &mut self,
        origin_raw: [i16; 3],
        template: RadialDamageTemplate,
    ) -> PlayingRadialOutcome {
        self.scheduler
            .apply_playing_radial_damage(PlayingRadialFrame {
                resources: &mut self.session.cache,
                static_damage: &mut self.static_damage,
                active_terminal_calls: Vec::new(),
                entities: &mut self.manager,
                player_hull: &mut self.hull,
                extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                origin_raw,
                template,
                world_fx: &mut self.fx,
                notifications: &mut self.notifications,
                retail_tick: self.tick,
            })
    }
}

fn has_completed_tail(owner: &SpecializedActorTaskOwner, manager: &EntityManager) -> bool {
    use crate::ordinary_type9_attract_attention_production::OrdinaryType9AttractAttentionProductionState as A;
    use crate::ordinary_type9_go_to_job_production::OrdinaryType9GoToJobProductionState as G;
    use crate::ordinary_type9_run_away_production::OrdinaryType9RunAwayProductionState as R;
    use crate::ordinary_type9_wander_production::OrdinaryType9WanderProductionState as W;
    match owner {
        SpecializedActorTaskOwner::OrdinaryType9Wander(o) => {
            matches!(o.state(), W::PostBasisTailPending { .. })
                && o.completed_visit_lease(manager).is_some()
        }
        SpecializedActorTaskOwner::OrdinaryType9GoToJob(o) => {
            matches!(o.state(), G::PostBasisTailPending { .. })
                && o.completed_visit_lease(manager).is_some()
        }
        SpecializedActorTaskOwner::OrdinaryType9RunAway(o) => {
            matches!(o.state(), R::PostBasisTailPending { .. })
                && o.completed_visit_lease(manager).is_some()
        }
        SpecializedActorTaskOwner::OrdinaryType9AttractAttention(o) => {
            matches!(o.state(), A::PostBasisTailPending { .. })
                && o.completed_visit_lease(manager).is_some()
        }
        _ => false,
    }
}

fn impulse_template() -> RadialDamageTemplate {
    RadialDamageTemplate {
        inner_radius_raw: 8,
        outer_radius_raw: 16,
        impulse_raw: 128,
        packet: DamagePacket::default(),
        trailing_raw: [0, 0],
    }
}

fn assert_one_applied(outcome: PlayingRadialOutcome) {
    assert!(outcome.blocked.is_none(), "{outcome:?}");
    assert_eq!(outcome.accepted_targets, 1);
    assert_eq!(outcome.completed_target_ids.len(), 1);
}

fn entity_visit_snapshot(entity: &Entity) -> String {
    // Wrapper identities, callback runtimes, elapsed ages and all task slots
    // must survive an external impulse without reinitialization.
    format!(
        "{:?} {:?} {:?} {:?} {:?}",
        entity.actor_tasks,
        entity.current_behavior_context,
        entity.position_raw(),
        entity.rotation_heading_pitch_roll_raw(),
        entity.physical_body_basis_q31
    )
}

#[v2k_test_support::retail_test]
fn legacy_radial_impulse_reproduces_windmill_outer_tail_drop() {
    let mut f = Fixture::new();
    let origin = f.origin();
    let velocity = f.entity(f.peasant_id).velocity_raw();
    let health = f.entity(f.peasant_id).collision.health_raw;
    let tasks = entity_visit_snapshot(f.entity(f.peasant_id));
    let legacy = f.manager.apply_dynamic_radial_damage(
        DynamicRadialPlayerContext::Present(&mut f.hull),
        origin,
        impulse_template(),
    );
    assert!(
        matches!(legacy, DynamicRadialDamageOutcome::Applied(ref applied)
        if applied.accepted_targets == 1),
        "{legacy:?}"
    );
    assert_eq!(
        f.entity(f.peasant_id).velocity_raw(),
        [velocity[0].wrapping_add(128), velocity[1], velocity[2]]
    );
    assert_eq!(f.entity(f.peasant_id).collision.health_raw, health);
    assert_eq!(entity_visit_snapshot(f.entity(f.peasant_id)), tasks);
    let outcomes = f.advance();
    let drop = outcomes
        .iter()
        .find(|outcome| outcome.entity_id() == f.peasant_id)
        .unwrap();
    assert!(
        format!("{drop:?}").contains("OuterTailStateMismatch"),
        "{drop:?}"
    );
    assert!(!f
        .scheduler
        .owners
        .iter()
        .any(|owner| owner.entity_id() == f.peasant_id));
}

#[v2k_test_support::retail_test]
fn playing_radial_repeated_impulse_and_nonlethal_damage_preserve_windmill_movement() {
    let mut f = Fixture::new();
    let origin = f.origin();
    let mut expected_velocity = f.entity(f.peasant_id).velocity_raw();
    let tasks = entity_visit_snapshot(f.entity(f.peasant_id));
    let health = f.entity(f.peasant_id).collision.health_raw;
    // Two static programs can write the same completed actor before its next
    // visit. Both impulses accumulate, while the graph and task ages stay put.
    for _ in 0..2 {
        assert_one_applied(f.deliver(origin, impulse_template()));
        expected_velocity[0] = expected_velocity[0].wrapping_add(128);
        assert_eq!(f.entity(f.peasant_id).velocity_raw(), expected_velocity);
        assert_eq!(f.entity(f.peasant_id).collision.health_raw, health);
        assert_eq!(entity_visit_snapshot(f.entity(f.peasant_id)), tasks);
    }
    let RetailRuntimeValue::Known(profile) = f.entity(f.peasant_id).collision.damage_profile else {
        panic!("native damage profile")
    };
    let mut damaging = impulse_template();
    damaging.packet = [1, 10, 100, 500]
        .into_iter()
        .map(|amount| DamagePacket {
            channels: [2, 0],
            amounts_raw: [amount, 0],
        })
        .find(|packet| {
            let damage = packet.filtered_raw(Some(&profile));
            damage > 0 && damage < 750
        })
        .unwrap();
    let filtered = damaging.packet.filtered_raw(Some(&profile));
    assert_one_applied(f.deliver(origin, damaging));
    assert_eq!(
        f.entity(f.peasant_id).collision.health_raw,
        health.map(|health| health - filtered)
    );
    assert_eq!(entity_visit_snapshot(f.entity(f.peasant_id)), tasks);
    let mut previous = f.entity(f.peasant_id).position_raw();
    let mut distance = 0_u64;
    for _ in 0..500 {
        for outcome in f.advance() {
            if outcome.entity_id() == f.peasant_id {
                let text = format!("{outcome:?}");
                assert!(
                    !text.contains("Blocked") && !text.contains("Dropped"),
                    "{text}"
                );
            }
        }
        assert!(f
            .scheduler
            .owners
            .iter()
            .any(|owner| owner.entity_id() == f.peasant_id));
        let after = f.entity(f.peasant_id).position_raw();
        distance += u64::from(after[0].wrapping_sub(previous[0]).unsigned_abs())
            + u64::from(after[2].wrapping_sub(previous[2]).unsigned_abs());
        previous = after;
    }
    assert!(
        distance > 100,
        "retained actor must continue to move: {distance}"
    );
}

#[v2k_test_support::retail_test]
fn later_missing_selected_owner_retains_the_earlier_completed_radial_prefix() {
    let mut f = Fixture::new();
    let mut ids = f
        .scheduler
        .owners
        .iter()
        .filter(|owner| has_completed_tail(owner, &f.manager))
        .map(SpecializedActorTaskOwner::entity_id)
        .collect::<Vec<_>>();
    for _ in 0..200 {
        if ids.len() >= 2 {
            break;
        }
        f.advance();
        ids = f
            .scheduler
            .owners
            .iter()
            .filter(|owner| has_completed_tail(owner, &f.manager))
            .map(SpecializedActorTaskOwner::entity_id)
            .collect();
    }
    ids.sort_unstable();
    assert!(ids.len() >= 2);
    let pair = [ids[0], ids[1]];
    // Deliberately admit only these two real allocations; no callback or
    // death policy is substituted. Both retain their authored task/body data.
    let others = f
        .manager
        .iter_all()
        .map(|entity| entity.id)
        .filter(|id| !pair.contains(id))
        .collect::<Vec<_>>();
    for id in others {
        f.manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(CHECKED_DAMAGE_ENABLED_STATE_BIT, 0);
    }
    let a = f.entity(pair[0]).position_raw();
    let b = f.entity(pair[1]).position_raw();
    let origin =
        std::array::from_fn(|axis| a[axis].wrapping_add(b[axis].wrapping_sub(a[axis]) / 2));
    let template = RadialDamageTemplate {
        inner_radius_raw: 30_000,
        outer_radius_raw: 32_767,
        ..impulse_template()
    };
    let owner_index = f
        .scheduler
        .owners
        .iter()
        .position(|owner| owner.entity_id() == pair[1])
        .unwrap();
    let missing = f.scheduler.owners.remove(owner_index);
    let before_velocity = pair.map(|id| f.entity(id).velocity_raw());
    let expected_impulse = crate::radial_damage::scale_radial_damage(template, origin, a, true)
        .unwrap()
        .impulse_vector_raw
        .unwrap();
    let result = f.deliver(origin, template);
    assert_eq!(
        result.blocked,
        Some(PlayingRadialBlock::Retained(blocked(pair[1])))
    );
    assert_eq!(result.completed_target_ids, vec![pair[0]]);
    assert_eq!(result.accepted_targets, 1);
    assert_eq!(
        f.entity(pair[0]).velocity_raw(),
        std::array::from_fn(|axis| {
            before_velocity[0][axis].wrapping_add(expected_impulse[axis])
        })
    );
    assert_eq!(f.entity(pair[1]).velocity_raw(), before_velocity[1]);
    // A missing later receipt does not roll back the earlier 14AE0 node.
    // Restore only the deliberately removed owner; do not replay the blast.
    f.scheduler.owners.insert(owner_index, missing);
    for id in pair {
        let owner = f
            .scheduler
            .owners
            .iter()
            .find(|owner| owner.entity_id() == id)
            .unwrap();
        assert!(has_completed_tail(owner, &f.manager));
    }
}

#[v2k_test_support::retail_test]
fn fixed_and_zero_impulse_targets_do_not_require_selected_owner_custody() {
    for fixed in [false, true] {
        let mut f = Fixture::new();
        let origin = f.origin();
        let before = f.entity(f.peasant_id).velocity_raw();
        f.scheduler
            .owners
            .retain(|owner| owner.entity_id() != f.peasant_id);
        if fixed {
            f.manager
                .entity_mut(f.peasant_id)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(
                    PAIR_COLLISION_FIXED_STATE_BIT,
                    PAIR_COLLISION_FIXED_STATE_BIT,
                );
        }
        let template = RadialDamageTemplate {
            impulse_raw: if fixed { 128 } else { 0 },
            // A nonzero but filtered-out damage amount still enters the radial
            // target path when no impulse output is requested.
            packet: DamagePacket {
                channels: [0, 0],
                amounts_raw: [1, 0],
            },
            ..impulse_template()
        };
        assert_one_applied(f.deliver(origin, template));
        assert_eq!(f.entity(f.peasant_id).velocity_raw(), before);
    }
}
