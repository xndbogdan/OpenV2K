//! Exact ownership and mutation seam for the retail three-slot actor task table.
//!
//! The model is bounded by the recovered behavior of `FUN_00401020`,
//! `FUN_004010D0`, `FUN_00401120`, `FUN_0040A7A0`, and `FUN_0040A800`:
//!
//! - an entity has exactly three physical task slots, visited in order 0, 1,
//!   then 2;
//! - each slot is read immediately before it is visited, so a task may replace
//!   or clear a later slot during the same pass;
//! - a replacement task is allocated and initialized before the old slot is
//!   destroyed;
//! - clearing or replacing a currently executing task destroys its inner state
//!   immediately but defers reclaiming the outer wrapper until the callback
//!   unwinds; and
//! - an initialization failure preserves the destination slot, while mutations
//!   already completed earlier in an ordered plan remain committed.
//!
//! This module intentionally does not interpret retail callback addresses or
//! execute actor movement. It only provides the ownership seam into which
//! proven task programs can later be connected.

use std::collections::BTreeMap;

pub const ACTOR_TASK_SLOT_COUNT: usize = 3;
pub const TASK_WRAPPER_CONSTRUCTOR_ADDRESS: u32 = 0x0040_1020;
pub const TASK_WRAPPER_DESTROY_ADDRESS: u32 = 0x0040_10D0;
pub const TASK_WRAPPER_TICK_ADDRESS: u32 = 0x0040_1120;
pub const TASK_SLOT_REPLACE_ADDRESS: u32 = 0x0040_A7A0;
pub const TASK_OWNER_TICK_ADDRESS: u32 = 0x0040_A800;

/// Physical index in the retail actor's three-entry task table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ActorTaskSlot {
    Primary = 0,
    Secondary = 1,
    Tertiary = 2,
}

impl ActorTaskSlot {
    pub const IN_RETAIL_TICK_ORDER: [Self; ACTOR_TASK_SLOT_COUNT] =
        [Self::Primary, Self::Secondary, Self::Tertiary];

    /// Retail-ordered suffix beginning at this freshly re-read slot.
    pub const fn retail_tick_suffix(self) -> &'static [Self] {
        match self {
            Self::Primary => &Self::IN_RETAIL_TICK_ORDER,
            Self::Secondary => &[Self::Secondary, Self::Tertiary],
            Self::Tertiary => &[Self::Tertiary],
        }
    }

    const fn index(self) -> usize {
        self as usize
    }
}

/// Stable identity for one outer retail-style task wrapper.
///
/// IDs are internal runtime identities, not retail pointers or entity handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActorTaskId(u64);

/// The two meaningful bytes in the retail eight-byte outer wrapper.
///
/// Retail leaves the upper two bytes of the wrapper's second dword
/// uninitialized. Keeping only these two flags prevents callers from treating
/// that padding as state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorTaskWrapperFlags {
    pub alive: bool,
    pub in_callback: bool,
}

/// A task whose allocation and initializer have already succeeded.
///
/// Construct this only after task-family-specific validation/initialization.
/// Consuming a `PreparedActorTask` in `replace_prepared` preserves retail's
/// prepare-before-destroy ordering.
#[derive(Debug)]
pub struct PreparedActorTask<T> {
    state: T,
}

impl<T> PreparedActorTask<T> {
    pub fn new(state: T) -> Self {
        Self { state }
    }

    /// Transform a prepared inner state without reopening allocation or
    /// initializer failure boundaries.
    ///
    /// This is primarily used by the heterogeneous dispatcher: a
    /// family-specific constructor can retain its exact preparation
    /// transaction, then wrap the finished state in the shared runtime enum
    /// before publication.
    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> PreparedActorTask<U> {
        PreparedActorTask {
            state: map(self.state),
        }
    }
}

#[derive(Debug)]
struct ActorTaskWrapper<T> {
    flags: ActorTaskWrapperFlags,
    /// `None` is the retail state after `FUN_004010D0` has destroyed/freed the
    /// inner 0x34-byte task state but the executing outer wrapper is still
    /// waiting to be reclaimed.
    state: Option<T>,
}

/// One structural action in a retail-ordered task transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActorTaskMutation<S> {
    Clear {
        slot: ActorTaskSlot,
    },
    TryInstall {
        slot: ActorTaskSlot,
        specification: S,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorTaskPrepareError<E> {
    pub action_index: usize,
    pub slot: ActorTaskSlot,
    pub error: E,
}

/// Identity supplied to the structural visitor for one executing slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorTaskVisit {
    pub slot: ActorTaskSlot,
    pub task_id: ActorTaskId,
}

/// Structural equivalent of the two scheduler outcomes relevant to the owner.
///
/// This is not an interpretation of the retail `0x9C00..0x9C02` result objects.
/// A future proven callback bridge may map its decoded result onto this enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActorTaskVisitControl<R> {
    Continue,
    Propagate(R),
}

/// Bounded owner for an actor's three retail task slots.
#[derive(Debug)]
pub struct ActorTaskOwner<T> {
    slots: [Option<ActorTaskId>; ACTOR_TASK_SLOT_COUNT],
    wrappers: BTreeMap<ActorTaskId, ActorTaskWrapper<T>>,
    next_task_id: u64,
}

impl<T> Default for ActorTaskOwner<T> {
    fn default() -> Self {
        Self {
            slots: [None; ACTOR_TASK_SLOT_COUNT],
            wrappers: BTreeMap::new(),
            // Zero remains available as an obvious invalid value in debugger
            // output even though `ActorTaskId` does not expose its field.
            next_task_id: 1,
        }
    }
}

impl<T> ActorTaskOwner<T> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Duplicate this owner only for the isolated Main Base abort transaction.
    ///
    /// The public task API deliberately keeps wrapper custody linear.  The
    /// abort adapter needs a speculative copy so a late unresolved callback
    /// can discard every earlier mutation.  Its transaction wrapper keeps the
    /// original manager inaccessible while this fork exists and either swaps
    /// the complete fork into production or drops it without exposing either
    /// set of wrapper identities.
    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self
    where
        T: Clone,
    {
        Self {
            slots: self.slots,
            wrappers: self
                .wrappers
                .iter()
                .map(|(&task_id, wrapper)| {
                    (
                        task_id,
                        ActorTaskWrapper {
                            flags: wrapper.flags,
                            state: wrapper.state.clone(),
                        },
                    )
                })
                .collect(),
            next_task_id: self.next_task_id,
        }
    }

    pub fn task_in_slot(&self, slot: ActorTaskSlot) -> Option<ActorTaskId> {
        self.slots[slot.index()]
    }

    pub fn state_in_slot(&self, slot: ActorTaskSlot) -> Option<&T> {
        self.task_in_slot(slot)
            .and_then(|task_id| self.task_state(task_id))
    }

    /// Returns live inner state. A self-cleared executing wrapper remains
    /// inspectable through `wrapper_flags`, but its inner state is already gone.
    pub fn task_state(&self, task_id: ActorTaskId) -> Option<&T> {
        self.wrappers
            .get(&task_id)
            .and_then(|wrapper| wrapper.state.as_ref())
    }

    pub fn task_state_mut(&mut self, task_id: ActorTaskId) -> Option<&mut T> {
        self.wrappers
            .get_mut(&task_id)
            .and_then(|wrapper| wrapper.state.as_mut())
    }

    pub fn wrapper_flags(&self, task_id: ActorTaskId) -> Option<ActorTaskWrapperFlags> {
        self.wrappers.get(&task_id).map(|wrapper| wrapper.flags)
    }

    #[cfg(test)]
    pub(crate) fn set_wrapper_flags_for_test(
        &mut self,
        task_id: ActorTaskId,
        flags: ActorTaskWrapperFlags,
    ) {
        if let Some(wrapper) = self.wrappers.get_mut(&task_id) {
            wrapper.flags = flags;
        }
    }

    /// Begin one already-authenticated wrapper callback without traversing the
    /// other two actor slots.
    ///
    /// Some bounded live adapters own only one captured callback while another
    /// task family remains installed in an earlier slot.  Running the ordinary
    /// three-slot visitor in that situation would falsely claim execution of
    /// those unrelated callbacks.  This seam retains the same pre-callback
    /// state mutation and wrapper flag ordering for one caller-supplied visit.
    pub(crate) fn begin_exact_visit_with<B>(
        &mut self,
        visit: ActorTaskVisit,
        before_callback: impl FnOnce(&mut T) -> B,
    ) -> Option<B> {
        if self.fresh_visit(visit.slot) != Some(visit) {
            return None;
        }
        let wrapper = self.wrappers.get_mut(&visit.task_id)?;
        if !wrapper.flags.alive || wrapper.flags.in_callback {
            return None;
        }
        let state = wrapper.state.as_mut()?;
        let before = before_callback(state);
        wrapper.flags.in_callback = true;
        Some(before)
    }

    /// Borrow the state of one exact visit only while its wrapper is inside
    /// the callback phase begun by [`Self::begin_exact_visit_with`].
    pub(crate) fn exact_callback_state_mut(&mut self, visit: ActorTaskVisit) -> Option<&mut T> {
        if self.slots[visit.slot.index()] != Some(visit.task_id) {
            return None;
        }
        let wrapper = self.wrappers.get_mut(&visit.task_id)?;
        if !wrapper.flags.alive || !wrapper.flags.in_callback {
            return None;
        }
        wrapper.state.as_mut()
    }

    /// Unwind one callback begun through [`Self::begin_exact_visit_with`].
    ///
    /// The return value says whether the exact wrapper survived, matching the
    /// per-slot decision made by the full retail-order visitor.
    pub(crate) fn finish_exact_visit(&mut self, visit: ActorTaskVisit) -> bool {
        let Some(wrapper) = self.wrappers.get(&visit.task_id) else {
            return false;
        };
        if !wrapper.flags.in_callback {
            return false;
        }
        self.finish_visit(visit.task_id)
    }

    /// Clear one slot using retail's destroy-old-then-write-null ordering.
    pub fn clear_slot(&mut self, slot: ActorTaskSlot) {
        self.clear_slot_with_retirement(slot, |_| {});
    }

    /// Clear a slot while applying the inner task's component destructor.
    /// Retail `FUN_004010D0` marks the wrapper dead, invokes that destructor,
    /// and only then frees the inner task. An executing outer wrapper remains
    /// allocated until callback unwind; its destructor is not deferred.
    pub fn clear_slot_with_retirement(&mut self, slot: ActorTaskSlot, retire: impl FnOnce(&T)) {
        let old_task = self.slots[slot.index()];
        if let Some(task_id) = old_task {
            self.retire_task_with(task_id, retire);
        }
        self.slots[slot.index()] = None;
    }

    /// Clear the three slots in outer behavior-initializer fallback order.
    pub(crate) fn clear_behavior_initializer_failure_slots(&mut self) {
        for slot in [
            ActorTaskSlot::Secondary,
            ActorTaskSlot::Tertiary,
            ActorTaskSlot::Primary,
        ] {
            self.clear_slot(slot);
        }
    }

    /// Install a task whose allocation and initialization have already
    /// succeeded.
    ///
    /// The new wrapper is staged before the old wrapper is retired. The slot is
    /// published only after retiring its previous owner.
    pub fn replace_prepared(
        &mut self,
        slot: ActorTaskSlot,
        prepared: PreparedActorTask<T>,
    ) -> ActorTaskId {
        self.replace_prepared_with_retirement(slot, prepared, |_| {})
    }

    /// Publish a prepared task only after applying its predecessor's inner
    /// destructor. This is also the ordering for replacement during a callback.
    pub fn replace_prepared_with_retirement(
        &mut self,
        slot: ActorTaskSlot,
        prepared: PreparedActorTask<T>,
        retire: impl FnOnce(&T),
    ) -> ActorTaskId {
        let new_task_id = self.allocate_task_id();
        let previous = self.wrappers.insert(
            new_task_id,
            ActorTaskWrapper {
                flags: ActorTaskWrapperFlags {
                    alive: true,
                    in_callback: false,
                },
                state: Some(prepared.state),
            },
        );
        debug_assert!(previous.is_none());

        let old_task = self.slots[slot.index()];
        if let Some(task_id) = old_task {
            self.retire_task_with(task_id, retire);
        }
        self.slots[slot.index()] = Some(new_task_id);
        new_task_id
    }

    /// Run task-family preparation before replacing the destination slot.
    ///
    /// An error leaves the old destination task intact.
    pub fn try_replace_with<E>(
        &mut self,
        slot: ActorTaskSlot,
        prepare: impl FnOnce() -> Result<PreparedActorTask<T>, E>,
    ) -> Result<ActorTaskId, E> {
        let prepared = prepare()?;
        Ok(self.replace_prepared(slot, prepared))
    }

    /// Apply a transition in exact array order without rollback.
    ///
    /// `prepare` is called at each `TryInstall` action. If it fails, the
    /// destination slot for that action is preserved, earlier actions remain
    /// committed, and later actions are not attempted.
    pub fn apply_ordered_mutations<S, E>(
        &mut self,
        actions: impl IntoIterator<Item = ActorTaskMutation<S>>,
        prepare: impl FnMut(S) -> Result<PreparedActorTask<T>, E>,
    ) -> Result<(), ActorTaskPrepareError<E>> {
        self.apply_ordered_mutations_with_retirement(actions, prepare, |_| {})
    }

    /// Ordered transitions whose retired inner tasks own component effects.
    pub fn apply_ordered_mutations_with_retirement<S, E>(
        &mut self,
        actions: impl IntoIterator<Item = ActorTaskMutation<S>>,
        mut prepare: impl FnMut(S) -> Result<PreparedActorTask<T>, E>,
        mut retire: impl FnMut(&T),
    ) -> Result<(), ActorTaskPrepareError<E>> {
        for (action_index, action) in actions.into_iter().enumerate() {
            match action {
                ActorTaskMutation::Clear { slot } => {
                    self.clear_slot_with_retirement(slot, &mut retire)
                }
                ActorTaskMutation::TryInstall {
                    slot,
                    specification,
                } => {
                    let prepared =
                        prepare(specification).map_err(|error| ActorTaskPrepareError {
                            action_index,
                            slot,
                            error,
                        })?;
                    self.replace_prepared_with_retirement(slot, prepared, &mut retire);
                }
            }
        }
        Ok(())
    }

    /// Visit slots in retail order, reading each slot immediately before use.
    ///
    /// The visitor may clear or replace slots through `owner`. Changes to later
    /// slots are observed by this same pass. If the current task clears or
    /// replaces itself, a `Propagate` value is dropped and traversal continues,
    /// matching `FUN_00401120`'s dead-wrapper path.
    ///
    /// This is only a structural scheduler seam; it does not dispatch or
    /// interpret any retail task callback.
    pub fn visit_slots_fresh<R>(
        &mut self,
        mut visitor: impl FnMut(&mut Self, ActorTaskVisit) -> ActorTaskVisitControl<R>,
    ) -> Option<R> {
        self.visit_slots_fresh_phased(
            |_state, _visit| (),
            |owner, visit| visitor(owner, visit),
            |_owner, _visit, (), control| control,
        )
    }

    /// Visit slots through the exact three phases owned by `FUN_00401120`.
    ///
    /// For each freshly read slot:
    ///
    /// 1. `before_callback` runs against live inner state while
    ///    `in_callback == false`;
    /// 2. `callback` runs while `in_callback == true`;
    /// 3. the wrapper is unwound, and only a surviving task reaches
    ///    `after_unwind` while `in_callback == false`.
    ///
    /// Retail uses phase 1 for elapsed-time accounting, phase 2 for the task
    /// tick itself, and phase 3 for tagged-result and timeout dispatch. If the
    /// callback clears or replaces itself, both phase outputs are discarded
    /// and traversal continues from the freshly read next slot. A mutation
    /// performed by `after_unwind` does not discard that phase's propagation:
    /// the callback had already survived and unwound before the transition.
    pub fn visit_slots_fresh_phased<B, C, R>(
        &mut self,
        mut before_callback: impl FnMut(&mut T, ActorTaskVisit) -> B,
        mut callback: impl FnMut(&mut Self, ActorTaskVisit) -> C,
        mut after_unwind: impl FnMut(&mut Self, ActorTaskVisit, B, C) -> ActorTaskVisitControl<R>,
    ) -> Option<R> {
        self.visit_slots_fresh_phased_with(
            &mut (),
            |(), state, visit| before_callback(state, visit),
            |(), owner, visit| callback(owner, visit),
            |(), owner, visit, before, callback_output| {
                after_unwind(owner, visit, before, callback_output)
            },
        )
    }

    /// Context-carrying form of [`Self::visit_slots_fresh_phased`].
    ///
    /// The context is passed sequentially through all three phases. This keeps
    /// a central heterogeneous dispatcher from hiding its external adapters
    /// behind interior mutability while preserving the same owner lifecycle
    /// and fresh-slot semantics.
    pub fn visit_slots_fresh_phased_with<Context, B, C, R>(
        &mut self,
        context: &mut Context,
        before_callback: impl FnMut(&mut Context, &mut T, ActorTaskVisit) -> B,
        callback: impl FnMut(&mut Context, &mut Self, ActorTaskVisit) -> C,
        after_unwind: impl FnMut(
            &mut Context,
            &mut Self,
            ActorTaskVisit,
            B,
            C,
        ) -> ActorTaskVisitControl<R>,
    ) -> Option<R> {
        self.visit_slots_fresh_phased_with_from(
            ActorTaskSlot::Primary,
            context,
            before_callback,
            callback,
            after_unwind,
        )
    }

    /// Resume one retail pass at an exact physical slot while retaining fresh
    /// reads for that slot and every later slot. Earlier slots are never
    /// revisited.
    pub fn visit_slots_fresh_phased_with_from<Context, B, C, R>(
        &mut self,
        start_slot: ActorTaskSlot,
        context: &mut Context,
        mut before_callback: impl FnMut(&mut Context, &mut T, ActorTaskVisit) -> B,
        mut callback: impl FnMut(&mut Context, &mut Self, ActorTaskVisit) -> C,
        mut after_unwind: impl FnMut(
            &mut Context,
            &mut Self,
            ActorTaskVisit,
            B,
            C,
        ) -> ActorTaskVisitControl<R>,
    ) -> Option<R> {
        for &slot in start_slot.retail_tick_suffix() {
            let Some(visit) = self.fresh_visit(slot) else {
                continue;
            };
            let before = {
                let wrapper = self
                    .wrappers
                    .get_mut(&visit.task_id)
                    .expect("a task slot must reference an owned wrapper");
                debug_assert!(wrapper.flags.alive);
                debug_assert!(!wrapper.flags.in_callback);
                let state = wrapper
                    .state
                    .as_mut()
                    .expect("a live task wrapper must own inner state");
                before_callback(context, state, visit)
            };

            self.begin_callback(visit.task_id);
            let callback_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                callback(context, self, visit)
            }));
            let survived = self.finish_visit(visit.task_id);
            let callback_output = match callback_result {
                Ok(output) => output,
                Err(payload) => std::panic::resume_unwind(payload),
            };
            if !survived {
                drop(before);
                drop(callback_output);
                continue;
            }

            if let ActorTaskVisitControl::Propagate(value) =
                after_unwind(context, self, visit, before, callback_output)
            {
                return Some(value);
            }
        }
        None
    }

    fn fresh_visit(&self, slot: ActorTaskSlot) -> Option<ActorTaskVisit> {
        let task_id = self.slots[slot.index()]?;
        Some(ActorTaskVisit { slot, task_id })
    }

    fn begin_callback(&mut self, task_id: ActorTaskId) {
        let wrapper = self
            .wrappers
            .get_mut(&task_id)
            .expect("a task slot must reference an owned wrapper");
        debug_assert!(wrapper.flags.alive);
        debug_assert!(!wrapper.flags.in_callback);
        wrapper.flags.in_callback = true;
    }

    /// Returns whether the task remained alive through the visit.
    fn finish_visit(&mut self, task_id: ActorTaskId) -> bool {
        let survived = {
            let wrapper = self
                .wrappers
                .get_mut(&task_id)
                .expect("an executing wrapper is reclaimed only after callback unwind");
            debug_assert!(wrapper.flags.in_callback);
            let survived = wrapper.flags.alive;
            wrapper.flags.in_callback = false;
            survived
        };
        if !survived {
            self.wrappers.remove(&task_id);
        }
        survived
    }

    /// Model `FUN_004010D0`: mark dead before dropping inner state, then defer
    /// only the outer-wrapper reclaim if execution is active.
    fn retire_task_with(&mut self, task_id: ActorTaskId, retire: impl FnOnce(&T)) {
        let reclaim_now = {
            let Some(wrapper) = self.wrappers.get_mut(&task_id) else {
                return;
            };
            if !wrapper.flags.alive {
                return;
            }
            wrapper.flags.alive = false;
            if let Some(state) = wrapper.state.as_ref() {
                retire(state);
            }
            drop(wrapper.state.take());
            !wrapper.flags.in_callback
        };
        if reclaim_now {
            self.wrappers.remove(&task_id);
        }
    }

    fn allocate_task_id(&mut self) -> ActorTaskId {
        let task_id = ActorTaskId(self.next_task_id);
        self.next_task_id = self
            .next_task_id
            .checked_add(1)
            .expect("actor task identity space exhausted");
        task_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn inner_retirement_effect_runs_before_callback_unwind_and_only_once() {
        let mut owner = ActorTaskOwner::new();
        let task_id =
            owner.replace_prepared(ActorTaskSlot::Secondary, PreparedActorTask::new("cue"));
        let mut events = Vec::new();
        owner.visit_slots_fresh::<()>(|owner, visit| {
            assert_eq!(visit.task_id, task_id);
            owner.clear_slot_with_retirement(visit.slot, |state| events.push(*state));
            assert_eq!(events, ["cue"]);
            assert_eq!(
                owner.wrapper_flags(task_id),
                Some(ActorTaskWrapperFlags {
                    alive: false,
                    in_callback: true
                })
            );
            assert!(owner.task_state(task_id).is_none());
            owner.clear_slot_with_retirement(visit.slot, |_| panic!("already retired"));
            events.push("callback tail");
            ActorTaskVisitControl::Continue
        });
        assert_eq!(events, ["cue", "callback tail"]);
        assert_eq!(owner.wrapper_flags(task_id), None);
    }

    #[test]
    fn replacement_retirement_runs_after_preparation_before_new_publication() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Tertiary, PreparedActorTask::new("old cue"));
        let mut events = Vec::new();
        events.push("prepared cue");
        owner.replace_prepared_with_retirement(
            ActorTaskSlot::Tertiary,
            PreparedActorTask::new("new cue"),
            |state| events.push(*state),
        );
        assert_eq!(
            owner.state_in_slot(ActorTaskSlot::Tertiary),
            Some(&"new cue")
        );
        events.push("new stop flag");
        assert_eq!(events, ["prepared cue", "old cue", "new stop flag"]);
    }

    fn install(owner: &mut ActorTaskOwner<&'static str>, slot: ActorTaskSlot, state: &'static str) {
        owner.replace_prepared(slot, PreparedActorTask::new(state));
    }

    #[test]
    fn failed_prepare_preserves_destination_without_rolling_back_earlier_clears() {
        let mut owner = ActorTaskOwner::new();
        install(&mut owner, ActorTaskSlot::Primary, "old primary");
        install(&mut owner, ActorTaskSlot::Secondary, "old secondary");
        install(&mut owner, ActorTaskSlot::Tertiary, "old tertiary");

        let result = owner.apply_ordered_mutations(
            [
                ActorTaskMutation::Clear {
                    slot: ActorTaskSlot::Secondary,
                },
                ActorTaskMutation::Clear {
                    slot: ActorTaskSlot::Tertiary,
                },
                ActorTaskMutation::TryInstall {
                    slot: ActorTaskSlot::Primary,
                    specification: "new primary",
                },
            ],
            |_specification| Err::<PreparedActorTask<&'static str>, _>("initializer failed"),
        );

        assert_eq!(
            result,
            Err(ActorTaskPrepareError {
                action_index: 2,
                slot: ActorTaskSlot::Primary,
                error: "initializer failed",
            })
        );
        assert_eq!(
            owner.state_in_slot(ActorTaskSlot::Primary),
            Some(&"old primary")
        );
        assert_eq!(owner.task_in_slot(ActorTaskSlot::Secondary), None);
        assert_eq!(owner.task_in_slot(ActorTaskSlot::Tertiary), None);
    }

    #[derive(Debug)]
    struct LoggedState {
        name: &'static str,
        events: Rc<RefCell<Vec<&'static str>>>,
    }

    impl Drop for LoggedState {
        fn drop(&mut self) {
            self.events.borrow_mut().push(self.name);
        }
    }

    #[test]
    fn successful_replacement_prepares_new_state_before_destroying_old_state() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(LoggedState {
                name: "drop old",
                events: Rc::clone(&events),
            }),
        );

        owner
            .try_replace_with(ActorTaskSlot::Primary, || {
                events.borrow_mut().push("prepare new");
                Ok::<_, ()>(PreparedActorTask::new(LoggedState {
                    name: "drop new",
                    events: Rc::clone(&events),
                }))
            })
            .unwrap();

        assert_eq!(&*events.borrow(), &["prepare new", "drop old"]);
    }

    #[test]
    fn traversal_reads_each_later_slot_after_preceding_mutations() {
        let mut owner = ActorTaskOwner::new();
        install(&mut owner, ActorTaskSlot::Primary, "primary");
        install(&mut owner, ActorTaskSlot::Secondary, "old secondary");
        install(&mut owner, ActorTaskSlot::Tertiary, "old tertiary");
        let mut visited = Vec::new();

        let propagated = owner.visit_slots_fresh(|owner, visit| {
            visited.push(*owner.task_state(visit.task_id).unwrap());
            if visit.slot == ActorTaskSlot::Primary {
                owner.clear_slot(ActorTaskSlot::Secondary);
                owner.replace_prepared(
                    ActorTaskSlot::Tertiary,
                    PreparedActorTask::new("new tertiary"),
                );
            }
            ActorTaskVisitControl::<()>::Continue
        });

        assert_eq!(propagated, None);
        assert_eq!(visited, ["primary", "new tertiary"]);
    }

    #[test]
    fn self_replacement_defers_outer_reclaim_and_discards_propagation() {
        let mut owner = ActorTaskOwner::new();
        install(&mut owner, ActorTaskSlot::Primary, "old primary");
        let old_task = owner.task_in_slot(ActorTaskSlot::Primary).unwrap();

        let propagated = owner.visit_slots_fresh(|owner, visit| {
            assert_eq!(visit.task_id, old_task);
            owner.replace_prepared(
                ActorTaskSlot::Primary,
                PreparedActorTask::new("new primary"),
            );
            assert_eq!(
                owner.wrapper_flags(old_task),
                Some(ActorTaskWrapperFlags {
                    alive: false,
                    in_callback: true,
                })
            );
            assert_eq!(owner.task_state(old_task), None);
            ActorTaskVisitControl::Propagate("ignored")
        });

        assert_eq!(propagated, None);
        assert_eq!(owner.wrapper_flags(old_task), None);
        assert_eq!(
            owner.state_in_slot(ActorTaskSlot::Primary),
            Some(&"new primary")
        );
    }

    #[test]
    fn self_clear_defers_outer_reclaim_and_continues_with_fresh_later_slots() {
        let mut owner = ActorTaskOwner::new();
        install(&mut owner, ActorTaskSlot::Primary, "primary");
        install(&mut owner, ActorTaskSlot::Secondary, "secondary");
        let primary = owner.task_in_slot(ActorTaskSlot::Primary).unwrap();
        let mut visited = Vec::new();

        let propagated = owner.visit_slots_fresh(|owner, visit| {
            visited.push(*owner.task_state(visit.task_id).unwrap());
            if visit.slot == ActorTaskSlot::Primary {
                owner.clear_slot(ActorTaskSlot::Primary);
                assert_eq!(
                    owner.wrapper_flags(primary),
                    Some(ActorTaskWrapperFlags {
                        alive: false,
                        in_callback: true,
                    })
                );
                assert_eq!(owner.task_state(primary), None);
            }
            ActorTaskVisitControl::<()>::Continue
        });

        assert_eq!(propagated, None);
        assert_eq!(visited, ["primary", "secondary"]);
        assert_eq!(owner.wrapper_flags(primary), None);
        assert_eq!(owner.task_in_slot(ActorTaskSlot::Primary), None);
    }

    #[test]
    fn a_live_propagation_stops_before_later_slots() {
        let mut owner = ActorTaskOwner::new();
        install(&mut owner, ActorTaskSlot::Primary, "primary");
        install(&mut owner, ActorTaskSlot::Secondary, "secondary");
        let mut visited = Vec::new();

        let propagated = owner.visit_slots_fresh(|owner, visit| {
            visited.push(*owner.task_state(visit.task_id).unwrap());
            ActorTaskVisitControl::Propagate(visit.slot)
        });

        assert_eq!(propagated, Some(ActorTaskSlot::Primary));
        assert_eq!(visited, ["primary"]);
    }

    #[derive(Debug)]
    struct PhasedState {
        name: &'static str,
        elapsed_ms: u32,
    }

    fn install_phased(
        owner: &mut ActorTaskOwner<PhasedState>,
        slot: ActorTaskSlot,
        name: &'static str,
    ) {
        owner.replace_prepared(
            slot,
            PreparedActorTask::new(PhasedState {
                name,
                elapsed_ms: 0,
            }),
        );
    }

    #[test]
    fn phased_visit_accounts_before_callback_and_dispatches_after_unwind() {
        let mut owner = ActorTaskOwner::new();
        install_phased(&mut owner, ActorTaskSlot::Primary, "primary");
        let primary = owner.task_in_slot(ActorTaskSlot::Primary).unwrap();

        let propagated = owner.visit_slots_fresh_phased(
            |state, visit| {
                assert_eq!(visit.task_id, primary);
                state.elapsed_ms += 17;
                state.elapsed_ms
            },
            |owner, visit| {
                assert_eq!(
                    owner.wrapper_flags(visit.task_id),
                    Some(ActorTaskWrapperFlags {
                        alive: true,
                        in_callback: true,
                    })
                );
                let state = owner.task_state(visit.task_id).unwrap();
                (state.name, state.elapsed_ms)
            },
            |owner, visit, elapsed_ms, callback_output| {
                assert_eq!(
                    owner.wrapper_flags(visit.task_id),
                    Some(ActorTaskWrapperFlags {
                        alive: true,
                        in_callback: false,
                    })
                );
                assert_eq!(elapsed_ms, 17);
                assert_eq!(callback_output, ("primary", 17));
                ActorTaskVisitControl::Propagate("after unwind")
            },
        );

        assert_eq!(propagated, Some("after unwind"));
    }

    #[test]
    fn phased_visit_discards_self_cleared_callback_output_and_reads_later_slot_fresh() {
        let mut owner = ActorTaskOwner::new();
        install_phased(&mut owner, ActorTaskSlot::Primary, "primary");
        install_phased(&mut owner, ActorTaskSlot::Secondary, "old secondary");
        let primary = owner.task_in_slot(ActorTaskSlot::Primary).unwrap();
        let mut callbacks = Vec::new();
        let mut after_unwinds = Vec::new();

        let propagated = owner.visit_slots_fresh_phased(
            |state, _visit| {
                state.elapsed_ms += 1;
            },
            |owner, visit| {
                let name = owner.task_state(visit.task_id).unwrap().name;
                callbacks.push(name);
                if visit.task_id == primary {
                    owner.clear_slot(ActorTaskSlot::Primary);
                    owner.replace_prepared(
                        ActorTaskSlot::Secondary,
                        PreparedActorTask::new(PhasedState {
                            name: "new secondary",
                            elapsed_ms: 0,
                        }),
                    );
                }
                name
            },
            |_owner, _visit, (), callback_output| {
                after_unwinds.push(callback_output);
                ActorTaskVisitControl::<()>::Continue
            },
        );

        assert_eq!(propagated, None);
        assert_eq!(callbacks, ["primary", "new secondary"]);
        assert_eq!(after_unwinds, ["new secondary"]);
        assert_eq!(owner.wrapper_flags(primary), None);
    }

    #[test]
    fn post_unwind_replacement_does_not_discard_propagation() {
        let mut owner = ActorTaskOwner::new();
        install_phased(&mut owner, ActorTaskSlot::Primary, "old primary");
        let old_primary = owner.task_in_slot(ActorTaskSlot::Primary).unwrap();

        let propagated = owner.visit_slots_fresh_phased(
            |_state, _visit| (),
            |_owner, _visit| (),
            |owner, visit, (), ()| {
                assert_eq!(visit.task_id, old_primary);
                assert_eq!(
                    owner.wrapper_flags(old_primary),
                    Some(ActorTaskWrapperFlags {
                        alive: true,
                        in_callback: false,
                    })
                );
                owner.replace_prepared(
                    ActorTaskSlot::Primary,
                    PreparedActorTask::new(PhasedState {
                        name: "new primary",
                        elapsed_ms: 0,
                    }),
                );
                ActorTaskVisitControl::Propagate("transition result")
            },
        );

        assert_eq!(propagated, Some("transition result"));
        assert_eq!(owner.wrapper_flags(old_primary), None);
        assert_eq!(
            owner
                .state_in_slot(ActorTaskSlot::Primary)
                .map(|state| state.name),
            Some("new primary")
        );
    }

    #[test]
    fn phased_visit_unwinds_live_wrapper_before_resuming_callback_panic() {
        let mut owner = ActorTaskOwner::new();
        install_phased(&mut owner, ActorTaskSlot::Primary, "primary");
        let primary = owner.task_in_slot(ActorTaskSlot::Primary).unwrap();

        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            owner.visit_slots_fresh_phased(
                |_state, _visit| (),
                |_owner, _visit| -> () { panic!("callback panic") },
                |_owner, _visit, (), ()| ActorTaskVisitControl::<()>::Continue,
            )
        }));

        assert!(panic.is_err());
        assert_eq!(
            owner.wrapper_flags(primary),
            Some(ActorTaskWrapperFlags {
                alive: true,
                in_callback: false,
            })
        );
        assert_eq!(
            owner
                .state_in_slot(ActorTaskSlot::Primary)
                .map(|state| state.name),
            Some("primary")
        );
    }

    #[test]
    fn phased_visit_reclaims_self_cleared_wrapper_before_resuming_callback_panic() {
        let mut owner = ActorTaskOwner::new();
        install_phased(&mut owner, ActorTaskSlot::Primary, "primary");
        let primary = owner.task_in_slot(ActorTaskSlot::Primary).unwrap();

        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            owner.visit_slots_fresh_phased(
                |_state, _visit| (),
                |owner, _visit| -> () {
                    owner.clear_slot(ActorTaskSlot::Primary);
                    panic!("self-cleared callback panic")
                },
                |_owner, _visit, (), ()| ActorTaskVisitControl::<()>::Continue,
            )
        }));

        assert!(panic.is_err());
        assert_eq!(owner.task_in_slot(ActorTaskSlot::Primary), None);
        assert_eq!(owner.wrapper_flags(primary), None);
    }
}
