//! The Type30 family's native10EB0/11250 impact entry and own Class12 death
//! publisher.
pub use crate::native_ground_actor::impact::{
    NativeGroundImpactBlock as Type30ImpactBlock, NativeGroundImpactOutcome as Type30ImpactOutcome,
};
pub(crate) fn apply_type30_particle_hit(
    frame: crate::shared_actor_impact::SharedActorImpactFrame<'_>,
    impact: crate::world_fx::ParticleEntityImpact,
) -> Type30ImpactOutcome {
    use crate::native_ground_actor::impact::apply_native_ground_particle_hit as apply;
    let row = frame
        .entities
        .iter_all()
        .find(|entity| entity.id == impact.target_entity_id)
        .and_then(super::type30_row);
    match row {
        Some(super::Type30Row::Type33) => apply::<super::profile::Type33Profile>(frame, impact),
        _ => apply::<super::profile::Type30Profile>(frame, impact),
    }
}
