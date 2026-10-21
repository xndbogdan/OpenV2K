//! Type30-family receipt binding for the complete shared late static-contact pass.
pub use crate::native_ground_actor::contact::{
    NativeGroundContactApplied as Type30ContactApplied,
    NativeGroundContactBlock as Type30ContactBlock,
    NativeGroundContactOutcome as Type30ContactOutcome,
};
pub fn resolve_type30_static_contact(
    frame: &mut crate::intro2_contacts::Intro2ContactFrame<'_>,
    id: u32,
) -> Type30ContactOutcome {
    use crate::native_ground_actor::contact::resolve_native_ground_static_contact as resolve;
    let row = frame
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .and_then(super::type30_row);
    match row {
        Some(super::Type30Row::Type33) => resolve::<super::profile::Type33Profile>(frame, id),
        _ => resolve::<super::profile::Type30Profile>(frame, id),
    }
}
