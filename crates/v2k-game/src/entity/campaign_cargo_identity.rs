//! Packed conventional cargo identities from `43260`, consumed by `451C00`.
//!
//! An unresolved fixture stamp retains its known type; it is neither a zero
//! stamp nor a missing slot. Native identities retain the exact packed dword.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignCargoIdentity {
    Packed(u32),
    UnresolvedStamp { entity_type: u32 },
}

impl CampaignCargoIdentity {
    pub const fn from_packed(word: u32) -> Self {
        Self::Packed(word)
    }

    /// `43260` adds the full type dword to the unsigned B4 word shifted left
    /// 16. Preserve its wrapping addition, even for noncanonical type inputs.
    pub const fn from_entity_parts(entity_type: u32, stamp: RetailRuntimeValue<u16>) -> Self {
        match stamp {
            RetailRuntimeValue::Known(stamp) => {
                Self::Packed(entity_type.wrapping_add((stamp as u32) << 16))
            }
            RetailRuntimeValue::Unresolved => Self::UnresolvedStamp { entity_type },
        }
    }

    /// `451D1D/451E21` test only this low half for list termination.
    pub const fn entity_type(self) -> u32 {
        match self {
            Self::Packed(word) => word & 0xffff,
            Self::UnresolvedStamp { entity_type } => entity_type & 0xffff,
        }
    }

    pub const fn packed_word(self) -> RetailRuntimeValue<u32> {
        match self {
            Self::Packed(word) => RetailRuntimeValue::Known(word),
            Self::UnresolvedStamp { .. } => RetailRuntimeValue::Unresolved,
        }
    }

    /// The raw saved high half, also used for the post-constructor B4 write
    /// on a miss. The matching consumer sign-extends it separately.
    pub const fn saved_stamp(self) -> RetailRuntimeValue<u16> {
        match self {
            Self::Packed(word) => RetailRuntimeValue::Known((word >> 16) as u16),
            Self::UnresolvedStamp { .. } => RetailRuntimeValue::Unresolved,
        }
    }
}

/// Controller `+19C` identities and its raw `+199` unlock byte. The snapshot
/// carries no health, position, task, animation, or world-local entity handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampaignCargoControllerState {
    pub unlock_raw: u8,
    pub(super) carried_identities: Vec<CampaignCargoIdentity>,
    /// Session `+184` packed owned-object types, distinct from cargo stamps.
    pub(super) auxiliary_owned_types: Vec<u32>,
}

impl CampaignCargoControllerState {
    /// Import the exact words of an existing serialized controller profile.
    pub fn from_packed(unlock_raw: u8, words: impl IntoIterator<Item = u32>) -> Self {
        Self {
            unlock_raw,
            carried_identities: words
                .into_iter()
                .map(CampaignCargoIdentity::from_packed)
                .collect(),
            auxiliary_owned_types: Vec::new(),
        }
    }

    pub fn with_auxiliary_owned_types(mut self, types: impl IntoIterator<Item = u32>) -> Self {
        self.auxiliary_owned_types = types.into_iter().collect();
        self
    }

    pub fn carried_identities(&self) -> &[CampaignCargoIdentity] {
        &self.carried_identities
    }

    pub fn auxiliary_owned_types(&self) -> &[u32] {
        &self.auxiliary_owned_types
    }

    /// Count visible occupied slots without compacting holes or applying the
    /// separate load-time first-zero and type-exclusion policy.
    pub fn occupied_slots(&self) -> usize {
        self.carried_identities
            .iter()
            .filter(|identity| identity.entity_type() != 0)
            .count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignCargoIdentityUnresolved {
    SavedStamp,
    CandidateStamp { entity_id: u32 },
}

impl EntityManager {
    /// `43260` clears unlocked slots, then writes direct Sub-J rows in order.
    /// Invalid/dying rows leave zero holes. Parent backlinks and `active` are
    /// not additional source predicates; a proxy's children are not recursed.
    pub fn campaign_cargo_controller_state(
        &self,
    ) -> RetailRuntimeValue<CampaignCargoControllerState> {
        let Some(player) = self.player() else {
            return RetailRuntimeValue::Unresolved;
        };
        let rows = match &player.sub_j_attachment_runtime {
            RetailRuntimeValue::Known(Some(runtime)) => runtime.ordered_entity_ids(),
            RetailRuntimeValue::Known(None) => &[],
            RetailRuntimeValue::Unresolved => return RetailRuntimeValue::Unresolved,
        };
        let mut carried_identities = vec![
            CampaignCargoIdentity::Packed(0);
            usize::from(self.player_cargo_unlock_raw).max(rows.len())
        ];
        for (slot, cargo_id) in carried_identities.iter_mut().zip(rows) {
            if let Some(entity) = self
                .entities
                .iter()
                .find(|entity| entity.id == *cargo_id && is_live_sub_j_profile_row(entity))
            {
                *slot = CampaignCargoIdentity::from_entity_parts(
                    entity.entity_type,
                    entity.construction_stamp_at_0xb4,
                );
            }
        }
        RetailRuntimeValue::Known(CampaignCargoControllerState {
            unlock_raw: self.player_cargo_unlock_raw,
            carried_identities,
            auxiliary_owned_types: self.auxiliary_owned_types(),
        })
    }

    /// `451D46..451D89`: first type/stamp match in intrusive live-list order,
    /// without filtering inactive, dying, attached, or unsupported actors.
    /// Callback admission belongs after this identity decision.
    pub(crate) fn find_campaign_cargo_identity(
        &self,
        identity: CampaignCargoIdentity,
    ) -> Result<Option<u32>, CampaignCargoIdentityUnresolved> {
        let RetailRuntimeValue::Known(packed) = identity.packed_word() else {
            return Err(CampaignCargoIdentityUnresolved::SavedStamp);
        };
        // Retail SARs the packed dword, then compares with a zero-extended
        // entity u16. Every negative saved high half therefore proves a miss.
        let saved_comparison = (packed as i32) >> 16;
        if saved_comparison < 0 {
            return Ok(None);
        }
        for entity in &self.entities {
            // Source reads stamp before type. Skipping a known other type is
            // still exact: that candidate cannot match regardless of stamp,
            // and the source comparison has no mutation or other side effect.
            if entity.entity_type != identity.entity_type() {
                continue;
            }
            let RetailRuntimeValue::Known(stamp) = entity.construction_stamp_at_0xb4 else {
                // Do not skip an unresolved earlier candidate and select a
                // later match, or manufacture a miss/new allocation.
                return Err(CampaignCargoIdentityUnresolved::CandidateStamp {
                    entity_id: entity.id,
                });
            };
            if i32::from(stamp) == saved_comparison {
                return Ok(Some(entity.id));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use v2k_formats::collision::{SubJAttachmentDescriptor, SubJAttachmentSlotDescriptor};

    fn body(id: u32, entity_type: u32, stamp: RetailRuntimeValue<u16>) -> Entity {
        let mut entity =
            Entity::unresolved_port_entity(id, EntityKind::from_type(entity_type), entity_type);
        entity.construction_stamp_at_0xb4 = stamp;
        entity.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
        entity
    }

    fn snapshot_manager(rows: &[u32], mut cargo: Vec<Entity>, unlock_raw: u8) -> EntityManager {
        let mut player = body(1, PLAYER_ENTITY_TYPE, RetailRuntimeValue::Known(0x0400));
        let descriptor = SubJAttachmentDescriptor {
            reserved_at_0x01: 0,
            slots: vec![
                SubJAttachmentSlotDescriptor {
                    policy_word_raw: 0,
                    local_offset_raw: [0; 3],
                };
                8
            ]
            .into_boxed_slice(),
        };
        let mut runtime = SubJAttachmentRuntime::from_descriptor(&descriptor).unwrap();
        for &id in rows {
            runtime.append(id).unwrap();
        }
        player.sub_j_attachment_runtime = RetailRuntimeValue::Known(Some(runtime));
        let mut entities = vec![player];
        entities.append(&mut cargo);
        let mut manager = EntityManager::from_entities_for_test(entities);
        manager.player_cargo_unlock_raw = unlock_raw;
        manager
    }

    fn snapshot(manager: &EntityManager) -> CampaignCargoControllerState {
        let RetailRuntimeValue::Known(state) = manager.campaign_cargo_controller_state() else {
            panic!("exact player Sub-J profile")
        };
        state
    }

    #[test]
    fn packed_words_preserve_full_stamp_and_source_wrapping_addition() {
        for stamp in [0, 0x7fff, 0x8000, 0xffff] {
            let identity =
                CampaignCargoIdentity::from_entity_parts(68, RetailRuntimeValue::Known(stamp));
            assert_eq!(identity.entity_type(), 68);
            assert_eq!(identity.saved_stamp(), RetailRuntimeValue::Known(stamp));
            assert_eq!(
                identity.packed_word(),
                RetailRuntimeValue::Known((u32::from(stamp) << 16) + 68)
            );
        }
        let carried_type_high_half = CampaignCargoIdentity::from_entity_parts(
            0x0001_0044,
            RetailRuntimeValue::Known(0xffff),
        );
        assert_eq!(
            carried_type_high_half.packed_word(),
            RetailRuntimeValue::Known(68)
        );
        assert_eq!(
            carried_type_high_half.saved_stamp(),
            RetailRuntimeValue::Known(0)
        );
    }

    #[test]
    fn unresolved_stamp_preserves_type_without_inventing_zero() {
        let identity = CampaignCargoIdentity::from_entity_parts(9, RetailRuntimeValue::Unresolved);
        assert_eq!(identity.entity_type(), 9);
        assert_eq!(identity.packed_word(), RetailRuntimeValue::Unresolved);
        assert_eq!(identity.saved_stamp(), RetailRuntimeValue::Unresolved);
        let manager = snapshot_manager(&[2], vec![body(2, 9, RetailRuntimeValue::Unresolved)], 3);
        let state = snapshot(&manager);
        assert_eq!(
            state.carried_identities(),
            &[
                identity,
                CampaignCargoIdentity::Packed(0),
                CampaignCargoIdentity::Packed(0)
            ]
        );
        assert_eq!(state.occupied_slots(), 1);
    }

    #[test]
    fn snapshot_keeps_row_order_holes_and_cleared_unlocked_tail() {
        let cargo = body(2, 68, RetailRuntimeValue::Known(0));
        let mut inactive = body(3, 92, RetailRuntimeValue::Known(0xffff));
        inactive.active = false;
        let mut dying = body(4, 68, RetailRuntimeValue::Known(0x0804));
        dying.collision.state_flags_at_0x08 = RetailStateWord::exact(4 | DYING_STATE_BIT);
        let manager = snapshot_manager(&[3, 99, 2, 4], vec![cargo, inactive, dying], 5);
        let state = snapshot(&manager);
        assert_eq!(
            state,
            CampaignCargoControllerState::from_packed(5, [0xffff_005c, 0, 68, 0, 0])
        );
        assert_eq!(state.occupied_slots(), 2);
        assert!(
            manager
                .entities
                .iter()
                .all(|entity| entity.attached_to.is_none()),
            "serialization follows rows rather than backlinks"
        );
    }

    #[test]
    fn snapshot_excludes_children_owned_only_by_a_departing_proxy() {
        let mut proxy_cargo = body(3, 68, RetailRuntimeValue::Known(0x0803));
        proxy_cargo.attached_to = Some(4);
        let manager = snapshot_manager(
            &[2],
            vec![
                body(2, 68, RetailRuntimeValue::Known(0x0802)),
                proxy_cargo,
                body(4, 93, RetailRuntimeValue::Known(0x0804)),
            ],
            3,
        );
        assert_eq!(
            snapshot(&manager),
            CampaignCargoControllerState::from_packed(3, [0x0802_0044, 0, 0])
        );
    }

    #[test]
    fn type_half_controls_occupied_slots_even_with_nonzero_high_half() {
        let state =
            CampaignCargoControllerState::from_packed(3, [0x8000_0000, 0x8000_0009, 0x0044]);
        assert_eq!(state.occupied_slots(), 2);
        assert_eq!(state.carried_identities()[0].entity_type(), 0);
        assert_eq!(
            state.carried_identities()[0].saved_stamp(),
            RetailRuntimeValue::Known(0x8000)
        );
    }

    #[test]
    fn matching_uses_first_list_identity_without_lifecycle_filters() {
        let mut first = body(10, 68, RetailRuntimeValue::Known(0x0801));
        first.active = false;
        first.attached_to = Some(999);
        first.collision.state_flags_at_0x08 = RetailStateWord::exact(DYING_STATE_BIT);
        let mut manager = EntityManager::from_entities_for_test(vec![
            first,
            body(7, 68, RetailRuntimeValue::Known(0x0801)),
        ]);
        manager.set_common_body_stamp_counter_for_test(2);
        let before = manager.next_common_body_ordinal();
        assert_eq!(
            manager.find_campaign_cargo_identity(CampaignCargoIdentity::from_packed(0x0801_0044)),
            Ok(Some(10))
        );
        assert_eq!(manager.next_common_body_ordinal(), before);
        assert_eq!(manager.retail_live_order_ids().collect::<Vec<_>>(), [10, 7]);
    }

    #[test]
    fn signed_saved_high_half_never_matches_unsigned_body_word() {
        let manager = EntityManager::from_entities_for_test(vec![
            body(1, 68, RetailRuntimeValue::Unresolved),
            body(2, 68, RetailRuntimeValue::Known(0x8000)),
            body(3, 68, RetailRuntimeValue::Known(0xffff)),
        ]);
        for word in [0x8000_0044, 0xffff_0044] {
            assert_eq!(
                manager.find_campaign_cargo_identity(CampaignCargoIdentity::from_packed(word)),
                Ok(None)
            );
        }
    }

    #[test]
    fn unknown_earlier_matching_type_blocks_instead_of_skipping_to_later_match() {
        let manager = EntityManager::from_entities_for_test(vec![
            body(5, 92, RetailRuntimeValue::Unresolved),
            body(4, 68, RetailRuntimeValue::Unresolved),
            body(3, 68, RetailRuntimeValue::Known(0)),
        ]);
        assert_eq!(
            manager.find_campaign_cargo_identity(CampaignCargoIdentity::from_packed(68)),
            Err(CampaignCargoIdentityUnresolved::CandidateStamp { entity_id: 4 })
        );
        assert_eq!(
            manager.find_campaign_cargo_identity(CampaignCargoIdentity::from_entity_parts(
                68,
                RetailRuntimeValue::Unresolved
            )),
            Err(CampaignCargoIdentityUnresolved::SavedStamp)
        );
        let manager = EntityManager::from_entities_for_test(vec![
            body(5, 92, RetailRuntimeValue::Unresolved),
            body(3, 68, RetailRuntimeValue::Known(0)),
        ]);
        assert_eq!(
            manager.find_campaign_cargo_identity(CampaignCargoIdentity::from_packed(68)),
            Ok(Some(3))
        );
    }
}
