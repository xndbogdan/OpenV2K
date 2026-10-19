//! 1BEB0's populated zero0x4C creature request ->438080/104B0/D4A0.

use super::*;
use crate::{
    hive_birth::HiveBirthRequest,
    intro2_flyers_live::{
        authenticate_native_flyer_body, authenticate_native_flyer_metadata, publish_native_flyer,
        Intro2FlyerPublicationError, Intro2FlyerSchedulerOwner, NativeFlyerConstructionRequest,
    },
    native_ground_actor::NativeGroundActorBlock,
    native_type18::{Type18Error, Type18Owner},
    native_type38::{Type38Block, Type38FamilyOwner, Type38Row},
    native_type76::{Type76Error, Type76FamilyOwner, Type76Row},
};

/// A Hive child's task owner, appended to the scheduler in the birth's pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeHiveNewborn {
    Flyer(Intro2FlyerSchedulerOwner),
    Type18(Type18Owner),
    Type38Family(Type38FamilyOwner),
    Type76Family(Type76FamilyOwner),
}

impl NativeHiveNewborn {
    pub fn entity_id(self) -> u32 {
        match self {
            Self::Flyer(owner) => owner.entity_id(),
            Self::Type18(owner) => owner.entity_id(),
            Self::Type38Family(owner) => owner.entity_id(),
            Self::Type76Family(owner) => owner.entity_id(),
        }
    }
}

/// The ground-host families whose shared constructor takes a zeroed record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HiveGroundFamily {
    Type18,
    Type38(Type38Row),
    Type76(Type76Row),
}

impl HiveGroundFamily {
    fn from_entity_type(entity_type: u32) -> Option<Self> {
        match entity_type {
            18 => Some(Self::Type18),
            _ => Type38Row::from_entity_type(entity_type)
                .map(Self::Type38)
                .or_else(|| Type76Row::from_entity_type(entity_type).map(Self::Type76)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HiveGroundFamilyError {
    Type18(Type18Error),
    Type38(Type38Block),
    Type76(Type76Error),
}

#[derive(Clone, Copy)]
pub(crate) struct NativeHiveChildConstructionContext<'a> {
    pub resources: EntityConstructionResources<'a>,
    pub retail_tick: u32,
    /// Actual controller+A4; independent of water rendering visibility.
    pub waves_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HiveNativeBirthBlock {
    UnsupportedEntityType {
        entity_type: u32,
    },
    Runtime(&'static str),
    Metadata(Intro2FlyerPublicationError),
    /// Defensive failure after104B0 entry. The body ordinal/SubD allocation
    /// and any reached family words remain consumed; never replay this prefix
    /// or reinterpret it as a proven retail allocator-error return.
    Publication {
        reason: Intro2FlyerPublicationError,
        committed_constructor_prefix: bool,
    },
    /// A ground family's metadata refused before104B0 entry.
    GroundMetadata(HiveGroundFamilyError),
    /// A ground family's construction or task adoption failed after104B0
    /// entry: a committed prefix, as `Publication`.
    GroundConstruction(HiveGroundFamilyError),
    GroundAdoption(NativeGroundActorBlock),
}

impl HiveNativeBirthBlock {
    /// The body ordinal, Sub-D and any family words stay consumed.
    pub(crate) const fn committed_constructor_prefix(&self) -> bool {
        matches!(
            self,
            Self::Publication {
                committed_constructor_prefix: true,
                ..
            } | Self::GroundConstruction(_)
                | Self::GroundAdoption(_)
        )
    }
}

impl EntityManager {
    pub(crate) fn append_native_hive_child(
        &mut self,
        request: HiveBirthRequest,
        context: NativeHiveChildConstructionContext<'_>,
        world_fx: &mut WorldFx,
    ) -> Result<NativeHiveNewborn, HiveNativeBirthBlock> {
        if let Some(family) = HiveGroundFamily::from_entity_type(request.entity_type) {
            return self.append_native_hive_ground_actor(family, request, context, world_fx);
        }
        if crate::intro2_flyers_live::native_flyer_model_id(request.entity_type).is_none() {
            return Err(HiveNativeBirthBlock::UnsupportedEntityType {
                entity_type: request.entity_type,
            });
        }
        self.append_native_hive_flyer(request, context, world_fx)
            .map(NativeHiveNewborn::Flyer)
    }

    /// The zeroed record through a ground family's shared constructor:
    /// 104B0's body ordinal, then 09A80's process-wide Sub-D, then the family's
    /// RNG, D4A0 anchor and initial selection over the current live list, as
    /// an authored birth with no index.
    fn append_native_hive_ground_actor(
        &mut self,
        family: HiveGroundFamily,
        request: HiveBirthRequest,
        context: NativeHiveChildConstructionContext<'_>,
        world_fx: &mut WorldFx,
    ) -> Result<NativeHiveNewborn, HiveNativeBirthBlock> {
        let metadata = self
            .type_metadata
            .get(request.entity_type as usize)
            .ok_or(HiveNativeBirthBlock::Runtime(
                "native ground-family metadata",
            ))?
            .clone();
        match family {
            HiveGroundFamily::Type18 => crate::native_type18::authenticate_metadata(&metadata)
                .map_err(HiveGroundFamilyError::Type18),
            HiveGroundFamily::Type38(row) => {
                crate::native_type38::authenticate_metadata(row, &metadata)
                    .map_err(HiveGroundFamilyError::Type38)
            }
            HiveGroundFamily::Type76(row) => {
                crate::native_type76::authenticate_metadata(row, &metadata)
                    .map_err(HiveGroundFamilyError::Type76)
            }
        }
        .map_err(HiveNativeBirthBlock::GroundMetadata)?;
        let RetailRuntimeValue::Known(Some(descriptor)) = metadata.sub_d_steering_descriptor else {
            return Err(HiveNativeBirthBlock::Runtime("native ground-family Sub-D"));
        };
        if !matches!(
            self.next_common_body_ordinal(),
            RetailRuntimeValue::Known(_)
        ) {
            return Err(HiveNativeBirthBlock::Runtime("native body stamp lineage"));
        }
        let terrain = context
            .resources
            .terrain
            .filter(|terrain| {
                terrain.cells.len()
                    == v2k_formats::terrain::GRID_SIZE * v2k_formats::terrain::GRID_SIZE
            })
            .ok_or(HiveNativeBirthBlock::Runtime(
                "native ground-family complete terrain",
            ))?;
        let constructor_surface_bits = crate::entity_initializer::constructor_surface_bits_at_tick(
            request.position_raw,
            terrain,
            context.retail_tick,
            context.waves_enabled,
        )
        .ok_or(HiveNativeBirthBlock::Runtime(
            "native ground-family constructor surface",
        ))?;
        let id = self.next_entity_id;
        if id == 0 || self.entities.iter().any(|entity| entity.id == id) {
            return Err(HiveNativeBirthBlock::Runtime(
                "native ground-family allocation handle",
            ));
        }
        let body = NativeInstanceBodyRequest {
            entity_type: request.entity_type,
            position_raw: request.position_raw,
            objective: request.objective,
        };
        let stamp = self.begin_common_body_attempt();
        let sub_d = world_fx.construct_entity_sub_d(descriptor);
        let mut entity = native_instance_body(id, stamp, &metadata, terrain, body);
        let allocation = observe_main_base_abort_actor(&entity, self.allocation_generation).lease;
        let record = NativeCreationRecord::zeroed(body);
        let resources = context.resources;
        let next_random = &mut || u32::from(world_fx.next_shared_retail_random_u16());
        match family {
            HiveGroundFamily::Type18 => crate::native_type18::publish_native_type18(
                crate::native_type18::NativeType18ConstructionRequest {
                    entity: &mut entity,
                    allocation,
                    metadata: &metadata,
                    record,
                    preceding: &self.entities,
                    resources,
                    constructor_surface_bits,
                    sub_d,
                },
                next_random,
            )
            .map(drop)
            .map_err(HiveGroundFamilyError::Type18),
            HiveGroundFamily::Type38(_) => crate::native_type38::publish_authored_type38(
                crate::native_type38::Type38AuthoredConstructionRequest {
                    entity: &mut entity,
                    allocation,
                    metadata: &metadata,
                    record,
                    preceding: &self.entities,
                    resources,
                    constructor_surface_bits,
                    sub_d,
                },
                next_random,
            )
            .map(drop)
            .map_err(HiveGroundFamilyError::Type38),
            HiveGroundFamily::Type76(_) => crate::native_type76::publish_native_type76(
                crate::native_type76::NativeType76ConstructionRequest {
                    entity: &mut entity,
                    allocation,
                    metadata: &metadata,
                    record,
                    resources,
                    constructor_surface_bits,
                    retail_tick: context.retail_tick,
                    sub_d,
                },
                next_random,
            )
            .map(drop)
            .map_err(HiveGroundFamilyError::Type76),
        }
        .map_err(HiveNativeBirthBlock::GroundConstruction)?;
        assert_eq!(self.allocate_live_entity_id(), id);
        self.append_live_entity(entity);
        match family {
            HiveGroundFamily::Type18 => Type18Owner::adopt(self, id).map(NativeHiveNewborn::Type18),
            HiveGroundFamily::Type38(_) => {
                Type38FamilyOwner::adopt(self, id).map(NativeHiveNewborn::Type38Family)
            }
            HiveGroundFamily::Type76(_) => {
                Type76FamilyOwner::adopt(self, id).map(NativeHiveNewborn::Type76Family)
            }
        }
        .map_err(HiveNativeBirthBlock::GroundAdoption)
    }

    fn append_native_hive_flyer(
        &mut self,
        request: HiveBirthRequest,
        context: NativeHiveChildConstructionContext<'_>,
        world_fx: &mut WorldFx,
    ) -> Result<Intro2FlyerSchedulerOwner, HiveNativeBirthBlock> {
        let metadata = self
            .type_metadata
            .get(request.entity_type as usize)
            .ok_or(HiveNativeBirthBlock::Runtime("native flyer metadata"))?
            .clone();
        authenticate_native_flyer_metadata(request.entity_type, &metadata)
            .map_err(HiveNativeBirthBlock::Metadata)?;
        if !matches!(
            self.next_common_body_ordinal(),
            RetailRuntimeValue::Known(_)
        ) {
            return Err(HiveNativeBirthBlock::Runtime("native body stamp lineage"));
        }
        let terrain = context
            .resources
            .terrain
            .ok_or(HiveNativeBirthBlock::Runtime("native flyer terrain"))?;
        if terrain.cells.len() != v2k_formats::terrain::GRID_SIZE * v2k_formats::terrain::GRID_SIZE
        {
            return Err(HiveNativeBirthBlock::Runtime(
                "native flyer complete terrain",
            ));
        }
        let constructor_surface_bits = crate::entity_initializer::constructor_surface_bits_at_tick(
            request.position_raw,
            terrain,
            context.retail_tick,
            context.waves_enabled,
        )
        .ok_or(HiveNativeBirthBlock::Runtime(
            "native flyer constructor surface",
        ))?;
        let id = self.next_entity_id;
        if id == 0 || self.entities.iter().any(|entity| entity.id == id) {
            return Err(HiveNativeBirthBlock::Runtime(
                "native flyer allocation handle",
            ));
        }
        // Stage the intrinsic body without publishing it or advancing the
        // process owner. This validates every feature dependency before104B0.
        // B4 remains unresolved until the actual entry below: the ordinal is
        // distinct from456C20's encoded logical-world/body stamp.
        let mut entity = native_instance_body(
            id,
            RetailRuntimeValue::Unresolved,
            &metadata,
            terrain,
            NativeInstanceBodyRequest {
                entity_type: request.entity_type,
                position_raw: request.position_raw,
                objective: request.objective,
            },
        );
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(SURFACE_STATE_MASK, constructor_surface_bits);
        let radius = match entity.collision.active_model_slot() {
            RetailRuntimeValue::Known(slot) => entity
                .model_in_slot(slot)
                .and_then(|id| {
                    context
                        .resources
                        .model_extent_raw
                        .and_then(|lookup| lookup(id))
                })
                .map(|radius| radius as i16),
            RetailRuntimeValue::Unresolved => None,
        };
        let RetailRuntimeValue::Known(position) =
            crate::entity_initializer::constructor_position_raw(
                &metadata,
                request.position_raw,
                Some(terrain),
                radius,
            )
        else {
            return Err(HiveNativeBirthBlock::Runtime("native flyer D4A0 placement"));
        };
        entity.set_position_raw(position);
        let allocation = observe_main_base_abort_actor(&entity, self.allocation_generation).lease;
        authenticate_native_flyer_body(&entity, allocation, &metadata, None)
            .map_err(HiveNativeBirthBlock::Metadata)?;

        // No unowned feature remains. A source construction failure after this
        // entry would retain these process words; the native allocator adapter
        // does not fabricate a retail Mem_Alloc error branch.
        entity.construction_stamp_at_0xb4 = self.begin_common_body_attempt();
        let owner = publish_native_flyer(
            NativeFlyerConstructionRequest {
                entity: &mut entity,
                allocation,
                metadata: &metadata,
                authored: None,
            },
            world_fx,
        )
        .map_err(|reason| HiveNativeBirthBlock::Publication {
            reason,
            committed_constructor_prefix: true,
        })?;
        assert_eq!(self.allocate_live_entity_id(), id);
        self.append_live_entity(entity);
        // +60, FIFO/count, ejection and scheduler same-walk registration are
        // the1BEB0 caller's successive phases, not constructor side effects.
        Ok(owner)
    }
}

#[cfg(test)]
mod tests;
