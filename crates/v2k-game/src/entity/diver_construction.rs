//! Type7 dynamic 104B0 -> 425680 -> class45/10/54/6 construction.
//! Body allocation is shared with other workers; the four-choice task owner
//! and the caller's Base/factory relation remain explicit phases.

use super::*;
use crate::native_type86::birth::{
    apply_prepared_root, prepare_unpublished_root, retain_native_type86_runtime,
    FourChoiceRootPlan, RootApplication,
};

impl EntityManager {
    pub(crate) fn prepare_diver_root(
        &self,
        position_raw: [i16; 3],
        terrain: Option<&TerrainGrid>,
    ) -> Result<FourChoiceRootPlan, MainBaseScientistSpawnError> {
        let position = self.type8_scientist_birth_position_raw(7, position_raw, terrain)?;
        let metadata = self
            .type_metadata
            .get(7)
            .ok_or(MainBaseScientistSpawnError::ScientistMetadataUnavailable)?;
        let preceding = self
            .retail_live_order_ids()
            .filter_map(|id| self.entities.iter().find(|entity| entity.id == id))
            .collect::<Vec<_>>();
        prepare_unpublished_root(self.next_entity_id, 7, position, metadata, &preceding)
            .map_err(|_| MainBaseScientistSpawnError::NativeWorkerPublication)
    }

    fn build_diver(
        &mut self,
        position_raw: [i16; 3],
        terrain: Option<&TerrainGrid>,
        constructor_surface_bits: Option<u32>,
        plan: FourChoiceRootPlan,
        world_fx: &mut WorldFx,
    ) -> Result<(Entity, RootApplication), MainBaseScientistSpawnError> {
        let body = self.begin_type8_body_construction(7, world_fx);
        let sub_d = body
            .native_sub_d()
            .ok_or(MainBaseScientistSpawnError::NativeWorkerPublication)?;
        let (mut entity, position) = self.build_native_worker_body(NativeWorkerBodyRequest {
            entity_type: 7,
            position_raw,
            terrain,
            constructor_surface_bits,
            selected_behavior: None,
            body,
        })?;
        // No task was selected during body assembly. The one source selector
        // now precedes the actual branch's complete constructor word sequence.
        entity.current_behavior_context = RetailRuntimeValue::Unresolved;
        entity.native_type86_anchor_raw_at_0x90 = RetailRuntimeValue::Known(position);
        let metadata = &self.type_metadata[7];
        let application = apply_prepared_root(&mut entity, metadata, plan, world_fx)
            .map_err(|_| MainBaseScientistSpawnError::NativeWorkerPublication)?;
        entity.initial_behavior = RetailRuntimeValue::Known(Some(application.selection));
        let allocation = observe_main_base_abort_actor(&entity, self.allocation_generation).lease;
        retain_native_type86_runtime(&mut entity, allocation, metadata, sub_d)
            .map_err(|_| MainBaseScientistSpawnError::NativeWorkerPublication)?;
        Ok((entity, application))
    }

    fn append_diver(&mut self, entity: Entity, application: &RootApplication) -> u32 {
        let id = entity.id;
        assert_eq!(self.allocate_live_entity_id(), id);
        self.append_live_entity(entity);
        if let Some(receipt) = application.resource_text {
            self.pending_fresh_level1_type9_resource_text_receipts
                .push(receipt);
        }
        id
    }

    pub(super) fn append_factory_diver(
        &mut self,
        source_factory: FactoryProductionEntityVersion,
        request: FactoryEntitySpawnRequest,
        retail_tick: u32,
        terrain: &TerrainGrid,
        world_fx: &mut WorldFx,
    ) -> Option<FactorySpawnedEntity> {
        let surface = crate::entity_initializer::constructor_surface_bits_at_tick(
            request.position_raw,
            terrain,
            retail_tick,
            self.common_environment_physics().waves_enabled,
        )?;
        let plan = self
            .prepare_diver_root(request.position_raw, Some(terrain))
            .ok()?;
        let (entity, application) = self
            .build_diver(
                request.position_raw,
                Some(terrain),
                Some(surface),
                plan,
                world_fx,
            )
            .expect("preflighted native diver construction remains valid");
        let id = self.append_diver(entity, &application);
        self.factory_converted_output_births
            .push(FactoryConvertedOutputBirthProvenance {
                entity_id: id,
                source_factory,
                spawn_request: request,
                selector_rng_word: application.selector_rng_word,
                constructor_rng_words: application.constructor_rng_words,
                selected_behavior_choice_index: application.selection.choice_index,
                selected_behavior_class_id: application.selection.program.class_id,
            });
        Some(FactorySpawnedEntity {
            entity_id: std::num::NonZeroU32::new(id)?,
            allocation_identity: main_base_abort_actor_allocation_identity(
                self.allocation_generation,
                id,
            ),
        })
    }

    pub(crate) fn append_main_base_diver(
        &mut self,
        request: crate::main_base_conversion::MainBaseReplacementSpawn,
        terrain: Option<&TerrainGrid>,
        constructor_surface_bits: Option<u32>,
        plan: FourChoiceRootPlan,
        world_fx: &mut WorldFx,
    ) -> Result<MainBaseScientistSpawned, MainBaseScientistSpawnError> {
        self.preflight_main_base_scientist_append(
            request.source_entity_id,
            request.main_base_entity_id,
        )?;
        let (entity, application) = self.build_diver(
            request.position_raw,
            terrain,
            constructor_surface_bits,
            plan,
            world_fx,
        )?;
        let position = entity.position;
        let id = self.append_diver(entity, &application);
        self.entity_mut(id)
            .expect("newly appended diver")
            .collision
            .recent_relation_id_at_0x60 =
            RetailRuntimeValue::Known(Some(request.main_base_entity_id));
        Ok(MainBaseScientistSpawned {
            source_id: request.source_entity_id,
            main_base_id: request.main_base_entity_id,
            replacement_id: id,
            replacement_position: position,
        })
    }
}
