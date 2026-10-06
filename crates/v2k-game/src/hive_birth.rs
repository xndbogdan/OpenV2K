//! Authored Sub-N creature rows (`41C32D..41C546`) and ejection custody.
//!
//! The component caller runs the rows after its live-health and class5 work,
//! then runs `advance_ejections` after the contact walk. Neither lane depends
//! on detailed/coarse update mode. All handles retain allocation identity.

use std::collections::VecDeque;

use v2k_formats::levels::EntityAnimation;

const DYING_STATE_BIT: u32 = 0x4000;
pub const HIVE_EJECTION_DELAY_US: i32 = 500_000;

/// Exact seven dwords consumed from one authored `0x1C`-byte record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HiveBirthRecord {
    pub entity_type: u32,
    pub initial_timer_us: i32,
    pub delay_base_us: i32,
    pub jitter_operand: i32,
    pub total_production_cap: i32,
    pub retained_child_cap: i32,
    pub flags: u32,
}

impl HiveBirthRecord {
    pub fn decode(raw: [u8; 0x1C]) -> Self {
        let word =
            |index: usize| u32::from_le_bytes(raw[index * 4..index * 4 + 4].try_into().unwrap());
        Self {
            entity_type: word(0),
            initial_timer_us: word(1) as i32,
            delay_base_us: word(2) as i32,
            jitter_operand: word(3) as i32,
            total_production_cap: word(4) as i32,
            retained_child_cap: word(5) as i32,
            flags: word(6),
        }
    }

    pub const fn selects_session(self, session_aborted: bool) -> bool {
        (self.flags & 2 != 0) == session_aborted
    }

    pub const fn objective(self) -> bool {
        self.flags & 1 != 0
    }

    /// PE `41C517..41C521`: low-dword IMUL, signed SAR16, wrapping ADD.
    pub fn delay_after_draw(self, random: u16) -> i32 {
        self.delay_base_us
            .wrapping_add(i32::from(random).wrapping_mul(self.jitter_operand) >> 16)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HiveBirthDecodeError {
    pub authored_count: i32,
    pub decoded_count: usize,
}

/// Retail runtime record: timer, successful-production count and child FIFO.
/// The original three FIFO words occupy the remaining bytes of its 20 bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HiveBirthRowRuntime {
    record: HiveBirthRecord,
    timer_us: i32,
    produced_count: i32,
    children: VecDeque<u32>,
    /// An unowned callback failed after reaching its constructor prefix.
    /// Retain custody explicitly; retrying would replay process RNG/counters.
    constructor_parked: bool,
}

impl HiveBirthRowRuntime {
    pub const fn record(&self) -> HiveBirthRecord {
        self.record
    }

    pub const fn timer_us(&self) -> i32 {
        self.timer_us
    }

    pub const fn produced_count(&self) -> i32 {
        self.produced_count
    }

    pub fn children(&self) -> &VecDeque<u32> {
        &self.children
    }

    pub const fn constructor_parked(&self) -> bool {
        self.constructor_parked
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HiveEjectionSlot {
    pub child_handle: Option<u32>,
    pub timer_us: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HiveBirthVisit {
    /// The current controller word after this callback's health/radial prefix.
    pub controller_state: u32,
    pub session_aborted: bool,
    pub elapsed_us: u32,
    pub source_id: u32,
    pub source_position_raw: [i16; 3],
}

/// The zeroed `38080` request's populated creature fields. Source identity is
/// used by the successful `+0x60` suffix, independently of relation `+0x80`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HiveBirthRequest {
    pub entity_type: u32,
    pub position_raw: [i16; 3],
    pub objective: bool,
    pub source_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HiveBirthAttempt<B = ()> {
    /// The host has published the complete native allocation/task custody.
    Created(u32),
    /// The host has dispatched the returned retail constructor error. Retail
    /// still resets the timer and consumes its delay draw on this branch.
    NativeError,
    /// A defensive unowned suffix retained a native constructor prefix. Park
    /// this row without a reset draw; later rows and final ejections continue.
    CommittedPrefixBlock(B),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HiveEjectionRequest {
    pub source_id: u32,
    pub child_handle: u32,
}

/// Fresh `1C830` source geometry, including Sub-N's retained birth anchor even
/// when its independent marker/contact flag is clear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HiveEjectionGeometry {
    pub anchor_raw: [i16; 3],
    pub model_extent_raw: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HiveChildEjection {
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub rotation_raw: [u16; 3],
}

/// One ordered host for native construction, current allocation lookups and
/// the process-wide RNG. A Created return authenticates the infallible child
/// publication methods. `prepare_hive_ejection` authenticates both handles,
/// current selected model and complete terrain before any ejection prefix.
pub trait HiveBirthHost {
    type Block;

    /// `Some(0)` is a present allocation, not the `3A580` miss branch.
    fn child_state_flags(&mut self, child_handle: u32) -> Result<Option<u32>, Self::Block>;
    /// An unowned-feature `Err` must precede allocation/counter/RNG mutation.
    /// A reached retail constructor failure is `Ok(NativeError)` instead,
    /// preserving its native prefix and the row's mandatory reset suffix.
    fn construct_hive_child(
        &mut self,
        request: HiveBirthRequest,
    ) -> Result<HiveBirthAttempt<Self::Block>, Self::Block>;
    fn publish_hive_child_source(&mut self, child_handle: u32, source_id: u32);
    fn prepare_hive_ejection(
        &mut self,
        request: HiveEjectionRequest,
    ) -> Result<HiveEjectionGeometry, Self::Block>;
    fn sample_hive_ejection_terrain_height_raw(&self, x_raw: i16, z_raw: i16) -> i16;
    /// Clears child state8000 and publishes the completed raw launch words.
    fn publish_hive_child_ejection(&mut self, child_handle: u32, ejection: HiveChildEjection);
    fn restore_hive_child_pairs(&mut self, child_handle: u32);
    fn next_shared_random_u16(&mut self) -> u16;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HiveEjectionBlock<B> {
    Host(B),
    ZeroModelExtent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HiveBirthBlock<B> {
    Row {
        row_index: usize,
        block: B,
    },
    Ejection {
        child_handle: u32,
        block: HiveEjectionBlock<B>,
    },
    EjectionTimer {
        slot_index: usize,
        block: B,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HiveEjectionOutcome {
    AlreadyRecorded,
    SlotsOccupied,
    Launched { slot_index: usize },
}

/// Loader-owned `1BC20` creature runtime and the two `1C830/1CA90` slots.
/// Alternate abort cleanup leaves this object intact.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HiveBirthRuntime {
    rows: Vec<HiveBirthRowRuntime>,
    ejection_slots: [HiveEjectionSlot; 2],
}

impl HiveBirthRuntime {
    pub fn from_animation(animation: &EntityAnimation) -> Result<Self, HiveBirthDecodeError> {
        let authored_count = i32::from_le_bytes(animation.header[16..20].try_into().unwrap());
        if authored_count < 0 || authored_count as usize != animation.frames.len() {
            return Err(HiveBirthDecodeError {
                authored_count,
                decoded_count: animation.frames.len(),
            });
        }
        Ok(Self {
            rows: animation
                .frames
                .iter()
                .copied()
                .map(HiveBirthRecord::decode)
                .map(|record| HiveBirthRowRuntime {
                    timer_us: record.initial_timer_us,
                    record,
                    produced_count: 0,
                    children: VecDeque::new(),
                    constructor_parked: false,
                })
                .collect(),
            ejection_slots: [HiveEjectionSlot::default(); 2],
        })
    }

    pub fn rows(&self) -> &[HiveBirthRowRuntime] {
        &self.rows
    }

    pub const fn ejection_slots(&self) -> &[HiveEjectionSlot; 2] {
        &self.ejection_slots
    }

    /// `41C32D..41C546`. Feature blocks retain the affected row's progress and
    /// leave its expired timer without a reset draw; later rows still execute.
    pub fn advance_rows<H: HiveBirthHost>(
        &mut self,
        visit: HiveBirthVisit,
        host: &mut H,
    ) -> Vec<HiveBirthBlock<H::Block>> {
        let mut blocks = Vec::new();
        if visit.controller_state != 1 {
            return blocks;
        }
        let elapsed_us = visit.elapsed_us as i32;
        for row_index in 0..self.rows.len() {
            let row = &mut self.rows[row_index];
            if row.constructor_parked {
                continue;
            }
            if !row.record.selects_session(visit.session_aborted) {
                continue;
            }
            if row.timer_us >= elapsed_us {
                row.timer_us = row.timer_us.wrapping_sub(elapsed_us);
                continue;
            }
            if row.record.total_production_cap > 0
                && row.produced_count >= row.record.total_production_cap
            {
                continue;
            }
            let mut count = row.children.len() as i32;
            let mut row_block = None;
            if count != 0 && count >= row.record.retained_child_cap {
                let mut index = 0;
                while index < row.children.len() {
                    match host.child_state_flags(row.children[index]) {
                        Ok(None) => {
                            row.children.remove(index);
                            count = count.wrapping_sub(1);
                        }
                        Ok(Some(flags)) if flags & DYING_STATE_BIT != 0 => {
                            row.children.remove(index);
                            count = count.wrapping_sub(1);
                        }
                        Ok(Some(_)) => index += 1,
                        Err(block) => {
                            row_block = Some(block);
                            break;
                        }
                    }
                }
            }
            if let Some(block) = row_block {
                blocks.push(HiveBirthBlock::Row { row_index, block });
                continue;
            }
            if count >= row.record.retained_child_cap {
                continue;
            }
            let request = HiveBirthRequest {
                entity_type: row.record.entity_type,
                position_raw: visit.source_position_raw,
                objective: row.record.objective(),
                source_id: visit.source_id,
            };
            let attempt = match host.construct_hive_child(request) {
                Ok(attempt) => attempt,
                Err(block) => {
                    blocks.push(HiveBirthBlock::Row { row_index, block });
                    continue;
                }
            };
            if let HiveBirthAttempt::CommittedPrefixBlock(block) = attempt {
                row.constructor_parked = true;
                blocks.push(HiveBirthBlock::Row { row_index, block });
                continue;
            }
            if let HiveBirthAttempt::Created(child_handle) = attempt {
                host.publish_hive_child_source(child_handle, visit.source_id);
                row.children.push_back(child_handle);
                row.produced_count = row.produced_count.wrapping_add(1);
                if let Err(block) = self.eject_child(
                    HiveEjectionRequest {
                        source_id: visit.source_id,
                        child_handle,
                    },
                    host,
                ) {
                    // The successful construction/list/count prefix remains
                    // owned. Only unowned ejection is skipped; resetting this
                    // row still prevents replaying that native birth prefix.
                    blocks.push(HiveBirthBlock::Ejection {
                        child_handle,
                        block,
                    });
                }
            }
            let random = host.next_shared_random_u16();
            let row = &mut self.rows[row_index];
            row.timer_us = row.record.delay_after_draw(random);
        }
        blocks
    }

    /// `1C830`: duplicate detection, first unresolved slot, raw launch then
    /// return. Full slots consume no RNG and do not reposition the child.
    pub fn eject_child<H: HiveBirthHost>(
        &mut self,
        request: HiveEjectionRequest,
        host: &mut H,
    ) -> Result<HiveEjectionOutcome, HiveEjectionBlock<H::Block>> {
        if self
            .ejection_slots
            .iter()
            .any(|slot| slot.child_handle == Some(request.child_handle))
        {
            return Ok(HiveEjectionOutcome::AlreadyRecorded);
        }
        let mut free_slot = None;
        for (index, slot) in self.ejection_slots.iter().enumerate() {
            let present = match slot.child_handle {
                Some(handle) => host
                    .child_state_flags(handle)
                    .map_err(HiveEjectionBlock::Host)?
                    .is_some(),
                None => false,
            };
            if !present {
                free_slot = Some(index);
                break;
            }
        }
        let Some(slot_index) = free_slot else {
            return Ok(HiveEjectionOutcome::SlotsOccupied);
        };
        let geometry = host
            .prepare_hive_ejection(request)
            .map_err(HiveEjectionBlock::Host)?;
        if geometry.model_extent_raw == 0 {
            return Err(HiveEjectionBlock::ZeroModelExtent);
        }
        self.ejection_slots[slot_index] = HiveEjectionSlot {
            child_handle: Some(request.child_handle),
            timer_us: HIVE_EJECTION_DELAY_US,
        };
        let side_draw = host.next_shared_random_u16();
        let right = side_draw & 1 != 0;
        let x_offset = (side_draw % geometry.model_extent_raw) as i16;
        let x_raw = if right {
            geometry.anchor_raw[0].wrapping_add(x_offset)
        } else {
            geometry.anchor_raw[0].wrapping_sub(x_offset)
        };
        let z_raw = geometry.anchor_raw[2]
            .wrapping_add(geometry.model_extent_raw as i16)
            .wrapping_add(100);
        let x_speed = (host.next_shared_random_u16() & 0xff) as i16;
        let y_raw = host
            .sample_hive_ejection_terrain_height_raw(x_raw, z_raw)
            .wrapping_add(400);
        let z_speed = (host.next_shared_random_u16() & 0xff) as i16;
        host.publish_hive_child_ejection(
            request.child_handle,
            HiveChildEjection {
                position_raw: [x_raw, y_raw, z_raw],
                velocity_raw: [if right { x_speed } else { -x_speed }, 850, z_speed],
                rotation_raw: [if right { 0 } else { 0x8000 }, 0, 0],
            },
        );
        Ok(HiveEjectionOutcome::Launched { slot_index })
    }

    /// Final `1CA90`, called once after this callback's contact walk. A new
    /// launch already ages by this callback's delta; equality restores8000.
    pub fn advance_ejections<H: HiveBirthHost>(
        &mut self,
        elapsed_us: u32,
        host: &mut H,
    ) -> Vec<HiveBirthBlock<H::Block>> {
        let elapsed_us = elapsed_us as i32;
        let mut blocks = Vec::new();
        for (slot_index, slot) in self.ejection_slots.iter_mut().enumerate() {
            if slot.timer_us == 0 {
                continue;
            }
            let child = slot.child_handle.expect("active ejection owns a handle");
            match host.child_state_flags(child) {
                Ok(None) => *slot = HiveEjectionSlot::default(),
                Ok(Some(_)) if elapsed_us < slot.timer_us => {
                    slot.timer_us = slot.timer_us.wrapping_sub(elapsed_us);
                }
                Ok(Some(_)) => {
                    host.restore_hive_child_pairs(child);
                    *slot = HiveEjectionSlot::default();
                }
                Err(block) => {
                    blocks.push(HiveBirthBlock::EjectionTimer { slot_index, block });
                }
            }
        }
        blocks
    }
}

#[cfg(test)]
mod tests;
