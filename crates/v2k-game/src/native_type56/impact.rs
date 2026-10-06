//! Type56's native10EB0/11250 impact entry and own class2 death publisher.
pub use crate::native_ground_actor::impact::{
    NativeGroundImpactBlock as Type56ImpactBlock, NativeGroundImpactOutcome as Type56ImpactOutcome,
};
pub(crate) fn apply_type56_particle_hit(
    frame: crate::shared_actor_impact::SharedActorImpactFrame<'_>,
    impact: crate::world_fx::ParticleEntityImpact,
) -> Type56ImpactOutcome {
    crate::native_ground_actor::impact::apply_native_ground_particle_hit::<
        super::profile::Type56Profile,
    >(frame, impact)
}
