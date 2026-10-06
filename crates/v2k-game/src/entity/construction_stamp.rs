//! Retail `456C20` common-body identity, stored by `104B0` at entity `+B4`.
//!
//! The source counter is session word `+2D4`. The active world manager hosts
//! that lineage after the loader's explicit `451710` reset. A constructor
//! consumes the ordinal before body allocation, including failed allocations;
//! handles, intrusive publication and the process Sub-D counter are separate.

use super::EntityManager;
use crate::entity_collision_state::RetailRuntimeValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CommonBodyStampCounter {
    logical_world_index: i32,
    next_ordinal: u16,
}

impl CommonBodyStampCounter {
    /// `451763` resets the word before player creation or its reserved attempt.
    /// The input is raw controller `+C4`, not an overlay or progress-array index.
    pub(super) const fn new(logical_world_index: i32) -> Self {
        Self {
            logical_world_index,
            next_ordinal: 0,
        }
    }

    /// `456C20` adds the full unsigned ordinal to the wrapping 32-bit world
    /// term; `4104F1` truncates the result when storing the body word.
    pub(super) fn consume(&mut self) -> u16 {
        let ordinal = self.next_ordinal;
        self.next_ordinal = ordinal.wrapping_add(1);
        self.logical_world_index
            .wrapping_mul(0x400)
            .wrapping_add(i32::from(ordinal)) as u16
    }
}

impl EntityManager {
    /// Enter one actual `104B0` allocation attempt. Call after host admission,
    /// before body/component allocation or selector RNG, and retain consumption
    /// if the source allocation fails. Read-only plans must not call this.
    pub(crate) fn begin_common_body_attempt(&mut self) -> RetailRuntimeValue<u16> {
        self.common_body_stamps
            .as_mut()
            .map_or(RetailRuntimeValue::Unresolved, |counter| {
                RetailRuntimeValue::Known(counter.consume())
            })
    }

    /// Read the next source ordinal without allocating or asserting lineage
    /// for generic fixture/resource snapshots.
    pub fn next_common_body_ordinal(&self) -> RetailRuntimeValue<u16> {
        self.common_body_stamps
            .as_ref()
            .map_or(RetailRuntimeValue::Unresolved, |counter| {
                RetailRuntimeValue::Known(counter.next_ordinal)
            })
    }

    /// Install synthetic load lineage for focused allocation tests. This does
    /// not authenticate or stamp existing fixture bodies as native allocations.
    #[cfg(test)]
    pub(crate) fn set_common_body_stamp_counter_for_test(&mut self, logical_world_index: i32) {
        self.common_body_stamps = Some(CommonBodyStampCounter::new(logical_world_index));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinal_above_1023_contributes_without_a_ten_bit_mask() {
        let mut counter = CommonBodyStampCounter::new(2);
        counter.next_ordinal = 0x03ff;
        assert_eq!(counter.consume(), 0x0bff);
        assert_eq!(counter.consume(), 0x0c00);
        assert_eq!(counter.next_ordinal, 0x0401);
    }

    #[test]
    fn counter_and_stored_stamp_wrap_independently() {
        let mut counter = CommonBodyStampCounter::new(37);
        counter.next_ordinal = u16::MAX;
        assert_eq!(counter.consume(), 0x93ff);
        assert_eq!(counter.next_ordinal, 0);
        assert_eq!(counter.consume(), 0x9400);
        assert_eq!(counter.next_ordinal, 1);
    }

    #[test]
    fn raw_logical_world_37_is_not_a_bounded_progress_slot() {
        let mut counter = CommonBodyStampCounter::new(37);
        assert_eq!(counter.consume(), 0x9400);
        assert_eq!(counter.consume(), 0x9401);
    }

    #[test]
    fn signed_world_term_uses_wrapping_i32_arithmetic() {
        for (world, ordinal, expected) in [
            (-1, 0, 0xfc00),
            (-1, 0x0800, 0x0400),
            (i32::MAX, 0x3456, 0x3056),
            (i32::MIN, 0xffff, 0xffff),
        ] {
            let mut counter = CommonBodyStampCounter::new(world);
            counter.next_ordinal = ordinal;
            assert_eq!(
                counter.consume(),
                expected,
                "world {world}, ordinal {ordinal}"
            );
        }
    }

    #[test]
    fn generic_fixture_lineage_remains_unavailable() {
        let mut manager = EntityManager::from_entities_for_test(Vec::new());
        assert_eq!(
            manager.next_common_body_ordinal(),
            RetailRuntimeValue::Unresolved
        );
        assert_eq!(
            manager.begin_common_body_attempt(),
            RetailRuntimeValue::Unresolved
        );
        assert_eq!(
            manager.next_common_body_ordinal(),
            RetailRuntimeValue::Unresolved
        );
        assert!(manager.common_body_stamps.is_none());
    }

    #[test]
    fn entered_attempt_consumes_even_without_a_published_body() {
        let mut manager = EntityManager::from_entities_for_test(Vec::new());
        manager.set_common_body_stamp_counter_for_test(2);
        let next_entity_id = manager.next_entity_id;
        assert_eq!(
            manager.next_common_body_ordinal(),
            RetailRuntimeValue::Known(0)
        );
        // The allocator may reject after this receipt, or the loader may
        // discard it for the skipped player's explicit reserved attempt.
        assert_eq!(
            manager.begin_common_body_attempt(),
            RetailRuntimeValue::Known(0x0800)
        );
        assert_eq!(
            manager.next_common_body_ordinal(),
            RetailRuntimeValue::Known(1)
        );
        assert_eq!(
            manager.next_common_body_ordinal(),
            RetailRuntimeValue::Known(1)
        );
        assert!(manager.entities.is_empty());
        assert_eq!(manager.next_entity_id, next_entity_id);
        assert_eq!(
            manager.begin_common_body_attempt(),
            RetailRuntimeValue::Known(0x0801)
        );
    }

    #[test]
    fn speculative_manager_fork_copies_but_does_not_share_the_counter() {
        let mut manager = EntityManager::from_entities_for_test(Vec::new());
        manager.set_common_body_stamp_counter_for_test(27);
        assert_eq!(
            manager.begin_common_body_attempt(),
            RetailRuntimeValue::Known(0x6c00)
        );
        let mut fork = manager.fork_for_main_base_abort_transaction();
        assert_eq!(
            fork.begin_common_body_attempt(),
            RetailRuntimeValue::Known(0x6c01)
        );
        assert_eq!(
            fork.next_common_body_ordinal(),
            RetailRuntimeValue::Known(2)
        );
        assert_eq!(
            manager.next_common_body_ordinal(),
            RetailRuntimeValue::Known(1)
        );
        assert_eq!(
            manager.begin_common_body_attempt(),
            RetailRuntimeValue::Known(0x6c01)
        );
    }
}
