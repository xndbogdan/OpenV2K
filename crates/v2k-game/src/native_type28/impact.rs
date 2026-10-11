//! Type28 receipt binding for its own native hit/death transaction.
pub use crate::native_ground_actor::impact::{
    NativeGroundImpactBlock as Type28ImpactBlock, NativeGroundImpactOutcome as Type28ImpactOutcome,
};
pub(crate) fn apply_type28_particle_hit(
    frame: crate::shared_actor_impact::SharedActorImpactFrame<'_>,
    impact: crate::world_fx::ParticleEntityImpact,
) -> Type28ImpactOutcome {
    crate::native_ground_actor::impact::apply_native_ground_particle_hit::<
        super::profile::Type28Profile,
    >(frame, impact)
}
