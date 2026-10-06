//! Detached `FUN_00402CA0` hook installed at task `+0x20` by `00403B70`.
//!
//! Static contact has no pair half-space test and does not call the pair
//! hook's `00401A20` effect. Sub-I turns heading by `0x2000`; without Sub-I,
//! the hook reverses the task direction for 2,500 ms and propagates that
//! direction through A/F/G. Both paths then retarget X and Z with two RNG
//! words. The live owner supplies authenticated actor/component snapshots
//! and commits this plan only while the same Following task survives.

use super::FollowBeaconsFollowingTaskState;
use crate::common_mover::SubAPropulsionRuntime;
use crate::entity_collision_state::RetailRuntimeValue;
use crate::wander_near_location::WanderNearPrivateState;

pub const FOLLOW_BEACONS_STATIC_CONTACT_CALLBACK_ADDRESS: u32 = 0x0040_2CA0;
pub const FOLLOW_BEACONS_STATIC_CONTACT_DIRECTION_WRITER_ADDRESS: u32 = 0x0040_19C0;
pub const FOLLOW_BEACONS_STATIC_CONTACT_REVERSAL_TIMER_MS: i32 = 2_500;
pub const FOLLOW_BEACONS_STATIC_CONTACT_HEADING_DELTA_RAW: u16 = 0x2000;

/// The consumed subset of the Section-12 common component table at `+0xC8`.
/// `sub_a: None` means the authored pointer is absent; a present component
/// carries its actual runtime, including any unresolved untouched fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowBeaconsStaticContactTopology {
    pub sub_i: bool,
    pub sub_a: Option<SubAPropulsionRuntime>,
    pub sub_f: bool,
    pub sub_g: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowBeaconsStaticContactRequest {
    pub task: FollowBeaconsFollowingTaskState,
    pub controlled_position_raw: [i16; 3],
    /// Entity word `+0xA2`, independent of body roll at `+0xA6`.
    pub heading_raw: u16,
    pub topology: RetailRuntimeValue<FollowBeaconsStaticContactTopology>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowBeaconsStaticContactBlock {
    UnresolvedTopology,
}

/// Expected inputs accompany every planned write for live-owner validation.
/// The complete updated task preserves target identity and elapsed time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowBeaconsStaticContactPlan {
    pub expected_task: FollowBeaconsFollowingTaskState,
    pub expected_position_raw: [i16; 3],
    pub expected_heading_raw: u16,
    pub expected_topology: FollowBeaconsStaticContactTopology,
    pub task_after: FollowBeaconsFollowingTaskState,
    pub heading_raw: u16,
    /// Present only when `004019C0` writes a present Sub-A allocation.
    pub sub_a_runtime: Option<SubAPropulsionRuntime>,
    pub sub_f_reverse_write: Option<bool>,
    pub sub_g_reverse_write: Option<bool>,
    pub rng_draw_count: u8,
}

/// Plan one admitted static-contact hook without mutating any live owner.
///
/// All validation precedes RNG. Retail consumes exactly two words, X then Z,
/// and uses only each unsigned low word shifted by six. Target Y, tracked
/// entity, lifetime, propulsion speed and drive scale are preserved. Unlike
/// `00401A20`, this hook does not call the Sub-D immediate reversal writer.
pub fn plan_follow_beacons_static_contact(
    request: FollowBeaconsStaticContactRequest,
    next_random: impl FnMut() -> u32,
) -> Result<FollowBeaconsStaticContactPlan, FollowBeaconsStaticContactBlock> {
    let shared = plan_wander_private_static_contact(
        WanderPrivateStaticContactRequest {
            private_state: request.task.private_state,
            controlled_position_raw: request.controlled_position_raw,
            heading_raw: request.heading_raw,
            topology: request.topology,
        },
        next_random,
    )?;
    let mut task_after = request.task;
    task_after.private_state = shared.private_state_after;
    Ok(FollowBeaconsStaticContactPlan {
        expected_task: request.task,
        expected_position_raw: request.controlled_position_raw,
        expected_heading_raw: request.heading_raw,
        expected_topology: shared.topology,
        task_after,
        heading_raw: shared.heading_raw,
        sub_a_runtime: shared.sub_a_runtime,
        sub_f_reverse_write: shared.sub_f_reverse_write,
        sub_g_reverse_write: shared.sub_g_reverse_write,
        rng_draw_count: 2,
    })
}

///02B10,03650,03B70 and03E20 install the identical02CA0 callback.
/// Only their shared private record is written; typed task owners preserve
/// their own lifetime, route and audio records when committing this result.
pub(crate) struct WanderPrivateStaticContactRequest {
    pub private_state: WanderNearPrivateState,
    pub controlled_position_raw: [i16; 3],
    pub heading_raw: u16,
    pub topology: RetailRuntimeValue<FollowBeaconsStaticContactTopology>,
}

pub(crate) struct WanderPrivateStaticContactPlan {
    pub private_state_after: WanderNearPrivateState,
    pub topology: FollowBeaconsStaticContactTopology,
    pub heading_raw: u16,
    pub sub_a_runtime: Option<SubAPropulsionRuntime>,
    pub sub_f_reverse_write: Option<bool>,
    pub sub_g_reverse_write: Option<bool>,
}

pub(crate) fn plan_wander_private_static_contact(
    request: WanderPrivateStaticContactRequest,
    mut next_random: impl FnMut() -> u32,
) -> Result<WanderPrivateStaticContactPlan, FollowBeaconsStaticContactBlock> {
    let topology = match request.topology {
        RetailRuntimeValue::Known(topology) => topology,
        RetailRuntimeValue::Unresolved => {
            return Err(FollowBeaconsStaticContactBlock::UnresolvedTopology);
        }
    };
    let mut private_state_after = request.private_state;
    let mut heading_raw = request.heading_raw;
    let mut sub_a_runtime = None;
    let mut sub_f_reverse_write = None;
    let mut sub_g_reverse_write = None;
    if topology.sub_i {
        heading_raw = heading_raw.wrapping_add(FOLLOW_BEACONS_STATIC_CONTACT_HEADING_DELTA_RAW);
    } else {
        private_state_after.reversal_timer_ms = FOLLOW_BEACONS_STATIC_CONTACT_REVERSAL_TIMER_MS;
        private_state_after.direction = private_state_after.direction.wrapping_neg();
        let direction = private_state_after.direction;
        // 004019C0 visits A, F, then G. The heading-only branch skips it.
        sub_a_runtime = topology.sub_a.map(|mut runtime| {
            runtime.set_direction_multiplier(direction);
            runtime
        });
        sub_f_reverse_write = topology.sub_f.then_some(direction == -1);
        sub_g_reverse_write = topology.sub_g.then_some(direction == -1);
    }

    let x_offset = contact_target_offset(next_random());
    private_state_after.target_position_raw[0] =
        request.controlled_position_raw[0].wrapping_add(x_offset);
    let z_offset = contact_target_offset(next_random());
    private_state_after.target_position_raw[2] =
        request.controlled_position_raw[2].wrapping_add(z_offset);

    Ok(WanderPrivateStaticContactPlan {
        topology,
        private_state_after,
        heading_raw,
        sub_a_runtime,
        sub_f_reverse_write,
        sub_g_reverse_write,
    })
}

fn contact_target_offset(random_word: u32) -> i16 {
    (((random_word as u16) >> 6) as i16).wrapping_sub(0x200)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wander_near_location::WanderNearPrivateState;

    fn task(direction: i32) -> FollowBeaconsFollowingTaskState {
        FollowBeaconsFollowingTaskState::from_parts(
            WanderNearPrivateState {
                target_position_raw: [123, -456, 789],
                tracked_entity_handle: 0x047F_0001,
                direction,
                reversal_timer_ms: 91,
            },
            8_999,
        )
    }

    fn request(direction: i32) -> FollowBeaconsStaticContactRequest {
        FollowBeaconsStaticContactRequest {
            task: task(direction),
            controlled_position_raw: [i16::MIN, 222, i16::MAX],
            heading_raw: 0xF123,
            topology: RetailRuntimeValue::Known(FollowBeaconsStaticContactTopology {
                sub_i: false,
                sub_a: Some(SubAPropulsionRuntime::from_retail_words(
                    RetailRuntimeValue::Known(333),
                    17,
                    73,
                )),
                sub_f: false,
                sub_g: false,
            }),
        }
    }

    #[test]
    fn type17_reverses_only_direction_and_retargets_with_two_ordered_words() {
        let request = request(1);
        let words = [0xFFFF_0000, 0x1234_FFFF];
        let mut draws = 0;
        let plan = plan_follow_beacons_static_contact(request, || {
            let word = words[draws];
            draws += 1;
            word
        })
        .unwrap();
        assert_eq!(draws, 2);
        assert_eq!(plan.rng_draw_count, 2);
        assert_eq!(plan.expected_task, request.task);
        assert_eq!(plan.expected_position_raw, request.controlled_position_raw);
        assert_eq!(plan.expected_heading_raw, request.heading_raw);
        assert_eq!(
            plan.task_after.private_state(),
            WanderNearPrivateState {
                target_position_raw: [32256, -456, -32258],
                tracked_entity_handle: 0x047F_0001,
                direction: -1,
                reversal_timer_ms: 2_500,
            }
        );
        assert_eq!(plan.task_after.elapsed_ms(), 8_999);
        assert_eq!(plan.heading_raw, request.heading_raw);
        assert_eq!(
            plan.sub_a_runtime,
            Some(SubAPropulsionRuntime::from_retail_words(
                RetailRuntimeValue::Known(333),
                -1,
                73,
            ))
        );
        assert_eq!(plan.sub_f_reverse_write, None);
        assert_eq!(plan.sub_g_reverse_write, None);
        assert_eq!(request.task, task(1));
    }

    #[test]
    fn sub_i_turns_heading_without_reversing_or_propagating_direction() {
        let mut request = request(-1);
        let RetailRuntimeValue::Known(mut topology) = request.topology else {
            unreachable!();
        };
        topology.sub_i = true;
        topology.sub_f = true;
        topology.sub_g = true;
        request.topology = RetailRuntimeValue::Known(topology);
        let mut draws = 0;
        let plan = plan_follow_beacons_static_contact(request, || {
            draws += 1;
            0x8000
        })
        .unwrap();
        assert_eq!(draws, 2);
        assert_eq!(plan.heading_raw, 0x1123);
        assert_eq!(plan.task_after.private_state().direction, -1);
        assert_eq!(plan.task_after.private_state().reversal_timer_ms, 91);
        assert_eq!(
            plan.task_after.private_state().target_position_raw,
            [i16::MIN, -456, i16::MAX]
        );
        assert_eq!(plan.task_after.target_id(), request.task.target_id());
        assert_eq!(plan.task_after.elapsed_ms(), request.task.elapsed_ms());
        assert_eq!(plan.sub_a_runtime, None);
        assert_eq!(plan.sub_f_reverse_write, None);
        assert_eq!(plan.sub_g_reverse_write, None);
    }

    #[test]
    fn reversal_wraps_and_animation_reverse_requires_exact_negative_one() {
        for (before, after, reverse) in [
            (1, -1, true),
            (-1, 1, false),
            (0, 0, false),
            (2, -2, false),
            (i32::MIN, i32::MIN, false),
        ] {
            let mut request = request(before);
            request.topology = RetailRuntimeValue::Known(FollowBeaconsStaticContactTopology {
                sub_i: false,
                sub_a: Some(SubAPropulsionRuntime::from_retail_words(
                    RetailRuntimeValue::Unresolved,
                    99,
                    -3,
                )),
                sub_f: true,
                sub_g: true,
            });
            let plan = plan_follow_beacons_static_contact(request, || 0x8000).unwrap();
            assert_eq!(plan.task_after.private_state().direction, after);
            assert_eq!(plan.task_after.private_state().reversal_timer_ms, 2_500);
            let sub_a = plan.sub_a_runtime.unwrap();
            assert_eq!(sub_a.direction_multiplier(), after);
            assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Unresolved);
            assert_eq!(sub_a.drive_scale_percent(), -3);
            assert_eq!(plan.sub_f_reverse_write, Some(reverse));
            assert_eq!(plan.sub_g_reverse_write, Some(reverse));
        }
    }

    #[test]
    fn absent_components_do_not_suppress_task_reversal_or_rng() {
        let mut request = request(1);
        request.topology = RetailRuntimeValue::Known(FollowBeaconsStaticContactTopology {
            sub_i: false,
            sub_a: None,
            sub_f: false,
            sub_g: false,
        });
        let mut draws = 0;
        let plan = plan_follow_beacons_static_contact(request, || {
            draws += 1;
            0
        })
        .unwrap();
        assert_eq!(draws, 2);
        assert_eq!(plan.task_after.private_state().direction, -1);
        assert_eq!(plan.task_after.private_state().reversal_timer_ms, 2_500);
        assert_eq!(plan.sub_a_runtime, None);
    }

    #[test]
    fn unresolved_topology_blocks_before_rng_or_task_changes() {
        let mut request = request(1);
        request.topology = RetailRuntimeValue::Unresolved;
        assert_eq!(
            plan_follow_beacons_static_contact(request, || panic!("no unresolved RNG")),
            Err(FollowBeaconsStaticContactBlock::UnresolvedTopology)
        );
        assert_eq!(request.task, task(1));
    }
}
