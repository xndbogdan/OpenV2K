//! Type53 graph admission over the shared ADE0/Chase phase.
use super::*;
pub(super) fn pursuing_graph_authenticates(entity: &Entity) -> bool {
    crate::native_ground_actor::search::pursuing_graph_authenticates::<super::profile::Type53Profile>(
        entity,
    )
}
