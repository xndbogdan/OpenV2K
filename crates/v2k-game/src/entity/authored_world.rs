//! Ordinary authored-world publication (`104B0 -> 09A80 -> D720`).
//!
//! Components and initializers complete before the next authored allocation is
//! exposed to nearby selectors. Session RNG and Sub-D allocation history belong
//! to the caller's persistent process owner.

use super::*;
use crate::native_type86::NativeFourChoiceProfile;
use v2k_formats::levels::EntitySpawn;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthoredPlayerArrival {
    pub position_raw: [i16; 3],
    pub heading_raw: u16,
}

pub struct AuthoredWorldConstruction<'a> {
    pub level: &'a LevelDescriptor,
    /// Raw controller+C4, independently of the bounded campaign trophy slot.
    pub logical_world_index: i32,
    pub type_metadata: &'a [EntityTypeRuntimeMetadata],
    pub resources: EntityConstructionResources<'a>,
    pub player_arrival: Option<AuthoredPlayerArrival>,
    pub retail_tick: u32,
}

/// Player publication differs only at the controller-restore boundary before
/// authored actors. Keep both routes on the same native world constructor.
pub enum AuthoredWorldLoadRequest<'a> {
    Ordinary {
        world: AuthoredWorldConstruction<'a>,
        exit_marker_bits: u32,
    },
    NativeSave {
        world: AuthoredWorldConstruction<'a>,
        saved: NativeSaveWorldState<'a>,
    },
}

/// Isolated mutable terrain is committed to the cache only after a successful
/// load. Every following actor reads this same working allocation.
pub struct NativeSaveWorldState<'a> {
    pub restore: &'a crate::save::NativeSaveRestore,
    pub terrain: &'a mut TerrainGrid,
    pub resources: &'a crate::resource_cache::ResourceCache,
    pub static_damage: &'a mut crate::static_damage::StaticDamageScheduler,
}

impl<'a> From<AuthoredWorldConstruction<'a>> for AuthoredWorldLoadRequest<'a> {
    fn from(world: AuthoredWorldConstruction<'a>) -> Self {
        Self::Ordinary {
            world,
            exit_marker_bits: 0,
        }
    }
}

impl<'a> AuthoredWorldConstruction<'a> {
    /// Retain42EF60's current controller word for terrain helper publication.
    pub fn with_exit_marker_bits(self, exit_marker_bits: u32) -> AuthoredWorldLoadRequest<'a> {
        AuthoredWorldLoadRequest::Ordinary {
            world: self,
            exit_marker_bits,
        }
    }

    pub fn with_native_save(self, saved: NativeSaveWorldState<'a>) -> AuthoredWorldLoadRequest<'a> {
        AuthoredWorldLoadRequest::NativeSave { world: self, saved }
    }
}

impl EntityManager {
    pub fn from_authored_world<'a>(
        request: impl Into<AuthoredWorldLoadRequest<'a>>,
        world_fx: &mut WorldFx,
    ) -> Result<Self, FreshNewGameEntityConstructionError> {
        let (mut request, restored_player, completed_world, exit_marker_bits) = match request.into()
        {
            AuthoredWorldLoadRequest::Ordinary {
                world,
                exit_marker_bits,
            } => (world, None, None, exit_marker_bits),
            AuthoredWorldLoadRequest::NativeSave { world, saved } => {
                let exit_marker_bits = saved
                    .restore
                    .campaign
                    .control_slot_bits(saved.restore.logical_level_id as usize)
                    .unwrap_or(0);
                (
                    world,
                    Some(NativeSavedPlayerConstruction {
                        player: saved.restore.player,
                        cargo_unlock_raw: saved.restore.cargo.unlock_raw,
                        pre_health_damage_buffer_raw: saved.restore.pre_health_damage_buffer_raw,
                        current_world_flags: saved
                            .restore
                            .campaign
                            .control_slot_bits(saved.restore.logical_level_id as usize)
                            .unwrap_or(0),
                    }),
                    Some(saved),
                    exit_marker_bits,
                )
            }
        };
        if let Some(saved) = restored_player {
            request.player_arrival = Some(AuthoredPlayerArrival {
                position_raw: saved.player.position_raw,
                heading_raw: saved.player.heading_raw,
            });
        }
        Self::from_level_with_type_metadata_and_provenance(
            request.level,
            request.type_metadata,
            request.resources,
            LevelEntityConstructionProvenance::NativeOrdinary {
                player: request.player_arrival,
                logical_world_index: request.logical_world_index,
                restored_player,
                exit_marker_bits,
            },
            Some((request.retail_tick, world_fx)),
            completed_world,
        )
    }

    pub(super) fn publish_native_authored_actor(
        &mut self,
        mut entity: Entity,
        metadata: Option<&EntityTypeRuntimeMetadata>,
        spawn: &EntitySpawn,
        resources: EntityConstructionResources<'_>,
        retail_tick: u32,
        waves_enabled: bool,
        sub_d: Option<crate::common_mover::sub_d::NativeSubDConstruction>,
        world_fx: &mut WorldFx,
    ) -> Result<(), String> {
        let metadata = metadata.ok_or("missing type metadata")?;
        let terrain = resources.terrain.ok_or("missing constructor terrain")?;
        let allocation = observe_main_base_abort_actor(&entity, self.allocation_generation).lease;
        match entity.entity_type {
            61 => {
                let surface = crate::entity_initializer::constructor_surface_bits_at_tick(
                    spawn.position_raw(),
                    terrain,
                    retail_tick,
                    waves_enabled,
                )
                .ok_or("Type61 constructor surface")?;
                crate::native_type61::publish_authored_power_up(
                    crate::native_type61::AuthoredPowerUpConstruction {
                        entity: &mut entity,
                        allocation,
                        metadata,
                        spawn,
                        terrain,
                        constructor_surface_bits: surface,
                    },
                    world_fx,
                )?;
            }
            92 | 96 | 97 | 99 | 102 | 103 | 104 | 112 | 113 | 115 => {
                let constructor_surface_bits =
                    crate::entity_initializer::constructor_surface_bits_at_tick(
                        spawn.position_raw(),
                        terrain,
                        retail_tick,
                        waves_enabled,
                    )
                    .ok_or("E/L gun turret constructor surface")?;
                crate::intro2_gun_turret::publish_authored_gun_turret(
                    crate::intro2_gun_turret::GunTurretAuthoredConstruction {
                        entity: &mut entity,
                        metadata,
                        allocation,
                        spawn,
                        terrain,
                        constructor_surface_bits,
                    },
                    &mut || u32::from(world_fx.next_shared_retail_random_u16()),
                )
                .map_err(|error| format!("E/L gun turret: {error:?}"))?;
            }
            49 => {
                if spawn.has_animation
                    || spawn.animation.is_some()
                    || spawn.has_config
                    || spawn.config.is_some()
                {
                    return Err("unsupported Type49 authored override".into());
                }
                let sub_d = sub_d.ok_or("missing Type49 Sub-D allocation")?;
                let constructor_surface_bits =
                    crate::entity_initializer::constructor_surface_bits_at_tick(
                        spawn.position_raw(),
                        terrain,
                        retail_tick,
                        waves_enabled,
                    )
                    .ok_or("Type49 constructor surface")?;
                crate::cleansing_vehicle::publish_cleansing_vehicle(
                    crate::cleansing_vehicle::CleansingVehicleConstruction {
                        entity: &mut entity,
                        allocation,
                        metadata,
                        preceding: &self.entities,
                        terrain,
                        authored_position_raw: spawn.position_raw(),
                        spawn_param_nonzero: spawn.param != 0,
                        constructor_surface_bits,
                        sub_d,
                    },
                    &mut || u32::from(world_fx.next_shared_retail_random_u16()),
                )
                .map_err(|error| format!("Type49: {error:?}"))?;
            }
            22 | 23 | 24 | 62 | 124 => {
                let sub_d = sub_d.ok_or("missing fish Sub-D allocation")?;
                let constructor_surface_bits =
                    crate::entity_initializer::constructor_surface_bits_at_tick(
                        spawn.position_raw(),
                        terrain,
                        retail_tick,
                        waves_enabled,
                    )
                    .ok_or("fish constructor surface")?;
                crate::shared_fish::publish_authored_fish(
                    crate::shared_fish::FishAuthoredConstruction {
                        entity: &mut entity,
                        allocation,
                        metadata,
                        spawn,
                        resources,
                        constructor_surface_bits,
                        sub_d,
                    },
                    &mut || u32::from(world_fx.next_shared_retail_random_u16()),
                )
                .map_err(|error| format!("fish: {error:?}"))?;
            }
            58 => {
                let sub_d = sub_d.ok_or("missing Type58 Sub-D allocation")?;
                let constructor_surface_bits =
                    crate::entity_initializer::constructor_surface_bits_at_tick(
                        spawn.position_raw(),
                        terrain,
                        retail_tick,
                        waves_enabled,
                    )
                    .ok_or("Type58 constructor surface")?;
                crate::intro2_type58::publish_authored_type58(
                    crate::intro2_type58::Type58AuthoredConstructionRequest {
                        entity: &mut entity,
                        allocation,
                        metadata,
                        spawn,
                        preceding: &self.entities,
                        resources,
                        constructor_surface_bits,
                        sub_d,
                    },
                    &mut || u32::from(world_fx.next_shared_retail_random_u16()),
                )
                .map_err(|error| format!("Type58: {error:?}"))?;
            }
            53 => {
                let sub_d = sub_d.ok_or("missing Type53 Sub-D allocation")?;
                let constructor_surface_bits =
                    crate::entity_initializer::constructor_surface_bits_at_tick(
                        spawn.position_raw(),
                        terrain,
                        retail_tick,
                        waves_enabled,
                    )
                    .ok_or("Type53 constructor surface")?;
                crate::intro2_type53::publish_authored_type53(
                    crate::intro2_type53::Type53AuthoredConstructionRequest {
                        entity: &mut entity,
                        allocation,
                        metadata,
                        spawn,
                        preceding: &self.entities,
                        resources,
                        constructor_surface_bits,
                        retail_tick,
                        sub_d,
                    },
                    &mut || u32::from(world_fx.next_shared_retail_random_u16()),
                )
                .map_err(|error| format!("Type53: {error:?}"))?;
            }
            30 => {
                let sub_d = sub_d.ok_or("missing Type30 Sub-D allocation")?;
                let constructor_surface_bits =
                    crate::entity_initializer::constructor_surface_bits_at_tick(
                        spawn.position_raw(),
                        terrain,
                        retail_tick,
                        waves_enabled,
                    )
                    .ok_or("Type30 constructor surface")?;
                crate::native_type30::publish_authored_type30(
                    crate::native_type30::Type30AuthoredConstructionRequest {
                        entity: &mut entity,
                        allocation,
                        metadata,
                        spawn,
                        preceding: &self.entities,
                        resources,
                        constructor_surface_bits,
                        sub_d,
                    },
                    &mut || u32::from(world_fx.next_shared_retail_random_u16()),
                )
                .map_err(|error| format!("Type30: {error:?}"))?;
            }
            40 => {
                let sub_d = sub_d.ok_or("missing Type40 Sub-D allocation")?;
                let constructor_surface_bits =
                    crate::entity_initializer::constructor_surface_bits_at_tick(
                        spawn.position_raw(),
                        terrain,
                        retail_tick,
                        waves_enabled,
                    )
                    .ok_or("Type40 constructor surface")?;
                crate::native_type40::publish_native_type40(
                    crate::native_type40::NativeType40ConstructionRequest {
                        entity: &mut entity,
                        allocation,
                        metadata,
                        spawn,
                        preceding: &self.entities,
                        resources,
                        constructor_surface_bits,
                        retail_tick,
                        sub_d,
                    },
                    &mut || u32::from(world_fx.next_shared_retail_random_u16()),
                )
                .map_err(|error| format!("Type40: {error:?}"))?;
            }
            122 => {
                let sub_d = sub_d.ok_or("missing Type122 Sub-D allocation")?;
                let constructor_surface_bits =
                    crate::entity_initializer::constructor_surface_bits_at_tick(
                        spawn.position_raw(),
                        terrain,
                        retail_tick,
                        waves_enabled,
                    )
                    .ok_or("Type122 constructor surface")?;
                crate::native_type122::publish_native_type122(
                    crate::native_type122::NativeType122ConstructionRequest {
                        entity: &mut entity,
                        allocation,
                        metadata,
                        spawn,
                        preceding: &self.entities,
                        resources,
                        constructor_surface_bits,
                        retail_tick,
                        sub_d,
                    },
                    &mut || u32::from(world_fx.next_shared_retail_random_u16()),
                )
                .map_err(|error| format!("Type122: {error:?}"))?;
            }
            26 => {
                let sub_d = sub_d.ok_or("missing Type26 Sub-D allocation")?;
                let constructor_surface_bits =
                    crate::entity_initializer::constructor_surface_bits_at_tick(
                        spawn.position_raw(),
                        terrain,
                        retail_tick,
                        waves_enabled,
                    )
                    .ok_or("Type26 constructor surface")?;
                crate::intro2_type26_defecate_virus::publish_authored_type26(
                    crate::intro2_type26_defecate_virus::Type26AuthoredConstruction {
                        entity: &mut entity,
                        allocation,
                        metadata,
                        spawn,
                        resources,
                        constructor_surface_bits,
                        sub_d,
                    },
                    &mut || u32::from(world_fx.next_shared_retail_random_u16()),
                )
                .map_err(|error| format!("Type26: {error:?}"))?;
            }
            47 => {
                let sub_d = sub_d.ok_or("missing Type47 Sub-D allocation")?;
                let constructor_surface_bits =
                    crate::entity_initializer::constructor_surface_bits_at_tick(
                        spawn.position_raw(),
                        terrain,
                        retail_tick,
                        waves_enabled,
                    )
                    .ok_or("Type47 constructor surface")?;
                crate::shared_type47::publish_authored_type47(
                    crate::shared_type47::Type47AuthoredConstructionRequest {
                        entity: &mut entity,
                        allocation,
                        metadata,
                        spawn,
                        resources,
                        constructor_surface_bits,
                        sub_d,
                    },
                    &mut || u32::from(world_fx.next_shared_retail_random_u16()),
                )
                .map_err(|error| format!("Type47: {error:?}"))?;
            }
            17 => {
                let sub_d = sub_d.ok_or("missing Type17 Sub-D allocation")?;
                let constructor_surface_bits =
                    crate::entity_initializer::constructor_surface_bits_at_tick(
                        spawn.position_raw(),
                        terrain,
                        retail_tick,
                        waves_enabled,
                    )
                    .ok_or("Type17 constructor surface")?;
                crate::intro2_type17::publish_authored_type17(
                    crate::intro2_type17::Type17AuthoredConstructionRequest {
                        entity: &mut entity,
                        allocation,
                        metadata,
                        spawn,
                        preceding: &self.entities,
                        resources,
                        constructor_surface_bits,
                        retail_tick,
                        sub_d,
                    },
                    &mut || u32::from(world_fx.next_shared_retail_random_u16()),
                )
                .map_err(|error| format!("Type17: {error:?}"))?;
            }
            52 | 54 | 68 => {
                if self.native_class0_construction_present(entity.id) {
                    return Err("class0 allocation already published".into());
                }
                let constructor_surface_bits =
                    crate::entity_initializer::constructor_surface_bits_at_tick(
                        spawn.position_raw(),
                        terrain,
                        retail_tick,
                        waves_enabled,
                    )
                    .ok_or("class0 constructor surface")?;
                let runtime = super::class0_actor::construct_class0_actor(
                    &mut entity,
                    super::class0_actor::Class0ActorConstruction {
                        allocation,
                        metadata,
                        spawn: super::class0_actor::Class0SpawnInput::Authored(spawn),
                        resources,
                        constructor_surface_bits,
                    },
                    &mut || u32::from(world_fx.next_shared_retail_random_u16()),
                )
                .map_err(|error| format!("class0: {error:?}"))?;
                self.append_live_entity(entity);
                self.native_class0_actors
                    .insert(allocation.entity_id, runtime);
                return Ok(());
            }
            entity_type if NativeWorkerProfile::from_entity_type(entity_type).is_some() => {
                let sub_d = sub_d.ok_or("missing worker Sub-D allocation")?;
                let surface = crate::entity_initializer::constructor_surface_bits_at_tick(
                    spawn.position_raw(),
                    terrain,
                    retail_tick,
                    waves_enabled,
                )
                .ok_or("worker constructor surface")?;
                crate::intro2_type8::publish_authored_type8(
                    crate::intro2_type8::Type8AuthoredConstructionRequest {
                        entity: &mut entity,
                        allocation,
                        metadata,
                        spawn,
                        preceding: &self.entities,
                        terrain,
                        sub_d,
                        constructor_surface_bits: surface,
                    },
                    &mut || u32::from(world_fx.next_shared_retail_random_u16()),
                )
                .map_err(|error| format!("worker: {error:?}"))?;
            }
            entity_type if NativeFourChoiceProfile::from_entity_type(entity_type).is_some() => {
                let sub_d = sub_d.ok_or("missing person Sub-D allocation")?;
                let surface = crate::entity_initializer::constructor_surface_bits_at_tick(
                    spawn.position_raw(),
                    terrain,
                    retail_tick,
                    waves_enabled,
                )
                .ok_or("person constructor surface")?;
                let receipt = crate::native_type86::publish_authored_type86(
                    crate::native_type86::Type86AuthoredConstructionRequest {
                        entity: &mut entity,
                        allocation,
                        metadata,
                        spawn,
                        preceding: &self.entities,
                        terrain,
                        sub_d,
                        constructor_surface_bits: surface,
                    },
                    world_fx,
                )
                .map_err(|error| format!("person {entity_type}: {error:?}"))?;
                // BA40's resource-text event is admitted only in gameplay
                // phase 5. Ordinary native Section-13 construction qualifies;
                // retain the receipt in the shared canonical-owner vec for
                // the load drain below, exactly like fresh Type9 births.
                if let Some(receipt) = receipt {
                    self.pending_fresh_level1_type9_resource_text_receipts
                        .push(receipt);
                }
            }
            123 => {
                let sub_d = sub_d.ok_or("missing Type123 Sub-D allocation")?;
                let surface = crate::entity_initializer::constructor_surface_bits_without_wave(
                    spawn.position_raw(),
                    terrain,
                )
                .ok_or("Type123 constructor requires current wave surface")?;
                let receipt = crate::native_type123::publish_authored_type123(
                    crate::native_type123::Type123AuthoredConstructionRequest {
                        entity: &mut entity,
                        allocation,
                        metadata,
                        spawn,
                        preceding: &self.entities,
                        terrain,
                        sub_d,
                        constructor_surface_bits: surface,
                    },
                    world_fx,
                )
                .map_err(|error| format!("Type123: {error:?}"))?;
                // BA40's resource-text event is admitted only in gameplay
                // phase 5. Ordinary native Section-13 construction qualifies;
                // retain the receipt in the shared canonical-owner vec for
                // the load drain below, exactly like fresh Type9 births.
                if let Some(receipt) = receipt {
                    self.pending_fresh_level1_type9_resource_text_receipts
                        .push(receipt);
                }
            }
            9 => {
                let sub_d = sub_d.ok_or("missing Type9 Sub-D allocation")?;
                let surface = crate::entity_initializer::constructor_surface_bits_without_wave(
                    spawn.position_raw(),
                    terrain,
                )
                .ok_or("Type9 constructor requires current wave surface")?;
                let ready = crate::ordinary_type9_construction::construct_ordinary_type9(
                    crate::ordinary_type9_construction::OrdinaryType9ConstructionRequest {
                        entity,
                        metadata,
                        spawn,
                        preceding: &self.entities,
                        terrain,
                        allocation,
                        sub_d_frame_owner: sub_d.frame_owner,
                        sub_d_runtime: sub_d.runtime,
                        constructor_surface_bits: surface,
                    },
                    world_fx,
                )
                .map_err(|error| format!("Type9: {error:?}"))?;
                self.append_and_finalize_fresh_level1_type9(ready);
                return Ok(());
            }
            66 => {
                crate::intro2_type66::publish_working_factory(
                    &mut entity,
                    allocation,
                    metadata,
                    spawn,
                    terrain,
                    &mut || u32::from(world_fx.next_shared_retail_random_u16()),
                )
                .map_err(|error| format!("working factory: {error:?}"))?;
            }
            6 => {
                crate::main_base_runtime::publish_main_base(
                    &mut entity,
                    allocation,
                    metadata,
                    spawn,
                    terrain,
                    &mut || u32::from(world_fx.next_shared_retail_random_u16()),
                )
                .map_err(|error| format!("Main Base: {error:?}"))?;
            }
            _ => unreachable!("shared authored family dispatch"),
        }
        self.append_live_entity(entity);
        Ok(())
    }
}
