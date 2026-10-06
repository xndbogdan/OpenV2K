//! Type30 receipt binding for the complete shared late static-contact pass.
pub use crate::native_ground_actor::contact::{
    NativeGroundContactApplied as Type30ContactApplied,
    NativeGroundContactBlock as Type30ContactBlock,
    NativeGroundContactOutcome as Type30ContactOutcome,
};
pub fn resolve_type30_static_contact(
    frame: &mut crate::intro2_contacts::Intro2ContactFrame<'_>,
    id: u32,
) -> Type30ContactOutcome {
    crate::native_ground_actor::contact::resolve_native_ground_static_contact::<
        super::profile::Type30Profile,
    >(frame, id)
}
