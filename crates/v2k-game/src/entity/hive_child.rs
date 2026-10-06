//! 1BEB0's populated zero0x4C creature request ->438080/104B0/D4A0.

use super::*;
use crate::{
    hive_birth::HiveBirthRequest,
    intro2_flyers_live::{
        authenticate_native_type15_body, authenticate_native_type15_metadata,
        publish_native_type15, Intro2FlyerPublicationError, Intro2FlyerSchedulerOwner,
        NativeType15ConstructionRequest,
    },
};

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
}

impl EntityManager {
    pub(crate) fn append_native_hive_child(
        &mut self,
        request: HiveBirthRequest,
        context: NativeHiveChildConstructionContext<'_>,
        world_fx: &mut WorldFx,
    ) -> Result<Intro2FlyerSchedulerOwner, HiveNativeBirthBlock> {
        if request.entity_type != 15 {
            return Err(HiveNativeBirthBlock::UnsupportedEntityType {
                entity_type: request.entity_type,
            });
        }
        let metadata = self
            .type_metadata
            .get(15)
            .ok_or(HiveNativeBirthBlock::Runtime("native Wasp metadata"))?
            .clone();
        authenticate_native_type15_metadata(&metadata).map_err(HiveNativeBirthBlock::Metadata)?;
        if !matches!(
            self.next_common_body_ordinal(),
            RetailRuntimeValue::Known(_)
        ) {
            return Err(HiveNativeBirthBlock::Runtime("native body stamp lineage"));
        }
        let terrain = context
            .resources
            .terrain
            .ok_or(HiveNativeBirthBlock::Runtime("native Wasp terrain"))?;
        if terrain.cells.len() != v2k_formats::terrain::GRID_SIZE * v2k_formats::terrain::GRID_SIZE
        {
            return Err(HiveNativeBirthBlock::Runtime(
                "native Wasp complete terrain",
            ));
        }
        let constructor_surface_bits = crate::entity_initializer::constructor_surface_bits_at_tick(
            request.position_raw,
            terrain,
            context.retail_tick,
            context.waves_enabled,
        )
        .ok_or(HiveNativeBirthBlock::Runtime(
            "native Wasp constructor surface",
        ))?;
        let id = self.next_entity_id;
        if id == 0 || self.entities.iter().any(|entity| entity.id == id) {
            return Err(HiveNativeBirthBlock::Runtime(
                "native Wasp allocation handle",
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
                entity_type: 15,
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
            return Err(HiveNativeBirthBlock::Runtime("native Wasp D4A0 placement"));
        };
        entity.set_position_raw(position);
        let allocation = observe_main_base_abort_actor(&entity, self.allocation_generation).lease;
        authenticate_native_type15_body(&entity, allocation, &metadata)
            .map_err(HiveNativeBirthBlock::Metadata)?;

        // No unowned feature remains. A source construction failure after this
        // entry would retain these process words; the native allocator adapter
        // does not fabricate a retail Mem_Alloc error branch.
        entity.construction_stamp_at_0xb4 = self.begin_common_body_attempt();
        let owner = publish_native_type15(
            NativeType15ConstructionRequest {
                entity: &mut entity,
                allocation,
                metadata: &metadata,
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
