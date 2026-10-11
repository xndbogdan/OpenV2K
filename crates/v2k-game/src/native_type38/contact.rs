//! Type38-family receipt binding for the complete shared late static-contact
//! pass. Playing lends its player to a lethal contact's class1/class63 radial.
pub use crate::native_ground_actor::contact::{
    NativeGroundContactApplied as Type38ContactApplied,
    NativeGroundContactBlock as Type38ContactBlock,
    NativeGroundContactOutcome as Type38ContactOutcome,
};

pub fn resolve_type38_family_static_contact(
    frame: &mut crate::intro2_contacts::Intro2ContactFrame<'_>,
    id: u32,
    playing: Option<crate::native_actor_capture::pair::PlayingPlayerContact<'_>>,
) -> Type38ContactOutcome {
    use crate::native_ground_actor::contact::resolve_native_ground_static_contact_with_playing as resolve;
    let row = frame
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .and_then(super::type38_row);
    match row {
        Some(super::Type38Row::Type38) => {
            resolve::<super::profile::Type38Profile>(frame, id, playing)
        }
        Some(super::Type38Row::Type129) => {
            resolve::<super::profile::Type129Profile>(frame, id, playing)
        }
        None => Type38ContactOutcome::Ineligible,
    }
}
