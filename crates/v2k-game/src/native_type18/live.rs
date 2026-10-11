//! Type18 allocation binding for the shared task walk. Class4's terrain
//! infection needs the mutable world; class9 needs the capture custody.

use crate::{
    entity::EntityManager, gameplay_notifications::GameplayNotifications,
    intro2_type17::capture::CaptureTaskCustody, native_ground_actor as shared,
    resource_cache::ResourceCache, world_fx::WorldFx,
};

pub use shared::{
    NativeGroundActorBlock as Type18Block, NativeGroundActorOutcome as Type18Outcome,
};
pub type Type18Owner = shared::NativeGroundActorOwner<super::profile::Type18Profile>;
pub type Type18Tick = shared::NativeGroundActorTick<super::profile::Type18Profile>;

pub struct Type18Frame<'a> {
    pub resources: &'a mut ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
    pub capture_tasks: &'a mut dyn CaptureTaskCustody,
    pub notifications: &'a mut GameplayNotifications,
}

pub fn tick_type18(
    manager: &mut EntityManager,
    owner: Type18Owner,
    frame: Type18Frame<'_>,
) -> Type18Tick {
    shared::tick_native_ground_actor(
        manager,
        owner,
        shared::NativeGroundActorFrame {
            resources: shared::NativeGroundResources::Mutable(frame.resources),
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
