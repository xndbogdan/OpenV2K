//! Common-body attempt and component prefix for converted/factory-born workers.

use super::*;
use crate::common_mover::sub_d::NativeSubDConstruction;

/// Successful worker body phases shared by factory and Main Base origins.
pub(super) struct NativeWorkerConstructionRequest<'a> {
    pub entity_type: u32,
    pub position_raw: [i16; 3],
    pub constructor_surface_bits: Option<u32>,
    pub selected_behavior: BehaviorSelection,
    pub prepared_go_to_job: Option<GoToJobSetupPlan>,
    pub body: Type8BodyConstruction,
    pub terrain: Option<&'a TerrainGrid>,
}

/// The 104B0/D4A0 body phase, independent of the selected task kernel.
pub(super) struct NativeWorkerBodyRequest<'a> {
    pub entity_type: u32,
    pub position_raw: [i16; 3],
    pub constructor_surface_bits: Option<u32>,
    pub selected_behavior: Option<BehaviorSelection>,
    pub body: Type8BodyConstruction,
    pub terrain: Option<&'a TerrainGrid>,
}

pub(crate) enum Type8BodyConstruction {
    Native {
        entity_type: u32,
        construction_stamp: RetailRuntimeValue<u16>,
        sub_a: RetailRuntimeValue<Option<SubAPropulsionRuntime>>,
        sub_d: Option<NativeSubDConstruction>,
    },
    #[cfg(test)]
    CapturedMainBaseFixture,
}

impl EntityManager {
    /// Called only after the caller has closed constructor prerequisites.
    /// 104B0 consumes the body stamp before09A80 allocates D then A;
    /// 20450's random draw precedes AC60. Publication never charges again.
    pub(crate) fn begin_type8_body_construction(
        &mut self,
        entity_type: u32,
        world_fx: &mut WorldFx,
    ) -> Type8BodyConstruction {
        let construction_stamp = self.begin_common_body_attempt();
        let metadata = self
            .type_metadata
            .get(entity_type as usize)
            .expect("worker metadata was preflighted");
        let sub_d = match metadata.sub_d_steering_descriptor {
            RetailRuntimeValue::Known(Some(descriptor)) => {
                Some(world_fx.construct_entity_sub_d(descriptor))
            }
            _ => None,
        };
        let sub_a = match metadata.sub_a_propulsion_descriptor {
            RetailRuntimeValue::Known(Some(descriptor)) => {
                RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::from_20450_constructor(
                    descriptor,
                    world_fx.next_shared_retail_random_u16(),
                )))
            }
            RetailRuntimeValue::Known(None) => RetailRuntimeValue::Known(None),
            RetailRuntimeValue::Unresolved => RetailRuntimeValue::Unresolved,
        };
        Type8BodyConstruction::Native {
            entity_type,
            construction_stamp,
            sub_a,
            sub_d,
        }
    }
}

impl Type8BodyConstruction {
    pub(super) fn is_for_entity_type(&self, entity_type: u32) -> bool {
        match self {
            Self::Native {
                entity_type: actual,
                ..
            } => *actual == entity_type,
            #[cfg(test)]
            Self::CapturedMainBaseFixture => entity_type == SCIENTIST_ENTITY_TYPE,
        }
    }

    pub(super) fn construction_stamp(&self) -> RetailRuntimeValue<u16> {
        match self {
            Self::Native {
                construction_stamp, ..
            } => *construction_stamp,
            #[cfg(test)]
            Self::CapturedMainBaseFixture => RetailRuntimeValue::Unresolved,
        }
    }
    pub(super) fn native_sub_d(&self) -> Option<NativeSubDConstruction> {
        match self {
            Self::Native { sub_d, .. } => *sub_d,
            #[cfg(test)]
            Self::CapturedMainBaseFixture => None,
        }
    }
    pub(super) fn apply_components(self, entity: &mut Entity) {
        match self {
            Self::Native { sub_a, sub_d, .. } => {
                entity.sub_a_propulsion_runtime = sub_a;
                if let Some(sub_d) = sub_d {
                    entity.type8_sub_d_frame_owner = Some(sub_d.frame_owner);
                    entity.type8_sub_d_runtime = Some(sub_d.runtime);
                }
            }
            #[cfg(test)]
            Self::CapturedMainBaseFixture => {
                entity.type8_sub_d_frame_owner =
                    crate::common_mover::sub_d::type8_first_query_owner_for_seed(
                        crate::common_mover::sub_d::TYPE8_MAIN_BASE_CONVERSION_SUB_D_SEED,
                    );
                entity.type8_sub_d_runtime =
                    Some(crate::common_mover::sub_d::Type9SubDRuntime::from_constructor());
            }
        }
    }
}
