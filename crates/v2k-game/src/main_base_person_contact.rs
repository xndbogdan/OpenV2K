//! Native people's component suffix after Main Base `4258A0` returns zero.
//!
//! The candidate's actual A900 slots select `02DA0` or a proven null callback.
//! `02DA0 -> 1E930 -> 01A20` reads the retained matrix and each task's private
//! direction. A/B/D/I people take the zero-RNG Sub-I branch: heading += 2000,
//! then `019C0 -> 204B0` propagates that direction to Sub-A. Neither the task
//! private state nor Sub-I, Sub-D, or the incoming matrix advances here.
//!
//! The pair host must obtain completed scheduler custody before planning. This
//! journal authenticates the native allocation and all current slots, then
//! revalidates them before the independent physical suffix. The preceding
//! `410B70` flags write is deliberately outside its immutable inputs.

use crate::native_type86::NativePersonProfile;
use crate::{
    actor_task_dispatcher::{ActorTaskRuntime, ActorTaskRuntimeFamily},
    actor_task_owner::{ActorTaskId, ActorTaskSlot, ActorTaskWrapperFlags},
    common_mover::{
        actor_abdi::{ActorAbdiTopology, ActorAbdiTopologyError},
        target_prelude::{
            CommonMoverPreludeSubA, CommonMoverPreludeSubD, CommonMoverTargetPreludeTopology,
        },
        type9_attitude::Type9BodyBasis,
        SubAPropulsionRuntime,
    },
    descriptor_contact::{
        plan_descriptor_contact, DescriptorContactBlock, DescriptorContactOutcome,
        DescriptorContactRequest, DescriptorContactSourceSnapshot, DescriptorContactTargetSnapshot,
    },
    entity::{Entity, EntityManager},
    entity_behavior::BehaviorContextRuntime,
    entity_collision_state::RetailRuntimeValue,
    main_base_abort::MainBaseAbortActorLease,
    native_actor_descriptor_contact::{
        private_state, task_has_descriptor_contact, task_has_null_contact,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativePersonReceipt {
    Type9(crate::ordinary_type9_construction::OrdinaryType9NativeReceipt),
    Type86(crate::native_type86::NativeType86Runtime),
    Type123(crate::native_type123::NativeType123Runtime),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ComponentSlot {
    id: ActorTaskId,
    state: ActorTaskRuntime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativePersonMainBaseComponentOutcome {
    Null,
    Behind,
    Applied {
        heading_raw_before: u16,
        heading_raw_after: u16,
        direction_multiplier: i32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativePersonMainBaseComponentError {
    Runtime(&'static str),
    UnsupportedTask {
        slot: ActorTaskSlot,
        family: ActorTaskRuntimeFamily,
    },
    Topology(ActorAbdiTopologyError),
    Descriptor(DescriptorContactBlock),
}

/// Complete immutable candidate component-chain journal. Slot results retain
/// Primary/Secondary/Tertiary order, including absent and explicit null slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativePersonMainBaseComponentPlan {
    source_id: u32,
    receipt: NativePersonReceipt,
    allocation: MainBaseAbortActorLease,
    position_raw: [i16; 3],
    rotation_raw: [i16; 3],
    basis: RetailRuntimeValue<Type9BodyBasis>,
    context: RetailRuntimeValue<Option<BehaviorContextRuntime>>,
    slots: [Option<ComponentSlot>; 3],
    sub_a: RetailRuntimeValue<Option<SubAPropulsionRuntime>>,
    target: DescriptorContactTargetSnapshot,
    topology: Option<ActorAbdiTopology>,
    resolved_heading: u16,
    resolved_sub_a: RetailRuntimeValue<Option<SubAPropulsionRuntime>>,
    outcomes: [NativePersonMainBaseComponentOutcome; 3],
}

impl NativePersonMainBaseComponentPlan {
    pub const fn entity_id(&self) -> u32 {
        self.source_id
    }

    pub const fn outcomes(&self) -> &[NativePersonMainBaseComponentOutcome; 3] {
        &self.outcomes
    }
}

/// Receipt authentication is independent of Main Base capability admission and
/// completed scheduler custody. Captured Type9 adapters have a separate owner.
pub fn native_person_allocation_authenticates(manager: &EntityManager, id: u32) -> bool {
    match manager
        .iter_all()
        .find(|entity| entity.id == id)
        .map(|entity| entity.entity_type)
    {
        Some(9) => {
            crate::ordinary_type9_construction::ordinary_type9_native_allocation_authenticates(
                manager, id,
            )
        }
        Some(type_id) if NativePersonProfile::from_entity_type(type_id).is_some() => {
            crate::native_type86::native_type86_manager_allocation_authenticates(manager, id)
        }
        Some(123) => {
            crate::native_type123::native_type123_manager_allocation_authenticates(manager, id)
        }
        _ => false,
    }
}

fn receipt(entity: &Entity) -> Option<NativePersonReceipt> {
    match entity.entity_type {
        9 => entity
            .ordinary_type9_native_receipt
            .map(NativePersonReceipt::Type9),
        type_id if NativePersonProfile::from_entity_type(type_id).is_some() => entity
            .native_type86_runtime
            .map(NativePersonReceipt::Type86),
        123 => entity
            .native_type123_runtime
            .map(NativePersonReceipt::Type123),
        _ => None,
    }
}

fn slots(
    entity: &Entity,
) -> Result<[Option<ComponentSlot>; 3], NativePersonMainBaseComponentError> {
    let mut result = [None; 3];
    for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
        if let Some(id) = entity.actor_tasks.task_in_slot(slot) {
            if entity.actor_tasks.wrapper_flags(id)
                != Some(ActorTaskWrapperFlags {
                    alive: true,
                    in_callback: false,
                })
            {
                return Err(NativePersonMainBaseComponentError::Runtime(
                    "completed component wrapper",
                ));
            }
            result[slot as usize] = Some(ComponentSlot {
                id,
                state: *entity.actor_tasks.task_state(id).ok_or(
                    NativePersonMainBaseComponentError::Runtime("component task"),
                )?,
            });
        }
    }
    Ok(result)
}

pub fn plan_native_person_main_base_components(
    manager: &EntityManager,
    source_id: u32,
    target_id: u32,
) -> Result<NativePersonMainBaseComponentPlan, NativePersonMainBaseComponentError> {
    use NativePersonMainBaseComponentError as Error;
    if !native_person_allocation_authenticates(manager, source_id) {
        return Err(Error::Runtime("native person allocation"));
    }
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == source_id)
        .unwrap();
    let target_entity = manager
        .iter_all()
        .find(|entity| entity.id == target_id)
        .ok_or(Error::Runtime("contacted allocation"))?;
    let target = DescriptorContactTargetSnapshot {
        entity_id: target_id,
        position_raw: target_entity.position_raw(),
    };
    let mut plan = NativePersonMainBaseComponentPlan {
        source_id,
        receipt: receipt(entity).unwrap(),
        allocation: manager
            .main_base_abort_actor_observation(source_id)
            .unwrap()
            .lease,
        position_raw: entity.position_raw(),
        rotation_raw: entity.rotation_heading_pitch_roll_raw(),
        basis: entity.physical_body_basis_q31(),
        context: entity.current_behavior_context,
        slots: slots(entity)?,
        sub_a: entity.sub_a_propulsion_runtime,
        target,
        topology: None,
        resolved_heading: entity.heading_raw(),
        resolved_sub_a: entity.sub_a_propulsion_runtime,
        outcomes: [NativePersonMainBaseComponentOutcome::Null; 3],
    };
    for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
        let Some(component) = plan.slots[slot as usize] else {
            continue;
        };
        if task_has_null_contact(&component.state) {
            continue;
        }
        if !task_has_descriptor_contact(&component.state) {
            return Err(Error::UnsupportedTask {
                slot,
                family: component.state.family(),
            });
        }
        let RetailRuntimeValue::Known(basis) = plan.basis else {
            return Err(Error::Runtime("retained physical basis"));
        };
        let source = DescriptorContactSourceSnapshot {
            entity_id: source_id,
            position_raw: plan.position_raw,
            forward_q31: basis.forward,
            heading_raw: plan.resolved_heading,
            roll_raw: plan.rotation_raw[2] as u16,
        };
        let mut request = DescriptorContactRequest {
            source: RetailRuntimeValue::Known(source),
            target: RetailRuntimeValue::Known(target),
            private_state: RetailRuntimeValue::Unresolved,
            topology: RetailRuntimeValue::Unresolved,
        };
        match plan_descriptor_contact(request, || unreachable!("half-space has no RNG")) {
            Ok(DescriptorContactOutcome::Miss(_)) => {
                plan.outcomes[slot as usize] = NativePersonMainBaseComponentOutcome::Behind;
                continue;
            }
            Err(DescriptorContactBlock::UnresolvedPrivateState) => {}
            _ => unreachable!("known source and target"),
        }
        let metadata = manager
            .type_runtime_metadata(entity.entity_type)
            .ok_or(Error::Runtime("person metadata"))?;
        let topology = ActorAbdiTopology::from_metadata(entity.entity_type as u16, metadata)
            .map_err(Error::Topology)?;
        let RetailRuntimeValue::Known(Some(sub_a)) = plan.resolved_sub_a else {
            return Err(Error::Runtime("Sub-A runtime"));
        };
        let RetailRuntimeValue::Known(Some(descriptor)) = metadata.sub_a_propulsion_descriptor
        else {
            return Err(Error::Runtime("Sub-A descriptor"));
        };
        request.private_state = RetailRuntimeValue::Known(
            private_state(&component.state).expect("descriptor task owns private state"),
        );
        request.topology = RetailRuntimeValue::Known(CommonMoverTargetPreludeTopology {
            sub_a: Some(CommonMoverPreludeSubA {
                descriptor: Some(descriptor),
                runtime: sub_a,
            }),
            sub_d: Some(CommonMoverPreludeSubD {
                steering_divisor_raw: topology.sub_d_descriptor().steering_divisor_raw,
                couple_yaw_into_roll: topology.sub_d_descriptor().couple_yaw_into_roll_raw != 0,
            }),
            sub_i: topology.component_topology().sub_i,
            sub_f: topology.component_topology().sub_f,
            sub_g: topology.component_topology().sub_g,
            sub_l: topology.component_topology().sub_l,
        });
        let DescriptorContactOutcome::Apply(effect) = plan_descriptor_contact(request, || {
            unreachable!("authenticated Sub-I branch has no RNG")
        })
        .map_err(Error::Descriptor)?
        else {
            unreachable!("same half-space");
        };
        plan.topology = Some(topology);
        plan.resolved_heading = effect.effect.heading_raw;
        plan.resolved_sub_a = RetailRuntimeValue::Known(effect.effect.sub_a_runtime);
        plan.outcomes[slot as usize] = NativePersonMainBaseComponentOutcome::Applied {
            heading_raw_before: source.heading_raw,
            heading_raw_after: plan.resolved_heading,
            direction_multiplier: effect.expected_private_state.direction,
        };
    }
    Ok(plan)
}

/// Manager-side validation precedes the caller's disjoint mutable body borrows.
pub fn validate_native_person_main_base_components(
    manager: &EntityManager,
    plan: &NativePersonMainBaseComponentPlan,
) -> Result<(), NativePersonMainBaseComponentError> {
    use NativePersonMainBaseComponentError as Error;
    if !native_person_allocation_authenticates(manager, plan.source_id)
        || manager
            .main_base_abort_actor_observation(plan.source_id)
            .is_none_or(|observation| observation.lease != plan.allocation)
    {
        return Err(Error::Runtime("native person allocation changed"));
    }
    if manager
        .iter_all()
        .find(|entity| entity.id == plan.target.entity_id)
        .is_none_or(|entity| entity.position_raw() != plan.target.position_raw)
    {
        return Err(Error::Runtime("contacted position changed"));
    }
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == plan.source_id)
        .unwrap();
    if let Some(expected) = plan.topology {
        let metadata = manager
            .type_runtime_metadata(entity.entity_type)
            .ok_or(Error::Runtime("person metadata"))?;
        if ActorAbdiTopology::from_metadata(entity.entity_type as u16, metadata)
            .map_err(Error::Topology)?
            != expected
        {
            return Err(Error::Runtime("person descriptor changed"));
        }
    }
    validate_entity(entity, plan)
}

fn validate_entity(
    entity: &Entity,
    plan: &NativePersonMainBaseComponentPlan,
) -> Result<(), NativePersonMainBaseComponentError> {
    if entity.id != plan.source_id
        || receipt(entity) != Some(plan.receipt)
        || entity.position_raw() != plan.position_raw
        || entity.rotation_heading_pitch_roll_raw() != plan.rotation_raw
        || entity.physical_body_basis_q31() != plan.basis
        || entity.current_behavior_context != plan.context
        || entity.sub_a_propulsion_runtime != plan.sub_a
        || slots(entity)? != plan.slots
    {
        return Err(NativePersonMainBaseComponentError::Runtime(
            "person component journal changed",
        ));
    }
    Ok(())
}

/// After manager-side validation, commit only the two Sub-I descriptor writes.
/// No task, animation, matrix or collision flags are replaced by this phase.
pub fn commit_native_person_main_base_components(
    entity: &mut Entity,
    plan: &NativePersonMainBaseComponentPlan,
) -> Result<[NativePersonMainBaseComponentOutcome; 3], NativePersonMainBaseComponentError> {
    validate_entity(entity, plan)?;
    if plan.outcomes.iter().any(|outcome| {
        matches!(
            outcome,
            NativePersonMainBaseComponentOutcome::Applied { .. }
        )
    }) {
        entity.set_heading_raw(plan.resolved_heading);
        entity.sub_a_propulsion_runtime = plan.resolved_sub_a;
    }
    Ok(plan.outcomes)
}

#[cfg(test)]
mod tests;
