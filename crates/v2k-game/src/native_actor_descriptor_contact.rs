//! Live 02DA0/01A20 component contact for retained native actor owners.
//!
//! Task constructors, not the current behavior's label or a whole-chain flag,
//! identify the callback. The signed forward-half-space test precedes every
//! descriptor/private-state read. A completed owner lends mutation custody;
//! the callback changes neither task lifetime nor the incoming physical basis.

use crate::{
    actor_task_dispatcher::{ActorTaskRuntime, ActorTaskRuntimeFamily},
    actor_task_owner::{ActorTaskId, ActorTaskSlot, ActorTaskWrapperFlags},
    common_mover::target_prelude::{
        CommonMoverPreludeSubA, CommonMoverPreludeSubD, CommonMoverTargetPreludeTopology,
    },
    descriptor_contact::{
        plan_descriptor_contact, DescriptorContactBlock, DescriptorContactOutcome,
        DescriptorContactPlan, DescriptorContactRequest, DescriptorContactSourceSnapshot,
        DescriptorContactTargetSnapshot,
    },
    entity::{Entity, EntityManager},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    intro2_contacts::Intro2ContactFrame,
    intro2_radial::Intro2RadialTaskCustody,
    intro2_type8::NativeWorkerProfile,
    native_type86::NativeFourChoiceProfile,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    wander_near_location::WanderNearPrivateState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeDescriptorContactOutcome {
    Null,
    Behind,
    Applied { rng_draws: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeDescriptorContactError {
    Runtime(&'static str),
    UnsupportedTask {
        entity_id: u32,
        slot: ActorTaskSlot,
        family: ActorTaskRuntimeFamily,
    },
    Descriptor(DescriptorContactBlock),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeDescriptorContactBlock {
    pub reason: NativeDescriptorContactError,
    pub committed_prefix: bool,
}

impl From<NativeDescriptorContactError> for NativeDescriptorContactBlock {
    fn from(reason: NativeDescriptorContactError) -> Self {
        Self {
            reason,
            committed_prefix: false,
        }
    }
}

/// Execute one current A900 slot. The caller retains the pair's physical plane
/// and proceeds to the next freshly read slot only after this returns.
pub fn resolve_native_actor_descriptor_contact(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    opposite: u32,
    slot: ActorTaskSlot,
) -> Result<NativeDescriptorContactOutcome, NativeDescriptorContactBlock> {
    use NativeDescriptorContactError as Error;
    let SlotAdmission::Descriptor { task_id, private } =
        admit_slot(frame.entities, frame.actor_tasks, id, slot)?
    else {
        return Ok(NativeDescriptorContactOutcome::Null);
    };
    let entity = actor(frame.entities, id)?;
    let source = descriptor_source(entity)?;
    let target = DescriptorContactTargetSnapshot {
        entity_id: opposite,
        position_raw: actor(frame.entities, opposite)?.position_raw(),
    };
    if !ahead(source, target) {
        return Ok(NativeDescriptorContactOutcome::Behind);
    }
    let metadata = frame
        .entities
        .type_runtime_metadata(entity.entity_type)
        .ok_or(Error::Runtime("descriptor metadata"))?;
    let topology = contact_topology(entity, metadata)?;
    let plan = plan_ahead(source, target, private, topology, || {
        u32::from(frame.world_fx.next_shared_retail_random_u16())
    })?;
    commit_plan(frame.entities, frame.actor_tasks, id, task_id, &plan)?;
    Ok(NativeDescriptorContactOutcome::Applied {
        rng_draws: plan.effect.rng_draw_count,
    })
}

/// One current A900 slot, classified under the late walk's custody.
enum SlotAdmission {
    Null,
    Descriptor {
        task_id: ActorTaskId,
        private: WanderNearPrivateState,
    },
}

fn admit_slot(
    entities: &EntityManager,
    tasks: &mut SpecializedActorTaskScheduler,
    id: u32,
    slot: ActorTaskSlot,
) -> Result<SlotAdmission, NativeDescriptorContactError> {
    use NativeDescriptorContactError as Error;
    let entity = actor(entities, id)?;
    let Some(task_id) = entity.actor_tasks.task_in_slot(slot) else {
        return Ok(SlotAdmission::Null);
    };
    if entity.actor_tasks.wrapper_flags(task_id)
        != Some(ActorTaskWrapperFlags {
            alive: true,
            in_callback: false,
        })
    {
        return Err(Error::Runtime("completed component wrapper"));
    }
    let task = entity
        .actor_tasks
        .task_state(task_id)
        .ok_or(Error::Runtime("component task"))?;
    if entity.native_entity_weapon_runtime.is_some() {
        //404580,403840,4069E0 and401F80 all retain05FF0's null+18.
        //Only the constructor lease and completed current owner establish
        //this program; a coincidental task enum cannot authorize it.
        if !native_weapon_contact_owner_authenticates(entities, tasks, id)
            || !matches!(
                task,
                ActorTaskRuntime::BoulderRolling(_)
                    | ActorTaskRuntime::RocketFlight(_)
                    | ActorTaskRuntime::RocketTrail
                    | ActorTaskRuntime::GuardLocationAcquisition(_)
            )
        {
            return Err(Error::Runtime("completed native weapon contact owner"));
        }
        return Ok(SlotAdmission::Null);
    }
    if matches!(task, ActorTaskRuntime::Intro2GunTurret(_)) {
        //4086D0 writes the template+18 null explicitly at408721. A native
        //receipt and completed current Class29 graph authorize that null;
        //a task enum by itself does not confer body mutation custody.
        if !crate::intro2_gun_turret::intro2_gun_turret_manager_allocation_authenticates(
            entities, id,
        ) || !crate::intro2_gun_turret::task::graph_authenticates(entity)
            || !tasks.prepare_native_actor_mutation(entities, id)
        {
            return Err(Error::Runtime("completed native turret contact owner"));
        }
        return Ok(SlotAdmission::Null);
    }
    if entity.rolling_boulder_runtime.is_some()
        && matches!(
            task,
            ActorTaskRuntime::BoulderRolling(_) | ActorTaskRuntime::BoulderResting(_)
        )
    {
        //00404580 and404B40 both build their Primary through05FF0, which
        //zeroes the task+18. Only the boulder's completed owner lends custody.
        if !crate::rolling_boulder::rolling_boulder_manager_allocation_authenticates(entities, id)
            || !tasks.prepare_native_actor_mutation(entities, id)
        {
            return Err(Error::Runtime("completed native boulder contact owner"));
        }
        return Ok(SlotAdmission::Null);
    }
    if matches!(task, ActorTaskRuntime::TumbleOutOfSky(_)) {
        //04360/Class11 retains05FF0's null task+18.
        if !native_insect_allocation_authenticates(entities, id)
            || !completed_contact_owner(entities, tasks, id)
        {
            return Err(Error::Runtime("completed native Tumble contact owner"));
        }
        return Ok(SlotAdmission::Null);
    }
    if matches!(task, ActorTaskRuntime::WorkingFactory(_)) {
        //25BD0 retains01020's template+18 null. Only the native Factory
        //receipt and its completed owner authenticate that task layout.
        if !crate::intro2_type66::intro2_type66_allocation_authenticates(entity)
            || !tasks.prepare_native_actor_mutation(entities, id)
        {
            return Err(Error::Runtime("completed native Factory contact owner"));
        }
        return Ok(SlotAdmission::Null);
    }
    if task_has_null_contact(task) {
        return Ok(SlotAdmission::Null);
    }
    if !task_has_descriptor_contact(task) {
        return Err(Error::UnsupportedTask {
            entity_id: id,
            slot,
            family: task.family(),
        });
    }
    let private = private_state(task).expect("known descriptor task");
    if !allocation_authenticates(entities, id, entity.entity_type) {
        return Err(Error::Runtime("native descriptor allocation"));
    }
    if !completed_contact_owner(entities, tasks, id) {
        return Err(Error::Runtime("completed descriptor owner"));
    }
    Ok(SlotAdmission::Descriptor { task_id, private })
}

fn descriptor_source(
    entity: &Entity,
) -> Result<DescriptorContactSourceSnapshot, NativeDescriptorContactError> {
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        return Err(NativeDescriptorContactError::Runtime(
            "descriptor physical basis",
        ));
    };
    let [heading, _, roll] = entity.rotation_heading_pitch_roll_raw();
    Ok(DescriptorContactSourceSnapshot {
        entity_id: entity.id,
        position_raw: entity.position_raw(),
        forward_q31: basis.forward,
        heading_raw: heading as u16,
        roll_raw: roll as u16,
    })
}

/// 02DA0's signed forward half-space, which precedes every descriptor read.
fn ahead(source: DescriptorContactSourceSnapshot, target: DescriptorContactTargetSnapshot) -> bool {
    match plan_descriptor_contact(
        DescriptorContactRequest {
            source: RetailRuntimeValue::Known(source),
            target: RetailRuntimeValue::Known(target),
            private_state: RetailRuntimeValue::Unresolved,
            topology: RetailRuntimeValue::Unresolved,
        },
        || unreachable!("02DA0 half-space consumes no RNG"),
    ) {
        Ok(DescriptorContactOutcome::Miss(_)) => false,
        Err(DescriptorContactBlock::UnresolvedPrivateState) => true,
        _ => unreachable!("known source/target half-space gate"),
    }
}

fn plan_ahead(
    source: DescriptorContactSourceSnapshot,
    target: DescriptorContactTargetSnapshot,
    private: WanderNearPrivateState,
    topology: CommonMoverTargetPreludeTopology,
    next_random: impl FnMut() -> u32,
) -> Result<DescriptorContactPlan, NativeDescriptorContactError> {
    match plan_descriptor_contact(
        DescriptorContactRequest {
            source: RetailRuntimeValue::Known(source),
            target: RetailRuntimeValue::Known(target),
            private_state: RetailRuntimeValue::Known(private),
            topology: RetailRuntimeValue::Known(topology),
        },
        next_random,
    )
    .map_err(NativeDescriptorContactError::Descriptor)?
    {
        DescriptorContactOutcome::Apply(plan) => Ok(plan),
        DescriptorContactOutcome::Miss(_) => unreachable!("same accepted half-space"),
    }
}

fn commit_plan(
    entities: &mut EntityManager,
    tasks: &mut SpecializedActorTaskScheduler,
    id: u32,
    task_id: ActorTaskId,
    plan: &DescriptorContactPlan,
) -> Result<(), NativeDescriptorContactBlock> {
    let entity = entities.entity_mut(id).unwrap();
    let kind = entity.entity_type;
    commit_private(
        entity.actor_tasks.task_state_mut(task_id).unwrap(),
        plan.effect.target_state,
    );
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(plan.effect.sub_a_runtime);
    if let Some(write) = plan.effect.sub_d_reversal_write {
        let runtime = match kind {
            16 | 128 => &mut entity.intro2_type16_runtime.as_mut().unwrap().sub_d_runtime,
            26 => entity.intro2_type26_sub_d_runtime.as_mut().unwrap(),
            30 => &mut entity.native_type30_runtime.as_mut().unwrap().sub_d_runtime,
            40 => &mut entity.native_type40_runtime.as_mut().unwrap().sub_d_runtime,
            56 => &mut entity.native_type56_runtime.as_mut().unwrap().sub_d_runtime,
            38 | 129 | 119 => &mut entity.native_type38_runtime.as_mut().unwrap().sub_d_runtime,
            12 | 127 => &mut entity.native_type12_runtime.as_mut().unwrap().sub_d_runtime,
            31 | 105 => &mut entity.native_type31_runtime.as_mut().unwrap().sub_d_runtime,
            13 => {
                &mut entity
                    .intro2_type13_common_mover_runtime
                    .as_mut()
                    .unwrap()
                    .sub_d_runtime
            }
            14 | 15 | 39 | 87 | 108 => {
                &mut entity
                    .intro2_flyer_frame_owner
                    .as_mut()
                    .unwrap()
                    .sub_d_runtime
            }
            10 | 5 | 80 | 126 | 55 | 81 => {
                &mut entity.intro2_type10_runtime.as_mut().unwrap().sub_d_runtime
            }
            57 => &mut entity.intro2_type57_runtime.as_mut().unwrap().sub_d_runtime,
            17 => entity.type17_sub_d_runtime.as_mut().unwrap(),
            47 | 71 => entity.intro2_type47_sub_d_runtime.as_mut().unwrap(),
            49 => {
                &mut entity
                    .cleansing_vehicle_runtime
                    .as_mut()
                    .unwrap()
                    .sub_d_runtime
            }
            53 => &mut entity.intro2_type53_runtime.as_mut().unwrap().sub_d_runtime,
            58 => &mut entity.intro2_type58_runtime.as_mut().unwrap().sub_d_runtime,
            122 => {
                &mut entity
                    .native_type122_runtime
                    .as_mut()
                    .unwrap()
                    .sub_d_runtime
            }
            18 => &mut entity.native_type18_runtime.as_mut().unwrap().sub_d_runtime,
            28 => &mut entity.native_type28_runtime.as_mut().unwrap().sub_d_runtime,
            76 | 77 => &mut entity.native_type76_runtime.as_mut().unwrap().sub_d_runtime,
            94 => &mut entity.intro2_type94_runtime.as_mut().unwrap().sub_d_runtime,
            22 | 23 | 24 | 62 | 124 => {
                &mut entity.shared_fish_runtime.as_mut().unwrap().sub_d_runtime
            }
            _ => unreachable!("Sub-I path has no Sub-D reversal write"),
        };
        runtime.last_yaw_step_raw = write.step_raw as i16;
    }
    if let Some(reversed) = plan.effect.sub_f_reverse_write {
        // 4019E3..4019F7 calls 424380 with direction == -1. No animation,
        // acceleration, target speed or Sub-F phase is advanced by contact.
        entity
            .shared_fish_runtime
            .as_mut()
            .expect("authenticated native Sub-F contact owner")
            .sub_f
            .set_reversal_raw(u8::from(reversed));
    }
    if let Some(reversed) = plan.effect.sub_g_reverse_write {
        let RetailRuntimeValue::Known(Some(g)) = &mut entity.sub_g_06070_runtime else {
            unreachable!("authenticated retained native G contact owner");
        };
        g.apply_direction_reverse_write(reversed);
    }
    entity.set_rotation_heading_pitch_roll_raw([
        plan.effect.heading_raw as i16,
        entity.rotation_heading_pitch_roll_raw()[1],
        plan.effect.roll_raw as i16,
    ]);
    // 401A20's Sub-I branch changes heading and propagates direction through
    // 019C0. It does not call 208C0, advance animation or rebuild the basis.
    if kind == 47 && !tasks.finish_native_type47_external_mutation(entities, id) {
        tasks.park_native_type47_external_prefix(id);
        return Err(NativeDescriptorContactBlock {
            reason: NativeDescriptorContactError::Runtime("completed Type47 descriptor suffix"),
            committed_prefix: true,
        });
    }
    Ok(())
}

/// A native actor's current A900 slots, read before the player's pair pass
/// under the late walk's custody. The pass plans each slot's 02DA0 at its
/// component callback, where the shared RNG is drawn, and commits after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeDescriptorContactSource {
    entity_id: u32,
    slots: [Option<(ActorTaskId, WanderNearPrivateState)>; 3],
    source: Result<DescriptorContactSourceSnapshot, NativeDescriptorContactError>,
    topology: Result<CommonMoverTargetPreludeTopology, NativeDescriptorContactError>,
}

/// One slot's staged 02DA0 effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeDescriptorContactSlotPlan {
    pub(crate) entity_id: u32,
    task_id: ActorTaskId,
    plan: DescriptorContactPlan,
}

impl NativeDescriptorContactSource {
    pub(crate) fn capture(
        entities: &EntityManager,
        tasks: &mut SpecializedActorTaskScheduler,
        id: u32,
    ) -> Result<Self, NativeDescriptorContactError> {
        let mut slots = [None; 3];
        for (index, slot) in ActorTaskSlot::IN_RETAIL_TICK_ORDER.into_iter().enumerate() {
            if let SlotAdmission::Descriptor { task_id, private } =
                admit_slot(entities, tasks, id, slot)?
            {
                slots[index] = Some((task_id, private));
            }
        }
        let entity = actor(entities, id)?;
        let topology = entities
            .type_runtime_metadata(entity.entity_type)
            .ok_or(NativeDescriptorContactError::Runtime("descriptor metadata"))
            .and_then(|metadata| contact_topology(entity, metadata));
        Ok(Self {
            entity_id: id,
            slots,
            source: descriptor_source(entity),
            topology,
        })
    }

    /// Run every slot's 02DA0 against `target` in A900 order. A slot reads
    /// the heading, roll and Sub-A runtime the previous slot's 01A20 wrote.
    pub(crate) fn plan(
        &self,
        target: DescriptorContactTargetSnapshot,
        next_random: &mut impl FnMut() -> u32,
    ) -> Result<Vec<NativeDescriptorContactSlotPlan>, NativeDescriptorContactError> {
        let mut plans = Vec::new();
        let mut chained: Option<(
            DescriptorContactSourceSnapshot,
            CommonMoverTargetPreludeTopology,
        )> = None;
        for (task_id, private) in self.slots.into_iter().flatten() {
            let source = match chained {
                Some((source, _)) => source,
                None => self.source?,
            };
            if !ahead(source, target) {
                continue;
            }
            let topology = match chained {
                Some((_, topology)) => topology,
                None => self.topology?,
            };
            let plan = plan_ahead(source, target, private, topology, &mut *next_random)?;
            let mut next_topology = topology;
            if let (Some(sub_a), Some(runtime)) =
                (next_topology.sub_a.as_mut(), plan.effect.sub_a_runtime)
            {
                sub_a.runtime = runtime;
            }
            chained = Some((
                DescriptorContactSourceSnapshot {
                    heading_raw: plan.effect.heading_raw,
                    roll_raw: plan.effect.roll_raw,
                    ..source
                },
                next_topology,
            ));
            plans.push(NativeDescriptorContactSlotPlan {
                entity_id: self.entity_id,
                task_id,
                plan,
            });
        }
        Ok(plans)
    }
}

/// Recheck a staged plan's owner and task immediately before the pass commits.
pub(crate) fn slot_plan_still_holds(
    entities: &EntityManager,
    tasks: &mut SpecializedActorTaskScheduler,
    plan: &NativeDescriptorContactSlotPlan,
) -> bool {
    let Ok(entity) = actor(entities, plan.entity_id) else {
        return false;
    };
    entity.actor_tasks.wrapper_flags(plan.task_id)
        == Some(ActorTaskWrapperFlags {
            alive: true,
            in_callback: false,
        })
        && entity
            .actor_tasks
            .task_state(plan.task_id)
            .and_then(private_state)
            == Some(plan.plan.expected_private_state)
        && allocation_authenticates(entities, plan.entity_id, entity.entity_type)
        && completed_contact_owner(entities, tasks, plan.entity_id)
}

pub(crate) fn commit_slot_plan(
    entities: &mut EntityManager,
    tasks: &mut SpecializedActorTaskScheduler,
    plan: &NativeDescriptorContactSlotPlan,
) -> Result<(), NativeDescriptorContactBlock> {
    commit_plan(entities, tasks, plan.entity_id, plan.task_id, &plan.plan)
}

/// Resolve only the component shapes whose retained allocations this adapter
/// owns. A/D/I people, A/D insects, B/D/G flyers and B/D/F fish retain
/// their distinct01A20 branches; unrelated components are not consumed here.
fn contact_topology(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<CommonMoverTargetPreludeTopology, NativeDescriptorContactError> {
    use NativeDescriptorContactError as Error;
    let RetailRuntimeValue::Known(topology) = metadata.common_mover_topology else {
        return Err(Error::Runtime("descriptor topology"));
    };
    // Type43 carries Sub-E alone: 01A20 skips its A and D branches, then
    // still installs the 1500-ms timer, negates direction and retargets.
    if entity.entity_type == crate::native_type43::ENTITY_TYPE {
        if topology != crate::native_type43::TOPOLOGY
            || entity.sub_a_propulsion_runtime != RetailRuntimeValue::Known(None)
            || metadata.sub_a_propulsion_descriptor != RetailRuntimeValue::Known(None)
            || metadata.sub_d_steering_descriptor != RetailRuntimeValue::Known(None)
        {
            return Err(Error::Runtime("native descriptor component shape"));
        }
        return Ok(CommonMoverTargetPreludeTopology {
            sub_a: None,
            sub_d: None,
            sub_i: false,
            sub_f: false,
            sub_g: false,
            sub_l: false,
        });
    }
    let fish = crate::shared_fish::is_shared_fish_type(entity.entity_type);
    let flying = matches!(
        entity.entity_type,
        13 | 14 | 15 | 39 | 87 | 108 | 10 | 5 | 80 | 126 | 55 | 81 | 57
    );
    let person = matches!(entity.entity_type, 9 | 123)
        || NativeWorkerProfile::from_entity_type(entity.entity_type).is_some()
        || NativeFourChoiceProfile::from_entity_type(entity.entity_type).is_some();
    let animation_presence_matches = matches!(metadata.actor_animation_descriptor,
        RetailRuntimeValue::Known(Some(_)) if topology.sub_i)
        || matches!(metadata.actor_animation_descriptor,
            RetailRuntimeValue::Known(None) if !topology.sub_i);
    if !topology.sub_d
        || topology.sub_g != flying
        // Type30 retains its constructor-owned K/L alongside A/B/C/D/E/H.
        //01A20 uses its A/D branch without advancing or reinterpreting K/L.
        || (topology.sub_l && !flying && !matches!(entity.entity_type, 30 | 38 | 129 | 119 | 12 | 127))
        || topology.sub_i != person
        || !animation_presence_matches
        || if fish {
            topology != crate::shared_fish::TOPOLOGY
        } else if flying {
            topology.sub_a || topology.sub_f
        } else {
            !topology.sub_a || topology.sub_f
        }
    {
        return Err(Error::Runtime("native descriptor component shape"));
    }
    let sub_a = if fish || flying {
        if entity.sub_a_propulsion_runtime != RetailRuntimeValue::Known(None)
            || metadata.sub_a_propulsion_descriptor != RetailRuntimeValue::Known(None)
        {
            return Err(Error::Runtime("fish descriptor Sub-A absence"));
        }
        None
    } else {
        let RetailRuntimeValue::Known(Some(runtime)) = entity.sub_a_propulsion_runtime else {
            return Err(Error::Runtime("descriptor Sub-A runtime"));
        };
        let RetailRuntimeValue::Known(Some(descriptor)) = metadata.sub_a_propulsion_descriptor
        else {
            return Err(Error::Runtime("descriptor Sub-A metadata"));
        };
        Some(CommonMoverPreludeSubA {
            descriptor: Some(descriptor),
            runtime,
        })
    };
    let RetailRuntimeValue::Known(Some(d)) = metadata.sub_d_steering_descriptor else {
        return Err(Error::Runtime("descriptor Sub-D metadata"));
    };
    if !topology.sub_i
        && match entity.entity_type {
            16 | 128 => entity.intro2_type16_runtime.is_none(),
            26 => entity.intro2_type26_sub_d_runtime.is_none(),
            30 => entity.native_type30_runtime.is_none(),
            40 => entity.native_type40_runtime.is_none(),
            56 => entity.native_type56_runtime.is_none(),
            38 | 129 | 119 => entity.native_type38_runtime.is_none(),
            12 | 127 => entity.native_type12_runtime.is_none(),
            31 | 105 => entity.native_type31_runtime.is_none(),
            13 => entity.intro2_type13_common_mover_runtime.is_none(),
            14 | 15 | 39 | 87 | 108 => entity.intro2_flyer_frame_owner.is_none(),
            10 | 5 | 80 | 126 | 55 | 81 => entity.intro2_type10_runtime.is_none(),
            57 => entity.intro2_type57_runtime.is_none(),
            17 => entity.type17_sub_d_runtime.is_none(),
            47 | 71 => entity.intro2_type47_sub_d_runtime.is_none(),
            49 => entity.cleansing_vehicle_runtime.is_none(),
            53 => entity.intro2_type53_runtime.is_none(),
            58 => entity.intro2_type58_runtime.is_none(),
            122 => entity.native_type122_runtime.is_none(),
            18 => entity.native_type18_runtime.is_none(),
            28 => entity.native_type28_runtime.is_none(),
            76 | 77 => entity.native_type76_runtime.is_none(),
            94 => entity.intro2_type94_runtime.is_none(),
            22 | 23 | 24 | 62 | 124 => entity.shared_fish_runtime.is_none(),
            _ => true,
        }
    {
        return Err(Error::Runtime("descriptor Sub-D runtime"));
    }
    Ok(CommonMoverTargetPreludeTopology {
        sub_a,
        sub_d: Some(CommonMoverPreludeSubD {
            steering_divisor_raw: d.steering_divisor_raw,
            couple_yaw_into_roll: d.couple_yaw_into_roll_raw != 0,
        }),
        sub_i: topology.sub_i,
        sub_f: topology.sub_f,
        sub_g: flying,
        //02DA0 ->01A20 writes A/F/G direction only, not the Sub-L target.
        sub_l: false,
    })
}

fn actor(manager: &EntityManager, id: u32) -> Result<&Entity, NativeDescriptorContactError> {
    manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(NativeDescriptorContactError::Runtime(
            "component allocation",
        ))
}

/// Independently constructed insect bodies admitted to the shared late
/// contact walk. Current task style never substitutes for allocation proof.
pub(crate) fn native_insect_allocation_authenticates(manager: &EntityManager, id: u32) -> bool {
    let Some(entity) = manager.iter_all().find(|entity| entity.id == id) else {
        return false;
    };
    match entity.entity_type {
        16 | 128 => crate::intro2_type16::intro2_type16_allocation_authenticates(entity),
        26 => crate::intro2_type26_defecate_virus::type26_manager_allocation_authenticates(
            manager, id,
        ),
        30 => crate::native_type30::manager_allocation_authenticates(manager, id),
        40 => crate::native_type40::manager_allocation_authenticates(manager, id),
        56 => crate::native_type56::manager_allocation_authenticates(manager, id),
        43 => crate::native_type43::manager_allocation_authenticates(manager, id),
        38 | 129 | 119 => crate::native_type38::manager_allocation_authenticates(manager, id),
        12 | 127 => crate::native_type12::manager_allocation_authenticates(manager, id),
        31 | 105 => crate::native_type31::manager_allocation_authenticates(manager, id),
        13 => crate::intro2_type13_live::type13_manager_allocation_authenticates(manager, id),
        14 | 15 | 39 | 87 | 108 => crate::intro2_flyers_live::flyer_identity_authenticates(entity),
        10 | 5 | 80 | 126 | 55 | 81 => {
            crate::intro2_type10::intro2_type10_allocation_authenticates(entity)
        }
        57 => crate::intro2_type57::intro2_type57_allocation_authenticates(entity),
        94 => crate::intro2_type94::intro2_type94_allocation_authenticates(entity),
        _ => false,
    }
}

pub(crate) fn completed_contact_owner(
    manager: &EntityManager,
    tasks: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler,
    id: u32,
) -> bool {
    let Some(entity) = manager.iter_all().find(|entity| entity.id == id) else {
        return false;
    };
    // 11AD0 has no dying test, and BC90 never calls A860: until 14990 a class63
    // carrier's retained tasks still take their contact hooks. Only its Finished
    // receipt stands in for the owner the terminal retired.
    if crate::class49_death::finished_terminal_hit_authenticates(manager, id)
        && crate::class49_death::source_profile(entity).is_some_and(|profile| {
            profile.policy() == crate::class49_death::NativeExplosionPolicy::Class63
        })
    {
        return true;
    }
    match entity.entity_type {
        42 | 59 if entity.native_entity_weapon_runtime.is_some() => {
            native_weapon_contact_owner_authenticates(manager, tasks, id)
        }
        13 => {
            crate::class49_death::finished_terminal_hit_authenticates(manager, id)
                || (!tasks.intro2_type13_has_pending_prefix(id)
                    && tasks.intro2_type13_completed_owner(manager, id))
        }
        // BAC0 cleared the class1 corpse's tasks; until 14990 only its
        // Finished receipt stands in for the retired living owner.
        43 => {
            crate::class49_death::finished_terminal_hit_authenticates(manager, id)
                || (!tasks.type43_has_pending_prefix(id)
                    && tasks.type43_completed_owner(manager, id))
        }
        // BAC0 cleared a Type38 corpse's tasks; only its receipt remains.
        38 | 129 | 119 | 12 | 127 => {
            crate::class49_death::finished_terminal_hit_authenticates(manager, id)
                || tasks.prepare_native_actor_mutation(manager, id)
        }
        15 | 87 | 108 => tasks.intro2_flyer_completed_owner(manager, id),
        // A class1 flyer's corpse keeps only its Finished receipt until 14990.
        14 | 39 => {
            crate::class49_death::finished_terminal_hit_authenticates(manager, id)
                || tasks.intro2_flyer_completed_owner(manager, id)
        }
        10 | 5 | 80 | 126 | 55 | 81 => {
            !tasks.intro2_type10_has_pending_prefix(id)
                && (tasks.intro2_type10_completed_owner(manager, id)
                    || tasks.intro2_type10_tumble_completed_owner(manager, id))
        }
        57 => {
            !tasks.intro2_type57_has_pending_prefix(id)
                && (tasks.intro2_type57_completed_owner(manager, id)
                    || tasks.intro2_type57_tumble_completed_owner(manager, id))
        }
        _ => tasks.prepare_native_actor_mutation(manager, id),
    }
}

pub(crate) fn native_weapon_contact_owner_authenticates(
    manager: &EntityManager,
    tasks: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler,
    id: u32,
) -> bool {
    manager
        .iter_all()
        .find(|entity| entity.id == id)
        .is_some_and(crate::native_entity_weapons::entity_authenticates)
        && crate::class49_death::allocation_authenticates(manager, id)
        && (crate::class49_death::finished_terminal_hit_authenticates(manager, id)
            || tasks.native_weapon_owner(manager, id).is_some())
}

fn allocation_authenticates(manager: &EntityManager, id: u32, kind: u32) -> bool {
    match kind {
        26 | 30 | 38 | 129 | 119 | 12 | 127 | 31 | 105 | 40 | 56 | 43 | 13 | 14 | 15 | 39 | 87
        | 108 | 10 | 5 | 80 | 126 | 55 | 81 | 57 => {
            native_insect_allocation_authenticates(manager, id)
        }
        kind if NativeWorkerProfile::from_entity_type(kind).is_some() => {
            crate::intro2_type8::intro2_type8_manager_allocation_authenticates(manager, id)
        }
        kind if NativeFourChoiceProfile::from_entity_type(kind).is_some() => {
            crate::native_type86::native_type86_manager_allocation_authenticates(manager, id)
        }
        123 => crate::native_type123::native_type123_manager_allocation_authenticates(manager, id),
        9 => {
            let Some(entity) = manager.iter_all().find(|entity| entity.id == id) else {
                return false;
            };
            if entity.ordinary_type9_native_receipt.is_some() {
                crate::ordinary_type9_construction::ordinary_type9_native_allocation_authenticates(
                    manager, id,
                )
            } else {
                // Intro2 still publishes its explicit constructor receipt.
                // The following scheduler custody check proves the issuing
                // allocation and completed graph for this route as well.
                crate::intro2_type9::intro2_type9_allocation_authenticates(entity)
            }
        }
        16 | 128 => manager
            .iter_all()
            .find(|entity| entity.id == id)
            .is_some_and(crate::intro2_type16::intro2_type16_allocation_authenticates),
        17 => crate::intro2_type17::type17_manager_allocation_authenticates(manager, id),
        22 | 23 | 24 | 62 | 124 => crate::shared_fish::allocation_authenticates(manager, id),
        47 | 71 => crate::shared_type47::type47_manager_allocation_authenticates(manager, id),
        49 => crate::cleansing_vehicle::allocation_authenticates(manager, id),
        53 => crate::intro2_type53::type53_manager_allocation_authenticates(manager, id),
        58 => crate::intro2_type58::type58_manager_allocation_authenticates(manager, id),
        122 => crate::native_type122::type122_manager_allocation_authenticates(manager, id),
        18 => crate::native_type18::manager_allocation_authenticates(manager, id),
        28 => crate::native_type28::manager_allocation_authenticates(manager, id),
        76 | 77 => crate::native_type76::manager_allocation_authenticates(manager, id),
        94 => manager
            .iter_all()
            .find(|entity| entity.id == id)
            .is_some_and(crate::intro2_type94::intro2_type94_allocation_authenticates),
        _ => false,
    }
}

pub(crate) fn task_has_null_contact(task: &ActorTaskRuntime) -> bool {
    // These constructors retain 05FF0's null task+18: 03230; acquisition
    // 01F80/02050/02100/02190; cue 02A10; Aim 02220; and dying 04120.
    // TrashFurniture401CA0 retains405FF0's null+18 (406013): it writes
    // only template+00/+04/+10/+14 before406030, never the contact slot.
    // Infect/cleanse02820 and the class68 timer02800 both retain05F80's null +18.
    // Hive's independent
    // 25E10/25F60 templates also zero +18 explicitly
    // (425E59/425FA9); their live/dying radial tick is not a contact callback.
    matches!(
        task,
        ActorTaskRuntime::None
            | ActorTaskRuntime::AttractAttentionCandidate(_)
            | ActorTaskRuntime::AttractAttentionCue(_)
            | ActorTaskRuntime::TargetAcquisition(_)
            | ActorTaskRuntime::FollowBeaconAcquisition(_)
            | ActorTaskRuntime::CaptureBeaconAcquisition
            | ActorTaskRuntime::GuardLocationAcquisition(_)
            | ActorTaskRuntime::AimAndFire(_)
            | ActorTaskRuntime::CommonDying(_)
            | ActorTaskRuntime::Class0Timer(_)
            | ActorTaskRuntime::TerrainCleansing(_)
            | ActorTaskRuntime::TrashFurniture(_)
            | ActorTaskRuntime::DefecateVirusTerrain(_)
            | ActorTaskRuntime::HiveRadial(_)
    )
}

pub(crate) fn task_has_descriptor_contact(task: &ActorTaskRuntime) -> bool {
    // The wander/retarget, route, job, chase, following and flee constructors
    // explicitly install 02DA0, despite their different task lifetimes.
    matches!(
        task,
        ActorTaskRuntime::OrdinaryType9Wander(_)
            | ActorTaskRuntime::SharedRetarget(_)
            // 032A0 differs from02B10 only by the Sub-A reset at creation;
            // 4032F3 installs the identical02DA0 callback in task+18.
            | ActorTaskRuntime::DefecateVirusWander(_)
            // 02FB0 installs the same02DA0 in the cleansing movement task.
            | ActorTaskRuntime::CleansingLandscape(_)
            | ActorTaskRuntime::AttractAttentionTargetRoute(_)
            | ActorTaskRuntime::CapturePeoplePursuit(_)
            | ActorTaskRuntime::GoToJob(_)
            | ActorTaskRuntime::ChaseTarget(_)
            | ActorTaskRuntime::FollowBeaconsFollowing(_)
            | ActorTaskRuntime::CapturePeopleFollowing(_)
            | ActorTaskRuntime::RunAway(_)
            | ActorTaskRuntime::FishTargetRoute(_)
    )
}

pub(crate) fn private_state(task: &ActorTaskRuntime) -> Option<WanderNearPrivateState> {
    Some(match task {
        ActorTaskRuntime::OrdinaryType9Wander(task) => task.private_state(),
        ActorTaskRuntime::SharedRetarget(task) => task.private_state(),
        ActorTaskRuntime::CleansingLandscape(task) => task.private,
        ActorTaskRuntime::DefecateVirusWander(task) => task.private_state(),
        ActorTaskRuntime::AttractAttentionTargetRoute(task)
        | ActorTaskRuntime::CapturePeoplePursuit(task)
        | ActorTaskRuntime::FishTargetRoute(task) => task.private_state(),
        ActorTaskRuntime::GoToJob(task) => task.private_state(),
        ActorTaskRuntime::ChaseTarget(task) => task.private_state(),
        ActorTaskRuntime::FollowBeaconsFollowing(task)
        | ActorTaskRuntime::CapturePeopleFollowing(task) => task.private_state(),
        ActorTaskRuntime::RunAway(task) => task.private_state(),
        _ => return None,
    })
}

pub(crate) fn commit_private(task: &mut ActorTaskRuntime, private: WanderNearPrivateState) {
    match task {
        ActorTaskRuntime::OrdinaryType9Wander(task) => task.replace_private_state(private),
        ActorTaskRuntime::SharedRetarget(task) => task.apply_static_contact_private_state(private),
        ActorTaskRuntime::CleansingLandscape(task) => task.private = private,
        ActorTaskRuntime::DefecateVirusWander(task) => {
            *task = crate::defecate_virus::DefecateVirusWanderTaskState::from_parts(
                private,
                task.elapsed_ms(),
            );
        }
        ActorTaskRuntime::AttractAttentionTargetRoute(task)
        | ActorTaskRuntime::CapturePeoplePursuit(task)
        | ActorTaskRuntime::FishTargetRoute(task) => {
            let mut stage = task.stage_callback();
            *stage.private_state_mut() = private;
            stage.commit(task);
        }
        ActorTaskRuntime::GoToJob(task) => {
            let mut stage = task.stage_callback();
            *stage.private_state_mut() = private;
            stage.commit(task);
        }
        ActorTaskRuntime::ChaseTarget(task) => {
            let mut stage = task.stage_callback();
            *stage.private_state_mut() = private;
            stage.commit(task);
        }
        ActorTaskRuntime::FollowBeaconsFollowing(task)
        | ActorTaskRuntime::CapturePeopleFollowing(task) => {
            let mut stage = task.stage_callback();
            *stage.private_state_mut() = private;
            stage.commit(task);
        }
        ActorTaskRuntime::RunAway(task) => {
            let mut stage = task.stage_callback();
            *stage.private_state_mut() = private;
            stage.commit(task);
        }
        _ => unreachable!("authenticated descriptor task"),
    }
}

#[cfg(test)]
mod fish_tests;
#[cfg(test)]
mod pair_tests;
#[cfg(test)]
mod people_tests;
#[cfg(test)]
mod player_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod type97_tests;
