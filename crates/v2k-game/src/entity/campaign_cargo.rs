//! Native conventional campaign cargo restoration (`451C00`).
//!
//! The destination authored list already exists. Each packed identity either
//! reuses its first source-order match or enters a real zero-record constructor;
//! both then use the same player attachment transaction as live collection.

use super::class0_actor::{
    authenticate_metadata, construct_class0_actor, Class0ActorConstruction, Class0SpawnInput,
    NativeType68ZeroRecordConstruction,
};
use super::*;
use crate::entity_collision_state::DYING_STATE_BIT;
use crate::gameplay_notifications::GameplayNotifications;
use crate::specialized_actor_task_production::SpecializedActorTaskScheduler;

pub struct CampaignCargoRestoreContext<'a> {
    pub level: &'a LevelDescriptor,
    pub resources: EntityConstructionResources<'a>,
    pub retail_tick: u32,
    pub scheduler: &'a mut SpecializedActorTaskScheduler,
    pub world_fx: &'a mut WorldFx,
    pub notifications: &'a mut GameplayNotifications,
}

#[derive(Debug)]
pub enum CampaignCargoRestoreError {
    MissingPlayer,
    PlayerAttachmentRuntimeUnavailable,
    TerrainUnavailable,
    Identity {
        slot: usize,
        reason: CampaignCargoIdentityUnresolved,
    },
    EntityTypeUnavailable {
        entity_type: u32,
    },
    UnsupportedConstructor {
        entity_type: u32,
    },
    Construction {
        entity_type: u32,
        reason: Class0ActorError,
    },
    GunTurret {
        entity_type: u32,
        reason: crate::intro2_gun_turret::Intro2GunTurretError,
    },
    Type123 {
        entity_type: u32,
        reason: crate::native_type123::Type123Block,
    },
    CleansingVehicle(crate::cleansing_vehicle::CleansingVehicleError),
    Attachment {
        entity_id: u32,
        reason: PlayerCargoAttachmentError,
    },
    AttachedMassUnavailable,
}

/// `451D11..451E29`: exclusions advance; a zero low half ends the list,
/// including a packed word whose high half is nonzero.
fn restored_cargo_identities(
    identities: &[CampaignCargoIdentity],
) -> impl Iterator<Item = (usize, CampaignCargoIdentity)> + '_ {
    identities
        .iter()
        .copied()
        .enumerate()
        .take_while(|(_, identity)| identity.entity_type() != 0)
        .filter(|(_, identity)| {
            !matches!(
                identity.entity_type(),
                7 | 8 | 9 | 78 | 79 | 86 | 90 | 91 | 95 | 116
            )
        })
}

impl EntityManager {
    /// Restore the conventional list and its player-surface/ballast phases.
    /// The separate auxiliary owned-object list at session+184 is not cargo
    /// and still requires its own controller-operation reconstruction.
    ///
    /// This is deliberately sequential: an attachment failure retains earlier
    /// rows and a just-constructed, stamp-overwritten newborn. There is no
    /// capacity preflight for the entire list and no speculative replacement
    /// of an unsupported first identity match.
    pub fn restore_campaign_cargo_controller_state(
        &mut self,
        state: CampaignCargoControllerState,
        mut context: CampaignCargoRestoreContext<'_>,
    ) -> Result<(), CampaignCargoRestoreError> {
        if self.player().is_none() {
            return Err(CampaignCargoRestoreError::MissingPlayer);
        }
        // 43560/418620 restores the controller limit without removing rows.
        // The normal destination player starts empty; replaying into a live
        // list must not invent destruction of its existing attachments.
        let attachments = self
            .player_sub_j_runtime_mut()
            .ok_or(CampaignCargoRestoreError::PlayerAttachmentRuntimeUnavailable)?;
        attachments.set_capacity_clamped(usize::from(state.unlock_raw));
        attachments.set_policy_raw_at_0x0c(1);
        self.player_cargo_unlock_raw = state.unlock_raw;

        self.restore_campaign_player_surface(&context)?;
        context.scheduler.adopt_class0_actors(self);
        // Player attach authenticates Type123 task custody through the
        // scheduler, which is still fresh this early in the load: adopt the
        // authored owners now, exactly like the later main-flow sweep would.
        // Miss newborns register below, right after construction.
        context.scheduler.adopt_native_type123(self);
        for (slot, identity) in restored_cargo_identities(state.carried_identities()) {
            let entity_id = match self
                .find_campaign_cargo_identity(identity)
                .map_err(|reason| CampaignCargoRestoreError::Identity { slot, reason })?
            {
                Some(entity_id) => entity_id,
                None => {
                    let entity_id =
                        self.construct_campaign_cargo(identity.entity_type(), &mut context)?;
                    // 451DFC overwrites the successful body's temporary stamp;
                    // it never changes/refunds the counter consumed by104B0.
                    self.entity_mut(entity_id)
                        .expect("newborn exists")
                        .construction_stamp_at_0xb4 = identity.saved_stamp();
                    entity_id
                }
            };
            self.attach_campaign_cargo(entity_id, &mut context)?;
        }

        // 42EB50 reads Section13+48. This is the world style, not a level id.
        // The signed comparison follows4185C0's wrapping unsigned mass sum.
        if context.level.world_style == 6 {
            let mass = match self.attached_cargo_mass_state() {
                RetailRuntimeValue::Known(mass) => mass as i32,
                RetailRuntimeValue::Unresolved => {
                    return Err(CampaignCargoRestoreError::AttachedMassUnavailable)
                }
            };
            if mass < 200 {
                // Exactly one zero-record birth, even for an empty cargo list.
                // A full slot fails only after the newborn has been committed.
                let ballast = self.construct_campaign_cargo(68, &mut context)?;
                self.attach_campaign_cargo(ballast, &mut context)?;
            }
        }

        // 451EDE walks session+184 until a whole-zero dword. The loop does
        // not inspect 0x38's return before advancing.
        self.restore_campaign_auxiliary_owned_objects(state.auxiliary_owned_types(), &mut context);
        Ok(())
    }

    fn construct_campaign_cargo(
        &mut self,
        entity_type: u32,
        context: &mut CampaignCargoRestoreContext<'_>,
    ) -> Result<u32, CampaignCargoRestoreError> {
        if self.type_runtime_metadata(entity_type).is_none() {
            return Err(CampaignCargoRestoreError::EntityTypeUnavailable { entity_type });
        }
        match entity_type {
            49 => {
                let metadata = self.type_runtime_metadata(49).unwrap().clone();
                crate::cleansing_vehicle::authenticate_metadata(&metadata)
                    .map_err(CampaignCargoRestoreError::CleansingVehicle)?;
                let terrain = context
                    .resources
                    .terrain
                    .ok_or(CampaignCargoRestoreError::TerrainUnavailable)?;
                let surface = crate::entity_initializer::constructor_surface_bits_at_tick(
                    [0; 3],
                    terrain,
                    context.retail_tick,
                    context.level.raw_u32(0x84).expect("descriptor+84") != 0,
                )
                .ok_or(CampaignCargoRestoreError::TerrainUnavailable)?;
                if !matches!(
                    self.next_common_body_ordinal(),
                    RetailRuntimeValue::Known(_)
                ) {
                    return Err(CampaignCargoRestoreError::CleansingVehicle(
                        crate::cleansing_vehicle::CleansingVehicleError::Runtime(
                            "body stamp lineage",
                        ),
                    ));
                }
                let id = self.next_entity_id;
                let stamp = self.begin_common_body_attempt();
                let mut entity = native_instance_body(
                    id,
                    stamp,
                    &metadata,
                    terrain,
                    NativeInstanceBodyRequest::zeroed(49),
                );
                let allocation =
                    observe_main_base_abort_actor(&entity, self.allocation_generation).lease;
                let sub_d = context
                    .world_fx
                    .construct_entity_sub_d(crate::common_mover::sub_d::CLEANSING_VEHICLE_SUB_D);
                crate::cleansing_vehicle::publish_cleansing_vehicle(
                    crate::cleansing_vehicle::CleansingVehicleConstruction {
                        entity: &mut entity,
                        allocation,
                        metadata: &metadata,
                        preceding: &self.entities,
                        terrain,
                        authored_position_raw: [0; 3],
                        spawn_param_nonzero: false,
                        constructor_surface_bits: surface,
                        sub_d,
                    },
                    &mut || u32::from(context.world_fx.next_shared_retail_random_u16()),
                )
                .map_err(CampaignCargoRestoreError::CleansingVehicle)?;
                assert_eq!(self.allocate_live_entity_id(), id);
                self.append_live_entity(entity);
                let owner = crate::cleansing_vehicle::CleansingVehicleOwner::adopt(self, id)
                    .expect("published native cleansing graph");
                context.scheduler.register_cleansing_vehicle(owner);
                Ok(id)
            }
            68 => {
                let owner = self
                    .append_native_zero_record_type68(
                        NativeType68ZeroRecordConstruction {
                            resources: context.resources,
                            retail_tick: context.retail_tick,
                            waves_enabled: context.level.raw_u32(0x84).expect("descriptor+84") != 0,
                        },
                        context.world_fx,
                    )
                    .map_err(|reason| CampaignCargoRestoreError::Construction {
                        entity_type,
                        reason,
                    })?;
                let entity_id = owner.entity_id();
                context.scheduler.register_class0_actor(owner);
                Ok(entity_id)
            }
            92 | 96 | 97 | 100 => {
                self.append_native_zero_record_class29_turret(entity_type, context)
            }
            123 => self.append_native_zero_record_type123(context),
            _ => Err(CampaignCargoRestoreError::UnsupportedConstructor { entity_type }),
        }
    }

    fn append_native_zero_record_class29_turret(
        &mut self,
        entity_type: u32,
        context: &mut CampaignCargoRestoreContext<'_>,
    ) -> Result<u32, CampaignCargoRestoreError> {
        let profile =
            crate::intro2_gun_turret::Intro2GunTurretProfile::for_ordinary_cargo(entity_type)
                .ok_or(CampaignCargoRestoreError::UnsupportedConstructor { entity_type })?;
        let metadata = self
            .type_runtime_metadata(entity_type)
            .cloned()
            .ok_or(CampaignCargoRestoreError::EntityTypeUnavailable { entity_type })?;
        crate::intro2_gun_turret::authenticate_metadata(profile, &metadata).map_err(|reason| {
            CampaignCargoRestoreError::GunTurret {
                entity_type,
                reason,
            }
        })?;
        if !matches!(
            self.next_common_body_ordinal(),
            RetailRuntimeValue::Known(_)
        ) {
            return Err(CampaignCargoRestoreError::Construction {
                entity_type,
                reason: Class0ActorError::Runtime("native body stamp lineage"),
            });
        }
        let terrain = context
            .resources
            .terrain
            .ok_or(CampaignCargoRestoreError::TerrainUnavailable)?;
        let constructor_surface_bits = crate::entity_initializer::constructor_surface_bits_at_tick(
            [0; 3],
            terrain,
            context.retail_tick,
            context.level.raw_u32(0x84).expect("descriptor+84") != 0,
        )
        .ok_or(CampaignCargoRestoreError::TerrainUnavailable)?;
        let id = self.next_entity_id;
        let stamp = self.begin_common_body_attempt();
        let mut entity = native_instance_body(
            id,
            stamp,
            &metadata,
            terrain,
            NativeInstanceBodyRequest::zeroed(entity_type),
        );
        entity.collision.state_flags_at_0x08.overwrite(
            crate::entity_initializer::CONSTRUCTOR_SURFACE_STATE_MASK,
            constructor_surface_bits,
        );
        crate::intro2_gun_turret::publish_ordinary_class29_turret_at_position(
            &mut entity,
            &metadata,
            [0; 3],
            terrain,
            &mut || u32::from(context.world_fx.next_shared_retail_random_u16()),
        )
        .map_err(|reason| CampaignCargoRestoreError::GunTurret {
            entity_type,
            reason,
        })?;
        assert_eq!(self.allocate_live_entity_id(), id);
        self.append_live_entity(entity);
        Ok(id)
    }

    fn append_native_zero_record_type123(
        &mut self,
        context: &mut CampaignCargoRestoreContext<'_>,
    ) -> Result<u32, CampaignCargoRestoreError> {
        const ENTITY_TYPE: u32 = 123;
        let metadata = self.type_runtime_metadata(ENTITY_TYPE).cloned().ok_or(
            CampaignCargoRestoreError::EntityTypeUnavailable {
                entity_type: ENTITY_TYPE,
            },
        )?;
        // No metadata pre-check: the stamp is consumed before 104B0 validates,
        // and a later construction failure does not refund it.
        if !matches!(
            self.next_common_body_ordinal(),
            RetailRuntimeValue::Known(_)
        ) {
            return Err(CampaignCargoRestoreError::Type123 {
                entity_type: ENTITY_TYPE,
                reason: crate::native_type123::Type123Block::Runtime("native body stamp lineage"),
            });
        }
        let terrain = context
            .resources
            .terrain
            .ok_or(CampaignCargoRestoreError::TerrainUnavailable)?;
        let constructor_surface_bits = crate::entity_initializer::constructor_surface_bits_at_tick(
            [0; 3],
            terrain,
            context.retail_tick,
            context.level.raw_u32(0x84).expect("descriptor+84") != 0,
        )
        .ok_or(CampaignCargoRestoreError::TerrainUnavailable)?;
        let id = self.next_entity_id;
        let stamp = self.begin_common_body_attempt();
        let mut entity = native_instance_body(
            id,
            stamp,
            &metadata,
            terrain,
            NativeInstanceBodyRequest::zeroed(ENTITY_TYPE),
        );
        entity.collision.state_flags_at_0x08.overwrite(
            crate::entity_initializer::CONSTRUCTOR_SURFACE_STATE_MASK,
            constructor_surface_bits,
        );
        let allocation = observe_main_base_abort_actor(&entity, self.allocation_generation).lease;
        let sub_d = context
            .world_fx
            .construct_entity_sub_d(crate::common_mover::sub_d::ORDINARY_TYPE90_SUB_D);
        let receipt = crate::native_type123::publish_zero_record_type123(
            crate::native_type123::Type123ZeroRecordConstruction {
                entity: &mut entity,
                allocation,
                metadata: &metadata,
                terrain,
                sub_d,
                constructor_surface_bits,
            },
            context.world_fx,
        )
        .map_err(|reason| CampaignCargoRestoreError::Type123 {
            entity_type: ENTITY_TYPE,
            reason,
        })?;
        // BA40's resource-text event is admitted only in gameplay phase 5;
        // the restore runs before the load drain below, exactly like authored
        // Section-13 construction, so retain the receipt in the shared
        // canonical-owner vec rather than dropping it.
        if let Some(receipt) = receipt {
            self.pending_fresh_level1_type9_resource_text_receipts
                .push(receipt);
        }
        assert_eq!(self.allocate_live_entity_id(), id);
        self.append_live_entity(entity);
        // The following attach authenticates task custody through the
        // scheduler, which will not sweep until after the restore: claim the
        // birth graph now, exactly like the later adoption sweep would.
        let owner = crate::native_type123::Type123Owner::take_birth(
            self.entity_mut(id).expect("newborn exists"),
        )
        .expect("published native Type123 birth graph");
        context.scheduler.register_native_type123(owner);
        Ok(id)
    }

    fn attach_campaign_cargo(
        &mut self,
        entity_id: u32,
        context: &mut CampaignCargoRestoreContext<'_>,
    ) -> Result<(), CampaignCargoRestoreError> {
        self.attach_player_cargo(
            entity_id,
            PlayerCargoAttachmentFrame {
                scheduler: context.scheduler,
                world_fx: context.world_fx,
                notifications: context.notifications,
                retail_tick: context.retail_tick,
            },
        )
        .map_err(|reason| CampaignCargoRestoreError::Attachment { entity_id, reason })
    }

    fn restore_campaign_player_surface(
        &mut self,
        context: &CampaignCargoRestoreContext<'_>,
    ) -> Result<(), CampaignCargoRestoreError> {
        let terrain = context
            .resources
            .terrain
            .ok_or(CampaignCargoRestoreError::TerrainUnavailable)?;
        let player = self
            .player()
            .ok_or(CampaignCargoRestoreError::MissingPlayer)?;
        let player_id = player.id;
        let mut position = player.position_raw();
        let surface_bits = crate::entity_initializer::constructor_surface_bits_at_tick(
            position,
            terrain,
            context.retail_tick,
            context.level.raw_u32(0x84).expect("descriptor+84") != 0,
        )
        .ok_or(CampaignCargoRestoreError::TerrainUnavailable)?;
        // 451C36..451CCA compares the old signed Y with445920 first. That
        // helper returns sea when waves are disabled or the coarse floor is
        // dry (44592A/44593A ->445A2F), exactly as the constructor classifier.
        let player = self.entity_mut(player_id).expect("player remains live");
        player.collision.state_flags_at_0x08.overwrite(
            crate::entity_collision_state::SURFACE_STATE_MASK,
            surface_bits,
        );
        let clearance = i32::from(terrain.bilinear_height_raw(position[0], position[2])) + 0x200;
        if i32::from(position[1]) < clearance {
            position[1] = clearance as i16;
            player.set_position_raw(position);
        }
        // Surface bits deliberately describe the pre-clearance position.
        Ok(())
    }

    fn restore_campaign_auxiliary_owned_objects(
        &mut self,
        types: &[u32],
        context: &mut CampaignCargoRestoreContext<'_>,
    ) {
        for &entity_type in types {
            if entity_type == 0 {
                break;
            }
            // 451C00 does not inspect 413600/0x38 before advancing.
            let _ = self.dispatch_controller_operation_0x38(entity_type, context);
        }
    }

    fn dispatch_controller_operation_0x38(
        &mut self,
        entity_type: u32,
        context: &mut CampaignCargoRestoreContext<'_>,
    ) -> Result<(), CampaignCargoRestoreError> {
        let player = self
            .player()
            .ok_or(CampaignCargoRestoreError::MissingPlayer)?;
        let player_id = player.id;
        let position_raw = player.position_raw();
        let rotation_raw = player.rotation_heading_pitch_roll_raw();
        let velocity_raw = player.velocity_raw();
        let newborn = self.construct_controller_0x38_body(
            entity_type,
            position_raw,
            rotation_raw,
            velocity_raw,
            context,
        )?;
        if let Some(entity) = self.entity_mut(newborn) {
            entity.collision.recent_relation_id_at_0x60 =
                RetailRuntimeValue::Known(Some(player_id));
        }
        let capability = self
            .iter_all()
            .find(|entity| entity.id == newborn)
            .map(|entity| entity.capability_flags)
            .unwrap_or(0);
        if capability & 0x1000 != 0 {
            let _ = self.attach_campaign_cargo(newborn, context);
            return Ok(());
        }
        self.compact_auxiliary_owned_ids();
        let duplicate_type64 = entity_type == 64
            && self.auxiliary_owned_ids.iter().any(|&id| {
                self.iter_all()
                    .find(|entity| entity.id == id)
                    .is_some_and(|entity| entity.entity_type == 64)
            });
        if duplicate_type64 || self.auxiliary_owned_ids.len() > 14 {
            self.stage_auxiliary_limit_destroy(newborn);
            return Ok(());
        }
        self.auxiliary_owned_ids.push(newborn);
        Ok(())
    }

    fn construct_controller_0x38_body(
        &mut self,
        entity_type: u32,
        position_raw: [i16; 3],
        rotation_raw: [i16; 3],
        velocity_raw: [i16; 3],
        context: &mut CampaignCargoRestoreContext<'_>,
    ) -> Result<u32, CampaignCargoRestoreError> {
        match entity_type {
            52 | 68 => {
                let owner = self.append_native_class0_at_pose(
                    entity_type,
                    position_raw,
                    rotation_raw,
                    velocity_raw,
                    context,
                )?;
                let id = owner.entity_id();
                context.scheduler.register_class0_actor(owner);
                Ok(id)
            }
            92 | 96 | 97 | 100 => {
                let id = self.append_native_class29_turret_at_pose(
                    entity_type,
                    position_raw,
                    rotation_raw,
                    velocity_raw,
                    context,
                )?;
                Ok(id)
            }
            _ => Err(CampaignCargoRestoreError::UnsupportedConstructor { entity_type }),
        }
    }

    fn append_native_class0_at_pose(
        &mut self,
        entity_type: u32,
        position_raw: [i16; 3],
        rotation_raw: [i16; 3],
        velocity_raw: [i16; 3],
        context: &mut CampaignCargoRestoreContext<'_>,
    ) -> Result<Class0ActorOwner, CampaignCargoRestoreError> {
        let metadata = self
            .type_runtime_metadata(entity_type)
            .cloned()
            .ok_or(CampaignCargoRestoreError::EntityTypeUnavailable { entity_type })?;
        authenticate_metadata(entity_type, &metadata).map_err(|reason| {
            CampaignCargoRestoreError::Construction {
                entity_type,
                reason,
            }
        })?;
        let terrain = context
            .resources
            .terrain
            .ok_or(CampaignCargoRestoreError::TerrainUnavailable)?;
        let constructor_surface_bits = crate::entity_initializer::constructor_surface_bits_at_tick(
            position_raw,
            terrain,
            context.retail_tick,
            context.level.raw_u32(0x84).expect("descriptor+84") != 0,
        )
        .ok_or(CampaignCargoRestoreError::TerrainUnavailable)?;
        let id = self.next_entity_id;
        let stamp = self.begin_common_body_attempt();
        let mut entity = native_instance_body(
            id,
            stamp,
            &metadata,
            terrain,
            NativeInstanceBodyRequest::zeroed(entity_type),
        );
        entity.set_position_raw(position_raw);
        entity.set_rotation_heading_pitch_roll_raw(rotation_raw);
        entity.set_velocity_raw(velocity_raw);
        let allocation = observe_main_base_abort_actor(&entity, self.allocation_generation).lease;
        let runtime = construct_class0_actor(
            &mut entity,
            Class0ActorConstruction {
                allocation,
                metadata: &metadata,
                spawn: Class0SpawnInput::AtPose {
                    entity_type,
                    position_raw,
                    rotation_raw,
                    velocity_raw,
                },
                resources: context.resources,
                constructor_surface_bits,
            },
            &mut || u32::from(context.world_fx.next_shared_retail_random_u16()),
        )
        .map_err(|reason| CampaignCargoRestoreError::Construction {
            entity_type,
            reason,
        })?;
        assert_eq!(self.allocate_live_entity_id(), id);
        self.append_live_entity(entity);
        self.native_class0_actors.insert(id, runtime);
        Class0ActorOwner::adopt(self, id).map_err(|reason| {
            CampaignCargoRestoreError::Construction {
                entity_type,
                reason,
            }
        })
    }

    fn append_native_class29_turret_at_pose(
        &mut self,
        entity_type: u32,
        position_raw: [i16; 3],
        rotation_raw: [i16; 3],
        velocity_raw: [i16; 3],
        context: &mut CampaignCargoRestoreContext<'_>,
    ) -> Result<u32, CampaignCargoRestoreError> {
        let profile =
            crate::intro2_gun_turret::Intro2GunTurretProfile::for_ordinary_cargo(entity_type)
                .ok_or(CampaignCargoRestoreError::UnsupportedConstructor { entity_type })?;
        let metadata = self
            .type_runtime_metadata(entity_type)
            .cloned()
            .ok_or(CampaignCargoRestoreError::EntityTypeUnavailable { entity_type })?;
        crate::intro2_gun_turret::authenticate_metadata(profile, &metadata).map_err(|reason| {
            CampaignCargoRestoreError::GunTurret {
                entity_type,
                reason,
            }
        })?;
        let terrain = context
            .resources
            .terrain
            .ok_or(CampaignCargoRestoreError::TerrainUnavailable)?;
        let constructor_surface_bits = crate::entity_initializer::constructor_surface_bits_at_tick(
            position_raw,
            terrain,
            context.retail_tick,
            context.level.raw_u32(0x84).expect("descriptor+84") != 0,
        )
        .ok_or(CampaignCargoRestoreError::TerrainUnavailable)?;
        let id = self.next_entity_id;
        let stamp = self.begin_common_body_attempt();
        let mut entity = native_instance_body(
            id,
            stamp,
            &metadata,
            terrain,
            NativeInstanceBodyRequest::zeroed(entity_type),
        );
        entity.set_position_raw(position_raw);
        entity.set_rotation_heading_pitch_roll_raw(rotation_raw);
        entity.set_velocity_raw(velocity_raw);
        entity.apply_d720_euler_body_basis();
        entity.collision.state_flags_at_0x08.overwrite(
            crate::entity_initializer::CONSTRUCTOR_SURFACE_STATE_MASK,
            constructor_surface_bits,
        );
        crate::intro2_gun_turret::publish_ordinary_class29_turret_at_position(
            &mut entity,
            &metadata,
            position_raw,
            terrain,
            &mut || u32::from(context.world_fx.next_shared_retail_random_u16()),
        )
        .map_err(|reason| CampaignCargoRestoreError::GunTurret {
            entity_type,
            reason,
        })?;
        assert_eq!(self.allocate_live_entity_id(), id);
        self.append_live_entity(entity);
        Ok(id)
    }

    fn compact_auxiliary_owned_ids(&mut self) {
        let live: Vec<u32> = self
            .auxiliary_owned_ids
            .iter()
            .copied()
            .filter(|&id| {
                self.iter_all()
                    .find(|entity| entity.id == id)
                    .is_some_and(|entity| {
                        entity.active
                            && entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT)
                                == RetailRuntimeValue::Known(0)
                    })
            })
            .collect();
        self.auxiliary_owned_ids = live;
    }

    fn stage_auxiliary_limit_destroy(&mut self, entity_id: u32) {
        if let Some(entity) = self.entity_mut(entity_id) {
            entity.mark_actor_deferred_destroy_pending();
        }
        self.queue_actor_deferred_destroy(entity_id);
    }

    pub(crate) fn auxiliary_owned_types(&self) -> Vec<u32> {
        self.auxiliary_owned_ids
            .iter()
            .filter_map(|&id| {
                self.iter_all()
                    .find(|entity| {
                        entity.id == id
                            && entity.active
                            && entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT)
                                == RetailRuntimeValue::Known(0)
                    })
                    .map(|entity| entity.entity_type)
            })
            .collect()
    }

    pub fn auxiliary_owned_entity_ids(&self) -> &[u32] {
        &self.auxiliary_owned_ids
    }
}
