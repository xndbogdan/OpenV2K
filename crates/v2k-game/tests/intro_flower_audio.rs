use std::collections::VecDeque;
use v2k_formats::anim_sound::SoundPool;
use v2k_game::{
    actor_detailed_sound::ActorDetailedSoundPolicy,
    entity::{EntityConstructionResources, EntityManager},
    entity_collision_state::EntityTypeRuntimeMetadata,
    entity_emitters::{AuthoredHiveComponentEffects, AuthoredRadialEmission, ComponentUpdateMode},
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    hive_controller::AuthoredHiveComponentFrame,
    infection_evolution::{InfectionEvolutionEffects, InfectionTailSound},
    session::GameSession,
    world_complete_results::WorldCompleteTally,
};

fn fixture() -> GameSession {
    let root = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&root).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    session
}

#[v2k_test_support::retail_test]
fn flower_scene_pcm37_is_the_hives_low_pitch_visible_actor_cue() {
    let session = fixture();
    let tables = session.cache.global_sound_tables();
    let pool = SoundPool::from_tables(&tables);
    let logical_ids: Vec<_> = pool
        .slots()
        .iter()
        .filter(|slot| pool.resolve(slot.global_id).unwrap().pcm_global_id == 37)
        .map(|slot| slot.global_id)
        .collect();
    assert_eq!(logical_ids, [37, 99]);
    let alias = pool.resolve(99).unwrap();
    assert_eq!(alias.frequency_multiplier, 26214.0 / 65536.0);
    assert_eq!(alias.volume_multiplier, 1.0);
    assert_eq!(alias.alias_hops[0].frequency_variance_16_16, 0);
    assert_eq!(pool.slot(37).unwrap().entry.size_or_scale, 111688);

    let mut authored_pcm37 = Vec::new();
    for type_id in 0..125 {
        let Some(record) = session.cache.global_entity_type(type_id) else {
            continue;
        };
        for offset in (0x94..=0xa2).step_by(2) {
            let word =
                u16::from_le_bytes(record.raw_header[offset..offset + 2].try_into().unwrap());
            if matches!(word, 37 | 99) {
                authored_pcm37.push((type_id, offset, word));
            }
        }
    }
    assert_eq!(authored_pcm37, [(67, 0x94, 99)]);
    let hive =
        ActorDetailedSoundPolicy::from_section12(session.cache.global_entity_type(67).unwrap());
    assert_eq!(hive.full_health_raw, 2000);
    assert_eq!(hive.healthy_sounds, [99, 0]);
    assert_eq!(hive.low_health_sound, 0);
    assert_eq!(hive.periods_raw, [500000, 0]);
    let flower = session.cache.global_entity_type(115).unwrap();
    let no_random = &mut || panic!("silent flower policy must not draw RNG");
    assert_eq!(
        ActorDetailedSoundPolicy::from_section12(flower).plan(1000, true, 125000, no_random),
        None
    );
    assert_eq!(flower.projectile_emitter_descriptor().unwrap().sound_id, 0);
}

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Radial,
    Sound(u16, [i16; 3]),
}

struct Effects {
    words: VecDeque<u16>,
    events: Vec<Event>,
}

impl InfectionEvolutionEffects for Effects {
    fn next_shared_random_u16(&mut self) -> u16 {
        self.words.pop_front().expect("unexpected shared RNG draw")
    }

    fn queue_infection_tail_sound(&mut self, _sound: InfectionTailSound) {
        panic!("cleared terrain cannot make an infection-tail cue");
    }
}

impl AuthoredHiveComponentEffects for Effects {
    fn emit_authored_radial(&mut self, _emission: AuthoredRadialEmission) {
        self.events.push(Event::Radial);
        // Native WorldFx's class-5 allocator consumes these three words.
        for _ in 0..3 {
            self.next_shared_random_u16();
        }
    }

    fn queue_hive_detailed_sound(&mut self, sound_id: u16, position_raw: [i16; 3]) {
        self.events.push(Event::Sound(sound_id, position_raw));
    }
}

fn manager(session: &GameSession) -> (EntityManager, u32) {
    let metadata: Vec<_> = (0..session.cache.global_entity_model_table().len())
        .map(|id| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect();
    let manager = EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects()),
    );
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 67)
        .unwrap()
        .id;
    (manager, id)
}

#[v2k_test_support::retail_test]
fn hive_cue_obeys_detailed_visible_activation_and_inclusive_chance_gates() {
    let session = fixture();
    for (mode, visible, enabled, word, expected) in [
        (ComponentUpdateMode::Detailed, true, true, Some(131), true),
        (ComponentUpdateMode::Detailed, true, true, Some(132), false),
        (ComponentUpdateMode::Coarse, true, true, None, false),
        (ComponentUpdateMode::Detailed, false, true, None, false),
        (ComponentUpdateMode::Detailed, true, false, None, false),
    ] {
        let (mut manager, id) = manager(&session);
        manager.set_authored_behavior_components_enabled(24, enabled);
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(0x800, if visible { 0x800 } else { 0 });
        let position = manager.entity_mut(id).unwrap().position_raw();
        let mut effects = Effects {
            words: word.into_iter().collect(),
            events: Vec::new(),
        };
        manager
            .advance_authored_hive_components(
                AuthoredHiveComponentFrame {
                    elapsed_us: 1000,
                    terrain: session.cache.terrain(),
                    retail_tick: 0,
                    notification_phase: GameplayNotificationPhase::NonGameplay,
                    notifications: &mut GameplayNotifications::new(),
                    world_complete_tally: &mut WorldCompleteTally::default(),
                },
                |_| mode,
                &mut effects,
            )
            .unwrap();
        assert!(effects.words.is_empty());
        assert_eq!(
            effects.events,
            if expected {
                vec![Event::Sound(99, position)]
            } else {
                Vec::new()
            }
        );
    }
}

#[v2k_test_support::retail_test]
fn hive_cue_follows_infection_and_radial_rng_in_the_same_callback() {
    let session = fixture();
    let (mut manager, id) = manager(&session);
    manager.set_authored_behavior_components_enabled(24, true);
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(0x800, 0x800);
    let position = manager.entity_mut(id).unwrap().position_raw();
    let mut terrain = session.cache.terrain().unwrap().clone();
    for cell in &mut terrain.cells {
        cell.terrain_type &= !0x10;
    }
    // 50,001us crosses ten 5,000us infection intervals (two words each),
    // then one 50,000us radial attempt (three words), then the cue gate.
    let mut effects = Effects {
        words: std::iter::repeat_n(0, 23).chain([6557]).collect(),
        events: Vec::new(),
    };
    manager
        .advance_authored_hive_components(
            AuthoredHiveComponentFrame {
                elapsed_us: 50001,
                terrain: Some(&terrain),
                retail_tick: 0,
                notification_phase: GameplayNotificationPhase::NonGameplay,
                notifications: &mut GameplayNotifications::new(),
                world_complete_tally: &mut WorldCompleteTally::default(),
            },
            |_| ComponentUpdateMode::Detailed,
            &mut effects,
        )
        .unwrap();
    assert!(effects.words.is_empty());
    assert_eq!(effects.events, [Event::Radial, Event::Sound(99, position)]);
}
