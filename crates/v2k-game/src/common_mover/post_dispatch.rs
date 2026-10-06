//! Exact post-dispatch component routing for retail `FUN_00401430`.
//!
//! After `FUN_004018A0` returns, the common mover independently applies
//! Sub-C, Sub-A, and Sub-B in that order. Sub-G suppresses the complete tail,
//! while Sub-A has one additional runtime zero-target gate. This module owns
//! only that decision matrix; component-specific mutation remains with each
//! runtime owner.

use crate::entity_collision_state::{CommonMoverComponentTopology, RetailRuntimeValue};

/// One component phase selected after retail's central dispatcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonMoverPostDispatchPhase {
    SubC,
    SubA,
    SubB,
}

/// Why the exact post-dispatch route cannot yet be selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonMoverPostDispatchBlock {
    /// Sub-A is eligible, but its constructor- or task-owned target-speed word
    /// has not been recovered.
    UnresolvedSubATargetSpeed,
}

/// Fixed-capacity ordered phase plan for `FUN_00401430`'s C/A/B tail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonMoverPostDispatchPlan {
    phases: [Option<CommonMoverPostDispatchPhase>; 3],
}

impl CommonMoverPostDispatchPlan {
    /// Select the next C/A/B phase at or after `from_index`.
    ///
    /// Unlike [`Self::from_topology`], this cursor-shaped query does not read
    /// a later phase's runtime gate before an earlier phase has committed.
    /// Retail applies Sub-C first and only then reads Sub-A's current target
    /// speed, so resumable callers must query again with a fresh snapshot.
    pub const fn next_phase(
        topology: CommonMoverComponentTopology,
        from_index: usize,
        sub_a_target_speed_raw: RetailRuntimeValue<i32>,
    ) -> Result<Option<(CommonMoverPostDispatchPhase, usize)>, CommonMoverPostDispatchBlock> {
        if topology.sub_g {
            return Ok(None);
        }

        let mut index = from_index;
        while index < 3 {
            let next_index = index + 1;
            let phase = match index {
                0 if topology.sub_c => Some(CommonMoverPostDispatchPhase::SubC),
                1 if topology.sub_a => match sub_a_target_speed_raw {
                    RetailRuntimeValue::Known(0) => None,
                    RetailRuntimeValue::Known(_) => Some(CommonMoverPostDispatchPhase::SubA),
                    RetailRuntimeValue::Unresolved => {
                        return Err(CommonMoverPostDispatchBlock::UnresolvedSubATargetSpeed);
                    }
                },
                2 if topology.sub_b => Some(CommonMoverPostDispatchPhase::SubB),
                _ => None,
            };
            if let Some(phase) = phase {
                return Ok(Some((phase, next_index)));
            }
            index = next_index;
        }
        Ok(None)
    }

    /// Recover the exact branches at `0x004017AD..0x00401819`.
    ///
    /// `sub_a_target_speed_raw` is deliberately ignored when Sub-G suppresses
    /// the whole tail or when Sub-A is absent. An unresolved word must not
    /// block a route which retail would never read.
    pub const fn from_topology(
        topology: CommonMoverComponentTopology,
        sub_a_target_speed_raw: RetailRuntimeValue<i32>,
    ) -> Result<Self, CommonMoverPostDispatchBlock> {
        if topology.sub_g {
            return Ok(Self {
                phases: [None, None, None],
            });
        }

        let sub_c = if topology.sub_c {
            Some(CommonMoverPostDispatchPhase::SubC)
        } else {
            None
        };
        let sub_a = if topology.sub_a {
            match sub_a_target_speed_raw {
                RetailRuntimeValue::Known(0) => None,
                RetailRuntimeValue::Known(_) => Some(CommonMoverPostDispatchPhase::SubA),
                RetailRuntimeValue::Unresolved => {
                    return Err(CommonMoverPostDispatchBlock::UnresolvedSubATargetSpeed);
                }
            }
        } else {
            None
        };
        let sub_b = if topology.sub_b {
            Some(CommonMoverPostDispatchPhase::SubB)
        } else {
            None
        };

        Ok(Self {
            phases: [sub_c, sub_a, sub_b],
        })
    }

    pub const fn phases(self) -> [Option<CommonMoverPostDispatchPhase>; 3] {
        self.phases
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_tail_is_ordered_c_then_a_then_b() {
        let topology = CommonMoverComponentTopology {
            sub_a: true,
            sub_b: true,
            sub_c: true,
            ..CommonMoverComponentTopology::default()
        };

        assert_eq!(
            CommonMoverPostDispatchPlan::from_topology(topology, RetailRuntimeValue::Known(1),)
                .unwrap()
                .phases(),
            [
                Some(CommonMoverPostDispatchPhase::SubC),
                Some(CommonMoverPostDispatchPhase::SubA),
                Some(CommonMoverPostDispatchPhase::SubB),
            ]
        );
    }

    #[test]
    fn sub_g_suppresses_the_complete_tail_without_reading_sub_a_runtime() {
        let topology = CommonMoverComponentTopology {
            sub_a: true,
            sub_b: true,
            sub_c: true,
            sub_g: true,
            ..CommonMoverComponentTopology::default()
        };

        assert_eq!(
            CommonMoverPostDispatchPlan::from_topology(topology, RetailRuntimeValue::Unresolved,)
                .unwrap()
                .phases(),
            [None, None, None]
        );
    }

    #[test]
    fn component_absence_is_independent() {
        type DisableComponent = fn(&mut CommonMoverComponentTopology);
        let cases: [(DisableComponent, [Option<CommonMoverPostDispatchPhase>; 3]); 3] = [
            (
                |value| value.sub_c = false,
                [
                    None,
                    Some(CommonMoverPostDispatchPhase::SubA),
                    Some(CommonMoverPostDispatchPhase::SubB),
                ],
            ),
            (
                |value| value.sub_a = false,
                [
                    Some(CommonMoverPostDispatchPhase::SubC),
                    None,
                    Some(CommonMoverPostDispatchPhase::SubB),
                ],
            ),
            (
                |value| value.sub_b = false,
                [
                    Some(CommonMoverPostDispatchPhase::SubC),
                    Some(CommonMoverPostDispatchPhase::SubA),
                    None,
                ],
            ),
        ];

        for (disable, expected) in cases {
            let mut topology = CommonMoverComponentTopology {
                sub_a: true,
                sub_b: true,
                sub_c: true,
                ..CommonMoverComponentTopology::default()
            };
            disable(&mut topology);
            assert_eq!(
                CommonMoverPostDispatchPlan::from_topology(topology, RetailRuntimeValue::Known(1),)
                    .unwrap()
                    .phases(),
                expected
            );
        }
    }

    #[test]
    fn zero_target_skips_only_sub_a() {
        let topology = CommonMoverComponentTopology {
            sub_a: true,
            sub_b: true,
            sub_c: true,
            ..CommonMoverComponentTopology::default()
        };

        assert_eq!(
            CommonMoverPostDispatchPlan::from_topology(topology, RetailRuntimeValue::Known(0),)
                .unwrap()
                .phases(),
            [
                Some(CommonMoverPostDispatchPhase::SubC),
                None,
                Some(CommonMoverPostDispatchPhase::SubB),
            ]
        );
    }

    #[test]
    fn unresolved_target_blocks_only_an_eligible_sub_a() {
        let eligible = CommonMoverComponentTopology {
            sub_a: true,
            ..CommonMoverComponentTopology::default()
        };
        assert_eq!(
            CommonMoverPostDispatchPlan::from_topology(eligible, RetailRuntimeValue::Unresolved,),
            Err(CommonMoverPostDispatchBlock::UnresolvedSubATargetSpeed)
        );

        let absent = CommonMoverComponentTopology::default();
        assert_eq!(
            CommonMoverPostDispatchPlan::from_topology(absent, RetailRuntimeValue::Unresolved,)
                .unwrap()
                .phases(),
            [None, None, None]
        );
    }

    #[test]
    fn cursor_defers_sub_a_runtime_until_after_sub_c() {
        let topology = CommonMoverComponentTopology {
            sub_a: true,
            sub_c: true,
            ..CommonMoverComponentTopology::default()
        };

        assert_eq!(
            CommonMoverPostDispatchPlan::next_phase(topology, 0, RetailRuntimeValue::Unresolved,),
            Ok(Some((CommonMoverPostDispatchPhase::SubC, 1)))
        );
        assert_eq!(
            CommonMoverPostDispatchPlan::next_phase(topology, 1, RetailRuntimeValue::Unresolved,),
            Err(CommonMoverPostDispatchBlock::UnresolvedSubATargetSpeed)
        );
    }

    #[test]
    fn cursor_never_reads_sub_a_when_sub_g_suppresses_the_tail() {
        let topology = CommonMoverComponentTopology {
            sub_a: true,
            sub_b: true,
            sub_c: true,
            sub_g: true,
            ..CommonMoverComponentTopology::default()
        };

        assert_eq!(
            CommonMoverPostDispatchPlan::next_phase(topology, 0, RetailRuntimeValue::Unresolved,),
            Ok(None)
        );
    }
}
