//! Native Type111 owns sound100 independently of campaign route admission.

use super::*;
use crate::{
    entity::{AuthoredWorldConstruction, EntityConstructionResources},
    entity_collision_state::EntityTypeRuntimeMetadata,
    session::GameSession,
    world_fx::WorldFx,
};

fn native_gates() -> (GameSession, EntityManager, WorldFx) {
    let data = v2k_test_support::retail_dir();
    assert!(data.join("PRELOAD.DAT").is_file(), "retail corpus required");
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(13, 1).unwrap();
    let metadata = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, _)| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect::<Vec<_>>();
    let mut fx = WorldFx::new();
    let mut entities = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            logical_world_index: 1,
            level: session.cache.level_desc().unwrap(),
            type_metadata: &metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            player_arrival: None,
            retail_tick: 0,
        },
        &mut fx,
    )
    .unwrap();
    entities.cleanup_pending_actor_deferred_destroys();
    (session, entities, fx)
}

fn gate_sources(entities: &EntityManager) -> Vec<EntityPositionalAudioSource> {
    admitted_sources(entities)
        .into_iter()
        .filter(|source| {
            matches!(
                source.policy,
                EntityPositionalAudioPolicy::GateHelper { .. }
            )
        })
        .collect()
}

fn mix(
    state: &mut OrderedEntityLoops<u32>,
    backend: &mut FakeBackend,
    entities: &EntityManager,
    pool: &SoundPool<'_>,
    listener_position: [i16; 3],
    fx: &mut WorldFx,
) -> usize {
    let mut draws = 0;
    state
        .reconcile(
            backend,
            &gate_sources(entities),
            LogicalAudioFrame {
                pool,
                listener: PositionalSoundListener {
                    position_raw: listener_position,
                    ..listener()
                },
                elapsed_micros: 100_000,
            },
            &mut || {
                draws += 1;
                fx.next_shared_retail_random_u16()
            },
        )
        .unwrap();
    draws
}

#[v2k_test_support::retail_test]
fn native_type111_sound100_follows_suspends_resumes_and_releases_its_pcm38_voice() {
    let (session, mut entities, mut fx) = native_gates();
    let mut expected_rng = fx.fork_for_main_base_abort_transaction();
    let sources = gate_sources(&entities);
    assert_eq!(sources.len(), 1, "first-world Hive cleanup leaves one gate");
    let source = sources[0];
    let id = source.entity_id;
    let pool = SoundPool::from_tables(&session.cache.global_sound_tables());
    let alias = pool.resolve(100).unwrap();
    assert_eq!(alias.pcm_global_id, 38);
    assert!(
        !alias.alias_hops.is_empty(),
        "sound100 is an alias, not direct PCM"
    );
    //4954EC consumes one word per alias hop even if its variance is zero.
    let admission_draws = alias.alias_hops.len();
    let mut state = OrderedEntityLoops::default();
    let mut backend = FakeBackend::default();
    assert_eq!(
        mix(
            &mut state,
            &mut backend,
            &entities,
            &pool,
            source.position_raw,
            &mut fx,
        ),
        admission_draws
    );
    for _ in 0..admission_draws {
        expected_rng.next_shared_retail_random_u16();
    }
    assert!(matches!(
        backend.events[0],
        FakeEvent::Play {
            handle: 1,
            sound_id: 38,
            ..
        }
    ));
    assert_eq!(state.rows.len(), 1);
    assert!(state.rows[0].warble.is_none());

    let moved = [
        source.position_raw[0].wrapping_add(256),
        source.position_raw[1],
        source.position_raw[2],
    ];
    entities.entity_mut(id).unwrap().set_position_raw(moved);
    assert_eq!(
        gate_sources(&entities)[0].position_raw,
        source.position_raw,
        "the retained C830 row moves only when44C920 publishes"
    );
    publish_constructor_sound_follow_position(entities.entity_mut(id).unwrap());
    assert_eq!(gate_sources(&entities)[0].position_raw, moved);
    assert_eq!(
        mix(&mut state, &mut backend, &entities, &pool, moved, &mut fx),
        0,
        "4957C0 updates the retained voice without fresh alias resolution or warble RNG"
    );
    assert!(matches!(
        backend.events.last(),
        Some(FakeEvent::Update {
            handle: 1,
            sound_id: 38,
            ..
        })
    ));
    let retained = state.rows[0].source;

    state.suspend_physical_voices(&mut backend);
    state.suspend_physical_voices(&mut backend);
    assert_eq!(state.rows.len(), 1);
    assert_eq!(state.rows[0].source, retained);
    assert!(state.rows[0].physical.is_none());
    assert!(state.rows[0].admitted_voice.is_none());
    assert_eq!(
        backend
            .events
            .iter()
            .filter(|event| **event == FakeEvent::Stop(1))
            .count(),
        1
    );
    assert_eq!(
        mix(&mut state, &mut backend, &entities, &pool, moved, &mut fx),
        admission_draws,
        "recreating the destroyed physical buffer traverses aliases again"
    );
    for _ in 0..admission_draws {
        expected_rng.next_shared_retail_random_u16();
    }
    assert!(matches!(
        backend.events.last(),
        Some(FakeEvent::Play {
            handle: 2,
            sound_id: 38,
            ..
        })
    ));

    entities
        .entity_mut(id)
        .unwrap()
        .mark_actor_deferred_destroy_pending();
    entities.queue_actor_deferred_destroy(id);
    assert_eq!(entities.cleanup_pending_actor_deferred_destroys(), vec![id]);
    assert_eq!(
        mix(&mut state, &mut backend, &entities, &pool, moved, &mut fx),
        0
    );
    assert!(state.rows.is_empty());
    assert_eq!(backend.events.last(), Some(&FakeEvent::Stop(2)));
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected_rng.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn type111_sound100_rejects_an_unreceipted_copy_and_wrong_attachment() {
    let (_, mut native, _) = native_gates();
    let source = gate_sources(&native)[0];
    let mut fake = entity(
        source.entity_id,
        111,
        16,
        RetailRuntimeValue::Known(Some(100)),
        source.position_raw,
    );
    publish_constructor_sound_follow_position(&mut fake);
    assert!(
        gate_sources(&manager(vec![fake])).is_empty(),
        "type/model/sound are not allocation custody"
    );
    native
        .entity_mut(source.entity_id)
        .unwrap()
        .collision
        .constructor_sound_attachment_id_at_0x8c = RetailRuntimeValue::Known(Some(11));
    assert!(gate_sources(&native).is_empty());
}
