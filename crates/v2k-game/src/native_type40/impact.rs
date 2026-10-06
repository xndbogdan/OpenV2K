//! Type40's native10EB0/11250 impact entry and own class18 death publisher.
pub use crate::native_ground_actor::impact::{
    NativeGroundImpactBlock as Type40ImpactBlock, NativeGroundImpactOutcome as Type40ImpactOutcome,
};
pub(crate) fn apply_type40_particle_hit(
    frame: crate::shared_actor_impact::SharedActorImpactFrame<'_>,
    impact: crate::world_fx::ParticleEntityImpact,
) -> Type40ImpactOutcome {
    crate::native_ground_actor::impact::apply_native_ground_particle_hit::<
        super::profile::Type40Profile,
    >(frame, impact)
}
