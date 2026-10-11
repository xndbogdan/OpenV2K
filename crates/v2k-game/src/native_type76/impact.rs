//! Type76-family receipt binding for the shared native hit/death transaction.
pub use crate::native_ground_actor::impact::{
    NativeGroundImpactBlock as Type76ImpactBlock, NativeGroundImpactOutcome as Type76ImpactOutcome,
};

pub(crate) fn apply_type76_family_particle_hit(
    frame: crate::shared_actor_impact::SharedActorImpactFrame<'_>,
    impact: crate::world_fx::ParticleEntityImpact,
) -> Type76ImpactOutcome {
    use crate::native_ground_actor::impact::apply_native_ground_particle_hit as apply;
    let row = frame
        .entities
        .iter_all()
        .find(|entity| entity.id == impact.target_entity_id)
        .and_then(super::type76_row);
    match row {
        Some(super::Type76Row::Type76) => apply::<super::profile::Type76Profile>(frame, impact),
        Some(super::Type76Row::Type77) => apply::<super::profile::Type77Profile>(frame, impact),
        None => Type76ImpactOutcome::Blocked {
            reason: Type76ImpactBlock::Runtime("native allocation"),
            committed_prefix: false,
        },
    }
}
