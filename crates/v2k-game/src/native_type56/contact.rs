//! Type56 receipt binding for the complete shared late static-contact pass.
pub use crate::native_ground_actor::contact::{
    NativeGroundContactApplied as Type56ContactApplied,
    NativeGroundContactBlock as Type56ContactBlock,
    NativeGroundContactOutcome as Type56ContactOutcome,
};
pub fn resolve_type56_static_contact(
    frame: &mut crate::intro2_contacts::Intro2ContactFrame<'_>,
    id: u32,
) -> Type56ContactOutcome {
    crate::native_ground_actor::contact::resolve_native_ground_static_contact::<
        super::profile::Type56Profile,
    >(frame, id)
}
