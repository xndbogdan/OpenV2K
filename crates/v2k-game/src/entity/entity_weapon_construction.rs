//! Native modeled44E770 births:438080 ->104B0/D4A0 ->D720, then source+60.

use super::*;
use crate::{
    guard_location_owner::acquisition::{
        GuardLocationAcquisitionTaskState, GuardLocationSearchContext,
    },
    native_entity_weapons::{
        grenade::{authenticate_grenade_metadata, GRENADE_CLASS35_CLEAR_ORDER},
        rocket::{
            authenticate_rocket_metadata, initialize_rocket_class22, RocketClass22InitHost,
            RocketClass22InitRequest, RocketFlightTaskState, RocketTaskPublication,
            ROCKET_ACQUISITION_FILTER_OVERRIDE_RAW,
        },
        EntityWeaponBlock, EntityWeaponConstructionRequest, EntityWeaponKind,
        NativeEntityWeaponOwner, NativeEntityWeaponRuntime,
    },
    resource_cache::ResourceCache,
    search_attack::SearchAttackCandidateFilter,
    wrapped_axis_range::WrappedAxisRange,
};

struct RocketInitializerHost<'a> {
    entity: &'a mut Entity,
    fx: &'a mut WorldFx,
}

impl RocketClass22InitHost for RocketInitializerHost<'_> {
    type PreparedTask = PreparedActorTask<ActorTaskRuntime>;
    type Error = EntityWeaponBlock;

    fn write_search_range_raw(&mut self, range_raw: i32) {
        let runtime = self.entity.native_entity_weapon_runtime.as_mut().unwrap();
        let context = runtime.search_context.unwrap();
        runtime.search_context = Some(GuardLocationSearchContext::new(
            WrappedAxisRange::from_raw(range_raw),
            context.filter(),
        ));
    }

    fn write_sub_a_target_speed_raw(&mut self, target_speed_raw: i32) {
        let RetailRuntimeValue::Known(Some(runtime)) = &mut self.entity.sub_a_propulsion_runtime
        else {
            unreachable!("authenticated Type42 owns its constructed SubA")
        };
        runtime.apply_shared_initializer_target_speed_write(target_speed_raw);
    }

    fn write_sub_a_direction_multiplier_raw(&mut self, value: i32) {
        let RetailRuntimeValue::Known(Some(runtime)) = &mut self.entity.sub_a_propulsion_runtime
        else {
            unreachable!("authenticated Type42 owns its constructed SubA")
        };
        runtime.set_direction_multiplier(value);
    }

    fn prepare_task(
        &mut self,
        request: RocketTaskPublication,
    ) -> Result<Self::PreparedTask, Self::Error> {
        let task = match request {
            RocketTaskPublication::Acquisition => ActorTaskRuntime::GuardLocationAcquisition(
                GuardLocationAcquisitionTaskState::new(ROCKET_ACQUISITION_FILTER_OVERRIDE_RAW),
            ),
            RocketTaskPublication::Trail => ActorTaskRuntime::RocketTrail,
            RocketTaskPublication::Flight => {
                ActorTaskRuntime::RocketFlight(RocketFlightTaskState::new())
            }
        };
        Ok(PreparedActorTask::new(task))
    }

    fn next_shared_retail_random_u16(&mut self) -> u16 {
        self.fx.next_shared_retail_random_u16()
    }

    fn publish_task(&mut self, request: RocketTaskPublication, prepared: Self::PreparedTask) {
        self.entity
            .actor_tasks
            .replace_prepared(request.slot(), prepared);
    }
}

impl EntityManager {
    /// Construct a real live allocation, preserving the common-body stamp,
    /// process RNG and three-slot custody. The successful D720 wrapper links
    /// the actor before building its matrix and setting bit4;44E770 assigns
    /// the shooter's relation only after that constructor returns zero.
    pub fn construct_entity_weapon(
        &mut self,
        request: EntityWeaponConstructionRequest,
        resources: &ResourceCache,
        fx: &mut WorldFx,
        retail_tick: u32,
    ) -> Result<NativeEntityWeaponOwner, EntityWeaponBlock> {
        let entity_type = request.kind.entity_type() as u32;
        let record = resources
            .global_entity_type(entity_type as usize)
            .ok_or(EntityWeaponBlock::Metadata)?;
        let metadata = EntityTypeRuntimeMetadata::from_section12(record);
        if self.type_metadata.get(entity_type as usize) != Some(&metadata) {
            return Err(EntityWeaponBlock::Metadata);
        }
        match request.kind {
            EntityWeaponKind::Rocket => authenticate_rocket_metadata(&metadata)?,
            EntityWeaponKind::Grenade | EntityWeaponKind::DepthCharge => {
                authenticate_grenade_metadata(&metadata)
                    .map_err(|_| EntityWeaponBlock::Metadata)?;
            }
        }
        if metadata
            .model_slots
            .iter()
            .any(|id| resources.global_model(usize::from(*id)).is_none())
            || metadata.initial_health_raw.is_none()
            || metadata.damage_profile.is_none()
            || metadata.constructor_sound_attachment_id != RetailRuntimeValue::Known(None)
        {
            return Err(EntityWeaponBlock::Metadata);
        }
        let terrain = resources
            .level_terrain()
            .ok_or(EntityWeaponBlock::Terrain)?;
        let surface_bits = crate::entity_initializer::constructor_surface_bits_at_tick(
            request.position_raw,
            terrain,
            retail_tick,
            self.common_environment_physics().waves_enabled,
        )
        .ok_or(EntityWeaponBlock::Terrain)?;
        let source = self
            .entities
            .iter()
            .find(|actor| actor.id == request.source_actor_id)
            .ok_or(EntityWeaponBlock::SourceAllocation)?;
        // Creation consumes the current source allocation, rather than granting
        // a type/model fixture native shooter custody.14870 serializes source
        // bit31 into packet+28;44E770 rejects that remote request before438080.
        // The dying bit4000 is not a launch gate: a queued transient may still
        // drain while its current source allocation remains linked.
        if !source.active
            || !matches!(
                source.construction_stamp_at_0xb4,
                RetailRuntimeValue::Known(_)
            )
            || source.collision.state_flags_at_0x08.masked(0x8000_0000)
                != RetailRuntimeValue::Known(0)
            || resources
                .global_entity_type(source.entity_type as usize)
                .is_none()
            || self.main_base_abort_actor_observation(source.id).is_none()
        {
            return Err(EntityWeaponBlock::SourceAllocation);
        }
        if !matches!(
            self.next_common_body_ordinal(),
            RetailRuntimeValue::Known(_)
        ) {
            return Err(EntityWeaponBlock::Runtime("native body stamp lineage"));
        }
        let id = self.next_entity_id.max(1);
        if self.entities.iter().any(|actor| actor.id == id) {
            return Err(EntityWeaponBlock::Allocation);
        }

        // Host admission is complete;104B0 consumes this before allocation.
        let stamp = self.begin_common_body_attempt();
        let mut entity =
            Entity::unresolved_port_entity(id, EntityKind::from_type(entity_type), entity_type);
        entity.construction_stamp_at_0xb4 = stamp;
        entity.set_motion_raw(request.position_raw, request.velocity_raw);
        entity.set_rotation_heading_pitch_roll_raw(request.rotation_raw);
        entity.mass_raw = metadata.mass_raw;
        entity.capability_flags = metadata.capability_flags;
        entity.model_slots = metadata.model_slots.map(|model| Some(usize::from(model)));
        entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(0);
        entity.sub_n_runtime = entity_sub_n_runtime_from_constructor(
            Some(&metadata),
            false,
            request.position_raw,
            None,
        );
        entity.base_factory_runtime =
            base_factory_runtime_from_constructor(entity_type, Some(&metadata), None);
        entity.actor_animation_runtime = actor_animation_runtime_from_constructor(Some(&metadata));
        entity.sub_g_06070_runtime = sub_g_06070_runtime_from_constructor(Some(&metadata));
        entity.sub_h_external_frame_runtime =
            sub_h_external_frame_runtime_from_constructor(Some(&metadata));
        entity.sub_j_attachment_runtime =
            sub_j_attachment_runtime_from_constructor(Some(&metadata), None, None);
        entity.actor_common_axis_descriptor =
            actor_common_axis_descriptor_from_constructor(Some(&metadata));
        entity.sub_a_propulsion_runtime = match metadata.sub_a_propulsion_descriptor {
            RetailRuntimeValue::Known(Some(descriptor)) => {
                RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::from_20450_constructor(
                    descriptor,
                    fx.next_shared_retail_random_u16(),
                )))
            }
            RetailRuntimeValue::Known(None) => RetailRuntimeValue::Known(None),
            RetailRuntimeValue::Unresolved => unreachable!("weapon topology was authenticated"),
        };
        let initializer = metadata.initializer.as_ref().unwrap();
        let selected = select_initial_behavior(
            &initializer.behavior_choices,
            |rule| i32::from(rule == BehaviorWeightRule::Always),
            &mut || u32::from(fx.next_shared_retail_random_u16()),
        )
        .map_err(|_| EntityWeaponBlock::Graph)?
        .ok_or(EntityWeaponBlock::Graph)?;
        let resolution =
            crate::entity_initializer::resolve_entity_initializer_with_selected_behavior(
                EntityInitializerRequest {
                    metadata: Some(&metadata),
                    spawn_param: 0,
                    authored_position_raw: request.position_raw,
                    terrain: Some(terrain),
                    resource_domain: ResourceDomainRelation::Current,
                },
                Some(selected),
            );
        let mut state_flags = resolution.state_flags;
        state_flags.overwrite(
            crate::entity_collision_state::SURFACE_STATE_MASK,
            surface_bits,
        );
        state_flags.overwrite(4, 0);
        entity.collision =
            EntityCollisionRuntimeState::from_constructor(Some(&metadata), 0, state_flags);
        // Labeled native-allocation residue policy:104B0/4572B0 allocate
        // without clearing +B2, and these templates/D720 do not write it.
        // As for fresh Type9, the host starts this transient contribution
        // empty so an immediate first callback has defined mass. This is an
        // allocation approximation, not a retail constructor invariant or
        // acceptance claim. Native animation/task writers and13500's clear
        // own all later values; follow-up owner:PLAYER_CRAFT.md.
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
        // Both templates have null inner component-contact hooks and104B0's
        // modifier/type-hit pointers are null. Rolling owns a live matrix, so
        // an Euler-only orientation policy would invent later reconstruction.
        entity.collision.pair_callbacks =
            EntityPairCallbackRuntimeState::audited_local(None, RetailRuntimeValue::Unresolved);
        entity.model_index = model_for_constructor_state(entity.model_slots, state_flags);
        entity.initial_behavior = RetailRuntimeValue::Known(Some(selected));
        entity.current_behavior_context = RetailRuntimeValue::Known(Some(
            BehaviorContextRuntime::from_fresh_weighted_selection(selected)
                .ok_or(EntityWeaponBlock::Graph)?,
        ));
        let allocation = observe_main_base_abort_actor(&entity, self.allocation_generation).lease;
        entity.native_entity_weapon_runtime = Some(NativeEntityWeaponRuntime {
            allocation,
            kind: request.kind,
            search_context: (request.kind == EntityWeaponKind::Rocket).then(|| {
                GuardLocationSearchContext::new(
                    WrappedAxisRange::from_raw(
                        initializer.common_axis_descriptor.strict_axis_limit_raw,
                    ),
                    SearchAttackCandidateFilter::from_raw(
                        initializer.common_axis_descriptor.raw_word_at_0x04,
                    ),
                )
            }),
            model_effect_bits_at_0x84: 0,
        });
        match request.kind {
            EntityWeaponKind::Rocket => {
                let RetailRuntimeValue::Known(Some(descriptor)) =
                    metadata.sub_a_propulsion_descriptor
                else {
                    unreachable!()
                };
                let RetailRuntimeValue::Known(Some(runtime)) = entity.sub_a_propulsion_runtime
                else {
                    unreachable!()
                };
                let RetailRuntimeValue::Known(speed) = runtime.target_speed_raw() else {
                    unreachable!()
                };
                initialize_rocket_class22(
                    RocketClass22InitRequest {
                        sub_a: descriptor,
                        existing_target_speed_raw: speed,
                    },
                    &mut RocketInitializerHost {
                        entity: &mut entity,
                        fx,
                    },
                )
                .map_err(|error| error.error)?;
            }
            EntityWeaponKind::Grenade | EntityWeaponKind::DepthCharge => {
                for slot in GRENADE_CLASS35_CLEAR_ORDER {
                    entity.actor_tasks.clear_slot(slot);
                }
                entity.actor_tasks.replace_prepared(
                    ActorTaskSlot::Primary,
                    PreparedActorTask::new(ActorTaskRuntime::BoulderRolling(
                        crate::intro2_meteors::BoulderRollingTaskState::new(request.position_raw),
                    )),
                );
            }
        }
        let task_ids =
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot));
        assert_eq!(self.allocate_live_entity_id(), id);
        self.append_live_entity(entity);
        let linked = self.entities.last_mut().unwrap();
        let [heading, pitch, roll] = request.rotation_raw;
        linked.physical_body_basis_q31 =
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(heading, pitch, roll));
        linked.collision.state_flags_at_0x08.overwrite(4, 4);
        linked.collision.recent_relation_id_at_0x60 =
            RetailRuntimeValue::Known(Some(request.source_actor_id));
        Ok(NativeEntityWeaponOwner {
            actor: allocation,
            kind: request.kind,
            task_ids,
        })
    }
}

#[cfg(test)]
#[path = "entity_weapon_construction_tests.rs"]
mod tests;
