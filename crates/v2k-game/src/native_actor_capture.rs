//! C910's captor-owned row and the distinct D040/D0B0/CF90 consumers.
//!
//! The row callbacks are CAD0 (16750 with the captor argument) and 443D10
//! (16750 with the child argument, then 10B70 on success). These are not the
//! player's materialiser callback. All child work is synchronous, including
//! while the actor-list cursor owns the child in its pending/retained storage.

pub(crate) mod carry_tasks;
pub mod pair;
#[cfg(test)]
mod pair_type40_tests;
#[cfg(test)]
pub(crate) mod tests;

use crate::{
    entity::{Entity, EntityManager},
    entity_collision_state::{RetailRuntimeValue, DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT},
    gameplay_notifications::GameplayNotifications,
    intro2_common_dying::Intro2CommonDyingOwner,
    main_base_abort::MainBaseAbortActorLease,
    world_fx::WorldFx,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeCaptureRelation {
    profile: NativeCaptorProfile,
    pub(crate) captor: MainBaseAbortActorLease,
    pub(crate) child: MainBaseAbortActorLease,
    pub(crate) row_present: bool,
}

/// Shared retail callbacks do not grant a shared construction identity.
/// Each profile authenticates its own allocation and complete authored body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeCaptorProfile {
    Type17,
    Type122,
    Type18,
    Type28,
}

impl NativeCaptorProfile {
    pub(crate) fn authenticate(manager: &EntityManager, id: u32) -> Result<Self, CaptureBlock> {
        let entity = actor(manager, id)?;
        let profile = match entity.entity_type {
            17 if crate::intro2_type17::type17_manager_allocation_authenticates(manager, id) => {
                Self::Type17
            }
            122 if crate::native_type122::type122_manager_allocation_authenticates(manager, id) => {
                Self::Type122
            }
            18 if crate::native_type18::manager_allocation_authenticates(manager, id) => {
                Self::Type18
            }
            28 if crate::native_type28::manager_allocation_authenticates(manager, id) => {
                Self::Type28
            }
            _ => return Err(CaptureBlock::new("capture native allocation")),
        };
        let metadata = manager
            .type_runtime_metadata(profile.entity_type())
            .ok_or(CaptureBlock::new("capture native metadata"))?;
        let authentic = match profile {
            Self::Type17 => crate::intro2_type17::authenticate_metadata(metadata).is_ok(),
            Self::Type122 => crate::native_type122::authenticate_metadata(metadata).is_ok(),
            Self::Type18 => crate::native_type18::authenticate_metadata(metadata).is_ok(),
            Self::Type28 => crate::native_type28::authenticate_metadata(metadata).is_ok(),
        };
        if !authentic {
            return Err(CaptureBlock::new("capture native metadata"));
        }
        Ok(profile)
    }

    pub(crate) fn entity_type(self) -> u32 {
        match self {
            Self::Type17 => 17,
            Self::Type122 => 122,
            Self::Type18 => 18,
            Self::Type28 => 28,
        }
    }

    fn reselect(
        self,
        manager: &mut EntityManager,
        id: u32,
        context: &mut CaptureContext<'_>,
    ) -> Result<(), CaptureBlock> {
        let result = match self {
            Self::Type17 => crate::intro2_type17::behavior::reselect(
                manager,
                id,
                context.retail_tick,
                context.world_fx,
                crate::intro2_type17::behavior::ReselectionEntry::Impact,
            )
            .map_err(|_| ()),
            Self::Type122 => crate::native_ground_actor::behavior::reselect::<
                crate::native_type122::profile::Type122Profile,
            >(
                manager,
                id,
                context.retail_tick,
                context.world_fx,
                context.resources,
                crate::native_ground_actor::behavior::ReselectionEntry::Impact,
            )
            .map_err(|_| ()),
            // Type18/28 roots weigh rule8: the entry must lend its world.
            Self::Type18 => crate::native_ground_actor::behavior::reselect::<
                crate::native_type18::profile::Type18Profile,
            >(
                manager,
                id,
                context.retail_tick,
                context.world_fx,
                context.resources,
                crate::native_ground_actor::behavior::ReselectionEntry::Impact,
            )
            .map_err(|_| ()),
            Self::Type28 => crate::native_ground_actor::behavior::reselect::<
                crate::native_type28::profile::Type28Profile,
            >(
                manager,
                id,
                context.retail_tick,
                context.world_fx,
                context.resources,
                crate::native_ground_actor::behavior::ReselectionEntry::Impact,
            )
            .map_err(|_| ()),
        };
        result.map_err(|_| CaptureBlock::new("capture living root selection").committed())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureBlock {
    pub reason: &'static str,
    pub committed_prefix: bool,
}
impl CaptureBlock {
    pub(crate) const fn new(reason: &'static str) -> Self {
        Self {
            reason,
            committed_prefix: false,
        }
    }
    fn committed(mut self) -> Self {
        self.committed_prefix = true;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureChildOperation {
    Attach { captor: u32 },
    Release { captor: u32, defer_destroy: bool },
    StandardDeath,
}

pub struct CaptureChildFrame<'a> {
    pub child: u32,
    pub operation: CaptureChildOperation,
    pub world_fx: &'a mut WorldFx,
    pub notifications: &'a mut GameplayNotifications,
    pub retail_tick: u32,
    pub result_screen: crate::main_base_type9_abort::MainBaseType9ResultScreenState,
}

/// Preflight the child's actual completed graph, then run the caller's row
/// prefix immediately before its child callback. Unknown evidence is a block,
/// never a fabricated nonzero retail callback result.
pub trait CaptureTaskCustody {
    fn capture_child_mutation_ready(&mut self, manager: &EntityManager, child: u32) -> bool;
    fn mutate_capture_child(
        &mut self,
        manager: &mut EntityManager,
        frame: CaptureChildFrame<'_>,
        prefix: &mut dyn FnMut(&mut EntityManager) -> Result<(), CaptureBlock>,
    ) -> Result<(), CaptureBlock>;
}

/// `FUN_00440950` inputs for a Hive67 capture-destination lethal.
#[derive(Clone, Copy, Default)]
pub struct CaptureHiveDyingBurst<'a> {
    pub model_extent_raw: Option<&'a dyn Fn(usize) -> Option<u16>>,
    pub sea_level_raw: Option<i16>,
}

pub struct CaptureContext<'a> {
    /// The resident world, when the entry holds it. A captor whose living
    /// root weighs rule8 (FurnitureNearby) needs it for C690's object scan.
    pub resources: Option<&'a crate::resource_cache::ResourceCache>,
    pub tasks: &'a mut dyn CaptureTaskCustody,
    pub world_fx: &'a mut WorldFx,
    pub notifications: &'a mut GameplayNotifications,
    pub retail_tick: u32,
    pub result_screen: crate::main_base_type9_abort::MainBaseType9ResultScreenState,
    pub hive_dying: CaptureHiveDyingBurst<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureRootCallback {
    ReleaseOrKill,
    Cleanup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CaptureCallbackCompletion {
    pub common_dying_owner: Option<Intro2CommonDyingOwner>,
}

pub(crate) fn actor(manager: &EntityManager, id: u32) -> Result<&Entity, CaptureBlock> {
    manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(CaptureBlock::new("capture allocation unavailable"))
}

fn captor(manager: &EntityManager, id: u32) -> Result<&Entity, CaptureBlock> {
    NativeCaptorProfile::authenticate(manager, id)?;
    actor(manager, id)
}

/// 235D0 restores both type-authored common-axis words.
fn reset_filter(manager: &mut EntityManager, id: u32) -> Result<(), CaptureBlock> {
    let entity = captor(manager, id)?;
    let RetailRuntimeValue::Known(_) = entity.actor_common_axis_descriptor else {
        return Err(CaptureBlock::new("capture axis storage"));
    };
    let axis = manager
        .type_runtime_metadata(entity.entity_type)
        .and_then(|metadata| metadata.initializer.as_ref())
        .ok_or(CaptureBlock::new("capture filter metadata"))?
        .common_axis_descriptor;
    manager.entity_mut(id).unwrap().actor_common_axis_descriptor = RetailRuntimeValue::Known(axis);
    Ok(())
}

pub fn attach_capture_child(
    manager: &mut EntityManager,
    id: u32,
    child: u32,
    context: &mut CaptureContext<'_>,
) -> Result<(), CaptureBlock> {
    let profile = NativeCaptorProfile::authenticate(manager, id)?;
    let entity = captor(manager, id)?;
    let RetailRuntimeValue::Known(Some(rows)) = &entity.sub_j_attachment_runtime else {
        return Err(CaptureBlock::new("capture Sub-J storage"));
    };
    if rows.authored_slot_count() != 1
        || rows.capacity() != 1
        || rows.policy_raw_at_0x0c() != 0
        || !rows.is_empty()
        || entity.native_capture_relation.is_some()
    {
        return Err(CaptureBlock::new("capture free row custody"));
    }
    let relation = NativeCaptureRelation {
        profile,
        captor: manager.main_base_abort_actor_observation(id).unwrap().lease,
        child: manager
            .main_base_abort_actor_observation(child)
            .ok_or(CaptureBlock::new("capture child allocation"))?
            .lease,
        row_present: true,
    };
    context.tasks.mutate_capture_child(
        manager,
        CaptureChildFrame {
            child,
            operation: CaptureChildOperation::Attach { captor: id },
            world_fx: context.world_fx,
            notifications: context.notifications,
            retail_tick: context.retail_tick,
            result_screen: context.result_screen,
        },
        &mut |manager| {
            let entity = manager.entity_mut(id).unwrap();
            let RetailRuntimeValue::Known(Some(rows)) = &mut entity.sub_j_attachment_runtime else {
                unreachable!()
            };
            rows.append(child)
                .map_err(|_| CaptureBlock::new("capture row changed"))?;
            entity.native_capture_relation = Some(relation);
            Ok(())
        },
    )
}

fn held_child(manager: &EntityManager, id: u32) -> Result<Option<u32>, CaptureBlock> {
    let profile = NativeCaptorProfile::authenticate(manager, id)?;
    let entity = captor(manager, id)?;
    let RetailRuntimeValue::Known(Some(rows)) = &entity.sub_j_attachment_runtime else {
        return Err(CaptureBlock::new("capture Sub-J storage"));
    };
    let relation = entity
        .native_capture_relation
        .ok_or(CaptureBlock::new("capture row callback custody"))?;
    if relation.profile != profile {
        return Err(CaptureBlock::new("capture row profile"));
    }
    if !relation.row_present && rows.is_empty() {
        if manager
            .main_base_abort_actor_observation(id)
            .map(|actor| actor.lease)
            != Some(relation.captor)
        {
            return Err(CaptureBlock::new("capture popped allocation changed"));
        }
        return Ok(None);
    }
    if rows.ordered_entity_ids() != [relation.child.entity_id]
        || manager
            .main_base_abort_actor_observation(id)
            .map(|actor| actor.lease)
            != Some(relation.captor)
        || manager
            .main_base_abort_actor_observation(relation.child.entity_id)
            .map(|actor| actor.lease)
            != Some(relation.child)
        || actor(manager, relation.child.entity_id)?.attached_to != Some(id)
    {
        return Err(CaptureBlock::new("capture relation allocation changed"));
    }
    Ok(Some(relation.child.entity_id))
}

/// Authenticate the retained callback row without executing a child callback.
/// 18640 may already have compacted a dead child; its empty-row receipt remains
/// valid until the current carrying root consumes it.
pub(crate) fn validate_capture_relation(
    manager: &EntityManager,
    id: u32,
) -> Result<(), CaptureBlock> {
    held_child(manager, id).map(|_| ())
}

fn pop_child(
    manager: &mut EntityManager,
    id: u32,
    destroy: bool,
    context: &mut CaptureContext<'_>,
) -> Result<Option<u32>, CaptureBlock> {
    let Some(child) = held_child(manager, id)? else {
        return Ok(None);
    };
    context.tasks.mutate_capture_child(
        manager,
        CaptureChildFrame {
            child,
            operation: CaptureChildOperation::Release {
                captor: id,
                defer_destroy: destroy,
            },
            world_fx: context.world_fx,
            notifications: context.notifications,
            retail_tick: context.retail_tick,
            result_screen: context.result_screen,
        },
        &mut |manager| {
            let entity = manager.entity_mut(id).unwrap();
            let RetailRuntimeValue::Known(Some(rows)) = &mut entity.sub_j_attachment_runtime else {
                unreachable!()
            };
            if rows.pop_last() != Some(child) {
                return Err(CaptureBlock::new("capture pop changed"));
            }
            entity.native_capture_relation.as_mut().unwrap().row_present = false;
            Ok(())
        },
    )?;
    Ok(Some(child))
}

fn reselect(
    manager: &mut EntityManager,
    id: u32,
    context: &mut CaptureContext<'_>,
) -> Result<CaptureCallbackCompletion, CaptureBlock> {
    let flags = actor(manager, id)?
        .collision
        .state_flags_at_0x08
        .masked(DYING_STATE_BIT);
    match flags {
        RetailRuntimeValue::Known(0) => {
            NativeCaptorProfile::authenticate(manager, id)?.reselect(manager, id, context)?;
            manager.entity_mut(id).unwrap().native_capture_relation = None;
            Ok(CaptureCallbackCompletion::default())
        }
        RetailRuntimeValue::Known(DYING_STATE_BIT) => {
            let owner = crate::intro2_common_dying::publish_intro2_common_dying_alternate(
                manager,
                id,
                context.world_fx,
            )
            .map_err(|_| CaptureBlock::new("capture dying alternate").committed())?;
            manager.entity_mut(id).unwrap().native_capture_relation = None;
            Ok(CaptureCallbackCompletion {
                common_dying_owner: owner,
            })
        }
        _ => Err(CaptureBlock::new("capture root state").committed()),
    }
}

pub fn execute_capture_root(
    manager: &mut EntityManager,
    id: u32,
    callback: CaptureRootCallback,
    context: &mut CaptureContext<'_>,
) -> Result<CaptureCallbackCompletion, CaptureBlock> {
    reset_filter(manager, id)?;
    // CF90 samples even for an empty Sub-J. D040 itself consumes no RNG.
    let kill = callback == CaptureRootCallback::ReleaseOrKill
        && context.world_fx.next_shared_retail_random_u16() & 3 == 0;
    let child = pop_child(manager, id, false, context).map_err(CaptureBlock::committed)?;
    if callback == CaptureRootCallback::Cleanup && child.is_none() {
        // 18500 returns BE5B0. D040 returns it without C690; its caller owns
        // disposal/continuation. No row is invented for the empty case.
        return Ok(CaptureCallbackCompletion::default());
    }
    if kill {
        if let Some(child) = child {
            kill_capture_contact(manager, child, context).map_err(CaptureBlock::committed)?;
        }
        // The empty-row output remains DAT4DCA00; its 10C10 is a no-op.
    }
    reselect(manager, id, context)
}

pub fn execute_capture_delivery(
    manager: &mut EntityManager,
    id: u32,
    contacted: u32,
    context: &mut CaptureContext<'_>,
) -> Result<CaptureCallbackCompletion, CaptureBlock> {
    reset_filter(manager, id)?;
    if !manager
        .iter_all()
        .any(|entity| entity.id == contacted && entity.capability_flags & 0x10 != 0)
    {
        return Ok(CaptureCallbackCompletion::default());
    }
    if pop_child(manager, id, true, context)
        .map_err(CaptureBlock::committed)?
        .is_none()
    {
        return Err(CaptureBlock::new("18590 requires its actual captured row").committed());
    }
    reselect(manager, id, context)
}

pub fn kill_capture_contact(
    manager: &mut EntityManager,
    child: u32,
    context: &mut CaptureContext<'_>,
) -> Result<(), CaptureBlock> {
    context.tasks.mutate_capture_child(
        manager,
        CaptureChildFrame {
            child,
            operation: CaptureChildOperation::StandardDeath,
            world_fx: context.world_fx,
            notifications: context.notifications,
            retail_tick: context.retail_tick,
            result_screen: context.result_screen,
        },
        &mut |_| Ok(()),
    )
}

/// The active-pair physical cap enters 15040 directly, after both participants'
/// displacement/velocity writes. It does not execute DAC0/DA00 or particle
/// accepted-hit presentation. Child replacement remains in cursor custody.
pub fn apply_capture_pair_checked_damage(
    manager: &mut EntityManager,
    target: u32,
    delivery: crate::damage::DamageDeliveryRecord,
    context: &mut CaptureContext<'_>,
) -> Result<CaptureCallbackCompletion, CaptureBlock> {
    use crate::live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageFeedback,
        LiveActorDamageRequest, LiveActorDeathResult,
    };
    let target_type = actor(manager, target)?.entity_type;
    let is_captor = matches!(target_type, 17 | 122 | 18 | 28);
    let is_hive_destination = target_type == 67;
    if is_captor {
        captor(manager, target)?;
    } else if !is_hive_destination && !context.tasks.capture_child_mutation_ready(manager, target) {
        return Err(CaptureBlock::new("capture physical child custody"));
    }
    let hive_dying = context.hive_dying;
    let result = apply_live_actor_checked_damage(
        manager,
        context.world_fx,
        LiveActorDamageRequest {
            ratio_numerator: 0,
            ratio_denominator: 0,
            entity_id: target,
            delivery,
            entry: LiveActorDamageEntry::Checked,
            feedback: Some(LiveActorDamageFeedback {
                notifications: context.notifications,
                retail_tick: context.retail_tick,
            }),
        },
        |manager, world_fx, feedback| {
            // A destination's 15040 admission/filter/buffer/surviving-health path
            // is generic. Its separate 10C10/Sub-K/radial lifecycle is not a child
            // release callback. Retain the reached lethal prefix until that Hive
            // owner is shared; never call the projectile adapter and stamp a hit.
            if is_hive_destination {
                let dying_model = manager
                    .entity_mut(target)
                    .and_then(|entity| entity.apply_hive_dying_initializer())
                    .ok_or_else(|| CaptureBlock::new("hive dying initializer"))?;
                let extent = hive_dying
                    .model_extent_raw
                    .and_then(|lookup| lookup(dying_model))
                    .unwrap_or(0);
                crate::hive_death::emit_hive_dying_surface_burst_for_entity(
                    manager,
                    world_fx,
                    target,
                    extent,
                    hive_dying.sea_level_raw,
                );
                return Ok(LiveActorDeathResult {
                    returned_nonzero: true,
                    publication: None,
                });
            }
            let feedback =
                feedback.ok_or(CaptureBlock::new("capture physical notification context"))?;
            let mut nested = CaptureContext {
                resources: None,
                tasks: context.tasks,
                world_fx,
                notifications: feedback.notifications,
                retail_tick: feedback.retail_tick,
                result_screen: context.result_screen,
                hive_dying,
            };
            let publication = if is_captor {
                publish_native_captor_standard_death(manager, target, &mut nested)?
            } else {
                kill_capture_contact(manager, target, &mut nested)?;
                None
            };
            let returned_nonzero = actor(manager, target)?
                .collision
                .state_flags_at_0x08
                .masked(DYING_STATE_BIT)
                == RetailRuntimeValue::Known(DYING_STATE_BIT);
            Ok::<_, CaptureBlock>(LiveActorDeathResult {
                returned_nonzero,
                publication,
            })
        },
    )
    .map_err(|error| CaptureBlock {
        reason: "capture physical checked damage",
        committed_prefix: error.committed_prefix,
    })?;
    Ok(CaptureCallbackCompletion {
        common_dying_owner: result.death_publication,
    })
}

/// 10C10 -> DB80. On successful occupied-row cleanup dying C690 installs
/// C620 once; DB80 then rereads the current AC40/context and installs it again.
/// Primary DAC0 cleanup is a separate, earlier living selection. Infected
/// DA00 has a null carry-style +20 hook and retains the row until lethal DB80.
pub fn publish_native_captor_standard_death(
    manager: &mut EntityManager,
    id: u32,
    context: &mut CaptureContext<'_>,
) -> Result<Option<Intro2CommonDyingOwner>, CaptureBlock> {
    let entity = captor(manager, id)?;
    let flags = entity
        .collision
        .state_flags_at_0x08
        .masked(REMOTE_OWNED_STATE_BIT | DYING_STATE_BIT);
    let RetailRuntimeValue::Known(flags) = flags else {
        return Err(CaptureBlock::new("capture death state"));
    };
    if flags != 0 {
        return Ok(None);
    }
    let RetailRuntimeValue::Known(Some(behavior)) = entity.current_behavior_context else {
        return Err(CaptureBlock::new("capture death context"));
    };
    if behavior.active_style().death_callback_policy()
        != crate::entity_behavior::DeathCallbackPolicy::CapturePeopleCleanup
    {
        return crate::intro2_common_dying::publish_intro2_common_standard_death(
            manager,
            id,
            context.world_fx,
        )
        .map_err(|_| CaptureBlock::new("capture standard death"));
    }
    let metadata = manager
        .type_runtime_metadata(entity.entity_type)
        .ok_or(CaptureBlock::new("capture death metadata"))?;
    let RetailRuntimeValue::Known(sound) = metadata.death_sound_id else {
        return Err(CaptureBlock::new("capture death sound"));
    };
    if entity.collision.constructor_sound_attachment_id_at_0x8c != RetailRuntimeValue::Known(None) {
        return Err(CaptureBlock::new("capture attached death sound"));
    }
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.health_raw = RetailRuntimeValue::Known(0);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
    if let Some(sound) = sound {
        context
            .world_fx
            .queue_fixed_positional_sound_raw(sound, entity.position_raw());
    }
    execute_capture_root(manager, id, CaptureRootCallback::Cleanup, context)
        .map_err(CaptureBlock::committed)?;
    // Re-read current context inside the shared alternate helper. Never reuse
    // the entry Capture context after its nested child release and C690.
    crate::intro2_common_dying::publish_intro2_common_dying_alternate(manager, id, context.world_fx)
        .map_err(|_| CaptureBlock::new("capture outer death continuation").committed())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeCorpseRelationPrefix {
    Unattached,
    Retained,
    Release,
}

/// 12DA0's local existing-parent membership check. Missing-parent180F0 is a
/// distinct callback boundary. For class14 the cached DC50 style hook is null,
/// so an absent row needs only16750's fixed writes and preserves the task.
pub(crate) fn prepare_native_corpse_relation(
    manager: &EntityManager,
    id: u32,
) -> Result<NativeCorpseRelationPrefix, CaptureBlock> {
    let entity = actor(manager, id)?;
    let attached = match entity.collision.state_flags_at_0x08.masked(0x1000) {
        RetailRuntimeValue::Known(value) => value != 0,
        _ => return Err(CaptureBlock::new("corpse relation state")),
    };
    if !attached {
        return if entity.attached_to.is_none() {
            Ok(NativeCorpseRelationPrefix::Unattached)
        } else {
            Err(CaptureBlock::new("corpse backlink without relation bit"))
        };
    }
    let parent = entity
        .attached_to
        .ok_or(CaptureBlock::new("corpse parent handle"))?;
    let parent = manager
        .iter_all()
        .find(|entity| entity.id == parent)
        .ok_or(CaptureBlock::new("missing-parent180F0"))?;
    if parent
        .collision
        .state_flags_at_0x08
        .masked(REMOTE_OWNED_STATE_BIT)
        != RetailRuntimeValue::Known(0)
    {
        return Err(CaptureBlock::new("corpse remote parent"));
    }
    let RetailRuntimeValue::Known(Some(rows)) = &parent.sub_j_attachment_runtime else {
        return Err(CaptureBlock::new("corpse parent Sub-J"));
    };
    if rows.contains(id) {
        return Ok(NativeCorpseRelationPrefix::Retained);
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(CaptureBlock::new("corpse release context"));
    };
    if context.active_style().release_callback_policy()
        != crate::entity_behavior::ReleaseCallbackPolicy::None
        || entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT)
            != RetailRuntimeValue::Known(DYING_STATE_BIT)
    {
        return Err(CaptureBlock::new("corpse null release callback"));
    }
    Ok(NativeCorpseRelationPrefix::Release)
}

pub(crate) fn commit_native_corpse_relation(
    entity: &mut Entity,
    prefix: NativeCorpseRelationPrefix,
) {
    if prefix == NativeCorpseRelationPrefix::Release {
        entity.collision.state_flags_at_0x08 =
            crate::entity_relation_release::relation_release_state_word_after(
                entity.collision.state_flags_at_0x08,
                0x2f,
            );
        entity.collision.default_state_flags_at_0xc8 = RetailRuntimeValue::Known(0x2f);
        entity.attached_to = None;
    }
}

/// Local single-player 18640. Row retirement is callback-free; its capture
/// receipt survives with an empty-row marker until the owning root completes.
pub fn update_carried_pose(manager: &mut EntityManager, id: u32) -> Result<(), CaptureBlock> {
    let profile = NativeCaptorProfile::authenticate(manager, id)?;
    let metadata = manager
        .type_runtime_metadata(profile.entity_type())
        .ok_or(CaptureBlock::new("capture pose metadata"))?;
    let parent = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(CaptureBlock::new("capture pose allocation"))?;
    let RetailRuntimeValue::Known(Some(rows)) = &parent.sub_j_attachment_runtime else {
        return Err(CaptureBlock::new("Sub-J pose storage"));
    };
    let RetailRuntimeValue::Known(Some(descriptor)) = &metadata.sub_j_attachment_descriptor else {
        return Err(CaptureBlock::new("capture pose metadata"));
    };
    if rows.authored_slot_count() != 1
        || rows.capacity() != 1
        || descriptor.slots.len() != 1
        || rows.ordered_entity_ids().len() > 1
    {
        return Err(CaptureBlock::new("Sub-J pose shape"));
    }
    let relation = parent.native_capture_relation;
    let Some(&child_id) = rows.ordered_entity_ids().first() else {
        if relation.is_some_and(|relation| relation.row_present) {
            return Err(CaptureBlock::new("Sub-J row custody"));
        }
        return Ok(());
    };
    let mut relation = relation.ok_or(CaptureBlock::new("Sub-J row callback custody"))?;
    if relation.profile != profile
        || !relation.row_present
        || relation.child.entity_id != child_id
        || manager
            .main_base_abort_actor_observation(id)
            .map(|actor| actor.lease)
            != Some(relation.captor)
    {
        return Err(CaptureBlock::new("Sub-J pose allocation"));
    }
    let parent_position = parent.position_raw();
    let parent_basis = parent.physical_body_basis_q31();
    let child = manager.iter_all().find(|entity| entity.id == child_id);
    let stale = match child {
        None => true,
        Some(child) => {
            if manager
                .main_base_abort_actor_observation(child_id)
                .map(|actor| actor.lease)
                != Some(relation.child)
            {
                return Err(CaptureBlock::new("Sub-J child allocation"));
            }
            let state = child.collision.state_flags_at_0x08;
            match state.masked(crate::entity_collision_state::DYING_STATE_BIT) {
                RetailRuntimeValue::Known(bits) if bits != 0 => true,
                RetailRuntimeValue::Known(_) if state.known_value_bits() != 0 => false,
                RetailRuntimeValue::Known(_) if state.known_mask() == u32::MAX => true,
                _ => return Err(CaptureBlock::new("Sub-J child state")),
            }
        }
    };
    if stale {
        let parent = manager.entity_mut(id).unwrap();
        let RetailRuntimeValue::Known(Some(rows)) = &mut parent.sub_j_attachment_runtime else {
            unreachable!()
        };
        rows.commit_stable_compaction(Vec::new());
        relation.row_present = false;
        parent.native_capture_relation = Some(relation);
        return Ok(());
    }
    let child = child.unwrap();
    let offset_enabled = match child.collision.state_flags_at_0x08.masked(0x800) {
        RetailRuntimeValue::Known(bits) => bits != 0,
        _ => return Err(CaptureBlock::new("capture child offset state")),
    };
    let mut position = parent_position;
    if offset_enabled {
        let RetailRuntimeValue::Known(basis) = parent_basis else {
            return Err(CaptureBlock::new("Sub-J parent basis"));
        };
        let [x, y, z] = descriptor.slots[0].local_offset_raw.map(i32::from);
        for axis in 0..3 {
            let delta = crate::hover::q31_mul(basis.lateral[axis], x)
                .wrapping_add(crate::hover::q31_mul(basis.up[axis], y))
                .wrapping_add(crate::hover::q31_mul(basis.forward[axis], z));
            position[axis] = position[axis].wrapping_add(delta as i16);
        }
    }
    manager
        .entity_mut(child_id)
        .unwrap()
        .set_motion_raw(position, [0; 3]);
    Ok(())
}
