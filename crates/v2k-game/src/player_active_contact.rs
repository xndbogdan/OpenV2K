//! Production adapter for local player versus active-entity contact.
//!
//! The exact ordered solver lives in [`crate::active_pair`]. This module owns
//! the runtime bridge which snapshots the live list, resolves oriented model
//! collision, evaluates audited callback identities against the opposing
//! entity's live capability flags, and commits a successful solid pass
//! atomically.
//! It is intentionally local-player-only: remote ownership, unknown callback
//! state, unauthenticated descriptor-component effects, topology changes, and
//! death transitions without a native owner stop before mutation. A lethal
//! candidate packet to a native Type17 stages a commit-time dispatch through
//! the shared capture-path death owner; a lethal subject packet re-enters the
//! player's own checked-damage terminal instead.

use crate::active_pair::{
    resolve_active_pair_pass, ActivePairBody, ActivePairContact, ActivePairModel,
    ActivePairModelState, ActivePairOracle, ActivePairPass, ActivePairPassTermination,
    ActivePairUnresolved, PairBehaviorCallbackOutcome, PairBehaviorCallbackRequest,
    PairBehaviorCallbackResult, PairCallbackPhase, PairCallbackStateUpdate,
    PairCandidateDisposition, PairComponentCallbackOutcome, PairComponentCallbackRequest,
    PairComponentCallbackResult, PairDamageDelivery, PairDamageModifierOutcome,
    PairDamageModifierRequest, PairDamageModifierResult, PairNarrowPhaseOutcome,
    PairNarrowPhaseRequest, PairRemainingChainOutcome, PairTaggedEffectDispatchOutcome,
    PairTaggedEffectDispatchRequest, PairTaggedEffectDispatchResult, PairTypeHitCallbackOutcome,
    PairTypeHitCallbackRequest, PairTypeHitCallbackResult, PAIR_REMOTE_OWNED_STATE_BIT,
};
use crate::actor_task_owner::{ActorTaskId, ActorTaskSlot};
use crate::common_mover::target_prelude::{
    CommonMoverPreludeSubA, CommonMoverPreludeSubD, CommonMoverTargetPreludeTopology,
};
use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::common_mover::SubAPropulsionRuntime;
use crate::descriptor_contact::{
    plan_descriptor_contact, DescriptorContactBlock, DescriptorContactOutcome,
    DescriptorContactPlan, DescriptorContactRequest, DescriptorContactSourceSnapshot,
    DescriptorContactTargetSnapshot,
};
use crate::entity::{Entity, EntityManager};
use crate::entity_behavior::{BehaviorContextRuntime, PairContactCallbackPolicy};
use crate::entity_collision_state::{
    EntityPairCallbackRuntimeState, EntityTypeRuntimeMetadata, PairComponentContactPolicy,
    PairOrientationPolicy, RetailRuntimeValue, DYING_STATE_BIT, PAIR_COLLISION_ENABLED_STATE_BIT,
};
use crate::gameplay_notifications::GameplayNotifications;
use crate::oriented_model_contact::{oriented_model_pair_narrow_phase, OrientedModelPairProbe};
use crate::player::PlayerCraft;
use crate::player_hull::PlayerHull;
use crate::specialized_actor_task_production::SpecializedActorTaskScheduler;
use crate::type9_descriptor_contact_live::{
    Type9DescriptorContactCommitOutcome, Type9DescriptorContactOwner, Type9DescriptorContactPlan,
    Type9DescriptorContactPreparation,
};
use crate::wander_near_location::WanderNearPrivateState;
use crate::world_fx::WorldFx;
use v2k_formats::models::{AnimVars, CollisionModelPool};
use v2k_render::orientation_from_ypr;

const PLAYER_ENTITY_TYPE: u32 = 46;
const MAIN_BASE_ENTITY_TYPE: u32 = 6;
const FACTORY_ENTITY_TYPE: u32 = 66;
const HIVE_ENTITY_TYPE: u32 = 67;
const PLAYER_CONTACT_MODE_CAPABILITY: u32 = 1;
const CONSUMABLE_TARGET_CAPABILITIES: u32 = 0x0c00;
const LIFTER_TARGET_CAPABILITY: u32 = 0x0400;
const MAIN_BASE_TARGET_CAPABILITY: u32 = 0x0800;
const HIVE_IMPACT_CAPABILITY: u32 = 0x2000;
const PLAYER_FORCE_FEEDBACK_DISTANCE_RAW: i32 = 0x200;

/// Presentation which retail may submit from the normal player callback but
/// which has no equivalent on the port's no-haptics backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerActivePairUnsupportedPresentation {
    DistanceForceFeedback {
        player_id: u32,
        candidate_id: u32,
        distance_squared_raw: i32,
    },
}

/// Generic-hit audio staged inside the pure callback ordering and exposed
/// only after the complete solid-contact transaction commits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerActivePairSound {
    pub entity_id: u32,
    pub sound_id: u16,
    pub position_raw: [i16; 3],
}

/// Complete successful local-player pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerActivePairPass {
    pub core: ActivePairPass<()>,
    pub unsupported_presentation: Vec<PlayerActivePairUnsupportedPresentation>,
    pub sounds: Vec<PlayerActivePairSound>,
    /// Positive descriptor effects which committed with the same body pass.
    /// Lazy rear-half-space misses are complete no-ops and are not listed.
    pub descriptor_contacts: Vec<Type9DescriptorContactCommitOutcome>,
}

/// Explicit cutover boundary for the production adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerActivePairError {
    PlayerMissing,
    PlayerWrongType {
        entity_type: u32,
    },
    PlayerCollisionProfileMismatch,
    PlayerHullStateMismatch,
    Core(ActivePairUnresolved),
    TopologyChangedBeforeCommit,
    TaskCustodyUnavailable {
        entity_id: u32,
    },
    Type17DescriptorContact {
        entity_id: u32,
        entity_type: u32,
        spawn_index: Option<usize>,
    },
    /// A lethal player-pair packet reached a native Type17 candidate but its
    /// capture-path death dispatch failed (allocation, custody, or the 15040
    /// death itself). Lethal packets to any other family keep the previous
    /// fail-closed `Core(DeathDispatchRequired)` boundary.
    Type17PairDamage {
        entity_id: u32,
    },
    /// A rolling boulder's pair wake, custody or class18 split failed.
    RollingBoulderPair {
        entity_id: u32,
    },
}

#[derive(Debug, Clone)]
struct RuntimePairSnapshot {
    id: u32,
    entity_type: u32,
    spawn_index: Option<usize>,
    capability_flags: u32,
    behavior_context: RetailRuntimeValue<Option<BehaviorContextRuntime>>,
    pair_callbacks: EntityPairCallbackRuntimeState,
    model_to_world: RetailRuntimeValue<[[f32; 3]; 3]>,
    anim_vars: AnimVars,
    type9_descriptor_contact_owner: Option<Type9DescriptorContactOwner>,
    type17_descriptor_contact_owner: Option<Type17DescriptorContactOwner>,
}

impl RuntimePairSnapshot {
    fn from_entity(
        entity: &Entity,
        model_to_world: RetailRuntimeValue<[[f32; 3]; 3]>,
        anim_vars: AnimVars,
        type9_descriptor_contact_owner: Option<Type9DescriptorContactOwner>,
        type17_descriptor_contact_owner: Option<Type17DescriptorContactOwner>,
    ) -> Self {
        Self {
            id: entity.id,
            entity_type: entity.entity_type,
            spawn_index: entity.authored_spawn_index,
            capability_flags: entity.capability_flags,
            behavior_context: entity.current_behavior_context,
            pair_callbacks: entity.collision.pair_callbacks,
            model_to_world,
            anim_vars,
            type9_descriptor_contact_owner,
            type17_descriptor_contact_owner,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Type17DescriptorContactOwner {
    source: DescriptorContactSourceSnapshot,
    entity_id: u32,
    entity_type: u32,
    spawn_index: Option<usize>,
    sub_a_descriptor: v2k_formats::collision::SubAPropulsionDescriptor,
    sub_d: CommonMoverPreludeSubD,
    sub_a_runtime: SubAPropulsionRuntime,
    slots: [Option<(ActorTaskSlot, ActorTaskId, WanderNearPrivateState)>; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Type17DescriptorContactPlan {
    entity_id: u32,
    task_id: ActorTaskId,
    descriptor: DescriptorContactPlan,
}

impl Type17DescriptorContactOwner {
    fn authenticate(entity: &Entity, metadata: &EntityTypeRuntimeMetadata) -> Option<Self> {
        if entity.entity_type != 17 || entity.intro2_type17_runtime.is_none() {
            return None;
        }
        let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
            return None;
        };
        let RetailRuntimeValue::Known(Some(sub_a_runtime)) = entity.sub_a_propulsion_runtime else {
            return None;
        };
        let RetailRuntimeValue::Known(Some(sub_a_descriptor)) =
            metadata.sub_a_propulsion_descriptor
        else {
            return None;
        };
        let RetailRuntimeValue::Known(Some(sub_d_descriptor)) = metadata.sub_d_steering_descriptor
        else {
            return None;
        };
        let RetailRuntimeValue::Known(topology) = metadata.common_mover_topology else {
            return None;
        };
        if !topology.sub_a
            || !topology.sub_d
            || topology.sub_i
            || topology.sub_f
            || topology.sub_g
            || topology.sub_l
            || entity.type17_sub_d_runtime.is_none()
        {
            return None;
        }
        let [heading, _, roll] = entity.rotation_heading_pitch_roll_raw();
        let mut slots = [None; 3];
        for (index, slot) in ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .iter()
            .copied()
            .enumerate()
        {
            let Some(task_id) = entity.actor_tasks.task_in_slot(slot) else {
                continue;
            };
            let Some(task) = entity.actor_tasks.task_state(task_id) else {
                continue;
            };
            if crate::native_actor_descriptor_contact::task_has_null_contact(task) {
                continue;
            }
            if !crate::native_actor_descriptor_contact::task_has_descriptor_contact(task) {
                return None;
            }
            let private = crate::native_actor_descriptor_contact::private_state(task)?;
            slots[index] = Some((slot, task_id, private));
        }
        Some(Self {
            source: DescriptorContactSourceSnapshot {
                entity_id: entity.id,
                position_raw: entity.position_raw(),
                forward_q31: basis.forward,
                heading_raw: heading as u16,
                roll_raw: roll as u16,
            },
            entity_id: entity.id,
            entity_type: entity.entity_type,
            spawn_index: entity.authored_spawn_index,
            sub_a_descriptor,
            sub_d: CommonMoverPreludeSubD {
                steering_divisor_raw: sub_d_descriptor.steering_divisor_raw,
                couple_yaw_into_roll: sub_d_descriptor.couple_yaw_into_roll_raw != 0,
            },
            sub_a_runtime,
            slots,
        })
    }

    fn plan(
        self,
        target: DescriptorContactTargetSnapshot,
        next_random: &mut impl FnMut() -> u32,
    ) -> Result<Vec<Type17DescriptorContactPlan>, PlayerActivePairError> {
        let mut plans = Vec::new();
        let fail = || PlayerActivePairError::Type17DescriptorContact {
            entity_id: self.entity_id,
            entity_type: self.entity_type,
            spawn_index: self.spawn_index,
        };
        for (_slot, task_id, private) in self.slots.into_iter().flatten() {
            let probe = plan_descriptor_contact(
                DescriptorContactRequest {
                    source: RetailRuntimeValue::Known(self.source),
                    target: RetailRuntimeValue::Known(target),
                    private_state: RetailRuntimeValue::Unresolved,
                    topology: RetailRuntimeValue::Unresolved,
                },
                || unreachable!("02DA0 half-space consumes no RNG"),
            );
            match probe {
                Ok(DescriptorContactOutcome::Miss(_)) => continue,
                Err(DescriptorContactBlock::UnresolvedPrivateState) => {}
                _ => return Err(fail()),
            }
            let topology = CommonMoverTargetPreludeTopology {
                sub_a: Some(CommonMoverPreludeSubA {
                    descriptor: Some(self.sub_a_descriptor),
                    runtime: self.sub_a_runtime,
                }),
                sub_d: Some(self.sub_d),
                sub_i: false,
                sub_f: false,
                sub_g: false,
                sub_l: false,
            };
            let complete = plan_descriptor_contact(
                DescriptorContactRequest {
                    source: RetailRuntimeValue::Known(self.source),
                    target: RetailRuntimeValue::Known(target),
                    private_state: RetailRuntimeValue::Known(private),
                    topology: RetailRuntimeValue::Known(topology),
                },
                &mut *next_random,
            )
            .map_err(|_| fail())?;
            let DescriptorContactOutcome::Apply(descriptor) = complete else {
                continue;
            };
            plans.push(Type17DescriptorContactPlan {
                entity_id: self.entity_id,
                task_id,
                descriptor,
            });
        }
        Ok(plans)
    }
}

fn apply_type17_descriptor_plan(entity: &mut Entity, plan: Type17DescriptorContactPlan) {
    let Some(task) = entity.actor_tasks.task_state_mut(plan.task_id) else {
        return;
    };
    crate::native_actor_descriptor_contact::commit_private(
        task,
        plan.descriptor.effect.target_state,
    );
    entity.sub_a_propulsion_runtime =
        RetailRuntimeValue::Known(plan.descriptor.effect.sub_a_runtime);
    if let Some(write) = plan.descriptor.effect.sub_d_reversal_write {
        if let Some(runtime) = entity.type17_sub_d_runtime.as_mut() {
            runtime.last_yaw_step_raw = write.step_raw as i16;
        }
    }
    let pitch = entity.rotation_heading_pitch_roll_raw()[1];
    entity.set_rotation_heading_pitch_roll_raw([
        plan.descriptor.effect.heading_raw as i16,
        pitch,
        plan.descriptor.effect.roll_raw as i16,
    ]);
}

struct PlayerPairOracle<'a, P: CollisionModelPool + ?Sized> {
    model_pool: &'a P,
    runtime: Vec<RuntimePairSnapshot>,
    unsupported_presentation: Vec<PlayerActivePairUnsupportedPresentation>,
    sounds: Vec<PlayerActivePairSound>,
    descriptor_contact_plans: Vec<Type9DescriptorContactPlan>,
    type17_descriptor_plans: Vec<Type17DescriptorContactPlan>,
    type17_block: Option<PlayerActivePairError>,
    /// Resting boulders whose style1 +18 (40C730) ran during the pass.
    rolling_boulder_wakes: Vec<u32>,
    world_fx: &'a mut WorldFx,
}

impl<P: CollisionModelPool + ?Sized> PlayerPairOracle<'_, P> {
    fn runtime(&self, id: u32) -> Option<&RuntimePairSnapshot> {
        self.runtime.iter().find(|snapshot| snapshot.id == id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BehaviorDecision {
    Continue,
    Unresolved,
}

impl<P: CollisionModelPool + ?Sized> ActivePairOracle<()> for PlayerPairOracle<'_, P> {
    fn narrow_phase(&mut self, request: PairNarrowPhaseRequest<'_>) -> PairNarrowPhaseOutcome {
        let Some(subject_runtime) = self.runtime(request.subject.id) else {
            return PairNarrowPhaseOutcome::Unresolved(
                v2k_formats::models::ModelCollisionError::MissingChild {
                    global_id: request.subject_model_at_entry.global_id,
                },
            );
        };
        let Some(candidate_runtime) = self.runtime(request.candidate.id) else {
            return PairNarrowPhaseOutcome::Unresolved(
                v2k_formats::models::ModelCollisionError::MissingChild {
                    global_id: request.candidate_model.global_id,
                },
            );
        };
        oriented_model_pair_narrow_phase(
            self.model_pool,
            OrientedModelPairProbe {
                subject: request.subject,
                subject_model: request.subject_model_at_entry,
                subject_to_world: subject_runtime.model_to_world,
                subject_anim_vars: &subject_runtime.anim_vars,
                candidate: request.candidate,
                candidate_model: request.candidate_model,
                candidate_to_world: candidate_runtime.model_to_world,
                candidate_anim_vars: &candidate_runtime.anim_vars,
            },
        )
    }

    fn behavior_callback(
        &mut self,
        request: PairBehaviorCallbackRequest<'_>,
    ) -> PairBehaviorCallbackResult<()> {
        let Some(owner) = self.runtime(request.owner.id).cloned() else {
            return unresolved_behavior_callback();
        };
        let Some(opposite) = self.runtime(request.opposite.id).cloned() else {
            return unresolved_behavior_callback();
        };

        // Resting style1's +18 is 40C730: C6B0 reinstalls rolling style0 at
        // once, so 40EA10's 50000 is visible to the response and damage.
        if let RetailRuntimeValue::Known(Some(context)) = owner.behavior_context {
            if matches!(owner.entity_type, 3 | 27)
                && context.active_style().pair_contact_callback_policy()
                    == PairContactCallbackPolicy::UnknownAddress(0x0040_C730)
                && known_local(request.owner)
                && known_local(request.opposite)
            {
                let mut body = request.owner.clone();
                let RetailRuntimeValue::Known(state) =
                    body.collision.state_flags_at_0x08.masked(u32::MAX)
                else {
                    return unresolved_behavior_callback();
                };
                body.collision.state_flags_at_0x08.overwrite(
                    u32::MAX,
                    crate::rolling_boulder::apply_style_install_state(
                        state,
                        crate::rolling_boulder::RollingBoulderStyle::Rolling,
                    ),
                );
                self.rolling_boulder_wakes.push(owner.id);
                let mut state_update = PairCallbackStateUpdate::default();
                match request.side {
                    crate::active_pair::PairCallbackSide::Subject => {
                        state_update.subject = Some(body)
                    }
                    crate::active_pair::PairCallbackSide::Candidate => {
                        state_update.candidate = Some(body)
                    }
                }
                return PairBehaviorCallbackResult {
                    outcome: PairBehaviorCallbackOutcome::Null,
                    state_update,
                    remaining_chain: PairRemainingChainOutcome::Preserved,
                };
            }
        }
        // The local adapter does not own network transport or remote callback
        // state. Reject an actual authored hit before response or damage.
        let outcome = if !known_local(request.owner) || !known_local(request.opposite) {
            PairBehaviorCallbackOutcome::Unresolved
        } else {
            match evaluate_behavior_callback(
                &owner,
                &opposite,
                request.owner.position_raw,
                request.opposite.position_raw,
                &mut self.unsupported_presentation,
            ) {
                BehaviorDecision::Continue => PairBehaviorCallbackOutcome::Null,
                BehaviorDecision::Unresolved => PairBehaviorCallbackOutcome::Unresolved,
            }
        };

        PairBehaviorCallbackResult {
            outcome,
            state_update: PairCallbackStateUpdate::default(),
            remaining_chain: PairRemainingChainOutcome::Preserved,
        }
    }

    fn component_callback(
        &mut self,
        request: PairComponentCallbackRequest<'_>,
    ) -> PairComponentCallbackResult {
        let outcome = self.runtime(request.owner.id).cloned().map_or(
            PairComponentCallbackOutcome::Unresolved,
            |owner| {
                if component_callbacks_are_noop(owner.pair_callbacks) {
                    return PairComponentCallbackOutcome::Completed;
                }
                if let Some(type17_owner) = owner.type17_descriptor_contact_owner {
                    let target = DescriptorContactTargetSnapshot {
                        entity_id: request.opposite.id,
                        position_raw: request.opposite.position_raw,
                    };
                    match type17_owner.plan(target, &mut || {
                        u32::from(self.world_fx.next_shared_retail_random_u16())
                    }) {
                        Ok(plans) => {
                            self.type17_descriptor_plans.extend(plans);
                            return PairComponentCallbackOutcome::Completed;
                        }
                        Err(error) => {
                            self.type17_block = Some(error);
                            return PairComponentCallbackOutcome::Unresolved;
                        }
                    }
                }
                if owner.entity_type == 17 {
                    self.type17_block = Some(PlayerActivePairError::Type17DescriptorContact {
                        entity_id: owner.id,
                        entity_type: owner.entity_type,
                        spawn_index: owner.spawn_index,
                    });
                    return PairComponentCallbackOutcome::Unresolved;
                }
                let Some(descriptor_owner) = owner.type9_descriptor_contact_owner else {
                    return PairComponentCallbackOutcome::Unresolved;
                };
                if descriptor_owner.source_snapshot().position_raw != request.owner.position_raw {
                    return PairComponentCallbackOutcome::Unresolved;
                }
                match descriptor_owner.plan(DescriptorContactTargetSnapshot {
                    entity_id: request.opposite.id,
                    position_raw: request.opposite.position_raw,
                }) {
                    Ok(Type9DescriptorContactPreparation::Miss(_)) => {
                        PairComponentCallbackOutcome::Completed
                    }
                    Ok(Type9DescriptorContactPreparation::Apply(plan)) => {
                        if self
                            .descriptor_contact_plans
                            .iter()
                            .any(|current| current.entity_id() == plan.entity_id())
                        {
                            PairComponentCallbackOutcome::Unresolved
                        } else {
                            self.descriptor_contact_plans.push(plan);
                            PairComponentCallbackOutcome::Completed
                        }
                    }
                    Err(_) => PairComponentCallbackOutcome::Unresolved,
                }
            },
        );
        PairComponentCallbackResult {
            outcome,
            state_update: PairCallbackStateUpdate::default(),
            remaining_chain: PairRemainingChainOutcome::Preserved,
        }
    }

    fn dispatch_tagged_effect(
        &mut self,
        _request: PairTaggedEffectDispatchRequest<'_, ()>,
    ) -> PairTaggedEffectDispatchResult {
        // The production bridge currently admits only behavior callbacks
        // proven to return null. Tagged callbacks remain dedicated bounded
        // transactions until their dispatch actions are journalled here.
        PairTaggedEffectDispatchResult {
            outcome: PairTaggedEffectDispatchOutcome::Unresolved,
            state_update: PairCallbackStateUpdate::default(),
            remaining_chain: PairRemainingChainOutcome::Unresolved,
        }
    }

    fn damage_modifier(
        &mut self,
        request: PairDamageModifierRequest<'_>,
    ) -> PairDamageModifierResult {
        let outcome = damage_modifier_outcome(self.runtime(request.body.id));
        PairDamageModifierResult {
            outcome,
            remaining_chain: PairRemainingChainOutcome::Preserved,
        }
    }

    fn type_hit_callback(
        &mut self,
        request: PairTypeHitCallbackRequest<'_>,
    ) -> PairTypeHitCallbackResult {
        let outcome = self.runtime(request.body.id).cloned().map_or(
            PairTypeHitCallbackOutcome::Unresolved,
            |runtime| {
                match request.body.collision.generic_hit_sound_id {
                    RetailRuntimeValue::Known(Some(sound_id)) => {
                        self.sounds.push(PlayerActivePairSound {
                            entity_id: request.body.id,
                            sound_id,
                            position_raw: request.body.position_raw,
                        });
                    }
                    RetailRuntimeValue::Known(None) => {}
                    RetailRuntimeValue::Unresolved => {
                        return PairTypeHitCallbackOutcome::Unresolved;
                    }
                }
                match runtime.pair_callbacks.type_hit_callback_address {
                    RetailRuntimeValue::Known(None) => {
                        PairTypeHitCallbackOutcome::AbsentOrRepresentedWithoutBodyChange
                    }
                    RetailRuntimeValue::Known(Some(_)) | RetailRuntimeValue::Unresolved => {
                        PairTypeHitCallbackOutcome::Unresolved
                    }
                }
            },
        );
        PairTypeHitCallbackResult {
            outcome,
            remaining_chain: PairRemainingChainOutcome::Preserved,
        }
    }
}

fn damage_modifier_outcome(runtime: Option<&RuntimePairSnapshot>) -> PairDamageModifierOutcome {
    runtime.map_or(
        PairDamageModifierOutcome::Unresolved,
        |runtime| match runtime.pair_callbacks.damage_modifier_address {
            RetailRuntimeValue::Known(None) => PairDamageModifierOutcome::Identity,
            RetailRuntimeValue::Known(Some(0x0044_84A0))
                if runtime
                    .pair_callbacks
                    .damage_modifier_identity_context_empty
                    == RetailRuntimeValue::Known(true) =>
            {
                PairDamageModifierOutcome::Identity
            }
            RetailRuntimeValue::Known(Some(_)) | RetailRuntimeValue::Unresolved => {
                PairDamageModifierOutcome::Unresolved
            }
        },
    )
}

impl EntityManager {
    /// Run and atomically commit one local player active-pair pass.
    pub fn resolve_player_active_contacts<P: CollisionModelPool + ?Sized>(
        &mut self,
        model_pool: &P,
        frame: PlayerActivePairFrame<'_>,
    ) -> Result<PlayerActivePairPass, PlayerActivePairError> {
        resolve_player_active_contacts(self, model_pool, frame)
    }
}

/// Ordered collision geometry can use a detached model pool; native player
/// death separately requires the actual world resources and effect custody.
pub struct PlayerActivePairFrame<'a> {
    pub resources: &'a crate::resource_cache::ResourceCache,
    pub retail_tick: u32,
    pub player_craft: &'a PlayerCraft,
    pub player_hull: &'a mut PlayerHull,
    pub scheduler: &'a mut SpecializedActorTaskScheduler,
    pub world_fx: &'a mut WorldFx,
    pub notifications: &'a mut GameplayNotifications,
    pub extra_lives: u8,
}

pub fn resolve_player_active_contacts<P: CollisionModelPool + ?Sized>(
    entities: &mut EntityManager,
    model_pool: &P,
    frame: PlayerActivePairFrame<'_>,
) -> Result<PlayerActivePairPass, PlayerActivePairError> {
    let PlayerActivePairFrame {
        resources: cache,
        retail_tick,
        player_craft,
        player_hull,
        scheduler,
        world_fx,
        notifications,
        extra_lives,
    } = frame;
    let player = entities
        .player()
        .ok_or(PlayerActivePairError::PlayerMissing)?;
    if player.entity_type != PLAYER_ENTITY_TYPE {
        return Err(PlayerActivePairError::PlayerWrongType {
            entity_type: player.entity_type,
        });
    }
    let hull_profile = player_hull.profile();
    if player.mass_raw != hull_profile.mass_raw
        || player.capability_flags != 5
        || player.collision.active_model_slot() != RetailRuntimeValue::Known(0)
        || player.model_in_slot(0) != Some(41)
        || player.collision.damage_profile != RetailRuntimeValue::Known(hull_profile.damage)
        || player
            .collision
            .state_flags_at_0x08
            .masked(PAIR_COLLISION_ENABLED_STATE_BIT)
            != RetailRuntimeValue::Known(PAIR_COLLISION_ENABLED_STATE_BIT)
    {
        return Err(PlayerActivePairError::PlayerCollisionProfileMismatch);
    }
    if player_hull.dying
        || player.collision.health_raw != RetailRuntimeValue::Known(player_hull.health_raw)
        || player.collision.pre_health_damage_buffer_raw
            != RetailRuntimeValue::Known(player_hull.pre_health_damage_buffer_raw)
    {
        return Err(PlayerActivePairError::PlayerHullStateMismatch);
    }

    let player_id = player.id;
    let expected_live_order = entities.retail_live_order_ids().collect::<Vec<_>>();
    let subject = active_pair_body_from_entity(player, model_pool);
    let original_subject = subject.clone();
    let mut player_runtime = RuntimePairSnapshot::from_entity(
        player,
        RetailRuntimeValue::Known(player_craft.model_orientation(player.heading)),
        player_craft.anim_vars(),
        None,
        None,
    );
    player_runtime
        .pair_callbacks
        .damage_modifier_identity_context_empty =
        entities.player_pair_damage_modifier_context_empty();
    let mut runtime = vec![player_runtime];
    let mut candidates = Vec::new();
    for candidate in entities
        .iter_collidable()
        .filter(|candidate| candidate.id != player_id)
        // The proven PowerUp behavior owns a separate bounded inventory
        // transaction. Level-1 authors those allocations after every audited
        // solid (indices 32..=34), so this pass retains the captured ordering
        // of all solid candidates. The inventory suffix is a separate bounded
        // transaction and runs only after this pass succeeds.
        .filter(|candidate| !uses_dedicated_power_up_transaction(candidate))
        // The native weapon host owns both pair directions, including the
        // player-first direction. Retain that routing custody after a terminal
        // or a parked committed prefix: a changed wrapper must not retry the
        // same allocation through this detached planning pass.
        .filter(|candidate| !native_weapon_owns_pair_lane(entities, candidate))
        // Unresolved pair identity used to abort the whole pass, which made
        // every non-Level-1 world skip solids including Main Base. This
        // adapter only visits candidates whose constructor/census pair fields
        // are closed. Retail still walks everyone; unknown actors stay out.
        .filter(|candidate| {
            candidate_pair_identity_is_closed(candidate)
                || type17_player_pair_admitted(candidate)
                || rolling_boulder_player_pair_admitted(candidate)
        })
    {
        candidates.push(active_pair_body_from_entity(candidate, model_pool));
        let metadata = entities.type_runtime_metadata(candidate.entity_type);
        let descriptor_contact_owner = metadata.and_then(|metadata| {
            Type9DescriptorContactOwner::authenticate(candidate, metadata).ok()
        });
        let type17_descriptor_contact_owner = metadata
            .and_then(|metadata| Type17DescriptorContactOwner::authenticate(candidate, metadata));
        runtime.push(RuntimePairSnapshot::from_entity(
            candidate,
            entity_pair_to_world(candidate),
            candidate.presentation_anim_vars(retail_tick),
            descriptor_contact_owner,
            type17_descriptor_contact_owner,
        ));
    }
    let original_candidates = candidates.clone();

    let mut oracle = PlayerPairOracle {
        model_pool,
        runtime,
        unsupported_presentation: Vec::new(),
        sounds: Vec::new(),
        descriptor_contact_plans: Vec::new(),
        type17_descriptor_plans: Vec::new(),
        type17_block: None,
        rolling_boulder_wakes: Vec::new(),
        world_fx,
    };
    let core = match resolve_active_pair_pass(subject, candidates, &mut oracle) {
        Ok(core) => core,
        Err(error) => {
            return Err(oracle
                .type17_block
                .take()
                .unwrap_or(PlayerActivePairError::Core(error)));
        }
    };

    let (health_raw, pre_health_damage_buffer_raw) = match (
        core.subject.collision.health_raw,
        core.subject.collision.pre_health_damage_buffer_raw,
    ) {
        (RetailRuntimeValue::Known(health), RetailRuntimeValue::Known(buffer)) if health > 0 => {
            (health, buffer)
        }
        _ => return Err(PlayerActivePairError::PlayerHullStateMismatch),
    };

    for plan in oracle.type17_descriptor_plans {
        let Some(entity) = entities.entity_mut(plan.entity_id) else {
            return Err(PlayerActivePairError::Type17DescriptorContact {
                entity_id: plan.entity_id,
                entity_type: 17,
                spawn_index: None,
            });
        };
        apply_type17_descriptor_plan(entity, plan);
    }
    // Lethal packets staged by the planner are delivered through native
    // death owners inside commit, in walk order: the subject direction
    // first, then candidates in disposition order. Anything the commit
    // cannot own stays fail-closed there.
    let mut subject_death_dispatch = None;
    let mut candidate_death_dispatches = Vec::new();
    for disposition in &core.dispositions {
        if let PairCandidateDisposition::Resolved {
            candidate_id,
            contact,
            ..
        } = disposition
        {
            if let PairDamageDelivery::SubjectDeathDispatch {
                capped_pair_damage_raw,
                transition,
            } = contact.subject_damage
            {
                // Retail processes every pair directionally; the first lethal
                // subject packet is the killing blow. Later pairs plan against
                // the staged pre-damage copy, so only the first is dispatched.
                if subject_death_dispatch.is_none() {
                    subject_death_dispatch =
                        Some((*candidate_id, capped_pair_damage_raw, transition));
                }
            }
            if let PairDamageDelivery::CandidateDeathDispatch {
                capped_pair_damage_raw,
                transition,
            } = contact.candidate_damage
            {
                candidate_death_dispatches.push((
                    *candidate_id,
                    capped_pair_damage_raw,
                    transition,
                ));
            }
        }
    }
    let descriptor_contacts = entities.commit_player_active_pair_bodies(
        cache,
        &expected_live_order,
        &original_subject,
        &original_candidates,
        &core.subject,
        &core.candidates,
        &oracle.descriptor_contact_plans,
        scheduler,
        oracle.world_fx,
        notifications,
        retail_tick,
        extra_lives,
        player_hull,
        subject_death_dispatch,
        &candidate_death_dispatches,
        &oracle.rolling_boulder_wakes,
    )?;
    // A woken boulder that survived the pass keeps its replaced rolling owner.
    for &id in &oracle.rolling_boulder_wakes {
        if let Ok(owner) = crate::rolling_boulder::RollingBoulderOwner::adopt(entities, id) {
            scheduler.register_rolling_boulder(owner);
        }
    }
    if subject_death_dispatch.is_some() {
        // The hull terminal owns the player death state; the planned
        // pre-damage values must not resurrect it. Verify sync only.
        let synced = entities.player().is_some_and(|player| {
            player.collision.health_raw == RetailRuntimeValue::Known(player_hull.health_raw)
                && player.collision.pre_health_damage_buffer_raw
                    == RetailRuntimeValue::Known(player_hull.pre_health_damage_buffer_raw)
                && player.collision.state_flags_at_0x08.masked(DYING_STATE_BIT)
                    == RetailRuntimeValue::Known(DYING_STATE_BIT)
                && player_hull.dying
                && player_hull.health_raw == 0
        });
        if !synced {
            return Err(PlayerActivePairError::PlayerHullStateMismatch);
        }
    } else {
        player_hull.health_raw = health_raw;
        player_hull.pre_health_damage_buffer_raw = pre_health_damage_buffer_raw;
    }

    Ok(PlayerActivePairPass {
        core,
        unsupported_presentation: oracle.unsupported_presentation,
        sounds: oracle.sounds,
        descriptor_contacts,
    })
}

fn uses_dedicated_power_up_transaction(entity: &Entity) -> bool {
    matches!(
        entity.current_behavior_context,
        RetailRuntimeValue::Known(Some(context))
            if context.active_style().pair_contact_callback_policy()
                == PairContactCallbackPolicy::PowerUp
    )
}

fn native_weapon_owns_pair_lane(manager: &EntityManager, entity: &Entity) -> bool {
    crate::native_entity_weapons::entity_authenticates(entity)
        && crate::class49_death::allocation_authenticates(manager, entity.id)
}

/// Resolve the live entity basis shared by authored pair and projectile/model
/// probes. Missing pitch/roll policy is not equivalent to a yaw-only basis.
/// Pair-probe to-world matrix.
///
/// Authored pair census owners use [`candidate_pair_orientation`]. Dynamically
/// spawned type-8 scientists do not have that census policy; they already own
/// the D720/13F70 Q31 basis, which is the same retail entity matrix the pair
/// probe reads. Type-17/47 Targetter lock and the selected-entity ring use
/// this same matrix. Unresolved both ways stays fail-closed.
pub fn entity_pair_to_world(entity: &Entity) -> RetailRuntimeValue<[[f32; 3]; 3]> {
    match candidate_pair_orientation(entity) {
        RetailRuntimeValue::Known(orientation) => RetailRuntimeValue::Known(orientation),
        RetailRuntimeValue::Unresolved => match entity.physical_body_basis_q31() {
            RetailRuntimeValue::Known(basis) => {
                RetailRuntimeValue::Known(Type9BodyBasis::orientation_world_from_model(basis))
            }
            RetailRuntimeValue::Unresolved => RetailRuntimeValue::Unresolved,
        },
    }
}

pub fn candidate_pair_orientation(entity: &Entity) -> RetailRuntimeValue<[[f32; 3]; 3]> {
    match entity.collision.pair_callbacks.orientation_policy {
        RetailRuntimeValue::Known(PairOrientationPolicy::LiveHeadingWithAuthoredPitchRoll {
            pitch_raw,
            roll_raw,
        }) => RetailRuntimeValue::Known(orientation_from_ypr(
            std::f32::consts::FRAC_PI_2 - entity.heading,
            angle_word_to_radians(pitch_raw),
            angle_word_to_radians(roll_raw),
        )),
        RetailRuntimeValue::Unresolved => RetailRuntimeValue::Unresolved,
    }
}

fn angle_word_to_radians(angle: u16) -> f32 {
    angle as f32 * std::f32::consts::TAU / 65_536.0
}

/// Snapshot the collision-relevant live entity state and selected model used
/// by every bounded active-pair runtime adapter.
///
/// Keeping model-slot resolution here prevents conversion-specific bridges
/// from acquiring a subtly different missing-model or selector policy.
pub(crate) fn active_pair_body_from_entity<P: CollisionModelPool + ?Sized>(
    entity: &Entity,
    model_pool: &P,
) -> ActivePairBody {
    let active_slot = match entity.collision.active_model_slot() {
        RetailRuntimeValue::Known(slot) => slot,
        RetailRuntimeValue::Unresolved => 0,
    };
    // Entity construction encodes a raw model-slot word0 as None. Retail
    // 11AD0/12530 still index the actual Section-8 pool at0: PRELOAD's
    // `nothing` model has collision radius0 and is rejected by the normal
    // subject/candidate radius gates. Resolve that record rather than treating
    // the host encoding as missing geometry or inventing an unconditional skip.
    let global_id = entity.model_in_slot(active_slot).unwrap_or(0);
    let active_model = match model_pool
        .collision_model(global_id)
        .map(|model| (global_id, model.collision_radius_raw))
    {
        Some((global_id, collision_radius_raw)) => {
            ActivePairModelState::Resolved(ActivePairModel {
                active_slot,
                global_id,
                collision_radius_raw,
            })
        }
        None => ActivePairModelState::Missing {
            active_slot,
            global_id: Some(global_id),
        },
    };
    ActivePairBody {
        id: entity.id,
        position_raw: entity.position_raw(),
        velocity_raw: entity.velocity_raw(),
        mass_raw: entity.mass_raw,
        active_model,
        collision: entity.collision.clone(),
    }
}

/// Classify one subject/candidate pair with the shared oriented Section-8 probe.
///
/// A behavior-callback unresolved result on the subject after the probe stored
/// a contact is the bounded adapters' "this pair touched" signal. Any other
/// active-pair error stays fail-closed.
pub(crate) struct OrientedActivePairContactRequest<'a> {
    pub subject: &'a Entity,
    pub candidate: &'a Entity,
    /// 11AD0 retains entry eligibility, domain and model across callbacks;
    /// current pose, recent relations and candidate state remain live.
    pub subject_entry: &'a ActivePairBody,
    pub retail_tick: u32,
}

pub(crate) fn classify_oriented_active_pair_contact<P: CollisionModelPool + ?Sized>(
    request: OrientedActivePairContactRequest<'_>,
    model_pool: &P,
) -> Result<Option<ActivePairContact>, ActivePairUnresolved> {
    let OrientedActivePairContactRequest {
        subject,
        candidate,
        subject_entry,
        retail_tick,
    } = request;
    let mut subject_body = active_pair_body_from_entity(subject, model_pool);
    subject_body.active_model = subject_entry.active_model;
    subject_body.collision.state_flags_at_0x08 = subject_entry.collision.state_flags_at_0x08;
    subject_body.collision.subject_scan_gate_at_0x70 =
        subject_entry.collision.subject_scan_gate_at_0x70;
    let candidate_body = active_pair_body_from_entity(candidate, model_pool);
    let mut oracle = OrientedPairContactClassifier {
        model_pool,
        subject_to_world: entity_pair_to_world(subject),
        subject_anim_vars: subject.presentation_anim_vars(retail_tick),
        candidate_to_world: entity_pair_to_world(candidate),
        candidate_anim_vars: candidate.presentation_anim_vars(retail_tick),
        contact: None,
    };
    match resolve_active_pair_pass(subject_body, vec![candidate_body], &mut oracle) {
        Ok(pass) => {
            debug_assert!(matches!(
                pass.termination,
                ActivePairPassTermination::IntrusiveChainExhausted
                    | ActivePairPassTermination::SubjectIneligible
                    | ActivePairPassTermination::SubjectScanGateClosed
                    | ActivePairPassTermination::SubjectZeroCollisionRadius
            ));
            Ok(None)
        }
        Err(ActivePairUnresolved::Callback {
            entity_id,
            phase: PairCallbackPhase::Behavior,
        }) if entity_id == subject.id && oracle.contact.is_some() => Ok(oracle.contact),
        Err(source) => Err(source),
    }
}

struct OrientedPairContactClassifier<'a, P: CollisionModelPool + ?Sized> {
    model_pool: &'a P,
    subject_to_world: RetailRuntimeValue<[[f32; 3]; 3]>,
    subject_anim_vars: AnimVars,
    candidate_to_world: RetailRuntimeValue<[[f32; 3]; 3]>,
    candidate_anim_vars: AnimVars,
    contact: Option<ActivePairContact>,
}

impl<P: CollisionModelPool + ?Sized> ActivePairOracle<()> for OrientedPairContactClassifier<'_, P> {
    fn narrow_phase(&mut self, request: PairNarrowPhaseRequest<'_>) -> PairNarrowPhaseOutcome {
        let outcome = oriented_model_pair_narrow_phase(
            self.model_pool,
            OrientedModelPairProbe {
                subject: request.subject,
                subject_model: request.subject_model_at_entry,
                subject_to_world: self.subject_to_world,
                subject_anim_vars: &self.subject_anim_vars,
                candidate: request.candidate,
                candidate_model: request.candidate_model,
                candidate_to_world: self.candidate_to_world,
                candidate_anim_vars: &self.candidate_anim_vars,
            },
        );
        if let PairNarrowPhaseOutcome::Contact(contact) = outcome {
            self.contact = Some(contact);
        }
        outcome
    }

    fn behavior_callback(
        &mut self,
        _request: PairBehaviorCallbackRequest<'_>,
    ) -> PairBehaviorCallbackResult<()> {
        PairBehaviorCallbackResult {
            outcome: PairBehaviorCallbackOutcome::Unresolved,
            state_update: Default::default(),
            remaining_chain: PairRemainingChainOutcome::Preserved,
        }
    }

    fn dispatch_tagged_effect(
        &mut self,
        _request: PairTaggedEffectDispatchRequest<'_, ()>,
    ) -> PairTaggedEffectDispatchResult {
        PairTaggedEffectDispatchResult {
            outcome: PairTaggedEffectDispatchOutcome::Unresolved,
            state_update: Default::default(),
            remaining_chain: PairRemainingChainOutcome::Unresolved,
        }
    }

    fn component_callback(
        &mut self,
        _request: PairComponentCallbackRequest<'_>,
    ) -> PairComponentCallbackResult {
        PairComponentCallbackResult {
            outcome: PairComponentCallbackOutcome::Unresolved,
            state_update: Default::default(),
            remaining_chain: PairRemainingChainOutcome::Unresolved,
        }
    }

    fn damage_modifier(
        &mut self,
        _request: PairDamageModifierRequest<'_>,
    ) -> PairDamageModifierResult {
        PairDamageModifierResult {
            outcome: PairDamageModifierOutcome::Unresolved,
            remaining_chain: PairRemainingChainOutcome::Unresolved,
        }
    }

    fn type_hit_callback(
        &mut self,
        _request: PairTypeHitCallbackRequest<'_>,
    ) -> PairTypeHitCallbackResult {
        PairTypeHitCallbackResult {
            outcome: PairTypeHitCallbackOutcome::Unresolved,
            remaining_chain: PairRemainingChainOutcome::Unresolved,
        }
    }
}

fn known_local(body: &ActivePairBody) -> bool {
    body.collision
        .state_flags_at_0x08
        .masked(PAIR_REMOTE_OWNED_STATE_BIT)
        == RetailRuntimeValue::Known(0)
}

fn unresolved_behavior_callback() -> PairBehaviorCallbackResult<()> {
    PairBehaviorCallbackResult {
        outcome: PairBehaviorCallbackOutcome::Unresolved,
        state_update: PairCallbackStateUpdate::default(),
        remaining_chain: PairRemainingChainOutcome::Unresolved,
    }
}

fn constructor_pair_policy_for_unresolved_style(
    entity_type: u32,
) -> Option<PairContactCallbackPolicy> {
    match entity_type {
        PLAYER_ENTITY_TYPE => Some(PairContactCallbackPolicy::PlayerContact),
        MAIN_BASE_ENTITY_TYPE => Some(PairContactCallbackPolicy::MainBaseConversion),
        FACTORY_ENTITY_TYPE => Some(PairContactCallbackPolicy::LifterDelivery),
        // Class 46 `FUN_004259F0` is an exact no-op against player capability
        // set 5. Generic construction leaves that style unpublished, so the
        // type-67 constructor identity must still admit the player-solid pass.
        HIVE_ENTITY_TYPE => Some(PairContactCallbackPolicy::Hive),
        _ => None,
    }
}

/// Native class20 boulders pair through their D720/13F70 Q31 body basis: the
/// rolling task turns that basis, so no authored pitch/roll policy applies.
/// Their component, modifier and type-hit slots are the audited null words.
fn rolling_boulder_player_pair_admitted(entity: &Entity) -> bool {
    crate::rolling_boulder::rolling_boulder_allocation_authenticates(entity)
        && entity.collision.pair_callbacks
            == crate::entity_collision_state::EntityPairCallbackRuntimeState::audited_local(
                None,
                RetailRuntimeValue::Unresolved,
            )
        && matches!(entity_pair_to_world(entity), RetailRuntimeValue::Known(_))
}

fn type17_player_pair_admitted(entity: &Entity) -> bool {
    entity.entity_type == 17
        && entity.intro2_type17_runtime.is_some()
        && matches!(entity_pair_to_world(entity), RetailRuntimeValue::Known(_))
}

fn candidate_pair_identity_is_closed(entity: &Entity) -> bool {
    let callbacks = entity.collision.pair_callbacks;
    matches!(
        (
            callbacks.component_contact,
            callbacks.damage_modifier_address,
            callbacks.type_hit_callback_address,
            callbacks.orientation_policy,
        ),
        (
            RetailRuntimeValue::Known(_),
            RetailRuntimeValue::Known(_),
            RetailRuntimeValue::Known(_),
            RetailRuntimeValue::Known(_),
        )
    )
}

fn component_callbacks_are_noop(callbacks: EntityPairCallbackRuntimeState) -> bool {
    matches!(
        callbacks.component_contact,
        RetailRuntimeValue::Known(callbacks)
            if callbacks
                .into_iter()
                .all(|callback| callback == PairComponentContactPolicy::None)
    )
}

fn evaluate_behavior_callback(
    owner: &RuntimePairSnapshot,
    opposite: &RuntimePairSnapshot,
    owner_position_raw: [i16; 3],
    opposite_position_raw: [i16; 3],
    unsupported_presentation: &mut Vec<PlayerActivePairUnsupportedPresentation>,
) -> BehaviorDecision {
    let policy = match owner.behavior_context {
        RetailRuntimeValue::Known(None) => PairContactCallbackPolicy::None,
        RetailRuntimeValue::Known(Some(context)) => {
            context.active_style().pair_contact_callback_policy()
        }
        RetailRuntimeValue::Unresolved => {
            match constructor_pair_policy_for_unresolved_style(owner.entity_type) {
                Some(policy) => policy,
                None => return BehaviorDecision::Unresolved,
            }
        }
    };
    match policy {
        PairContactCallbackPolicy::None => BehaviorDecision::Continue,
        PairContactCallbackPolicy::PlayerContact => {
            if owner.capability_flags & PLAYER_CONTACT_MODE_CAPABILITY == 0 {
                return if opposite.capability_flags & CONSUMABLE_TARGET_CAPABILITIES == 0 {
                    BehaviorDecision::Continue
                } else {
                    BehaviorDecision::Unresolved
                };
            }
            if opposite.capability_flags & CONSUMABLE_TARGET_CAPABILITIES == 0 {
                let distance_squared_raw =
                    squared_wrapping_distance_raw(owner_position_raw, opposite_position_raw);
                if distance_squared_raw
                    > PLAYER_FORCE_FEEDBACK_DISTANCE_RAW * PLAYER_FORCE_FEEDBACK_DISTANCE_RAW
                {
                    unsupported_presentation.push(
                        PlayerActivePairUnsupportedPresentation::DistanceForceFeedback {
                            player_id: owner.id,
                            candidate_id: opposite.id,
                            distance_squared_raw,
                        },
                    );
                }
            }
            BehaviorDecision::Continue
        }
        PairContactCallbackPolicy::CapturePeople => {
            if opposite.capability_flags & CONSUMABLE_TARGET_CAPABILITIES == 0 {
                BehaviorDecision::Continue
            } else {
                BehaviorDecision::Unresolved
            }
        }
        PairContactCallbackPolicy::LifterDelivery => {
            if opposite.capability_flags & LIFTER_TARGET_CAPABILITY == 0 {
                BehaviorDecision::Continue
            } else {
                BehaviorDecision::Unresolved
            }
        }
        PairContactCallbackPolicy::MainBaseConversion => {
            if opposite.capability_flags & MAIN_BASE_TARGET_CAPABILITY == 0 {
                BehaviorDecision::Continue
            } else {
                BehaviorDecision::Unresolved
            }
        }
        PairContactCallbackPolicy::Hive => {
            if opposite.capability_flags & (CONSUMABLE_TARGET_CAPABILITIES | HIVE_IMPACT_CAPABILITY)
                == 0
            {
                BehaviorDecision::Continue
            } else {
                BehaviorDecision::Unresolved
            }
        }
        // Production filters this proven callback class into the dedicated
        // inventory transaction. Reaching it here means the snapshot and
        // dispatch policy diverged, so the solid pass must fail closed.
        PairContactCallbackPolicy::PowerUp => BehaviorDecision::Unresolved,
        PairContactCallbackPolicy::UnknownAddress(_) => BehaviorDecision::Unresolved,
    }
}

fn squared_wrapping_distance_raw(first: [i16; 3], second: [i16; 3]) -> i32 {
    (0..3).fold(0_i32, |sum, axis| {
        let delta = i32::from(first[axis].wrapping_sub(second[axis]));
        sum.wrapping_add(delta.wrapping_mul(delta))
    })
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::actor_task_dispatcher::ActorTaskRuntime;
    use crate::actor_task_owner::{ActorTaskSlot, ActorTaskVisit, ActorTaskWrapperFlags};
    use crate::entity_behavior::{behavior_program, BehaviorSelection};
    use crate::entity_collision_state::{
        EntityPairCallbackRuntimeState, EntityTypeRuntimeMetadata,
    };
    use crate::ordinary_type9_initial_production::FreshLevel1Type9InitialProductionBranch;
    use crate::ordinary_type9_wander_initializer::OrdinaryType9WanderInitializerOutcome;
    use crate::session::GameSession;
    use crate::world_fx::WorldFx;

    #[v2k_test_support::retail_test]
    fn native_weapon_allocation_keeps_pair_routing_after_its_wrapper_changes() {
        use crate::native_entity_weapons::{EntityWeaponConstructionRequest, EntityWeaponKind};
        let (session, mut manager, mut fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(17);
        let player_id = manager.player().unwrap().id;
        let owner = manager
            .construct_entity_weapon(
                EntityWeaponConstructionRequest {
                    kind: EntityWeaponKind::Grenade,
                    source_actor_id: player_id,
                    position_raw: manager.player().unwrap().position_raw(),
                    velocity_raw: [0; 3],
                    rotation_raw: [0; 3],
                },
                &session.cache,
                &mut fx,
                0,
            )
            .unwrap();
        for id in manager.retail_live_order_ids().collect::<Vec<_>>() {
            if id != player_id && id != owner.entity_id() {
                manager
                    .entity_mut(id)
                    .unwrap()
                    .collision
                    .state_flags_at_0x08
                    .overwrite(u32::MAX, 0);
            }
        }
        let mut hull = PlayerHull::default();
        manager.sync_player_hull_collision_state(&hull);
        let craft = PlayerCraft::new();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.register_native_weapon(&manager, owner).unwrap();
        let mut notifications = GameplayNotifications::new();
        for clear_primary in [false, true] {
            if clear_primary {
                manager
                    .entity_mut(owner.entity_id())
                    .unwrap()
                    .actor_tasks
                    .clear_slot(ActorTaskSlot::Primary);
                assert_eq!(
                    scheduler.native_weapon_owner(&manager, owner.entity_id()),
                    None
                );
            }
            assert!(native_weapon_owns_pair_lane(
                &manager,
                manager
                    .iter_all()
                    .find(|e| e.id == owner.entity_id())
                    .unwrap(),
            ));
            let rng_before = *fx.entity_construction_state().0;
            let pass = resolve_player_active_contacts(
                &mut manager,
                &session.cache,
                PlayerActivePairFrame {
                    resources: &session.cache,
                    retail_tick: 0,
                    player_craft: &craft,
                    player_hull: &mut hull,
                    scheduler: &mut scheduler,
                    world_fx: &mut fx,
                    notifications: &mut notifications,
                    extra_lives: 3,
                },
            )
            .unwrap();
            assert!(pass
                .core
                .candidates
                .iter()
                .all(|candidate| candidate.id != owner.entity_id()));
            assert_eq!(*fx.entity_construction_state().0, rng_before);
        }
        // A type/model coincidence cannot borrow the constructor's route.
        let mut unowned =
            Entity::unresolved_port_entity(u32::MAX, crate::entity::EntityKind::from_type(59), 59);
        unowned.model_slots = [Some(128); 4];
        assert!(!native_weapon_owns_pair_lane(&manager, &unowned));
    }

    #[v2k_test_support::retail_test]
    fn authored_world46_zero_model_skips_both_pair_directions_before_callbacks() {
        let (session, manager, _) = crate::native_type122::construction_tests::native_fixture(46);
        let empty_model = session.cache.global_model(0).unwrap();
        assert_eq!(empty_model.name.as_deref(), Some("nothing"));
        assert_eq!(empty_model.collision_radius_raw, 0);
        let captor = manager
            .iter_all()
            .find(|entity| entity.entity_type == 122)
            .unwrap();
        let model_less: Vec<_> = manager
            .iter_all()
            .filter(|entity| entity.entity_type == 35)
            .collect();
        assert_eq!(model_less.len(), 4);
        for empty in model_less {
            assert_eq!(empty.model_slots, [None; 4]);
            assert_eq!(
                empty.current_behavior_context,
                RetailRuntimeValue::Unresolved
            );
            assert_eq!(
                active_pair_body_from_entity(empty, &session.cache).active_model,
                ActivePairModelState::Resolved(ActivePairModel {
                    active_slot: 0,
                    global_id: 0,
                    collision_radius_raw: 0,
                })
            );
            for (subject, candidate) in [(empty, captor), (captor, empty)] {
                let entry = active_pair_body_from_entity(subject, &session.cache);
                assert_eq!(
                    classify_oriented_active_pair_contact(
                        OrientedActivePairContactRequest {
                            subject,
                            candidate,
                            subject_entry: &entry,
                            retail_tick: 4793,
                        },
                        &session.cache,
                    ),
                    Ok(None)
                );
            }
        }
    }

    #[v2k_test_support::retail_test]
    fn pair_model_zero_is_pool_backed_and_missing_nonzero_still_blocks() {
        let (session, mut manager, _) =
            crate::native_type122::construction_tests::native_fixture(46);
        let empty = manager
            .iter_all()
            .find(|entity| entity.entity_type == 35)
            .unwrap();
        // The zero word is a real resource lookup, not a type/None whitelist.
        assert_eq!(
            active_pair_body_from_entity(empty, &()).active_model,
            ActivePairModelState::Missing {
                active_slot: 0,
                global_id: Some(0),
            }
        );
        let absent_id = empty.id;
        manager.entity_mut(absent_id).unwrap().model_slots = [Some(usize::MAX); 4];
        let absent = manager
            .iter_all()
            .find(|entity| entity.id == absent_id)
            .unwrap();
        let captor = manager
            .iter_all()
            .find(|entity| entity.entity_type == 122)
            .unwrap();
        for (subject, candidate) in [(absent, captor), (captor, absent)] {
            let entry = active_pair_body_from_entity(subject, &session.cache);
            assert_eq!(
                classify_oriented_active_pair_contact(
                    OrientedActivePairContactRequest {
                        subject,
                        candidate,
                        subject_entry: &entry,
                        retail_tick: 4793,
                    },
                    &session.cache,
                ),
                Err(ActivePairUnresolved::MissingModel {
                    entity_id: absent.id,
                    active_slot: 0,
                    global_id: Some(usize::MAX),
                })
            );
        }
    }

    fn fresh_level_one() -> (GameSession, EntityManager) {
        let data_dir = v2k_test_support::retail_dir();
        let mut session = GameSession::init(&data_dir).expect("retail PRELOAD");
        session
            .load_auxiliary_ovl(3, 1)
            .expect("normal-tier system overlay");
        let type_metadata = session
            .cache
            .global_entity_model_table()
            .iter()
            .copied()
            .enumerate()
            .map(|(entity_type, model_slots)| {
                session
                    .cache
                    .global_entity_type(entity_type)
                    .map(EntityTypeRuntimeMetadata::from_section12)
                    .unwrap_or(EntityTypeRuntimeMetadata {
                        model_slots,
                        ..EntityTypeRuntimeMetadata::default()
                    })
            })
            .collect::<Vec<_>>();
        session
            .load_level_by_id(13, 1)
            .expect("normal-tier world 13");
        let mut world_fx = WorldFx::new();
        let manager = EntityManager::from_fresh_new_game_level_with_type_metadata(
            session.cache.level_desc().expect("world 13 descriptor"),
            &type_metadata,
            Some(session.cache.terrain().expect("world 13 terrain")),
            0,
            &mut world_fx,
        )
        .expect("fresh type-17 birth publication");
        (session, manager)
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct PublishedWanderFixture {
        source_id: u32,
        source_position_raw: [i16; 3],
        task_visits: [Option<ActorTaskVisit>; 3],
        task_before: ActorTaskRuntime,
        wrapper_before: ActorTaskWrapperFlags,
        selected_before: crate::ordinary_type9_live::OrdinaryType9SelectedComponentRuntime,
    }

    fn publish_type9_wander_fixture(manager: &mut EntityManager) -> PublishedWanderFixture {
        let (source_id, task_visits) = manager
            .fresh_level1_type9_initial_productions
            .iter()
            .find_map(|owner| {
                matches!(
                    owner.branch(),
                    FreshLevel1Type9InitialProductionBranch::Wander(
                        OrdinaryType9WanderInitializerOutcome::Published { .. }
                    )
                )
                .then(|| (owner.entity_id(), owner.task_visits()))
            })
            .expect("fresh Level-1 production must retain a selected Wander owner");
        let source = manager
            .entity_mut_for_test(source_id)
            .expect("live type-9 source");
        let source_position_raw = source.position_raw();
        let visit = task_visits[ActorTaskSlot::Primary as usize]
            .expect("selected Wander retains its Primary task visit");
        let task_before = *source
            .actor_tasks
            .task_state(visit.task_id)
            .expect("selected Wander retains its task state");
        let wrapper_before = source
            .actor_tasks
            .wrapper_flags(visit.task_id)
            .expect("selected Wander retains its wrapper state");
        let selected_before = source
            .ordinary_type9_selected_component_runtime
            .expect("selected Wander retains component custody");
        source.set_heading_raw(0);
        let RetailRuntimeValue::Known(Some(mut sub_a)) = source.sub_a_propulsion_runtime else {
            panic!("fresh type-9 Sub-A runtime");
        };
        sub_a.set_direction_multiplier(-1);
        source.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(sub_a));
        PublishedWanderFixture {
            source_id,
            source_position_raw,
            task_visits,
            task_before,
            wrapper_before,
            selected_before,
        }
    }

    fn assert_wander_production_custody_intact(
        manager: &EntityManager,
        fixture: PublishedWanderFixture,
    ) {
        let owner = manager
            .fresh_level1_type9_initial_productions
            .iter()
            .find(|owner| owner.entity_id() == fixture.source_id)
            .expect("descriptor contact must preserve the manager sidecar");
        assert_eq!(owner.task_visits(), fixture.task_visits);
        assert!(matches!(
            owner.branch(),
            FreshLevel1Type9InitialProductionBranch::Wander(
                OrdinaryType9WanderInitializerOutcome::Published { .. }
            )
        ));
        let source = manager
            .iter_all()
            .find(|entity| entity.id == fixture.source_id)
            .expect("descriptor contact must preserve the source entity");
        let visit = fixture.task_visits[ActorTaskSlot::Primary as usize].unwrap();
        assert_eq!(
            source.actor_tasks.task_state(visit.task_id),
            Some(&fixture.task_before)
        );
        assert_eq!(
            source.actor_tasks.wrapper_flags(visit.task_id),
            Some(fixture.wrapper_before)
        );
        assert_eq!(
            source.ordinary_type9_selected_component_runtime,
            Some(fixture.selected_before)
        );
    }

    fn runtime(id: u32, capability_flags: u32, class_id: u32) -> RuntimePairSnapshot {
        RuntimePairSnapshot {
            id,
            entity_type: 0,
            capability_flags,
            behavior_context: RetailRuntimeValue::Known(Some(
                BehaviorContextRuntime::from_published_weighted_selection(BehaviorSelection {
                    choice_index: 0,
                    program: behavior_program(class_id).unwrap(),
                })
                .expect("test class comes from the weighted behavior catalog"),
            )),
            pair_callbacks: EntityPairCallbackRuntimeState::unresolved(),
            model_to_world: RetailRuntimeValue::Known([
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ]),
            anim_vars: AnimVars::default(),
            type9_descriptor_contact_owner: None,
            type17_descriptor_contact_owner: None,
            spawn_index: None,
        }
    }

    fn unresolved_style(id: u32, entity_type: u32, capability_flags: u32) -> RuntimePairSnapshot {
        RuntimePairSnapshot {
            id,
            entity_type,
            spawn_index: None,
            capability_flags,
            behavior_context: RetailRuntimeValue::Unresolved,
            pair_callbacks:
                EntityPairCallbackRuntimeState::unresolved_with_constructor_null_modifier(),
            model_to_world: RetailRuntimeValue::Known([
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ]),
            anim_vars: AnimVars::default(),
            type9_descriptor_contact_owner: None,
            type17_descriptor_contact_owner: None,
        }
    }

    #[test]
    fn unresolved_style_uses_constructor_pair_policy_for_player_base_factory_and_hive() {
        let player = unresolved_style(1, PLAYER_ENTITY_TYPE, 5);
        let main_base = unresolved_style(2, MAIN_BASE_ENTITY_TYPE, 0);
        let factory = unresolved_style(3, FACTORY_ENTITY_TYPE, 0);
        let hive = unresolved_style(67, HIVE_ENTITY_TYPE, 0);
        let villager = unresolved_style(4, 9, 0);
        let mut presentation = Vec::new();

        assert_eq!(
            evaluate_behavior_callback(&player, &main_base, [0; 3], [0; 3], &mut presentation),
            BehaviorDecision::Continue
        );
        assert_eq!(
            evaluate_behavior_callback(&main_base, &player, [0; 3], [0; 3], &mut presentation),
            BehaviorDecision::Continue
        );
        assert_eq!(
            evaluate_behavior_callback(&factory, &player, [0; 3], [0; 3], &mut presentation),
            BehaviorDecision::Continue
        );
        assert_eq!(
            evaluate_behavior_callback(&hive, &player, [0; 3], [0; 3], &mut presentation),
            BehaviorDecision::Continue
        );
        assert_eq!(
            evaluate_behavior_callback(&villager, &player, [0; 3], [0; 3], &mut presentation),
            BehaviorDecision::Unresolved
        );
    }

    #[test]
    fn player_main_base_and_lifter_callbacks_use_opposing_capabilities() {
        let player = runtime(1, 5, 24);
        let main_base = runtime(2, 0, 41);
        let lifter = runtime(3, 0, 39);
        let mut presentation = Vec::new();

        assert_eq!(
            evaluate_behavior_callback(&player, &main_base, [0; 3], [0; 3], &mut presentation,),
            BehaviorDecision::Continue
        );
        assert_eq!(
            evaluate_behavior_callback(&main_base, &player, [0; 3], [0; 3], &mut presentation,),
            BehaviorDecision::Continue
        );
        assert_eq!(
            evaluate_behavior_callback(&lifter, &player, [0; 3], [0; 3], &mut presentation,),
            BehaviorDecision::Continue
        );
    }

    #[test]
    fn hive_pair_callback_is_a_noop_for_the_player_capability_set() {
        let hive = runtime(67, 0, 46);
        let player = runtime(46, 5, 24);
        let consumable = runtime(9, CONSUMABLE_TARGET_CAPABILITIES, 6);
        let mut presentation = Vec::new();

        assert_eq!(
            evaluate_behavior_callback(&hive, &player, [0; 3], [0; 3], &mut presentation),
            BehaviorDecision::Continue
        );
        assert_eq!(
            evaluate_behavior_callback(&hive, &consumable, [0; 3], [0; 3], &mut presentation),
            BehaviorDecision::Unresolved,
            "the exact player admission must not broaden Hive consumption"
        );
    }

    #[test]
    fn unexpectedly_reached_power_up_policy_fails_closed() {
        let power_up = runtime(61, 0, 23);
        let player = runtime(46, 5, 24);
        let mut presentation = Vec::new();
        assert_eq!(
            evaluate_behavior_callback(&power_up, &player, [0; 3], [0; 3], &mut presentation,),
            BehaviorDecision::Unresolved
        );
    }

    #[test]
    fn player_distance_feedback_is_explicit_but_does_not_block_contact() {
        let player = runtime(46, 5, 24);
        let obstacle = runtime(68, 0, 0);
        let mut presentation = Vec::new();
        assert_eq!(
            evaluate_behavior_callback(&player, &obstacle, [0; 3], [513, 0, 0], &mut presentation,),
            BehaviorDecision::Continue
        );
        assert_eq!(presentation.len(), 1);
    }

    #[test]
    fn descriptor_components_remain_a_fail_closed_boundary() {
        let callbacks = EntityPairCallbackRuntimeState {
            component_contact: RetailRuntimeValue::Known([
                PairComponentContactPolicy::DescriptorContact,
                PairComponentContactPolicy::None,
                PairComponentContactPolicy::None,
            ]),
            damage_modifier_address: RetailRuntimeValue::Known(None),
            damage_modifier_identity_context_empty: RetailRuntimeValue::Known(true),
            type_hit_callback_address: RetailRuntimeValue::Known(None),
            orientation_policy: RetailRuntimeValue::Unresolved,
        };
        assert!(!component_callbacks_are_noop(callbacks));
    }

    #[test]
    fn player_modifier_identity_requires_a_known_empty_sub_j_context() {
        let mut player = runtime(46, 5, 24);
        player.pair_callbacks = EntityPairCallbackRuntimeState::audited_player();
        assert_eq!(
            damage_modifier_outcome(Some(&player)),
            PairDamageModifierOutcome::Unresolved
        );

        player.pair_callbacks.damage_modifier_identity_context_empty =
            RetailRuntimeValue::Known(false);
        assert_eq!(
            damage_modifier_outcome(Some(&player)),
            PairDamageModifierOutcome::Unresolved
        );

        player.pair_callbacks.damage_modifier_identity_context_empty =
            RetailRuntimeValue::Known(true);
        assert_eq!(
            damage_modifier_outcome(Some(&player)),
            PairDamageModifierOutcome::Identity
        );
    }

    #[v2k_test_support::retail_test]
    fn authenticated_type9_descriptor_contact_commits_with_player_body_pass() {
        let (session, mut manager) = fresh_level_one();
        let fixture = publish_type9_wander_fixture(&mut manager);

        manager
            .player_mut()
            .expect("persistent player")
            .set_motion_raw(fixture.source_position_raw, [0; 3]);
        let mut hull = PlayerHull::default();
        let mut notifications = GameplayNotifications::new();
        let pass = resolve_player_active_contacts(
            &mut manager,
            &session.cache,
            PlayerActivePairFrame {
                resources: &session.cache,
                retail_tick: 12_010,
                player_craft: &PlayerCraft::new(),
                player_hull: &mut hull,
                scheduler: &mut SpecializedActorTaskScheduler::new(),
                world_fx: &mut WorldFx::new(),
                notifications: &mut notifications,
                extra_lives: 0,
            },
        )
        .expect("authenticated descriptor contact");

        assert_eq!(pass.descriptor_contacts.len(), 1);
        let outcome = pass.descriptor_contacts[0];
        assert_eq!(outcome.entity_id, fixture.source_id);
        assert_eq!(outcome.heading_raw_before, 0);
        assert_eq!(outcome.heading_raw_after, 0x2000);
        assert_eq!(outcome.direction_multiplier_before, -1);
        assert_eq!(outcome.direction_multiplier_after, 1);
        let source = manager
            .iter_all()
            .find(|entity| entity.id == fixture.source_id)
            .expect("committed type-9 source");
        assert_eq!(source.heading_raw(), 0x2000);
        let RetailRuntimeValue::Known(Some(sub_a)) = source.sub_a_propulsion_runtime else {
            panic!("committed type-9 Sub-A runtime");
        };
        assert_eq!(sub_a.direction_multiplier(), 1);
        assert_wander_production_custody_intact(&manager, fixture);
    }

    #[v2k_test_support::retail_test]
    fn later_damage_block_discards_staged_type9_descriptor_contact() {
        let (session, mut manager) = fresh_level_one();
        let fixture = publish_type9_wander_fixture(&mut manager);
        let source = manager
            .entity_mut_for_test(fixture.source_id)
            .expect("live type-9 source");
        source.collision.pair_callbacks.type_hit_callback_address =
            RetailRuntimeValue::Known(Some(0x0040_DEAD));
        let sub_a_before = source.sub_a_propulsion_runtime;

        manager
            .player_mut()
            .expect("persistent player")
            .set_motion_raw(fixture.source_position_raw, [0, -4_096, 0]);
        let mut hull = PlayerHull::default();
        let hull_before = hull;
        let mut notifications = GameplayNotifications::new();
        let error = resolve_player_active_contacts(
            &mut manager,
            &session.cache,
            PlayerActivePairFrame {
                resources: &session.cache,
                retail_tick: 12_010,
                player_craft: &PlayerCraft::new(),
                player_hull: &mut hull,
                scheduler: &mut SpecializedActorTaskScheduler::new(),
                world_fx: &mut WorldFx::new(),
                notifications: &mut notifications,
                extra_lives: 0,
            },
        )
        .expect_err("unknown post-component type-hit callback must stop the pass");

        assert!(matches!(
            error,
            PlayerActivePairError::Core(ActivePairUnresolved::Callback {
                entity_id,
                phase: crate::active_pair::PairCallbackPhase::TypeHit,
            }) if entity_id == fixture.source_id
        ));
        let source = manager
            .iter_all()
            .find(|entity| entity.id == fixture.source_id)
            .expect("rolled-back type-9 source");
        assert_eq!(source.heading_raw(), 0);
        assert_eq!(source.sub_a_propulsion_runtime, sub_a_before);
        assert_eq!(hull, hull_before);
        assert_wander_production_custody_intact(&manager, fixture);
    }
}
