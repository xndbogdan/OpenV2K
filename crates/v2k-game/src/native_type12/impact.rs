//! Type12-family hits through the shared ground host. A lethal hit's class1
//! (Type12) or class63 (Type127) terminal needs Playing's static world and
//! player, so only Playing's particle visit can deliver it.

pub use crate::native_ground_actor::impact::{
    NativeGroundImpactBlock as Type12ImpactBlock, NativeGroundImpactOutcome as Type12ImpactOutcome,
};
use crate::{shared_actor_impact::PlayingActorImpactFrame, world_fx::ParticleEntityImpact};

pub(crate) fn apply_playing_type12_family_particle_hit(
    frame: PlayingActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> Type12ImpactOutcome {
    use crate::intro2_radial::Intro2RadialTaskCustody;
    use crate::native_ground_actor::impact::apply_playing_native_ground_particle_hit as apply;
    let id = impact.target_entity_id;
    let row = frame
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .and_then(super::type12_row);
    // The Finished terminal receipt stands in for a retired owner; any other
    // visit needs the completed living owner before a prefix can commit.
    let Some(row) = row else {
        return Type12ImpactOutcome::Blocked {
            reason: Type12ImpactBlock::Runtime("native allocation"),
            committed_prefix: false,
        };
    };
    if !(crate::class49_death::finished_terminal_hit_authenticates(frame.entities, id)
        || frame
            .scheduler
            .prepare_native_actor_mutation(frame.entities, id))
    {
        return Type12ImpactOutcome::Blocked {
            reason: Type12ImpactBlock::Runtime("completed native allocation/task custody"),
            committed_prefix: false,
        };
    }
    let PlayingActorImpactFrame {
        resources,
        entities,
        world_fx,
        scheduler,
        notifications,
        static_damage,
        player_hull,
        extra_lives,
        retail_tick,
    } = frame;
    let lent = PlayingActorImpactFrame {
        resources: &mut *resources,
        entities: &mut *entities,
        world_fx: &mut *world_fx,
        scheduler: &mut *scheduler,
        notifications: &mut *notifications,
        static_damage: &mut *static_damage,
        player_hull: &mut *player_hull,
        extra_lives,
        retail_tick,
    };
    let result = match row {
        super::Type12Row::Type12 => apply::<super::profile::Type12Profile>(lent, impact),
        super::Type12Row::Type127 => apply::<super::profile::Type127Profile>(lent, impact),
    };
    if matches!(
        result,
        Type12ImpactOutcome::Blocked {
            committed_prefix: true,
            ..
        }
    ) {
        scheduler.park_native_contact_prefix(entities, id);
    }
    result
}
