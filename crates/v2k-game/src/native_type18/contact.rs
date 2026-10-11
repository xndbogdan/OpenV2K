//! Type18 D920 includes D9B0 generic crushing (0x400) before its null style+1C.

use crate::intro2_contacts::Intro2ContactFrame;
pub use crate::native_ground_actor::contact::{
    NativeGroundContactApplied as Type18ContactApplied,
    NativeGroundContactBlock as Type18ContactBlock,
    NativeGroundContactOutcome as Type18ContactOutcome,
};

pub fn resolve_type18_static_contact(
    frame: &mut Intro2ContactFrame<'_>,
    entity_id: u32,
) -> Type18ContactOutcome {
    crate::native_ground_actor::contact::resolve_native_ground_static_contact::<
        super::profile::Type18Profile,
    >(frame, entity_id)
}
