//! Type31-family receipt binding for the complete shared late static-contact
//! pass.
pub use crate::native_ground_actor::contact::{
    NativeGroundContactApplied as Type31ContactApplied,
    NativeGroundContactBlock as Type31ContactBlock,
    NativeGroundContactOutcome as Type31ContactOutcome,
};

pub fn resolve_type31_family_static_contact(
    frame: &mut crate::intro2_contacts::Intro2ContactFrame<'_>,
    id: u32,
) -> Type31ContactOutcome {
    use crate::native_ground_actor::contact::resolve_native_ground_static_contact as resolve;
    let row = frame
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .and_then(super::type31_row);
    match row {
        Some(super::Type31Row::Type31) => resolve::<super::profile::Type31Profile>(frame, id),
        Some(super::Type31Row::Type105) => resolve::<super::profile::Type105Profile>(frame, id),
        None => Type31ContactOutcome::Ineligible,
    }
}
