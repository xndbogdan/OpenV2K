//! Real factory Sub-M configuration, durable retunes and physical-loop custody.
use super::*;
use crate::{
    entity::{BaseFactoryRuntimeState, EntityConstructionResources},
    entity_collision_state::EntityTypeRuntimeMetadata,
    entity_scheduler::{
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT, SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    },
    factory_production::{
        FactoryProductionPhase, FactoryProductionRuntime, FactorySection13Config,
    },
    gameplay_notifications::GameplayNotifications,
    intro2_type66::{
        publish_working_factory, tick_intro2_type66_owner, Intro2Type66Frame, Intro2Type66Outcome,
        Intro2Type66Owner,
    },
    session::GameSession,
    static_damage::StaticDamageScheduler,
    world_fx::WorldFx,
};
use std::collections::BTreeMap;

fn native_world(world: u32, replacement_sounds: Option<[u32; 2]>) -> (GameSession, EntityManager) {
    let root = v2k_test_support::retail_dir();
    assert!(
        root.join("PRELOAD.DAT").exists(),
        "canonical retail corpus required"
    );
    let mut session = GameSession::init(&root).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(world, 1).unwrap();
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
    let level = session.cache.level_desc().unwrap();
    let mut spawns = level.entities.clone();
    if let Some(sounds) = replacement_sounds {
        for spawn in spawns.iter_mut().filter(|spawn| spawn.entity_type == 66) {
            let config = spawn.config.as_mut().unwrap();
            config[0x50..0x54].copy_from_slice(&sounds[0].to_le_bytes());
            config[0x54..0x58].copy_from_slice(&sounds[1].to_le_bytes());
        }
    }
    let mut manager = EntityManager::from_level_with_type_metadata(
        &level,
        &metadata,
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects()),
    );
    for spawn in spawns.iter().filter(|spawn| spawn.entity_type == 66) {
        let id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(spawn.index))
            .unwrap()
            .id;
        let allocation = manager.main_base_abort_actor_observation(id).unwrap().lease;
        if replacement_sounds.is_some() {
            // The controlled template changes only the two selectors. Match
            // the generic18A90 component prefix before native publication.
            factory_mut(&mut manager, id).production =
                Some(FactoryProductionRuntime::from_retail_template(
                    FactorySection13Config::decode(spawn.config.as_ref().unwrap()),
                    metadata[66].initial_health_raw.unwrap(),
                ));
        }
        publish_working_factory(
            manager.entity_mut(id).unwrap(),
            allocation,
            &metadata[66],
            spawn,
            session.cache.terrain().unwrap(),
            &mut || 0x4567,
        )
        .unwrap();
    }
    (session, manager)
}

fn factory(manager: &EntityManager, id: u32) -> BaseFactoryRuntimeState {
    match manager
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap()
        .base_factory_runtime
    {
        RetailRuntimeValue::Known(Some(state)) => state,
        _ => panic!("native factory component"),
    }
}

fn factory_mut(manager: &mut EntityManager, id: u32) -> &mut BaseFactoryRuntimeState {
    match &mut manager.entity_mut(id).unwrap().base_factory_runtime {
        RetailRuntimeValue::Known(Some(state)) => state,
        _ => panic!("native factory component"),
    }
}

fn factory_sources(manager: &EntityManager) -> Vec<EntityPositionalAudioSource> {
    admitted_sources(manager)
        .into_iter()
        .filter(|source| matches!(source.policy, EntityPositionalAudioPolicy::Factory { .. }))
        .collect()
}

fn mix(
    state: &mut OrderedEntityLoops<u32>,
    backend: &mut FakeBackend,
    manager: &EntityManager,
    position_raw: [i16; 3],
) {
    let table = direct_sound_table();
    let pool = SoundPool::from_tables(&[&table]);
    state
        .reconcile(
            backend,
            &factory_sources(manager),
            LogicalAudioFrame {
                pool: &pool,
                listener: PositionalSoundListener {
                    position_raw,
                    ..listener()
                },
                elapsed_micros: 20_000,
            },
            &mut || panic!("direct factory samples consume no random words"),
        )
        .unwrap();
}

fn step(
    session: &mut GameSession,
    manager: &mut EntityManager,
    id: u32,
    owner: Intro2Type66Owner,
    tick: u32,
) -> Intro2Type66Owner {
    let world_style_raw = session.cache.level_desc().unwrap().world_style;
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.state_flags_at_0x08.overwrite(
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    );
    entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    let result = tick_intro2_type66_owner(
        manager,
        owner,
        Intro2Type66Frame {
            resources: &mut session.cache,
            world_fx: &mut WorldFx::new(),
            notifications: &mut GameplayNotifications::new(),
            static_damage: &mut StaticDamageScheduler::new(),
            elapsed_micros: 20_000,
            retail_tick: tick,
            world_style_raw,
            main_base_abort_active: false,
        },
    );
    assert!(
        matches!(result.outcome, Intro2Type66Outcome::Advanced { .. }),
        "{:?}",
        result.outcome
    );
    result.retained_owner.unwrap()
}

#[v2k_test_support::retail_test]
fn all_81_authored_factories_admit_only_their_configured_component_voices() {
    let mut count = 0;
    let mut selectors = BTreeMap::new();
    let mut grounded_differently = 0;
    for world in 13..=49 {
        let (session, manager) = native_world(world, None);
        let sources = factory_sources(&manager);
        for entity in manager.iter_all().filter(|entity| entity.entity_type == 66) {
            count += 1;
            let spawn =
                &session.cache.level_desc().unwrap().entities[entity.authored_spawn_index.unwrap()];
            let config = FactorySection13Config::decode(spawn.config.as_ref().unwrap());
            let sounds = [
                config.primary_voice_sound_id(),
                config.secondary_voice_sound_id(),
            ];
            *selectors.entry(sounds).or_insert(0_usize) += 1;
            let own: Vec<_> = sources
                .iter()
                .filter(|source| source.entity_id == entity.id)
                .collect();
            assert_eq!(
                own.iter()
                    .map(|source| source.policy.sound_id() as u32)
                    .collect::<Vec<_>>(),
                sounds
                    .into_iter()
                    .filter(|sound| *sound != 0)
                    .collect::<Vec<_>>(),
                "world{world}/spawn{}",
                spawn.index
            );
            assert_eq!(
                entity.collision.constructor_sound_attachment_id_at_0x8c,
                RetailRuntimeValue::Known(None)
            );
            for source in own {
                assert_eq!(source.position_raw, spawn.position_raw());
                assert_eq!((source.gain_raw_16_16, source.rate_raw_16_16), (0, 0x10000));
            }
            grounded_differently += usize::from(spawn.position_raw() != entity.position_raw());
        }
    }
    assert_eq!(count, 81);
    assert!(
        grounded_differently > 0,
        "constructor emitter and grounded body are distinct"
    );
    eprintln!("factory voice selectors: {selectors:?}");
}

#[v2k_test_support::retail_test]
fn native_factory_production_repair_and_recovery_retunes_survive_completed_prefixes() {
    let (mut session, mut manager) = native_world(13, None);
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 66)
        .unwrap()
        .id;
    let emitter = factory_sources(&manager)[0].position_raw;
    let mut state = OrderedEntityLoops::default();
    let mut backend = FakeBackend::default();
    mix(&mut state, &mut backend, &manager, emitter);
    assert!(
        backend.events.is_empty(),
        "constructor allocates an initially silent logical voice"
    );
    assert_eq!(state.rows.len(), 1);
    factory_mut(&mut manager, id)
        .production
        .as_mut()
        .unwrap()
        .adjust_staffing(2);
    let mut owner = Intro2Type66Owner::adopt(&manager, id).unwrap();
    owner = step(&mut session, &mut manager, id, owner, 100);
    assert_eq!(
        factory(&manager, id)
            .live_owner
            .unwrap()
            .animation_state_raw,
        1
    );
    mix(&mut state, &mut backend, &manager, emitter);
    assert!(
        matches!(backend.events.as_slice(), [FakeEvent::Play { sound_id: 31, params, .. }] if params.rate == 1.0 && params.gain == 1.0)
    );
    let handle = state.rows[0].physical.unwrap();

    // A small real health loss preserves both scientists and enters19010's
    // repair branch. Its C8E0 retune must survive subsequent status snapshots.
    manager.entity_mut(id).unwrap().collision.health_raw = RetailRuntimeValue::Known(98_999);
    owner = step(&mut session, &mut manager, id, owner, 101);
    let repair = factory(&manager, id);
    assert_eq!(repair.live_owner.unwrap().animation_state_raw, 5);
    assert_eq!(
        repair
            .production
            .unwrap()
            .primary_voice
            .unwrap()
            .rate_raw_16_16,
        0x18000
    );
    backend.events.clear();
    mix(&mut state, &mut backend, &manager, emitter);
    assert!(
        matches!(backend.events.as_slice(), [FakeEvent::Update { handle: updated, sound_id: 31, params }] if *updated == handle && params.rate == 1.5 && params.gain == 1.0)
    );

    manager.entity_mut(id).unwrap().collision.health_raw = RetailRuntimeValue::Known(99_999);
    owner = step(&mut session, &mut manager, id, owner, 102);
    assert_eq!(
        factory(&manager, id)
            .production
            .unwrap()
            .primary_voice
            .unwrap()
            .rate_raw_16_16,
        0x10000
    );
    backend.events.clear();
    mix(&mut state, &mut backend, &manager, emitter);
    assert!(
        matches!(backend.events.as_slice(), [FakeEvent::Update { handle: updated, params, .. }] if *updated == handle && params.rate == 1.0)
    );
    let production = factory_mut(&mut manager, id).production.as_mut().unwrap();
    production.production_progress_micros_raw = production.production_threshold_micros_raw - 20_000;
    owner = step(&mut session, &mut manager, id, owner, 103);
    let delivering = factory(&manager, id);
    assert_eq!(
        delivering.production.unwrap().phase,
        FactoryProductionPhase::Delivering
    );
    assert_eq!(delivering.live_owner.unwrap().animation_state_raw, 3);
    assert_eq!(
        delivering
            .production
            .unwrap()
            .primary_voice
            .unwrap()
            .gain_raw_16_16,
        0x10000
    );
    backend.events.clear();
    mix(&mut state, &mut backend, &manager, emitter);
    assert!(
        matches!(backend.events.as_slice(), [FakeEvent::Update { handle: updated, params, .. }] if *updated == handle && params.rate == 1.0)
    );
    assert_eq!(owner.entity_id(), id);
}

#[v2k_test_support::retail_test]
fn factory_channels_keep_same_pcm_independent_and_release_secondary_first() {
    // Controlled source template proves neither31 nor unique PCM is policy.
    let (_, mut manager) = native_world(13, Some([44, 44]));
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 66)
        .unwrap()
        .id;
    let emitter = factory_sources(&manager)[0].position_raw;
    factory_mut(&mut manager, id)
        .production
        .as_mut()
        .unwrap()
        .retune_voices_from_animation_state(3, 0);
    let mut state = OrderedEntityLoops::default();
    let mut backend = FakeBackend::default();
    mix(&mut state, &mut backend, &manager, emitter);
    assert_eq!(state.rows.len(), 2);
    assert!(backend
        .events
        .iter()
        .all(|event| matches!(event, FakeEvent::Play { sound_id: 44, .. })));
    assert_eq!(backend.active.len(), 2);
    let primary = state.rows[0].physical.unwrap();
    let secondary = state.rows[1].physical.unwrap();
    backend.events.clear();
    factory_mut(&mut manager, id)
        .production
        .as_mut()
        .unwrap()
        .release_voices();
    mix(&mut state, &mut backend, &manager, emitter);
    assert_eq!(
        backend.events,
        [FakeEvent::Stop(secondary), FakeEvent::Stop(primary)]
    );
    assert!(state.rows.is_empty());
}

#[v2k_test_support::retail_test]
fn factory_voice_lease_rejects_foreign_receipt_and_never_reuses_an_equal_id_handle() {
    let (_, mut first) = native_world(13, None);
    let (_, mut second) = native_world(13, None);
    let id = first
        .iter_all()
        .find(|entity| entity.entity_type == 66)
        .unwrap()
        .id;
    let emitter = factory_sources(&first)[0].position_raw;
    for manager in [&mut first, &mut second] {
        factory_mut(manager, id)
            .production
            .as_mut()
            .unwrap()
            .retune_voices_from_animation_state(1, 0);
    }
    let mut state = OrderedEntityLoops::default();
    let mut backend = FakeBackend::default();
    mix(&mut state, &mut backend, &first, emitter);
    let old_handle = state.rows[0].physical.unwrap();
    backend.events.clear();
    mix(&mut state, &mut backend, &second, emitter);
    assert_ne!(state.rows[0].physical, Some(old_handle));
    assert!(backend.events.contains(&FakeEvent::Stop(old_handle)));
    let foreign = first
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap()
        .intro2_type66_runtime;
    second.entity_mut(id).unwrap().intro2_type66_runtime = foreign;
    backend.events.clear();
    mix(&mut state, &mut backend, &second, emitter);
    assert!(state.rows.is_empty());
    assert!(matches!(backend.events.as_slice(), [FakeEvent::Stop(_)]));
}

#[v2k_test_support::retail_test]
fn silent_factory_row_survives_culling_and_never_follows_the_grounded_body() {
    let (_, mut manager) = native_world(13, None);
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 66)
        .unwrap()
        .id;
    let emitter = factory_sources(&manager)[0].position_raw;
    factory_mut(&mut manager, id)
        .production
        .as_mut()
        .unwrap()
        .retune_voices_from_animation_state(1, 0);
    let mut state = OrderedEntityLoops::default();
    let mut backend = FakeBackend::default();
    mix(&mut state, &mut backend, &manager, emitter);
    let first = state.rows[0].physical.unwrap();
    backend.events.clear();
    let far = [emitter[0].wrapping_add(0x3000), emitter[1], emitter[2]];
    manager.entity_mut(id).unwrap().set_position_raw(far);
    mix(&mut state, &mut backend, &manager, far);
    assert_eq!(backend.events, [FakeEvent::Stop(first)]);
    assert_eq!(state.rows[0].source.position_raw, emitter);
    assert!(state.rows[0].physical.is_none());
    mix(&mut state, &mut backend, &manager, emitter);
    assert!(state.rows[0].physical.is_some());
    factory_mut(&mut manager, id)
        .production
        .as_mut()
        .unwrap()
        .silence_voices();
    backend.events.clear();
    mix(&mut state, &mut backend, &manager, emitter);
    assert!(matches!(backend.events.as_slice(), [FakeEvent::Stop(_)]));
    assert_eq!(
        state.rows.len(),
        1,
        "19750 silences without freeing the C830 row"
    );
}

#[v2k_test_support::retail_test]
fn map_mask_stops_retained_loops_once_and_resumes_aliases_without_resetting_factory_or_warble() {
    let (_, mut entities) = native_world(13, None);
    let id = entities
        .iter_all()
        .find(|entity| entity.entity_type == 66)
        .unwrap()
        .id;
    factory_mut(&mut entities, id)
        .production
        .as_mut()
        .unwrap()
        .retune_voices_from_animation_state(5, 0);
    let voices_before = factory(&entities, id).production.unwrap();
    let mut sources = factory_sources(&entities);
    let emitter = sources[0].position_raw;
    sources.extend(admitted_sources(&manager(vec![sound11_entity(
        999, 15, emitter, true,
    )])));
    let mut table = direct_sound_table();
    table.entries[11] = AnimSoundEntry {
        index: 11,
        entry_type: EntryType::Alias,
        raw_type: 5,
        offset_or_index: 0,
        size_or_scale: 0x8000,
        param1: 0x10000,
        param2: 0x10000,
    };
    let pool = SoundPool::from_tables(&[&table]);
    let frame = LogicalAudioFrame {
        pool: &pool,
        listener: PositionalSoundListener {
            position_raw: emitter,
            ..listener()
        },
        elapsed_micros: 0,
    };
    let mut state = OrderedEntityLoops::default();
    let mut backend = FakeBackend::default();
    let mut draws = 0;
    state
        .reconcile(&mut backend, &sources, frame, &mut || {
            draws += 1;
            0
        })
        .unwrap();
    assert_eq!(draws, 3, "two initial warble words, then one alias word");
    let old_handles: Vec<_> = state.rows.iter().map(|row| row.physical.unwrap()).collect();
    let logical_before: Vec<_> = state
        .rows
        .iter()
        .map(|row| (row.source, row.warble))
        .collect();
    // An unrelated mixer handle stands for UI/already-playing disposable
    // one-shots, which44CE70 cannot reach through a retained logical row.
    backend.active.insert(999_999);
    backend.events.clear();
    state.suspend_physical_voices(&mut backend);
    state.suspend_physical_voices(&mut backend);
    assert_eq!(draws, 3);
    assert_eq!(
        backend.events,
        old_handles
            .iter()
            .copied()
            .map(FakeEvent::Stop)
            .collect::<Vec<_>>()
    );
    assert_eq!(backend.active, BTreeSet::from([999_999]));
    assert!(state
        .rows
        .iter()
        .all(|row| row.physical.is_none() && row.admitted_voice.is_none()));
    assert_eq!(
        state
            .rows
            .iter()
            .map(|row| (row.source, row.warble))
            .collect::<Vec<_>>(),
        logical_before
    );
    assert_eq!(
        factory(&entities, id).production.unwrap(),
        voices_before,
        "map masking must not issue18F60 silence or18BE0 release"
    );

    backend.events.clear();
    state
        .reconcile(&mut backend, &sources, frame, &mut || {
            draws += 1;
            0
        })
        .unwrap();
    assert_eq!(
        draws, 4,
        "recreated buffer resolves its alias once; warble is retained"
    );
    assert_eq!(state.rows[1].warble, logical_before[1].1);
    assert!(state
        .rows
        .iter()
        .zip(old_handles)
        .all(|(row, old)| row.physical.unwrap() != old));
    assert!(
        matches!(backend.events[0], FakeEvent::Play { sound_id: 31, params, .. } if params.rate == 1.5)
    );
    state
        .reconcile(&mut backend, &sources, frame, &mut || {
            panic!("existing buffers must not reroll aliases")
        })
        .unwrap();
}
