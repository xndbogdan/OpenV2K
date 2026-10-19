//! The abort predicates in campaign selector zero (`0042DD10`).
//!
//! An abort row (flag `0x80`) runs `0042DEF8..0042E06F`'s predicates in this
//! order, and the first that fails resumes the scan at the next record
//! (`0042E074`):
//!
//! - `0x02`: peasants (controller `+0x1B0`) at least signed byte `+0x18`;
//! - `0x04`: the casualty limit. The scientist-plus-peasant census is compared
//!   with signed byte `+0x19`; it is a minimum-survivor boundary, not a
//!   subtraction from the load-time population. Sub-M occupants are already
//!   included in the scientist bucket by `0042E210`;
//! - `0x08`: `0042E210` bucket `+0x14` (masks `DAT_004CAA30`) is empty;
//! - `0x20`: `FUN_004366F0`'s landscape virus percentage reaches signed byte
//!   `+0x1A`, after the critical-level warning.
//!
//! Every authored abort row is `0x84`, `0x86`, `0x88` or `0xA0`; any other
//! bit stops the scan with [`UnsupportedCasualtyRecord`].
//!
//! The single-player caller evaluates these predicates after the actor and
//! contact phases, at the same deferred campaign boundary as Main Base loss.
//! This module also owns selector entry ordering, including the aborted
//! interior's early current-world return. Marker arrival and the systemic
//! `00456960` body retain their respective owners. No actor is removed by
//! these predicates.

use crate::world_complete_results::Fun0042e210CapabilityCensus;
use v2k_formats::levels::CampaignRecord;

const ABORT_FLAG: u32 = 0x80;
const MINIMUM_PEASANTS_FLAG: u32 = 0x02;
const CASUALTY_FLAG: u32 = 0x04;
const CENSUS_FLAG: u32 = 0x08;
const VIRUS_FLAG: u32 = 0x20;
const SUPPORTED_ABORT_FLAGS: u32 =
    ABORT_FLAG | MINIMUM_PEASANTS_FLAG | CASUALTY_FLAG | CENSUS_FLAG | VIRUS_FLAG;
/// `0042DFCC`: the warning runs while at most eight points remain.
const VIRUS_WARNING_POINTS: i32 = 8;
/// `0042DFEA`: each remaining point adds 20 world ticks between warnings.
const VIRUS_WARNING_TICKS_PER_POINT: i32 = 20;

/// Direct `00456900` requests; these use the ordinary replaceable text slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignCasualtyText {
    OneMoreLoss,
    WorldLost,
    /// `Warning: Virus level critical`.
    VirusCritical,
    /// `The virus went critical. World lost.`
    VirusWorldLost,
}

impl CampaignCasualtyText {
    pub const fn string_id(self) -> usize {
        match self {
            Self::OneMoreLoss => 0xc7,
            Self::WorldLost => 0xd4,
            Self::VirusCritical => 0xcb,
            Self::VirusWorldLost => 0xd5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignCasualtyAction {
    DirectText(CampaignCasualtyText),
    /// `0042DFD3`: resource event `0x12`, before the warning's own gates.
    VirusCriticalHint,
    /// `0042E01D`: the managed non-positional cue that follows text `0xCB`.
    VirusCriticalSound,
    /// Invoke the shared `00456960` transaction after the preceding text.
    /// The record's destination dword does not initiate a warp on this path.
    AbortWorld {
        record_index: usize,
    },
}

/// The selector's entry snapshot. Completion is `0042ED80`'s saved-world bit;
/// abort is the session `+0x28F` byte returned by `00456CB0`, not timer state 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CampaignCasualtyFrame {
    pub census: Fun0042e210CapabilityCensus,
    /// Every `0042E210` bucket count (controller `+0x178 + 8k`).
    pub capability_buckets: [i32; 14],
    /// `FUN_004366F0` over live `DAT_004dc684`; `None` without a loaded grid.
    pub virus_percent: Option<i32>,
    /// `FUN_00456DD0`: session `+0x2BC` is nonzero (a stamped completion or
    /// an unlocked Hive's -1), which silences the virus warning's text.
    pub results_stamped: bool,
    /// `DAT_004FED60`, the 50-Hz world clock.
    pub world_clock: i32,
    pub world_saved: bool,
    pub abort_active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsupportedCasualtyRecord {
    pub record_index: usize,
    pub flags: u32,
}

/// Controller `+0x1E8` and `+0x1E4`, both reset by each `0042E570` world load.
/// A warning sets the casualty latch even if the text slot declines it
/// because sticky D9 is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CampaignCasualtyState {
    warning_issued: bool,
    /// The world clock of the last virus warning text.
    virus_warning_clock: i32,
}

/// Reject an abort row this module cannot evaluate before any of its effects.
fn unsupported_abort_record(
    record_index: usize,
    record: &CampaignRecord,
    frame: CampaignCasualtyFrame,
) -> Result<(), UnsupportedCasualtyRecord> {
    let flags = record.flags() & 0xff;
    let unsupported = flags & ABORT_FLAG != 0
        && (flags & !SUPPORTED_ABORT_FLAGS != 0
            || flags & CENSUS_FLAG != 0 && census_bucket(record).is_none()
            || flags & VIRUS_FLAG != 0 && frame.virus_percent.is_none());
    if unsupported {
        return Err(UnsupportedCasualtyRecord {
            record_index,
            flags,
        });
    }
    Ok(())
}

/// `0042DFA7` indexes the bucket by record dword `+0x14` without a bound.
fn census_bucket(record: &CampaignRecord) -> Option<usize> {
    let bucket = u32::from_le_bytes(record.data[0x14..0x18].try_into().unwrap()) as usize;
    (bucket < 14).then_some(bucket)
}

impl CampaignCasualtyState {
    pub fn evaluate(
        &mut self,
        records: &[CampaignRecord],
        frame: CampaignCasualtyFrame,
    ) -> Result<Vec<CampaignCasualtyAction>, UnsupportedCasualtyRecord> {
        // 0042DE94..0042DEAA skips abort records on either condition. Neither clears
        // the shared warning latch; only world load does so.
        if frame.world_saved || frame.abort_active {
            return Ok(Vec::new());
        }

        // Preflight every row before publishing any effects.
        for (record_index, record) in records.iter().enumerate() {
            unsupported_abort_record(record_index, record, frame)?;
        }

        Ok(records
            .iter()
            .enumerate()
            .flat_map(|(index, record)| self.evaluate_record(index, record, frame))
            .collect())
    }

    /// One preflighted row: [`unsupported_abort_record`] has accepted it.
    fn evaluate_record(
        &mut self,
        record_index: usize,
        record: &CampaignRecord,
        frame: CampaignCasualtyFrame,
    ) -> Vec<CampaignCasualtyAction> {
        let mut actions = Vec::new();
        let flags = record.flags() & 0xff;
        if frame.world_saved || frame.abort_active || flags & ABORT_FLAG == 0 {
            return actions;
        }
        // 0042DF28 precedes the warning as well as the loss comparison.
        if flags & MINIMUM_PEASANTS_FLAG != 0
            && frame.census.cap_0x800 < i32::from(record.data[0x18] as i8)
        {
            return actions;
        }
        if flags & CASUALTY_FLAG != 0 {
            let survivors = frame.census.cap_0x400.wrapping_add(frame.census.cap_0x800);
            let threshold = i32::from(record.data[0x19] as i8);
            if survivors == threshold + 1 && !self.warning_issued {
                self.warning_issued = true;
                actions.push(CampaignCasualtyAction::DirectText(
                    CampaignCasualtyText::OneMoreLoss,
                ));
            }
            if survivors > threshold {
                return actions;
            }
            // 0042DF98 sends D4 before the remaining predicates.
            actions.push(CampaignCasualtyAction::DirectText(
                CampaignCasualtyText::WorldLost,
            ));
        }
        if flags & CENSUS_FLAG != 0 {
            let bucket = census_bucket(record).expect("preflighted census bucket");
            if frame.capability_buckets[bucket] > 0 {
                return actions;
            }
        }
        if flags & VIRUS_FLAG != 0 {
            let percent = frame.virus_percent.expect("preflighted virus census");
            let limit = i32::from(record.data[0x1a] as i8);
            let remaining = limit.wrapping_sub(percent);
            if remaining <= VIRUS_WARNING_POINTS {
                actions.push(CampaignCasualtyAction::VirusCriticalHint);
                let due = frame
                    .world_clock
                    .wrapping_sub(remaining.wrapping_mul(VIRUS_WARNING_TICKS_PER_POINT));
                if !frame.results_stamped && self.virus_warning_clock < due {
                    actions.push(CampaignCasualtyAction::DirectText(
                        CampaignCasualtyText::VirusCritical,
                    ));
                    actions.push(CampaignCasualtyAction::VirusCriticalSound);
                    self.virus_warning_clock = frame.world_clock;
                }
            }
            if percent < limit {
                return actions;
            }
            actions.push(CampaignCasualtyAction::DirectText(
                CampaignCasualtyText::VirusWorldLost,
            ));
        }
        // 0042E06F invokes the abort. The caller must deliver the preceding
        // text first.
        actions.push(CampaignCasualtyAction::AbortWorld { record_index });
        actions
    }
}

/// One source-ordered selector-zero visit. The entry saved/abort census snapshot
/// remains fixed while a yielded abort runs; retail resumes at the next record.
/// A successful CheckRoute terminates the caller's cursor immediately (42E099).
#[derive(Debug)]
pub struct CampaignSelectorCursor {
    frame: CampaignCasualtyFrame,
    next_record: usize,
    main_base_abort: bool,
    retry_current_world: bool,
    finished: bool,
    pending: std::collections::VecDeque<CampaignSelectorStep>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignSelectorStep {
    DirectText(CampaignCasualtyText),
    VirusCriticalHint,
    VirusCriticalSound,
    AbortMainBase,
    /// 42E06F after an abort row's predicates have passed.
    AbortRecord {
        record_index: usize,
    },
    /// 42DE20..43 returns controller+C4 without scanning authored records.
    RetryCurrentWorld,
    CheckRoute {
        record_index: usize,
    },
    Finished,
}

/// Controller requests sampled at selector entry. `+1F4` is raised by the
/// aborted wreck's `456D10`; `+1F8` is the separate Main Base loss request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CampaignSelectorEntry {
    pub main_base_request: bool,
    pub failed_world_interior_request: bool,
}

impl CampaignSelectorCursor {
    pub fn new(frame: CampaignCasualtyFrame, entry: CampaignSelectorEntry) -> Self {
        Self {
            frame,
            next_record: 0,
            // 42DE44 consumes +1F8 even when saved suppresses its abort.
            main_base_abort: entry.main_base_request && !frame.world_saved && !frame.abort_active,
            retry_current_world: entry.failed_world_interior_request && frame.abort_active,
            finished: false,
            pending: std::collections::VecDeque::new(),
        }
    }

    pub fn next_step(
        &mut self,
        state: &mut CampaignCasualtyState,
        records: &[CampaignRecord],
    ) -> Result<CampaignSelectorStep, UnsupportedCasualtyRecord> {
        if self.finished {
            return Ok(CampaignSelectorStep::Finished);
        }
        if self.retry_current_world {
            self.finished = true;
            return Ok(CampaignSelectorStep::RetryCurrentWorld);
        }
        if self.main_base_abort {
            self.finished = true; // 42DE64 aborts and returns before the record scan.
            return Ok(CampaignSelectorStep::AbortMainBase);
        }
        if let Some(step) = self.pending.pop_front() {
            return Ok(step);
        }
        while let Some(record) = records.get(self.next_record) {
            let record_index = self.next_record;
            self.next_record += 1;
            let flags = record.flags() & 0xff; // 42DE91 tests AL.
            if flags & ABORT_FLAG == 0 {
                // AuthoredCampaignRoute already authenticates all non-abort rows
                // as plain marker transitions before this world becomes live.
                return Ok(CampaignSelectorStep::CheckRoute { record_index });
            }
            if self.frame.world_saved || self.frame.abort_active {
                continue;
            }
            if let Err(error) = unsupported_abort_record(record_index, record, self.frame) {
                self.finished = true;
                return Err(error);
            }
            self.pending.extend(
                state
                    .evaluate_record(record_index, record, self.frame)
                    .into_iter()
                    .map(|action| match action {
                        CampaignCasualtyAction::DirectText(text) => {
                            CampaignSelectorStep::DirectText(text)
                        }
                        CampaignCasualtyAction::VirusCriticalHint => {
                            CampaignSelectorStep::VirusCriticalHint
                        }
                        CampaignCasualtyAction::VirusCriticalSound => {
                            CampaignSelectorStep::VirusCriticalSound
                        }
                        CampaignCasualtyAction::AbortWorld { record_index } => {
                            CampaignSelectorStep::AbortRecord { record_index }
                        }
                    }),
            );
            if let Some(step) = self.pending.pop_front() {
                return Ok(step);
            }
        }
        self.finished = true;
        Ok(CampaignSelectorStep::Finished)
    }
}

#[cfg(test)]
mod tests;
