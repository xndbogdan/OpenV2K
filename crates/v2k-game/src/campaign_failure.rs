//! The native-casualty predicates in campaign selector zero (`0042DD10`).
//!
//! Section-13 flag `0x04` compares the current scientist-plus-peasant census
//! with signed record byte `+0x19`. It is a minimum-survivor boundary, not a
//! subtraction from the load-time population. Sub-M occupants are already
//! included in the scientist bucket by `0042E210`.
//!
//! The single-player caller evaluates these predicates after the actor and
//! contact phases, at the same deferred campaign boundary as Main Base loss.
//! This module owns authored casualty-abort records (`0x84` / `0x86`) and
//! selector entry ordering, including the aborted interior's early current-
//! world return. Marker arrival, virus limits and the systemic `00456960`
//! body retain their respective owners. No actor is removed by this predicate.

use crate::world_complete_results::Fun0042e210CapabilityCensus;
use v2k_formats::levels::CampaignRecord;

const CASUALTY_ABORT_FLAGS: u32 = 0x84;
const MINIMUM_PEASANTS_FLAG: u32 = 0x02;

/// Direct `00456900` requests; these use the ordinary replaceable text slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignCasualtyText {
    OneMoreLoss,
    WorldLost,
}

impl CampaignCasualtyText {
    pub const fn string_id(self) -> usize {
        match self {
            Self::OneMoreLoss => 0xc7,
            Self::WorldLost => 0xd4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignCasualtyAction {
    DirectText(CampaignCasualtyText),
    /// Invoke the shared `00456960` transaction after the preceding D4 text.
    /// The record's destination dword does not initiate a warp on this path.
    AbortWorld {
        record_index: usize,
    },
}

/// The selector's entry snapshot. Completion is `0042ED80`'s saved-world bit;
/// abort is the session `+0x28F` byte returned by `00456CB0`, not timer state 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CampaignCasualtyFrame {
    pub census: Fun0042e210CapabilityCensus,
    pub world_saved: bool,
    pub abort_active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsupportedCasualtyRecord {
    pub record_index: usize,
    pub flags: u32,
}

/// Controller `+0x1E8`, reset by each `0042E570` world load. A warning sets
/// this latch even if the text slot declines it because sticky D9 is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CampaignCasualtyState {
    warning_issued: bool,
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

        // Preflight the bounded lane before publishing any effects. Mixed
        // casualty/time/marker/virus predicates cannot be reduced to a pure
        // population test. Every retail campaign casualty record is 84 or 86.
        for (record_index, record) in records.iter().enumerate() {
            let flags = record.flags() & 0xff;
            if flags & CASUALTY_ABORT_FLAGS == CASUALTY_ABORT_FLAGS
                && flags & !(CASUALTY_ABORT_FLAGS | MINIMUM_PEASANTS_FLAG) != 0
            {
                return Err(UnsupportedCasualtyRecord {
                    record_index,
                    flags,
                });
            }
        }

        Ok(records
            .iter()
            .enumerate()
            .flat_map(|(index, record)| self.evaluate_record(index, record, frame))
            .collect())
    }

    fn evaluate_record(
        &mut self,
        record_index: usize,
        record: &CampaignRecord,
        frame: CampaignCasualtyFrame,
    ) -> Vec<CampaignCasualtyAction> {
        if frame.world_saved || frame.abort_active {
            return Vec::new();
        }
        let survivors = frame.census.cap_0x400.wrapping_add(frame.census.cap_0x800);
        let mut actions = Vec::new();
        let flags = record.flags() & 0xff;
        if flags & CASUALTY_ABORT_FLAGS != CASUALTY_ABORT_FLAGS {
            return actions;
        }
        // 0042DF28 precedes the warning as well as the loss comparison.
        if flags & MINIMUM_PEASANTS_FLAG != 0
            && frame.census.cap_0x800 < i32::from(record.data[0x18] as i8)
        {
            return actions;
        }
        let threshold = i32::from(record.data[0x19] as i8);
        if survivors == threshold + 1 && !self.warning_issued {
            self.warning_issued = true;
            actions.push(CampaignCasualtyAction::DirectText(
                CampaignCasualtyText::OneMoreLoss,
            ));
        }
        if survivors <= threshold {
            // 0042DF98 sends D4; 0042E06F invokes the abort. The caller
            // must preserve that order when delivering this result.
            actions.push(CampaignCasualtyAction::DirectText(
                CampaignCasualtyText::WorldLost,
            ));
            actions.push(CampaignCasualtyAction::AbortWorld { record_index });
        }
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
    AbortMainBase,
    AbortCasualties {
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
            if flags & 0x80 == 0 {
                // AuthoredCampaignRoute already authenticates all non-abort rows
                // as plain marker transitions before this world becomes live.
                return Ok(CampaignSelectorStep::CheckRoute { record_index });
            }
            if self.frame.world_saved || self.frame.abort_active {
                continue;
            }
            if flags & CASUALTY_ABORT_FLAGS != CASUALTY_ABORT_FLAGS {
                continue;
            }
            if flags & !(CASUALTY_ABORT_FLAGS | MINIMUM_PEASANTS_FLAG) != 0 {
                self.finished = true;
                return Err(UnsupportedCasualtyRecord {
                    record_index,
                    flags,
                });
            }
            self.pending.extend(
                state
                    .evaluate_record(record_index, record, self.frame)
                    .into_iter()
                    .map(|action| match action {
                        CampaignCasualtyAction::DirectText(text) => {
                            CampaignSelectorStep::DirectText(text)
                        }
                        CampaignCasualtyAction::AbortWorld { record_index } => {
                            CampaignSelectorStep::AbortCasualties { record_index }
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
