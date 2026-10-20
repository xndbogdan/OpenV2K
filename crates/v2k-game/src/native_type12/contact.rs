//! Type12-family receipt binding for the complete shared late static-contact
//! pass. Class15's `4C76F0` calls `C890` on the touched object; Playing lends
//! its player to a lethal contact's class1/class63 radial.
pub use crate::native_ground_actor::contact::{
    NativeGroundContactApplied as Type12ContactApplied,
    NativeGroundContactBlock as Type12ContactBlock,
    NativeGroundContactOutcome as Type12ContactOutcome,
};

pub fn resolve_type12_family_static_contact(
    frame: &mut crate::intro2_contacts::Intro2ContactFrame<'_>,
    id: u32,
    playing: Option<crate::native_actor_capture::pair::PlayingPlayerContact<'_>>,
) -> Type12ContactOutcome {
    use crate::native_ground_actor::contact::resolve_native_ground_static_contact_with_playing as resolve;
    let row = frame
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .and_then(super::type12_row);
    match row {
        Some(super::Type12Row::Type12) => {
            resolve::<super::profile::Type12Profile>(frame, id, playing)
        }
        Some(super::Type12Row::Type127) => {
            resolve::<super::profile::Type127Profile>(frame, id, playing)
        }
        None => Type12ContactOutcome::Ineligible,
    }
}
