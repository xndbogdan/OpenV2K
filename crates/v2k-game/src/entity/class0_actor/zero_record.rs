//!451C00's Type68 miss: zero-record438080 ->104B0/D4A0/C490, then publication.

use super::*;

#[cfg(test)]
mod tests;

pub(crate) struct NativeType68ZeroRecordConstruction<'a> {
    pub resources: EntityConstructionResources<'a>,
    pub retail_tick: u32,
    /// Loaded controller+A4/descriptor84, independently of water visibility.
    pub waves_enabled: bool,
}

impl EntityManager {
    /// Construct the actual zero-record Type68 on a campaign miss. The caller
    /// owns the later saved-B4 overwrite and player attachment transaction.
    /// This returns exact task custody after registering the manager receipt;
    /// the caller must hand that owner to its scheduler before attachment.
    pub(crate) fn append_native_zero_record_type68(
        &mut self,
        request: NativeType68ZeroRecordConstruction<'_>,
        world_fx: &mut WorldFx,
    ) -> Result<Class0ActorOwner, Class0ActorError> {
        let metadata = self
            .type_metadata
            .get(68)
            .ok_or(Class0ActorError::Metadata)?
            .clone();
        authenticate_metadata(68, &metadata)?;
        if !matches!(
            self.next_common_body_ordinal(),
            RetailRuntimeValue::Known(_)
        ) {
            return Err(Class0ActorError::Runtime("native body stamp lineage"));
        }
        let terrain = request
            .resources
            .terrain
            .ok_or(Class0ActorError::Runtime("constructor terrain"))?;
        let constructor_surface_bits = crate::entity_initializer::constructor_surface_bits_at_tick(
            [0; 3],
            terrain,
            request.retail_tick,
            request.waves_enabled,
        )
        .ok_or(Class0ActorError::Runtime("constructor surface"))?;
        let id = self.next_entity_id;
        std::num::NonZeroU32::new(id).ok_or(Class0ActorError::Allocation)?;
        if self.native_class0_construction_present(id) {
            return Err(Class0ActorError::Allocation);
        }

        // Host metadata/resource admission precedes the actual104B0 attempt.
        // Once entered, a later construction failure does not refund B4.
        let stamp = self.begin_common_body_attempt();
        let mut entity = native_instance_body(
            id,
            stamp,
            &metadata,
            terrain,
            NativeInstanceBodyRequest::zeroed(68),
        );
        let allocation = observe_main_base_abort_actor(&entity, self.allocation_generation).lease;
        let runtime = construct_class0_actor(
            &mut entity,
            Class0ActorConstruction {
                allocation,
                metadata: &metadata,
                spawn: Class0SpawnInput::ZeroRecord { entity_type: 68 },
                resources: request.resources,
                constructor_surface_bits,
            },
            &mut || u32::from(world_fx.next_shared_retail_random_u16()),
        )?;
        assert_eq!(self.allocate_live_entity_id(), id);
        self.append_live_entity(entity);
        self.native_class0_actors.insert(id, runtime);
        Class0ActorOwner::adopt(self, id)
    }
}
