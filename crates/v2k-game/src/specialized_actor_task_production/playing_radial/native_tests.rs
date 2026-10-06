use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    damage::DamagePacket,
    entity::{AuthoredPlayerArrival, AuthoredWorldConstruction, EntityConstructionResources},
    entity_collision_state::{
        EntityTypeRuntimeMetadata, RetailRuntimeValue, CHECKED_DAMAGE_ENABLED_STATE_BIT,
        DYING_STATE_BIT,
    },
    session::GameSession,
    specialized_actor_task_production::SpecializedActorTaskFamily,
};

struct Fixture {
    session: GameSession,
    manager: EntityManager,
    scheduler: SpecializedActorTaskScheduler,
    fx: WorldFx,
    notifications: GameplayNotifications,
    hull: PlayerHull,
}

fn fixture() -> Fixture {
    let data = v2k_test_support::retail_dir();
    assert!(data.join("PRELOAD.DAT").is_file(), "retail corpus required");
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(25, 1).unwrap();
    let metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(kind, _)| {
            EntityTypeRuntimeMetadata::from_section12(
                session.cache.global_entity_type(kind).unwrap(),
            )
        })
        .collect();
    let mut fx = WorldFx::new();
    let mut manager = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            logical_world_index: 13,
            level: session.cache.level_desc().unwrap(),
            type_metadata: &metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            player_arrival: Some(AuthoredPlayerArrival {
                position_raw: [19_712, -500, 14_848],
                heading_raw: 0x4000,
            }),
            retail_tick: 0,
        },
        &mut fx,
    )
    .unwrap();
    assert!(
        manager.player().is_some(),
        "exercise the actual player-manager admission"
    );
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_intro2_type8(&mut manager);
    scheduler.adopt_intro2_type66(&manager);
    scheduler.adopt_main_base(&manager);
    fx.process_pending();
    fx.take_positional_sounds();
    Fixture {
        session,
        manager,
        scheduler,
        fx,
        notifications: GameplayNotifications::new(),
        hull: PlayerHull::default(),
    }
}

fn template() -> RadialDamageTemplate {
    RadialDamageTemplate {
        inner_radius_raw: 8,
        outer_radius_raw: 16,
        impulse_raw: 0,
        packet: DamagePacket {
            channels: [2, 3],
            amounts_raw: [500, 6000],
        },
        trailing_raw: [0, 0],
    }
}

fn isolate(f: &mut Fixture, ids: &[u32]) {
    let other_ids: Vec<_> = f
        .manager
        .iter_all()
        .map(|entity| entity.id)
        .filter(|id| !ids.contains(id))
        .collect();
    for id in other_ids {
        f.manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(CHECKED_DAMAGE_ENABLED_STATE_BIT, 0);
    }
}

fn deliver(
    f: &mut Fixture,
    origin_raw: [i16; 3],
    template: RadialDamageTemplate,
) -> PlayingRadialOutcome {
    f.scheduler.apply_playing_radial_damage(PlayingRadialFrame {
        resources: &mut f.session.cache,
        static_damage: &mut crate::static_damage::StaticDamageScheduler::new(),
        active_terminal_calls: Vec::new(),
        entities: &mut f.manager,
        player_hull: &mut f.hull,
        extra_lives: crate::entity_collision_state::RetailRuntimeValue::Known(2),
        origin_raw,
        template,
        world_fx: &mut f.fx,
        notifications: &mut f.notifications,
        retail_tick: 17,
    })
}

#[v2k_test_support::retail_test]
fn real_player_manager_radial_publishes_native_death_tasks_and_updates_player_hull() {
    for kind in [6, 8, 66] {
        let mut f = fixture();
        let target = f
            .manager
            .iter_all()
            .find(|entity| entity.entity_type == kind)
            .unwrap()
            .id;
        let player_id = f.manager.player().unwrap().id;
        isolate(&mut f, &[target, player_id]);
        let entity = f.manager.entity_mut(target).unwrap();
        let origin = entity.position_raw();
        // Prior damage is a live health value; birth/graph/basis/config receipts
        // remain untouched. This small real packet crosses the death boundary.
        entity.collision.health_raw = RetailRuntimeValue::Known(1);
        let primary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let before_body = (
            entity.position_raw(),
            entity.physical_body_basis_q31,
            entity.model_slots,
        );
        let player = f.manager.player_mut().unwrap();
        player.set_motion_raw(origin, [0; 3]);
        player.collision.health_raw = RetailRuntimeValue::Known(f.hull.health_raw);
        player.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
        let RetailRuntimeValue::Known(profile) = player.collision.damage_profile else {
            panic!()
        };
        let expected_player_health = f.hull.health_raw - profile.filter(template().packet);
        assert!(expected_player_health > 0 && expected_player_health < f.hull.health_raw);
        let expected_order: Vec<_> = f
            .manager
            .retail_live_order_ids()
            .filter(|id| *id == target || *id == player_id)
            .collect();
        let mut expected_fx = f.fx.fork_for_main_base_abort_transaction();
        expected_fx.next_shared_retail_random_u16(); //25730/257C0 selector orC3A0 Sub-A constructor.
        let outcome = deliver(&mut f, origin, template());
        assert!(outcome.blocked.is_none(), "type{kind}: {outcome:?}");
        assert_eq!(outcome.completed_target_ids, expected_order);
        assert_eq!(outcome.accepted_targets, 2);
        assert_eq!(f.hull.health_raw, expected_player_health);
        assert_eq!(
            f.manager.player().unwrap().collision.health_raw,
            RetailRuntimeValue::Known(expected_player_health)
        );
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            expected_fx.next_shared_retail_random_u16()
        );
        let entity = f.manager.entity_mut(target).unwrap();
        assert_eq!(
            (
                entity.position_raw(),
                entity.physical_body_basis_q31,
                entity.model_slots
            ),
            before_body
        );
        assert_ne!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            Some(primary)
        );
        assert_eq!(entity.actor_tasks.wrapper_flags(primary), None);
        if kind == 8 {
            assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
            assert_eq!(
                entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
                RetailRuntimeValue::Known(DYING_STATE_BIT)
            );
            assert!(matches!(
                entity.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::SharedRetarget(_))
            ));
            assert_eq!(
                f.scheduler.family_for(target),
                Some(SpecializedActorTaskFamily::Intro2Type8)
            );
        } else {
            assert_eq!(
                entity.collision.health_raw,
                RetailRuntimeValue::Known(10_000_000)
            );
            let RetailRuntimeValue::Known(Some(base)) = entity.base_factory_runtime else {
                panic!()
            };
            assert_eq!(base.progressive_death.elapsed_micros_raw, 1);
            assert_eq!(
                f.scheduler.family_for(target),
                Some(if kind == 6 {
                    SpecializedActorTaskFamily::MainBase
                } else {
                    SpecializedActorTaskFamily::Intro2Type66
                })
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn later_native_custody_block_preserves_prior_factory_death_and_rng_once() {
    let mut f = fixture();
    let ids: Vec<_> = f
        .manager
        .iter_all()
        .filter(|entity| entity.entity_type == 66)
        .take(2)
        .map(|entity| entity.id)
        .collect();
    assert_eq!(ids.len(), 2);
    isolate(&mut f, &ids);
    let positions: Vec<_> = ids
        .iter()
        .map(|id| {
            let entity = f.manager.entity_mut(*id).unwrap();
            entity.collision.health_raw = RetailRuntimeValue::Known(1);
            entity.position_raw()
        })
        .collect();
    let before_primary = f
        .manager
        .entity_mut(ids[0])
        .unwrap()
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary);
    f.scheduler
        .owners
        .retain(|owner| owner.entity_id() != ids[1]);
    let origin = std::array::from_fn(|axis| {
        positions[0][axis].wrapping_add(positions[1][axis].wrapping_sub(positions[0][axis]) / 2)
    });
    let mut expected_fx = f.fx.fork_for_main_base_abort_transaction();
    expected_fx.next_shared_retail_random_u16();
    let outcome = deliver(
        &mut f,
        origin,
        RadialDamageTemplate {
            inner_radius_raw: 30_000,
            outer_radius_raw: 32_767,
            ..template()
        },
    );
    assert!(
        matches!(outcome.blocked, Some(PlayingRadialBlock::Native(DynamicRadialLiveBlock {
        target_id, target_prefix_committed: false,
        reason: crate::entity::DynamicRadialLiveBlockReason::NativeActorMutationCustody, ..
    })) if target_id == ids[1]),
        "{outcome:?}"
    );
    assert_eq!(outcome.completed_target_ids, vec![ids[0]]);
    assert_eq!(
        f.manager.entity_mut(ids[0]).unwrap().collision.health_raw,
        RetailRuntimeValue::Known(10_000_000)
    );
    assert_ne!(
        f.manager
            .entity_mut(ids[0])
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary),
        before_primary
    );
    assert_eq!(
        f.manager.entity_mut(ids[1]).unwrap().collision.health_raw,
        RetailRuntimeValue::Known(1)
    );
    assert_eq!(
        f.fx.next_shared_retail_random_u16(),
        expected_fx.next_shared_retail_random_u16()
    );
}
