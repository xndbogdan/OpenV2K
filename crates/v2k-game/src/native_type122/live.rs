//! Type122 allocation and capture-context binding for the shared task walk.

use crate::{
    entity::EntityManager, gameplay_notifications::GameplayNotifications,
    intro2_type17::capture::CaptureTaskCustody, native_ground_actor as shared,
    resource_cache::ResourceCache, world_fx::WorldFx,
};

pub use shared::{
    NativeGroundActorBlock as Type122Block, NativeGroundActorOutcome as Type122Outcome,
};
pub type Type122Owner = shared::NativeGroundActorOwner<super::profile::Type122Profile>;
pub type Type122Tick = shared::NativeGroundActorTick<super::profile::Type122Profile>;

pub struct Type122Frame<'a> {
    pub resources: &'a ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
    pub capture_tasks: &'a mut dyn CaptureTaskCustody,
    pub notifications: &'a mut GameplayNotifications,
}

pub fn tick_type122(
    manager: &mut EntityManager,
    owner: Type122Owner,
    frame: Type122Frame<'_>,
) -> Type122Tick {
    shared::tick_native_ground_actor(
        manager,
        owner,
        shared::NativeGroundActorFrame {
            resources: shared::NativeGroundResources::ReadOnly(frame.resources),
            world_fx: frame.world_fx,
            elapsed_micros: frame.elapsed_micros,
            retail_tick: frame.retail_tick,
            capture: shared::NativeCaptureDispatch::Transport {
                tasks: frame.capture_tasks,
                notifications: frame.notifications,
            },
        },
    )
}
