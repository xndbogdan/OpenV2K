//! Type28 allocation binding for the shared task walk. Class9 needs the
//! capture custody; no class writes the terrain.

use crate::{
    entity::EntityManager, gameplay_notifications::GameplayNotifications,
    intro2_type17::capture::CaptureTaskCustody, native_ground_actor as shared,
    resource_cache::ResourceCache, world_fx::WorldFx,
};

pub use shared::{
    NativeGroundActorBlock as Type28Block, NativeGroundActorOutcome as Type28Outcome,
};
pub type Type28Owner = shared::NativeGroundActorOwner<super::profile::Type28Profile>;
pub type Type28Tick = shared::NativeGroundActorTick<super::profile::Type28Profile>;

pub struct Type28Frame<'a> {
    pub resources: &'a ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
    pub capture_tasks: &'a mut dyn CaptureTaskCustody,
    pub notifications: &'a mut GameplayNotifications,
}

pub fn tick_type28(
    manager: &mut EntityManager,
    owner: Type28Owner,
    frame: Type28Frame<'_>,
) -> Type28Tick {
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
