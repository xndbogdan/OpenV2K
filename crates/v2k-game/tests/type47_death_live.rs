//! Live Level-1 Type-47 15040/10C10: surviving hits must be able to go lethal.

use v2k_game::damage::EntityHitEntry;
use v2k_game::damage::{DamageDeliveryRecord, PRIMARY_PROJECTILE_DAMAGE_PACKET};
use v2k_game::entity::{EntityConstructionResources, EntityManager};
use v2k_game::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
use v2k_game::ordinary_type47_live::FRESH_LEVEL1_ORDINARY_TYPE47_SPAWN_INDICES;
use v2k_game::session::GameSession;
use v2k_game::type47_checked_damage::{
    apply_type47_checked_damage_after_c690, Type47CheckedDamageOutcome, Type47CheckedDamageRequest,
};
use v2k_game::type47_common_dying_production::{
    Type47CommonDyingProductionFrame, Type47CommonDyingProductionOutcome,
    Type47CommonDyingScheduler,
};
use v2k_game::type47_impact_live::{apply_type47_impact_c690_live, Type47ImpactLiveRequest};
use v2k_game::world_fx::WorldFx;

fn type_metadata(session: &GameSession) -> Vec<EntityTypeRuntimeMetadata> {
    session
        .cache
        .global_entity_model_table()
        .iter()
        .copied()
        .enumerate()
        .map(|(entity_type, model_slots)| {
            session
                .cache
                .global_entity_type(entity_type)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots,
                    ..EntityTypeRuntimeMetadata::default()
                })
        })
        .collect()
}

#[v2k_test_support::retail_test]
fn live_spawn11_primary_packets_reach_10c10() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("init session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier first-world resources");
    session
        .load_level_by_id(13, 1)
        .expect("load normal-tier Level 1");

    let type_metadata = type_metadata(&session);
    let resources =
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects());
    let mut world_fx = WorldFx::new();
    let mut manager = EntityManager::from_fresh_new_game_level_with_type_metadata(
        session.cache.level_desc().expect("Level-1 Section 13"),
        &type_metadata,
        resources,
        0,
        &mut world_fx,
    )
    .expect("fresh Level-1 construction");

    let ant = manager
        .iter_all()
        .find(|entity| {
            entity.authored_spawn_index == Some(FRESH_LEVEL1_ORDINARY_TYPE47_SPAWN_INDICES[0])
        })
        .expect("spawn 11");
    let ant_id = ant.id;
    eprintln!(
        "spawn 11 birth flags value={:#010x} mask={:#010x}",
        ant.collision.state_flags_at_0x08.known_value_bits(),
        ant.collision.state_flags_at_0x08.known_mask(),
    );

    let mut last = None;
    let mut survived = 0usize;
    let player_handle = manager.player().expect("fresh first-world player").id;
    for shot in 0..8 {
        let _ = apply_type47_impact_c690_live(
            &mut manager,
            ant_id,
            &mut world_fx,
            Type47ImpactLiveRequest {
                entry: EntityHitEntry::PrimaryProjectile,
                retail_tick: shot,
            },
        );
        let outcome = apply_type47_checked_damage_after_c690(
            &mut manager,
            &mut world_fx,
            ant_id,
            Type47CheckedDamageRequest {
                delivery: DamageDeliveryRecord {
                    packet: PRIMARY_PROJECTILE_DAMAGE_PACKET,
                    source_entity_type_raw: 46,
                    owner_handle: player_handle,
                },
                entry: EntityHitEntry::PrimaryProjectile,
                retail_tick: shot,
            },
        );
        let health = manager
            .iter_all()
            .find(|entity| entity.id == ant_id)
            .and_then(|entity| match entity.collision.health_raw {
                RetailRuntimeValue::Known(health) => Some(health),
                _ => None,
            });
        eprintln!("shot {shot}: {outcome:?} health={health:?}");
        match &outcome {
            Type47CheckedDamageOutcome::Survived { .. } => survived += 1,
            Type47CheckedDamageOutcome::Lethal { publication, .. } => {
                let flags = manager
                    .iter_all()
                    .find(|entity| entity.id == ant_id)
                    .unwrap()
                    .collision
                    .state_flags_at_0x08;
                eprintln!(
                    "after 10C10 flags value={:#010x} mask={:#010x} detail={:?}",
                    flags.known_value_bits(),
                    flags.known_mask(),
                    flags.masked(0x0200_0000),
                );
                let publication = publication
                    .as_ref()
                    .expect("lethal 10C10 must publish class-12");
                let mut dying = Type47CommonDyingScheduler::new();
                dying.register(publication.owner);
                let pass = dying.tick(
                    &mut manager,
                    Type47CommonDyingProductionFrame {
                        terrain: session.cache.terrain().expect("Level-1 terrain"),
                        elapsed_micros: 19_500,
                    },
                    &mut || u32::from(world_fx.next_shared_retail_random_u16()),
                );
                eprintln!("common-dying pass: {:?}", pass.outcomes);
                assert!(
                    matches!(
                        pass.outcomes.as_slice(),
                        [Type47CommonDyingProductionOutcome::DeferredDestroyStaged(_)]
                    ),
                    "class-12 must Flip Over And Die after 10C10, not stall at health 0 with class-5 smoke: {:?}",
                    pass.outcomes
                );
                assert_eq!(manager.pending_actor_deferred_destroy_ids(), [ant_id]);
                assert_eq!(manager.cleanup_pending_actor_deferred_destroys(), [ant_id]);
                assert!(manager.iter_all().all(|entity| entity.id != ant_id));
                return;
            }
            other => panic!(
                "spawn 11 15040 stalled before 10C10: {other:?} after {survived} surviving hits"
            ),
        }
        last = Some(outcome);
    }
    panic!("spawn 11 never reached lethal 10C10 after 8 packets, last={last:?}");
}
