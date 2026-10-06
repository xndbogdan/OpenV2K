//! Type53 allocation binding for the shared native ground-actor task walk.
use super::*;
use crate::{native_ground_actor as shared, resource_cache::ResourceCache, world_fx::WorldFx};
pub use shared::{
    NativeGroundActorBlock as Intro2Type53Block, NativeGroundActorOutcome as Intro2Type53Outcome,
};
pub type Intro2Type53Owner = shared::NativeGroundActorOwner<super::profile::Type53Profile>;
pub type Intro2Type53Tick = shared::NativeGroundActorTick<super::profile::Type53Profile>;
pub struct Intro2Type53Frame<'a> {
    pub resources: &'a ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
}
pub fn tick_intro2_type53(
    manager: &mut EntityManager,
    owner: Intro2Type53Owner,
    frame: Intro2Type53Frame<'_>,
) -> Intro2Type53Tick {
    shared::tick_native_ground_actor(
        manager,
        owner,
        shared::NativeGroundActorFrame {
            resources: shared::NativeGroundResources::ReadOnly(frame.resources),
            world_fx: frame.world_fx,
            elapsed_micros: frame.elapsed_micros,
            retail_tick: frame.retail_tick,
            capture: shared::NativeCaptureDispatch::PursuitOnly,
        },
    )
}
