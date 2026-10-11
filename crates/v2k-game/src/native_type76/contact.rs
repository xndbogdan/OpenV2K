//! Type76-family D920: Type77's 0x439 runs D9B0's generic crush, Type76's
//! 0x39 does not; both style+1C hooks are null except Furniture's C890.

use crate::intro2_contacts::Intro2ContactFrame;
pub use crate::native_ground_actor::contact::{
    NativeGroundContactApplied as Type76ContactApplied,
    NativeGroundContactBlock as Type76ContactBlock,
    NativeGroundContactOutcome as Type76ContactOutcome,
};

pub fn resolve_type76_family_static_contact(
    frame: &mut Intro2ContactFrame<'_>,
    entity_id: u32,
) -> Type76ContactOutcome {
    use crate::native_ground_actor::contact::resolve_native_ground_static_contact as resolve;
    let row = frame
        .entities
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .and_then(super::type76_row);
    match row {
        Some(super::Type76Row::Type76) => {
            resolve::<super::profile::Type76Profile>(frame, entity_id)
        }
        Some(super::Type76Row::Type77) => {
            resolve::<super::profile::Type77Profile>(frame, entity_id)
        }
        None => Type76ContactOutcome::Ineligible,
    }
}
