//! Exact component mutation from the captured type-9 Main Base contact.
//!
//! The accepted
//! `20260727-235038-base-conversion-pair-tail.txt` trace identifies the
//! first-world source as type 9 and its slot-zero pair callback as
//! `FUN_00402DA0`. After that callback's authored forward-half-space gate,
//! descriptor effect `FUN_00401A20` took its non-null Sub-I branch:
//!
//! 1. entity heading word `+0xA2` changed by wrapping `+0x2000`;
//! 2. `FUN_004019C0` propagated that task's retained direction;
//! 3. descriptor `+0x08` selected the nested `FUN_004204B0` write to slot 3,
//!    the Sub-A propulsion runtime's direction dword at `+0x04`, to exactly
//!    `1`.
//!
//! The captured source changed heading `0xD7B1 -> 0xF7B1`, and the shared RNG
//! remained `0x9CA563C8` across the complete component effect. This module
//! consequently accepts no RNG owner and does not model any other descriptor
//! branch. It also does not claim the subsequent physical/damage suffix.
//! Native allocations use `main_base_person_contact`, which reads their actual
//! task direction and retained basis instead of this captured direction-1 oracle.

use crate::common_mover::SubAPropulsionRuntime;
use crate::entity::Entity;
use crate::entity_collision_state::{PairOrientationPolicy, RetailRuntimeValue};
use crate::hover::{dot_q31, HoverBasis};

/// Authored first-world peasant type accepted by the Main Base callback.
pub const FIRST_WORLD_MAIN_BASE_SOURCE_ENTITY_TYPE: u32 = 9;
/// The audited type-9 pair callback is installed in component slot zero.
pub const FIRST_WORLD_TYPE9_PAIR_COMPONENT_SLOT: usize = 0;
/// Type-9 slot-zero pair callback.
pub const FIRST_WORLD_TYPE9_PAIR_CALLBACK_ADDRESS: u32 = 0x0040_2DA0;
/// Descriptor effect reached after the callback's forward-half-space gate.
pub const TYPE9_DESCRIPTOR_CONTACT_EFFECT_ADDRESS: u32 = 0x0040_1A20;
/// Nested descriptor callback which forwards the captured value.
pub const TYPE9_DESCRIPTOR_NESTED_CALLBACK_ADDRESS: u32 = 0x0040_19C0;
/// Final writer targeting the Sub-A propulsion runtime `+0x04`.
pub const TYPE9_DESCRIPTOR_NESTED_WRITER_ADDRESS: u32 = 0x0042_04B0;
/// Wrapping delta applied to entity heading word `+0xA2`.
pub const TYPE9_MAIN_BASE_HEADING_DELTA_RAW: u16 = 0x2000;
/// Exact value written to Sub-A runtime `+0x04`.
pub const TYPE9_MAIN_BASE_DIRECTION_MULTIPLIER: i32 = 1;

/// Immutable preflight for the proven type-9 descriptor effect.
///
/// Both mutable owners are retained so commit can reject stale state before
/// either write becomes visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type9MainBaseComponentPlan {
    entity_id: u32,
    expected_heading_raw: u16,
    expected_sub_a_runtime: SubAPropulsionRuntime,
    resolved_heading_raw: u16,
    resolved_sub_a_runtime: SubAPropulsionRuntime,
}

impl Type9MainBaseComponentPlan {
    pub const fn entity_id(self) -> u32 {
        self.entity_id
    }

    pub const fn expected_heading_raw(self) -> u16 {
        self.expected_heading_raw
    }

    pub const fn resolved_heading_raw(self) -> u16 {
        self.resolved_heading_raw
    }
}

/// Why the captured component branch could not be fully preflighted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type9MainBaseComponentPlanError {
    WrongEntityType { entity_type: u32 },
    ForwardHalfSpaceUnresolved,
    SubAPropulsionRuntimeUnresolved,
    SubAPropulsionRuntimeAbsent,
}

/// A stale plan is rejected before either retained owner is changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type9MainBaseComponentCommitError {
    EntityIdentityChanged { expected: u32, actual: u32 },
    EntityTypeChanged { actual: u32 },
    HeadingChanged { expected: u16, actual: u16 },
    SubAPropulsionRuntimeChanged,
}

/// Auditable result of the two-field commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type9MainBaseComponentOutcome {
    pub entity_id: u32,
    pub heading_raw_before: u16,
    pub heading_raw_after: u16,
    pub direction_multiplier_before: i32,
    pub direction_multiplier_after: i32,
}

/// Evaluate retail `FUN_0041E930` for one retained live entity and target.
///
/// Retail subtracts the entity's three signed 8.8 position words from the
/// target with word wrapping, projects that displacement onto the entity's
/// stored signed-Q31 forward basis, and accepts projection zero as well as the
/// positive half-space. The port reconstructs that exact forward basis only
/// when the allocation retained a proven live-heading/authored-pitch-roll
/// policy; an unavailable basis remains explicitly unresolved.
///
/// The first-world type-9 Main Base component passes the contacted Main Base
/// position as `target_position_raw`, then forwards this value directly to
/// [`plan_type9_main_base_component`].
pub fn entity_forward_half_space(
    entity: &Entity,
    target_position_raw: [i16; 3],
) -> RetailRuntimeValue<bool> {
    match forward_projection_raw(entity, target_position_raw) {
        RetailRuntimeValue::Known(projection) => RetailRuntimeValue::Known(projection >= 0),
        RetailRuntimeValue::Unresolved => RetailRuntimeValue::Unresolved,
    }
}

fn forward_projection_raw(
    entity: &Entity,
    target_position_raw: [i16; 3],
) -> RetailRuntimeValue<i32> {
    let PairOrientationPolicy::LiveHeadingWithAuthoredPitchRoll {
        pitch_raw,
        roll_raw,
    } = match entity.collision.pair_callbacks.orientation_policy {
        RetailRuntimeValue::Known(policy) => policy,
        RetailRuntimeValue::Unresolved => return RetailRuntimeValue::Unresolved,
    };
    let forward = HoverBasis::from_angle_words(
        entity.heading_raw() as i16,
        pitch_raw as i16,
        roll_raw as i16,
    )
    .forward;
    let entity_position_raw = entity.position_raw();
    let displacement_raw = std::array::from_fn(|axis| {
        target_position_raw[axis].wrapping_sub(entity_position_raw[axis])
    });
    RetailRuntimeValue::Known(dot_q31(forward, displacement_raw))
}

/// Plan the exact type-9 descriptor effect after its authored directional
/// gate.
///
/// `Ok(None)` is the retail no-op when the half-space test rejects contact.
/// Unknown gate or component ownership fails closed. The function deliberately
/// accepts no RNG argument because this captured branch consumes none.
pub fn plan_type9_main_base_component(
    entity: &Entity,
    forward_half_space: RetailRuntimeValue<bool>,
) -> Result<Option<Type9MainBaseComponentPlan>, Type9MainBaseComponentPlanError> {
    if entity.entity_type != FIRST_WORLD_MAIN_BASE_SOURCE_ENTITY_TYPE {
        return Err(Type9MainBaseComponentPlanError::WrongEntityType {
            entity_type: entity.entity_type,
        });
    }

    match forward_half_space {
        RetailRuntimeValue::Known(false) => return Ok(None),
        RetailRuntimeValue::Known(true) => {}
        RetailRuntimeValue::Unresolved => {
            return Err(Type9MainBaseComponentPlanError::ForwardHalfSpaceUnresolved);
        }
    }

    let expected_sub_a_runtime = match entity.sub_a_propulsion_runtime {
        RetailRuntimeValue::Known(Some(runtime)) => runtime,
        RetailRuntimeValue::Known(None) => {
            return Err(Type9MainBaseComponentPlanError::SubAPropulsionRuntimeAbsent);
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type9MainBaseComponentPlanError::SubAPropulsionRuntimeUnresolved);
        }
    };
    let expected_heading_raw = entity.heading_raw();
    let mut resolved_sub_a_runtime = expected_sub_a_runtime;
    resolved_sub_a_runtime.set_direction_multiplier(TYPE9_MAIN_BASE_DIRECTION_MULTIPLIER);

    Ok(Some(Type9MainBaseComponentPlan {
        entity_id: entity.id,
        expected_heading_raw,
        expected_sub_a_runtime,
        resolved_heading_raw: expected_heading_raw.wrapping_add(TYPE9_MAIN_BASE_HEADING_DELTA_RAW),
        resolved_sub_a_runtime,
    }))
}

/// Atomically commit a previously preflighted type-9 component plan.
///
/// Validation covers both mutable fields before either assignment. A Main Base
/// suffix host may therefore combine this primitive with a separately proven
/// physical plan without risking a half-applied component effect.
pub fn commit_type9_main_base_component(
    entity: &mut Entity,
    plan: Type9MainBaseComponentPlan,
) -> Result<Type9MainBaseComponentOutcome, Type9MainBaseComponentCommitError> {
    validate_type9_main_base_component(entity, plan)?;

    entity.set_heading_raw(plan.resolved_heading_raw);
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(plan.resolved_sub_a_runtime));

    Ok(Type9MainBaseComponentOutcome {
        entity_id: entity.id,
        heading_raw_before: plan.expected_heading_raw,
        heading_raw_after: plan.resolved_heading_raw,
        direction_multiplier_before: plan.expected_sub_a_runtime.direction_multiplier(),
        direction_multiplier_after: plan.resolved_sub_a_runtime.direction_multiplier(),
    })
}

/// Validate both retained component owners without applying either write.
///
/// The concrete Main Base transaction uses this before validating its
/// independently planned physical suffix, then performs the now-infallible
/// component commit and both physical writes as one local commit phase.
pub(crate) fn validate_type9_main_base_component(
    entity: &Entity,
    plan: Type9MainBaseComponentPlan,
) -> Result<(), Type9MainBaseComponentCommitError> {
    if entity.id != plan.entity_id {
        return Err(Type9MainBaseComponentCommitError::EntityIdentityChanged {
            expected: plan.entity_id,
            actual: entity.id,
        });
    }
    if entity.entity_type != FIRST_WORLD_MAIN_BASE_SOURCE_ENTITY_TYPE {
        return Err(Type9MainBaseComponentCommitError::EntityTypeChanged {
            actual: entity.entity_type,
        });
    }

    let actual_heading_raw = entity.heading_raw();
    if actual_heading_raw != plan.expected_heading_raw {
        return Err(Type9MainBaseComponentCommitError::HeadingChanged {
            expected: plan.expected_heading_raw,
            actual: actual_heading_raw,
        });
    }
    if entity.sub_a_propulsion_runtime
        != RetailRuntimeValue::Known(Some(plan.expected_sub_a_runtime))
    {
        return Err(Type9MainBaseComponentCommitError::SubAPropulsionRuntimeChanged);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor_animation::ActorAnimationController;
    use crate::entity::{Entity, EntityKind};
    use crate::entity_collision_state::EntityCollisionRuntimeState;
    use v2k_formats::collision::ActorAnimationDescriptor;

    const TYPE9_ANIMATION: ActorAnimationDescriptor = ActorAnimationDescriptor {
        capability_bit_3_sound_id: 72,
        capability_mask_0x201_sound_id: 0,
        attention_stop_sound_id: 72,
        variable_binding: 1,
        frames_per_direction: 4,
    };

    fn type9_entity(heading_raw: u16) -> Entity {
        let mut controller =
            ActorAnimationController::from_descriptor(TYPE9_ANIMATION).expect("valid descriptor");
        controller.advance_neutral(20_000, 0xB000);
        let mut entity = Entity {
            construction_stamp_at_0xb4: RetailRuntimeValue::Unresolved,
            id: 0x04AC_0001,
            authored_spawn_index: Some(0),
            kind: EntityKind::Unknown(FIRST_WORLD_MAIN_BASE_SOURCE_ENTITY_TYPE),
            entity_type: FIRST_WORLD_MAIN_BASE_SOURCE_ENTITY_TYPE,
            authored_follow_beacon_priority_raw: None,
            power_up_payload_packed: None,
            auto_pilot_payload_packed: None,
            factory_type61_birth_provenance: None,
            type60_construction_provenance: None,
            main_base_type54_sea_delta_source:
                crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            position: [0.0; 3],
            heading: 0.0,
            pitch_roll_raw: [0; 2],
            physical_body_basis_q31: RetailRuntimeValue::Unresolved,
            velocity: [0.0; 3],
            surface_lifetime_timer_ms_at_0x48: RetailRuntimeValue::Known(0),
            mass_raw: 1,
            capability_flags: 0x800,
            attached_to: None,
            model_slots: [None; 4],
            model_index: None,
            collision: EntityCollisionRuntimeState::unresolved_port_entity(0),
            initial_behavior: RetailRuntimeValue::Unresolved,
            current_behavior_context: RetailRuntimeValue::Unresolved,
            authored_radial_emitter: None,
            sub_n_runtime: RetailRuntimeValue::Unresolved,
            base_factory_runtime: RetailRuntimeValue::Unresolved,
            actor_animation_runtime: RetailRuntimeValue::Known(Some(controller)),
            sub_a_propulsion_runtime: RetailRuntimeValue::Known(Some(
                SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(1_548), -1, 100),
            )),
            sub_g_06070_runtime: RetailRuntimeValue::Unresolved,
            intro2_type13_common_mover_runtime: None,
            native_type13_allocation: None,
            intro2_type13_aim_runtime: None,
            intro2_type16_aim_runtime: None,
            intro2_type58_aim_runtime: None,
            intro2_type94_aim_runtime: None,
            intro2_flyer_aim_runtime: None,
            sub_h_external_frame_runtime: RetailRuntimeValue::Unresolved,
            sub_j_attachment_runtime: RetailRuntimeValue::Unresolved,
            actor_common_axis_descriptor: RetailRuntimeValue::Unresolved,
            actor_tasks: Default::default(),
            ordinary_type9_pending_initial_selection: None,
            ordinary_type9_selected_component_runtime: None,
            main_base_type9_death_component_runtime: None,
            ordinary_type47_aim_and_fire_runtime: None,
            native_type61_allocation: None,
            native_type26_allocation: None,
            intro2_type26_sub_d_frame_owner: None,
            intro2_type26_sub_d_runtime: None,
            intro2_type47_sub_d_frame_owner: None,
            intro2_type47_sub_d_runtime: None,
            native_type47_construction: None,
            intro2_flyer_frame_owner: None,
            intro2_type53_runtime: None,
            native_type122_runtime: None,
            native_type30_runtime: None,
            native_type30_aim_runtime: None,
            native_type40_runtime: None,
            native_type40_aim_runtime: None,
            native_type43_runtime: None,
            native_type43_aim_runtime: None,
            native_type38_runtime: None,
            native_type38_aim_runtime: None,
            native_type18_runtime: None,
            native_type18_aim_runtime: None,
            native_type56_runtime: None,
            native_type56_aim_runtime: None,
            native_type122_aim_runtime: None,
            shared_fish_runtime: None,
            cleansing_vehicle_runtime: None,
            intro2_type16_runtime: None,
            intro2_type58_runtime: None,
            intro2_type66_runtime: None,
            intro2_gun_turret_runtime: None,
            intro2_gun_turret_aim_runtime: None,
            class49_death_runtime: None,
            native_entity_weapon_runtime: None,
            intro2_type10_runtime: None,
            intro2_type10_aim_runtime: None,
            intro2_type57_runtime: None,
            intro2_type57_aim_runtime: None,
            intro2_type94_runtime: None,
            intro2_type17_runtime: None,
            native_capture_relation: None,
            intro2_type8_runtime: None,
            native_type123_runtime: None,
            native_type123_anchor_raw_at_0x90: RetailRuntimeValue::Unresolved,
            native_type86_runtime: None,
            native_type86_anchor_raw_at_0x90: RetailRuntimeValue::Unresolved,
            intro2_type9_runtime: None,
            ordinary_type9_native_receipt: None,
            main_base_runtime: None,
            type17_sub_d_frame_owner: None,
            type17_sub_d_runtime: None,
            type8_sub_d_frame_owner: None,
            type8_wander_anchor_raw_at_0x90: RetailRuntimeValue::Unresolved,
            type8_sub_d_runtime: None,
            type47_immutable_anchor_raw_at_0x90: RetailRuntimeValue::Unresolved,
            active: true,
        };
        entity.set_heading_raw(heading_raw);
        entity.collision.pair_callbacks.orientation_policy =
            RetailRuntimeValue::Known(PairOrientationPolicy::LiveHeadingWithAuthoredPitchRoll {
                pitch_raw: 0,
                roll_raw: 0,
            });
        entity
    }

    fn world_from_raw(position_raw: [i16; 3]) -> [f32; 3] {
        position_raw.map(|word| f32::from(word) / 256.0)
    }

    #[test]
    fn captured_main_base_target_is_in_the_type9_forward_half_space() {
        let mut entity = type9_entity(0xD7B1);
        entity.position = world_from_raw([0x4E17, 0xFD1Bu16 as i16, 0x3CDF]);
        let main_base_position = [0x5000, 0xFD00u16 as i16, 0x3C00];

        assert_eq!(
            forward_projection_raw(&entity, main_base_position),
            RetailRuntimeValue::Known(454),
            "the accepted trace stores forward Q31 [464a0000, 0, 950f0000]"
        );
        assert_eq!(
            entity_forward_half_space(&entity, main_base_position),
            RetailRuntimeValue::Known(true)
        );
    }

    #[test]
    fn signed_position_subtraction_wraps_before_forward_projection() {
        let mut entity = type9_entity(0);
        entity.position = world_from_raw([32_760, 0, 0]);

        assert_eq!(
            forward_projection_raw(&entity, [-32_760, 0, 0]),
            RetailRuntimeValue::Known(15),
            "-32760 - 32760 wraps to a +16 signed-word displacement"
        );

        entity.position = world_from_raw([-32_760, 0, 0]);
        assert_eq!(
            forward_projection_raw(&entity, [32_760, 0, 0]),
            RetailRuntimeValue::Known(-16),
            "32760 - -32760 wraps to a -16 signed-word displacement"
        );
    }

    #[test]
    fn binary_heading_boundaries_select_the_expected_closed_half_space() {
        for (heading, ahead, behind) in [
            (0x0000, [0x0100, 0, 0], [-0x0100, 0, 0]),
            (0x4000, [0, 0, 0x0100], [0, 0, -0x0100]),
            (0x8000, [-0x0100, 0, 0], [0x0100, 0, 0]),
            (0xC000, [0, 0, -0x0100], [0, 0, 0x0100]),
        ] {
            let entity = type9_entity(heading);
            assert_eq!(
                entity_forward_half_space(&entity, ahead),
                RetailRuntimeValue::Known(true),
                "heading {heading:04x} must accept its forward cardinal"
            );
            assert_eq!(
                entity_forward_half_space(&entity, behind),
                RetailRuntimeValue::Known(false),
                "heading {heading:04x} must reject its rear cardinal"
            );
            assert_eq!(
                entity_forward_half_space(&entity, [0, 0, 0]),
                RetailRuntimeValue::Known(true),
                "FUN_0041E930 uses >= 0, so the boundary plane is accepted"
            );
        }
    }

    #[test]
    fn unavailable_live_basis_keeps_the_directional_gate_unresolved() {
        let mut entity = type9_entity(0);
        entity.collision.pair_callbacks.orientation_policy = RetailRuntimeValue::Unresolved;
        assert_eq!(
            entity_forward_half_space(&entity, [0x100, 0, 0]),
            RetailRuntimeValue::Unresolved
        );
    }

    #[test]
    fn accepted_capture_commits_heading_and_sub_a_without_touching_animation() {
        let mut entity = type9_entity(0xD7B1);
        let animation_before = match entity.actor_animation_runtime {
            RetailRuntimeValue::Known(Some(controller)) => controller,
            _ => unreachable!(),
        };
        let sub_a_before = match entity.sub_a_propulsion_runtime {
            RetailRuntimeValue::Known(Some(runtime)) => runtime,
            _ => unreachable!(),
        };
        assert_eq!(sub_a_before.direction_multiplier(), -1);

        let plan = plan_type9_main_base_component(&entity, RetailRuntimeValue::Known(true))
            .unwrap()
            .unwrap();
        assert_eq!(plan.expected_heading_raw(), 0xD7B1);
        assert_eq!(plan.resolved_heading_raw(), 0xF7B1);

        let outcome = commit_type9_main_base_component(&mut entity, plan).unwrap();
        assert_eq!(
            outcome,
            Type9MainBaseComponentOutcome {
                entity_id: 0x04AC_0001,
                heading_raw_before: 0xD7B1,
                heading_raw_after: 0xF7B1,
                direction_multiplier_before: -1,
                direction_multiplier_after: TYPE9_MAIN_BASE_DIRECTION_MULTIPLIER,
            }
        );
        let sub_a_after = match entity.sub_a_propulsion_runtime {
            RetailRuntimeValue::Known(Some(runtime)) => runtime,
            _ => unreachable!(),
        };
        assert_eq!(entity.heading_raw(), 0xF7B1);
        assert_eq!(sub_a_after.direction_multiplier(), 1);
        assert_eq!(
            sub_a_after.target_speed_raw(),
            sub_a_before.target_speed_raw()
        );
        assert_eq!(
            sub_a_after.drive_scale_percent(),
            sub_a_before.drive_scale_percent()
        );
        assert_eq!(
            entity.actor_animation_runtime,
            RetailRuntimeValue::Known(Some(animation_before)),
            "Sub-I animation is common component-table slot 4, not slot 3"
        );
    }

    #[test]
    fn heading_addition_uses_retail_word_wrapping() {
        let mut entity = type9_entity(0xF7B1);
        let plan = plan_type9_main_base_component(&entity, RetailRuntimeValue::Known(true))
            .unwrap()
            .unwrap();
        assert_eq!(plan.resolved_heading_raw(), 0x17B1);
        commit_type9_main_base_component(&mut entity, plan).unwrap();
        assert_eq!(entity.heading_raw(), 0x17B1);
    }

    #[test]
    fn rejected_half_space_is_a_noop_and_unknown_state_fails_closed() {
        let mut entity = type9_entity(0xD7B1);
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(None);
        assert_eq!(
            plan_type9_main_base_component(&entity, RetailRuntimeValue::Known(false)),
            Ok(None),
            "a rejected directional gate never reaches the descriptor"
        );
        assert_eq!(
            plan_type9_main_base_component(&entity, RetailRuntimeValue::Known(true)),
            Err(Type9MainBaseComponentPlanError::SubAPropulsionRuntimeAbsent)
        );
        assert_eq!(
            plan_type9_main_base_component(&entity, RetailRuntimeValue::Unresolved),
            Err(Type9MainBaseComponentPlanError::ForwardHalfSpaceUnresolved)
        );

        entity.entity_type = 17;
        assert_eq!(
            plan_type9_main_base_component(&entity, RetailRuntimeValue::Known(false)),
            Err(Type9MainBaseComponentPlanError::WrongEntityType { entity_type: 17 })
        );
    }

    #[test]
    fn stale_plan_rejects_before_either_write() {
        let mut entity = type9_entity(0xD7B1);
        let plan = plan_type9_main_base_component(&entity, RetailRuntimeValue::Known(true))
            .unwrap()
            .unwrap();
        let stale_heading = 0xD7B2;
        entity.set_heading_raw(stale_heading);
        let sub_a_before_commit = entity.sub_a_propulsion_runtime;

        assert_eq!(
            commit_type9_main_base_component(&mut entity, plan),
            Err(Type9MainBaseComponentCommitError::HeadingChanged {
                expected: 0xD7B1,
                actual: stale_heading,
            })
        );
        assert_eq!(entity.heading_raw(), stale_heading);
        assert_eq!(entity.sub_a_propulsion_runtime, sub_a_before_commit);
    }

    #[test]
    fn stale_sub_a_runtime_rejects_before_heading_write() {
        let mut entity = type9_entity(0xD7B1);
        let plan = plan_type9_main_base_component(&entity, RetailRuntimeValue::Known(true))
            .unwrap()
            .unwrap();
        if let RetailRuntimeValue::Known(Some(runtime)) = &mut entity.sub_a_propulsion_runtime {
            runtime.set_direction_multiplier(7);
        }
        let sub_a_before_commit = entity.sub_a_propulsion_runtime;

        assert_eq!(
            commit_type9_main_base_component(&mut entity, plan),
            Err(Type9MainBaseComponentCommitError::SubAPropulsionRuntimeChanged)
        );
        assert_eq!(entity.heading_raw(), 0xD7B1);
        assert_eq!(entity.sub_a_propulsion_runtime, sub_a_before_commit);
    }

    #[test]
    fn missing_sub_i_animation_does_not_block_the_sub_a_write() {
        let mut entity = type9_entity(0xD7B1);
        entity.actor_animation_runtime = RetailRuntimeValue::Known(None);
        let plan = plan_type9_main_base_component(&entity, RetailRuntimeValue::Known(true))
            .unwrap()
            .unwrap();
        commit_type9_main_base_component(&mut entity, plan).unwrap();
        assert_eq!(
            entity.actor_animation_runtime,
            RetailRuntimeValue::Known(None)
        );
        let RetailRuntimeValue::Known(Some(runtime)) = entity.sub_a_propulsion_runtime else {
            panic!("Sub-A runtime must remain installed");
        };
        assert_eq!(runtime.direction_multiplier(), 1);
    }
}
