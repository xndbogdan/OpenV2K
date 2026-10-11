//! Type53 weighted reselection binding for its actual native allocation.
use super::*;
use crate::{native_ground_actor as shared, world_fx::WorldFx};
pub(super) use shared::behavior::ReselectionEntry;
pub(super) fn reselect(
    manager: &mut EntityManager,
    id: u32,
    tick: u32,
    world_fx: &mut WorldFx,
    resources: Option<&crate::resource_cache::ResourceCache>,
    entry: ReselectionEntry,
) -> Result<(), Intro2Type53Block> {
    shared::behavior::reselect::<super::profile::Type53Profile>(
        manager, id, tick, world_fx, resources, entry,
    )
}
