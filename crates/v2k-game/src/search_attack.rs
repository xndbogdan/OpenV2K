//! Pure retail class-7 `"Search And Attack Target"` planning.
//!
//! `FUN_00422CD0` performs deterministic target acquisition over the intrusive
//! live-entity list. The three style initializers then publish a
//! failure-order-sensitive task graph. This module retains those closed
//! decisions without running the shared mover, firing a projectile, playing
//! audio, consuming RNG, or mutating an entity. The separate
//! [`crate::search_attack_owner`] module applies the authenticated task-setup
//! order through the shared mutation-safe owner.

use std::num::NonZeroU32;

use crate::entity_collision_state::{
    recent_relation_suppresses_pair, EntityCollisionRuntimeState, RetailRuntimeValue,
    RetailStateWord, DYING_STATE_BIT,
};
use crate::wrapped_axis_range::{within_wrapped_axis_range, WrappedAxisRange};

pub const SEARCH_ATTACK_BEHAVIOR_CLASS_ID: u8 = 7;
pub const SEARCH_ATTACK_DESCRIPTOR_ADDRESS: u32 = 0x004C_88A8;
pub const SEARCH_ATTACK_STYLE_SWITCH_ADDRESS: u32 = 0x0040_C6B0;
pub const SEARCH_ATTACK_TARGET_HANDOFF_CALLBACK_ADDRESS: u32 = 0x0040_C7D0;
pub const SEARCH_ATTACK_TARGET_CONTEXT_OFFSET: u16 = 0x08;
pub const SEARCH_ATTACK_NO_BOUNDED_TARGET_TAG_ADDRESS: u32 = 0x004B_E8C0;

pub const SEARCH_ATTACK_FILTER_TYPE_STATE_OFFSET: u16 = 0x04;
pub const SEARCH_ATTACK_OPTIONAL_ATTACK_SOUND_TYPE_OFFSET: u16 = 0x9A;
pub const SEARCH_ATTACK_ATTACK_PARAMETER_TYPE_OFFSETS: &[u16] = &[0x9C, 0xA8];

/// Class-7's name for the shared `FUN_00423030` policy.
pub type SearchAttackRadius = WrappedAxisRange;

/// A zero mask means same entity type; a nonzero mask replaces that check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchAttackCandidateFilter {
    SameEntityType,
    CapabilityMask(NonZeroU32),
}

impl SearchAttackCandidateFilter {
    pub const fn from_raw(raw: u32) -> Self {
        match NonZeroU32::new(raw) {
            Some(mask) => Self::CapabilityMask(mask),
            None => Self::SameEntityType,
        }
    }

    pub const fn raw(self) -> u32 {
        match self {
            Self::SameEntityType => 0,
            Self::CapabilityMask(mask) => mask.get(),
        }
    }
}

/// Exact fields read from one resolved retail list entry.
///
/// The slice supplied to [`select_search_attack_target`] must remain in
/// intrusive-list order. Entries whose handles did not resolve are omitted;
/// unresolved live fields on a resolved entry fail closed.
#[derive(Debug, Clone, Copy)]
pub struct SearchAttackEntityRef<'a> {
    pub id: u32,
    pub entity_type: u32,
    pub position_raw: [i16; 3],
    pub capability_flags: RetailRuntimeValue<u32>,
    pub collision: &'a EntityCollisionRuntimeState,
}

#[derive(Debug, Clone, Copy)]
pub struct SearchAttackAcquisitionRequest<'a> {
    pub owner: SearchAttackEntityRef<'a>,
    pub candidates_in_intrusive_order: &'a [SearchAttackEntityRef<'a>],
    pub radius: SearchAttackRadius,
    pub filter: SearchAttackCandidateFilter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchAttackTarget {
    pub id: u32,
    pub scaled_distance_squared_raw: i32,
}

/// Retail return status and output mutation are independent.
///
/// In the ordinary authored domain a tagged status has no output. Keeping the
/// output here is necessary for wrapping-square edge cases where retail writes
/// a handle and still returns the tag because the final distance equals the
/// original squared-radius sentinel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchAttackTargetSelection {
    Success {
        output_write: Option<SearchAttackTarget>,
    },
    TaggedNoBoundedTarget {
        output_write: Option<SearchAttackTarget>,
        retail_tag_address: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchAttackSelectionError {
    CandidateStateUnresolved { id: u32 },
    CandidateCapabilityUnresolved { id: u32 },
    RecentRelationUnresolved { id: u32 },
}

/// Select one class-7 target without dispatching the successful callback.
///
/// This deliberately preserves `FUN_00422CD0`'s unusual `best == 0` sentinel:
/// a zero-distance target leaves the sentinel armed, so the next accepted
/// candidate replaces it regardless of distance.
pub fn select_search_attack_target(
    request: SearchAttackAcquisitionRequest<'_>,
) -> Result<SearchAttackTargetSelection, SearchAttackSelectionError> {
    let radius_raw = request.radius.raw();
    let initial_distance = radius_raw.wrapping_mul(radius_raw);
    let mut best_distance = initial_distance;
    let mut selected = None;

    for candidate in request.candidates_in_intrusive_order {
        if candidate.id == request.owner.id {
            continue;
        }
        if !candidate_state_is_eligible(candidate.id, candidate.collision.state_flags_at_0x08)? {
            continue;
        }

        match recent_relation_suppresses_pair(
            request.owner.id,
            request.owner.collision,
            candidate.id,
            candidate.collision,
        ) {
            RetailRuntimeValue::Known(true) => continue,
            RetailRuntimeValue::Known(false) => {}
            RetailRuntimeValue::Unresolved => {
                return Err(SearchAttackSelectionError::RecentRelationUnresolved {
                    id: candidate.id,
                });
            }
        }

        match request.filter {
            SearchAttackCandidateFilter::SameEntityType => {
                if candidate.entity_type != request.owner.entity_type {
                    continue;
                }
            }
            SearchAttackCandidateFilter::CapabilityMask(mask) => {
                let capability_flags = match candidate.capability_flags {
                    RetailRuntimeValue::Known(flags) => flags,
                    RetailRuntimeValue::Unresolved => {
                        return Err(SearchAttackSelectionError::CandidateCapabilityUnresolved {
                            id: candidate.id,
                        });
                    }
                };
                if capability_flags & mask.get() == 0 {
                    continue;
                }
            }
        }

        let distance =
            scaled_wrapping_distance_squared(request.owner.position_raw, candidate.position_raw);
        if distance < best_distance || best_distance == 0 {
            best_distance = distance;
            selected = Some(SearchAttackTarget {
                id: candidate.id,
                scaled_distance_squared_raw: distance,
            });
        }
    }

    if radius_raw != 0 && best_distance == initial_distance {
        Ok(SearchAttackTargetSelection::TaggedNoBoundedTarget {
            output_write: selected,
            retail_tag_address: SEARCH_ATTACK_NO_BOUNDED_TARGET_TAG_ADDRESS,
        })
    } else {
        Ok(SearchAttackTargetSelection::Success {
            output_write: selected,
        })
    }
}

fn candidate_state_is_eligible(
    id: u32,
    state: RetailStateWord,
) -> Result<bool, SearchAttackSelectionError> {
    if state.known_value_bits() == 0 {
        if state.known_mask() == u32::MAX {
            return Ok(false);
        }
        return Err(SearchAttackSelectionError::CandidateStateUnresolved { id });
    }

    match state.masked(DYING_STATE_BIT) {
        RetailRuntimeValue::Known(0) => Ok(true),
        RetailRuntimeValue::Known(_) => Ok(false),
        RetailRuntimeValue::Unresolved => {
            Err(SearchAttackSelectionError::CandidateStateUnresolved { id })
        }
    }
}

fn scaled_wrapping_distance_squared(first: [i16; 3], second: [i16; 3]) -> i32 {
    first
        .into_iter()
        .zip(second)
        .fold(0_i32, |sum, (first, second)| {
            let delta = i32::from(first.wrapping_sub(second));
            sum.wrapping_add(delta.wrapping_mul(delta) >> 2)
        })
}

/// Exact `FUN_00423030` strict wrapped-coordinate cube test.
pub fn search_attack_target_within_axis_range(
    radius: SearchAttackRadius,
    source_raw: [i16; 3],
    target_raw: [i16; 3],
) -> bool {
    within_wrapped_axis_range(radius, source_raw, target_raw)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchAttackVariant {
    Acquiring = 0,
    Pursuing = 1,
    ExternalEvent = 2,
}

impl SearchAttackVariant {
    pub const fn style_address(self) -> u32 {
        match self {
            Self::Acquiring => 0x004C_7A50,
            Self::Pursuing => 0x004C_7A98,
            Self::ExternalEvent => 0x004C_7AE0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchAttackTaskRole {
    AcquireTarget,
    Wander,
    AimAndFire,
    PursueTarget,
    ExternalEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchAttackTaskLifetime {
    CallbackOwned,
    FixedMilliseconds(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchAttackTaskSpec {
    pub role: SearchAttackTaskRole,
    pub slot: u8,
    pub constructor_address: u32,
    pub tick_address: u32,
    pub lifetime: SearchAttackTaskLifetime,
}

/// One ordered setup phase. A failed install stops the initializer before any
/// later phase, including that later phase's `clear_slots_before` writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchAttackSetupPhase {
    pub clear_slots_before: &'static [u8],
    pub install: SearchAttackTaskSpec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchAttackVariantSetupSpec {
    pub variant: SearchAttackVariant,
    pub style_address: u32,
    pub initializer_address: u32,
    pub target_required: bool,
    pub filter_type_state_offset: Option<u16>,
    pub optional_sound_type_offset: Option<u16>,
    pub task_parameter_type_offsets: &'static [u16],
    pub initial_clear_slots: &'static [u8],
    pub ordered_phases: &'static [SearchAttackSetupPhase],
}

const ACQUIRE_TARGET_TASK: SearchAttackTaskSpec = SearchAttackTaskSpec {
    role: SearchAttackTaskRole::AcquireTarget,
    slot: 1,
    constructor_address: 0x0040_2050,
    tick_address: 0x0040_2080,
    lifetime: SearchAttackTaskLifetime::CallbackOwned,
};
const WANDER_TASK: SearchAttackTaskSpec = SearchAttackTaskSpec {
    role: SearchAttackTaskRole::Wander,
    slot: 0,
    constructor_address: 0x0040_2B10,
    tick_address: 0x0040_2BA0,
    lifetime: SearchAttackTaskLifetime::FixedMilliseconds(500),
};
const AIM_AND_FIRE_TASK: SearchAttackTaskSpec = SearchAttackTaskSpec {
    role: SearchAttackTaskRole::AimAndFire,
    slot: 2,
    constructor_address: 0x0040_2220,
    tick_address: 0x0040_2300,
    lifetime: SearchAttackTaskLifetime::FixedMilliseconds(5_000),
};
const PURSUE_TARGET_TASK: SearchAttackTaskSpec = SearchAttackTaskSpec {
    role: SearchAttackTaskRole::PursueTarget,
    slot: 0,
    constructor_address: 0x0040_3360,
    tick_address: 0x0040_3490,
    lifetime: SearchAttackTaskLifetime::FixedMilliseconds(5_000),
};
const EXTERNAL_EVENT_TASK: SearchAttackTaskSpec = SearchAttackTaskSpec {
    role: SearchAttackTaskRole::ExternalEvent,
    slot: 0,
    constructor_address: 0x0040_3230,
    tick_address: 0x0040_3250,
    lifetime: SearchAttackTaskLifetime::CallbackOwned,
};

const ACQUIRING_PHASES: &[SearchAttackSetupPhase] = &[
    SearchAttackSetupPhase {
        clear_slots_before: &[],
        install: ACQUIRE_TARGET_TASK,
    },
    SearchAttackSetupPhase {
        clear_slots_before: &[],
        install: WANDER_TASK,
    },
];
const PURSUING_PHASES: &[SearchAttackSetupPhase] = &[
    SearchAttackSetupPhase {
        clear_slots_before: &[],
        install: AIM_AND_FIRE_TASK,
    },
    SearchAttackSetupPhase {
        clear_slots_before: &[1],
        install: PURSUE_TARGET_TASK,
    },
];
const EXTERNAL_EVENT_PHASES: &[SearchAttackSetupPhase] = &[SearchAttackSetupPhase {
    clear_slots_before: &[],
    install: EXTERNAL_EVENT_TASK,
}];

pub const fn search_attack_variant_setup(
    variant: SearchAttackVariant,
) -> SearchAttackVariantSetupSpec {
    match variant {
        SearchAttackVariant::Acquiring => SearchAttackVariantSetupSpec {
            variant,
            style_address: variant.style_address(),
            initializer_address: 0x0040_B6C0,
            target_required: false,
            filter_type_state_offset: Some(SEARCH_ATTACK_FILTER_TYPE_STATE_OFFSET),
            optional_sound_type_offset: None,
            task_parameter_type_offsets: &[],
            initial_clear_slots: &[2],
            ordered_phases: ACQUIRING_PHASES,
        },
        SearchAttackVariant::Pursuing => SearchAttackVariantSetupSpec {
            variant,
            style_address: variant.style_address(),
            initializer_address: 0x0040_ADE0,
            target_required: true,
            filter_type_state_offset: None,
            optional_sound_type_offset: Some(SEARCH_ATTACK_OPTIONAL_ATTACK_SOUND_TYPE_OFFSET),
            task_parameter_type_offsets: SEARCH_ATTACK_ATTACK_PARAMETER_TYPE_OFFSETS,
            initial_clear_slots: &[],
            ordered_phases: PURSUING_PHASES,
        },
        SearchAttackVariant::ExternalEvent => SearchAttackVariantSetupSpec {
            variant,
            style_address: variant.style_address(),
            initializer_address: 0x0040_ADB0,
            target_required: false,
            filter_type_state_offset: None,
            optional_sound_type_offset: None,
            task_parameter_type_offsets: &[],
            initial_clear_slots: &[2, 1],
            ordered_phases: EXTERNAL_EVENT_PHASES,
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchAttackTargetHandoff {
    pub target_id: u32,
    pub target_context_offset: u16,
    pub callback_address: u32,
    pub style_switch_address: u32,
    pub next_variant: SearchAttackVariant,
}

pub const fn search_attack_target_handoff(target: SearchAttackTarget) -> SearchAttackTargetHandoff {
    SearchAttackTargetHandoff {
        target_id: target.id,
        target_context_offset: SEARCH_ATTACK_TARGET_CONTEXT_OFFSET,
        callback_address: SEARCH_ATTACK_TARGET_HANDOFF_CALLBACK_ADDRESS,
        style_switch_address: SEARCH_ATTACK_STYLE_SWITCH_ADDRESS,
        next_variant: SearchAttackVariant::Pursuing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_collision_state::PAIR_COLLISION_INELIGIBLE_STATE_BIT;

    fn collision(state: RetailStateWord) -> EntityCollisionRuntimeState {
        EntityCollisionRuntimeState::from_constructor(None, 0, state)
    }

    fn entity<'a>(
        id: u32,
        entity_type: u32,
        position_raw: [i16; 3],
        capability_flags: RetailRuntimeValue<u32>,
        collision: &'a EntityCollisionRuntimeState,
    ) -> SearchAttackEntityRef<'a> {
        SearchAttackEntityRef {
            id,
            entity_type,
            position_raw,
            capability_flags,
            collision,
        }
    }

    fn selected(
        owner: SearchAttackEntityRef<'_>,
        candidates: &[SearchAttackEntityRef<'_>],
        radius: SearchAttackRadius,
        filter: SearchAttackCandidateFilter,
    ) -> Result<SearchAttackTargetSelection, SearchAttackSelectionError> {
        select_search_attack_target(SearchAttackAcquisitionRequest {
            owner,
            candidates_in_intrusive_order: candidates,
            radius,
            filter,
        })
    }

    fn success_target(id: u32, scaled_distance_squared_raw: i32) -> SearchAttackTargetSelection {
        SearchAttackTargetSelection::Success {
            output_write: Some(SearchAttackTarget {
                id,
                scaled_distance_squared_raw,
            }),
        }
    }

    fn success_without_output() -> SearchAttackTargetSelection {
        SearchAttackTargetSelection::Success { output_write: None }
    }

    fn tagged_without_output() -> SearchAttackTargetSelection {
        SearchAttackTargetSelection::TaggedNoBoundedTarget {
            output_write: None,
            retail_tag_address: SEARCH_ATTACK_NO_BOUNDED_TARGET_TAG_ADDRESS,
        }
    }

    fn tagged_with_target(
        id: u32,
        scaled_distance_squared_raw: i32,
    ) -> SearchAttackTargetSelection {
        SearchAttackTargetSelection::TaggedNoBoundedTarget {
            output_write: Some(SearchAttackTarget {
                id,
                scaled_distance_squared_raw,
            }),
            retail_tag_address: SEARCH_ATTACK_NO_BOUNDED_TARGET_TAG_ADDRESS,
        }
    }

    fn strict_radius(raw: i32) -> SearchAttackRadius {
        SearchAttackRadius::strict(raw).expect("test radius must be nonzero")
    }

    #[test]
    fn non_initial_styles_are_authenticated_with_exact_callback_slots() {
        assert_eq!(
            crate::entity_behavior::audited_behavior_style(7, 1),
            Some(&crate::entity_behavior::BehaviorStyle {
                class_id: 7,
                variant: 1,
                frame_address: 0x004C_7A98,
                release_callback_address: None,
                pair_contact_callback_address: None,
                impact_callback_address: Some(0x0040_C690),
                death_callback_address: None,
            })
        );
        assert_eq!(
            crate::entity_behavior::audited_behavior_style(7, 2),
            Some(&crate::entity_behavior::BehaviorStyle {
                class_id: 7,
                variant: 2,
                frame_address: 0x004C_7AE0,
                release_callback_address: Some(0x0040_CE90),
                pair_contact_callback_address: None,
                impact_callback_address: None,
                death_callback_address: None,
            })
        );
    }

    #[test]
    fn setup_specs_preserve_task_identity_duration_and_failure_order() {
        let acquiring = search_attack_variant_setup(SearchAttackVariant::Acquiring);
        assert_eq!(acquiring.initial_clear_slots, &[2]);
        assert_eq!(acquiring.filter_type_state_offset, Some(0x04));
        assert_eq!(
            acquiring
                .ordered_phases
                .iter()
                .map(|phase| phase.install)
                .collect::<Vec<_>>(),
            [ACQUIRE_TARGET_TASK, WANDER_TASK]
        );

        let pursuing = search_attack_variant_setup(SearchAttackVariant::Pursuing);
        assert!(pursuing.target_required);
        assert_eq!(pursuing.optional_sound_type_offset, Some(0x9A));
        assert_eq!(pursuing.task_parameter_type_offsets, &[0x9C, 0xA8]);
        assert_eq!(pursuing.ordered_phases[0].install, AIM_AND_FIRE_TASK);
        assert_eq!(pursuing.ordered_phases[1].clear_slots_before, &[1]);
        assert_eq!(pursuing.ordered_phases[1].install, PURSUE_TARGET_TASK);

        let external = search_attack_variant_setup(SearchAttackVariant::ExternalEvent);
        assert_eq!(external.initial_clear_slots, &[2, 1]);
        assert_eq!(
            external.ordered_phases[0].install,
            SearchAttackTaskSpec {
                role: SearchAttackTaskRole::ExternalEvent,
                slot: 0,
                constructor_address: 0x0040_3230,
                tick_address: 0x0040_3250,
                lifetime: SearchAttackTaskLifetime::CallbackOwned,
            }
        );
    }

    #[test]
    fn selector_rejects_owner_zero_dying_and_relation_but_not_state_1000() {
        let owner_collision = collision(RetailStateWord::exact(1));
        let zero_collision = collision(RetailStateWord::exact(0));
        let dying_collision = collision(RetailStateWord::exact(DYING_STATE_BIT));
        let state_1000_collision =
            collision(RetailStateWord::exact(PAIR_COLLISION_INELIGIBLE_STATE_BIT));
        let mut related_collision = collision(RetailStateWord::exact(1));
        related_collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(Some(1));
        related_collision.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(1);

        let owner = entity(
            1,
            17,
            [0; 3],
            RetailRuntimeValue::Known(0),
            &owner_collision,
        );
        let candidates = [
            owner,
            entity(
                2,
                17,
                [1, 0, 0],
                RetailRuntimeValue::Known(0),
                &zero_collision,
            ),
            entity(
                3,
                17,
                [2, 0, 0],
                RetailRuntimeValue::Known(0),
                &dying_collision,
            ),
            entity(
                4,
                17,
                [3, 0, 0],
                RetailRuntimeValue::Known(0),
                &related_collision,
            ),
            entity(
                5,
                17,
                [4, 0, 0],
                RetailRuntimeValue::Known(0),
                &state_1000_collision,
            ),
        ];
        assert_eq!(
            selected(
                owner,
                &candidates,
                SearchAttackRadius::Unbounded,
                SearchAttackCandidateFilter::SameEntityType,
            ),
            Ok(success_target(5, 4))
        );
    }

    #[test]
    fn zero_mask_matches_type_and_nonzero_mask_replaces_the_type_check() {
        let live = collision(RetailStateWord::exact(1));
        let owner = entity(1, 17, [0; 3], RetailRuntimeValue::Unresolved, &live);
        let candidates = [
            entity(2, 99, [1, 0, 0], RetailRuntimeValue::Known(0x20), &live),
            entity(3, 17, [2, 0, 0], RetailRuntimeValue::Unresolved, &live),
        ];
        assert_eq!(
            selected(
                owner,
                &candidates,
                SearchAttackRadius::Unbounded,
                SearchAttackCandidateFilter::SameEntityType,
            )
            .unwrap(),
            success_target(3, 1)
        );
        assert_eq!(
            selected(
                owner,
                &[
                    candidates[0],
                    entity(3, 17, [2, 0, 0], RetailRuntimeValue::Known(0), &live,),
                ],
                SearchAttackRadius::Unbounded,
                SearchAttackCandidateFilter::CapabilityMask(NonZeroU32::new(0x20).unwrap()),
            )
            .unwrap(),
            success_target(2, 0)
        );
    }

    #[test]
    fn wrapped_distance_strict_radius_and_equal_ties_match_intrusive_order() {
        let live = collision(RetailStateWord::exact(1));
        let owner = entity(
            1,
            17,
            [i16::MAX - 1, 0, 0],
            RetailRuntimeValue::Known(0),
            &live,
        );
        let candidates = [
            entity(
                2,
                17,
                [i16::MIN + 2, 0, 0],
                RetailRuntimeValue::Known(0),
                &live,
            ),
            entity(
                3,
                17,
                [i16::MAX - 5, 0, 0],
                RetailRuntimeValue::Known(0),
                &live,
            ),
            entity(
                4,
                17,
                [i16::MAX.wrapping_add(7), 0, 0],
                RetailRuntimeValue::Known(0),
                &live,
            ),
        ];
        assert_eq!(
            selected(
                owner,
                &candidates,
                strict_radius(5),
                SearchAttackCandidateFilter::SameEntityType,
            )
            .unwrap(),
            success_target(2, 4)
        );
        assert_eq!(
            selected(
                owner,
                &candidates[2..],
                strict_radius(4),
                SearchAttackCandidateFilter::SameEntityType,
            ),
            Ok(tagged_without_output())
        );
    }

    #[test]
    fn unbounded_empty_and_zero_distance_sentinel_preserve_retail_oddities() {
        let live = collision(RetailStateWord::exact(1));
        let owner = entity(1, 17, [0; 3], RetailRuntimeValue::Known(0), &live);
        assert_eq!(
            selected(
                owner,
                &[],
                SearchAttackRadius::Unbounded,
                SearchAttackCandidateFilter::SameEntityType,
            ),
            Ok(success_without_output())
        );

        let candidates = [
            entity(2, 17, [0; 3], RetailRuntimeValue::Known(0), &live),
            entity(3, 17, [100, 0, 0], RetailRuntimeValue::Known(0), &live),
        ];
        assert_eq!(
            selected(
                owner,
                &candidates,
                strict_radius(200),
                SearchAttackCandidateFilter::SameEntityType,
            )
            .unwrap(),
            success_target(3, 2_500)
        );

        // The return tag compares the final best distance to the original
        // wrapping radius square, independently of the output write.
        let quantized_zero = entity(4, 17, [1, 0, 0], RetailRuntimeValue::Known(0), &live);
        assert_eq!(
            selected(
                owner,
                &[quantized_zero],
                strict_radius(65_536),
                SearchAttackCandidateFilter::SameEntityType,
            ),
            Ok(tagged_with_target(4, 0))
        );
    }

    #[test]
    fn unresolved_live_fields_fail_closed_only_when_consumed() {
        let owner_collision = collision(RetailStateWord::exact(1));
        let unknown_state = collision(RetailStateWord::unknown());
        let owner = entity(
            1,
            17,
            [0; 3],
            RetailRuntimeValue::Known(0),
            &owner_collision,
        );
        let candidate = entity(
            2,
            17,
            [1, 0, 0],
            RetailRuntimeValue::Unresolved,
            &unknown_state,
        );
        assert_eq!(
            selected(
                owner,
                &[candidate],
                SearchAttackRadius::Unbounded,
                SearchAttackCandidateFilter::SameEntityType,
            ),
            Err(SearchAttackSelectionError::CandidateStateUnresolved { id: 2 })
        );

        let live = collision(RetailStateWord::exact(1));
        let mut unresolved_owner_relation = collision(RetailStateWord::exact(1));
        unresolved_owner_relation.recent_relation_id_at_0x60 = RetailRuntimeValue::Unresolved;
        let owner_with_unresolved_relation = entity(
            1,
            17,
            [0; 3],
            RetailRuntimeValue::Known(0),
            &unresolved_owner_relation,
        );
        let related_candidate = entity(4, 17, [1, 0, 0], RetailRuntimeValue::Known(0), &live);
        assert_eq!(
            selected(
                owner_with_unresolved_relation,
                &[related_candidate],
                SearchAttackRadius::Unbounded,
                SearchAttackCandidateFilter::SameEntityType,
            ),
            Err(SearchAttackSelectionError::RecentRelationUnresolved { id: 4 })
        );

        let candidate = entity(3, 99, [1, 0, 0], RetailRuntimeValue::Unresolved, &live);
        assert_eq!(
            selected(
                owner,
                &[candidate],
                SearchAttackRadius::Unbounded,
                SearchAttackCandidateFilter::SameEntityType,
            ),
            Ok(success_without_output())
        );
        assert_eq!(
            selected(
                owner,
                &[candidate],
                SearchAttackRadius::Unbounded,
                SearchAttackCandidateFilter::CapabilityMask(NonZeroU32::new(1).unwrap()),
            ),
            Err(SearchAttackSelectionError::CandidateCapabilityUnresolved { id: 3 })
        );
    }

    #[test]
    fn strict_axis_range_wraps_and_rejects_equal_or_negative_boundaries() {
        assert_eq!(SearchAttackRadius::strict(0), None);
        assert_eq!(
            SearchAttackRadius::from_raw(0),
            SearchAttackRadius::Unbounded
        );
        assert!(search_attack_target_within_axis_range(
            SearchAttackRadius::Unbounded,
            [0; 3],
            [i16::MIN; 3],
        ));
        assert!(search_attack_target_within_axis_range(
            strict_radius(5),
            [i16::MAX - 1, 0, 0],
            [i16::MIN + 2, 4, -4],
        ));
        assert!(!search_attack_target_within_axis_range(
            strict_radius(4),
            [0; 3],
            [4, 0, 0],
        ));
        assert!(!search_attack_target_within_axis_range(
            strict_radius(-1),
            [0; 3],
            [0; 3],
        ));
    }

    #[test]
    fn target_handoff_pins_context_callback_and_next_variant_without_side_effects() {
        assert_eq!(
            search_attack_target_handoff(SearchAttackTarget {
                id: 0x0497_0001,
                scaled_distance_squared_raw: 123,
            }),
            SearchAttackTargetHandoff {
                target_id: 0x0497_0001,
                target_context_offset: 0x08,
                callback_address: 0x0040_C7D0,
                style_switch_address: 0x0040_C6B0,
                next_variant: SearchAttackVariant::Pursuing,
            }
        );
    }
}
