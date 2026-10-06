//! Type17 callers of the shared, independently authenticated captor relation.

pub use crate::native_actor_capture::{
    apply_capture_pair_checked_damage, attach_capture_child, execute_capture_delivery,
    execute_capture_root, kill_capture_contact,
    publish_native_captor_standard_death as publish_type17_standard_death, CaptureBlock,
    CaptureCallbackCompletion, CaptureChildFrame, CaptureChildOperation, CaptureContext,
    CaptureHiveDyingBurst, CaptureRootCallback, CaptureTaskCustody,
};
pub(crate) use crate::native_actor_capture::{
    commit_native_corpse_relation, prepare_native_corpse_relation,
};

#[cfg(test)]
use crate::native_actor_capture::actor;
#[cfg(test)]
use crate::{
    entity::EntityManager,
    entity_collision_state::{RetailRuntimeValue, DYING_STATE_BIT},
    gameplay_notifications::GameplayNotifications,
    world_fx::WorldFx,
};
#[cfg(test)]
pub(crate) mod tests;
