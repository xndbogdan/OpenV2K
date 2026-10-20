//! The family's native 10EB0/11250 impact entry, through each row's profile.
pub use crate::native_ground_actor::impact::{
    NativeGroundImpactBlock as Type31ImpactBlock, NativeGroundImpactOutcome as Type31ImpactOutcome,
};

pub(crate) fn apply_type31_family_particle_hit(
    frame: crate::shared_actor_impact::SharedActorImpactFrame<'_>,
    impact: crate::world_fx::ParticleEntityImpact,
) -> Type31ImpactOutcome {
    use crate::native_ground_actor::impact::apply_native_ground_particle_hit as apply;
    let row = frame
        .entities
        .iter_all()
        .find(|entity| entity.id == impact.target_entity_id)
        .and_then(super::type31_row);
    match row {
        Some(super::Type31Row::Type31) => apply::<super::profile::Type31Profile>(frame, impact),
        Some(super::Type31Row::Type105) => apply::<super::profile::Type105Profile>(frame, impact),
        None => Type31ImpactOutcome::Blocked {
            reason: Type31ImpactBlock::Runtime("native allocation"),
            committed_prefix: false,
        },
    }
}
