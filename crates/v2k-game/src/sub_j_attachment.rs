//! Entity-owned ordered runtime for Section-12 Sub-J attachments.
//!
//! Retail `FUN_00418330` / demo `FUN_004182F0` allocate a fixed backing array
//! from the authored descriptor, then track a separate mutable length and
//! capacity. The array order is authoritative: child relation backlinks and
//! manager live-list order cannot reconstruct it.

use v2k_formats::collision::SubJAttachmentDescriptor;

/// Result of retail's append-or-deduplicate operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubJAttachmentAppend {
    Appended,
    AlreadyPresent,
}

/// The one non-success result from an attachment append.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubJAttachmentFull;

/// Authored descriptors above retail/demo's fixed eight-record allocation
/// limit cannot construct a live component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubJAttachmentDescriptorTooLarge {
    pub slot_count: usize,
}

/// Mutable header and ordered child identities owned by one Sub-J component.
///
/// The retail live records also carry two function pointers. Their effects
/// belong to the attachment-owning subsystem and are not interchangeable
/// across player cargo and captured actors, so this shared data shape retains
/// only the proven common authority: ordered child identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubJAttachmentRuntime {
    authored_slot_count: usize,
    capacity: usize,
    ordered_entity_ids: Vec<u32>,
    policy_raw_at_0x0c: u32,
}

impl SubJAttachmentRuntime {
    /// Construct the zero-filled retail header and empty ordered array.
    pub fn from_descriptor(
        descriptor: &SubJAttachmentDescriptor,
    ) -> Result<Self, SubJAttachmentDescriptorTooLarge> {
        let authored_slot_count = descriptor.slots.len();
        if authored_slot_count > 8 {
            return Err(SubJAttachmentDescriptorTooLarge {
                slot_count: authored_slot_count,
            });
        }
        Ok(Self {
            authored_slot_count,
            capacity: authored_slot_count,
            ordered_entity_ids: Vec::with_capacity(authored_slot_count),
            policy_raw_at_0x0c: 0,
        })
    }

    /// Backing-array ceiling authored by the Sub-J count byte.
    pub const fn authored_slot_count(&self) -> usize {
        self.authored_slot_count
    }

    /// Current append limit at runtime header `+0x08`.
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Current ordered live-record count at runtime header `+0x04`.
    pub fn len(&self) -> usize {
        self.ordered_entity_ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ordered_entity_ids.is_empty()
    }

    /// Constructor-zeroed outer policy dword at runtime header `+0x0C`.
    pub const fn policy_raw_at_0x0c(&self) -> u32 {
        self.policy_raw_at_0x0c
    }

    /// Publish an owner-specific outer policy recovered from its controller.
    pub(crate) fn set_policy_raw_at_0x0c(&mut self, policy_raw: u32) {
        self.policy_raw_at_0x0c = policy_raw;
    }

    /// Child handles in authoritative Sub-J insertion order.
    pub fn ordered_entity_ids(&self) -> &[u32] {
        &self.ordered_entity_ids
    }

    pub fn contains(&self, entity_id: u32) -> bool {
        self.ordered_entity_ids.contains(&entity_id)
    }

    /// Retail `FUN_00418620`: replace the live limit, clamped to the authored
    /// backing-array width. Lowering the limit does not discard existing rows.
    pub fn set_capacity_clamped(&mut self, requested: usize) {
        self.capacity = requested.min(self.authored_slot_count);
    }

    /// Monotonic capacity-upgrade wrapper used by the player controller.
    pub fn raise_capacity_to(&mut self, requested: usize) {
        self.set_capacity_clamped(self.capacity.max(requested));
    }

    /// Append in order, preserving retail's full-before-duplicate gate.
    pub fn append(&mut self, entity_id: u32) -> Result<SubJAttachmentAppend, SubJAttachmentFull> {
        if self.len() >= self.capacity {
            return Err(SubJAttachmentFull);
        }
        if self.contains(entity_id) {
            return Ok(SubJAttachmentAppend::AlreadyPresent);
        }
        self.ordered_entity_ids.push(entity_id);
        Ok(SubJAttachmentAppend::Appended)
    }

    pub fn last(&self) -> Option<u32> {
        self.ordered_entity_ids.last().copied()
    }

    pub fn pop_last(&mut self) -> Option<u32> {
        self.ordered_entity_ids.pop()
    }

    pub fn clear(&mut self) {
        self.ordered_entity_ids.clear();
    }

    pub fn take_ordered_entity_ids(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.ordered_entity_ids)
    }

    /// Commit `FUN_00418640`'s callback-free stable row compaction.
    ///
    /// The updater may only remove rows: every retained identity must remain
    /// in its original relative order, and neither capacity nor outer policy
    /// changes. The caller preflights row state before publishing this result.
    pub(crate) fn commit_stable_compaction(&mut self, retained_entity_ids: Vec<u32>) {
        let mut original = self.ordered_entity_ids.iter();
        debug_assert!(retained_entity_ids
            .iter()
            .all(|retained| original.by_ref().any(|current| current == retained)));
        debug_assert!(retained_entity_ids.len() <= self.authored_slot_count);
        self.ordered_entity_ids = retained_entity_ids;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use v2k_formats::collision::{SubJAttachmentDescriptor, SubJAttachmentSlotDescriptor};

    fn descriptor(count: usize) -> SubJAttachmentDescriptor {
        SubJAttachmentDescriptor {
            reserved_at_0x01: 0,
            slots: vec![
                SubJAttachmentSlotDescriptor {
                    policy_word_raw: 1,
                    local_offset_raw: [0; 3],
                };
                count
            ]
            .into_boxed_slice(),
        }
    }

    #[test]
    fn constructor_retains_authored_ceiling_and_zero_header_state() {
        let runtime = SubJAttachmentRuntime::from_descriptor(&descriptor(3)).unwrap();
        assert_eq!(runtime.authored_slot_count(), 3);
        assert_eq!(runtime.capacity(), 3);
        assert!(runtime.ordered_entity_ids().is_empty());
        assert_eq!(runtime.policy_raw_at_0x0c(), 0);
    }

    #[test]
    fn append_is_ordered_full_gated_and_duplicate_safe() {
        let mut runtime = SubJAttachmentRuntime::from_descriptor(&descriptor(3)).unwrap();
        runtime.set_capacity_clamped(2);
        assert_eq!(runtime.append(9), Ok(SubJAttachmentAppend::Appended));
        assert_eq!(runtime.append(9), Ok(SubJAttachmentAppend::AlreadyPresent));
        assert_eq!(runtime.append(4), Ok(SubJAttachmentAppend::Appended));
        // Retail checks fullness before duplicate identity.
        assert_eq!(runtime.append(4), Err(SubJAttachmentFull));
        assert_eq!(runtime.append(7), Err(SubJAttachmentFull));
        assert_eq!(runtime.ordered_entity_ids(), [9, 4]);
    }

    #[test]
    fn capacity_is_clamped_and_lifo_pop_preserves_remaining_order() {
        let mut runtime = SubJAttachmentRuntime::from_descriptor(&descriptor(3)).unwrap();
        runtime.append(8).unwrap();
        runtime.append(2).unwrap();
        runtime.append(5).unwrap();
        runtime.set_capacity_clamped(99);
        assert_eq!(runtime.capacity(), 3);
        assert_eq!(runtime.pop_last(), Some(5));
        assert_eq!(runtime.ordered_entity_ids(), [8, 2]);
        runtime.clear();
        assert!(runtime.is_empty());
    }

    #[test]
    fn constructor_rejects_descriptors_above_the_retail_ceiling() {
        assert_eq!(
            SubJAttachmentRuntime::from_descriptor(&descriptor(9)),
            Err(SubJAttachmentDescriptorTooLarge { slot_count: 9 })
        );
    }

    #[test]
    fn lowering_capacity_preserves_rows_and_blocks_further_appends() {
        let mut runtime = SubJAttachmentRuntime::from_descriptor(&descriptor(3)).unwrap();
        runtime.append(8).unwrap();
        runtime.append(2).unwrap();
        runtime.set_capacity_clamped(1);

        assert_eq!(runtime.capacity(), 1);
        assert_eq!(runtime.ordered_entity_ids(), [8, 2]);
        assert_eq!(runtime.append(5), Err(SubJAttachmentFull));
    }

    #[test]
    fn taking_rows_preserves_order_and_header_policy() {
        let mut runtime = SubJAttachmentRuntime::from_descriptor(&descriptor(3)).unwrap();
        runtime.set_policy_raw_at_0x0c(7);
        runtime.append(8).unwrap();
        runtime.append(2).unwrap();

        assert_eq!(runtime.take_ordered_entity_ids(), [8, 2]);
        assert!(runtime.is_empty());
        assert_eq!(runtime.authored_slot_count(), 3);
        assert_eq!(runtime.capacity(), 3);
        assert_eq!(runtime.policy_raw_at_0x0c(), 7);
        assert_eq!(runtime.append(5), Ok(SubJAttachmentAppend::Appended));
    }

    #[test]
    fn stable_compaction_removes_rows_without_changing_header_state() {
        let mut runtime = SubJAttachmentRuntime::from_descriptor(&descriptor(3)).unwrap();
        runtime.set_policy_raw_at_0x0c(7);
        runtime.append(8).unwrap();
        runtime.append(2).unwrap();
        runtime.append(5).unwrap();
        runtime.set_capacity_clamped(2);

        runtime.commit_stable_compaction(vec![8, 5]);

        assert_eq!(runtime.ordered_entity_ids(), [8, 5]);
        assert_eq!(runtime.authored_slot_count(), 3);
        assert_eq!(runtime.capacity(), 2);
        assert_eq!(runtime.policy_raw_at_0x0c(), 7);
    }
}
