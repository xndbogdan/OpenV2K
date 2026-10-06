//! Type122 receipt binding for its own native hit/death transaction.
pub use crate::native_ground_actor::impact::{
    NativeGroundImpactBlock as Type122ImpactBlock,
    NativeGroundImpactOutcome as Type122ImpactOutcome,
};
pub(crate) fn apply_type122_particle_hit(
    frame: crate::shared_actor_impact::SharedActorImpactFrame<'_>,
    impact: crate::world_fx::ParticleEntityImpact,
) -> Type122ImpactOutcome {
    crate::native_ground_actor::impact::apply_native_ground_particle_hit::<
        super::profile::Type122Profile,
    >(frame, impact)
}
