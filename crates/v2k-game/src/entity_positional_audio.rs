//! Ordered lifetime owner for persistent entity and factory-component audio.
//!
//! Retail stores one logical sound record at entity `+0x8C`. Physical mixer
//! voices may disappear outside the positional radius, but that logical row
//! survives until generic release clears the entity field or removes the
//! allocation. Type 61 keeps the fixed sound-44 loop. Types 15, 44, 87, and
//! 108 mix sound 11 from the `FUN_0044C920` pre-integration snapshot and the
//! detached audible-only warble.
//! Factory Sub-M independently owns two C830 rows at component+8C/+90;
//!18F60 retunes their configured samples without restarting audible voices.

use v2k_formats::anim_sound::{SoundPool, SoundResolutionError};
use v2k_render::{
    sound::{
        positional_sound_mix, resolve_sound_playback, PositionalSoundListener,
        ResolvedSoundPlayback, SoundPlaybackMode, SoundPlaybackRequest,
    },
    SoundManager, VoiceHandle,
};

use crate::{
    entity::{Entity, EntityManager},
    entity_collision_state::RetailRuntimeValue,
    main_base_type61_abort::{
        LEVEL_ONE_TYPE61_CONSTRUCTOR_SOUND_ATTACHMENT_ID, LEVEL_ONE_TYPE61_ENTITY_TYPE,
    },
    sound11_warble::Sound11WarbleState,
};

pub const SOUND11_CONSTRUCTOR_ATTACHMENT_ID: u16 = 11;
pub const SOUND11_OWNER_ENTITY_TYPES: [u32; 4] = [15, 44, 87, 108];

pub const TYPE61_FIXED_LOOP_SOUND_ID: usize =
    LEVEL_ONE_TYPE61_CONSTRUCTOR_SOUND_ATTACHMENT_ID as usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntityPositionalAudioPolicy {
    Type61FixedLoop,
    GateHelper {
        allocation: crate::main_base_abort::MainBaseAbortActorLease,
    },
    Sound11Warble,
    Factory {
        allocation: crate::main_base_abort::MainBaseAbortActorLease,
        component_identity: u64,
        channel: FactoryVoiceChannel,
        sound_id: u16,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FactoryVoiceChannel {
    Primary,
    Secondary,
}

impl EntityPositionalAudioPolicy {
    const fn sound_id(self) -> usize {
        match self {
            Self::Type61FixedLoop => TYPE61_FIXED_LOOP_SOUND_ID,
            Self::GateHelper { .. } => 100,
            Self::Sound11Warble => SOUND11_CONSTRUCTOR_ATTACHMENT_ID as usize,
            Self::Factory { sound_id, .. } => sound_id as usize,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EntityPositionalAudioSource {
    entity_id: u32,
    position_raw: [i16; 3],
    policy: EntityPositionalAudioPolicy,
    gain_raw_16_16: u32,
    rate_raw_16_16: u32,
}

#[derive(Debug)]
struct LogicalEntityLoop<Handle> {
    source: EntityPositionalAudioSource,
    physical: Option<Handle>,
    warble: Option<Sound11WarbleState>,
    /// Native logical admission is independent of optional hardware. A mixer
    /// retry reuses the resolved request instead of rerolling alias words.
    admitted_voice: Option<ResolvedSoundPlayback>,
}

#[derive(Debug)]
struct OrderedEntityLoops<Handle> {
    rows: Vec<LogicalEntityLoop<Handle>>,
}

impl<Handle> Default for OrderedEntityLoops<Handle> {
    fn default() -> Self {
        Self { rows: Vec::new() }
    }
}

trait LoopingAudioBackend {
    type Handle: Copy;

    fn play_looping(&mut self, playback: ResolvedSoundPlayback) -> Option<Self::Handle>;
    fn update_looping(&mut self, handle: Self::Handle, playback: ResolvedSoundPlayback) -> bool;
    fn stop(&mut self, handle: Self::Handle);
}

struct SoundManagerBackend<'a>(Option<&'a mut SoundManager>);

impl LoopingAudioBackend for SoundManagerBackend<'_> {
    type Handle = VoiceHandle;

    fn play_looping(&mut self, playback: ResolvedSoundPlayback) -> Option<Self::Handle> {
        self.0.as_mut()?.play_resolved(playback)
    }

    fn update_looping(&mut self, handle: Self::Handle, playback: ResolvedSoundPlayback) -> bool {
        self.0
            .as_mut()
            .is_some_and(|manager| manager.update_resolved(handle, playback))
    }

    fn stop(&mut self, handle: Self::Handle) {
        if let Some(manager) = self.0.as_mut() {
            manager.stop(handle);
        }
    }
}

/// Inputs to the logical 0044C970 pass, including an optional audio sink.
pub struct EntityPositionalAudioFrame<'a, 'pool> {
    pub sound_manager: Option<&'a mut SoundManager>,
    pub sound_pool: &'a SoundPool<'pool>,
    pub listener: PositionalSoundListener,
    pub elapsed_micros: u32,
}

#[derive(Clone, Copy)]
struct LogicalAudioFrame<'a, 'pool> {
    pool: &'a SoundPool<'pool>,
    listener: PositionalSoundListener,
    elapsed_micros: u32,
}

/// Persistent logical owner for constructor-attached and Sub-M sounds.
///
/// Rows retain `EntityManager::iter_all` order. This matters even for fixed
/// loops because the physical voice pool is bounded and retry order is
/// observable. The owner deliberately does not sort by entity id or use map
/// iteration.
#[derive(Debug, Default)]
pub struct EntityPositionalAudio {
    fixed_loops: OrderedEntityLoops<VoiceHandle>,
}

impl EntityPositionalAudio {
    /// `44F3E0 ->44CE70 ->44C810`: entering a masked phase releases retained
    /// physical buffers while keeping logical rows and warble state alive.
    /// The phase owner skips `update` until `44F400` clears the mask. Buffer
    /// recreation then resolves aliases again, as a new `495480` admission.
    pub fn suspend_physical_voices(&mut self, sound_manager: Option<&mut SoundManager>) {
        self.fixed_loops
            .suspend_physical_voices(&mut SoundManagerBackend(sound_manager));
    }

    /// Reconcile the exact currently admitted logical rows, then update their
    /// physical voices through retail's positional transform.
    pub fn update(
        &mut self,
        entities: &EntityManager,
        frame: EntityPositionalAudioFrame<'_, '_>,
        next_random: &mut impl FnMut() -> u16,
    ) -> Result<(), SoundResolutionError> {
        let sources = admitted_sources(entities);
        self.fixed_loops.reconcile(
            &mut SoundManagerBackend(frame.sound_manager),
            &sources,
            LogicalAudioFrame {
                pool: frame.sound_pool,
                listener: frame.listener,
                elapsed_micros: frame.elapsed_micros,
            },
            next_random,
        )
    }

    /// Forget allocation ids and physical handles after a true world/audio
    /// teardown. The caller stops the global mixer first.
    pub fn reset_after_audio_stop(&mut self) {
        *self = Self::default();
    }
}

impl<Handle: Copy> OrderedEntityLoops<Handle> {
    fn suspend_physical_voices(&mut self, backend: &mut impl LoopingAudioBackend<Handle = Handle>) {
        for row in &mut self.rows {
            if let Some(handle) = row.physical.take() {
                backend.stop(handle);
            }
            row.admitted_voice = None;
        }
    }

    fn reconcile(
        &mut self,
        backend: &mut impl LoopingAudioBackend<Handle = Handle>,
        sources: &[EntityPositionalAudioSource],
        frame: LogicalAudioFrame<'_, '_>,
        next_random: &mut impl FnMut() -> u16,
    ) -> Result<(), SoundResolutionError> {
        let mut retained = std::mem::take(&mut self.rows);
        let mut next = Vec::with_capacity(sources.len());

        for &source in sources {
            let mut row = retained
                .iter()
                .position(|row| {
                    row.source.entity_id == source.entity_id && row.source.policy == source.policy
                })
                .map(|index| retained.remove(index))
                .unwrap_or(LogicalEntityLoop {
                    source,
                    physical: None,
                    warble: matches!(source.policy, EntityPositionalAudioPolicy::Sound11Warble)
                        .then(Sound11WarbleState::new),
                    admitted_voice: None,
                });
            row.source = source;

            if let Some(mix) =
                positional_sound_mix(frame.listener, source.position_raw, source.gain_raw_16_16)
            {
                let rate = match row.warble.as_mut() {
                    Some(warble) => {
                        warble.advance_audible(frame.elapsed_micros, next_random) as u32
                    }
                    None => source.rate_raw_16_16,
                };
                let request = SoundPlaybackRequest {
                    global_sound_id: source.policy.sound_id(),
                    frequency_q16: rate,
                    volume_q16: mix.volume_q16,
                    pan: mix.pan,
                    mode: SoundPlaybackMode::Looping,
                };
                let playback = if let Some(admitted) = row.admitted_voice {
                    // 4C970 calls 957C0 with current wrapper parameters on an
                    // existing voice; it does not traverse aliases again.
                    ResolvedSoundPlayback {
                        frequency_q16: rate,
                        volume_q16: mix.volume_q16,
                        pan: mix.pan,
                        ..admitted
                    }
                } else {
                    match resolve_sound_playback(frame.pool, request, next_random) {
                        Ok(playback) => playback,
                        Err(error) => {
                            // Keep the preceding rows and this row's committed
                            // audible warble prefix if the asset is invalid.
                            next.push(row);
                            next.extend(retained);
                            self.rows = next;
                            return Err(error);
                        }
                    }
                };
                row.admitted_voice = Some(playback);
                if !row
                    .physical
                    .is_some_and(|handle| backend.update_looping(handle, playback))
                {
                    row.physical = backend.play_looping(playback);
                }
            } else {
                row.admitted_voice = None;
                if let Some(handle) = row.physical.take() {
                    backend.stop(handle);
                }
            }
            next.push(row);
        }

        //18BE0 frees a factory's secondary handle before its primary. Other
        // constructor rows retain their established live-list release order.
        while !retained.is_empty() {
            let row = retained.remove(0);
            if let EntityPositionalAudioPolicy::Factory {
                allocation,
                component_identity,
                channel: FactoryVoiceChannel::Primary,
                ..
            } = row.source.policy
            {
                if let Some(index) = retained.iter().position(|candidate| {
                    matches!(candidate.source.policy,
                        EntityPositionalAudioPolicy::Factory {
                            allocation: candidate_allocation,
                            component_identity: candidate_component,
                            channel: FactoryVoiceChannel::Secondary,
                            ..
                        } if candidate_allocation == allocation
                            && candidate_component == component_identity)
                }) {
                    if let Some(handle) = retained.remove(index).physical {
                        backend.stop(handle);
                    }
                }
            }
            if let Some(handle) = row.physical {
                backend.stop(handle);
            }
        }
        self.rows = next;
        Ok(())
    }
}

pub fn is_sound11_constructor_owner(entity_type: u32, attachment: Option<u16>) -> bool {
    attachment == Some(SOUND11_CONSTRUCTOR_ATTACHMENT_ID)
        && matches!(entity_type, 15 | 44 | 87 | 108)
}

/// `FUN_0044C920`: copy live `+0x96/+0x98/+0x9A` into the `+0x8C` emitter.
pub fn publish_constructor_sound_follow_position(entity: &mut Entity) {
    let attachment = match entity.collision.constructor_sound_attachment_id_at_0x8c {
        RetailRuntimeValue::Known(attachment) => attachment,
        RetailRuntimeValue::Unresolved => return,
    };
    if !is_sound11_constructor_owner(entity.entity_type, attachment)
        && !(entity.entity_type == 111 && attachment == Some(100))
    {
        return;
    }
    entity.collision.constructor_sound_follow_position_raw =
        RetailRuntimeValue::Known(entity.position_raw());
    entity.collision.constructor_sound_follow_published = true;
}

fn admitted_sources(entities: &EntityManager) -> Vec<EntityPositionalAudioSource> {
    let mut sources = Vec::new();
    for entity in entities.iter_all() {
        if entities.native_gate_helper_allocation_authenticates(entity.id)
            && entity.collision.constructor_sound_attachment_id_at_0x8c
                == RetailRuntimeValue::Known(Some(100))
        {
            if let RetailRuntimeValue::Known(position_raw) =
                entity.collision.constructor_sound_follow_position_raw
            {
                sources.push(EntityPositionalAudioSource {
                    entity_id: entity.id,
                    position_raw,
                    policy: EntityPositionalAudioPolicy::GateHelper {
                        allocation: entities
                            .main_base_abort_actor_observation(entity.id)
                            .expect("authenticated gate")
                            .lease,
                    },
                    gain_raw_16_16: 0x10000,
                    rate_raw_16_16: 0x10000,
                });
            }
        }
        if entity.entity_type == LEVEL_ONE_TYPE61_ENTITY_TYPE
            && entity.collision.constructor_sound_attachment_id_at_0x8c
                == RetailRuntimeValue::Known(Some(LEVEL_ONE_TYPE61_CONSTRUCTOR_SOUND_ATTACHMENT_ID))
        {
            sources.push(EntityPositionalAudioSource {
                entity_id: entity.id,
                position_raw: entity.position_raw(),
                policy: EntityPositionalAudioPolicy::Type61FixedLoop,
                gain_raw_16_16: 0x10000,
                rate_raw_16_16: 0x10000,
            });
        }
        // A component voice is independent of entity+8C and of the
        // player's sound31 fan. Never deduplicate by PCM identity.
        append_factory_sources(entities, entity, &mut sources);
        let RetailRuntimeValue::Known(Some(SOUND11_CONSTRUCTOR_ATTACHMENT_ID)) =
            entity.collision.constructor_sound_attachment_id_at_0x8c
        else {
            continue;
        };
        if !is_sound11_constructor_owner(
            entity.entity_type,
            Some(SOUND11_CONSTRUCTOR_ATTACHMENT_ID),
        ) {
            continue;
        }
        let RetailRuntimeValue::Known(position_raw) =
            entity.collision.constructor_sound_follow_position_raw
        else {
            continue;
        };
        sources.push(EntityPositionalAudioSource {
            entity_id: entity.id,
            position_raw,
            policy: EntityPositionalAudioPolicy::Sound11Warble,
            gain_raw_16_16: 0x10000,
            rate_raw_16_16: 0x10000,
        });
    }
    sources
}

fn append_factory_sources(
    entities: &EntityManager,
    entity: &Entity,
    sources: &mut Vec<EntityPositionalAudioSource>,
) {
    if !crate::intro2_type66::type66_manager_allocation_authenticates(entities, entity.id) {
        return;
    }
    let Some(native) = entity.intro2_type66_runtime else {
        return;
    };
    let RetailRuntimeValue::Known(Some(factory)) = entity.base_factory_runtime else {
        return;
    };
    let (Some(owner), Some(production)) = (factory.live_owner, factory.production) else {
        return;
    };
    for (channel, voice) in [
        (FactoryVoiceChannel::Primary, production.primary_voice),
        (FactoryVoiceChannel::Secondary, production.secondary_voice),
    ] {
        let Some(voice) = voice else { continue };
        sources.push(EntityPositionalAudioSource {
            entity_id: entity.id,
            position_raw: native.voice_position_raw(),
            policy: EntityPositionalAudioPolicy::Factory {
                allocation: native.allocation(),
                component_identity: owner.allocation_identity,
                channel,
                sound_id: voice.sound_id,
            },
            gain_raw_16_16: voice.gain_raw_16_16 as u32,
            rate_raw_16_16: voice.rate_raw_16_16 as u32,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{raw_position_world, Entity, EntityKind};
    use std::collections::BTreeSet;
    use v2k_formats::anim_sound::{AnimSoundEntry, AnimSoundTable, EntryType};
    use v2k_render::LoopingVoiceParams;

    mod factory;
    mod gates;

    fn direct_sound_table() -> AnimSoundTable {
        let mut table = v2k_formats::anim_sound::parse_anim_sound(&[0; 4]).unwrap();
        for index in 0..=TYPE61_FIXED_LOOP_SOUND_ID {
            table.entries.push(AnimSoundEntry {
                index,
                entry_type: EntryType::DataBlob,
                raw_type: 1,
                offset_or_index: 0,
                size_or_scale: 0,
                param1: 0,
                param2: 0,
            });
        }
        table
    }

    fn test_voice_params(playback: ResolvedSoundPlayback) -> LoopingVoiceParams {
        assert_eq!(playback.mode, SoundPlaybackMode::Looping);
        LoopingVoiceParams {
            rate: playback.frequency_q16 as f32 / 65536.0,
            gain: playback.volume_q16 as f32 / 65536.0,
            pan: playback.pan as f32 / 10000.0,
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum FakeEvent {
        Play {
            handle: u32,
            sound_id: usize,
            params: LoopingVoiceParams,
        },
        Update {
            handle: u32,
            sound_id: usize,
            params: LoopingVoiceParams,
        },
        Stop(u32),
    }

    #[derive(Default)]
    struct FakeBackend {
        next_handle: u32,
        active: BTreeSet<u32>,
        events: Vec<FakeEvent>,
    }

    impl LoopingAudioBackend for FakeBackend {
        type Handle = u32;

        fn play_looping(&mut self, playback: ResolvedSoundPlayback) -> Option<Self::Handle> {
            self.next_handle += 1;
            let handle = self.next_handle;
            self.active.insert(handle);
            self.events.push(FakeEvent::Play {
                handle,
                sound_id: playback.pcm_global_id,
                params: test_voice_params(playback),
            });
            Some(handle)
        }

        fn update_looping(
            &mut self,
            handle: Self::Handle,
            playback: ResolvedSoundPlayback,
        ) -> bool {
            self.events.push(FakeEvent::Update {
                handle,
                sound_id: playback.pcm_global_id,
                params: test_voice_params(playback),
            });
            self.active.contains(&handle)
        }

        fn stop(&mut self, handle: Self::Handle) {
            self.active.remove(&handle);
            self.events.push(FakeEvent::Stop(handle));
        }
    }

    fn entity(
        id: u32,
        entity_type: u32,
        model_id: usize,
        attachment: RetailRuntimeValue<Option<u16>>,
        position_raw: [i16; 3],
    ) -> Entity {
        let mut entity =
            Entity::unresolved_port_entity(id, EntityKind::Unknown(entity_type), entity_type);
        entity.model_slots = [Some(model_id); 4];
        entity.model_index = Some(model_id);
        entity.position = raw_position_world(position_raw);
        entity.collision.constructor_sound_attachment_id_at_0x8c = attachment;
        entity
    }

    fn manager(entities: Vec<Entity>) -> EntityManager {
        EntityManager::from_entities_for_test(entities)
    }

    fn listener() -> PositionalSoundListener {
        PositionalSoundListener {
            position_raw: [0; 3],
            world_to_view_q31: [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]],
        }
    }

    fn reconcile(
        state: &mut OrderedEntityLoops<u32>,
        backend: &mut FakeBackend,
        entities: &EntityManager,
    ) {
        let table = direct_sound_table();
        let pool = SoundPool::from_tables(&[&table]);
        state
            .reconcile(
                backend,
                &admitted_sources(entities),
                LogicalAudioFrame {
                    pool: &pool,
                    listener: listener(),
                    elapsed_micros: 0,
                },
                &mut || 0,
            )
            .unwrap();
    }

    #[test]
    fn model_override_and_out_of_order_ids_preserve_live_list_order() {
        let mut overridden = entity(
            90,
            LEVEL_ONE_TYPE61_ENTITY_TYPE,
            138,
            RetailRuntimeValue::Known(Some(TYPE61_FIXED_LOOP_SOUND_ID as u16)),
            [0x0100, 0, 0],
        );
        overridden.active = false;
        overridden.attached_to = Some(0x1234);
        let entities = manager(vec![
            overridden,
            entity(
                4,
                LEVEL_ONE_TYPE61_ENTITY_TYPE,
                82,
                RetailRuntimeValue::Known(Some(TYPE61_FIXED_LOOP_SOUND_ID as u16)),
                [0x0200, 0, 0],
            ),
        ]);
        let mut state = OrderedEntityLoops::default();
        let mut backend = FakeBackend::default();

        reconcile(&mut state, &mut backend, &entities);

        assert_eq!(
            state
                .rows
                .iter()
                .map(|row| row.source.entity_id)
                .collect::<Vec<_>>(),
            [90, 4]
        );
        assert_eq!(
            backend
                .events
                .iter()
                .filter_map(|event| match event {
                    FakeEvent::Play { sound_id, .. } => Some(*sound_id),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            [TYPE61_FIXED_LOOP_SOUND_ID; 2]
        );
    }

    #[test]
    fn admission_rejects_wrong_type_attachment_and_unresolved_custody() {
        let entities = manager(vec![
            entity(
                1,
                60,
                82,
                RetailRuntimeValue::Known(Some(TYPE61_FIXED_LOOP_SOUND_ID as u16)),
                [0; 3],
            ),
            entity(
                2,
                LEVEL_ONE_TYPE61_ENTITY_TYPE,
                82,
                RetailRuntimeValue::Known(Some(11)),
                [0; 3],
            ),
            entity(
                3,
                LEVEL_ONE_TYPE61_ENTITY_TYPE,
                82,
                RetailRuntimeValue::Unresolved,
                [0; 3],
            ),
        ]);
        assert!(admitted_sources(&entities).is_empty());
    }

    #[test]
    fn fixed_loop_uses_one_x_controls_without_an_rng_input() {
        let entities = manager(vec![entity(
            1,
            LEVEL_ONE_TYPE61_ENTITY_TYPE,
            82,
            RetailRuntimeValue::Known(Some(TYPE61_FIXED_LOOP_SOUND_ID as u16)),
            [0x0100, 0, 0],
        )]);
        let mut state = OrderedEntityLoops::default();
        let mut backend = FakeBackend::default();
        reconcile(&mut state, &mut backend, &entities);

        let FakeEvent::Play {
            sound_id, params, ..
        } = backend.events[0]
        else {
            panic!("expected fixed-loop creation");
        };
        assert_eq!(sound_id, TYPE61_FIXED_LOOP_SOUND_ID);
        assert_eq!(params.rate, 1.0);
        assert_eq!(params.gain, 1.0);
        assert_eq!(params.pan, 622.0 / 10000.0);
    }

    #[test]
    fn positional_cull_retains_logical_row_and_reentry_restarts_physical_voice() {
        let near = manager(vec![entity(
            1,
            LEVEL_ONE_TYPE61_ENTITY_TYPE,
            82,
            RetailRuntimeValue::Known(Some(TYPE61_FIXED_LOOP_SOUND_ID as u16)),
            [0x0100, 0, 0],
        )]);
        let far = manager(vec![entity(
            1,
            LEVEL_ONE_TYPE61_ENTITY_TYPE,
            82,
            RetailRuntimeValue::Known(Some(TYPE61_FIXED_LOOP_SOUND_ID as u16)),
            [0x2000, 0, 0],
        )]);
        let mut state = OrderedEntityLoops::default();
        let mut backend = FakeBackend::default();

        reconcile(&mut state, &mut backend, &near);
        reconcile(&mut state, &mut backend, &far);
        assert_eq!(state.rows.len(), 1);
        assert_eq!(state.rows[0].physical, None);
        assert!(backend.events.contains(&FakeEvent::Stop(1)));

        reconcile(&mut state, &mut backend, &near);
        assert_eq!(state.rows[0].physical, Some(2));
    }

    #[test]
    fn release_disappearance_and_world_reset_stop_or_forget_exact_ownership() {
        let live = manager(vec![entity(
            7,
            LEVEL_ONE_TYPE61_ENTITY_TYPE,
            82,
            RetailRuntimeValue::Known(Some(TYPE61_FIXED_LOOP_SOUND_ID as u16)),
            [0; 3],
        )]);
        let released = manager(vec![entity(
            7,
            LEVEL_ONE_TYPE61_ENTITY_TYPE,
            82,
            RetailRuntimeValue::Known(None),
            [0; 3],
        )]);
        let mut state = OrderedEntityLoops::default();
        let mut backend = FakeBackend::default();

        reconcile(&mut state, &mut backend, &live);
        reconcile(&mut state, &mut backend, &released);
        assert!(state.rows.is_empty());
        assert!(backend.events.contains(&FakeEvent::Stop(1)));

        reconcile(&mut state, &mut backend, &live);
        reconcile(&mut state, &mut backend, &manager(Vec::new()));
        assert!(state.rows.is_empty());
        assert!(backend.events.contains(&FakeEvent::Stop(2)));

        reconcile(&mut state, &mut backend, &live);
        state = OrderedEntityLoops::default();
        assert!(
            state.rows.is_empty(),
            "a new world must not inherit entity id 7"
        );
    }

    #[test]
    fn stale_physical_handle_is_recreated_without_losing_logical_order() {
        let entities = manager(vec![entity(
            3,
            LEVEL_ONE_TYPE61_ENTITY_TYPE,
            82,
            RetailRuntimeValue::Known(Some(TYPE61_FIXED_LOOP_SOUND_ID as u16)),
            [0; 3],
        )]);
        let mut state = OrderedEntityLoops::default();
        let mut backend = FakeBackend::default();
        reconcile(&mut state, &mut backend, &entities);
        backend.active.clear();

        reconcile(&mut state, &mut backend, &entities);

        assert_eq!(state.rows[0].physical, Some(2));
        assert!(matches!(
            backend.events[1],
            FakeEvent::Update { handle: 1, .. }
        ));
        assert!(matches!(
            backend.events[2],
            FakeEvent::Play { handle: 2, .. }
        ));
    }

    fn sound11_entity(id: u32, entity_type: u32, position_raw: [i16; 3], snapshot: bool) -> Entity {
        let mut entity = entity(
            id,
            entity_type,
            276,
            RetailRuntimeValue::Known(Some(SOUND11_CONSTRUCTOR_ATTACHMENT_ID)),
            position_raw,
        );
        if snapshot {
            publish_constructor_sound_follow_position(&mut entity);
        }
        entity
    }

    #[test]
    fn sound11_types_need_the_4c920_snapshot() {
        for entity_type in SOUND11_OWNER_ENTITY_TYPES {
            let unpublished = manager(vec![sound11_entity(1, entity_type, [0x0100, 0, 0], false)]);
            assert!(
                admitted_sources(&unpublished).is_empty(),
                "type {entity_type} must fail closed without FUN_0044C920"
            );
            let published = manager(vec![sound11_entity(1, entity_type, [0x0100, 0, 0], true)]);
            assert_eq!(
                admitted_sources(&published)[0].policy,
                EntityPositionalAudioPolicy::Sound11Warble
            );
        }
        let ptersect = manager(vec![sound11_entity(1, 13, [0x0100, 0, 0], true)]);
        assert!(admitted_sources(&ptersect).is_empty());
    }

    #[test]
    fn sound11_mixes_the_snapshot_not_a_later_body() {
        let mut entity = sound11_entity(1, 15, [0x0100, 0, 0], true);
        entity.set_position_raw([0x4000, 0, 0]);
        let entities = manager(vec![entity]);
        assert_eq!(admitted_sources(&entities)[0].position_raw, [0x0100, 0, 0]);
    }

    #[test]
    fn sound11_warble_freezes_when_culled() {
        let near = manager(vec![sound11_entity(1, 15, [0x0100, 0, 0], true)]);
        let far = manager(vec![sound11_entity(1, 15, [0x3000, 0, 0], true)]);
        let mut state = OrderedEntityLoops::default();
        let mut backend = FakeBackend::default();
        let table = direct_sound_table();
        let pool = SoundPool::from_tables(&[&table]);
        let frame = LogicalAudioFrame {
            pool: &pool,
            listener: listener(),
            elapsed_micros: 40_000,
        };
        let mut draws = 0;
        state
            .reconcile(&mut backend, &admitted_sources(&near), frame, &mut || {
                draws += 1;
                0
            })
            .unwrap();
        let audible_draws = draws;
        assert_eq!(audible_draws, 2);
        let audible_warble = state.rows[0].warble;
        state
            .reconcile(&mut backend, &admitted_sources(&far), frame, &mut || {
                draws += 1;
                0
            })
            .unwrap();
        assert_eq!(draws, audible_draws);
        assert_eq!(state.rows[0].warble, audible_warble);
        assert_eq!(state.rows[0].physical, None);
    }

    #[test]
    fn no_device_keeps_flyer_warble_order_and_one_rollover_per_audible_callback() {
        let entities = manager(vec![
            sound11_entity(90, 15, [0; 3], true),
            sound11_entity(4, 87, [0; 3], true),
        ]);
        let table = direct_sound_table();
        let pool = SoundPool::from_tables(&[&table]);
        let mut state = EntityPositionalAudio::default();
        let mut words = [0xFFFF, 0, 0xFFFE, 0xFFFF].into_iter();
        state
            .update(
                &entities,
                EntityPositionalAudioFrame {
                    sound_manager: None,
                    sound_pool: &pool,
                    listener: listener(),
                    elapsed_micros: 0,
                },
                &mut || words.next().expect("two words per audible flyer"),
            )
            .unwrap();
        assert_eq!(words.next(), None);
        assert_eq!(
            state
                .fixed_loops
                .rows
                .iter()
                .map(|row| row.source.entity_id)
                .collect::<Vec<_>>(),
            [90, 4]
        );
        assert!(state
            .fixed_loops
            .rows
            .iter()
            .all(|row| row.physical.is_none() && row.admitted_voice.is_some()));
        state
            .update(
                &entities,
                EntityPositionalAudioFrame {
                    sound_manager: None,
                    sound_pool: &pool,
                    listener: listener(),
                    elapsed_micros: 15_000,
                },
                &mut || panic!("interpolation must not sample RNG"),
            )
            .unwrap();
        let first = state.fixed_loops.rows[0]
            .admitted_voice
            .unwrap()
            .frequency_q16;
        let second = state.fixed_loops.rows[1]
            .admitted_voice
            .unwrap()
            .frequency_q16;
        assert_eq!(first, 0x10FFF);
        assert!(
            second < 0x10000,
            "the second live-list row consumed the downward word"
        );
        let mut draws = 0;
        state
            .update(
                &entities,
                EntityPositionalAudioFrame {
                    sound_manager: None,
                    sound_pool: &pool,
                    listener: listener(),
                    elapsed_micros: 1_000_000,
                },
                &mut || {
                    draws += 1;
                    0
                },
            )
            .unwrap();
        assert_eq!(
            draws, 4,
            "overshoot advances one segment per flyer, without catch-up"
        );
    }

    #[test]
    fn hardware_absence_does_not_reroll_alias_but_cull_reentry_does() {
        let near = manager(vec![sound11_entity(1, 15, [0; 3], true)]);
        let far = manager(vec![sound11_entity(1, 15, [0x2000, 0, 0], true)]);
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
        let mut state = EntityPositionalAudio::default();
        let mut words = [0, 0, 2].into_iter();
        state
            .update(
                &near,
                EntityPositionalAudioFrame {
                    sound_manager: None,
                    sound_pool: &pool,
                    listener: listener(),
                    elapsed_micros: 0,
                },
                &mut || {
                    words
                        .next()
                        .expect("warble words must precede the alias word")
                },
            )
            .unwrap();
        assert_eq!(words.next(), None);
        assert_eq!(
            state.fixed_loops.rows[0]
                .admitted_voice
                .unwrap()
                .frequency_q16,
            65_537
        );
        state
            .update(
                &near,
                EntityPositionalAudioFrame {
                    sound_manager: None,
                    sound_pool: &pool,
                    listener: listener(),
                    elapsed_micros: 0,
                },
                &mut || panic!("an admitted logical voice must not retry alias resolution"),
            )
            .unwrap();
        assert_eq!(
            state.fixed_loops.rows[0]
                .admitted_voice
                .unwrap()
                .frequency_q16,
            65_536,
            "957C0 retunes an existing voice with current wrapper values"
        );
        let warble = state.fixed_loops.rows[0].warble;
        state
            .update(
                &far,
                EntityPositionalAudioFrame {
                    sound_manager: None,
                    sound_pool: &pool,
                    listener: listener(),
                    elapsed_micros: 1_000_000,
                },
                &mut || panic!("inaudible rows must not advance warble or aliases"),
            )
            .unwrap();
        assert_eq!(state.fixed_loops.rows[0].admitted_voice, None);
        assert_eq!(state.fixed_loops.rows[0].warble, warble);
        let mut draws = 0;
        state
            .update(
                &near,
                EntityPositionalAudioFrame {
                    sound_manager: None,
                    sound_pool: &pool,
                    listener: listener(),
                    elapsed_micros: 0,
                },
                &mut || {
                    draws += 1;
                    4
                },
            )
            .unwrap();
        assert_eq!(
            draws, 1,
            "reentry recreates the logical voice without restarting warble"
        );
        assert_eq!(
            state.fixed_loops.rows[0]
                .admitted_voice
                .unwrap()
                .frequency_q16,
            65_534
        );
    }
}
