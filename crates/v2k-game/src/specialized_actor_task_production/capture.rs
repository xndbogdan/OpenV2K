//! Synchronous captor callbacks against the exact scheduler/cursor storage.
use super::*;
use crate::intro2_type8::NativeWorkerProfile;
use crate::native_actor_attachment::NativeActorAttachOutcome;
use crate::native_type86::NativeFourChoiceProfile;
use crate::{
    entity_relation_release::relation_release_state_word_after,
    intro2_type17::capture::{
        CaptureBlock, CaptureChildFrame, CaptureChildOperation, CaptureTaskCustody,
    },
    ordinary_type9_cargo::{
        self, Type9CargoReleasePosition, Type9CargoReleaseRequest, Type9RelationOwner,
    },
    ordinary_type9_current_task::OrdinaryType9CurrentTaskAuthority,
    ordinary_type9_go_to_job_initializer::OrdinaryType9GoToJobCandidateEvidence,
    ordinary_type9_standard_death::OrdinaryType9StandardDeathEntry,
};

impl SpecializedActorTaskScheduler {
    /// CD50's Class14 variant retires the graph synchronously. Remove its
    /// completed owner while retaining the body/relation for ordinary cleanup.
    pub(crate) fn finish_native_actor_attachment(
        &mut self,
        id: u32,
        outcome: NativeActorAttachOutcome,
    ) {
        if outcome == NativeActorAttachOutcome::DeferredDestroy {
            self.owners.retain(|owner| owner.entity_id() != id);
        }
    }
}

impl CaptureTaskCustody for SpecializedActorTaskScheduler {
    fn capture_child_mutation_ready(&mut self, manager: &EntityManager, child: u32) -> bool {
        ready(&mut self.owners, manager, child)
    }
    fn mutate_capture_child(
        &mut self,
        manager: &mut EntityManager,
        frame: CaptureChildFrame<'_>,
        prefix: &mut dyn FnMut(&mut EntityManager) -> Result<(), CaptureBlock>,
    ) -> Result<(), CaptureBlock> {
        mutate(&mut self.owners, manager, frame, prefix)
    }
}

impl CaptureTaskCustody for Intro2RadialCursorCustody<'_> {
    fn capture_child_mutation_ready(&mut self, manager: &EntityManager, child: u32) -> bool {
        let owners = if self.pending.iter().any(|owner| owner.entity_id() == child) {
            &mut *self.pending
        } else {
            &mut *self.retained
        };
        ready(owners, manager, child)
    }
    fn mutate_capture_child(
        &mut self,
        manager: &mut EntityManager,
        frame: CaptureChildFrame<'_>,
        prefix: &mut dyn FnMut(&mut EntityManager) -> Result<(), CaptureBlock>,
    ) -> Result<(), CaptureBlock> {
        // Replacement stays in the storage of the original child. A later
        // child gets its new callback this pass; an already visited child does
        // not receive an extra callback merely because the parent released it.
        let owners = if self
            .pending
            .iter()
            .any(|owner| owner.entity_id() == frame.child)
        {
            &mut *self.pending
        } else {
            &mut *self.retained
        };
        mutate(owners, manager, frame, prefix)
    }
}

fn blocked(reason: &'static str) -> CaptureBlock {
    CaptureBlock::new(reason)
}

fn ready(owners: &mut Vec<SpecializedActorTaskOwner>, manager: &EntityManager, id: u32) -> bool {
    if owners
        .iter()
        .any(|owner| owner.entity_id() == id && owner.is_delivered_type9_contact())
    {
        let authenticated =
            delivered_type9_contact::authenticated_retained_type9_contact(owners, manager, id);
        return delivered_type9_contact::with_retained_type9_contact(
            owners,
            id,
            authenticated,
            false,
            |current| ready(current, manager, id),
        );
    }
    if actual_child(manager, id).is_err() {
        return false;
    }
    if manager.iter_all().any(|entity| {
        entity.id == id && NativeWorkerProfile::from_entity_type(entity.entity_type).is_some()
    }) {
        owners.iter().any(|owner| {
            matches!(owner, SpecializedActorTaskOwner::Intro2Type8(owner)
                if owner.entity_id() == id && owner.completed_hit_boundary(manager))
        })
    } else if manager
        .iter_all()
        .any(|entity| entity.id == id && entity.entity_type == 123)
    {
        owners.iter().any(|owner| {
            matches!(owner, SpecializedActorTaskOwner::NativeType123(owner)
                if owner.entity_id() == id && owner.completed_hit_boundary(manager))
        })
    } else if manager.iter_all().any(|entity| {
        entity.id == id && NativeFourChoiceProfile::from_entity_type(entity.entity_type).is_some()
    }) {
        owners.iter().any(|owner| {
            matches!(owner, SpecializedActorTaskOwner::NativeType86(owner)
                if owner.entity_id() == id && owner.completed_hit_boundary(manager))
        })
    } else {
        begin_native_type9_external_mutation(owners, manager, id).is_some()
    }
}

fn actual_child(manager: &EntityManager, id: u32) -> Result<MainBaseAbortActorLease, CaptureBlock> {
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(blocked("capture child missing"))?;
    let native = match entity.entity_type {
        9 if entity.ordinary_type9_native_receipt.is_some() => {
            crate::ordinary_type9_construction::ordinary_type9_native_allocation_authenticates(
                manager, id,
            )
        }
        9 => crate::intro2_type9::intro2_type9_allocation_authenticates(entity),
        type_id if NativeWorkerProfile::from_entity_type(type_id).is_some() => {
            crate::intro2_type8::intro2_type8_manager_allocation_authenticates(manager, id)
        }
        123 => crate::native_type123::native_type123_manager_allocation_authenticates(manager, id),
        type_id if NativeFourChoiceProfile::from_entity_type(type_id).is_some() => {
            crate::native_type86::native_type86_manager_allocation_authenticates(manager, id)
        }
        _ => false,
    };
    if !native {
        return Err(blocked("capture child native constructor"));
    }
    manager
        .main_base_abort_actor_observation(id)
        .map(|observation| observation.lease)
        .ok_or(blocked("capture child lease"))
}

fn mutate(
    owners: &mut Vec<SpecializedActorTaskOwner>,
    manager: &mut EntityManager,
    frame: CaptureChildFrame<'_>,
    prefix: &mut dyn FnMut(&mut EntityManager) -> Result<(), CaptureBlock>,
) -> Result<(), CaptureBlock> {
    if owners
        .iter()
        .any(|owner| owner.entity_id() == frame.child && owner.is_delivered_type9_contact())
    {
        let authenticated = delivered_type9_contact::authenticated_retained_type9_contact(
            owners,
            manager,
            frame.child,
        );
        return delivered_type9_contact::with_retained_type9_contact(
            owners,
            frame.child,
            authenticated,
            Err(blocked("delivered peasant contact allocation")),
            |current| mutate(current, manager, frame, prefix),
        );
    }
    let CaptureChildFrame {
        child: id,
        operation,
        world_fx,
        notifications,
        retail_tick,
        result_screen,
    } = frame;
    let actor = actual_child(manager, id)?;
    let index = owners
        .iter()
        .position(|owner| owner.entity_id() == id)
        .ok_or(blocked("capture child task custody"))?;
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    let entity_type = entity.entity_type;
    if NativeWorkerProfile::from_entity_type(entity_type).is_some() {
        if !matches!(
            &owners[index],
            SpecializedActorTaskOwner::Intro2Type8(owner)
                if owner.completed_hit_boundary(manager)
        ) {
            return Err(blocked("capture worker pending callback"));
        }
        match operation {
            CaptureChildOperation::Attach { captor } => {
                let plan = crate::intro2_type8::cargo::prepare_attach(manager, id, captor)
                    .map_err(|_| blocked("capture worker attach"))?;
                prefix(manager)?;
                let entity = manager.entity_mut(id).unwrap();
                entity
                    .collision
                    .state_flags_at_0x08
                    .overwrite(0x1000, 0x1000);
                entity.attached_to = Some(captor);
                let outcome =
                    crate::intro2_type8::cargo::commit_attach(manager, id, plan, world_fx);
                if outcome == NativeActorAttachOutcome::DeferredDestroy {
                    owners.remove(index);
                }
            }
            CaptureChildOperation::Release {
                captor,
                defer_destroy,
            } => {
                let plan = crate::intro2_type8::cargo::prepare_release(
                    manager,
                    id,
                    captor,
                    Type9CargoReleasePosition::Retained,
                )
                .map_err(|_| blocked("capture worker release"))?;
                prefix(manager)?;
                crate::intro2_type8::cargo::commit_release(
                    manager.entity_mut(id).unwrap(),
                    plan,
                    world_fx,
                );
                if defer_destroy {
                    retire_delivered_child(manager, owners, index);
                }
            }
            CaptureChildOperation::StandardDeath => {
                prefix(manager)?;
                let entry_dying = manager
                    .entity_mut(id)
                    .unwrap()
                    .collision
                    .state_flags_at_0x08
                    .masked(crate::entity_collision_state::DYING_STATE_BIT);
                let death = match crate::intro2_type8::impact::run_intro2_type8_standard_death(
                    manager, id, world_fx,
                ) {
                    Ok(death) => death,
                    Err(_) => {
                        // 10C10 can write health/DYING, then DB80/C3A0 can
                        // stop on a missing component. Retain the actual child
                        // in its current scheduler/cursor storage. Metadata,
                        // allocation and entry-state errors consume no prefix.
                        let entity = manager.entity_mut(id).unwrap();
                        let committed_prefix = entry_dying == RetailRuntimeValue::Known(0)
                            && entity.collision.health_raw == RetailRuntimeValue::Known(0)
                            && entity
                                .collision
                                .state_flags_at_0x08
                                .masked(crate::entity_collision_state::DYING_STATE_BIT)
                                == RetailRuntimeValue::Known(
                                    crate::entity_collision_state::DYING_STATE_BIT,
                                );
                        if committed_prefix {
                            let SpecializedActorTaskOwner::Intro2Type8(owner) = &mut owners[index]
                            else {
                                unreachable!()
                            };
                            owner.park_external_prefix();
                        }
                        return Err(CaptureBlock {
                            reason: "capture worker standard death",
                            committed_prefix,
                        });
                    }
                };
                if let Some(owner) = death.publication {
                    owners[index] = SpecializedActorTaskOwner::Intro2Type8(owner);
                }
            }
        }
        return Ok(());
    }
    if entity_type == 123 {
        if !matches!(
            &owners[index],
            SpecializedActorTaskOwner::NativeType123(owner)
                if owner.completed_hit_boundary(manager)
        ) {
            return Err(blocked("capture person pending callback"));
        }
        match operation {
            CaptureChildOperation::Attach { captor } => {
                let plan = crate::native_type123::cargo::prepare_attach(manager, id, captor)
                    .map_err(|_| blocked("capture person attach"))?;
                prefix(manager)?;
                let entity = manager.entity_mut(id).unwrap();
                entity
                    .collision
                    .state_flags_at_0x08
                    .overwrite(0x1000, 0x1000);
                entity.attached_to = Some(captor);
                let outcome =
                    crate::native_type123::cargo::commit_attach(manager, id, plan, world_fx);
                if outcome == NativeActorAttachOutcome::DeferredDestroy {
                    owners.remove(index);
                }
            }
            CaptureChildOperation::Release {
                captor,
                defer_destroy,
            } => {
                let plan = crate::native_type123::cargo::prepare_release(
                    manager,
                    id,
                    captor,
                    Type9CargoReleasePosition::Retained,
                )
                .map_err(|_| blocked("capture person release"))?;
                prefix(manager)?;
                let owner =
                    crate::native_type123::cargo::commit_release(manager, id, plan, world_fx);
                owners[index] = SpecializedActorTaskOwner::NativeType123(owner);
                if defer_destroy {
                    retire_delivered_child(manager, owners, index);
                }
            }
            CaptureChildOperation::StandardDeath => {
                prefix(manager)?;
                let entry_dying = manager
                    .entity_mut(id)
                    .unwrap()
                    .collision
                    .state_flags_at_0x08
                    .masked(crate::entity_collision_state::DYING_STATE_BIT);
                let death = match crate::native_type123::impact::run_native_type123_standard_death(
                    manager, id, world_fx,
                ) {
                    Ok(death) => death,
                    Err(_) => {
                        // 10C10 can write health/DYING, then DB80/C3A0 can
                        // stop on a missing component. Retain the actual child
                        // in its current scheduler/cursor storage. Metadata,
                        // allocation and entry-state errors consume no prefix.
                        let entity = manager.entity_mut(id).unwrap();
                        let committed_prefix = entry_dying == RetailRuntimeValue::Known(0)
                            && entity.collision.health_raw == RetailRuntimeValue::Known(0)
                            && entity
                                .collision
                                .state_flags_at_0x08
                                .masked(crate::entity_collision_state::DYING_STATE_BIT)
                                == RetailRuntimeValue::Known(
                                    crate::entity_collision_state::DYING_STATE_BIT,
                                );
                        if committed_prefix {
                            let SpecializedActorTaskOwner::NativeType123(owner) =
                                &mut owners[index]
                            else {
                                unreachable!()
                            };
                            owner.park_external_prefix();
                        }
                        return Err(CaptureBlock {
                            reason: "capture person standard death",
                            committed_prefix,
                        });
                    }
                };
                if let Some(owner) = death.publication {
                    owners[index] = SpecializedActorTaskOwner::NativeType123(owner);
                }
            }
        }
        return Ok(());
    }
    if NativeFourChoiceProfile::from_entity_type(entity_type).is_some() {
        if !matches!(
            &owners[index],
            SpecializedActorTaskOwner::NativeType86(owner)
                if owner.completed_hit_boundary(manager)
        ) {
            return Err(blocked("capture four-choice actor pending callback"));
        }
        match operation {
            CaptureChildOperation::Attach { captor } => {
                let plan = crate::native_type86::cargo::prepare_attach(manager, id, captor)
                    .map_err(|_| blocked("capture four-choice actor attach"))?;
                prefix(manager)?;
                let entity = manager.entity_mut(id).unwrap();
                entity
                    .collision
                    .state_flags_at_0x08
                    .overwrite(0x1000, 0x1000);
                entity.attached_to = Some(captor);
                let outcome =
                    crate::native_type86::cargo::commit_attach(manager, id, plan, world_fx);
                if outcome == NativeActorAttachOutcome::DeferredDestroy {
                    owners.remove(index);
                }
            }
            CaptureChildOperation::Release {
                captor,
                defer_destroy,
            } => {
                let plan = crate::native_type86::cargo::prepare_release(
                    manager,
                    id,
                    captor,
                    Type9CargoReleasePosition::Retained,
                )
                .map_err(|_| blocked("capture four-choice actor release"))?;
                prefix(manager)?;
                let owner =
                    crate::native_type86::cargo::commit_release(manager, id, plan, world_fx);
                owners[index] = SpecializedActorTaskOwner::NativeType86(owner);
                if notifications
                    .drain_attract_attention_receipts(manager, retail_tick as i32)
                    .is_err()
                {
                    let SpecializedActorTaskOwner::NativeType86(owner) = &mut owners[index] else {
                        unreachable!("release just published the four-choice owner")
                    };
                    owner.park_external_prefix();
                    return Err(CaptureBlock {
                        reason: "capture four-choice actor notification receipt",
                        committed_prefix: true,
                    });
                }
                if defer_destroy {
                    retire_delivered_child(manager, owners, index);
                }
            }
            CaptureChildOperation::StandardDeath => {
                prefix(manager)?;
                let entry_dying = manager
                    .entity_mut(id)
                    .unwrap()
                    .collision
                    .state_flags_at_0x08
                    .masked(crate::entity_collision_state::DYING_STATE_BIT);
                let death = match crate::native_type86::impact::run_native_type86_standard_death(
                    manager, id, world_fx,
                ) {
                    Ok(death) => death,
                    Err(_) => {
                        // 10C10 can write health/DYING, then DB80/C3A0 can
                        // stop on a missing component. Retain the actual child
                        // in its current scheduler/cursor storage. Metadata,
                        // allocation and entry-state errors consume no prefix.
                        let entity = manager.entity_mut(id).unwrap();
                        let committed_prefix = entry_dying == RetailRuntimeValue::Known(0)
                            && entity.collision.health_raw == RetailRuntimeValue::Known(0)
                            && entity
                                .collision
                                .state_flags_at_0x08
                                .masked(crate::entity_collision_state::DYING_STATE_BIT)
                                == RetailRuntimeValue::Known(
                                    crate::entity_collision_state::DYING_STATE_BIT,
                                );
                        if committed_prefix {
                            let SpecializedActorTaskOwner::NativeType86(owner) = &mut owners[index]
                            else {
                                unreachable!()
                            };
                            owner.park_external_prefix();
                        }
                        return Err(CaptureBlock {
                            reason: "capture four-choice actor standard death",
                            committed_prefix,
                        });
                    }
                };
                if let Some(owner) = death.publication {
                    owners[index] = SpecializedActorTaskOwner::NativeType86(owner);
                }
            }
        }
        return Ok(());
    }
    if begin_native_type9_external_mutation(owners, manager, id).is_none() {
        return Err(blocked("capture peasant pending callback"));
    }
    match operation {
        CaptureChildOperation::Attach { captor } => {
            if !matches!(
                &owners[index],
                SpecializedActorTaskOwner::OrdinaryType9Wander(_)
                    | SpecializedActorTaskOwner::OrdinaryType9RunAway(_)
                    | SpecializedActorTaskOwner::OrdinaryType9GoToJob(_)
                    | SpecializedActorTaskOwner::OrdinaryType9AttractAttention(_)
            ) {
                return Err(blocked("capture peasant selected graph"));
            }
            let parent = manager
                .iter_all()
                .find(|entity| entity.id == captor)
                .ok_or(blocked("capture parent missing"))?;
            let metadata = manager
                .type_runtime_metadata(9)
                .ok_or(blocked("capture peasant metadata"))?;
            let plan = ordinary_type9_cargo::plan_attach(
                manager.iter_all().find(|entity| entity.id == id).unwrap(),
                metadata,
                Type9RelationOwner {
                    id: captor,
                    capability_flags: parent.capability_flags,
                    position_raw: parent.position_raw(),
                },
            )
            .map_err(|_| blocked("capture peasant attach"))?;
            prefix(manager)?;
            let entity = manager.entity_mut(id).unwrap();
            entity
                .collision
                .state_flags_at_0x08
                .overwrite(0x1000, 0x1000);
            entity.attached_to = Some(captor);
            if let Some((sound, position)) = plan.sound() {
                world_fx.queue_fixed_positional_sound_raw(sound, position);
            }
            let owner = ordinary_type9_cargo::commit_attach(entity, actor, plan).map_err(|_| {
                CaptureBlock {
                    reason: "capture peasant attach publication",
                    committed_prefix: true,
                }
            })?;
            owners[index] = SpecializedActorTaskOwner::OrdinaryType9Carried(owner);
        }
        CaptureChildOperation::Release {
            captor,
            defer_destroy,
        } => {
            let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
            if entity.attached_to != Some(captor) {
                return Err(blocked("capture peasant parent changed"));
            }
            if matches!(
                &owners[index],
                SpecializedActorTaskOwner::Intro2Type9Class14(_)
                    | SpecializedActorTaskOwner::OrdinaryType9Class14(_)
            ) {
                let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context
                else {
                    return Err(blocked("capture corpse context"));
                };
                if context.active_style().release_callback_policy()
                    != crate::entity_behavior::ReleaseCallbackPolicy::None
                {
                    return Err(blocked("capture corpse release hook"));
                }
                let metadata = manager
                    .type_runtime_metadata(9)
                    .ok_or(blocked("capture corpse metadata"))?;
                if !crate::main_base_type9_abort::exact_level_one_type9_metadata(metadata) {
                    return Err(blocked("capture corpse metadata"));
                }
                prefix(manager)?;
                let entity = manager.entity_mut(id).unwrap();
                entity.collision.state_flags_at_0x08 =
                    relation_release_state_word_after(entity.collision.state_flags_at_0x08, 0x2f);
                entity.collision.default_state_flags_at_0xc8 = RetailRuntimeValue::Known(0x2f);
                entity.attached_to = None;
                // Null DC50 style callback preserves the exact corpse wrapper,
                // elapsed clocks and Sub-I state. It does not reselect CE90.
            } else {
                let SpecializedActorTaskOwner::OrdinaryType9Carried(carried) = &owners[index]
                else {
                    return Err(blocked("capture peasant carried custody"));
                };
                let metadata = manager
                    .type_runtime_metadata(9)
                    .ok_or(blocked("capture peasant metadata"))?
                    .clone();
                let candidates = manager
                    .retail_live_order_ids()
                    .filter_map(|id| manager.iter_all().find(|entity| entity.id == id))
                    .map(|entity| {
                        crate::ordinary_type9_root_reselection::OrdinaryType9RootEntityRef {
                            id: entity.id,
                            entity_type: entity.entity_type,
                            position_raw: entity.position_raw(),
                            state_flags_raw: entity.collision.state_flags_at_0x08,
                            capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
                            attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
                        }
                    })
                    .collect::<Vec<_>>();
                let jobs = manager
                    .retail_live_order_ids()
                    .filter_map(|id| manager.iter_all().find(|entity| entity.id == id))
                    .map(|entity| OrdinaryType9GoToJobCandidateEvidence {
                        candidate_id: entity.id,
                        state_flags: entity.collision.state_flags_at_0x08,
                        capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
                        capacity: match entity.base_factory_runtime {
                            RetailRuntimeValue::Known(Some(state)) => RetailRuntimeValue::Known(
                                Some(crate::job_nearby::JobCapacityState {
                                    current_jobs_raw: i32::from(state.current_scientists),
                                    capacity_raw: i32::from(state.required_scientists),
                                }),
                            ),
                            RetailRuntimeValue::Known(None) => RetailRuntimeValue::Known(None),
                            RetailRuntimeValue::Unresolved => RetailRuntimeValue::Unresolved,
                        },
                    })
                    .collect::<Vec<_>>();
                let request = Type9CargoReleaseRequest {
                    metadata: &metadata,
                    candidates: &candidates,
                    job_evidence: &jobs,
                    position: Type9CargoReleasePosition::Retained,
                };
                let prepared = ordinary_type9_cargo::prepare_release(entity, carried, &request)
                    .map_err(|_| blocked("capture peasant release"))?;
                prefix(manager)?;
                let SpecializedActorTaskOwner::OrdinaryType9Carried(carried) = owners.remove(index)
                else {
                    unreachable!()
                };
                let mut sounds = Vec::new();
                let publication = ordinary_type9_cargo::commit_release(
                    manager.entity_mut(id).unwrap(),
                    carried,
                    prepared,
                    request,
                    || u32::from(world_fx.next_shared_retail_random_u16()),
                    |text| {
                        // The class45 initializer emits its resource request
                        // before constructing Cue/Primary and consuming their
                        // RNG. Keep the notification in that live callback.
                        notifications
                            .queue_attract_attention_resource_text(text, retail_tick as i32)
                            .expect("canonical resource event");
                    },
                    |sound| sounds.push(sound),
                )
                .expect("child graph preflight and captor-only row prefix");
                for sound in sounds {
                    world_fx.queue_fixed_positional_sound_raw(
                        sound.global_sound_id,
                        sound.position_raw,
                    );
                }
                let authority = OrdinaryType9CurrentTaskAuthority::from_publication(
                    manager.iter_all().find(|entity| entity.id == id).unwrap(),
                    publication,
                )
                .expect("completed release publication");
                owners.insert(
                    index,
                    SpecializedActorTaskOwner::from_current_type9(authority, actor, 1),
                );
            }
            if defer_destroy {
                retire_delivered_child(manager, owners, index);
            }
        }
        CaptureChildOperation::StandardDeath => {
            prefix(manager)?;
            manager
                .publish_ordinary_type9_standard_death(
                    id,
                    OrdinaryType9StandardDeathEntry::GenericDeath,
                    result_screen,
                    world_fx,
                    retail_tick as i32,
                    Some(notifications),
                )
                .map_err(|_| blocked("capture peasant standard death"))?;
            if let Some(lease) =
                crate::ordinary_type9_outer_tail::outer_tail_class14_exploding_task_lease(
                    manager, actor,
                )
            {
                if !matches!(
                    &owners[index],
                    SpecializedActorTaskOwner::Intro2Type9Class14(_)
                        | SpecializedActorTaskOwner::OrdinaryType9Class14(_)
                ) {
                    owners[index] = SpecializedActorTaskOwner::Intro2Type9Class14(
                        crate::intro2_type9_class14::Intro2Type9Class14Owner::adopt(lease),
                    );
                }
            }
        }
    }
    Ok(())
}

fn retire_delivered_child(
    manager: &mut EntityManager,
    owners: &mut Vec<SpecializedActorTaskOwner>,
    index: usize,
) {
    let id = owners[index].entity_id();
    // 443D10 calls this only after a successful 16750; no health write,
    // explosion constructor or death sound is implied by the destroy callback.
    // 10B70 accepts only a body whose pending bit was clear. A later
    // same-walk reattachment/release can publish another completed graph on
    // the queued allocation; it must not repeat the 0x60000 clear or enqueue.
    if manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .masked(0x100000)
        != RetailRuntimeValue::Known(0x100000)
    {
        manager
            .entity_mut(id)
            .unwrap()
            .mark_actor_deferred_destroy_pending();
        manager.queue_actor_deferred_destroy(id);
    }
    // 10B70 clears ordinary callback flags but retains the completed16750
    // graph until14990. Preserve the genuine Type9 publication for a later
    // contact in this same walk, without advancing its newborn release task.
    let retained = owners.remove(index);
    if let Some(allocation) = retained.type9_actor_lease() {
        owners.insert(
            index,
            SpecializedActorTaskOwner::DeliveredType9Contact {
                allocation,
                retained: Box::new(retained),
            },
        );
    }
}

#[cfg(test)]
mod tests;
