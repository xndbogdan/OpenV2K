//! Type40 receipt binding for the complete shared late static-contact pass.
pub use crate::native_ground_actor::contact::{
    NativeGroundContactApplied as Type40ContactApplied,
    NativeGroundContactBlock as Type40ContactBlock,
    NativeGroundContactOutcome as Type40ContactOutcome,
};
pub fn resolve_type40_static_contact(
    frame: &mut crate::intro2_contacts::Intro2ContactFrame<'_>,
    id: u32,
) -> Type40ContactOutcome {
    crate::native_ground_actor::contact::resolve_native_ground_static_contact::<
        super::profile::Type40Profile,
    >(frame, id)
}
