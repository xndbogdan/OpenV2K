//! Type122 D920 includes D9B0 generic crushing before its null style+1C.

use crate::intro2_contacts::Intro2ContactFrame;
pub use crate::native_ground_actor::contact::{
    NativeGroundContactApplied as Type122ContactApplied,
    NativeGroundContactBlock as Type122ContactBlock,
    NativeGroundContactOutcome as Type122ContactOutcome,
};

pub fn resolve_type122_static_contact(
    frame: &mut Intro2ContactFrame<'_>,
    entity_id: u32,
) -> Type122ContactOutcome {
    crate::native_ground_actor::contact::resolve_native_ground_static_contact::<
        super::profile::Type122Profile,
    >(frame, entity_id)
}
