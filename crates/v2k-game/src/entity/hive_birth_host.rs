//! Manager adapter for native Hive children and `1C830/1CA90` raw commits.

use super::{
    EntityConstructionResources, EntityManager, HiveNativeBirthBlock,
    NativeHiveChildConstructionContext,
};
use crate::{
    entity_collision_state::RetailRuntimeValue,
    hive_birth::{
        HiveBirthAttempt, HiveBirthHost, HiveBirthRequest, HiveChildEjection, HiveEjectionGeometry,
        HiveEjectionRequest,
    },
    intro2_flyers_live::Intro2FlyerSchedulerOwner,
    world_fx::WorldFx,
};
use v2k_formats::terrain::GRID_SIZE;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HiveBirthManagerBlock {
    Native(HiveNativeBirthBlock),
    ChildState { child_handle: u32 },
    SourceAllocation { source_id: u32 },
    ChildAllocation { child_handle: u32 },
    SourceSubN { source_id: u32 },
    SourceAnchor { source_id: u32 },
    SourceModel { source_id: u32 },
    ModelExtent { model_index: usize },
    Terrain,
    ConstructorWavePolicy,
}

/// Only native construction requires the authored wave policy. An unresolved
/// value blocks that feature while retained ejection geometry/timers continue.
#[derive(Clone, Copy)]
pub(crate) struct HiveBirthManagerContext<'a> {
    pub resources: EntityConstructionResources<'a>,
    pub retail_tick: u32,
    pub waves_enabled: RetailRuntimeValue<bool>,
}

pub(crate) struct HiveBirthManagerHost<'manager, 'resources> {
    manager: &'manager mut EntityManager,
    context: HiveBirthManagerContext<'resources>,
    world_fx: &'manager mut WorldFx,
    newborn_owners: Vec<Intro2FlyerSchedulerOwner>,
}

impl<'manager, 'resources> HiveBirthManagerHost<'manager, 'resources> {
    pub(crate) fn new(
        manager: &'manager mut EntityManager,
        context: HiveBirthManagerContext<'resources>,
        world_fx: &'manager mut WorldFx,
    ) -> Self {
        Self {
            manager,
            context,
            world_fx,
            newborn_owners: Vec::new(),
        }
    }

    pub(crate) fn take_newborn_owners(&mut self) -> Vec<Intro2FlyerSchedulerOwner> {
        std::mem::take(&mut self.newborn_owners)
    }
}

impl HiveBirthHost for HiveBirthManagerHost<'_, '_> {
    type Block = HiveBirthManagerBlock;

    fn child_state_flags(&mut self, child_handle: u32) -> Result<Option<u32>, Self::Block> {
        let Some(entity) = self
            .manager
            .iter_all()
            .find(|entity| entity.id == child_handle)
        else {
            return Ok(None);
        };
        // 16460 consults only dying. Other unavailable bits cannot turn a
        // present state-zero allocation into a lookup miss or block this lane.
        match entity.collision.state_flags_at_0x08.masked(0x4000) {
            RetailRuntimeValue::Known(flags) => Ok(Some(flags)),
            RetailRuntimeValue::Unresolved => {
                Err(HiveBirthManagerBlock::ChildState { child_handle })
            }
        }
    }

    fn construct_hive_child(
        &mut self,
        request: HiveBirthRequest,
    ) -> Result<HiveBirthAttempt<Self::Block>, Self::Block> {
        let RetailRuntimeValue::Known(waves_enabled) = self.context.waves_enabled else {
            return Err(HiveBirthManagerBlock::ConstructorWavePolicy);
        };
        let context = NativeHiveChildConstructionContext {
            resources: self.context.resources,
            retail_tick: self.context.retail_tick,
            waves_enabled,
        };
        match self
            .manager
            .append_native_hive_child(request, context, self.world_fx)
        {
            Ok(owner) => {
                let child = owner.entity_id();
                self.newborn_owners.push(owner);
                Ok(HiveBirthAttempt::Created(child))
            }
            Err(
                block @ HiveNativeBirthBlock::Publication {
                    committed_constructor_prefix: true,
                    ..
                },
            ) => Ok(HiveBirthAttempt::CommittedPrefixBlock(
                HiveBirthManagerBlock::Native(block),
            )),
            Err(block) => Err(HiveBirthManagerBlock::Native(block)),
        }
    }

    fn publish_hive_child_source(&mut self, child_handle: u32, source_id: u32) {
        let child = self
            .manager
            .entity_mut(child_handle)
            .expect("Created native Hive child retains its allocation");
        child.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(Some(source_id));
    }

    fn prepare_hive_ejection(
        &mut self,
        request: HiveEjectionRequest,
    ) -> Result<HiveEjectionGeometry, Self::Block> {
        let source = self
            .manager
            .iter_all()
            .find(|entity| entity.id == request.source_id && entity.entity_type == 67)
            .ok_or(HiveBirthManagerBlock::SourceAllocation {
                source_id: request.source_id,
            })?;
        if !self
            .manager
            .iter_all()
            .any(|entity| entity.id == request.child_handle)
        {
            return Err(HiveBirthManagerBlock::ChildAllocation {
                child_handle: request.child_handle,
            });
        }
        let RetailRuntimeValue::Known(Some(sub_n)) = source.sub_n_runtime else {
            return Err(HiveBirthManagerBlock::SourceSubN {
                source_id: request.source_id,
            });
        };
        let RetailRuntimeValue::Known(anchor_raw) = sub_n.anchor_raw() else {
            return Err(HiveBirthManagerBlock::SourceAnchor {
                source_id: request.source_id,
            });
        };
        let RetailRuntimeValue::Known(slot) = source.collision.active_model_slot() else {
            return Err(HiveBirthManagerBlock::SourceModel {
                source_id: request.source_id,
            });
        };
        let model = source
            .model_index
            .filter(|model| source.model_in_slot(slot) == Some(*model))
            .ok_or(HiveBirthManagerBlock::SourceModel {
                source_id: request.source_id,
            })?;
        let extent = self
            .context
            .resources
            .model_extent_raw
            .and_then(|lookup| lookup(model))
            .ok_or(HiveBirthManagerBlock::ModelExtent { model_index: model })?;
        if !self
            .context
            .resources
            .terrain
            .is_some_and(|terrain| terrain.cells.len() == GRID_SIZE * GRID_SIZE)
        {
            return Err(HiveBirthManagerBlock::Terrain);
        }
        Ok(HiveEjectionGeometry {
            anchor_raw,
            model_extent_raw: extent,
        })
    }

    fn sample_hive_ejection_terrain_height_raw(&self, x_raw: i16, z_raw: i16) -> i16 {
        self.context
            .resources
            .terrain
            .expect("ejection terrain preflighted")
            .bilinear_height_raw(x_raw, z_raw)
    }

    fn publish_hive_child_ejection(&mut self, child_handle: u32, ejection: HiveChildEjection) {
        let child = self
            .manager
            .entity_mut(child_handle)
            .expect("prepared ejection retains child allocation");
        child.collision.state_flags_at_0x08.overwrite(0x8000, 0);
        child.set_motion_raw(ejection.position_raw, ejection.velocity_raw);
        child.set_rotation_heading_pitch_roll_raw(ejection.rotation_raw.map(|word| word as i16));
        // 1C830 changes the three angle words but has no13F70/D720 call.
        // Keep the constructor's physical matrix until an actual owner writes it.
    }

    fn restore_hive_child_pairs(&mut self, child_handle: u32) {
        self.manager
            .entity_mut(child_handle)
            .expect("1CA90 just resolved the child allocation")
            .collision
            .state_flags_at_0x08
            .overwrite(0x8000, 0x8000);
    }

    fn next_shared_random_u16(&mut self) -> u16 {
        self.world_fx.next_shared_retail_random_u16()
    }
}

#[cfg(test)]
mod tests;
