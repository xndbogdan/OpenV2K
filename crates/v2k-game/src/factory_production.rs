//! Pure Working Factory production state recovered from retail.
//!
//! `FUN_00418A90` copies the complete Section-13 0x58-byte factory template
//! into a zeroed 0xB8-byte runtime component and derives the counters below
//! from the entity's Section-12 health. `FUN_00418C20` changes staffing, while
//! the full-health branch of `FUN_00419010` owns the five production phases.
//!
//! This module deliberately does not find or move scientists, touch the live
//! entity list, or choose the world-style-dependent converted output. Those
//! are separate retail systems. It is an observational replay oracle: callers
//! supply facts already observed from retail, and receive a trace describing
//! the branch retail took. The trace is not a live execution plan.

/// Section-13 factory templates are copied as 22 little-endian dwords.
pub const FACTORY_CONFIG_BYTES: usize = 0x58;
const FACTORY_CONFIG_WORDS: usize = FACTORY_CONFIG_BYTES / 4;

/// Intermediate collectible created by phase 0.
pub const FACTORY_PICKUP_ENTITY_TYPE: u32 = 0x3D;
/// Materialiser proxy paired to an expired-stock converted output.
pub const FACTORY_MATERIALISER_ENTITY_TYPE: u32 = 0x5D;
/// Fixed positional sound played after a successful output/materialiser link.
///
/// Retail `FUN_0044F480` and demo `FUN_0044EC80` both receive global
/// Section-11 sound id 8 with full gain and a fixed 1.0 playback rate.
pub const FACTORY_OUTPUT_CONVERSION_SOUND_ID: u16 = 8;
/// Retail waits until production exceeds its threshold by this signed amount.
///
/// The comparison in `FUN_00419010` is strictly `< 0x3567E1`, so the first
/// conversion frame occurs only after more than 3,500,000 microseconds.
pub const FACTORY_OUTPUT_CONVERSION_GATE_RAW: i32 = 0x35_67E1;
/// Direct-text identifier emitted when a visible staffing change fills the
/// factory.
pub const FACTORY_CAPACITY_DIRECT_TEXT_ID: u16 = 0x00CE;
/// Direct-text identifier emitted when the understaffed timer expires.
pub const FACTORY_UNDERSTAFFED_DIRECT_TEXT_ID: u16 = 0x00CF;
/// Direct-text identifier emitted after a successful slow pickup production.
pub const FACTORY_HIGH_THRESHOLD_DIRECT_TEXT_ID: u16 = 0x00D1;
/// Deduplicated HUD resource event emitted before an output conversion attempt.
pub const FACTORY_OUTPUT_CONVERSION_HUD_RESOURCE_ID: u16 = 0x0016;

const CONFIG_OUTPUT_PAYLOAD: usize = 0x00 / 4;
const CONFIG_SCIENTIST_CAPACITY: usize = 0x04 / 4;
const CONFIG_PRODUCTION_THRESHOLD: usize = 0x08 / 4;
const CONFIG_DELIVERY_DURATION: usize = 0x0C / 4;
const CONFIG_COOLDOWN_DURATION: usize = 0x10 / 4;
const CONFIG_UNDERSTAFFED_LIMIT: usize = 0x14 / 4;

/// Exact Section-13 0x58-byte template used by `FUN_00418A90`.
///
/// Unknown words remain retained rather than receiving speculative names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactorySection13Config {
    words: [u32; FACTORY_CONFIG_WORDS],
}

impl FactorySection13Config {
    pub fn decode(raw: &[u8; FACTORY_CONFIG_BYTES]) -> Self {
        let mut words = [0; FACTORY_CONFIG_WORDS];
        for (word, bytes) in words.iter_mut().zip(raw.chunks_exact(4)) {
            *word = u32::from_le_bytes(bytes.try_into().expect("fixed four-byte chunk"));
        }
        Self { words }
    }

    pub const fn raw_words(self) -> [u32; FACTORY_CONFIG_WORDS] {
        self.words
    }

    /// Packed payload copied to the phase-0 type-61 pickup request.
    pub const fn output_payload_packed(self) -> u32 {
        self.words[CONFIG_OUTPUT_PAYLOAD]
    }

    pub const fn scientist_capacity_raw(self) -> i32 {
        self.words[CONFIG_SCIENTIST_CAPACITY] as i32
    }

    pub const fn production_threshold_micros_raw(self) -> i32 {
        self.words[CONFIG_PRODUCTION_THRESHOLD] as i32
    }

    pub const fn delivery_duration_micros_raw(self) -> i32 {
        self.words[CONFIG_DELIVERY_DURATION] as i32
    }

    pub const fn cooldown_duration_micros_raw(self) -> i32 {
        self.words[CONFIG_COOLDOWN_DURATION] as i32
    }

    pub const fn understaffed_limit_micros_raw(self) -> i32 {
        self.words[CONFIG_UNDERSTAFFED_LIMIT] as i32
    }

    /// Config dword 20: `FUN_00418A90` `param_2[0x14]` C830 primary selector.
    pub const fn primary_voice_sound_id(self) -> u32 {
        self.words[20]
    }

    /// Config dword 21: `FUN_00418A90` `param_2[0x15]` C830 secondary selector.
    pub const fn secondary_voice_sound_id(self) -> u32 {
        self.words[21]
    }
}

/// Logical `FUN_0044C830` loop stored at production `+0x8C` / `+0x90`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryVoiceLoop {
    pub sound_id: u16,
    pub gain_raw_16_16: i32,
    pub rate_raw_16_16: i32,
}

impl FactoryVoiceLoop {
    fn allocate(sound_id: u32) -> Option<Self> {
        if sound_id == 0 || sound_id > u32::from(u16::MAX) {
            return None;
        }
        Some(Self {
            sound_id: sound_id as u16,
            gain_raw_16_16: 0,
            rate_raw_16_16: 0x1_0000,
        })
    }

    const fn retune(self, gain_raw_16_16: i32, rate_raw_16_16: i32) -> Self {
        Self {
            sound_id: self.sound_id,
            gain_raw_16_16,
            rate_raw_16_16,
        }
    }

    const fn silence(self) -> Self {
        self.retune(0, 0x1_0000)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum FactoryProductionPhase {
    Producing = 0,
    Delivering = 1,
    WaitingForPickup = 2,
    Cooldown = 3,
    IdleEmpty = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservedFactorySpawnResult {
    Spawned { handle: u32 },
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservedFactoryPickupPresence {
    Active,
    Gone,
}

/// Facts already observed for one full-health `FUN_00419010` update.
///
/// Optional facts are required only on the branch that consumes them. Missing
/// branch-local evidence returns [`FactoryProductionReplayBlocked`] and leaves
/// the runtime byte-for-byte unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryProductionObservation {
    pub elapsed_micros: u32,
    pub current_health_raw: i32,
    pub maximum_health_raw: i32,
    pub pickup_presence: Option<ObservedFactoryPickupPresence>,
    pub pickup_spawn_result: Option<ObservedFactorySpawnResult>,
    /// Exact type returned by the separately recovered `FUN_0042EB70`.
    pub converted_output_entity_type: Option<u32>,
    pub converted_output_spawn_result: Option<ObservedFactorySpawnResult>,
    pub materialiser_spawn_result: Option<ObservedFactorySpawnResult>,
    /// Observed `DAT_004F741C != 0` during the phase-1 active-progress branch.
    ///
    /// Retail dispatches a presentation callback and clears the caller's
    /// status-publication suppression (`param_3 = 0`) on that branch.
    pub phase1_presentation_enabled: bool,
    /// Whether the call entered with nonzero `param_3`, suppressing the common
    /// status publication unless the phase-1 presentation branch clears it.
    pub suppress_status_publication: bool,
}

impl FactoryProductionObservation {
    pub const fn elapsed_at_full_health(elapsed_micros: u32, health_raw: i32) -> Self {
        Self {
            elapsed_micros,
            current_health_raw: health_raw,
            maximum_health_raw: health_raw,
            pickup_presence: None,
            pickup_spawn_result: None,
            converted_output_entity_type: None,
            converted_output_spawn_result: None,
            materialiser_spawn_result: None,
            phase1_presentation_enabled: false,
            suppress_status_publication: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryProductionReplayBlocked {
    /// The damaged-building recovery branch is intentionally outside this
    /// production-only module.
    DamagedFactoryRecovery,
    /// The cached-health delta can remove scientists before production runs;
    /// integration must recover that path before crossing this boundary.
    HealthTransition,
    PickupSpawnResult,
    PickupPresence,
    ConvertedOutputEntityType,
    ConvertedOutputSpawnResult,
    MaterialiserSpawnResult,
}

/// Non-executable trace of actions observed on the recovered retail branch.
///
/// This observational trace remains deliberately non-executable: callers
/// supply completed branch facts and the replay rolls back when one is absent.
/// [`crate::factory_production_live`] owns the exact 0x4C-byte spawn requests
/// and receipt-bound live ordering recovered later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryProductionTraceEvent {
    UnderstaffedCountdownExpiredObserved {
        direct_text_id: u16,
    },
    PickupSpawnAttemptObserved {
        entity_type: u32,
        payload_packed: u32,
    },
    PickupOwnerLinkObserved {
        pickup_handle: u32,
    },
    HighThresholdDirectTextObserved {
        direct_text_id: u16,
    },
    EntityLifetimeClearObserved,
    ConvertedOutputSpawnAttemptObserved {
        entity_type: u32,
    },
    ConvertedOutputOwnerLinkObserved {
        output_handle: u32,
    },
    MaterialiserSpawnAttemptObserved {
        entity_type: u32,
    },
    MaterialiserOutputLinkObserved {
        materialiser_handle: u32,
        output_handle: u32,
    },
    OutputConversionPositionalSoundObserved {
        sound_id: u16,
    },
    DeduplicatedHudResourceObserved {
        resource_id: u16,
    },
    Phase1PresentationDispatchObserved,
    StatusPublicationObserved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactoryProductionReplay {
    pub trace: Vec<FactoryProductionTraceEvent>,
    /// Retail sets entity flag `0x80` for the active phase-0, phase-1, and
    /// phase-3 updates when the caller requests normal publication.
    pub entity_dirty_flag_observed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryStaffingChange {
    pub previous_scientists_raw: i32,
    pub current_scientists_raw: i32,
    pub reached_capacity: bool,
    /// `FUN_00456900(0xCE)` is direct text, not audio. The `i32::MAX`
    /// initialization sentinel suppresses it.
    pub capacity_direct_text_id: Option<u16>,
    /// The same sentinel suppresses the explicit HUD refresh.
    pub refreshes_status: bool,
}

/// Exact production-owned subset of the retail 0xB8-byte component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryProductionRuntime {
    pub output_payload_packed: u32,
    pub scientist_capacity_raw: i32,
    pub production_threshold_micros_raw: i32,
    pub delivery_duration_micros_raw: i32,
    pub cooldown_duration_micros_raw: i32,
    pub understaffed_limit_micros_raw: i32,
    pub remaining_stock_raw: i32,
    pub cooldown_remaining_micros_raw: i32,
    pub production_progress_micros_raw: i32,
    pub delivery_progress_micros_raw: i32,
    pub current_scientists_raw: i32,
    pub understaffed_countdown_micros_raw: i32,
    /// Damage carried toward the next strict health-per-scientist staffing
    /// loss at retail component `+0x70`.
    pub health_loss_accumulator_raw: i32,
    pub cached_health_raw: i32,
    pub health_per_scientist_raw: i32,
    pub repair_rate_remainder_raw: i32,
    pub repair_rate_raw: i32,
    pub understaffed_step_per_scientist_raw: i32,
    pub spawned_pickup_handle: u32,
    pub phase: FactoryProductionPhase,
    /// Production `+0x8C` primary C830 loop. `None` is authored-silent.
    pub primary_voice: Option<FactoryVoiceLoop>,
    /// Production `+0x90` secondary C830 loop.
    pub secondary_voice: Option<FactoryVoiceLoop>,
}

impl FactoryProductionRuntime {
    /// Reproduce the deterministic numeric initialization in `FUN_00418A90`.
    ///
    /// `section12_health_raw` is the current entity health at construction;
    /// retail caches it at component `+0x74` and derives its repair rates from
    /// it and the Section-13 capacity.
    pub fn from_retail_template(config: FactorySection13Config, section12_health_raw: i32) -> Self {
        let capacity = config.scientist_capacity_raw();
        let (health_per_scientist_raw, repair_rate_raw, understaffed_step_per_scientist_raw) =
            if capacity != 0 {
                let per_scientist = section12_health_raw / capacity;
                (
                    per_scientist,
                    if per_scientist < 0x3D {
                        1
                    } else {
                        per_scientist / 0x3C
                    },
                    config.understaffed_limit_micros_raw() / capacity,
                )
            } else {
                (0, section12_health_raw / 0x3C, 0)
            };

        Self {
            output_payload_packed: config.output_payload_packed(),
            scientist_capacity_raw: capacity,
            production_threshold_micros_raw: config.production_threshold_micros_raw(),
            delivery_duration_micros_raw: config.delivery_duration_micros_raw(),
            cooldown_duration_micros_raw: config.cooldown_duration_micros_raw(),
            understaffed_limit_micros_raw: config.understaffed_limit_micros_raw(),
            remaining_stock_raw: if config.cooldown_duration_micros_raw() < 0 {
                1
            } else {
                i32::MAX
            },
            cooldown_remaining_micros_raw: 0,
            production_progress_micros_raw: 0,
            delivery_progress_micros_raw: 0,
            current_scientists_raw: 0,
            understaffed_countdown_micros_raw: 0,
            health_loss_accumulator_raw: 0,
            cached_health_raw: section12_health_raw,
            health_per_scientist_raw,
            repair_rate_remainder_raw: 0,
            repair_rate_raw,
            understaffed_step_per_scientist_raw,
            spawned_pickup_handle: 0,
            phase: FactoryProductionPhase::Producing,
            primary_voice: FactoryVoiceLoop::allocate(config.primary_voice_sound_id()),
            secondary_voice: FactoryVoiceLoop::allocate(config.secondary_voice_sound_id()),
        }
    }

    /// `FUN_00418F60` retunes C830 handles from production `+0xB4` bits.
    pub fn retune_voices_from_animation_state(
        &mut self,
        next_state_raw: u32,
        previous_state_raw: u32,
    ) {
        let changed = previous_state_raw ^ next_state_raw;
        if changed & 5 != 0 {
            if let Some(voice) = self.primary_voice {
                self.primary_voice = Some(if next_state_raw & 4 == 0 {
                    voice.retune(((next_state_raw & 1) << 16) as i32, 0x1_0000)
                } else {
                    voice.retune(0x1_0000, 0x1_8000)
                });
            }
        }
        if changed & 2 != 0 {
            if let Some(voice) = self.secondary_voice {
                self.secondary_voice =
                    Some(voice.retune(((next_state_raw & 2) << 15) as i32, 0x1_0000));
            }
        }
    }

    /// `FUN_00419750` silences both loops without releasing them.
    pub fn silence_voices(&mut self) {
        if let Some(voice) = self.primary_voice {
            self.primary_voice = Some(voice.silence());
        }
        if let Some(voice) = self.secondary_voice {
            self.secondary_voice = Some(voice.silence());
        }
    }

    /// `FUN_00418BE0`: `FUN_0044CC90` secondary (`+0x90`) then primary (`+0x8C`).
    pub fn release_voices(&mut self) {
        self.secondary_voice = None;
        self.primary_voice = None;
    }

    /// Reproduce the accepted staffing mutation in `FUN_00418C20`.
    ///
    /// Delivery validation and scientist movement are intentionally external.
    pub fn adjust_staffing(&mut self, delta: i32) -> FactoryStaffingChange {
        let previous = self.current_scientists_raw;
        let candidate = previous.wrapping_add(delta);
        if candidate < self.scientist_capacity_raw {
            self.current_scientists_raw = candidate;
            if self.understaffed_limit_micros_raw > 0 {
                let adjusted = self
                    .understaffed_countdown_micros_raw
                    .wrapping_sub(self.understaffed_step_per_scientist_raw.wrapping_mul(delta));
                self.understaffed_countdown_micros_raw = adjusted.max(0);
            }
            FactoryStaffingChange {
                previous_scientists_raw: previous,
                current_scientists_raw: candidate,
                reached_capacity: false,
                capacity_direct_text_id: None,
                refreshes_status: false,
            }
        } else {
            self.current_scientists_raw = self.scientist_capacity_raw;
            if self.understaffed_limit_micros_raw != 0 {
                self.understaffed_countdown_micros_raw = 0;
            }
            let visible_change = delta != i32::MAX;
            FactoryStaffingChange {
                previous_scientists_raw: previous,
                current_scientists_raw: self.current_scientists_raw,
                reached_capacity: true,
                capacity_direct_text_id: (self.understaffed_limit_micros_raw != 0
                    && visible_change)
                    .then_some(FACTORY_CAPACITY_DIRECT_TEXT_ID),
                refreshes_status: visible_change,
            }
        }
    }

    /// Replay one observed full-health production update.
    ///
    /// This is an observational oracle, not a live request executor. Spawn and
    /// presence results must have been observed before this call; the returned
    /// trace describes what retail did and must not be executed to manufacture
    /// those already-supplied observations.
    ///
    /// The replay is transactional: unresolved external state returns an error
    /// without committing timer or phase changes.
    pub fn replay_observed_update(
        &mut self,
        observation: FactoryProductionObservation,
    ) -> Result<FactoryProductionReplay, FactoryProductionReplayBlocked> {
        let mut next = *self;
        let replay = next.replay_observed_update_inner(observation)?;
        *self = next;
        Ok(replay)
    }

    fn replay_observed_update_inner(
        &mut self,
        observation: FactoryProductionObservation,
    ) -> Result<FactoryProductionReplay, FactoryProductionReplayBlocked> {
        if observation.current_health_raw != observation.maximum_health_raw {
            return Err(FactoryProductionReplayBlocked::DamagedFactoryRecovery);
        }
        if self.cached_health_raw != observation.current_health_raw {
            return Err(FactoryProductionReplayBlocked::HealthTransition);
        }

        let elapsed = observation.elapsed_micros as i32;
        let fully_staffed = self.current_scientists_raw >= self.scientist_capacity_raw;
        let mut trace = Vec::new();
        if fully_staffed {
            if observation.elapsed_micros < self.understaffed_countdown_micros_raw as u32 {
                self.understaffed_countdown_micros_raw =
                    self.understaffed_countdown_micros_raw.wrapping_sub(elapsed);
            } else {
                self.understaffed_countdown_micros_raw = 0;
            }
        } else if self.understaffed_limit_micros_raw != 0 {
            self.understaffed_countdown_micros_raw =
                self.understaffed_countdown_micros_raw.wrapping_add(elapsed);
            if self.understaffed_limit_micros_raw <= self.understaffed_countdown_micros_raw {
                trace.push(
                    FactoryProductionTraceEvent::UnderstaffedCountdownExpiredObserved {
                        direct_text_id: FACTORY_UNDERSTAFFED_DIRECT_TEXT_ID,
                    },
                );
            }
        }

        let mut dirty_phase_observed = false;
        let mut status_publication_enabled = !observation.suppress_status_publication;
        if fully_staffed {
            match self.phase {
                FactoryProductionPhase::Producing => {
                    dirty_phase_observed = true;
                    if self.production_progress_micros_raw < self.production_threshold_micros_raw {
                        self.production_progress_micros_raw =
                            self.production_progress_micros_raw.wrapping_add(elapsed);
                        if self.production_threshold_micros_raw
                            <= self.production_progress_micros_raw
                        {
                            let spawn = observation
                                .pickup_spawn_result
                                .ok_or(FactoryProductionReplayBlocked::PickupSpawnResult)?;
                            trace.push(FactoryProductionTraceEvent::PickupSpawnAttemptObserved {
                                entity_type: FACTORY_PICKUP_ENTITY_TYPE,
                                payload_packed: self.output_payload_packed,
                            });
                            if let ObservedFactorySpawnResult::Spawned { handle } = spawn {
                                self.spawned_pickup_handle = handle;
                                self.remaining_stock_raw = self.remaining_stock_raw.wrapping_sub(1);
                                trace.push(FactoryProductionTraceEvent::PickupOwnerLinkObserved {
                                    pickup_handle: handle,
                                });
                                if 4_000_000 < self.production_threshold_micros_raw {
                                    trace.push(
                                        FactoryProductionTraceEvent::HighThresholdDirectTextObserved {
                                            direct_text_id:
                                                FACTORY_HIGH_THRESHOLD_DIRECT_TEXT_ID,
                                        },
                                    );
                                }
                            }
                            self.production_progress_micros_raw =
                                self.production_threshold_micros_raw;
                            self.phase = FactoryProductionPhase::Delivering;
                            trace.push(FactoryProductionTraceEvent::EntityLifetimeClearObserved);
                        }
                    }
                }
                FactoryProductionPhase::Delivering => {
                    dirty_phase_observed = true;
                    if self.delivery_progress_micros_raw < self.delivery_duration_micros_raw {
                        self.delivery_progress_micros_raw =
                            self.delivery_progress_micros_raw.wrapping_add(elapsed);
                        if self.delivery_duration_micros_raw <= self.delivery_progress_micros_raw {
                            self.delivery_progress_micros_raw = self.delivery_duration_micros_raw;
                            self.phase = FactoryProductionPhase::WaitingForPickup;
                        }
                        if observation.phase1_presentation_enabled {
                            trace.push(
                                FactoryProductionTraceEvent::Phase1PresentationDispatchObserved,
                            );
                            status_publication_enabled = true;
                        }
                    }
                }
                FactoryProductionPhase::WaitingForPickup => {
                    if self.remaining_stock_raw == 0 {
                        if self.current_scientists_raw == 0 {
                            self.phase = FactoryProductionPhase::IdleEmpty;
                            self.production_progress_micros_raw = 0;
                        } else if self
                            .production_progress_micros_raw
                            .wrapping_sub(self.production_threshold_micros_raw)
                            < FACTORY_OUTPUT_CONVERSION_GATE_RAW
                        {
                            self.production_progress_micros_raw =
                                self.production_progress_micros_raw.wrapping_add(elapsed);
                        } else {
                            let entity_type = observation
                                .converted_output_entity_type
                                .ok_or(FactoryProductionReplayBlocked::ConvertedOutputEntityType)?;
                            let output_spawn = observation.converted_output_spawn_result.ok_or(
                                FactoryProductionReplayBlocked::ConvertedOutputSpawnResult,
                            )?;
                            trace.push(
                                FactoryProductionTraceEvent::DeduplicatedHudResourceObserved {
                                    resource_id: FACTORY_OUTPUT_CONVERSION_HUD_RESOURCE_ID,
                                },
                            );
                            trace.push(
                                FactoryProductionTraceEvent::ConvertedOutputSpawnAttemptObserved {
                                    entity_type,
                                },
                            );
                            if let ObservedFactorySpawnResult::Spawned {
                                handle: output_handle,
                            } = output_spawn
                            {
                                let materialiser_spawn =
                                    observation.materialiser_spawn_result.ok_or(
                                        FactoryProductionReplayBlocked::MaterialiserSpawnResult,
                                    )?;
                                self.current_scientists_raw =
                                    self.current_scientists_raw.wrapping_sub(1);
                                self.scientist_capacity_raw =
                                    self.scientist_capacity_raw.wrapping_sub(1);
                                trace.push(
                                    FactoryProductionTraceEvent::ConvertedOutputOwnerLinkObserved {
                                        output_handle,
                                    },
                                );
                                trace.push(
                                    FactoryProductionTraceEvent::MaterialiserSpawnAttemptObserved {
                                        entity_type: FACTORY_MATERIALISER_ENTITY_TYPE,
                                    },
                                );
                                if let ObservedFactorySpawnResult::Spawned {
                                    handle: materialiser_handle,
                                } = materialiser_spawn
                                {
                                    trace.push(
                                        FactoryProductionTraceEvent::MaterialiserOutputLinkObserved {
                                            materialiser_handle,
                                            output_handle,
                                        },
                                    );
                                    trace.push(
                                        FactoryProductionTraceEvent::OutputConversionPositionalSoundObserved {
                                            sound_id: FACTORY_OUTPUT_CONVERSION_SOUND_ID,
                                        },
                                    );
                                }
                            }
                        }
                    } else {
                        let presence = observation
                            .pickup_presence
                            .ok_or(FactoryProductionReplayBlocked::PickupPresence)?;
                        if presence == ObservedFactoryPickupPresence::Gone {
                            self.cooldown_remaining_micros_raw = self.cooldown_duration_micros_raw;
                            self.production_progress_micros_raw = 0;
                            self.delivery_progress_micros_raw = 0;
                            self.phase = FactoryProductionPhase::Cooldown;
                        }
                    }
                }
                FactoryProductionPhase::Cooldown => {
                    dirty_phase_observed = true;
                    if elapsed < self.cooldown_remaining_micros_raw {
                        self.cooldown_remaining_micros_raw =
                            self.cooldown_remaining_micros_raw.wrapping_sub(elapsed);
                    } else {
                        self.cooldown_remaining_micros_raw = 0;
                        self.phase = FactoryProductionPhase::Producing;
                    }
                }
                FactoryProductionPhase::IdleEmpty => {
                    if self.current_scientists_raw != 0 {
                        self.phase = FactoryProductionPhase::WaitingForPickup;
                    }
                }
            }
        }

        if status_publication_enabled {
            trace.push(FactoryProductionTraceEvent::StatusPublicationObserved);
        }

        Ok(FactoryProductionReplay {
            trace,
            entity_dirty_flag_observed: status_publication_enabled && dirty_phase_observed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(
        payload: u32,
        capacity: i32,
        production: i32,
        delivery: i32,
        cooldown: i32,
        understaffed: i32,
    ) -> FactorySection13Config {
        let mut raw = [0; FACTORY_CONFIG_BYTES];
        for (offset, value) in [
            (0x00, payload),
            (0x04, capacity as u32),
            (0x08, production as u32),
            (0x0C, delivery as u32),
            (0x10, cooldown as u32),
            (0x14, understaffed as u32),
        ] {
            raw[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        FactorySection13Config::decode(&raw)
    }

    fn full_observation(elapsed_micros: u32) -> FactoryProductionObservation {
        FactoryProductionObservation::elapsed_at_full_health(elapsed_micros, 10_000_000)
    }

    #[test]
    fn constructor_derives_section12_health_rates_and_stock_sentinel() {
        let finite = FactoryProductionRuntime::from_retail_template(
            config(0x1_F412, 2, 6_000_000, 5_000_000, -1_000, 120_000),
            10_000_000,
        );
        assert_eq!(finite.remaining_stock_raw, 1);
        assert_eq!(finite.health_per_scientist_raw, 5_000_000);
        assert_eq!(finite.repair_rate_raw, 83_333);
        assert_eq!(finite.understaffed_step_per_scientist_raw, 60_000);

        let unlimited = FactoryProductionRuntime::from_retail_template(
            config(0, 0, 1, 1, 5_000_000, 0),
            10_000_000,
        );
        assert_eq!(unlimited.remaining_stock_raw, i32::MAX);
        assert_eq!(unlimited.health_per_scientist_raw, 0);
        assert_eq!(unlimited.repair_rate_raw, 166_666);
    }

    #[test]
    fn staffing_clamps_at_capacity_and_applies_exact_countdown_delta() {
        let mut runtime = FactoryProductionRuntime::from_retail_template(
            config(0, 2, 1, 1, 1, 120_000),
            10_000_000,
        );
        runtime.understaffed_countdown_micros_raw = 10_000;

        let first = runtime.adjust_staffing(1);
        assert_eq!(first.current_scientists_raw, 1);
        assert!(!first.reached_capacity);
        assert_eq!(runtime.understaffed_countdown_micros_raw, 0);

        runtime.understaffed_countdown_micros_raw = 90_000;
        let full = runtime.adjust_staffing(9);
        assert_eq!(full.current_scientists_raw, 2);
        assert!(full.reached_capacity);
        assert_eq!(
            full.capacity_direct_text_id,
            Some(FACTORY_CAPACITY_DIRECT_TEXT_ID)
        );
        assert!(full.refreshes_status);
        assert_eq!(runtime.understaffed_countdown_micros_raw, 0);

        let mut initialization = FactoryProductionRuntime::from_retail_template(
            config(0, 2, 1, 1, 1, 120_000),
            10_000_000,
        );
        let sentinel = initialization.adjust_staffing(i32::MAX);
        assert!(sentinel.reached_capacity);
        assert_eq!(sentinel.capacity_direct_text_id, None);
        assert!(!sentinel.refreshes_status);
    }

    #[test]
    fn producing_replays_pickup_then_delivery_reaches_wait_state() {
        let mut runtime = FactoryProductionRuntime::from_retail_template(
            config(0x1_F412, 2, 100, 50, 30, 0),
            10_000_000,
        );
        runtime.adjust_staffing(2);

        assert_eq!(
            runtime
                .replay_observed_update(full_observation(99))
                .unwrap(),
            FactoryProductionReplay {
                trace: vec![FactoryProductionTraceEvent::StatusPublicationObserved],
                entity_dirty_flag_observed: true,
            }
        );
        assert_eq!(runtime.production_progress_micros_raw, 99);

        let before = runtime;
        assert_eq!(
            runtime.replay_observed_update(full_observation(1)),
            Err(FactoryProductionReplayBlocked::PickupSpawnResult)
        );
        assert_eq!(runtime, before, "missing spawn result is transactional");

        let mut spawn = full_observation(1);
        spawn.pickup_spawn_result = Some(ObservedFactorySpawnResult::Spawned { handle: 0x1234 });
        let trace = runtime.replay_observed_update(spawn).unwrap().trace;
        assert_eq!(
            trace,
            [
                FactoryProductionTraceEvent::PickupSpawnAttemptObserved {
                    entity_type: 61,
                    payload_packed: 0x1_F412,
                },
                FactoryProductionTraceEvent::PickupOwnerLinkObserved {
                    pickup_handle: 0x1234,
                },
                FactoryProductionTraceEvent::EntityLifetimeClearObserved,
                FactoryProductionTraceEvent::StatusPublicationObserved,
            ]
        );
        assert_eq!(runtime.phase, FactoryProductionPhase::Delivering);
        assert_eq!(runtime.production_progress_micros_raw, 100);
        assert_eq!(runtime.remaining_stock_raw, i32::MAX - 1);

        runtime
            .replay_observed_update(full_observation(49))
            .unwrap();
        assert_eq!(runtime.phase, FactoryProductionPhase::Delivering);
        runtime.replay_observed_update(full_observation(1)).unwrap();
        assert_eq!(runtime.phase, FactoryProductionPhase::WaitingForPickup);
        assert_eq!(runtime.delivery_progress_micros_raw, 50);
    }

    #[test]
    fn failed_pickup_spawn_still_enters_delivery_and_clears_lifetime() {
        let mut runtime = FactoryProductionRuntime::from_retail_template(
            config(0x1_F412, 1, 100, 50, 30, 0),
            10_000_000,
        );
        runtime.adjust_staffing(1);
        let stock_before = runtime.remaining_stock_raw;

        let mut observation = full_observation(100);
        observation.pickup_spawn_result = Some(ObservedFactorySpawnResult::Failed);
        let replay = runtime.replay_observed_update(observation).unwrap();

        assert_eq!(
            replay.trace,
            [
                FactoryProductionTraceEvent::PickupSpawnAttemptObserved {
                    entity_type: FACTORY_PICKUP_ENTITY_TYPE,
                    payload_packed: 0x1_F412,
                },
                FactoryProductionTraceEvent::EntityLifetimeClearObserved,
                FactoryProductionTraceEvent::StatusPublicationObserved,
            ]
        );
        assert_eq!(runtime.phase, FactoryProductionPhase::Delivering);
        assert_eq!(runtime.production_progress_micros_raw, 100);
        assert_eq!(runtime.remaining_stock_raw, stock_before);
        assert_eq!(runtime.spawned_pickup_handle, 0);
    }

    #[test]
    fn high_threshold_direct_text_requires_a_successful_pickup_spawn() {
        let make_runtime = || {
            let mut runtime = FactoryProductionRuntime::from_retail_template(
                config(0, 1, 4_000_001, 1, 1, 0),
                10_000_000,
            );
            runtime.adjust_staffing(1);
            runtime
        };

        let mut failed = make_runtime();
        let mut failed_observation = full_observation(4_000_001);
        failed_observation.pickup_spawn_result = Some(ObservedFactorySpawnResult::Failed);
        let failed_trace = failed
            .replay_observed_update(failed_observation)
            .unwrap()
            .trace;
        assert!(!failed_trace.iter().any(|event| matches!(
            event,
            FactoryProductionTraceEvent::HighThresholdDirectTextObserved { .. }
        )));

        let mut succeeded = make_runtime();
        let mut succeeded_observation = full_observation(4_000_001);
        succeeded_observation.pickup_spawn_result =
            Some(ObservedFactorySpawnResult::Spawned { handle: 7 });
        let succeeded_trace = succeeded
            .replay_observed_update(succeeded_observation)
            .unwrap()
            .trace;
        assert!(succeeded_trace.contains(
            &FactoryProductionTraceEvent::HighThresholdDirectTextObserved {
                direct_text_id: FACTORY_HIGH_THRESHOLD_DIRECT_TEXT_ID,
            }
        ));
    }

    #[test]
    fn pickup_presence_then_cooldown_follows_strict_phase_boundaries() {
        let mut runtime =
            FactoryProductionRuntime::from_retail_template(config(0, 0, 1, 1, 30, 0), 10_000_000);
        runtime.remaining_stock_raw = 7;
        runtime.phase = FactoryProductionPhase::WaitingForPickup;

        let before = runtime;
        assert_eq!(
            runtime.replay_observed_update(full_observation(20)),
            Err(FactoryProductionReplayBlocked::PickupPresence)
        );
        assert_eq!(runtime, before);

        let mut active = full_observation(20);
        active.pickup_presence = Some(ObservedFactoryPickupPresence::Active);
        runtime.replay_observed_update(active).unwrap();
        assert_eq!(runtime.phase, FactoryProductionPhase::WaitingForPickup);

        let mut gone = full_observation(20);
        gone.pickup_presence = Some(ObservedFactoryPickupPresence::Gone);
        runtime.replay_observed_update(gone).unwrap();
        assert_eq!(runtime.phase, FactoryProductionPhase::Cooldown);
        assert_eq!(runtime.cooldown_remaining_micros_raw, 30);

        runtime
            .replay_observed_update(full_observation(29))
            .unwrap();
        assert_eq!(runtime.phase, FactoryProductionPhase::Cooldown);
        assert_eq!(runtime.cooldown_remaining_micros_raw, 1);
        runtime.replay_observed_update(full_observation(1)).unwrap();
        assert_eq!(runtime.phase, FactoryProductionPhase::Producing);
    }

    #[test]
    fn exhausted_stock_conversion_replays_complete_success() {
        let mut runtime = FactoryProductionRuntime::from_retail_template(
            config(0, 2, 100, 1, -1_000, 0),
            10_000_000,
        );
        runtime.adjust_staffing(2);
        runtime.remaining_stock_raw = 0;
        runtime.production_progress_micros_raw = 100 + FACTORY_OUTPUT_CONVERSION_GATE_RAW - 1;
        runtime.phase = FactoryProductionPhase::WaitingForPickup;

        runtime.replay_observed_update(full_observation(1)).unwrap();
        assert_eq!(
            runtime.production_progress_micros_raw,
            100 + FACTORY_OUTPUT_CONVERSION_GATE_RAW,
            "the last below-gate frame may advance exactly to 0x3567E1"
        );

        let before = runtime;
        assert_eq!(
            runtime.replay_observed_update(full_observation(1)),
            Err(FactoryProductionReplayBlocked::ConvertedOutputEntityType)
        );
        assert_eq!(runtime, before);

        let mut convert = full_observation(1);
        convert.converted_output_entity_type = Some(8);
        convert.converted_output_spawn_result =
            Some(ObservedFactorySpawnResult::Spawned { handle: 0x2222 });
        convert.materialiser_spawn_result =
            Some(ObservedFactorySpawnResult::Spawned { handle: 0x3333 });
        let result = runtime.replay_observed_update(convert).unwrap();
        assert_eq!(
            result.trace,
            [
                FactoryProductionTraceEvent::DeduplicatedHudResourceObserved {
                    resource_id: FACTORY_OUTPUT_CONVERSION_HUD_RESOURCE_ID,
                },
                FactoryProductionTraceEvent::ConvertedOutputSpawnAttemptObserved { entity_type: 8 },
                FactoryProductionTraceEvent::ConvertedOutputOwnerLinkObserved {
                    output_handle: 0x2222,
                },
                FactoryProductionTraceEvent::MaterialiserSpawnAttemptObserved { entity_type: 93 },
                FactoryProductionTraceEvent::MaterialiserOutputLinkObserved {
                    materialiser_handle: 0x3333,
                    output_handle: 0x2222,
                },
                FactoryProductionTraceEvent::OutputConversionPositionalSoundObserved {
                    sound_id: 8,
                },
                FactoryProductionTraceEvent::StatusPublicationObserved,
            ]
        );
        assert_eq!(runtime.current_scientists_raw, 1);
        assert_eq!(runtime.scientist_capacity_raw, 1);

        runtime.current_scientists_raw = 0;
        runtime.scientist_capacity_raw = 0;
        runtime.replay_observed_update(full_observation(1)).unwrap();
        assert_eq!(runtime.phase, FactoryProductionPhase::IdleEmpty);
        assert_eq!(runtime.production_progress_micros_raw, 0);
    }

    #[test]
    fn failed_output_spawn_retries_without_decrementing_staffing() {
        let mut runtime = conversion_ready_runtime();
        let before_counts = (
            runtime.current_scientists_raw,
            runtime.scientist_capacity_raw,
        );
        let observation = failed_output_observation();

        let first = runtime.replay_observed_update(observation).unwrap();
        assert_eq!(
            first.trace,
            [
                FactoryProductionTraceEvent::DeduplicatedHudResourceObserved {
                    resource_id: FACTORY_OUTPUT_CONVERSION_HUD_RESOURCE_ID,
                },
                FactoryProductionTraceEvent::ConvertedOutputSpawnAttemptObserved { entity_type: 8 },
                FactoryProductionTraceEvent::StatusPublicationObserved,
            ]
        );
        assert_eq!(
            (
                runtime.current_scientists_raw,
                runtime.scientist_capacity_raw
            ),
            before_counts
        );
        assert_eq!(runtime.phase, FactoryProductionPhase::WaitingForPickup);

        let second = runtime.replay_observed_update(observation).unwrap();
        assert_eq!(
            second.trace, first.trace,
            "retail retries on the next update"
        );
        assert_eq!(
            (
                runtime.current_scientists_raw,
                runtime.scientist_capacity_raw
            ),
            before_counts
        );
    }

    #[test]
    fn materialiser_failure_still_consumes_one_scientist_and_capacity_slot() {
        let mut runtime = conversion_ready_runtime();
        let mut observation = full_observation(1);
        observation.converted_output_entity_type = Some(8);
        observation.converted_output_spawn_result =
            Some(ObservedFactorySpawnResult::Spawned { handle: 0x2222 });
        observation.materialiser_spawn_result = Some(ObservedFactorySpawnResult::Failed);

        let replay = runtime.replay_observed_update(observation).unwrap();
        assert_eq!(runtime.current_scientists_raw, 1);
        assert_eq!(runtime.scientist_capacity_raw, 1);
        assert!(replay.trace.contains(
            &FactoryProductionTraceEvent::MaterialiserSpawnAttemptObserved {
                entity_type: FACTORY_MATERIALISER_ENTITY_TYPE,
            }
        ));
        assert!(!replay.trace.iter().any(|event| matches!(
            event,
            FactoryProductionTraceEvent::MaterialiserOutputLinkObserved { .. }
                | FactoryProductionTraceEvent::OutputConversionPositionalSoundObserved { .. }
        )));
    }

    #[test]
    fn conversion_gate_overshoot_is_acted_on_only_next_update() {
        let mut runtime = conversion_ready_runtime();
        runtime.production_progress_micros_raw =
            runtime.production_threshold_micros_raw + FACTORY_OUTPUT_CONVERSION_GATE_RAW - 1;

        let first = runtime
            .replay_observed_update(full_observation(20))
            .unwrap();
        assert_eq!(
            runtime.production_progress_micros_raw,
            runtime.production_threshold_micros_raw + FACTORY_OUTPUT_CONVERSION_GATE_RAW + 19
        );
        assert_eq!(
            first.trace,
            [FactoryProductionTraceEvent::StatusPublicationObserved]
        );

        let before = runtime;
        assert_eq!(
            runtime.replay_observed_update(full_observation(1)),
            Err(FactoryProductionReplayBlocked::ConvertedOutputEntityType)
        );
        assert_eq!(runtime, before);

        let retry = runtime
            .replay_observed_update(failed_output_observation())
            .unwrap();
        assert!(retry.trace.contains(
            &FactoryProductionTraceEvent::ConvertedOutputSpawnAttemptObserved { entity_type: 8 }
        ));
    }

    #[test]
    fn understaffed_expiry_is_direct_text_and_does_not_advance_production() {
        let mut runtime = FactoryProductionRuntime::from_retail_template(
            config(0, 2, 100, 1, 1, 100),
            10_000_000,
        );

        let replay = runtime
            .replay_observed_update(full_observation(100))
            .unwrap();
        assert_eq!(
            replay.trace,
            [
                FactoryProductionTraceEvent::UnderstaffedCountdownExpiredObserved {
                    direct_text_id: FACTORY_UNDERSTAFFED_DIRECT_TEXT_ID,
                },
                FactoryProductionTraceEvent::StatusPublicationObserved,
            ]
        );
        assert_eq!(runtime.production_progress_micros_raw, 0);
        assert!(!replay.entity_dirty_flag_observed);
    }

    #[test]
    fn phase1_presentation_clears_status_suppression_only_on_active_progress() {
        let make_runtime = || {
            let mut runtime = FactoryProductionRuntime::from_retail_template(
                config(0, 1, 1, 100, 1, 0),
                10_000_000,
            );
            runtime.adjust_staffing(1);
            runtime.phase = FactoryProductionPhase::Delivering;
            runtime
        };

        let mut suppressed = make_runtime();
        let mut observation = full_observation(1);
        observation.suppress_status_publication = true;
        let replay = suppressed.replay_observed_update(observation).unwrap();
        assert!(replay.trace.is_empty());
        assert!(!replay.entity_dirty_flag_observed);

        let mut presentation = make_runtime();
        observation.phase1_presentation_enabled = true;
        let replay = presentation.replay_observed_update(observation).unwrap();
        assert_eq!(
            replay.trace,
            [
                FactoryProductionTraceEvent::Phase1PresentationDispatchObserved,
                FactoryProductionTraceEvent::StatusPublicationObserved,
            ]
        );
        assert!(replay.entity_dirty_flag_observed);

        let mut normal = make_runtime();
        let replay = normal.replay_observed_update(full_observation(1)).unwrap();
        assert_eq!(
            replay.trace,
            [FactoryProductionTraceEvent::StatusPublicationObserved]
        );
        assert!(replay.entity_dirty_flag_observed);

        let mut completed = make_runtime();
        completed.delivery_progress_micros_raw = completed.delivery_duration_micros_raw;
        let replay = completed.replay_observed_update(observation).unwrap();
        assert!(
            replay.trace.is_empty(),
            "the DAT_004F741C branch is nested inside active phase-1 progress"
        );
        assert!(!replay.entity_dirty_flag_observed);
    }

    #[test]
    fn damaged_or_changed_health_fails_closed_without_mutation() {
        let mut runtime =
            FactoryProductionRuntime::from_retail_template(config(0, 0, 1, 1, 1, 0), 10_000_000);
        let before = runtime;
        let mut damaged = full_observation(20_000);
        damaged.current_health_raw -= 1;
        assert_eq!(
            runtime.replay_observed_update(damaged),
            Err(FactoryProductionReplayBlocked::DamagedFactoryRecovery)
        );
        assert_eq!(runtime, before);

        let changed = FactoryProductionObservation::elapsed_at_full_health(20_000, 9_000_000);
        assert_eq!(
            runtime.replay_observed_update(changed),
            Err(FactoryProductionReplayBlocked::HealthTransition)
        );
        assert_eq!(runtime, before);
    }

    fn conversion_ready_runtime() -> FactoryProductionRuntime {
        let mut runtime = FactoryProductionRuntime::from_retail_template(
            config(0, 2, 100, 1, -1_000, 0),
            10_000_000,
        );
        runtime.adjust_staffing(2);
        runtime.remaining_stock_raw = 0;
        runtime.production_progress_micros_raw =
            runtime.production_threshold_micros_raw + FACTORY_OUTPUT_CONVERSION_GATE_RAW;
        runtime.phase = FactoryProductionPhase::WaitingForPickup;
        runtime
    }

    fn failed_output_observation() -> FactoryProductionObservation {
        let mut observation = full_observation(1);
        observation.converted_output_entity_type = Some(8);
        observation.converted_output_spawn_result = Some(ObservedFactorySpawnResult::Failed);
        observation
    }

    #[test]
    fn config_word_20_allocates_primary_c830_loop_and_authored_none_stays_silent() {
        let mut raw = [0_u8; FACTORY_CONFIG_BYTES];
        raw[0x50..0x54].copy_from_slice(&31_u32.to_le_bytes());
        let voiced = FactoryProductionRuntime::from_retail_template(
            FactorySection13Config::decode(&raw),
            99_999,
        );
        assert_eq!(
            voiced.primary_voice,
            Some(FactoryVoiceLoop {
                sound_id: 31,
                gain_raw_16_16: 0,
                rate_raw_16_16: 0x1_0000,
            })
        );
        assert_eq!(voiced.secondary_voice, None);

        let silent = FactoryProductionRuntime::from_retail_template(
            FactorySection13Config::decode(&[0; FACTORY_CONFIG_BYTES]),
            99_999,
        );
        assert_eq!(silent.primary_voice, None);
        assert_eq!(silent.secondary_voice, None);

        let mut delivering = voiced;
        delivering.retune_voices_from_animation_state(1, 0);
        assert_eq!(delivering.primary_voice.unwrap().gain_raw_16_16, 0x1_0000);
        assert_eq!(delivering.primary_voice.unwrap().rate_raw_16_16, 0x1_0000);
        delivering.retune_voices_from_animation_state(4, 1);
        assert_eq!(delivering.primary_voice.unwrap().gain_raw_16_16, 0x1_0000);
        assert_eq!(delivering.primary_voice.unwrap().rate_raw_16_16, 0x1_8000);
        delivering.silence_voices();
        assert_eq!(delivering.primary_voice.unwrap().gain_raw_16_16, 0);
        assert_eq!(delivering.primary_voice.unwrap().rate_raw_16_16, 0x1_0000);
        delivering.release_voices();
        assert_eq!(delivering.primary_voice, None);
        assert_eq!(delivering.secondary_voice, None);
    }
}
