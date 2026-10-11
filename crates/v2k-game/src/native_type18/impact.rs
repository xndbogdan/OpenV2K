//! Type18 receipt binding for its own native hit/death transaction.
pub use crate::native_ground_actor::impact::{
    NativeGroundImpactBlock as Type18ImpactBlock, NativeGroundImpactOutcome as Type18ImpactOutcome,
};
pub(crate) fn apply_type18_particle_hit(
    frame: crate::shared_actor_impact::SharedActorImpactFrame<'_>,
    impact: crate::world_fx::ParticleEntityImpact,
) -> Type18ImpactOutcome {
    crate::native_ground_actor::impact::apply_native_ground_particle_hit::<
        super::profile::Type18Profile,
    >(frame, impact)
}
