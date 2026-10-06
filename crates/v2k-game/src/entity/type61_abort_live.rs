//! Live composition for Type 61's Main-Base-abort class-49 continuation.
//!
//! The public contract and recovered constants live in
//! `main_base_type61_abort`; this module owns EntityManager's list, lifecycle,
//! radial-plan, and bounded class-49 presentation transactions. Shared Type-60
//! construction lives in `type60_exploding_ring_live`.

use super::{observe_main_base_abort_actor, Entity, EntityManager, PlayerHull};
use crate::actor_task_dispatcher::ActorTaskRuntime;
use crate::actor_task_owner::ActorTaskSlot;
use crate::entity_behavior::{
    behavior_program, ActiveBehaviorStyle, BehaviorChoiceListSource, BehaviorContextRuntime,
    BehaviorDescriptorIdentity, DeathCallbackPolicy, EXPLODE_WITH_RING_BEHAVIOR_PROGRAM,
    QUIET_DEATH_BEHAVIOR_PROGRAM,
};
use crate::entity_collision_state::{
    RetailRuntimeValue, DEFERRED_DESTROY_PENDING_STATE_BIT, DEFERRED_DESTROY_STATE_WRITE_MASK,
    DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT,
};
use crate::factory_production_live::{FactoryRequestedEntityHandle, FactoryType61BirthProvenance};
use crate::main_base_abort::{
    classify_main_base_abort_actor, MainBaseAbortActorFacts, MainBaseAbortActorLease,
    MainBaseAbortActorRoute, MainBaseDeferredDestroyEntry, MainBaseExternalDeferredDestroyOwner,
    ORDINARY_DEATH_STATE_SKIP_MASK,
};
use crate::main_base_type61_abort::{
    exact_level_one_type61_metadata, type61_radial_damage_template, MainBaseType60RingDeathAdvance,
    MainBaseType60RingDeathBlock, MainBaseType60RingDeathOutcome, MainBaseType61CapturedSpawn,
    MainBaseType61DeathAdvance, MainBaseType61DeathBlock, MainBaseType61DeathOutcome,
    MainBaseType61DeathRequest, MainBaseType61LogicalOwner, MainBaseType61NetworkDisposition,
    MainBaseType61NetworkSession, MainBaseType61StaticRadialEffects,
    FACTORY_TYPE61_NEWBORN_STATE_RAW, LEVEL_ONE_TYPE61_AUTHORED_MODEL_ID,
    LEVEL_ONE_TYPE61_CONSTRUCTOR_SOUND_ATTACHMENT_ID, LEVEL_ONE_TYPE61_DEATH_BEHAVIOR_CLASS,
    LEVEL_ONE_TYPE61_DEFAULT_MODEL_EXTENT_RAW, LEVEL_ONE_TYPE61_ENTITY_TYPE,
    LEVEL_ONE_TYPE61_INITIAL_BEHAVIOR_CLASS, LEVEL_ONE_TYPE61_INITIAL_HEALTH_RAW,
};

use crate::type60_exploding_ring::{
    exact_type60_ring_metadata, HostType60Allocator, Type60Allocator, Type60ConstructionProvenance,
    Type60ConstructionRequest, Type60ExplodingRingTaskLease, Type60InitializerDisposition,
    TYPE60_RING_ENTITY_TYPE, TYPE60_RING_FAILED_TASK_CONTROL_OUTPUT_1_RAW,
    TYPE60_RING_INITIAL_BEHAVIOR_CLASS, TYPE60_RING_INITIAL_CONTROL_OUTPUT_1_RAW,
    TYPE60_RING_INITIAL_HEALTH_RAW,
};
use crate::type60_exploding_ring_production::Type60ExplodingRingProductionOwner;
use crate::world_fx::WorldFx;
use v2k_formats::terrain::TerrainGrid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MainBaseType61ConstructionOrigin {
    Authored(MainBaseType61CapturedSpawn),
    Factory(FactoryType61BirthProvenance),
}

fn main_base_type61_construction_origin(
    entity: &Entity,
) -> Option<MainBaseType61ConstructionOrigin> {
    match (
        entity
            .authored_spawn_index
            .and_then(MainBaseType61CapturedSpawn::for_spawn),
        entity.factory_type61_birth_provenance(),
    ) {
        (Some(captured), None) => Some(MainBaseType61ConstructionOrigin::Authored(captured)),
        (None, Some(provenance)) => Some(MainBaseType61ConstructionOrigin::Factory(provenance)),
        (None, None) | (Some(_), Some(_)) => None,
    }
}

fn authenticate_type61_death_context(
    context: BehaviorContextRuntime,
) -> Option<BehaviorContextRuntime> {
    let initial = behavior_program(u32::from(LEVEL_ONE_TYPE61_INITIAL_BEHAVIOR_CLASS))?;
    if context.choice_list_source()
        != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        || context.descriptor() != BehaviorDescriptorIdentity::Named(initial)
        || context.active_style() != ActiveBehaviorStyle::Audited(initial.initial_style)
        || context.active_style().death_callback_policy() != DeathCallbackPolicy::None
        || EXPLODE_WITH_RING_BEHAVIOR_PROGRAM.class_id != LEVEL_ONE_TYPE61_DEATH_BEHAVIOR_CLASS
    {
        return None;
    }
    context.reselect_audited_type_default(
        &EXPLODE_WITH_RING_BEHAVIOR_PROGRAM,
        EXPLODE_WITH_RING_BEHAVIOR_PROGRAM.initial_style_table_index_raw,
        EXPLODE_WITH_RING_BEHAVIOR_PROGRAM.initial_style,
    )
}

fn type60_named_context_matches(context: BehaviorContextRuntime) -> bool {
    let Some(program) = behavior_program(u32::from(TYPE60_RING_INITIAL_BEHAVIOR_CLASS)) else {
        return false;
    };
    context.choice_list_source() == RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        && context.descriptor() == BehaviorDescriptorIdentity::Named(program)
        && context.active_style() == ActiveBehaviorStyle::Audited(program.initial_style)
        && context.active_style().death_callback_policy() == DeathCallbackPolicy::None
}

fn type60_fallback_context_matches(context: BehaviorContextRuntime) -> bool {
    context.choice_list_source() == RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        && context.descriptor() == BehaviorDescriptorIdentity::InitializerFailureFallback
        && context.active_style() == ActiveBehaviorStyle::InitializerFailureFallback
        && context.active_style().death_callback_policy() == DeathCallbackPolicy::None
}

fn type60_quiet_death_context(context: BehaviorContextRuntime) -> Option<BehaviorContextRuntime> {
    context.reselect_audited_type_default(
        &QUIET_DEATH_BEHAVIOR_PROGRAM,
        QUIET_DEATH_BEHAVIOR_PROGRAM.initial_style_table_index_raw,
        QUIET_DEATH_BEHAVIOR_PROGRAM.initial_style,
    )
}

impl EntityManager {
    /// Execute Type-61 generic-death/class-49.
    ///
    /// Authored Level-1 pickups still require the captured spawn table and the
    /// fresh first-world flag. Factory products authenticate the producing
    /// Type66 allocation identity in any ordinary world that constructed them.
    /// Both radial target domains are preflighted before the generic-death
    /// prefix. Commit then follows retail order: prefix/class-49 publication,
    /// particle and network-no-op bundle, static radial, dynamic radial,
    /// source task clears, fallible Type-60 tail construction, and deferred
    /// source destruction. Successor sampling happens only after the complete
    /// callback, so the final authored Type-61 can expose an earlier appended
    /// Type-60 tail to the same forward sweep.
    pub fn apply_main_base_abort_type61_death(
        &mut self,
        lease: MainBaseAbortActorLease,
        request: MainBaseType61DeathRequest,
        terrain: &TerrainGrid,
        player_hull: &mut PlayerHull,
        world_fx: &mut WorldFx,
        static_radial: &mut impl MainBaseType61StaticRadialEffects,
    ) -> Result<MainBaseType61DeathAdvance, MainBaseType61DeathBlock> {
        self.apply_main_base_abort_type61_death_with_allocator(
            lease,
            request,
            terrain,
            player_hull,
            world_fx,
            static_radial,
            &mut HostType60Allocator,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_main_base_abort_type61_death_with_allocator(
        &mut self,
        lease: MainBaseAbortActorLease,
        request: MainBaseType61DeathRequest,
        terrain: &TerrainGrid,
        player_hull: &mut PlayerHull,
        world_fx: &mut WorldFx,
        static_radial: &mut impl MainBaseType61StaticRadialEffects,
        allocator: &mut impl Type60Allocator,
    ) -> Result<MainBaseType61DeathAdvance, MainBaseType61DeathBlock> {
        let entity_id = lease.entity_id;
        let Some(entity_index) = self
            .entities
            .iter()
            .position(|entity| entity.id == entity_id)
        else {
            return Err(MainBaseType61DeathBlock::EntityMissing);
        };
        let entity = &self.entities[entity_index];
        let expected_lease =
            observe_main_base_abort_actor(entity, self.allocation_generation).lease;
        if lease != expected_lease {
            return Err(MainBaseType61DeathBlock::ActorLeaseMismatch {
                expected: expected_lease,
                actual: lease,
            });
        }
        let route = classify_main_base_abort_actor(MainBaseAbortActorFacts {
            entity_id,
            entity_type: entity.entity_type,
            capability_flags: entity.capability_flags,
            state_flags: entity
                .collision
                .state_flags_at_0x08
                .masked(ORDINARY_DEATH_STATE_SKIP_MASK),
        });
        if route != (MainBaseAbortActorRoute::OrdinaryDeath { entity_id }) {
            return Err(MainBaseType61DeathBlock::NotOrdinaryRoute(route));
        }
        if entity.entity_type != LEVEL_ONE_TYPE61_ENTITY_TYPE {
            return Err(MainBaseType61DeathBlock::UnsupportedEntityType {
                actual: entity.entity_type,
            });
        }
        // Retail's generic-death no-ops precede all type/style/effect reads.
        let remote_owned = match entity
            .collision
            .state_flags_at_0x08
            .masked(REMOTE_OWNED_STATE_BIT)
        {
            RetailRuntimeValue::Known(value) => value != 0,
            RetailRuntimeValue::Unresolved => {
                return Err(MainBaseType61DeathBlock::RemoteOwnershipUnresolved)
            }
        };
        if remote_owned {
            return Ok(self.finish_type61_death(
                entity_id,
                MainBaseType61DeathOutcome::RemoteOwnedNoOp {
                    entity_id,
                    entity_type: entity.entity_type,
                },
            ));
        }
        let already_dying = match entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT) {
            RetailRuntimeValue::Known(value) => value != 0,
            RetailRuntimeValue::Unresolved => {
                return Err(MainBaseType61DeathBlock::DyingStateUnresolved)
            }
        };
        if already_dying {
            return Ok(self.finish_type61_death(
                entity_id,
                MainBaseType61DeathOutcome::AlreadyDyingNoOp {
                    entity_id,
                    entity_type: entity.entity_type,
                },
            ));
        }

        let Some(construction_origin) = main_base_type61_construction_origin(entity) else {
            return Err(MainBaseType61DeathBlock::UnauthenticatedSpawn {
                actual: entity.authored_spawn_index,
            });
        };
        if matches!(
            construction_origin,
            MainBaseType61ConstructionOrigin::Authored(_)
        ) && !self.fresh_new_game_first_world
        {
            return Err(MainBaseType61DeathBlock::NotFreshNewGameFirstWorld);
        }

        let deferred_destroy_entry = self
            .preflight_main_base_deferred_destroy_entry(
                entity_id,
                entity.collision.state_flags_at_0x08,
            )
            .map_err(MainBaseType61DeathBlock::DeferredDestroyCustody)?;
        if let MainBaseDeferredDestroyEntry::AlreadyPending(owner) = deferred_destroy_entry {
            if owner != MainBaseExternalDeferredDestroyOwner::Type61DeferredQueue {
                return Err(MainBaseType61DeathBlock::DeferredDestroyOwnerUnsupported(
                    owner,
                ));
            }
        }

        let metadata = self
            .type_metadata
            .get(LEVEL_ONE_TYPE61_ENTITY_TYPE as usize)
            .ok_or(MainBaseType61DeathBlock::TypeMetadataUnavailable)?;
        if !exact_level_one_type61_metadata(metadata) {
            return Err(MainBaseType61DeathBlock::TypeMetadataMismatch);
        }
        let ring_metadata = self
            .type_metadata
            .get(TYPE60_RING_ENTITY_TYPE as usize)
            .ok_or(MainBaseType61DeathBlock::RingTypeMetadataUnavailable)?;
        if !exact_type60_ring_metadata(ring_metadata) {
            return Err(MainBaseType61DeathBlock::RingTypeMetadataMismatch);
        }
        let (expected_model_slots, expected_active_model, expected_model_extent_raw) =
            match construction_origin {
                MainBaseType61ConstructionOrigin::Authored(captured) => {
                    let expected_preabort_state_raw = match deferred_destroy_entry {
                        MainBaseDeferredDestroyEntry::Unstaged => captured.preabort_state_raw,
                        MainBaseDeferredDestroyEntry::AlreadyPending(_) => {
                            (captured.preabort_state_raw & !DEFERRED_DESTROY_STATE_WRITE_MASK)
                                | DEFERRED_DESTROY_PENDING_STATE_BIT
                        }
                    };
                    if entity.position_raw() != captured.position_raw
                        || entity.power_up_payload_packed != Some(captured.power_up_payload_packed)
                        || entity
                            .rotation_heading_pitch_roll_raw()
                            .map(|word| word as u16)
                            != captured.rotation_raw
                        || entity.collision.pre_health_damage_buffer_raw
                            != RetailRuntimeValue::Known(captured.initial_damage_buffer_raw)
                        || entity.collision.state_flags_at_0x08
                            != crate::entity_collision_state::RetailStateWord::exact(
                                expected_preabort_state_raw,
                            )
                    {
                        return Err(MainBaseType61DeathBlock::CapturedSpawnStateMismatch);
                    }
                    (
                        captured.model_slots.map(Some),
                        captured.active_model_id,
                        captured.active_model_extent_raw,
                    )
                }
                MainBaseType61ConstructionOrigin::Factory(provenance) => {
                    let birth_request = provenance.spawn_request();
                    let source_factory = provenance.source_factory();
                    let expected_preabort_state_raw = match deferred_destroy_entry {
                        MainBaseDeferredDestroyEntry::Unstaged => FACTORY_TYPE61_NEWBORN_STATE_RAW,
                        MainBaseDeferredDestroyEntry::AlreadyPending(_) => {
                            (FACTORY_TYPE61_NEWBORN_STATE_RAW & !DEFERRED_DESTROY_STATE_WRITE_MASK)
                                | DEFERRED_DESTROY_PENDING_STATE_BIT
                        }
                    };
                    if birth_request.requested_handle != FactoryRequestedEntityHandle::Allocate
                        || birth_request.entity_type != LEVEL_ONE_TYPE61_ENTITY_TYPE
                        || entity.authored_spawn_index.is_some()
                        || entity.position_raw() != birth_request.position_raw
                        || entity.power_up_payload_packed != Some(birth_request.spawn_parameter_6)
                        || entity.rotation_heading_pitch_roll_raw() != [0; 3]
                        || entity.collision.pre_health_damage_buffer_raw
                            != RetailRuntimeValue::Known(0)
                        || entity.collision.state_flags_at_0x08
                            != crate::entity_collision_state::RetailStateWord::exact(
                                expected_preabort_state_raw,
                            )
                        || entity.collision.recent_relation_id_at_0x60
                            != RetailRuntimeValue::Known(Some(source_factory.entity_id))
                        || !self.live_type66_matches_factory_birth_source(source_factory)
                    {
                        return Err(MainBaseType61DeathBlock::FactoryBirthProvenanceMismatch);
                    }
                    (
                        [Some(LEVEL_ONE_TYPE61_AUTHORED_MODEL_ID); 4],
                        LEVEL_ONE_TYPE61_AUTHORED_MODEL_ID,
                        LEVEL_ONE_TYPE61_DEFAULT_MODEL_EXTENT_RAW,
                    )
                }
            };
        if entity.model_slots != expected_model_slots
            || entity.collision.active_model_slot() != RetailRuntimeValue::Known(0)
            || entity.model_index != Some(expected_active_model)
            || request.active_model_extent_raw != expected_model_extent_raw
        {
            return Err(MainBaseType61DeathBlock::ActiveModelMismatch);
        }
        if entity.mass_raw != metadata.mass_raw
            || entity.capability_flags != metadata.capability_flags
            || entity.collision.health_raw
                != RetailRuntimeValue::Known(LEVEL_ONE_TYPE61_INITIAL_HEALTH_RAW)
            || entity.collision.damage_profile
                != RetailRuntimeValue::Known(metadata.damage_profile.unwrap())
            || entity.collision.death_sound_id != RetailRuntimeValue::Known(None)
            || entity.collision.generic_hit_sound_id != RetailRuntimeValue::Known(None)
        {
            return Err(MainBaseType61DeathBlock::TypeMetadataMismatch);
        }
        let RetailRuntimeValue::Known(Some(initial_behavior)) = entity.initial_behavior else {
            return Err(MainBaseType61DeathBlock::InitialBehaviorMismatch);
        };
        if initial_behavior.choice_index != 0
            || initial_behavior.program.class_id != LEVEL_ONE_TYPE61_INITIAL_BEHAVIOR_CLASS
        {
            return Err(MainBaseType61DeathBlock::InitialBehaviorMismatch);
        }
        let context = match entity.current_behavior_context {
            RetailRuntimeValue::Known(Some(context)) => context,
            RetailRuntimeValue::Known(None) => {
                return Err(MainBaseType61DeathBlock::CurrentBehaviorContextAbsent)
            }
            RetailRuntimeValue::Unresolved => {
                return Err(MainBaseType61DeathBlock::CurrentBehaviorContextMismatch)
            }
        };
        let death_context = authenticate_type61_death_context(context)
            .ok_or(MainBaseType61DeathBlock::CurrentBehaviorContextMismatch)?;
        if ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .any(|slot| entity.actor_task_state(slot).is_some())
        {
            return Err(MainBaseType61DeathBlock::UnexpectedPublishedTask);
        }
        if entity.collision.constructor_sound_attachment_id_at_0x8c
            != RetailRuntimeValue::Known(Some(LEVEL_ONE_TYPE61_CONSTRUCTOR_SOUND_ATTACHMENT_ID))
        {
            return Err(MainBaseType61DeathBlock::ConstructorSoundAttachmentStateMismatch);
        }
        let logical_owner = match entity.collision.recent_relation_id_at_0x60 {
            RetailRuntimeValue::Unresolved => {
                return Err(MainBaseType61DeathBlock::LogicalOwnerUnresolved)
            }
            RetailRuntimeValue::Known(None) => MainBaseType61LogicalOwner {
                entity_id,
                entity_type: entity.entity_type,
            },
            RetailRuntimeValue::Known(Some(owner_id)) => {
                let mut owners = self
                    .entities
                    .iter()
                    .filter(|candidate| candidate.id == owner_id);
                match (owners.next(), owners.next()) {
                    (Some(owner), None) => MainBaseType61LogicalOwner {
                        entity_id: owner.id,
                        entity_type: owner.entity_type,
                    },
                    // FUN_00416F90 treats a dangling relation as absent and
                    // falls back to the source. Duplicate ids violate the
                    // port's allocation invariant and remain ambiguous.
                    (None, None) => MainBaseType61LogicalOwner {
                        entity_id,
                        entity_type: entity.entity_type,
                    },
                    _ => return Err(MainBaseType61DeathBlock::LogicalOwnerUnavailable),
                }
            }
        };
        if request.logical_owner != logical_owner {
            return Err(MainBaseType61DeathBlock::LogicalOwnerMismatch {
                expected: logical_owner,
                actual: request.logical_owner,
            });
        }
        if request.network_session != MainBaseType61NetworkSession::SoloNetworkingDisabled {
            return Err(MainBaseType61DeathBlock::NetworkSessionUnsupported(
                request.network_session,
            ));
        }
        let expected_sea_level_raw = (terrain.header[0] >> 8) as i16;
        if request.sea_level_raw != Some(expected_sea_level_raw) {
            return Err(MainBaseType61DeathBlock::SeaLevelMismatch {
                expected: expected_sea_level_raw,
                actual: request.sea_level_raw,
            });
        }
        let position_raw = entity.position_raw();
        let radial = type61_radial_damage_template(logical_owner);
        let mut staged_source_collision = entity.collision.clone();
        staged_source_collision.health_raw = RetailRuntimeValue::Known(0);
        staged_source_collision
            .state_flags_at_0x08
            .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
        staged_source_collision.constructor_sound_attachment_id_at_0x8c =
            RetailRuntimeValue::Known(None);
        // The accepted Type-61 pair/runtime evidence fixes both generic damage
        // callbacks as null. Component contact/orientation remain independent
        // and are not consulted by radial delivery.
        staged_source_collision
            .pair_callbacks
            .damage_modifier_address = RetailRuntimeValue::Known(None);
        staged_source_collision
            .pair_callbacks
            .damage_modifier_identity_context_empty = RetailRuntimeValue::Known(true);
        staged_source_collision
            .pair_callbacks
            .type_hit_callback_address = RetailRuntimeValue::Known(None);
        let dynamic_plan = self
            .preflight_dynamic_radial_damage_with_collision_override(
                player_hull,
                position_raw,
                radial,
                entity_id,
                &staged_source_collision,
            )
            .map_err(|_| MainBaseType61DeathBlock::DynamicRadialDamagePreflightUnresolved)?;
        let static_plan = static_radial
            .preflight_static_radial(position_raw, radial)
            .ok_or(MainBaseType61DeathBlock::StaticRadialDamagePreflightUnresolved)?;

        // Generic death plus the direct class-49 alternate (no selector RNG).
        let source = &mut self.entities[entity_index];
        source.collision.health_raw = RetailRuntimeValue::Known(0);
        source
            .collision
            .state_flags_at_0x08
            .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
        source.collision.constructor_sound_attachment_id_at_0x8c = RetailRuntimeValue::Known(None);
        source.current_behavior_context = RetailRuntimeValue::Known(Some(death_context));

        world_fx.emit_explode_with_ring_burst_raw(crate::world_fx::ExplodeWithRingBurstRequest {
            position_raw: position_raw,
            source_extent_raw: request.active_model_extent_raw,
            sea_level_raw: request.sea_level_raw,
            logical_owner_entity_id: logical_owner.entity_id,
            logical_owner_entity_type: logical_owner.entity_type as u8,
            scatter_count: 16,
            scatter_classes: [94, 95],
            suppresses_impact_damage: false,
        });
        let network = MainBaseType61NetworkDisposition::Kind2SuppressedBySoloSession;
        static_radial.commit_static_radial(world_fx, static_plan);
        let dynamic_applied =
            self.commit_preflighted_dynamic_radial_damage(Some(player_hull), dynamic_plan);
        for sound in &dynamic_applied.sounds {
            world_fx.queue_fixed_positional_sound_raw(sound.sound_id, sound.position_raw);
        }
        let sea_level_raw = terrain.water_enabled().then(|| terrain.sea_level_raw());
        for burst in dynamic_applied.hive_dying_bursts {
            crate::hive_death::emit_hive_dying_surface_burst(
                world_fx,
                burst.position_raw,
                static_radial
                    .hive_dying_burst_model_extent_raw(burst.dying_model)
                    .unwrap_or(0),
                sea_level_raw,
                burst.entity_id,
            );
        }

        let source = &mut self.entities[entity_index];
        for slot in [
            ActorTaskSlot::Primary,
            ActorTaskSlot::Secondary,
            ActorTaskSlot::Tertiary,
        ] {
            source.actor_tasks.clear_slot(slot);
        }
        let ring = self.construct_type60_exploding_ring_with_allocator(
            Type60ConstructionRequest::class49_at(position_raw),
            terrain,
            world_fx,
            allocator,
        );
        let deferred_destroy =
            self.commit_main_base_deferred_destroy(entity_id, deferred_destroy_entry);

        Ok(self.finish_type61_death(
            entity_id,
            MainBaseType61DeathOutcome::ExplodeWithRingCommitted {
                entity_id,
                released_constructor_sound_attachment_id:
                    LEVEL_ONE_TYPE61_CONSTRUCTOR_SOUND_ATTACHMENT_ID,
                request,
                network,
                radial,
                dynamic_radial_accepted_targets: dynamic_applied.accepted_targets,
                ring,
                deferred_destroy,
            },
        ))
    }

    /// Execute generic death plus class 2 for one runtime Type-60 actor.
    /// Same-sweep class-49 tails and older hard-water rings have distinct
    /// provenance/custody shapes, while successful and fallback initializers
    /// retain distinct context/task/control shapes until this boundary.
    pub fn apply_main_base_abort_type60_ring_death(
        &mut self,
        lease: MainBaseAbortActorLease,
        scheduler_owner: Option<&Type60ExplodingRingProductionOwner>,
        world_fx: &mut WorldFx,
    ) -> Result<MainBaseType60RingDeathAdvance, MainBaseType60RingDeathBlock> {
        let entity_id = lease.entity_id;
        let Some(entity_index) = self
            .entities
            .iter()
            .position(|entity| entity.id == entity_id)
        else {
            return Err(MainBaseType60RingDeathBlock::EntityMissing);
        };
        let entity = &self.entities[entity_index];
        let expected_lease =
            observe_main_base_abort_actor(entity, self.allocation_generation).lease;
        if lease != expected_lease {
            return Err(MainBaseType60RingDeathBlock::ActorLeaseMismatch {
                expected: expected_lease,
                actual: lease,
            });
        }
        let route = classify_main_base_abort_actor(MainBaseAbortActorFacts {
            entity_id,
            entity_type: entity.entity_type,
            capability_flags: entity.capability_flags,
            state_flags: entity
                .collision
                .state_flags_at_0x08
                .masked(ORDINARY_DEATH_STATE_SKIP_MASK),
        });
        if route != (MainBaseAbortActorRoute::OrdinaryDeath { entity_id }) {
            return Err(MainBaseType60RingDeathBlock::NotOrdinaryRoute(route));
        }
        if entity.entity_type != TYPE60_RING_ENTITY_TYPE || entity.authored_spawn_index.is_some() {
            return Err(MainBaseType60RingDeathBlock::UnsupportedEntityType {
                actual: entity.entity_type,
            });
        }
        let provenance = entity
            .type60_construction_provenance()
            .ok_or(MainBaseType60RingDeathBlock::ConstructionProvenanceMismatch)?;
        //10C10 and the class48 -> class2 cleanup are identical in every
        //world. Constructor provenance authenticates model selection; the
        //live task/owner checks below authenticate the callback lifetime.
        let remote_owned = match entity
            .collision
            .state_flags_at_0x08
            .masked(REMOTE_OWNED_STATE_BIT)
        {
            RetailRuntimeValue::Known(value) => value != 0,
            RetailRuntimeValue::Unresolved => {
                return Err(MainBaseType60RingDeathBlock::RemoteOwnershipUnresolved)
            }
        };
        if remote_owned {
            return Ok(self.finish_type60_ring_death(
                entity_id,
                MainBaseType60RingDeathOutcome::RemoteOwnedNoOp {
                    entity_id,
                    entity_type: entity.entity_type,
                },
            ));
        }
        let already_dying = match entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT) {
            RetailRuntimeValue::Known(value) => value != 0,
            RetailRuntimeValue::Unresolved => {
                return Err(MainBaseType60RingDeathBlock::DyingStateUnresolved)
            }
        };
        if already_dying {
            return Ok(self.finish_type60_ring_death(
                entity_id,
                MainBaseType60RingDeathOutcome::AlreadyDyingNoOp {
                    entity_id,
                    entity_type: entity.entity_type,
                },
            ));
        }
        let metadata = self
            .type_metadata
            .get(TYPE60_RING_ENTITY_TYPE as usize)
            .ok_or(MainBaseType60RingDeathBlock::TypeMetadataUnavailable)?;
        if !exact_type60_ring_metadata(metadata) {
            return Err(MainBaseType60RingDeathBlock::TypeMetadataMismatch);
        }
        let expected_model_id = provenance.presentation_model_id();
        if entity.model_slots != [Some(expected_model_id); 4]
            || entity.model_index != Some(expected_model_id)
        {
            return Err(MainBaseType60RingDeathBlock::LiveModelMismatch);
        }
        if entity.mass_raw != metadata.mass_raw
            || entity.capability_flags != metadata.capability_flags
            || entity.collision.health_raw
                != RetailRuntimeValue::Known(TYPE60_RING_INITIAL_HEALTH_RAW)
            || entity.collision.pre_health_damage_buffer_raw != RetailRuntimeValue::Known(0)
            || entity.collision.constructor_sound_attachment_id_at_0x8c
                != RetailRuntimeValue::Known(None)
            || entity.actor_common_axis_descriptor
                != RetailRuntimeValue::Known(
                    metadata
                        .initializer
                        .as_ref()
                        .expect("exact Type-60 metadata has an initializer")
                        .common_axis_descriptor,
                )
        {
            return Err(MainBaseType60RingDeathBlock::ConstructorStateMismatch);
        }
        let RetailRuntimeValue::Known(Some(initial_behavior)) = entity.initial_behavior else {
            return Err(MainBaseType60RingDeathBlock::ConstructorStateMismatch);
        };
        if initial_behavior.choice_index != 0
            || initial_behavior.program.class_id != TYPE60_RING_INITIAL_BEHAVIOR_CLASS
        {
            return Err(MainBaseType60RingDeathBlock::ConstructorStateMismatch);
        }
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(MainBaseType60RingDeathBlock::CurrentBehaviorContextMismatch);
        };
        let quiet_context = type60_quiet_death_context(context)
            .ok_or(MainBaseType60RingDeathBlock::CurrentBehaviorContextMismatch)?;
        let associated = world_fx
            .exploding_rings()
            .iter()
            .filter(|ring| ring.associated_entity_id() == entity_id)
            .collect::<Vec<_>>();
        if associated.len() != 1
            || associated[0].model_id != expected_model_id
            || super::world_position_raw(associated[0].position) != entity.position_raw()
        {
            return Err(MainBaseType60RingDeathBlock::PresentationStateMismatch);
        }
        let initializer = if type60_named_context_matches(context) {
            let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) else {
                return Err(MainBaseType60RingDeathBlock::InitializerTaskStateMismatch);
            };
            let Some(ActorTaskRuntime::ExplodingRing(state)) =
                entity.actor_tasks.task_state(task_id)
            else {
                return Err(MainBaseType60RingDeathBlock::InitializerTaskStateMismatch);
            };
            if ActorTaskSlot::IN_RETAIL_TICK_ORDER.into_iter().any(|slot| {
                slot != ActorTaskSlot::Primary && entity.actor_task_state(slot).is_some()
            }) || associated[0].control_output_1_raw() != state.control_output_1_raw()
                || entity.actor_tasks.wrapper_flags(task_id)
                    != Some(crate::actor_task_owner::ActorTaskWrapperFlags {
                        alive: true,
                        in_callback: false,
                    })
            {
                return Err(MainBaseType60RingDeathBlock::InitializerTaskStateMismatch);
            }
            let task_lease = Type60ExplodingRingTaskLease::issue(lease, task_id);
            if let Some(scheduler_owner) = scheduler_owner {
                // Native BD20 registers its ring before this synchronous walk
                // reaches the appended tail. Older class49 and hard-water
                // rings retain the same exact progressed callback custody.
                if scheduler_owner.task_lease() != task_lease {
                    return Err(
                        MainBaseType60RingDeathBlock::SchedulerOwnerTaskLeaseMismatch {
                            expected: task_lease,
                            actual: scheduler_owner.task_lease(),
                        },
                    );
                }
                if scheduler_owner.next_callback_sequence() != state.next_callback_sequence() {
                    return Err(
                        MainBaseType60RingDeathBlock::SchedulerOwnerSequenceMismatch {
                            expected: state.next_callback_sequence(),
                            actual: scheduler_owner.next_callback_sequence(),
                        },
                    );
                }
            } else {
                match provenance {
                    Type60ConstructionProvenance::Class49ExplosionTail => {
                        // The captured Type61 adapter constructs an unscheduled
                        // same-sweep tail. Only its untouched initializer may
                        // enter without a production owner.
                        if state.next_callback_sequence() != 1
                            || state.control_output_1_raw()
                                != TYPE60_RING_INITIAL_CONTROL_OUTPUT_1_RAW
                        {
                            return Err(MainBaseType60RingDeathBlock::InitializerTaskStateMismatch);
                        }
                    }
                    Type60ConstructionProvenance::HardWaterModerate
                    | Type60ConstructionProvenance::HardWaterSevere => {
                        return Err(MainBaseType60RingDeathBlock::SchedulerOwnerMissing);
                    }
                }
            }
            Type60InitializerDisposition::PrimaryPublished {
                task_lease,
                control_output_1_raw: state.control_output_1_raw(),
            }
        } else if type60_fallback_context_matches(context) {
            if scheduler_owner.is_some() {
                return Err(MainBaseType60RingDeathBlock::SchedulerOwnerUnexpected);
            }
            if ActorTaskSlot::IN_RETAIL_TICK_ORDER
                .into_iter()
                .any(|slot| entity.actor_task_state(slot).is_some())
                || associated[0].control_output_1_raw()
                    != TYPE60_RING_FAILED_TASK_CONTROL_OUTPUT_1_RAW
            {
                return Err(MainBaseType60RingDeathBlock::InitializerTaskStateMismatch);
            }
            Type60InitializerDisposition::FallbackPublishedAfterPrimaryAllocationFailure {
                control_output_1_raw: TYPE60_RING_FAILED_TASK_CONTROL_OUTPUT_1_RAW,
            }
        } else {
            return Err(MainBaseType60RingDeathBlock::CurrentBehaviorContextMismatch);
        };
        match entity
            .collision
            .state_flags_at_0x08
            .masked(DEFERRED_DESTROY_PENDING_STATE_BIT)
        {
            RetailRuntimeValue::Known(0) => {}
            RetailRuntimeValue::Known(_) => {
                return Err(MainBaseType60RingDeathBlock::DeferredDestroyAlreadyPending)
            }
            RetailRuntimeValue::Unresolved => {
                return Err(MainBaseType60RingDeathBlock::DeferredDestroyStateUnresolved)
            }
        }
        if self.pending_actor_deferred_destroy_ids.contains(&entity_id) {
            return Err(MainBaseType60RingDeathBlock::DeferredDestroyAlreadyQueued);
        }

        let entity = &mut self.entities[entity_index];
        entity.collision.health_raw = RetailRuntimeValue::Known(0);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
        entity.current_behavior_context = RetailRuntimeValue::Known(Some(quiet_context));
        for slot in [
            ActorTaskSlot::Primary,
            ActorTaskSlot::Secondary,
            ActorTaskSlot::Tertiary,
        ] {
            entity.actor_tasks.clear_slot(slot);
        }
        let removed_ring = world_fx.remove_type60_exploding_ring(entity_id);
        debug_assert!(removed_ring);
        entity.mark_actor_deferred_destroy_pending();
        self.queue_actor_deferred_destroy(entity_id);
        Ok(self.finish_type60_ring_death(
            entity_id,
            MainBaseType60RingDeathOutcome::DeferredDestroyStaged {
                entity_id,
                initializer,
            },
        ))
    }

    fn finish_type61_death(
        &self,
        entity_id: u32,
        outcome: MainBaseType61DeathOutcome,
    ) -> MainBaseType61DeathAdvance {
        match self.main_base_abort_successor_after_callback(entity_id) {
            Ok(next_actor) => MainBaseType61DeathAdvance::Advanced {
                outcome,
                next_actor,
            },
            Err(()) => MainBaseType61DeathAdvance::SuccessorUnavailableAfterCommit { outcome },
        }
    }

    fn finish_type60_ring_death(
        &self,
        entity_id: u32,
        outcome: MainBaseType60RingDeathOutcome,
    ) -> MainBaseType60RingDeathAdvance {
        match self.main_base_abort_successor_after_callback(entity_id) {
            Ok(next_actor) => MainBaseType60RingDeathAdvance::Advanced {
                outcome,
                next_actor,
            },
            Err(()) => MainBaseType60RingDeathAdvance::SuccessorUnavailableAfterCommit { outcome },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::raw_position_world;
    use super::*;
    use crate::actor_task_owner::PreparedActorTask;
    use crate::damage::DamageProfile;
    use crate::entity_collision_state::{
        EntityCollisionRuntimeState, EntityInitializerSpec, EntityTypeRuntimeMetadata,
        RetailStateWord,
    };
    use crate::main_base_abort::MainBaseDeferredDestroyCommit;
    use crate::main_base_type61_abort::{
        LEVEL_ONE_TYPE61_AUTHORED_MODEL_ID, LEVEL_ONE_TYPE61_CAPABILITY_FLAGS,
        LEVEL_ONE_TYPE61_CAPTURED_SPAWNS, LEVEL_ONE_TYPE61_INITIALIZER_STATE_RAW,
        LEVEL_ONE_TYPE61_MASS_RAW, TYPE61_DAMAGE_PROFILE,
    };
    use crate::main_base_type66_abort::{
        LEVEL_ONE_TYPE66_ENTITY_TYPE, LEVEL_ONE_TYPE66_SPAWN_INDEX,
    };
    use crate::type60_exploding_ring::{
        Type60ConstructionOutcome, Type60ConstructorRngDisposition, Type60ExplodingRingTaskState,
        TYPE60_COMPONENT_TOPOLOGY, TYPE60_RING_ALTERNATE_BEHAVIOR_CLASS,
        TYPE60_RING_CAPABILITY_FLAGS, TYPE60_RING_COMMON_AXIS_DESCRIPTOR,
        TYPE60_RING_DAMAGE_PROFILE, TYPE60_RING_INITIALIZER_STATE_RAW, TYPE60_RING_MASS_RAW,
        TYPE60_RING_MODEL_ID,
    };
    use v2k_formats::collision::{BehaviorChoice, CommonAxisDescriptor};
    use v2k_formats::levels::{EntitySpawn, LevelDescriptor};
    use v2k_formats::terrain::{TerrainCell, GRID_SIZE};

    struct StaticRadialRecorder {
        preflight_ok: bool,
        preflights: Vec<([i16; 3], crate::radial_damage::RadialDamageTemplate)>,
        commits: Vec<([i16; 3], crate::radial_damage::RadialDamageTemplate)>,
        commit_rng_words: Vec<u16>,
        hive_dying_model_extents: Vec<(usize, u16)>,
    }

    impl StaticRadialRecorder {
        fn accepting() -> Self {
            Self {
                preflight_ok: true,
                preflights: Vec::new(),
                commits: Vec::new(),
                commit_rng_words: Vec::new(),
                hive_dying_model_extents: Vec::new(),
            }
        }

        fn with_hive_dying_extent(mut self, model_id: usize, extent_raw: u16) -> Self {
            self.hive_dying_model_extents.push((model_id, extent_raw));
            self
        }
    }

    impl MainBaseType61StaticRadialEffects for StaticRadialRecorder {
        type Prepared = ([i16; 3], crate::radial_damage::RadialDamageTemplate);

        fn preflight_static_radial(
            &mut self,
            origin_raw: [i16; 3],
            template: crate::radial_damage::RadialDamageTemplate,
        ) -> Option<Self::Prepared> {
            self.preflights.push((origin_raw, template));
            self.preflight_ok.then_some((origin_raw, template))
        }

        fn commit_static_radial(&mut self, world_fx: &mut WorldFx, prepared: Self::Prepared) {
            self.commits.push(prepared);
            self.commit_rng_words
                .push(world_fx.next_shared_retail_random_u16());
        }

        fn hive_dying_burst_model_extent_raw(&self, model_id: usize) -> Option<u16> {
            self.hive_dying_model_extents
                .iter()
                .find(|(id, _)| *id == model_id)
                .map(|(_, extent)| *extent)
        }
    }

    #[derive(Debug, PartialEq)]
    struct EntitySnapshot {
        position_raw: [i16; 3],
        velocity_raw: [i16; 3],
        collision: EntityCollisionRuntimeState,
        current_behavior_context: RetailRuntimeValue<Option<BehaviorContextRuntime>>,
        tasks: [Option<ActorTaskRuntime>; 3],
        model_index: Option<usize>,
    }

    fn snapshot(entity: &Entity) -> EntitySnapshot {
        EntitySnapshot {
            position_raw: entity.position_raw(),
            velocity_raw: entity.velocity_raw(),
            collision: entity.collision.clone(),
            current_behavior_context: entity.current_behavior_context,
            tasks: ActorTaskSlot::IN_RETAIL_TICK_ORDER
                .map(|slot| entity.actor_task_state(slot).copied()),
            model_index: entity.model_index,
        }
    }

    struct RejectBeforeSelector;

    impl Type60Allocator for RejectBeforeSelector {
        fn prepare_components(&mut self, _request: Type60ConstructionRequest) -> bool {
            false
        }

        fn prepare_behavior_context(&mut self) -> bool {
            panic!("pre-selector rejection must not prepare a context")
        }

        fn prepare_primary_task(
            &mut self,
            _state: Type60ExplodingRingTaskState,
        ) -> Option<PreparedActorTask<Type60ExplodingRingTaskState>> {
            panic!("pre-selector rejection must not prepare Primary")
        }
    }

    struct RejectAfterSelector;

    impl Type60Allocator for RejectAfterSelector {
        fn prepare_components(&mut self, _request: Type60ConstructionRequest) -> bool {
            true
        }

        fn prepare_behavior_context(&mut self) -> bool {
            false
        }

        fn prepare_primary_task(
            &mut self,
            _state: Type60ExplodingRingTaskState,
        ) -> Option<PreparedActorTask<Type60ExplodingRingTaskState>> {
            panic!("post-selector context rejection must not prepare Primary")
        }
    }

    struct RejectPrimary;

    impl Type60Allocator for RejectPrimary {
        fn prepare_components(&mut self, _request: Type60ConstructionRequest) -> bool {
            true
        }

        fn prepare_behavior_context(&mut self) -> bool {
            true
        }

        fn prepare_primary_task(
            &mut self,
            _state: Type60ExplodingRingTaskState,
        ) -> Option<PreparedActorTask<Type60ExplodingRingTaskState>> {
            None
        }
    }

    fn exact_metadata(entity_type: u32) -> EntityTypeRuntimeMetadata {
        let (
            model,
            mass,
            capability_flags,
            health,
            profile,
            attachment,
            initializer_state,
            common_axis,
            initial_behavior,
            alternate_behavior,
        ) = if entity_type == LEVEL_ONE_TYPE61_ENTITY_TYPE {
            (
                LEVEL_ONE_TYPE61_AUTHORED_MODEL_ID,
                LEVEL_ONE_TYPE61_MASS_RAW,
                LEVEL_ONE_TYPE61_CAPABILITY_FLAGS,
                LEVEL_ONE_TYPE61_INITIAL_HEALTH_RAW,
                TYPE61_DAMAGE_PROFILE,
                Some(LEVEL_ONE_TYPE61_CONSTRUCTOR_SOUND_ATTACHMENT_ID),
                LEVEL_ONE_TYPE61_INITIALIZER_STATE_RAW,
                CommonAxisDescriptor::default(),
                LEVEL_ONE_TYPE61_INITIAL_BEHAVIOR_CLASS,
                LEVEL_ONE_TYPE61_DEATH_BEHAVIOR_CLASS,
            )
        } else {
            (
                TYPE60_RING_MODEL_ID,
                TYPE60_RING_MASS_RAW,
                TYPE60_RING_CAPABILITY_FLAGS,
                TYPE60_RING_INITIAL_HEALTH_RAW,
                TYPE60_RING_DAMAGE_PROFILE,
                None,
                TYPE60_RING_INITIALIZER_STATE_RAW,
                TYPE60_RING_COMMON_AXIS_DESCRIPTOR,
                TYPE60_RING_INITIAL_BEHAVIOR_CLASS,
                TYPE60_RING_ALTERNATE_BEHAVIOR_CLASS,
            )
        };
        EntityTypeRuntimeMetadata {
            cured_model_presentation_sound_id: RetailRuntimeValue::Unresolved,
            common_world_effects: RetailRuntimeValue::Unresolved,
            detailed_sound_policy: RetailRuntimeValue::Unresolved,
            model_slots: [model as u16; 4],
            mass_raw: mass,
            capability_flags,
            initial_health_raw: Some(health),
            damage_profile: Some(profile),
            accepted_hit_presentation_sound_id: RetailRuntimeValue::Known(None),
            infected_model_presentation_sound_id: RetailRuntimeValue::Known(None),
            death_sound_id: RetailRuntimeValue::Known(None),
            target_warning_sound_id: RetailRuntimeValue::Unresolved,
            constructor_sound_attachment_id: RetailRuntimeValue::Known(attachment),
            generic_hit_sound_id: RetailRuntimeValue::Known(None),
            search_attack_optional_prelude_sound_id: RetailRuntimeValue::Unresolved,
            search_attack_aim_sound_id: RetailRuntimeValue::Unresolved,
            search_attack_aim_sound_period_raw: RetailRuntimeValue::Unresolved,
            run_away_optional_sound_id: RetailRuntimeValue::Unresolved,
            run_away_sound_period_raw: RetailRuntimeValue::Unresolved,
            terrain_contact_task_lifetime_ms: RetailRuntimeValue::Unresolved,
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(None),
            sub_b_lateral_descriptor: RetailRuntimeValue::Known(None),
            sub_c_lift_descriptor: RetailRuntimeValue::Known(None),
            sub_d_steering_descriptor: RetailRuntimeValue::Known(None),
            sub_f_swimming_descriptor: RetailRuntimeValue::Known(None),
            model_variable_count_raw: RetailRuntimeValue::Unresolved,
            projectile_emitter_descriptor: RetailRuntimeValue::Known(None),
            sub_n_payload: None,
            status_component_descriptor: RetailRuntimeValue::Known(None),
            actor_animation_descriptor: RetailRuntimeValue::Known(None),
            sub_h_external_frame_descriptor: RetailRuntimeValue::Known(None),
            sub_j_attachment_descriptor: RetailRuntimeValue::Known(None),
            common_mover_topology: RetailRuntimeValue::Known(TYPE60_COMPONENT_TOPOLOGY),
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: initializer_state,
                common_axis_descriptor: common_axis,
                behavior_choices: vec![BehaviorChoice {
                    weight_rule_id: 1,
                    weight_multiplier: 1,
                    behavior_class_id: u32::from(initial_behavior),
                }]
                .into_boxed_slice(),
                behavior_rule_ref: 1,
                alternate_behavior_class_ref: u32::from(alternate_behavior),
            }),
            common_mover_gkl_payloads: RetailRuntimeValue::Unresolved,
        }
    }

    fn exact_terrain() -> TerrainGrid {
        TerrainGrid {
            header: [-216_832, 0, 0, 0, 0],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0,
                };
                GRID_SIZE * GRID_SIZE
            ],
        }
    }

    fn spawn_from_capture(captured: MainBaseType61CapturedSpawn) -> EntitySpawn {
        let mut extra = [0; 40];
        extra[8..12].copy_from_slice(&captured.power_up_payload_packed.to_le_bytes());
        EntitySpawn {
            index: captured.spawn_index,
            entity_type: LEVEL_ONE_TYPE61_ENTITY_TYPE,
            pos_data_1: [
                captured.position_raw[0].to_le_bytes()[0],
                captured.position_raw[0].to_le_bytes()[1],
                captured.position_raw[1].to_le_bytes()[0],
                captured.position_raw[1].to_le_bytes()[1],
            ],
            pos_data_2: [
                captured.position_raw[2].to_le_bytes()[0],
                captured.position_raw[2].to_le_bytes()[1],
                0,
                0,
            ],
            param: 0,
            rotation: captured.rotation_raw,
            extra,
            initial_damage_buffer_raw: captured.initial_damage_buffer_raw,
            model_overrides: if captured.active_model_id == LEVEL_ONE_TYPE61_AUTHORED_MODEL_ID {
                [0; 4]
            } else {
                [captured.active_model_id as u32; 4]
            },
            has_animation: false,
            anim_frames: 0,
            animation: None,
            has_config: false,
            config: None,
        }
    }

    fn manager_and_terrain() -> (EntityManager, TerrainGrid) {
        let terrain = exact_terrain();
        let entities = LEVEL_ONE_TYPE61_CAPTURED_SPAWNS
            .into_iter()
            .map(spawn_from_capture)
            .collect::<Vec<_>>();
        let level = LevelDescriptor {
            raw_header: [0; 0xD0],
            name: "Type61 suffix fixture".into(),
            world_style: 0,
            terrain_sprite_base: 0,
            sky_color_index: 0,
            sky_model: 0,
            main_base_abort_sky_color_index: 0,
            main_base_abort_sky_model: 0,
            terrain_draw_depth: 30,
            sub_count: entities.len() as u32,
            campaign_record_count: 0,
            entities,
            campaign_records: Vec::new(),
        };
        let mut metadata = vec![EntityTypeRuntimeMetadata::default(); 62];
        metadata[TYPE60_RING_ENTITY_TYPE as usize] = exact_metadata(TYPE60_RING_ENTITY_TYPE);
        metadata[LEVEL_ONE_TYPE61_ENTITY_TYPE as usize] =
            exact_metadata(LEVEL_ONE_TYPE61_ENTITY_TYPE);
        let mut manager =
            EntityManager::from_level_with_type_metadata(&level, &metadata, Some(&terrain));
        manager.fresh_new_game_first_world = true;
        for (entity, captured) in manager
            .entities
            .iter_mut()
            .zip(LEVEL_ONE_TYPE61_CAPTURED_SPAWNS)
        {
            entity.collision.state_flags_at_0x08 =
                RetailStateWord::exact(captured.preabort_state_raw);
        }
        (manager, terrain)
    }

    fn request_for(
        manager: &EntityManager,
        entity_id: u32,
        terrain: &TerrainGrid,
    ) -> MainBaseType61DeathRequest {
        let entity = manager
            .entities
            .iter()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        let captured = MainBaseType61CapturedSpawn::for_spawn(
            entity
                .authored_spawn_index
                .expect("authored Type-61 fixture"),
        )
        .unwrap();
        MainBaseType61DeathRequest {
            active_model_extent_raw: captured.active_model_extent_raw,
            sea_level_raw: Some((terrain.header[0] >> 8) as i16),
            logical_owner: MainBaseType61LogicalOwner {
                entity_id,
                entity_type: entity.entity_type,
            },
            network_session: MainBaseType61NetworkSession::SoloNetworkingDisabled,
        }
    }

    #[test]
    fn three_type61_callbacks_append_real_type60_tails_and_the_sweep_reaches_them() {
        let (mut manager, terrain) = manager_and_terrain();
        let mut hull = PlayerHull::default();
        let mut world_fx = WorldFx::new();
        let mut static_radial = StaticRadialRecorder::accepting();
        let mut rng_oracle = WorldFx::new();

        let expected_burst_word = rng_oracle.next_shared_retail_random_u16();
        let expected_static_word = rng_oracle.next_shared_retail_random_u16();
        let expected_first_selector_word = rng_oracle.next_shared_retail_random_u16();
        let _ = expected_burst_word;

        for source_id in 1_u32..=3 {
            let actor = manager
                .main_base_abort_actor_observation(source_id)
                .unwrap();
            let request = request_for(&manager, source_id, &terrain);
            let MainBaseType61DeathAdvance::Advanced {
                outcome:
                    MainBaseType61DeathOutcome::ExplodeWithRingCommitted {
                        ring: Type60ConstructionOutcome::ActorLinked(receipt),
                        dynamic_radial_accepted_targets,
                        deferred_destroy,
                        ..
                    },
                next_actor,
            } = manager
                .apply_main_base_abort_type61_death(
                    actor.lease,
                    request,
                    &terrain,
                    &mut hull,
                    &mut world_fx,
                    &mut static_radial,
                )
                .expect("exact Type-61 callback")
            else {
                panic!("exact Type-61 publishes one Type-60 tail")
            };
            assert_eq!(dynamic_radial_accepted_targets, 1);
            assert_eq!(
                deferred_destroy,
                MainBaseDeferredDestroyCommit::StagedByCallback
            );
            assert_eq!(receipt.actor().entity_id, 3 + source_id);
            assert!(matches!(
                receipt.initializer(),
                Type60InitializerDisposition::PrimaryPublished {
                    control_output_1_raw: u16::MAX,
                    ..
                }
            ));
            if source_id == 1 {
                assert_eq!(
                    receipt.constructor_rng(),
                    Type60ConstructorRngDisposition::SelectorWord(expected_first_selector_word)
                );
                assert_eq!(static_radial.commit_rng_words, [expected_static_word]);
            }
            let expected_next = if source_id < 3 {
                Some((source_id + 1, LEVEL_ONE_TYPE61_ENTITY_TYPE))
            } else {
                Some((4, TYPE60_RING_ENTITY_TYPE))
            };
            assert_eq!(
                next_actor.map(|actor| (actor.lease.entity_id, actor.entity_type)),
                expected_next
            );
        }

        assert_eq!(manager.entities.len(), 6);
        assert_eq!(manager.pending_actor_deferred_destroy_ids(), &[1, 2, 3]);
        assert_eq!(world_fx.exploding_rings().len(), 3);
        assert!(manager.entities[..3].iter().all(|source| {
            source.collision.health_raw == RetailRuntimeValue::Known(0)
                && source.collision.constructor_sound_attachment_id_at_0x8c
                    == RetailRuntimeValue::Known(None)
        }));

        for ring_id in 4_u32..=6 {
            let actor = manager.main_base_abort_actor_observation(ring_id).unwrap();
            let MainBaseType60RingDeathAdvance::Advanced {
                outcome: MainBaseType60RingDeathOutcome::DeferredDestroyStaged { entity_id, .. },
                next_actor,
            } = manager
                .apply_main_base_abort_type60_ring_death(actor.lease, None, &mut world_fx)
                .expect("same sweep reaches the appended Type-60 actor")
            else {
                panic!("Type-60 class-2 death stays linked through callback return")
            };
            assert_eq!(entity_id, ring_id);
            assert_eq!(
                next_actor.map(|actor| actor.lease.entity_id),
                (ring_id < 6).then_some(ring_id + 1)
            );
        }
        assert!(world_fx.exploding_rings().is_empty());
        assert_eq!(
            manager.pending_actor_deferred_destroy_ids(),
            &[1, 2, 3, 4, 5, 6]
        );
    }

    #[test]
    fn factory_constructor_receipt_admits_type61_without_replaying_birth_rng() {
        let mut manager = crate::entity::terminal_abort_composition_manager();
        let terrain = exact_terrain();
        let factory = manager
            .entities
            .iter()
            .find(|entity| {
                entity.entity_type == LEVEL_ONE_TYPE66_ENTITY_TYPE
                    && entity.authored_spawn_index == Some(LEVEL_ONE_TYPE66_SPAWN_INDEX)
            })
            .expect("the exact abort fixture retains its Working Factory");
        let RetailRuntimeValue::Known(Some(factory_runtime)) = factory.base_factory_runtime else {
            panic!("the exact Working Factory retains its live owner")
        };
        let live_owner = factory_runtime
            .live_owner
            .expect("factory allocation identity");
        let source_factory = crate::factory_production_live::FactoryProductionEntityVersion {
            entity_id: factory.id,
            allocation_identity: live_owner.allocation_identity,
            state_version: live_owner.state_version,
        };
        let request = crate::factory_production_live::FactoryEntitySpawnRequest {
            requested_handle: FactoryRequestedEntityHandle::Allocate,
            entity_type: LEVEL_ONE_TYPE61_ENTITY_TYPE,
            position_raw: factory.position_raw(),
            spawn_parameter_6: factory_runtime
                .production
                .expect("exact factory production state")
                .output_payload_packed,
        };
        // The broad composition fixture intentionally collapses many
        // unrelated actors onto placeholder coordinates. Keep only the
        // factory at the real product origin so this focused callback does not
        // acquire unsupported radial targets that production would encounter
        // at their actual authored positions.
        for entity in &mut manager.entities {
            if entity.id != source_factory.entity_id {
                entity.position = raw_position_world([i16::MAX, 0, i16::MAX]);
            }
        }

        let mut world_fx = WorldFx::new();
        let mut rng_oracle = WorldFx::new();
        let expected_birth_word = rng_oracle.next_shared_retail_random_u16();
        let spawned = manager
            .append_factory_pickup(source_factory, request, &mut world_fx)
            .expect("exact factory request constructs one Type-61");
        assert!(manager.link_factory_pickup_owner(spawned, source_factory));
        let source_id = spawned.entity_id.get();
        manager
            .entities
            .iter_mut()
            .find(|entity| entity.id == source_factory.entity_id)
            .expect("source factory remains linked")
            .position = raw_position_world([i16::MAX, 0, i16::MAX]);
        let source = manager
            .entities
            .iter()
            .find(|entity| entity.id == source_id)
            .expect("factory product remains tail-linked");
        let provenance = source
            .factory_type61_birth_provenance()
            .expect("factory product retains its constructor receipt");
        assert_eq!(provenance.source_factory(), source_factory);
        assert_eq!(provenance.spawn_request(), request);
        assert_eq!(provenance.selector_rng_word(), expected_birth_word);
        assert_eq!(
            source.collision.state_flags_at_0x08,
            RetailStateWord::exact(FACTORY_TYPE61_NEWBORN_STATE_RAW)
        );

        // The callback starts at the next word: the retained birth word is
        // provenance only and must never be sampled again during abort.
        let expected_burst_word = rng_oracle.next_shared_retail_random_u16();
        let expected_static_word = rng_oracle.next_shared_retail_random_u16();
        let expected_ring_word = rng_oracle.next_shared_retail_random_u16();
        let actor = manager
            .main_base_abort_actor_observation(source_id)
            .expect("factory product receives an allocation-authenticated abort lease");
        let death_request = MainBaseType61DeathRequest {
            active_model_extent_raw: LEVEL_ONE_TYPE61_DEFAULT_MODEL_EXTENT_RAW,
            sea_level_raw: Some((terrain.header[0] >> 8) as i16),
            logical_owner: MainBaseType61LogicalOwner {
                entity_id: source_factory.entity_id,
                entity_type: LEVEL_ONE_TYPE66_ENTITY_TYPE,
            },
            network_session: MainBaseType61NetworkSession::SoloNetworkingDisabled,
        };
        let mut hull = PlayerHull::default();
        let mut static_radial = StaticRadialRecorder::accepting();
        let MainBaseType61DeathAdvance::Advanced {
            outcome:
                MainBaseType61DeathOutcome::ExplodeWithRingCommitted {
                    request: committed_request,
                    ring: Type60ConstructionOutcome::ActorLinked(ring),
                    deferred_destroy: MainBaseDeferredDestroyCommit::StagedByCallback,
                    ..
                },
            next_actor,
        } = manager
            .apply_main_base_abort_type61_death(
                actor.lease,
                death_request,
                &terrain,
                &mut hull,
                &mut world_fx,
                &mut static_radial,
            )
            .expect("receipt-backed factory Type-61 takes the shared class-49 path")
        else {
            panic!("factory Type-61 appends one real Type-60 tail")
        };
        assert_eq!(committed_request, death_request);
        assert_eq!(static_radial.commit_rng_words, [expected_static_word]);
        assert_eq!(
            ring.constructor_rng(),
            Type60ConstructorRngDisposition::SelectorWord(expected_ring_word)
        );
        assert_eq!(
            next_actor.map(|next| (next.lease.entity_id, next.entity_type)),
            Some((ring.actor().entity_id, TYPE60_RING_ENTITY_TYPE))
        );
        assert_eq!(
            world_fx.take_positional_sounds()[0].frequency_q16,
            0x1_0000 + u32::from(expected_burst_word >> 3)
        );
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            rng_oracle.next_shared_retail_random_u16()
        );
    }

    #[test]
    fn factory_type61_abort_uses_allocation_identity_outside_first_world() {
        let mut manager = crate::entity::terminal_abort_composition_manager();
        manager.fresh_new_game_first_world = false;
        let terrain = exact_terrain();
        let factory = manager
            .entities
            .iter()
            .find(|entity| entity.entity_type == LEVEL_ONE_TYPE66_ENTITY_TYPE)
            .expect("composition fixture retains a Working Factory");
        assert_ne!(
            factory.authored_spawn_index, None,
            "fixture factory still has an authored index; identity must not depend on it"
        );
        let RetailRuntimeValue::Known(Some(factory_runtime)) = factory.base_factory_runtime else {
            panic!("Working Factory live owner")
        };
        let live_owner = factory_runtime.live_owner.expect("factory allocation");
        let source_factory = crate::factory_production_live::FactoryProductionEntityVersion {
            entity_id: factory.id,
            allocation_identity: live_owner.allocation_identity,
            state_version: live_owner.state_version,
        };
        let request = crate::factory_production_live::FactoryEntitySpawnRequest {
            requested_handle: FactoryRequestedEntityHandle::Allocate,
            entity_type: LEVEL_ONE_TYPE61_ENTITY_TYPE,
            position_raw: factory.position_raw(),
            spawn_parameter_6: factory_runtime
                .production
                .expect("factory production")
                .output_payload_packed,
        };
        for entity in &mut manager.entities {
            if entity.id != source_factory.entity_id {
                entity.position = raw_position_world([i16::MAX, 0, i16::MAX]);
            }
        }
        let mut world_fx = WorldFx::new();
        let spawned = manager
            .append_factory_pickup(source_factory, request, &mut world_fx)
            .expect("factory constructor");
        assert!(manager.link_factory_pickup_owner(spawned, source_factory));
        manager
            .entities
            .iter_mut()
            .find(|entity| entity.id == source_factory.entity_id)
            .unwrap()
            .position = raw_position_world([i16::MAX, 0, i16::MAX]);
        let source_id = spawned.entity_id.get();
        let actor = manager
            .main_base_abort_actor_observation(source_id)
            .unwrap();
        let death_request = MainBaseType61DeathRequest {
            active_model_extent_raw: LEVEL_ONE_TYPE61_DEFAULT_MODEL_EXTENT_RAW,
            sea_level_raw: Some((terrain.header[0] >> 8) as i16),
            logical_owner: MainBaseType61LogicalOwner {
                entity_id: source_factory.entity_id,
                entity_type: LEVEL_ONE_TYPE66_ENTITY_TYPE,
            },
            network_session: MainBaseType61NetworkSession::SoloNetworkingDisabled,
        };
        let mut hull = PlayerHull::default();
        let mut static_radial = StaticRadialRecorder::accepting();
        let MainBaseType61DeathAdvance::Advanced {
            outcome: MainBaseType61DeathOutcome::ExplodeWithRingCommitted { ring, .. },
            next_actor,
        } = manager
            .apply_main_base_abort_type61_death(
                actor.lease,
                death_request,
                &terrain,
                &mut hull,
                &mut world_fx,
                &mut static_radial,
            )
            .expect("factory Type-61 abort is not a first-world spawn-index gate")
        else {
            panic!("factory Type-61 still appends a Type-60 tail")
        };
        let Type60ConstructionOutcome::ActorLinked(ring) = ring else {
            panic!("class-49 tail is linked")
        };
        assert_eq!(
            next_actor.map(|next| next.lease.entity_id),
            Some(ring.actor().entity_id)
        );
        manager.fresh_new_game_first_world = false;
        manager
            .apply_main_base_abort_type60_ring_death(ring.actor(), None, &mut world_fx)
            .expect("class-49 Type-60 tail follows the parent factory product");
    }

    #[test]
    fn factory_type61_abort_does_not_roll_back_on_in_range_live_hive67() {
        use crate::entity_behavior::{audited_behavior_style, BehaviorContextRuntime};
        use crate::entity_collision_state::{
            ACTIVE_MODEL_SLOT_HIGH_STATE_BIT, CHECKED_DAMAGE_ENABLED_STATE_BIT,
            RADIAL_DAMAGE_ALTERNATE_ELIGIBLE_STATE_BIT,
        };
        use crate::hive_controller::HIVE_ENTITY_TYPE;
        use crate::hive_death::HiveRadialTaskState;

        let mut manager = crate::entity::terminal_abort_composition_manager();
        manager.fresh_new_game_first_world = false;
        let terrain = exact_terrain();
        let hive_id = manager
            .entities
            .iter()
            .find(|entity| entity.entity_type == HIVE_ENTITY_TYPE)
            .expect("composition fixture retains the Alien Hive")
            .id;
        let hive_position = manager
            .entities
            .iter()
            .find(|entity| entity.id == hive_id)
            .unwrap()
            .position_raw();
        {
            let hive = manager
                .entities
                .iter_mut()
                .find(|entity| entity.id == hive_id)
                .unwrap();
            hive.model_slots = [Some(341), Some(343), Some(341), Some(343)];
            hive.model_index = Some(341);
            hive.collision.health_raw = RetailRuntimeValue::Known(200);
            hive.collision.generic_hit_sound_id = RetailRuntimeValue::Known(Some(7));
            hive.collision.damage_profile = RetailRuntimeValue::Known(DamageProfile {
                thresholds_raw: [0; 7],
                multipliers_q8: [256; 7],
            });
            hive.collision.state_flags_at_0x08.overwrite(
                CHECKED_DAMAGE_ENABLED_STATE_BIT
                    | RADIAL_DAMAGE_ALTERNATE_ELIGIBLE_STATE_BIT
                    | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
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
        }
        let factory = manager
            .entities
            .iter()
            .find(|entity| entity.entity_type == LEVEL_ONE_TYPE66_ENTITY_TYPE)
            .expect("composition fixture retains a Working Factory");
        let RetailRuntimeValue::Known(Some(factory_runtime)) = factory.base_factory_runtime else {
            panic!("Working Factory live owner")
        };
        let live_owner = factory_runtime.live_owner.expect("factory allocation");
        let source_factory = crate::factory_production_live::FactoryProductionEntityVersion {
            entity_id: factory.id,
            allocation_identity: live_owner.allocation_identity,
            state_version: live_owner.state_version,
        };
        let request = crate::factory_production_live::FactoryEntitySpawnRequest {
            requested_handle: FactoryRequestedEntityHandle::Allocate,
            entity_type: LEVEL_ONE_TYPE61_ENTITY_TYPE,
            position_raw: hive_position,
            spawn_parameter_6: factory_runtime
                .production
                .expect("factory production")
                .output_payload_packed,
        };
        for entity in &mut manager.entities {
            if entity.id != source_factory.entity_id && entity.id != hive_id {
                entity.position = raw_position_world([i16::MAX, 0, i16::MAX]);
            }
        }
        let mut world_fx = WorldFx::new();
        let spawned = manager
            .append_factory_pickup(source_factory, request, &mut world_fx)
            .expect("factory constructor");
        assert!(manager.link_factory_pickup_owner(spawned, source_factory));
        manager
            .entities
            .iter_mut()
            .find(|entity| entity.id == source_factory.entity_id)
            .unwrap()
            .position = raw_position_world([i16::MAX, 0, i16::MAX]);
        manager
            .entities
            .iter_mut()
            .find(|entity| entity.id == spawned.entity_id.get())
            .unwrap()
            .position = raw_position_world(hive_position);
        let source_id = spawned.entity_id.get();
        let actor = manager
            .main_base_abort_actor_observation(source_id)
            .unwrap();
        let death_request = MainBaseType61DeathRequest {
            active_model_extent_raw: LEVEL_ONE_TYPE61_DEFAULT_MODEL_EXTENT_RAW,
            sea_level_raw: Some((terrain.header[0] >> 8) as i16),
            logical_owner: MainBaseType61LogicalOwner {
                entity_id: source_factory.entity_id,
                entity_type: LEVEL_ONE_TYPE66_ENTITY_TYPE,
            },
            network_session: MainBaseType61NetworkSession::SoloNetworkingDisabled,
        };
        let mut hull = PlayerHull::default();
        let mut static_radial = StaticRadialRecorder::accepting().with_hive_dying_extent(343, 0x34);
        let particles_before = world_fx.particle_count();
        manager
            .apply_main_base_abort_type61_death(
                actor.lease,
                death_request,
                &terrain,
                &mut hull,
                &mut world_fx,
                &mut static_radial,
            )
            .expect("in-range Hive67 lethal radial must not roll back Type61 abort");
        let hive = manager
            .entities
            .iter()
            .find(|entity| entity.id == hive_id)
            .unwrap();
        assert_eq!(hive.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            hive.actor_task_state(ActorTaskSlot::Primary),
            Some(&ActorTaskRuntime::HiveRadial(HiveRadialTaskState::dying()))
        );
        assert!(
            world_fx.particle_count() > particles_before,
            "FUN_00440950 hive burst must emit after Type61 abort commit"
        );
    }

    #[test]
    fn factory_type61_abort_rejects_stale_factory_allocation_identity() {
        let mut manager = crate::entity::terminal_abort_composition_manager();
        manager.fresh_new_game_first_world = false;
        let terrain = exact_terrain();
        let factory = manager
            .entities
            .iter()
            .find(|entity| entity.entity_type == LEVEL_ONE_TYPE66_ENTITY_TYPE)
            .unwrap();
        let RetailRuntimeValue::Known(Some(factory_runtime)) = factory.base_factory_runtime else {
            panic!("factory runtime")
        };
        let live_owner = factory_runtime.live_owner.unwrap();
        let source_factory = crate::factory_production_live::FactoryProductionEntityVersion {
            entity_id: factory.id,
            allocation_identity: live_owner.allocation_identity,
            state_version: live_owner.state_version,
        };
        let request = crate::factory_production_live::FactoryEntitySpawnRequest {
            requested_handle: FactoryRequestedEntityHandle::Allocate,
            entity_type: LEVEL_ONE_TYPE61_ENTITY_TYPE,
            position_raw: factory.position_raw(),
            spawn_parameter_6: factory_runtime.production.unwrap().output_payload_packed,
        };
        for entity in &mut manager.entities {
            if entity.id != source_factory.entity_id {
                entity.position = raw_position_world([i16::MAX, 0, i16::MAX]);
            }
        }
        let mut world_fx = WorldFx::new();
        let spawned = manager
            .append_factory_pickup(source_factory, request, &mut world_fx)
            .unwrap();
        assert!(manager.link_factory_pickup_owner(spawned, source_factory));
        {
            let factory = manager
                .entities
                .iter_mut()
                .find(|entity| entity.id == source_factory.entity_id)
                .unwrap();
            let RetailRuntimeValue::Known(Some(runtime)) = &mut factory.base_factory_runtime else {
                panic!("factory runtime remains known")
            };
            runtime.live_owner.as_mut().unwrap().allocation_identity =
                live_owner.allocation_identity.wrapping_add(1);
        }
        let actor = manager
            .main_base_abort_actor_observation(spawned.entity_id.get())
            .unwrap();
        let err = manager
            .apply_main_base_abort_type61_death(
                actor.lease,
                MainBaseType61DeathRequest {
                    active_model_extent_raw: LEVEL_ONE_TYPE61_DEFAULT_MODEL_EXTENT_RAW,
                    sea_level_raw: Some((terrain.header[0] >> 8) as i16),
                    logical_owner: MainBaseType61LogicalOwner {
                        entity_id: source_factory.entity_id,
                        entity_type: LEVEL_ONE_TYPE66_ENTITY_TYPE,
                    },
                    network_session: MainBaseType61NetworkSession::SoloNetworkingDisabled,
                },
                &terrain,
                &mut PlayerHull::default(),
                &mut world_fx,
                &mut StaticRadialRecorder::accepting(),
            )
            .expect_err("stale factory allocation cannot abort the product");
        assert_eq!(
            err,
            MainBaseType61DeathBlock::FactoryBirthProvenanceMismatch
        );
    }

    #[v2k_test_support::retail_test]
    fn ordinary_world_factory_type61_abort_ignores_level_one_spawn_index() {
        let data = v2k_test_support::retail_dir();
        let mut session = crate::session::GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(14, 1).unwrap();
        let metadata: Vec<_> = session
            .cache
            .global_entity_model_table()
            .iter()
            .enumerate()
            .map(|(id, models)| {
                session
                    .cache
                    .global_entity_type(id)
                    .map(crate::entity_collision_state::EntityTypeRuntimeMetadata::from_section12)
                    .unwrap_or(crate::entity_collision_state::EntityTypeRuntimeMetadata {
                        model_slots: *models,
                        ..Default::default()
                    })
            })
            .collect();
        let mut world_fx = WorldFx::new();
        let mut manager = crate::entity::EntityManager::from_authored_world(
            crate::entity::AuthoredWorldConstruction {
                logical_world_index: 2,
                level: session.cache.level_desc().unwrap(),
                type_metadata: &metadata,
                resources: crate::entity::EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| {
                        session.cache.global_model(id).map(|model| model.radius)
                    }),
                },
                player_arrival: None,
                retail_tick: 0,
            },
            &mut world_fx,
        )
        .expect("overlay 14 native construction");
        assert!(!manager.is_fresh_new_game_first_world());
        let factory = manager
            .entities
            .iter()
            .find(|entity| entity.entity_type == LEVEL_ONE_TYPE66_ENTITY_TYPE)
            .expect("overlay 14 authors a Working Factory");
        assert_ne!(
            factory.authored_spawn_index,
            Some(LEVEL_ONE_TYPE66_SPAWN_INDEX),
            "overlay 14 must not reuse Level-1 spawn 23 as the admission key"
        );
        let factory_id = factory.id;
        let factory_position = factory.position_raw();
        let RetailRuntimeValue::Known(Some(factory_runtime)) = factory.base_factory_runtime else {
            panic!("native factory publishes live runtime")
        };
        let live_owner = factory_runtime.live_owner.expect("native factory owner");
        let payload = factory_runtime
            .production
            .expect("native factory production")
            .output_payload_packed;
        let source_factory = crate::factory_production_live::FactoryProductionEntityVersion {
            entity_id: factory_id,
            allocation_identity: live_owner.allocation_identity,
            state_version: live_owner.state_version,
        };
        for entity in &mut manager.entities {
            if entity.id != factory_id {
                entity.position = raw_position_world([i16::MAX, 0, i16::MAX]);
            }
        }
        let request = crate::factory_production_live::FactoryEntitySpawnRequest {
            requested_handle: FactoryRequestedEntityHandle::Allocate,
            entity_type: LEVEL_ONE_TYPE61_ENTITY_TYPE,
            position_raw: factory_position,
            spawn_parameter_6: payload,
        };
        let spawned = manager
            .append_factory_pickup(source_factory, request, &mut world_fx)
            .expect("overlay 14 Type-61 constructor is the invariant Section-12 row");
        assert!(manager.link_factory_pickup_owner(spawned, source_factory));
        manager
            .entities
            .iter_mut()
            .find(|entity| entity.id == factory_id)
            .unwrap()
            .position = raw_position_world([i16::MAX, 0, i16::MAX]);
        let source_id = spawned.entity_id.get();
        let terrain = session.cache.terrain().expect("overlay 14 terrain");
        let actor = manager
            .main_base_abort_actor_observation(source_id)
            .unwrap();
        let MainBaseType61DeathAdvance::Advanced {
            outcome: MainBaseType61DeathOutcome::ExplodeWithRingCommitted { ring, .. },
            ..
        } = manager
            .apply_main_base_abort_type61_death(
                actor.lease,
                MainBaseType61DeathRequest {
                    active_model_extent_raw: LEVEL_ONE_TYPE61_DEFAULT_MODEL_EXTENT_RAW,
                    sea_level_raw: Some((terrain.header[0] >> 8) as i16),
                    logical_owner: MainBaseType61LogicalOwner {
                        entity_id: factory_id,
                        entity_type: LEVEL_ONE_TYPE66_ENTITY_TYPE,
                    },
                    network_session: MainBaseType61NetworkSession::SoloNetworkingDisabled,
                },
                terrain,
                &mut PlayerHull::default(),
                &mut world_fx,
                &mut StaticRadialRecorder::accepting(),
            )
            .expect("ordinary-world factory Type-61 uses the shared class-49 owner")
        else {
            panic!("overlay 14 factory product appends a Type-60 tail")
        };
        let Type60ConstructionOutcome::ActorLinked(ring) = ring else {
            panic!("linked class-49 tail")
        };
        manager
            .apply_main_base_abort_type60_ring_death(ring.actor(), None, &mut world_fx)
            .expect("ordinary-world class-49 tail takes the shared Type-60 death owner");
    }

    #[test]
    fn power_up_pending_type61_executes_class49_without_duplicate_deferred_owner() {
        let (mut manager, terrain) = manager_and_terrain();
        manager.queue_power_up_destroys(&[1]);
        let actor = manager.main_base_abort_actor_observation(1).unwrap();
        let request = request_for(&manager, 1, &terrain);
        let mut hull = PlayerHull::default();
        let mut world_fx = WorldFx::new();
        let mut static_radial = StaticRadialRecorder::accepting();

        let MainBaseType61DeathAdvance::Advanced {
            outcome:
                MainBaseType61DeathOutcome::ExplodeWithRingCommitted {
                    ring: Type60ConstructionOutcome::ActorLinked(receipt),
                    deferred_destroy,
                    dynamic_radial_accepted_targets,
                    ..
                },
            next_actor,
        } = manager
            .apply_main_base_abort_type61_death(
                actor.lease,
                request,
                &terrain,
                &mut hull,
                &mut world_fx,
                &mut static_radial,
            )
            .expect("Power-Up-owned pending state does not suppress class 49")
        else {
            panic!("the externally pending source still publishes one Type-60 tail")
        };

        assert_eq!(
            deferred_destroy,
            MainBaseDeferredDestroyCommit::AlreadyPending(
                MainBaseExternalDeferredDestroyOwner::Type61DeferredQueue
            )
        );
        assert_eq!(dynamic_radial_accepted_targets, 1);
        assert_eq!(receipt.actor().entity_id, 4);
        assert_eq!(next_actor.map(|actor| actor.lease.entity_id), Some(2));
        assert_eq!(manager.pending_power_up_destroy_ids(), [1]);
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
        assert_eq!(manager.entities.len(), 4);
        assert_eq!(world_fx.exploding_rings().len(), 1);
        let source = &manager.entities[0];
        assert_eq!(source.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            source
                .collision
                .state_flags_at_0x08
                .masked(DYING_STATE_BIT | DEFERRED_DESTROY_PENDING_STATE_BIT),
            RetailRuntimeValue::Known(DYING_STATE_BIT | DEFERRED_DESTROY_PENDING_STATE_BIT)
        );

        assert_eq!(manager.cleanup_pending_power_up_destroys(), [1]);
        assert_eq!(
            manager.retail_live_order_ids().collect::<Vec<_>>(),
            [2, 3, 4]
        );
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
    }

    #[test]
    fn type60_constructor_failures_keep_exact_rng_and_publication_boundaries() {
        for case in 0..3 {
            let (mut manager, terrain) = manager_and_terrain();
            let actor = manager.main_base_abort_actor_observation(1).unwrap();
            let request = request_for(&manager, 1, &terrain);
            let mut hull = PlayerHull::default();
            let mut world_fx = WorldFx::new();
            let mut static_radial = StaticRadialRecorder::accepting();
            let mut oracle = WorldFx::new();
            let _burst = oracle.next_shared_retail_random_u16();
            let expected_static = oracle.next_shared_retail_random_u16();
            let expected_selector = oracle.next_shared_retail_random_u16();

            let advance = match case {
                0 => manager.apply_main_base_abort_type61_death_with_allocator(
                    actor.lease,
                    request,
                    &terrain,
                    &mut hull,
                    &mut world_fx,
                    &mut static_radial,
                    &mut RejectBeforeSelector,
                ),
                1 => manager.apply_main_base_abort_type61_death_with_allocator(
                    actor.lease,
                    request,
                    &terrain,
                    &mut hull,
                    &mut world_fx,
                    &mut static_radial,
                    &mut RejectAfterSelector,
                ),
                _ => manager.apply_main_base_abort_type61_death_with_allocator(
                    actor.lease,
                    request,
                    &terrain,
                    &mut hull,
                    &mut world_fx,
                    &mut static_radial,
                    &mut RejectPrimary,
                ),
            }
            .expect("ring allocation failure is non-fatal");
            let MainBaseType61DeathAdvance::Advanced {
                outcome: MainBaseType61DeathOutcome::ExplodeWithRingCommitted { ring, .. },
                ..
            } = advance
            else {
                panic!("source death committed")
            };
            assert_eq!(static_radial.commit_rng_words, [expected_static]);
            match (case, ring) {
                (0, Type60ConstructionOutcome::RejectedBeforeSelector) => {
                    assert_eq!(manager.entities.len(), 3);
                    assert!(world_fx.exploding_rings().is_empty());
                    assert_eq!(
                        world_fx.next_shared_retail_random_u16(),
                        expected_selector,
                        "pre-selector rejection consumes no selector word"
                    );
                }
                (1, Type60ConstructionOutcome::RejectedAfterSelector { selector_rng_word }) => {
                    assert_eq!(selector_rng_word, expected_selector);
                    assert_eq!(manager.entities.len(), 3);
                    assert!(world_fx.exploding_rings().is_empty());
                }
                (2, Type60ConstructionOutcome::ActorLinked(receipt)) => {
                    assert_eq!(
                        receipt.constructor_rng(),
                        Type60ConstructorRngDisposition::SelectorWord(expected_selector)
                    );
                    assert_eq!(
                        receipt.initializer(),
                        Type60InitializerDisposition::
                            FallbackPublishedAfterPrimaryAllocationFailure {
                                control_output_1_raw: 0,
                            }
                    );
                    assert_eq!(manager.entities.len(), 4);
                    assert_eq!(world_fx.exploding_rings()[0].control_output_1_raw(), 0);
                    let ring_actor = manager
                        .main_base_abort_actor_observation(receipt.actor().entity_id)
                        .unwrap();
                    assert!(manager
                        .apply_main_base_abort_type60_ring_death(
                            ring_actor.lease,
                            None,
                            &mut world_fx,
                        )
                        .is_ok());
                }
                other => panic!("unexpected constructor outcome: {other:?}"),
            }
        }
    }

    #[test]
    fn radial_preflight_and_static_preflight_fail_without_partial_commit() {
        let (mut manager, terrain) = manager_and_terrain();
        let actor = manager.main_base_abort_actor_observation(1).unwrap();
        let request = request_for(&manager, 1, &terrain);
        let mut hull = PlayerHull::default();
        let hull_before = hull;
        let source_before = snapshot(&manager.entities[0]);
        let mut world_fx = WorldFx::new();
        let mut rng_oracle = WorldFx::new();
        let mut static_radial = StaticRadialRecorder::accepting();
        static_radial.preflight_ok = false;

        assert_eq!(
            manager.apply_main_base_abort_type61_death(
                actor.lease,
                request,
                &terrain,
                &mut hull,
                &mut world_fx,
                &mut static_radial,
            ),
            Err(MainBaseType61DeathBlock::StaticRadialDamagePreflightUnresolved)
        );
        assert_eq!(snapshot(&manager.entities[0]), source_before);
        assert_eq!(hull, hull_before);
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
        assert!(world_fx.exploding_rings().is_empty());
        assert!(world_fx.take_positional_sounds().is_empty());
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            rng_oracle.next_shared_retail_random_u16()
        );

        // Place a second healthy actor within the blast. Its unresolved
        // modifier/type-hit fields make the dynamic pass fail before static
        // preflight or any generic-death/effect mutation.
        let (mut manager, terrain) = manager_and_terrain();
        manager.entities[1].position = manager.entities[0].position;
        manager.entities[1].collision.damage_profile = RetailRuntimeValue::Known(DamageProfile {
            thresholds_raw: [0; 7],
            multipliers_q8: [256; 7],
        });
        let actor = manager.main_base_abort_actor_observation(1).unwrap();
        let request = request_for(&manager, 1, &terrain);
        let source_before = snapshot(&manager.entities[0]);
        let target_before = snapshot(&manager.entities[1]);
        let mut hull = PlayerHull::default();
        let mut world_fx = WorldFx::new();
        let mut static_radial = StaticRadialRecorder::accepting();
        assert_eq!(
            manager.apply_main_base_abort_type61_death(
                actor.lease,
                request,
                &terrain,
                &mut hull,
                &mut world_fx,
                &mut static_radial,
            ),
            Err(MainBaseType61DeathBlock::DynamicRadialDamagePreflightUnresolved)
        );
        assert_eq!(snapshot(&manager.entities[0]), source_before);
        assert_eq!(snapshot(&manager.entities[1]), target_before);
        assert!(static_radial.preflights.is_empty());
        assert!(world_fx.exploding_rings().is_empty());
    }

    #[test]
    fn relation_owner_is_derived_and_mismatches_fail_before_effects() {
        let (mut manager, terrain) = manager_and_terrain();
        manager.entities[0].collision.recent_relation_id_at_0x60 =
            RetailRuntimeValue::Known(Some(2));
        let actor = manager.main_base_abort_actor_observation(1).unwrap();
        let mut request = request_for(&manager, 1, &terrain);
        let mut hull = PlayerHull::default();
        let mut world_fx = WorldFx::new();
        let mut static_radial = StaticRadialRecorder::accepting();

        assert_eq!(
            manager.apply_main_base_abort_type61_death(
                actor.lease,
                request,
                &terrain,
                &mut hull,
                &mut world_fx,
                &mut static_radial,
            ),
            Err(MainBaseType61DeathBlock::LogicalOwnerMismatch {
                expected: MainBaseType61LogicalOwner {
                    entity_id: 2,
                    entity_type: LEVEL_ONE_TYPE61_ENTITY_TYPE,
                },
                actual: request.logical_owner,
            })
        );
        assert!(static_radial.preflights.is_empty());

        request.logical_owner = MainBaseType61LogicalOwner {
            entity_id: 2,
            entity_type: LEVEL_ONE_TYPE61_ENTITY_TYPE,
        };
        assert!(manager
            .apply_main_base_abort_type61_death(
                actor.lease,
                request,
                &terrain,
                &mut hull,
                &mut world_fx,
                &mut static_radial,
            )
            .is_ok());
        let particles = world_fx.exploding_rings().len();
        assert_eq!(particles, 1);
    }

    #[test]
    fn dangling_relation_falls_back_to_the_type61_source() {
        let (mut manager, terrain) = manager_and_terrain();
        manager.entities[0].collision.recent_relation_id_at_0x60 =
            RetailRuntimeValue::Known(Some(0xDEAD_BEEF));
        let actor = manager.main_base_abort_actor_observation(1).unwrap();
        let request = request_for(&manager, 1, &terrain);
        let mut hull = PlayerHull::default();
        let mut world_fx = WorldFx::new();
        let mut static_radial = StaticRadialRecorder::accepting();

        let MainBaseType61DeathAdvance::Advanced {
            outcome: MainBaseType61DeathOutcome::ExplodeWithRingCommitted { radial, .. },
            ..
        } = manager
            .apply_main_base_abort_type61_death(
                actor.lease,
                request,
                &terrain,
                &mut hull,
                &mut world_fx,
                &mut static_radial,
            )
            .expect("dangling retail relation is treated as absent")
        else {
            panic!("source death committed")
        };
        assert_eq!(
            radial.trailing_raw,
            [LEVEL_ONE_TYPE61_ENTITY_TYPE as i32, 1]
        );
    }

    #[test]
    fn model_extent_must_match_the_captured_active_model() {
        for source_id in 1_u32..=3 {
            let (mut manager, terrain) = manager_and_terrain();
            let actor = manager
                .main_base_abort_actor_observation(source_id)
                .unwrap();
            let mut request = request_for(&manager, source_id, &terrain);
            request.active_model_extent_raw ^= 1;
            let before = snapshot(&manager.entities[(source_id - 1) as usize]);
            let mut hull = PlayerHull::default();
            let mut world_fx = WorldFx::new();
            let mut static_radial = StaticRadialRecorder::accepting();

            assert_eq!(
                manager.apply_main_base_abort_type61_death(
                    actor.lease,
                    request,
                    &terrain,
                    &mut hull,
                    &mut world_fx,
                    &mut static_radial,
                ),
                Err(MainBaseType61DeathBlock::ActiveModelMismatch)
            );
            assert_eq!(
                snapshot(&manager.entities[(source_id - 1) as usize]),
                before
            );
            assert!(static_radial.preflights.is_empty());
            assert_eq!(world_fx.particle_count(), 0);
        }
    }

    #[test]
    fn remote_and_already_dying_noops_precede_downstream_type61_requirements() {
        for state in [REMOTE_OWNED_STATE_BIT, DYING_STATE_BIT] {
            let (mut manager, terrain) = manager_and_terrain();
            manager.entities[0].collision.state_flags_at_0x08 = RetailStateWord::exact(state);
            manager.pending_actor_deferred_destroy_ids.push(1);
            manager.type_metadata.truncate(1);
            let actor = manager.main_base_abort_actor_observation(1).unwrap();
            let request = MainBaseType61DeathRequest {
                active_model_extent_raw: 0,
                sea_level_raw: None,
                logical_owner: MainBaseType61LogicalOwner {
                    entity_id: u32::MAX,
                    entity_type: u32::MAX,
                },
                network_session: MainBaseType61NetworkSession::Unresolved,
            };
            let before = snapshot(&manager.entities[0]);
            let mut hull = PlayerHull::default();
            let mut world_fx = WorldFx::new();
            let mut static_radial = StaticRadialRecorder::accepting();
            let outcome = manager
                .apply_main_base_abort_type61_death(
                    actor.lease,
                    request,
                    &terrain,
                    &mut hull,
                    &mut world_fx,
                    &mut static_radial,
                )
                .expect("generic no-op reads no downstream Type-61 state");
            assert!(matches!(
                outcome,
                MainBaseType61DeathAdvance::Advanced {
                    outcome: MainBaseType61DeathOutcome::RemoteOwnedNoOp { .. },
                    ..
                } | MainBaseType61DeathAdvance::Advanced {
                    outcome: MainBaseType61DeathOutcome::AlreadyDyingNoOp { .. },
                    ..
                }
            ));
            assert_eq!(snapshot(&manager.entities[0]), before);
            assert_eq!(manager.pending_actor_deferred_destroy_ids(), [1]);
            assert!(static_radial.preflights.is_empty());
        }
    }
}
