//! Type53 binding to the source-equivalent native ground static-contact phases.
#[cfg(test)]
use super::*;
use crate::intro2_contacts::Intro2ContactFrame;
pub use crate::native_ground_actor::contact::{
    NativeGroundContactApplied as Type53ContactApplied,
    NativeGroundContactBlock as Type53ContactBlock,
    NativeGroundContactOutcome as Type53ContactOutcome,
};

pub fn resolve_type53_static_contact(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
) -> Type53ContactOutcome {
    crate::native_ground_actor::contact::resolve_native_ground_static_contact::<
        super::profile::Type53Profile,
    >(frame, id)
}

#[cfg(test)]
#[path = "contact_tests.rs"]
mod tests;
