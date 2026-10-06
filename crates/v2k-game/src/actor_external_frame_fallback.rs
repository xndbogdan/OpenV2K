//! Terminal actor selector routing from retail `FUN_0040A9F0` and `40A980`.
//!
//! Earlier Sub-H, joint, Sub-M and live Sub-G callbacks retain their owners.
//! Call this only when an executed type-14 slot reaches the terminal suffix,
//! after its authored face has passed the normal test. The slot cache, rather
//! than this arithmetic helper, owns exactly-once visitation.

use crate::entity_collision_state::RetailRuntimeValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorExternalFrameReservations {
    pub sub_h_records: u16,
    pub sub_e_joint_slots: u8,
    pub sub_m_present: bool,
    pub live_sub_g_present: bool,
}

impl ActorExternalFrameReservations {
    pub fn terminal_start(self) -> i32 {
        2 * i32::from(self.sub_h_records)
            + i32::from(self.sub_e_joint_slots)
            + i32::from(self.sub_m_present)
            + 2 * i32::from(self.live_sub_g_present)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorExternalFrameFallbackBoundary {
    NegativeSelector(i16),
    InvalidJointSlotCount(u8),
    UnresolvedRelation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorExternalFrameFallbackMode {
    Relation,
    SelfWithJitter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorExternalFrameFallbackPoint {
    pub mode: ActorExternalFrameFallbackMode,
    pub position_raw: [i16; 3],
}

/// Snapshot `43A580`-resolved positions before lending the mutable draw owner.
/// `Known(None)` covers a null or stale `entity+0x60` handle. An unowned handle
/// word remains `Unresolved`; it must not become the self branch by default.
pub struct ActorExternalFrameFallback<'a> {
    pub reservations: ActorExternalFrameReservations,
    pub own_origin_raw: [i16; 3],
    pub relation_origin_raw: RetailRuntimeValue<Option<[i16; 3]>>,
    pub random_u16: &'a mut dyn FnMut() -> u16,
    pub last_boundary: Option<ActorExternalFrameFallbackBoundary>,
}

impl ActorExternalFrameFallback<'_> {
    pub fn resolve_raw(
        &mut self,
        parameters: [i16; 3],
    ) -> Result<Option<ActorExternalFrameFallbackPoint>, ActorExternalFrameFallbackBoundary> {
        let selector = parameters[0];
        if selector < 0 {
            return self.reject(ActorExternalFrameFallbackBoundary::NegativeSelector(
                selector,
            ));
        }
        if self.reservations.sub_e_joint_slots > 2 {
            return self.reject(ActorExternalFrameFallbackBoundary::InvalidJointSlotCount(
                self.reservations.sub_e_joint_slots,
            ));
        }
        let remaining = i32::from(selector) - self.reservations.terminal_start();
        if remaining < 0 {
            return Ok(None);
        }
        let (mode, origin_raw) = if remaining == 0 {
            match self.relation_origin_raw {
                RetailRuntimeValue::Known(Some(origin)) => {
                    (ActorExternalFrameFallbackMode::Relation, origin)
                }
                RetailRuntimeValue::Known(None) => (
                    ActorExternalFrameFallbackMode::SelfWithJitter,
                    self.own_origin_raw,
                ),
                RetailRuntimeValue::Unresolved => {
                    return self.reject(ActorExternalFrameFallbackBoundary::UnresolvedRelation);
                }
            }
        } else {
            (
                ActorExternalFrameFallbackMode::SelfWithJitter,
                self.own_origin_raw,
            )
        };
        // 40A980 calls 57930 on every axis even when its multiplier is zero.
        // Only AX after SHR 0xB contributes; ADD AX wraps before MOVSX.
        let position_raw = origin_raw.map(|origin| {
            let draw = (self.random_u16)();
            let offset = if mode == ActorExternalFrameFallbackMode::Relation {
                0
            } else {
                (draw >> 11) as i16
            };
            origin.wrapping_add(offset)
        });
        Ok(Some(ActorExternalFrameFallbackPoint { mode, position_raw }))
    }

    fn reject<T>(
        &mut self,
        boundary: ActorExternalFrameFallbackBoundary,
    ) -> Result<T, ActorExternalFrameFallbackBoundary> {
        self.last_boundary = Some(boundary);
        Err(boundary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type47_reservations() -> ActorExternalFrameReservations {
        ActorExternalFrameReservations {
            sub_h_records: 6,
            // Type47 Sub-E +12 is source slot150; +14 is zero.
            sub_e_joint_slots: 1,
            sub_m_present: false,
            live_sub_g_present: false,
        }
    }

    #[test]
    fn type47_terminal_selector13_relation_consumes_three_draws_without_jitter() {
        let mut consumed = Vec::new();
        let draws = [0, 0xffff, 0x8123];
        let mut random = || {
            let value = draws[consumed.len()];
            consumed.push(value);
            value
        };
        let mut fallback = ActorExternalFrameFallback {
            reservations: type47_reservations(),
            own_origin_raw: [10, 20, 30],
            relation_origin_raw: RetailRuntimeValue::Known(Some([-32768, 10, 32767])),
            random_u16: &mut random,
            last_boundary: None,
        };
        assert_eq!(
            fallback.resolve_raw([13, 123, -456]),
            Ok(Some(ActorExternalFrameFallbackPoint {
                mode: ActorExternalFrameFallbackMode::Relation,
                position_raw: [-32768, 10, 32767],
            }))
        );
        assert_eq!(consumed, draws);
    }

    #[test]
    fn null_or_stale_relation_uses_signed_word_self_jitter() {
        let mut count = 0;
        let mut random = || {
            let value = [0x7ff, 0x800, 0xffff][count];
            count += 1;
            value
        };
        let mut fallback = ActorExternalFrameFallback {
            reservations: type47_reservations(),
            own_origin_raw: [100, 32767, -32768],
            relation_origin_raw: RetailRuntimeValue::Known(None),
            random_u16: &mut random,
            last_boundary: None,
        };
        assert_eq!(
            fallback.resolve_raw([13, 0, 0]),
            Ok(Some(ActorExternalFrameFallbackPoint {
                mode: ActorExternalFrameFallbackMode::SelfWithJitter,
                position_raw: [100, -32768, -32737],
            }))
        );
        assert_eq!(count, 3);
    }

    #[test]
    fn reserved_callbacks_never_borrow_terminal_rng() {
        let mut random = || panic!("reserved callback borrowed terminal RNG");
        let mut fallback = ActorExternalFrameFallback {
            reservations: ActorExternalFrameReservations {
                sub_h_records: 6,
                sub_e_joint_slots: 2,
                sub_m_present: true,
                live_sub_g_present: true,
            },
            own_origin_raw: [0; 3],
            relation_origin_raw: RetailRuntimeValue::Unresolved,
            random_u16: &mut random,
            last_boundary: None,
        };
        for selector in 0..17 {
            assert_eq!(fallback.resolve_raw([selector, 0, 0]), Ok(None));
        }
        assert_eq!(fallback.last_boundary, None);
    }

    #[test]
    fn unresolved_relation_blocks_zero_suffix_but_later_selectors_use_self() {
        let mut count = 0;
        let mut random = || {
            count += 1;
            0xffff
        };
        let mut fallback = ActorExternalFrameFallback {
            reservations: type47_reservations(),
            own_origin_raw: [0, 10, 20],
            relation_origin_raw: RetailRuntimeValue::Unresolved,
            random_u16: &mut random,
            last_boundary: None,
        };
        assert_eq!(
            fallback.resolve_raw([13, 0, 0]),
            Err(ActorExternalFrameFallbackBoundary::UnresolvedRelation)
        );
        assert_eq!(
            fallback.resolve_raw([14, 0, 0]),
            Ok(Some(ActorExternalFrameFallbackPoint {
                mode: ActorExternalFrameFallbackMode::SelfWithJitter,
                position_raw: [31, 41, 51],
            }))
        );
        assert_eq!(count, 3);
    }

    #[test]
    fn malformed_selector_or_joint_reservations_never_draw_rng() {
        let mut random = || panic!("invalid metadata borrowed RNG");
        let mut fallback = ActorExternalFrameFallback {
            reservations: type47_reservations(),
            own_origin_raw: [0; 3],
            relation_origin_raw: RetailRuntimeValue::Known(None),
            random_u16: &mut random,
            last_boundary: None,
        };
        assert_eq!(
            fallback.resolve_raw([-1, 0, 0]),
            Err(ActorExternalFrameFallbackBoundary::NegativeSelector(-1))
        );
        fallback.reservations.sub_e_joint_slots = 3;
        assert_eq!(
            fallback.resolve_raw([13, 0, 0]),
            Err(ActorExternalFrameFallbackBoundary::InvalidJointSlotCount(3))
        );
    }
}
