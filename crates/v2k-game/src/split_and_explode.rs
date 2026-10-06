//! Class18 `FUN_0040C080` synchronous split-and-explode initializer.
//!
//! This is the shared initializer kernel, not Type40/56 actor admission. The
//! host must supply real callback/allocation custody and publish each completed
//! child before returning: `14AE0` reads its successor after this callback and
//! can damage newly appended children during the same blast. PE `40C080..40C372`
//! owns the order and arithmetic; `ALPINE_INSECTS.md` owns the recording bounds.

/// C080's signed limit applies to the exact native `468D00` result.
pub const SPLIT_NATIVE_LIST_LIMIT: i32 = 80;
pub const SPLIT_BIRTH_RECORD_BYTES: usize = 0x4c;

/// Policy words read once after `40A860` has released slots0/1/2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitSource {
    pub entity_type: u32,
    /// Current null/default handle value `DAT_004DCA00`, copied to birth+00.
    /// `104B0` allocates a fresh handle when zero; otherwise it requests that
    /// explicit handle via43A3C0. This is a value, never the global address.
    pub requested_entity_handle_raw: u32,
}

/// Refresh these parent words for every child, after the preceding constructor.
/// A nested constructor may mutate the parent; the launch stream may not reuse
/// the first child's position, heading or objective snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitLaunchSource {
    pub position_raw: [i16; 3],
    pub heading_raw: u16,
    /// Parent state `+08 & 01000000`, written as boolean birth-record `+14`.
    pub objective: bool,
}

/// Remote parents still release tasks and mark deferred destruction, but do
/// not read local birth geometry or count allocations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitSourceOwnership {
    Local(SplitSource),
    Remote,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitChildRequest {
    pub requested_entity_handle_raw: u32,
    pub entity_type: u32,
    pub position_raw: [i16; 3],
    pub objective: bool,
    pub velocity_raw: [i16; 3],
    /// Birth-record `+24/+26/+28`: heading, pitch, roll, in that draw order.
    pub rotation_heading_pitch_roll_raw: [u16; 3],
}

impl SplitChildRequest {
    /// The constructor consumes a zeroed `0x4C` record with these fields set.
    /// All unowned instance fields remain zero; this never alters an OVL.
    pub fn to_native_record(self) -> [u8; SPLIT_BIRTH_RECORD_BYTES] {
        let mut record = [0; SPLIT_BIRTH_RECORD_BYTES];
        record[0..4].copy_from_slice(&self.requested_entity_handle_raw.to_le_bytes());
        record[8..12].copy_from_slice(&self.entity_type.to_le_bytes());
        for (index, value) in self.position_raw.into_iter().enumerate() {
            record[0x0c + index * 2..0x0e + index * 2].copy_from_slice(&value.to_le_bytes());
        }
        record[0x14..0x18].copy_from_slice(&u32::from(self.objective).to_le_bytes());
        for (index, value) in self.velocity_raw.into_iter().enumerate() {
            record[0x18 + index * 2..0x1a + index * 2].copy_from_slice(&value.to_le_bytes());
        }
        for (index, value) in self.rotation_heading_pitch_roll_raw.into_iter().enumerate() {
            record[0x24 + index * 2..0x26 + index * 2].copy_from_slice(&value.to_le_bytes());
        }
        record
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum SplitChildAttempt<E> {
    /// `438080` returned zero. The host has linked the actual native allocation
    /// and published its constructor/task receipt synchronously.
    Created,
    /// `438080` returned this error object; `4575A0` disposes it before stopping
    /// the remaining loop. This is separate from an unowned host continuation.
    NativeError(E),
}

/// One host owns all process state, including constructor RNG. Each successful
/// method completes its native phase synchronously. A host error may retain an
/// internal committed prefix; the execution then parks permanently, so neither
/// that phase nor prior RNG/effects are replayed. The host retains any partial
/// constructor/disposal custody when returning its own error.
pub trait SplitAndExplodeHost {
    type Block;
    type ConstructorError;

    /// `440950`, with the selected model extent and source component policy.
    /// Its nonzero native return exits C080 before task release or destruction.
    fn emit_split_burst(&mut self, source_id: u32) -> Result<u32, Self::Block>;
    /// `40A860` releases all three physical task slots in their native order.
    fn clear_source_tasks(&mut self, source_id: u32) -> Result<(), Self::Block>;
    fn source_after_task_clear(
        &mut self,
        source_id: u32,
    ) -> Result<SplitSourceOwnership, Self::Block>;
    /// Exact `414920 -> 468D00` native traversal result. It skips the first
    /// actor and counts the terminal sentinel, so its numeric result equals
    /// the full actor allocation count: include deferred parents and cargo,
    /// and do not add an extra sentinel. Active-only or per-family counts are
    /// incorrect. This count is read once before the child-constructor loop.
    fn native_entity_list_count_raw(&mut self) -> Result<i32, Self::Block>;
    fn current_launch_source(&mut self, source_id: u32) -> Result<SplitLaunchSource, Self::Block>;
    /// This primitive mutates only the shared RNG, not source geometry/state.
    fn next_shared_random_u16(&mut self) -> u16;
    fn construct_split_child(
        &mut self,
        request: SplitChildRequest,
    ) -> Result<SplitChildAttempt<Self::ConstructorError>, Self::Block>;
    fn dispose_constructor_error(
        &mut self,
        error: Self::ConstructorError,
    ) -> Result<(), Self::Block>;
    /// `410B70`: retain the parent link, clear60000/set100000 and stage its
    /// idempotent deferred removal. Do not splice it out inside this callback.
    fn mark_source_deferred_destroy(&mut self, source_id: u32) -> Result<(), Self::Block>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitAndExplodePhase {
    Burst,
    TaskClear,
    SourceOwnership,
    GlobalCount,
    LaunchSource,
    ChildConstruction,
    ErrorDisposal,
    DeferredDestroy,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SplitAndExplodeProgress {
    pub burst_return_raw: Option<u32>,
    pub tasks_cleared: bool,
    /// Exact `468D00` result under its native start/terminal traversal policy.
    pub native_entity_list_count_raw: Option<i32>,
    pub permitted_children: usize,
    pub attempted_children: usize,
    pub created_children: usize,
    /// Only C080's launch prefix; a child constructor owns its separate draws.
    pub launch_rng_words: usize,
    pub constructor_error_disposed: bool,
    pub source_deferred_destroyed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitAndExplodeCompletion {
    /// Propagate `440950`'s native nonzero return without running the suffix.
    BurstReturned(u32),
    Remote,
    ChildrenComplete,
    NativeConstructorError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SplitAndExplodeBlock<B> {
    /// This initializer already completed or retained a blocked host prefix.
    /// Re-running it would repeat damage, allocation counters or process RNG.
    AlreadyVisited,
    Host {
        phase: SplitAndExplodePhase,
        reason: B,
    },
}

/// Linear initializer custody. It is deliberately not Clone: both completion
/// and a host block consume the visit, while `progress` retains known prefixes.
#[derive(Debug)]
pub struct SplitAndExplodeExecution {
    source_id: u32,
    visited: bool,
    progress: SplitAndExplodeProgress,
}

impl SplitAndExplodeExecution {
    pub const fn new(source_id: u32) -> Self {
        Self {
            source_id,
            visited: false,
            progress: SplitAndExplodeProgress {
                burst_return_raw: None,
                tasks_cleared: false,
                native_entity_list_count_raw: None,
                permitted_children: 0,
                attempted_children: 0,
                created_children: 0,
                launch_rng_words: 0,
                constructor_error_disposed: false,
                source_deferred_destroyed: false,
            },
        }
    }

    pub const fn progress(&self) -> SplitAndExplodeProgress {
        self.progress
    }

    pub fn execute<H: SplitAndExplodeHost>(
        &mut self,
        host: &mut H,
    ) -> Result<SplitAndExplodeCompletion, SplitAndExplodeBlock<H::Block>> {
        use SplitAndExplodePhase as Phase;
        if self.visited {
            return Err(SplitAndExplodeBlock::AlreadyVisited);
        }
        self.visited = true;
        let host_block = |phase, reason| SplitAndExplodeBlock::Host { phase, reason };
        let burst_return = host
            .emit_split_burst(self.source_id)
            .map_err(|reason| host_block(Phase::Burst, reason))?;
        self.progress.burst_return_raw = Some(burst_return);
        if burst_return != 0 {
            return Ok(SplitAndExplodeCompletion::BurstReturned(burst_return));
        }
        host.clear_source_tasks(self.source_id)
            .map_err(|reason| host_block(Phase::TaskClear, reason))?;
        self.progress.tasks_cleared = true;
        let source = host
            .source_after_task_clear(self.source_id)
            .map_err(|reason| host_block(Phase::SourceOwnership, reason))?;
        let completion = if let SplitSourceOwnership::Local(source) = source {
            let native_count = host
                .native_entity_list_count_raw()
                .map_err(|reason| host_block(Phase::GlobalCount, reason))?;
            self.progress.native_entity_list_count_raw = Some(native_count);
            let (child_type, requested) = split_policy(source.entity_type);
            // PE40C235..40C249 uses signed comparisons after wrapping x86 ADD.
            let permitted = if native_count.wrapping_add(requested) > SPLIT_NATIVE_LIST_LIMIT {
                SPLIT_NATIVE_LIST_LIMIT.wrapping_sub(native_count)
            } else {
                requested
            };
            self.progress.permitted_children = permitted.max(0) as usize;
            let mut completion = SplitAndExplodeCompletion::ChildrenComplete;
            for _ in 0..self.progress.permitted_children {
                let launch_source = host
                    .current_launch_source(self.source_id)
                    .map_err(|reason| host_block(Phase::LaunchSource, reason))?;
                let words = std::array::from_fn(|_| host.next_shared_random_u16());
                self.progress.attempted_children += 1;
                self.progress.launch_rng_words += 9;
                let request = child_request(source, launch_source, child_type, words);
                match host
                    .construct_split_child(request)
                    .map_err(|reason| host_block(Phase::ChildConstruction, reason))?
                {
                    SplitChildAttempt::Created => self.progress.created_children += 1,
                    SplitChildAttempt::NativeError(error) => {
                        host.dispose_constructor_error(error)
                            .map_err(|reason| host_block(Phase::ErrorDisposal, reason))?;
                        self.progress.constructor_error_disposed = true;
                        completion = SplitAndExplodeCompletion::NativeConstructorError;
                        break;
                    }
                }
            }
            completion
        } else {
            SplitAndExplodeCompletion::Remote
        };
        host.mark_source_deferred_destroy(self.source_id)
            .map_err(|reason| host_block(Phase::DeferredDestroy, reason))?;
        self.progress.source_deferred_destroyed = true;
        Ok(completion)
    }
}

fn split_policy(parent_type: u32) -> (u32, i32) {
    match parent_type {
        27 => (3, 2),
        31 => (105, 8),
        34 => (4, 2),
        40 => (56, 8),
        other => (other, 2),
    }
}

fn child_request(
    source: SplitSource,
    launch: SplitLaunchSource,
    entity_type: u32,
    words: [u16; 9],
) -> SplitChildRequest {
    SplitChildRequest {
        requested_entity_handle_raw: source.requested_entity_handle_raw,
        entity_type,
        position_raw: std::array::from_fn(|index| {
            launch.position_raw[index].wrapping_add(((words[index] >> 6) as i16).wrapping_sub(512))
        }),
        objective: launch.objective,
        velocity_raw: [
            ((words[3] >> 5) as i16).wrapping_sub(1024),
            (words[4] >> 6) as i16,
            ((words[5] >> 5) as i16).wrapping_sub(1024),
        ],
        rotation_heading_pitch_roll_raw: [
            launch
                .heading_raw
                .wrapping_add((words[6] >> 4).wrapping_sub(2048)),
            (words[7] >> 4).wrapping_sub(2048),
            (words[8] >> 4).wrapping_sub(2048),
        ],
    }
}

#[cfg(test)]
mod tests;
