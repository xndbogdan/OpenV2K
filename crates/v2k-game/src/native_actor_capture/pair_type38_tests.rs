//! A lethal pair delivery runs each Type38-family row's terminal inside the
//! pair callback: class1 for Type38, class63 with the Type61 drop for Type129.
use super::*;
use crate::{
    damage::{DamageDeliveryRecord, DamagePacket},
    gameplay_notifications::GameplayNotifications,
    native_type38::Type38Row,
    player_hull::PlayerHull,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
};

#[v2k_test_support::retail_test]
fn type38_family_lethal_pair_runs_the_row_terminal() {
    for (level, row) in [(41, Type38Row::Type129), (42, Type38Row::Type38)] {
        let (mut session, mut entities, mut fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(level);
        entities.cleanup_pending_actor_deferred_destroys();
        let entity = entities
            .iter_all()
            .find(|entity| entity.entity_type == row.entity_type())
            .unwrap();
        let (target, payload) = (entity.id, entity.auto_pilot_payload_packed);
        let newest = entities.iter_all().map(|entity| entity.id).max().unwrap();
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert!(tasks.adopt_type38_family(&entities) > 0);
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let mut hull = PlayerHull::default();
        let mut committed = false;
        apply_pair_checked_damage(
            &mut Intro2ContactFrame {
                entities: &mut entities,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut static_damage,
                notifications: &mut notifications,
                retail_tick: 600,
                actor_tasks: &mut tasks,
            },
            target,
            DamageDeliveryRecord {
                packet: DamagePacket {
                    channels: [2, 0],
                    amounts_raw: [1_000_000, 0],
                },
                source_entity_type_raw: 34,
                owner_handle: 0,
            },
            Some(PlayingPlayerContact {
                hull: &mut hull,
                extra_lives: RetailRuntimeValue::Known(3),
            }),
            &mut committed,
        )
        .unwrap_or_else(|error| panic!("world{level}: {error:?}"));
        assert!(crate::class49_death::finished_terminal_hit_authenticates(
            &entities, target
        ));
        assert!(tasks.family_for(target).is_none(), "world{level}");
        let drops: Vec<_> = entities
            .iter_all()
            .filter(|entity| entity.id > newest && entity.entity_type == 61)
            .map(|entity| entity.power_up_payload_packed)
            .collect();
        match row {
            Type38Row::Type38 => {
                assert!(drops.is_empty());
                assert!(entities
                    .pending_actor_deferred_destroy_ids()
                    .contains(&target));
            }
            Type38Row::Type129 => assert_eq!(drops, [payload]),
        }
        // The finished corpse stays pairable until 14990 without replaying
        // its terminal.
        apply_pair_checked_damage(
            &mut Intro2ContactFrame {
                entities: &mut entities,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut static_damage,
                notifications: &mut notifications,
                retail_tick: 601,
                actor_tasks: &mut tasks,
            },
            target,
            DamageDeliveryRecord {
                packet: DamagePacket {
                    channels: [2, 0],
                    amounts_raw: [1_000_000, 0],
                },
                source_entity_type_raw: 34,
                owner_handle: 0,
            },
            None,
            &mut committed,
        )
        .unwrap_or_else(|error| panic!("world{level} corpse: {error:?}"));
        assert_eq!(
            entities
                .iter_all()
                .filter(|entity| entity.id > newest && entity.entity_type == 61)
                .count(),
            usize::from(row == Type38Row::Type129)
        );
    }
}
