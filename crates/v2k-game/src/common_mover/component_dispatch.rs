//! Exact component routing for retail `FUN_004018A0`.
//!
//! The common mover has one mutually exclusive primary component branch and,
//! in normal task mode, two independent suffixes. This module owns that
//! decision matrix so bounded mover implementations do not each grow their own
//! subtly different F/H/I/G/K/L boolean ladder. Component-specific mutation
//! remains with the corresponding runtime owner.

use crate::entity_collision_state::CommonMoverComponentTopology;

/// Recovered interpretation of `FUN_004018A0`'s final integer argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonMoverDispatchMode {
    /// Retail word zero: F, H, I, or G primary, followed by optional K and L.
    Normal,
    /// Any nonzero retail word: F or G only.
    Restricted,
}

impl CommonMoverDispatchMode {
    pub const fn from_retail_word(value: i32) -> Self {
        if value == 0 {
            Self::Normal
        } else {
            Self::Restricted
        }
    }
}

/// One component callback selected by the retail dispatcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonMoverDispatchPhase {
    SubF,
    SubH,
    SubI,
    SubG,
    SubK,
    SubL,
}

/// Fixed-capacity ordered callback plan produced by `FUN_004018A0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonMoverDispatchPlan {
    phases: [Option<CommonMoverDispatchPhase>; 3],
}

impl CommonMoverDispatchPlan {
    pub const fn from_topology(
        topology: CommonMoverComponentTopology,
        mode: CommonMoverDispatchMode,
    ) -> Self {
        let primary = match mode {
            CommonMoverDispatchMode::Normal => {
                if topology.sub_f {
                    Some(CommonMoverDispatchPhase::SubF)
                } else if topology.sub_h {
                    Some(CommonMoverDispatchPhase::SubH)
                } else if topology.sub_i {
                    Some(CommonMoverDispatchPhase::SubI)
                } else if topology.sub_g {
                    Some(CommonMoverDispatchPhase::SubG)
                } else {
                    None
                }
            }
            CommonMoverDispatchMode::Restricted => {
                if topology.sub_f {
                    Some(CommonMoverDispatchPhase::SubF)
                } else if topology.sub_g {
                    Some(CommonMoverDispatchPhase::SubG)
                } else {
                    None
                }
            }
        };
        let (sub_k, sub_l) = match mode {
            CommonMoverDispatchMode::Normal => (
                if topology.sub_k {
                    Some(CommonMoverDispatchPhase::SubK)
                } else {
                    None
                },
                if topology.sub_l {
                    Some(CommonMoverDispatchPhase::SubL)
                } else {
                    None
                },
            ),
            CommonMoverDispatchMode::Restricted => (None, None),
        };

        Self {
            phases: [primary, sub_k, sub_l],
        }
    }

    pub const fn phases(self) -> [Option<CommonMoverDispatchPhase>; 3] {
        self.phases
    }

    /// Whether the normal dispatcher selects only Sub-I.
    ///
    /// A/B/C/D are adjacent common-mover phases rather than callbacks owned by
    /// `FUN_004018A0`, so callers must still validate those separately.
    pub const fn is_sub_i_only(self) -> bool {
        matches!(
            self.phases,
            [Some(CommonMoverDispatchPhase::SubI), None, None]
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn topology() -> CommonMoverComponentTopology {
        CommonMoverComponentTopology::default()
    }

    #[test]
    fn normal_primary_priority_is_f_then_h_then_i_then_g() {
        type EnableComponent = fn(&mut CommonMoverComponentTopology);
        let expected: [(CommonMoverDispatchPhase, EnableComponent); 4] = [
            (
                CommonMoverDispatchPhase::SubF,
                |value: &mut CommonMoverComponentTopology| {
                    value.sub_f = true;
                },
            ),
            (
                CommonMoverDispatchPhase::SubH,
                |value: &mut CommonMoverComponentTopology| {
                    value.sub_h = true;
                },
            ),
            (
                CommonMoverDispatchPhase::SubI,
                |value: &mut CommonMoverComponentTopology| {
                    value.sub_i = true;
                },
            ),
            (
                CommonMoverDispatchPhase::SubG,
                |value: &mut CommonMoverComponentTopology| {
                    value.sub_g = true;
                },
            ),
        ];

        for (index, (phase, enable)) in expected.into_iter().enumerate() {
            let mut components = topology();
            for (_, enable_higher_priority) in expected.iter().skip(index) {
                enable_higher_priority(&mut components);
            }
            enable(&mut components);
            assert_eq!(
                CommonMoverDispatchPlan::from_topology(
                    components,
                    CommonMoverDispatchMode::Normal,
                )
                .phases()[0],
                Some(phase)
            );
        }
    }

    #[test]
    fn normal_k_and_l_suffixes_are_independent_and_ordered() {
        let mut components = topology();
        components.sub_k = true;
        components.sub_l = true;
        assert_eq!(
            CommonMoverDispatchPlan::from_topology(components, CommonMoverDispatchMode::Normal,)
                .phases(),
            [
                None,
                Some(CommonMoverDispatchPhase::SubK),
                Some(CommonMoverDispatchPhase::SubL),
            ]
        );

        components.sub_i = true;
        assert_eq!(
            CommonMoverDispatchPlan::from_topology(components, CommonMoverDispatchMode::Normal,)
                .phases(),
            [
                Some(CommonMoverDispatchPhase::SubI),
                Some(CommonMoverDispatchPhase::SubK),
                Some(CommonMoverDispatchPhase::SubL),
            ]
        );
    }

    #[test]
    fn restricted_mode_selects_f_else_g_and_skips_every_suffix() {
        let mut components = topology();
        components.sub_h = true;
        components.sub_i = true;
        components.sub_g = true;
        components.sub_k = true;
        components.sub_l = true;
        assert_eq!(
            CommonMoverDispatchPlan::from_topology(
                components,
                CommonMoverDispatchMode::Restricted,
            )
            .phases(),
            [Some(CommonMoverDispatchPhase::SubG), None, None]
        );

        components.sub_f = true;
        assert_eq!(
            CommonMoverDispatchPlan::from_topology(
                components,
                CommonMoverDispatchMode::from_retail_word(-7),
            )
            .phases(),
            [Some(CommonMoverDispatchPhase::SubF), None, None]
        );
    }

    #[test]
    fn sub_i_only_requires_no_preemptor_or_suffix() {
        let mut components = topology();
        components.sub_i = true;
        assert!(CommonMoverDispatchPlan::from_topology(
            components,
            CommonMoverDispatchMode::Normal,
        )
        .is_sub_i_only());

        for mutate in [
            (|value: &mut CommonMoverComponentTopology| value.sub_f = true)
                as fn(&mut CommonMoverComponentTopology),
            |value| value.sub_h = true,
            |value| value.sub_k = true,
            |value| value.sub_l = true,
        ] {
            let mut unsupported = components;
            mutate(&mut unsupported);
            assert!(!CommonMoverDispatchPlan::from_topology(
                unsupported,
                CommonMoverDispatchMode::Normal,
            )
            .is_sub_i_only());
        }
    }
}
