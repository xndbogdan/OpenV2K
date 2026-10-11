//! Mutation-safe dynamic half of retail radial delivery.
//!
//! `FUN_004566E0` completes its static-cell scan before `FUN_00414AE0` walks
//! the intrusive live-entity list.  The port retains that ordering at the
//! caller, while this module preflights the complete dynamic walk into a pure
//! plan.  An unresolved in-range callback or lifecycle discards that plan, so
//! a fail-closed request cannot leave an earlier entity with impulse, damage,
//! lifecycle state, or audio from a partially executed pass.

use super::{BaseFactoryRuntimeState, Entity, EntityManager};
use crate::base_factory_progression::{ProgressiveDeathBegin, PROGRESSION_REVIVE_HEALTH_RAW};
use crate::damage::{
    generic_entity_damage_transition, GenericEntityDamageStage, GenericEntityDamageState,
};
use crate::entity_behavior::DeathCallbackPolicy;
use crate::entity_collision_state::{
    EntityCollisionRuntimeState, RetailRuntimeValue, CHECKED_DAMAGE_ENABLED_STATE_BIT,
    DYING_STATE_BIT, PAIR_COLLISION_FIXED_STATE_BIT, PAIR_COLLISION_INELIGIBLE_STATE_BIT,
    RADIAL_DAMAGE_ALTERNATE_ELIGIBLE_STATE_BIT, REMOTE_OWNED_STATE_BIT,
};
use crate::player_hull::PlayerHull;
use crate::radial_damage::{radial_distance_raw, scale_radial_damage, RadialDamageTemplate};

mod live;
pub use live::{
    DynamicRadialDeathPublication, DynamicRadialLiveBlock, DynamicRadialLiveBlockReason,
    DynamicRadialLiveCallbacks, DynamicRadialLiveOutcome, DynamicRadialLivePhase,
    DynamicRadialLiveRequest,
};

const IMPULSE_SUPPRESSED_ENTITY_TYPE: u32 = 0x23;
const PLAYER_DAMAGE_MODIFIER_ADDRESS: u32 = 0x0044_84A0;

/// One positional sound requested by the generic damage path, in retail
/// live-list/callback order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicRadialSound {
    pub sound_id: u16,
    pub position_raw: [i16; 3],
}

/// Successfully committed dynamic pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicRadialApplied {
    /// Number of entities for which `FUN_00425430` returned nonzero. A target
    /// may count here even when its filtered packet is zero and no impulse was
    /// requested.
    pub accepted_targets: usize,
    pub sounds: Vec<DynamicRadialSound>,
    /// Hive67 lethal follow-ups for the caller's `FUN_00440950` burst.
    pub hive_dying_bursts: Vec<HiveDyingBurst>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HiveDyingBurst {
    pub entity_id: u32,
    pub position_raw: [i16; 3],
    pub dying_model: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicRadialRuntimeField {
    EligibilityState,
    FixedState,
    RemoteOwnership,
    DamageProfile,
    DamageModifier,
    DamageModifierContext,
    TypeHitCallback,
    Health,
    PreHealthDamageBuffer,
    DyingState,
    GenericHitSound,
    DeathSound,
    CurrentBehaviorStyle,
    BaseFactoryRuntime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicRadialUnresolvedReason {
    SelectedType9MutationCustody,
    PlayerHullUnavailable,
    MissingRuntimeField(DynamicRadialRuntimeField),
    RemoteDamageDispatchRequired,
    UnknownDamageModifier(u32),
    PlayerDamageModifierContextNotEmpty,
    TypeHitCallbackRequired(u32),
    FilteredZeroFeedback,
    DeathCallbackPolicy(DeathCallbackPolicy),
    ProgressiveDeathTerminalReentry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicRadialUnresolved {
    pub target_id: u32,
    pub reason: DynamicRadialUnresolvedReason,
}

/// Failure unique to a composed preflight that substitutes one already
/// staged collision snapshot for its live allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DynamicRadialCollisionOverrideError {
    TargetMissing { target_id: u32 },
    TargetDuplicate { target_id: u32 },
    Dynamic(DynamicRadialUnresolved),
}

impl From<DynamicRadialUnresolved> for DynamicRadialCollisionOverrideError {
    fn from(unresolved: DynamicRadialUnresolved) -> Self {
        Self::Dynamic(unresolved)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DynamicRadialDamageOutcome {
    Applied(DynamicRadialApplied),
    Aborted(DynamicRadialUnresolved),
}

/// Cinematic worlds have no player allocation or hull authority. Gameplay
/// passes the live hull so damage updates its controller and entity together.
pub enum DynamicRadialPlayerContext<'a> {
    Absent,
    Present(&'a mut PlayerHull),
}

#[derive(Debug)]
struct PlannedEntityMutation {
    index: usize,
    target_id: u32,
    velocity_raw: [i16; 3],
    collision: EntityCollisionRuntimeState,
    base_factory_runtime: RetailRuntimeValue<Option<BaseFactoryRuntimeState>>,
    hive_dying: bool,
}

/// Complete read-only result of one dynamic radial walk.
///
/// The plan owns every entity, hull, and sound value needed by commit.  It is
/// intentionally neither `Clone` nor externally mutable: a wider transaction
/// can preflight static targets after this succeeds, perform the ordered
/// static/WorldFx prefix, and then consume this plan without re-entering any
/// callback-support or lifecycle gate.
#[must_use = "a preflighted dynamic radial pass has no effect until committed"]
#[derive(Debug)]
pub(crate) struct DynamicRadialDamagePlan {
    mutations: Vec<PlannedEntityMutation>,
    player_hull_after: Option<PlayerHull>,
    accepted_targets: usize,
    sounds: Vec<DynamicRadialSound>,
}

impl DynamicRadialDamagePlan {
    /// A parked contact owner cannot accept buffer/health changes merely
    /// because this radial plan leaves its velocity unchanged. Filtered-zero,
    /// zero-impulse identity plans remain no-ops. The surviving/death sound
    /// branches accompany a changed health, buffer or progression state.
    pub(crate) fn changes_target(&self, manager: &EntityManager, target_id: u32) -> bool {
        self.mutations.iter().any(|mutation| {
            if mutation.target_id != target_id {
                return false;
            }
            let entity = &manager.entities[mutation.index];
            debug_assert_eq!(entity.id, mutation.target_id);
            entity.velocity_raw() != mutation.velocity_raw
                || entity.collision != mutation.collision
                || entity.base_factory_runtime != mutation.base_factory_runtime
        })
    }

    /// Only actual velocity writes need a selected mover's body receipt.
    /// Fixed targets and zero impulses preserve their existing custody.
    pub(crate) fn changed_velocity_targets(&self, manager: &EntityManager) -> Vec<(u32, [i16; 3])> {
        self.mutations
            .iter()
            .filter_map(|mutation| {
                let entity = &manager.entities[mutation.index];
                debug_assert_eq!(entity.id, mutation.target_id);
                (entity.velocity_raw() != mutation.velocity_raw)
                    .then_some((mutation.target_id, mutation.velocity_raw))
            })
            .collect()
    }
}

impl EntityManager {
    /// Apply `FUN_00414AE0` only when every in-range mutation and callback is
    /// supported by retained live data.
    ///
    /// This deliberately does not widen the active-pair whitelist: radial
    /// admission follows each allocation's callback/state data independently.
    pub fn apply_dynamic_radial_damage(
        &mut self,
        player: DynamicRadialPlayerContext<'_>,
        origin_raw: [i16; 3],
        template: RadialDamageTemplate,
    ) -> DynamicRadialDamageOutcome {
        let player_hull = match player {
            DynamicRadialPlayerContext::Absent => {
                if let Some(target_id) = self.player_id {
                    return DynamicRadialDamageOutcome::Aborted(unresolved(
                        target_id,
                        DynamicRadialUnresolvedReason::PlayerHullUnavailable,
                    ));
                }
                None
            }
            DynamicRadialPlayerContext::Present(hull) => Some(hull),
        };
        let plan = match plan_dynamic_radial_damage(
            self.entities.iter().enumerate(),
            self.player_id,
            self.player_pair_damage_modifier_context_empty(),
            player_hull.as_deref().copied(),
            origin_raw,
            template,
            None,
        ) {
            Ok(plan) => plan,
            Err(unresolved) => return DynamicRadialDamageOutcome::Aborted(unresolved),
        };

        DynamicRadialDamageOutcome::Applied(
            self.commit_preflighted_dynamic_radial_damage(player_hull, plan),
        )
    }

    /// Resolve the complete dynamic half of `FUN_004566E0` without mutation.
    ///
    /// A successful plan has already traversed every in-range entity in live
    /// order and resolved all eligibility, ownership, modifier, callback,
    /// health, sound, and death-lifecycle state that commit could need.  Code
    /// composing the static and dynamic halves may therefore hold this plan
    /// while it performs only non-entity preflight or the ordered
    /// static/WorldFx prefix. It must not mutate this manager or `player_hull`
    /// before passing both back to
    /// [`Self::commit_preflighted_dynamic_radial_damage`].
    #[cfg(test)]
    pub(crate) fn preflight_dynamic_radial_damage(
        &self,
        player_hull: &PlayerHull,
        origin_raw: [i16; 3],
        template: RadialDamageTemplate,
    ) -> Result<DynamicRadialDamagePlan, DynamicRadialUnresolved> {
        plan_dynamic_radial_damage(
            self.entities.iter().enumerate(),
            self.player_id,
            self.player_pair_damage_modifier_context_empty(),
            Some(*player_hull),
            origin_raw,
            template,
            None,
        )
    }

    /// Plan one callback-free target at its position in the live radial walk.
    /// Global entity indices remain intact for the eventual commit. Native
    /// death owners execute separately between these target-local plans.
    pub(crate) fn preflight_dynamic_radial_target(
        &self,
        player_hull: &PlayerHull,
        origin_raw: [i16; 3],
        template: RadialDamageTemplate,
        target_id: u32,
    ) -> Result<DynamicRadialDamagePlan, DynamicRadialUnresolved> {
        plan_dynamic_radial_damage(
            self.entities
                .iter()
                .enumerate()
                .filter(|(_, entity)| entity.id == target_id),
            self.player_id,
            self.player_pair_damage_modifier_context_empty(),
            Some(*player_hull),
            origin_raw,
            template,
            None,
        )
    }

    /// Resolve the dynamic walk against one collision snapshot staged by an
    /// earlier phase of the same transaction.
    ///
    /// The target identity must name exactly one live allocation. Only that
    /// allocation's collision state is substituted; its position, velocity,
    /// behavior, factory runtime, and every other entity remain live reads.
    /// Neither the manager nor the borrowed snapshot is mutated.
    pub(super) fn preflight_dynamic_radial_damage_with_collision_override(
        &self,
        player_hull: &PlayerHull,
        origin_raw: [i16; 3],
        template: RadialDamageTemplate,
        target_id: u32,
        collision_override: &EntityCollisionRuntimeState,
    ) -> Result<DynamicRadialDamagePlan, DynamicRadialCollisionOverrideError> {
        let mut matching_targets = self.entities.iter().filter(|entity| entity.id == target_id);
        if matching_targets.next().is_none() {
            return Err(DynamicRadialCollisionOverrideError::TargetMissing { target_id });
        }
        if matching_targets.next().is_some() {
            return Err(DynamicRadialCollisionOverrideError::TargetDuplicate { target_id });
        }

        plan_dynamic_radial_damage(
            self.entities.iter().enumerate(),
            self.player_id,
            self.player_pair_damage_modifier_context_empty(),
            Some(*player_hull),
            origin_raw,
            template,
            Some((target_id, collision_override)),
        )
        .map_err(Into::into)
    }

    /// Consume an already complete dynamic radial plan without another
    /// support check, callback dispatch, RNG draw, or fallible outcome.
    ///
    /// The caller retains custody of the manager and player hull between
    /// preflight and commit. Static-target mutation and WorldFx publication do
    /// not touch either authority and are therefore safe to order between the
    /// two calls.
    pub(crate) fn commit_preflighted_dynamic_radial_damage(
        &mut self,
        player_hull: Option<&mut PlayerHull>,
        plan: DynamicRadialDamagePlan,
    ) -> DynamicRadialApplied {
        let DynamicRadialDamagePlan {
            mutations,
            player_hull_after,
            accepted_targets,
            sounds,
        } = plan;

        // The planner performs no callbacks or topology changes, so these
        // identities must still match when the atomic commit begins.
        let mut hive_dying_bursts = Vec::new();
        for mutation in mutations {
            let entity = self
                .entities
                .get_mut(mutation.index)
                .expect("radial plan index remains live until commit");
            debug_assert_eq!(entity.id, mutation.target_id);
            entity.velocity = super::raw_position_world(mutation.velocity_raw);
            entity.collision = mutation.collision;
            entity.base_factory_runtime = mutation.base_factory_runtime;
            if mutation.hive_dying {
                if let Some(dying_model) = entity.apply_hive_dying_initializer() {
                    hive_dying_bursts.push(HiveDyingBurst {
                        entity_id: mutation.target_id,
                        position_raw: entity.position_raw(),
                        dying_model,
                    });
                }
            }
        }
        match (player_hull, player_hull_after) {
            (Some(hull), Some(after)) => *hull = after,
            (None, None) => {}
            _ => unreachable!("radial plan retains its caller's hull authority"),
        }

        DynamicRadialApplied {
            accepted_targets,
            sounds,
            hive_dying_bursts,
        }
    }
}

fn plan_dynamic_radial_damage<'a>(
    entities: impl Iterator<Item = (usize, &'a Entity)>,
    player_id: Option<u32>,
    player_modifier_context_empty: RetailRuntimeValue<bool>,
    player_hull: Option<PlayerHull>,
    origin_raw: [i16; 3],
    template: RadialDamageTemplate,
    collision_override: Option<(u32, &EntityCollisionRuntimeState)>,
) -> Result<DynamicRadialDamagePlan, DynamicRadialUnresolved> {
    let mut plan = DynamicRadialDamagePlan {
        mutations: Vec::new(),
        player_hull_after: player_hull,
        accepted_targets: 0,
        sounds: Vec::new(),
    };

    for (index, target) in entities {
        let target_raw = target.position_raw();
        let collision_before = collision_override
            .filter(|(target_id, _)| *target_id == target.id)
            .map_or(&target.collision, |(_, collision)| collision);
        // Reading the gate precedes distance in retail, but neither read has a
        // side effect. Rejecting the strict outer miss first lets unrelated
        // unresolved allocations remain outside this transaction boundary.
        if radial_distance_raw(origin_raw, target_raw) >= i32::from(template.outer_radius_raw) {
            continue;
        }

        let enabled = required(
            collision_before
                .state_flags_at_0x08
                .masked(CHECKED_DAMAGE_ENABLED_STATE_BIT),
            target.id,
            DynamicRadialRuntimeField::EligibilityState,
        )?;
        if enabled == 0 {
            continue;
        }
        let alternate = required(
            collision_before
                .state_flags_at_0x08
                .masked(RADIAL_DAMAGE_ALTERNATE_ELIGIBLE_STATE_BIT),
            target.id,
            DynamicRadialRuntimeField::EligibilityState,
        )?;
        if alternate == 0 {
            let ineligible = required(
                collision_before
                    .state_flags_at_0x08
                    .masked(PAIR_COLLISION_INELIGIBLE_STATE_BIT),
                target.id,
                DynamicRadialRuntimeField::EligibilityState,
            )?;
            if ineligible != 0 {
                continue;
            }
        }

        let request_impulse = if target.mass_raw < 2 {
            false
        } else {
            required(
                collision_before
                    .state_flags_at_0x08
                    .masked(PAIR_COLLISION_FIXED_STATE_BIT),
                target.id,
                DynamicRadialRuntimeField::FixedState,
            )? == 0
        };
        let Some(scaled) = scale_radial_damage(template, origin_raw, target_raw, request_impulse)
        else {
            continue;
        };
        plan.accepted_targets += 1;

        let mut velocity_raw = target.velocity_raw();
        let mut collision = collision_before.clone();
        let mut base_factory_runtime = target.base_factory_runtime;
        let mut mutated = false;
        let mut hive_dying = false;

        if target.entity_type != IMPULSE_SUPPRESSED_ENTITY_TYPE {
            if let Some(impulse) = scaled.impulse_vector_raw {
                require_local_ownership(target.id, collision_before)?;
                velocity_raw = [
                    velocity_raw[0].wrapping_add(impulse[0]),
                    velocity_raw[1].wrapping_add(impulse[1]),
                    velocity_raw[2].wrapping_add(impulse[2]),
                ];
                mutated = true;
            }
        }

        // Full-radius delivery enters FUN_00415040; its outer gate is the same
        // live 0x8000 bit already resolved above. Falloff enters FUN_00414E10.
        let profile = required(
            collision.damage_profile,
            target.id,
            DynamicRadialRuntimeField::DamageProfile,
        )?;
        let filtered_damage_raw = scaled.packet.filtered_raw(Some(&profile));
        if filtered_damage_raw == 0 {
            if filtered_zero_feedback_required(target.capability_flags, scaled) {
                return Err(unresolved(
                    target.id,
                    DynamicRadialUnresolvedReason::FilteredZeroFeedback,
                ));
            }
            if mutated {
                plan.mutations.push(PlannedEntityMutation {
                    index,
                    target_id: target.id,
                    velocity_raw,
                    collision,
                    base_factory_runtime,
                    hive_dying: false,
                });
            }
            continue;
        }

        admit_damage_modifier(
            target.id,
            collision_before,
            player_id,
            player_modifier_context_empty,
        )?;
        admit_type_hit_callback(target.id, collision_before)?;
        require_local_ownership(target.id, collision_before)?;

        let health_raw = required(
            collision.health_raw,
            target.id,
            DynamicRadialRuntimeField::Health,
        )?;
        let pre_health_buffer_raw = required(
            collision.pre_health_damage_buffer_raw,
            target.id,
            DynamicRadialRuntimeField::PreHealthDamageBuffer,
        )?;
        let already_dying = required(
            collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
            target.id,
            DynamicRadialRuntimeField::DyingState,
        )? != 0;
        let transition = generic_entity_damage_transition(
            GenericEntityDamageState {
                health_raw,
                pre_health_buffer_raw,
                already_dying,
            },
            filtered_damage_raw,
        );

        match transition.stage {
            GenericEntityDamageStage::AlreadyDying => {
                collision.pre_health_damage_buffer_raw =
                    RetailRuntimeValue::Known(transition.pre_health_buffer_after_raw);
                if player_id == Some(target.id) {
                    plan.player_hull_after
                        .as_mut()
                        .expect("player identity has a live hull")
                        .pre_health_damage_buffer_raw = transition.pre_health_buffer_after_raw;
                }
            }
            GenericEntityDamageStage::Survived => {
                stage_live_hit_sound(target.id, target_raw, collision_before, &mut plan.sounds)?;
                collision.health_raw =
                    RetailRuntimeValue::Known(transition.health_after_subtraction_raw);
                collision.pre_health_damage_buffer_raw =
                    RetailRuntimeValue::Known(transition.pre_health_buffer_after_raw);
                if player_id == Some(target.id) {
                    plan.player_hull_after
                        .as_mut()
                        .expect("player identity has a live hull")
                        .health_raw = transition.health_after_subtraction_raw;
                    plan.player_hull_after
                        .as_mut()
                        .expect("player identity has a live hull")
                        .pre_health_damage_buffer_raw = transition.pre_health_buffer_after_raw;
                }
            }
            GenericEntityDamageStage::DeathDispatchRequired => {
                stage_live_hit_sound(target.id, target_raw, collision_before, &mut plan.sounds)?;
                stage_death_sound(target.id, target_raw, collision_before, &mut plan.sounds)?;
                if player_id == Some(target.id) {
                    collision.health_raw = RetailRuntimeValue::Known(0);
                    collision.pre_health_damage_buffer_raw =
                        RetailRuntimeValue::Known(transition.pre_health_buffer_after_raw);
                    collision
                        .state_flags_at_0x08
                        .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
                    plan.player_hull_after
                        .as_mut()
                        .expect("player identity has a live hull")
                        .health_raw = 0;
                    plan.player_hull_after
                        .as_mut()
                        .expect("player identity has a live hull")
                        .pre_health_damage_buffer_raw = transition.pre_health_buffer_after_raw;
                    plan.player_hull_after
                        .as_mut()
                        .expect("player identity has a live hull")
                        .dying = true;
                } else {
                    let style = required(
                        target.current_behavior_context,
                        target.id,
                        DynamicRadialRuntimeField::CurrentBehaviorStyle,
                    )?;
                    let death_policy = style.map_or(DeathCallbackPolicy::None, |context| {
                        context.active_style().death_callback_policy()
                    });
                    let hive_ready = style.is_some_and(|context| {
                        context.active_style().audited().is_some_and(|live| {
                            crate::hive_death::hive_dying_initializer_ready(
                                target.entity_type,
                                target.model_slots,
                                match collision.active_model_slot() {
                                    RetailRuntimeValue::Known(slot) => slot,
                                    RetailRuntimeValue::Unresolved => usize::MAX,
                                },
                                live,
                            )
                        })
                    });
                    if hive_ready {
                        collision.health_raw = RetailRuntimeValue::Known(0);
                        collision.pre_health_damage_buffer_raw =
                            RetailRuntimeValue::Known(transition.pre_health_buffer_after_raw);
                        collision
                            .state_flags_at_0x08
                            .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
                        hive_dying = true;
                    } else if death_policy != DeathCallbackPolicy::BaseFactoryProgression {
                        return Err(unresolved(
                            target.id,
                            DynamicRadialUnresolvedReason::DeathCallbackPolicy(death_policy),
                        ));
                    } else {
                        let mut runtime = required(
                            base_factory_runtime,
                            target.id,
                            DynamicRadialRuntimeField::BaseFactoryRuntime,
                        )?
                        .ok_or_else(|| {
                            unresolved(
                                target.id,
                                DynamicRadialUnresolvedReason::MissingRuntimeField(
                                    DynamicRadialRuntimeField::BaseFactoryRuntime,
                                ),
                            )
                        })?;
                        let ProgressiveDeathBegin::Revived { state } =
                            runtime.progressive_death.begin()
                        else {
                            return Err(unresolved(
                                target.id,
                                DynamicRadialUnresolvedReason::ProgressiveDeathTerminalReentry,
                            ));
                        };
                        runtime.progressive_death = state;
                        collision.health_raw =
                            RetailRuntimeValue::Known(PROGRESSION_REVIVE_HEALTH_RAW);
                        collision.pre_health_damage_buffer_raw =
                            RetailRuntimeValue::Known(transition.pre_health_buffer_after_raw);
                        collision.state_flags_at_0x08.overwrite(DYING_STATE_BIT, 0);
                        base_factory_runtime = RetailRuntimeValue::Known(Some(runtime));
                    }
                }
            }
        }
        mutated = true;

        if mutated {
            plan.mutations.push(PlannedEntityMutation {
                index,
                target_id: target.id,
                velocity_raw,
                collision,
                base_factory_runtime,
                hive_dying,
            });
        }
    }

    Ok(plan)
}

fn filtered_zero_feedback_required(
    capability_flags: u32,
    scaled: crate::radial_damage::ScaledRadialDamage,
) -> bool {
    // 15040's filtered-zero UI feedback is specifically player-originated.
    // 14AE0 passes the template's source word through unchanged; a meteor or
    // static explosion cannot acquire this branch from target capability 8.
    scaled.checked_damage
        && crate::damage::DamageDeliveryRecord {
            packet: scaled.packet,
            source_entity_type_raw: scaled.trailing_raw[0] as u32,
            owner_handle: scaled.trailing_raw[1] as u32,
        }
        .filtered_zero_feedback_required(capability_flags)
}

fn required<T: Copy>(
    value: RetailRuntimeValue<T>,
    target_id: u32,
    field: DynamicRadialRuntimeField,
) -> Result<T, DynamicRadialUnresolved> {
    match value {
        RetailRuntimeValue::Known(value) => Ok(value),
        RetailRuntimeValue::Unresolved => Err(unresolved(
            target_id,
            DynamicRadialUnresolvedReason::MissingRuntimeField(field),
        )),
    }
}

fn unresolved(target_id: u32, reason: DynamicRadialUnresolvedReason) -> DynamicRadialUnresolved {
    DynamicRadialUnresolved { target_id, reason }
}

fn require_local_ownership(
    target_id: u32,
    collision: &EntityCollisionRuntimeState,
) -> Result<(), DynamicRadialUnresolved> {
    let remote = required(
        collision.state_flags_at_0x08.masked(REMOTE_OWNED_STATE_BIT),
        target_id,
        DynamicRadialRuntimeField::RemoteOwnership,
    )?;
    if remote != 0 {
        return Err(unresolved(
            target_id,
            DynamicRadialUnresolvedReason::RemoteDamageDispatchRequired,
        ));
    }
    Ok(())
}

fn admit_damage_modifier(
    target_id: u32,
    collision: &EntityCollisionRuntimeState,
    player_id: Option<u32>,
    player_modifier_context_empty: RetailRuntimeValue<bool>,
) -> Result<(), DynamicRadialUnresolved> {
    match required(
        collision.pair_callbacks.damage_modifier_address,
        target_id,
        DynamicRadialRuntimeField::DamageModifier,
    )? {
        None => Ok(()),
        Some(PLAYER_DAMAGE_MODIFIER_ADDRESS) if player_id == Some(target_id) => {
            if required(
                player_modifier_context_empty,
                target_id,
                DynamicRadialRuntimeField::DamageModifierContext,
            )? {
                Ok(())
            } else {
                Err(unresolved(
                    target_id,
                    DynamicRadialUnresolvedReason::PlayerDamageModifierContextNotEmpty,
                ))
            }
        }
        Some(address) => Err(unresolved(
            target_id,
            DynamicRadialUnresolvedReason::UnknownDamageModifier(address),
        )),
    }
}

fn admit_type_hit_callback(
    target_id: u32,
    collision: &EntityCollisionRuntimeState,
) -> Result<(), DynamicRadialUnresolved> {
    match required(
        collision.pair_callbacks.type_hit_callback_address,
        target_id,
        DynamicRadialRuntimeField::TypeHitCallback,
    )? {
        None => Ok(()),
        Some(address) => Err(unresolved(
            target_id,
            DynamicRadialUnresolvedReason::TypeHitCallbackRequired(address),
        )),
    }
}

fn stage_live_hit_sound(
    target_id: u32,
    position_raw: [i16; 3],
    collision: &EntityCollisionRuntimeState,
    sounds: &mut Vec<DynamicRadialSound>,
) -> Result<(), DynamicRadialUnresolved> {
    if let Some(sound_id) = required(
        collision.generic_hit_sound_id,
        target_id,
        DynamicRadialRuntimeField::GenericHitSound,
    )? {
        sounds.push(DynamicRadialSound {
            sound_id,
            position_raw,
        });
    }
    Ok(())
}

fn stage_death_sound(
    target_id: u32,
    position_raw: [i16; 3],
    collision: &EntityCollisionRuntimeState,
    sounds: &mut Vec<DynamicRadialSound>,
) -> Result<(), DynamicRadialUnresolved> {
    if let Some(sound_id) = required(
        collision.death_sound_id,
        target_id,
        DynamicRadialRuntimeField::DeathSound,
    )? {
        sounds.push(DynamicRadialSound {
            sound_id,
            position_raw,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor_task_dispatcher::ActorTaskRuntime;
    use crate::actor_task_owner::ActorTaskSlot;
    use crate::base_factory_progression::ProgressiveDeathState;
    use crate::damage::{DamageDeliveryRecord, DamagePacket, DamageProfile};
    use crate::entity::{
        next_entity_manager_allocation_generation, CommonEnvironmentPhysics, EntityKind,
        EntityManager, FACTORY_ENTITY_TYPE, INITIAL_PLAYER_CARGO_CAPACITY, PLAYER_ENTITY_TYPE,
    };
    use crate::entity_behavior::{
        audited_behavior_style, behavior_program, BehaviorContextRuntime, BehaviorSelection,
    };
    use crate::entity_collision_state::{
        EntityPairCallbackRuntimeState, PairOrientationPolicy, RetailStateWord,
        ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
    };
    use crate::hive_death::HiveRadialTaskState;
    use crate::hover::HoverPhysicsConfig;
    use crate::intro2_type17::capture::{
        apply_capture_pair_checked_damage, CaptureBlock, CaptureChildFrame, CaptureContext,
        CaptureHiveDyingBurst, CaptureTaskCustody,
    };
    use crate::world_fx::{METEOR_SURFACE_CLASS, UNDERWATER_IMPACT_CLASSES};
    use std::cell::Cell;
    use v2k_formats::collision::StatusComponentDescriptor;

    const TEMPLATE: RadialDamageTemplate = RadialDamageTemplate {
        inner_radius_raw: 0x100,
        outer_radius_raw: 0x200,
        impulse_raw: 2_000,
        packet: DamagePacket {
            channels: [1, 3],
            amounts_raw: [1_000, 1_000],
        },
        trailing_raw: [-1, 0],
    };

    const TYPE61_SHAPED_TEMPLATE: RadialDamageTemplate = RadialDamageTemplate {
        inner_radius_raw: 0x200,
        outer_radius_raw: 0x400,
        impulse_raw: 2_000,
        packet: DamagePacket {
            channels: [1, 3],
            amounts_raw: [4_000, 4_000],
        },
        // The later owner supplies the live type-61 handle in the second word.
        trailing_raw: [61, 0x1234],
    };

    fn profile() -> DamageProfile {
        DamageProfile {
            thresholds_raw: [0; 7],
            multipliers_q8: [256; 7],
        }
    }

    fn entity(id: u32, position_raw: [i16; 3], health: i32) -> Entity {
        let mut collision = EntityCollisionRuntimeState::unresolved_port_entity(0);
        collision.health_raw = RetailRuntimeValue::Known(health);
        collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
        collision.damage_profile = RetailRuntimeValue::Known(profile());
        collision.generic_hit_sound_id = RetailRuntimeValue::Known(Some(7));
        collision.death_sound_id = RetailRuntimeValue::Known(Some(9));
        collision.state_flags_at_0x08 = RetailStateWord::exact(
            CHECKED_DAMAGE_ENABLED_STATE_BIT | RADIAL_DAMAGE_ALTERNATE_ELIGIBLE_STATE_BIT,
        );
        collision.pair_callbacks = EntityPairCallbackRuntimeState::audited_local(
            None,
            RetailRuntimeValue::Known(PairOrientationPolicy::LiveHeadingWithAuthoredPitchRoll {
                pitch_raw: 0,
                roll_raw: 0,
            }),
        );
        Entity {
            id,
            construction_stamp_at_0xb4: RetailRuntimeValue::Unresolved,
            authored_spawn_index: Some(id as usize),
            kind: EntityKind::Obstacle,
            entity_type: 68,
            authored_follow_beacon_priority_raw: None,
            power_up_payload_packed: None,
            auto_pilot_payload_packed: None,
            factory_type61_birth_provenance: None,
            type60_construction_provenance: None,
            main_base_type54_sea_delta_source:
                crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            position: super::super::raw_position_world(position_raw),
            heading: 0.0,
            pitch_roll_raw: [0; 2],
            physical_body_basis_q31: RetailRuntimeValue::Unresolved,
            velocity: [0.0; 3],
            surface_lifetime_timer_ms_at_0x48: RetailRuntimeValue::Known(0),
            mass_raw: 100,
            capability_flags: 0,
            attached_to: None,
            model_slots: [None; 4],
            model_index: None,
            collision,
            initial_behavior: RetailRuntimeValue::Known(None),
            current_behavior_context: RetailRuntimeValue::Known(None),
            authored_radial_emitter: None,
            sub_n_runtime: RetailRuntimeValue::Known(None),
            base_factory_runtime: RetailRuntimeValue::Known(None),
            actor_animation_runtime: RetailRuntimeValue::Known(None),
            sub_a_propulsion_runtime: RetailRuntimeValue::Known(None),
            sub_g_06070_runtime: RetailRuntimeValue::Known(None),
            intro2_type13_common_mover_runtime: None,
            native_type13_allocation: None,
            intro2_type13_aim_runtime: None,
            intro2_type16_aim_runtime: None,
            intro2_type58_aim_runtime: None,
            intro2_type94_aim_runtime: None,
            intro2_flyer_aim_runtime: None,
            sub_h_external_frame_runtime: RetailRuntimeValue::Known(None),
            sub_j_attachment_runtime: RetailRuntimeValue::Known(None),
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
            native_type28_runtime: None,
            native_type76_runtime: None,
            native_type76_aim_runtime: None,
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
        }
    }

    fn manager(entities: Vec<Entity>) -> EntityManager {
        EntityManager {
            common_body_stamps: None,
            next_entity_id: entities
                .iter()
                .map(|entity| entity.id)
                .max()
                .map(|id| {
                    let next = id.wrapping_add(1);
                    if next == 0 {
                        1
                    } else {
                        next
                    }
                })
                .unwrap_or(1),
            entities,
            allocation_generation: next_entity_manager_allocation_generation(),
            native_class0_actors: std::collections::BTreeMap::new(),
            auxiliary_owned_ids: Vec::new(),
            fresh_new_game_first_world: false,
            construction_provenance: crate::entity::LevelEntityConstructionProvenance::Generic,
            fresh_level1_type9_initial_productions: Vec::new(),
            fresh_level1_type47_initial_productions: Vec::new(),
            pending_fresh_level1_type9_resource_text_receipts: Vec::new(),
            pending_power_up_destroy_ids: Vec::new(),
            pending_main_base_conversion_destroy_ids: Vec::new(),
            pending_factory_scientist_destroy_ids: Vec::new(),
            pending_actor_deferred_destroy_ids: Vec::new(),
            player_id: None,
            player_dying_contact_runtime: None,
            player_pending_replacement_handle: RetailRuntimeValue::Known(None),
            type_primary_models: Vec::new(),
            type_metadata: Vec::new(),
            player_cargo_unlock_raw: INITIAL_PLAYER_CARGO_CAPACITY as u8,
            beam_timer_ms: 0,
            factory_converted_output_births: Vec::new(),
            factory_output_materialiser_births: Vec::new(),
            cargo_drop_proxies: Vec::new(),
            cargo_proxy_events: Vec::new(),
            hover_physics: HoverPhysicsConfig::default(),
            environment_physics: CommonEnvironmentPhysics {
                runtime_wind_mode: 0,
                drag_strength: 0,
                ..CommonEnvironmentPhysics::default()
            },
        }
    }

    #[test]
    fn absent_player_world_uses_the_same_dynamic_damage_and_impulse() {
        let mut cinematic = manager(vec![entity(1, [64, 0, 0], 10_000)]);
        let mut with_hull = manager(vec![entity(1, [64, 0, 0], 10_000)]);
        let mut hull = PlayerHull::new(crate::player_hull::PLAYER_TYPE_46_HULL_PROFILE);
        let hull_before = hull;
        let cinematic_result = cinematic.apply_dynamic_radial_damage(
            DynamicRadialPlayerContext::Absent,
            [0; 3],
            TEMPLATE,
        );
        let with_hull_result = with_hull.apply_dynamic_radial_damage(
            DynamicRadialPlayerContext::Present(&mut hull),
            [0; 3],
            TEMPLATE,
        );
        assert!(matches!(
            cinematic_result,
            DynamicRadialDamageOutcome::Applied(_)
        ));
        assert_eq!(cinematic_result, with_hull_result);
        assert_eq!(
            cinematic.entities[0].velocity_raw(),
            with_hull.entities[0].velocity_raw()
        );
        assert_eq!(
            cinematic.entities[0].collision,
            with_hull.entities[0].collision
        );
        assert_eq!(hull, hull_before);
    }

    #[test]
    fn absent_hull_rejects_a_world_that_has_a_player_before_any_damage() {
        let mut entities = manager(vec![entity(1, [64, 0, 0], 10_000)]);
        entities.player_id = Some(1);
        let before = entities.entities[0].collision.clone();
        assert_eq!(
            entities.apply_dynamic_radial_damage(
                DynamicRadialPlayerContext::Absent,
                [0; 3],
                TEMPLATE,
            ),
            DynamicRadialDamageOutcome::Aborted(DynamicRadialUnresolved {
                target_id: 1,
                reason: DynamicRadialUnresolvedReason::PlayerHullUnavailable,
            })
        );
        assert_eq!(entities.entities[0].collision, before);
        assert_eq!(entities.entities[0].velocity_raw(), [0; 3]);
    }

    #[test]
    fn filtered_zero_feedback_requires_the_player_source_word_and_packet_gate() {
        for (source, channels, amount, feedback) in [
            (34, [1, 3], 1_000, false),
            (61, [1, 3], 1_000, false),
            (46, [1, 3], 1_000, true),
            (46, [1, 3], 0, true),
            (46, [1, 0], 1_000, false),
            (46, [1, 0], 0, false),
            (46, [3, 0], 0, true),
        ] {
            let mut target = entity(1, [64, 0, 0], 10_000);
            target.capability_flags = 8;
            target.collision.damage_profile = RetailRuntimeValue::Known(DamageProfile {
                thresholds_raw: [i32::MAX; 7],
                multipliers_q8: [256; 7],
            });
            let mut entities = manager(vec![target]);
            let template = RadialDamageTemplate {
                packet: DamagePacket {
                    channels,
                    amounts_raw: [amount, 1_000],
                },
                trailing_raw: [source, 0],
                ..TEMPLATE
            };
            let outcome = entities.apply_dynamic_radial_damage(
                DynamicRadialPlayerContext::Absent,
                [0; 3],
                template,
            );
            assert_eq!(
                matches!(
                    outcome,
                    DynamicRadialDamageOutcome::Aborted(DynamicRadialUnresolved {
                        reason: DynamicRadialUnresolvedReason::FilteredZeroFeedback,
                        ..
                    })
                ),
                feedback,
                "source={source} channels={channels:?} amount={amount}: {outcome:?}"
            );
            assert_eq!(
                entities.entities[0].collision.health_raw,
                RetailRuntimeValue::Known(10_000)
            );
            assert_eq!(
                entities.entities[0].velocity_raw(),
                if feedback { [0; 3] } else { [2_000, 0, 0] }
            );
        }
    }

    #[test]
    fn live_radial_retains_sound_and_buffer_before_a_type_hit_block() {
        let first = entity(1, [64, 0, 0], 10_000);
        let mut second = entity(2, [128, 0, 0], 10_000);
        second.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(3_000);
        second.collision.pair_callbacks.type_hit_callback_address = RetailRuntimeValue::Unresolved;
        let third = entity(3, [192, 0, 0], 10_000);
        let mut entities = manager(vec![first, second, third]);
        let mut world_fx = crate::world_fx::WorldFx::new();
        let mut notifications = crate::gameplay_notifications::GameplayNotifications::new();
        let result = entities.apply_dynamic_radial_damage_live(DynamicRadialLiveRequest {
            origin_raw: [0; 3],
            template: TEMPLATE,
            world_fx: &mut world_fx,
            retail_tick: 50,
            notifications: &mut notifications,
            callbacks: &mut |_: &EntityManager, _: u32| {
                panic!("synthetic targets are not native Type9")
            },
        });
        assert_eq!(result.accepted_targets, 2);
        assert_eq!(result.completed_target_ids, [1]);
        assert_eq!(
            result.blocked.as_ref().map(|block| (
                block.target_id,
                block.phase,
                block.target_prefix_committed
            )),
            Some((2, DynamicRadialLivePhase::TypeHit, true))
        );
        assert_eq!(
            entities.entities[0].collision.health_raw,
            RetailRuntimeValue::Known(8_000)
        );
        assert_eq!(
            entities.entities[1].collision.health_raw,
            RetailRuntimeValue::Known(10_000)
        );
        assert_eq!(
            entities.entities[1].collision.pre_health_damage_buffer_raw,
            RetailRuntimeValue::Known(1_000)
        );
        assert_eq!(entities.entities[1].velocity_raw(), [2_000, 0, 0]);
        assert_eq!(entities.entities[2].velocity_raw(), [0; 3]);
        world_fx.process_pending();
        let sounds = world_fx.take_positional_sounds();
        assert_eq!(
            sounds.len(),
            2,
            "full buffer absorption still reaches hit sound before +30"
        );
    }

    #[test]
    fn unresolved_later_target_rolls_back_earlier_impulse_damage_and_audio() {
        let first = entity(0, [64, 0, 0], 10_000);
        let mut second = entity(1, [128, 0, 0], 10_000);
        second.collision.pair_callbacks.damage_modifier_address = RetailRuntimeValue::Unresolved;
        let mut manager = manager(vec![first, second]);
        let before = manager.entities[0].velocity_raw();
        let mut hull = PlayerHull::default();

        assert_eq!(
            manager.apply_dynamic_radial_damage(
                DynamicRadialPlayerContext::Present(&mut hull),
                [0; 3],
                TEMPLATE
            ),
            DynamicRadialDamageOutcome::Aborted(DynamicRadialUnresolved {
                target_id: 1,
                reason: DynamicRadialUnresolvedReason::MissingRuntimeField(
                    DynamicRadialRuntimeField::DamageModifier,
                ),
            })
        );
        assert_eq!(manager.entities[0].velocity_raw(), before);
        assert_eq!(
            manager.entities[0].collision.health_raw,
            RetailRuntimeValue::Known(10_000)
        );
    }

    #[test]
    fn preflight_is_read_only_and_type61_shape_commits_without_new_gates() {
        let mut manager = manager(vec![
            entity(0, [0x200, 0, 0], 20_000),
            entity(1, [0x300, 0, 0], 20_000),
        ]);
        let mut hull = PlayerHull::default();
        let hull_before = hull;
        let before = manager
            .entities
            .iter()
            .map(|entity| (entity.velocity_raw(), entity.collision.clone()))
            .collect::<Vec<_>>();

        let plan = manager
            .preflight_dynamic_radial_damage(&hull, [0; 3], TYPE61_SHAPED_TEMPLATE)
            .expect("the complete dynamic walk is supported");

        assert_eq!(hull, hull_before);
        assert!(manager
            .entities
            .iter()
            .zip(&before)
            .all(
                |(entity, (velocity_raw, collision))| entity.velocity_raw() == *velocity_raw
                    && entity.collision == *collision
            ));

        // No runtime value is consulted after this boundary: commit consumes
        // the staged snapshots and has no unresolved result.
        let applied = manager.commit_preflighted_dynamic_radial_damage(Some(&mut hull), plan);
        assert_eq!(applied.accepted_targets, 2);
        assert_eq!(
            applied
                .sounds
                .iter()
                .map(|sound| sound.sound_id)
                .collect::<Vec<_>>(),
            vec![7, 7]
        );
        assert_eq!(manager.entities[0].velocity_raw(), [2_000, 0, 0]);
        assert_eq!(manager.entities[1].velocity_raw(), [1_000, 0, 0]);
        assert_eq!(
            manager.entities[0].collision.health_raw,
            RetailRuntimeValue::Known(12_000)
        );
        assert_eq!(
            manager.entities[1].collision.health_raw,
            RetailRuntimeValue::Known(16_000)
        );
        assert_eq!(hull, hull_before);
    }

    #[test]
    fn lethal_hive67_radial_uses_dying_initializer_instead_of_death_callback_policy() {
        let mut hive = entity(1, [64, 0, 0], 200);
        hive.entity_type = crate::hive_controller::HIVE_ENTITY_TYPE;
        hive.model_slots = [Some(341), Some(343), Some(341), Some(343)];
        hive.model_index = Some(341);
        hive.collision.state_flags_at_0x08 = RetailStateWord::exact(
            CHECKED_DAMAGE_ENABLED_STATE_BIT
                | RADIAL_DAMAGE_ALTERNATE_ELIGIBLE_STATE_BIT
                | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
        );
        hive.current_behavior_context = RetailRuntimeValue::Known(Some(
            BehaviorContextRuntime::named_audited(
                behavior_program(46).expect("Alien Hive"),
                0,
                RetailRuntimeValue::Unresolved,
                RetailRuntimeValue::Unresolved,
                RetailRuntimeValue::Unresolved,
                *audited_behavior_style(46, 0).expect("live hive style"),
            )
            .expect("class-46 variant 0"),
        ));
        let mut manager = manager(vec![hive]);
        let lethal = RadialDamageTemplate {
            packet: DamagePacket {
                channels: [1, 4],
                amounts_raw: [10_000, 8_000],
            },
            trailing_raw: [61, 0x1234],
            ..TYPE61_SHAPED_TEMPLATE
        };
        let outcome =
            manager.apply_dynamic_radial_damage(DynamicRadialPlayerContext::Absent, [0; 3], lethal);
        assert!(
            matches!(outcome, DynamicRadialDamageOutcome::Applied(_)),
            "Hive67 lethal radial must not roll back on DeathCallbackPolicy: {outcome:?}"
        );
        assert_eq!(
            manager.entities[0].collision.health_raw,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            manager.entities[0].collision.active_model_slot(),
            RetailRuntimeValue::Known(3)
        );
        assert_eq!(manager.entities[0].model_index, Some(343));
        assert_eq!(
            manager.entities[0].actor_task_state(ActorTaskSlot::Primary),
            Some(&ActorTaskRuntime::HiveRadial(HiveRadialTaskState::dying()))
        );
    }

    fn hive67_lethal_entity() -> Entity {
        let mut hive = entity(1, [64, 0, 0], 200);
        hive.entity_type = crate::hive_controller::HIVE_ENTITY_TYPE;
        hive.model_slots = [Some(341), Some(343), Some(341), Some(343)];
        hive.model_index = Some(341);
        hive.collision.state_flags_at_0x08 = RetailStateWord::exact(
            CHECKED_DAMAGE_ENABLED_STATE_BIT
                | RADIAL_DAMAGE_ALTERNATE_ELIGIBLE_STATE_BIT
                | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
        );
        hive.current_behavior_context = RetailRuntimeValue::Known(Some(
            BehaviorContextRuntime::named_audited(
                behavior_program(46).expect("Alien Hive"),
                0,
                RetailRuntimeValue::Unresolved,
                RetailRuntimeValue::Unresolved,
                RetailRuntimeValue::Unresolved,
                *audited_behavior_style(46, 0).expect("live hive style"),
            )
            .expect("class-46 variant 0"),
        ));
        hive
    }

    struct HiveBurstCallbacks {
        seen_model: Cell<Option<usize>>,
        extent: u16,
        sea: Option<i16>,
    }

    impl DynamicRadialLiveCallbacks for HiveBurstCallbacks {
        fn before_native_actor_mutation(&mut self, _: &EntityManager, _: u32) -> bool {
            true
        }
        fn hive_dying_burst_model_extent_raw(&self, model_id: usize) -> Option<u16> {
            self.seen_model.set(Some(model_id));
            Some(self.extent)
        }
        fn hive_dying_burst_sea_level_raw(&self) -> Option<i16> {
            self.sea
        }
    }

    struct SilentCapture;

    impl CaptureTaskCustody for SilentCapture {
        fn capture_child_mutation_ready(&mut self, _: &EntityManager, _: u32) -> bool {
            true
        }
        fn mutate_capture_child(
            &mut self,
            _: &mut EntityManager,
            _: CaptureChildFrame<'_>,
            _: &mut dyn FnMut(&mut EntityManager) -> Result<(), CaptureBlock>,
        ) -> Result<(), CaptureBlock> {
            Ok(())
        }
    }

    #[test]
    fn live_hive67_radial_death_emits_fun_00440950_with_dying_model_extent_and_sea() {
        let mut manager = manager(vec![hive67_lethal_entity()]);
        let mut world_fx = crate::world_fx::WorldFx::new();
        let mut notifications = crate::gameplay_notifications::GameplayNotifications::new();
        let mut callbacks = HiveBurstCallbacks {
            seen_model: Cell::new(None),
            extent: 0x80,
            sea: Some(256),
        };
        let result = manager.apply_dynamic_radial_damage_live(DynamicRadialLiveRequest {
            origin_raw: [0; 3],
            template: RadialDamageTemplate {
                packet: DamagePacket {
                    channels: [1, 4],
                    amounts_raw: [10_000, 8_000],
                },
                trailing_raw: [61, 0x1234],
                ..TYPE61_SHAPED_TEMPLATE
            },
            world_fx: &mut world_fx,
            retail_tick: 50,
            notifications: &mut notifications,
            callbacks: &mut callbacks,
        });
        assert_eq!(result.completed_target_ids, [1], "{result:?}");
        assert_eq!(callbacks.seen_model.get(), Some(343));
        assert_eq!(manager.entities[0].model_index, Some(343));
        assert_ne!(
            manager.entities[0].collision.health_raw,
            RetailRuntimeValue::Known(200)
        );
        let classes: Vec<u8> = world_fx
            .test_particles_in_virgin_birth_order()
            .iter()
            .map(|particle| particle.source_class)
            .collect();
        assert!(
            classes.contains(&UNDERWATER_IMPACT_CLASSES[0]),
            "sea above hive Y must take the FUN_00440950 underwater tail: {classes:?}"
        );
        assert!(
            !classes.contains(&METEOR_SURFACE_CLASS),
            "None sea would emit class 18 instead: {classes:?}"
        );
    }

    #[test]
    fn capture_hive67_destination_emits_fun_00440950_with_dying_model_extent_and_sea() {
        let mut manager = manager(vec![hive67_lethal_entity()]);
        let mut world_fx = crate::world_fx::WorldFx::new();
        let mut notifications = crate::gameplay_notifications::GameplayNotifications::new();
        let mut tasks = SilentCapture;
        let seen_model = Cell::new(None);
        let lookup = |model_id: usize| {
            seen_model.set(Some(model_id));
            Some(0x80_u16)
        };
        apply_capture_pair_checked_damage(
            &mut manager,
            1,
            DamageDeliveryRecord {
                packet: DamagePacket {
                    channels: [1, 4],
                    amounts_raw: [10_000, 8_000],
                },
                source_entity_type_raw: 17,
                owner_handle: 2,
            },
            &mut CaptureContext {
                resources: None,
                tasks: &mut tasks,
                world_fx: &mut world_fx,
                notifications: &mut notifications,
                retail_tick: 50,
                result_screen:
                    crate::main_base_type9_abort::MainBaseType9ResultScreenState::NotShown,
                hive_dying: CaptureHiveDyingBurst {
                    model_extent_raw: Some(&lookup),
                    sea_level_raw: Some(256),
                },
            },
        )
        .expect("Hive67 capture destination must run the dying initializer");
        assert_eq!(seen_model.get(), Some(343));
        assert_eq!(manager.entities[0].model_index, Some(343));
        assert_ne!(
            manager.entities[0].collision.health_raw,
            RetailRuntimeValue::Known(200)
        );
        let classes: Vec<u8> = world_fx
            .test_particles_in_virgin_birth_order()
            .iter()
            .map(|particle| particle.source_class)
            .collect();
        assert!(
            classes.contains(&UNDERWATER_IMPACT_CLASSES[0]),
            "capture FUN_00440950 must pass sea into the underwater tail: {classes:?}"
        );
        assert!(
            !classes.contains(&METEOR_SURFACE_CLASS),
            "extent/sea hardcode 0/None would emit class 18: {classes:?}"
        );
    }

    #[test]
    fn staged_dying_collision_override_admits_class23_type61_source() {
        let mut source = entity(7, [64, 0, 0], 1_000);
        source.entity_type = 61;
        source.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(500);
        source.collision.constructor_sound_attachment_id_at_0x8c =
            RetailRuntimeValue::Known(Some(44));
        source.current_behavior_context = RetailRuntimeValue::Known(Some(
            BehaviorContextRuntime::from_infallible_initial_selection(BehaviorSelection {
                choice_index: 0,
                program: behavior_program(23).expect("Power Up behavior"),
            })
            .expect("class 23 has an identity-determined initializer"),
        ));
        let mut manager = manager(vec![source]);
        let mut hull = PlayerHull::default();
        let live_collision = manager.entities[0].collision.clone();

        assert_eq!(
            manager
                .preflight_dynamic_radial_damage(&hull, [0; 3], TYPE61_SHAPED_TEMPLATE)
                .expect_err("the healthy source reaches unsupported generic death"),
            DynamicRadialUnresolved {
                target_id: 7,
                reason: DynamicRadialUnresolvedReason::DeathCallbackPolicy(
                    DeathCallbackPolicy::None,
                ),
            }
        );
        assert_eq!(manager.entities[0].collision, live_collision);

        // Type 61 emits after the generic-death prefix has zeroed health,
        // marked the source dying, and released its live +0x8C attachment.
        let mut staged_collision = live_collision.clone();
        staged_collision.health_raw = RetailRuntimeValue::Known(0);
        staged_collision
            .state_flags_at_0x08
            .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
        staged_collision.constructor_sound_attachment_id_at_0x8c = RetailRuntimeValue::Known(None);

        let plan = manager
            .preflight_dynamic_radial_damage_with_collision_override(
                &hull,
                [0; 3],
                TYPE61_SHAPED_TEMPLATE,
                7,
                &staged_collision,
            )
            .expect("the staged source takes the already-dying radial path");

        // Preflight is still read-only, including ownership of the borrowed
        // prefix snapshot.
        assert_eq!(manager.entities[0].collision, live_collision);
        assert_eq!(
            staged_collision.pre_health_damage_buffer_raw,
            RetailRuntimeValue::Known(500)
        );

        let applied = manager.commit_preflighted_dynamic_radial_damage(Some(&mut hull), plan);
        assert_eq!(applied.accepted_targets, 1);
        assert!(applied.sounds.is_empty());
        assert_eq!(manager.entities[0].velocity_raw(), [2_000, 0, 0]);
        assert_eq!(
            manager.entities[0].collision.health_raw,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            manager.entities[0].collision.pre_health_damage_buffer_raw,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            manager.entities[0]
                .collision
                .state_flags_at_0x08
                .masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(DYING_STATE_BIT)
        );
        assert_eq!(
            manager.entities[0]
                .collision
                .constructor_sound_attachment_id_at_0x8c,
            RetailRuntimeValue::Known(None)
        );
    }

    #[test]
    fn collision_override_authenticates_exactly_one_live_target_id() {
        let hull = PlayerHull::default();
        let override_collision = entity(99, [0; 3], 1_000).collision;
        let missing = manager(vec![entity(1, [64, 0, 0], 1_000)]);
        assert_eq!(
            missing
                .preflight_dynamic_radial_damage_with_collision_override(
                    &hull,
                    [0; 3],
                    TEMPLATE,
                    99,
                    &override_collision,
                )
                .expect_err("an override cannot name an absent allocation"),
            DynamicRadialCollisionOverrideError::TargetMissing { target_id: 99 }
        );

        let duplicate = manager(vec![
            entity(7, [64, 0, 0], 1_000),
            entity(7, [128, 0, 0], 1_000),
        ]);
        assert_eq!(
            duplicate
                .preflight_dynamic_radial_damage_with_collision_override(
                    &hull,
                    [0; 3],
                    TEMPLATE,
                    7,
                    &override_collision,
                )
                .expect_err("an override cannot ambiguously name two allocations"),
            DynamicRadialCollisionOverrideError::TargetDuplicate { target_id: 7 }
        );
    }

    #[test]
    fn unresolved_player_sub_j_context_aborts_before_radial_mutation() {
        let mut player = entity(0, [64, 0, 0], 10_000);
        player.kind = EntityKind::Player;
        player.entity_type = PLAYER_ENTITY_TYPE;
        player.collision.pair_callbacks = EntityPairCallbackRuntimeState::audited_player();
        player.sub_j_attachment_runtime = RetailRuntimeValue::Unresolved;
        let mut manager = manager(vec![player]);
        manager.player_id = Some(0);
        let velocity_before = manager.entities[0].velocity_raw();
        let collision_before = manager.entities[0].collision.clone();
        let mut hull = PlayerHull::default();
        hull.health_raw = 10_000;
        let hull_before = hull;

        assert_eq!(
            manager.apply_dynamic_radial_damage(
                DynamicRadialPlayerContext::Present(&mut hull),
                [0; 3],
                TEMPLATE
            ),
            DynamicRadialDamageOutcome::Aborted(DynamicRadialUnresolved {
                target_id: 0,
                reason: DynamicRadialUnresolvedReason::MissingRuntimeField(
                    DynamicRadialRuntimeField::DamageModifierContext,
                ),
            })
        );
        assert_eq!(manager.entities[0].velocity_raw(), velocity_before);
        assert_eq!(manager.entities[0].collision, collision_before);
        assert_eq!(hull, hull_before);
    }

    #[test]
    fn pass_commits_live_order_impulse_damage_and_generic_audio() {
        let mut manager = manager(vec![
            entity(0, [64, 0, 0], 10_000),
            entity(1, [384, 0, 0], 10_000),
        ]);
        let mut hull = PlayerHull::default();
        let outcome = manager.apply_dynamic_radial_damage(
            DynamicRadialPlayerContext::Present(&mut hull),
            [0; 3],
            TEMPLATE,
        );
        let DynamicRadialDamageOutcome::Applied(applied) = outcome else {
            panic!("resolved fixture must commit");
        };
        assert_eq!(applied.accepted_targets, 2);
        assert_eq!(
            applied
                .sounds
                .iter()
                .map(|sound| sound.sound_id)
                .collect::<Vec<_>>(),
            vec![7, 7]
        );
        assert_eq!(manager.entities[0].velocity_raw(), [2_000, 0, 0]);
        assert_eq!(
            manager.entities[0].collision.health_raw,
            RetailRuntimeValue::Known(8_000)
        );
        assert_eq!(
            manager.entities[1].collision.health_raw,
            RetailRuntimeValue::Known(9_000)
        );
    }

    #[test]
    fn ineligible_and_strict_outer_targets_need_no_callback_state() {
        let mut ineligible = entity(0, [64, 0, 0], 10_000);
        ineligible.collision.state_flags_at_0x08 =
            RetailStateWord::from_known_bits(0, CHECKED_DAMAGE_ENABLED_STATE_BIT);
        ineligible.collision.pair_callbacks.damage_modifier_address =
            RetailRuntimeValue::Unresolved;
        let mut outside = entity(1, [512, 0, 0], 10_000);
        outside.collision.state_flags_at_0x08 = RetailStateWord::unknown();
        let mut manager = manager(vec![ineligible, outside]);
        let mut hull = PlayerHull::default();

        assert_eq!(
            manager.apply_dynamic_radial_damage(
                DynamicRadialPlayerContext::Present(&mut hull),
                [0; 3],
                TEMPLATE
            ),
            DynamicRadialDamageOutcome::Applied(DynamicRadialApplied {
                accepted_targets: 0,
                sounds: Vec::new(),
                hive_dying_bursts: Vec::new(),
            })
        );
    }

    #[test]
    fn type_35_impulse_output_can_admit_zero_damage_but_suppresses_velocity_write() {
        let mut target = entity(0, [64, 0, 0], 10_000);
        target.entity_type = IMPULSE_SUPPRESSED_ENTITY_TYPE;
        let mut manager = manager(vec![target]);
        let mut hull = PlayerHull::default();
        let zero_damage = RadialDamageTemplate {
            packet: DamagePacket::default(),
            ..TEMPLATE
        };

        assert_eq!(
            manager.apply_dynamic_radial_damage(
                DynamicRadialPlayerContext::Present(&mut hull),
                [0; 3],
                zero_damage
            ),
            DynamicRadialDamageOutcome::Applied(DynamicRadialApplied {
                accepted_targets: 1,
                sounds: Vec::new(),
                hive_dying_bursts: Vec::new(),
            })
        );
        assert_eq!(manager.entities[0].velocity_raw(), [0; 3]);
        assert_eq!(
            manager.entities[0].collision.health_raw,
            RetailRuntimeValue::Known(10_000)
        );
    }

    #[test]
    fn lethal_player_damage_commits_hull_and_entity_state_together() {
        let mut player = entity(0, [64, 0, 0], 1_000);
        player.kind = EntityKind::Player;
        player.entity_type = PLAYER_ENTITY_TYPE;
        player.collision.pair_callbacks = EntityPairCallbackRuntimeState::audited_player();
        let mut manager = manager(vec![player]);
        manager.player_id = Some(0);
        let mut hull = PlayerHull::default();
        hull.health_raw = 1_000;

        let DynamicRadialDamageOutcome::Applied(applied) = manager.apply_dynamic_radial_damage(
            DynamicRadialPlayerContext::Present(&mut hull),
            [0; 3],
            TEMPLATE,
        ) else {
            panic!("audited empty-cargo player must commit");
        };
        assert_eq!(
            applied
                .sounds
                .iter()
                .map(|sound| sound.sound_id)
                .collect::<Vec<_>>(),
            vec![7, 9]
        );
        assert_eq!(hull.health_raw, 0);
        assert!(hull.dying);
        assert_eq!(
            manager.entities[0].collision.health_raw,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            manager.entities[0]
                .collision
                .state_flags_at_0x08
                .masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(DYING_STATE_BIT)
        );
    }

    #[test]
    fn lethal_factory_damage_starts_callback_owned_progression_atomically() {
        let mut factory = entity(0, [64, 0, 0], 1_000);
        factory.entity_type = FACTORY_ENTITY_TYPE;
        factory.current_behavior_context = RetailRuntimeValue::Known(Some(
            BehaviorContextRuntime::from_published_weighted_selection(BehaviorSelection {
                choice_index: 0,
                program: behavior_program(39).expect("Working Factory behavior"),
            })
            .expect("Working Factory is a weighted catalog behavior"),
        ));
        factory.base_factory_runtime = RetailRuntimeValue::Known(Some(BaseFactoryRuntimeState {
            status_descriptor: StatusComponentDescriptor {
                raw_word_at_0x00: 8,
                variable_bindings: [4, 3, 0, 2, 1, 0],
                raw_tail: [0; 10],
            },
            control_value_raw: 0,
            required_scientists: 3,
            current_scientists: 1,
            lifter_progress_raw: 0,
            production_progress_raw: 0,
            recovery_progress_raw: 0,
            production: None,
            live_owner: None,
            progressive_death: ProgressiveDeathState::idle(0),
        }));
        let mut manager = manager(vec![factory]);
        let mut hull = PlayerHull::default();

        let DynamicRadialDamageOutcome::Applied(applied) = manager.apply_dynamic_radial_damage(
            DynamicRadialPlayerContext::Present(&mut hull),
            [0; 3],
            TEMPLATE,
        ) else {
            panic!("audited factory callback must commit");
        };
        assert_eq!(
            applied
                .sounds
                .iter()
                .map(|sound| sound.sound_id)
                .collect::<Vec<_>>(),
            vec![7, 9]
        );
        assert_eq!(
            manager.entities[0].collision.health_raw,
            RetailRuntimeValue::Known(PROGRESSION_REVIVE_HEALTH_RAW)
        );
        assert_eq!(
            manager.entities[0]
                .collision
                .state_flags_at_0x08
                .masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(0)
        );
        let RetailRuntimeValue::Known(Some(runtime)) = manager.entities[0].base_factory_runtime
        else {
            panic!("factory runtime remains installed");
        };
        assert_eq!(runtime.progressive_death.elapsed_micros_raw, 1);
    }
}
