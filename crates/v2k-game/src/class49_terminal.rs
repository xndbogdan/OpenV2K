//! Shared synchronous BAF0 -> radial -> A860, then the source's explicit
//! Class1 removal or Class49 ring-and-removal suffix.
use crate::{
    class49_death::*,
    entity::EntityManager,
    gameplay_notifications::GameplayNotifications,
    intro2_radial::{
        apply_intro2_radial_damage, apply_static_radial_damage, Intro2RadialFrame,
        Intro2RadialReport, Intro2RadialTaskCustody, Intro2RadialTerminalCall,
    },
    live_actor_checked_damage::LiveActorDeathResult,
    player_hull::PlayerHull,
    resource_cache::ResourceCache,
    specialized_actor_task_production::{
        PlayingRadialFrame, PlayingRadialOutcome, SpecializedActorTaskScheduler,
    },
    static_damage::{StaticDamageOutcome, StaticDamageScheduler},
    static_damage_live::CurrentStaticDamageLookupError,
    world_fx::WorldFx,
};

pub enum Class49WorldContext<'a> {
    Cinematic {
        actor_tasks: &'a mut dyn Intro2RadialTaskCustody,
        active_terminal_calls: Vec<Intro2RadialTerminalCall>,
    },
    Playing {
        scheduler: &'a mut SpecializedActorTaskScheduler,
        player_hull: &'a mut PlayerHull,
        extra_lives: crate::entity_collision_state::RetailRuntimeValue<u8>,
        active_terminal_calls: Vec<Intro2RadialTerminalCall>,
    },
}
pub struct Class49TerminalFrame<'a> {
    pub entities: &'a mut EntityManager,
    pub resources: &'a mut ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub static_damage: &'a mut StaticDamageScheduler,
    pub notifications: &'a mut GameplayNotifications,
    pub retail_tick: u32,
    pub world: Class49WorldContext<'a>,
}
impl<'a> Class49TerminalFrame<'a> {
    pub(crate) fn from_cinematic(frame: Intro2RadialFrame<'a>) -> Self {
        Self {
            entities: frame.entities,
            resources: frame.resources,
            world_fx: frame.world_fx,
            static_damage: frame.static_damage,
            notifications: frame.notifications,
            retail_tick: frame.retail_tick,
            world: Class49WorldContext::Cinematic {
                actor_tasks: frame.actor_tasks,
                active_terminal_calls: frame.active_terminal_calls,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Class49RadialReport {
    Cinematic(Intro2RadialReport),
    Playing {
        static_deliveries: Vec<StaticDamageOutcome>,
        dynamic: PlayingRadialOutcome,
    },
}
impl Class49RadialReport {
    fn completed(&self) -> bool {
        match self {
            Self::Cinematic(Intro2RadialReport::Applied { dynamic, .. }) => dynamic.completed(),
            Self::Cinematic(_) => false,
            Self::Playing { dynamic, .. } => dynamic.blocked.is_none(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Class49TerminalBlock {
    Death(Class49DeathBlock),
    Claim,
    PlayingPlayerContextUnavailable,
    StaticLookup(CurrentStaticDamageLookupError),
    Radial(Box<Class49RadialReport>),
}

pub(crate) fn run_class49_standard_death(
    mut frame: Class49TerminalFrame<'_>,
    id: u32,
) -> Result<LiveActorDeathResult<()>, Class49TerminalBlock> {
    let result = run(&mut frame, id);
    if result.is_err() {
        match &mut frame.world {
            Class49WorldContext::Cinematic { actor_tasks, .. } => {
                actor_tasks.park_class49_terminal(frame.entities, id)
            }
            Class49WorldContext::Playing { scheduler, .. } => {
                scheduler.park_class49_terminal(frame.entities, id)
            }
        }
    }
    result
}
fn run(
    frame: &mut Class49TerminalFrame<'_>,
    id: u32,
) -> Result<LiveActorDeathResult<()>, Class49TerminalBlock> {
    use Class49TerminalBlock as Block;
    let Some(receipt) = begin_class49_standard_death(
        frame.entities,
        id,
        frame.resources,
        frame.world_fx,
        frame.retail_tick,
    )
    .map_err(Block::Death)?
    else {
        return Ok(LiveActorDeathResult {
            returned_nonzero: false,
            publication: None,
        });
    };
    if !claim_class49_terminal(frame.entities, &receipt) {
        return Err(Block::Claim);
    }
    let radial = match &mut frame.world {
        Class49WorldContext::Cinematic {
            actor_tasks,
            active_terminal_calls,
        } => {
            let mut calls = active_terminal_calls.clone();
            calls.push(Intro2RadialTerminalCall::Class49(receipt));
            Class49RadialReport::Cinematic(apply_intro2_radial_damage(
                &mut Intro2RadialFrame {
                    active_terminal_calls: calls,
                    entities: frame.entities,
                    resources: frame.resources,
                    world_fx: frame.world_fx,
                    static_damage: frame.static_damage,
                    notifications: frame.notifications,
                    retail_tick: frame.retail_tick,
                    actor_tasks: &mut **actor_tasks,
                },
                receipt.position_raw,
                receipt.radial_damage,
            ))
        }
        Class49WorldContext::Playing {
            scheduler,
            player_hull,
            extra_lives,
            active_terminal_calls,
        } => {
            let static_deliveries = apply_static_radial_damage(
                frame.resources,
                frame.static_damage,
                frame.world_fx,
                receipt.position_raw,
                receipt.radial_damage,
            )
            .map_err(Block::StaticLookup)?;
            let mut calls = active_terminal_calls.clone();
            calls.push(Intro2RadialTerminalCall::Class49(receipt));
            let dynamic = scheduler.apply_playing_radial_damage(PlayingRadialFrame {
                entities: frame.entities,
                player_hull,
                extra_lives: *extra_lives,
                origin_raw: receipt.position_raw,
                template: receipt.radial_damage,
                world_fx: frame.world_fx,
                notifications: frame.notifications,
                retail_tick: frame.retail_tick,
                resources: frame.resources,
                static_damage: frame.static_damage,
                active_terminal_calls: calls,
            });
            Class49RadialReport::Playing {
                static_deliveries,
                dynamic,
            }
        }
    };
    if !radial.completed() {
        return Err(Block::Radial(Box::new(radial)));
    }
    let completed =
        finish_class49_terminal(frame.entities, receipt, frame.resources, frame.world_fx)
            .map_err(Block::Death)?;
    match &mut frame.world {
        Class49WorldContext::Cinematic { actor_tasks, .. } => {
            actor_tasks.finish_class49_terminal(receipt, completed.ring_owner)
        }
        Class49WorldContext::Playing { scheduler, .. } => {
            scheduler.finish_class49_terminal(receipt, completed.ring_owner)
        }
    }
    Ok(LiveActorDeathResult {
        returned_nonzero: true,
        publication: None,
    })
}

#[cfg(test)]
mod tests;
