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
impl<'a> Class49WorldContext<'a> {
    /// A late contact walk lends Playing's player only when it owns one;
    /// otherwise the terminal keeps the cinematic radial owner.
    pub(crate) fn for_contact(
        scheduler: &'a mut SpecializedActorTaskScheduler,
        player: Option<crate::native_actor_capture::pair::PlayingPlayerContact<'a>>,
    ) -> Self {
        match player {
            Some(player) => Self::Playing {
                scheduler,
                player_hull: player.hull,
                extra_lives: player.extra_lives,
                active_terminal_calls: Vec::new(),
            },
            None => Self::Cinematic {
                actor_tasks: scheduler,
                active_terminal_calls: Vec::new(),
            },
        }
    }
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
    pub(crate) fn completed(&self) -> bool {
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
    /// A carrying class63 carrier's D040 child release.
    Capture(crate::native_actor_capture::CaptureBlock),
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
    if frame
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .is_some_and(carrying_class63_source)
    {
        return run_carrying_class63(frame, id);
    }
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
    terminal(frame, receipt)?;
    Ok(LiveActorDeathResult {
        returned_nonzero: true,
        publication: None,
    })
}

/// 10C10 -> DB80 on a class63 carrier holding a carrying style. Style+2C is
/// D040: it pops the child (16750 releases it with the carrier already
/// dying) and, on the child's zero return, C690 -> AC60 -> 25660 installs
/// class63, whose BC90 bursts, blasts and drops a Type61. DB80 then rereads
/// +BC while the allocation still resolves, so 25660 installs class63 and BC90
/// runs again: two bursts, two radials, two Type61s and one 10B70. An empty
/// row returns BE5B0 from 18500, D040 skips C690, and only the reread runs.
fn run_carrying_class63(
    frame: &mut Class49TerminalFrame<'_>,
    id: u32,
) -> Result<LiveActorDeathResult<()>, Class49TerminalBlock> {
    use Class49TerminalBlock as Block;
    if !begin_carrying_class63_death(frame.entities, id, frame.resources, frame.world_fx)
        .map_err(Block::Death)?
    {
        return Ok(LiveActorDeathResult {
            returned_nonzero: false,
            publication: None,
        });
    }
    let tasks: &mut dyn crate::native_actor_capture::CaptureTaskCustody = match &mut frame.world {
        Class49WorldContext::Cinematic { actor_tasks, .. } => &mut **actor_tasks,
        Class49WorldContext::Playing { scheduler, .. } => &mut **scheduler,
    };
    let completion = crate::native_actor_capture::execute_capture_root(
        frame.entities,
        id,
        crate::native_actor_capture::CaptureRootCallback::Cleanup,
        &mut crate::native_actor_capture::CaptureContext {
            resources: Some(&*frame.resources),
            tasks,
            world_fx: frame.world_fx,
            notifications: frame.notifications,
            retail_tick: frame.retail_tick,
            // D040 only releases; the result screen matters to a kill.
            result_screen: crate::main_base_type9_abort::MainBaseType9ResultScreenState::NotShown,
            hive_dying: Default::default(),
        },
    )
    .map_err(Block::Capture)?;
    let installs = if completion.class63_install { 2 } else { 1 };
    for _ in 0..installs {
        let receipt = reinstall_class63_alternate(
            frame.entities,
            id,
            frame.resources,
            frame.world_fx,
            frame.retail_tick,
        )
        .map_err(Block::Death)?;
        terminal(frame, receipt)?;
    }
    Ok(LiveActorDeathResult {
        returned_nonzero: true,
        publication: None,
    })
}

/// Claim, BAF0's static then dynamic radial, and the policy's finish.
fn terminal(
    frame: &mut Class49TerminalFrame<'_>,
    receipt: Class49TerminalReceipt,
) -> Result<(), Class49TerminalBlock> {
    use Class49TerminalBlock as Block;
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
    Ok(())
}

#[cfg(test)]
mod tests;
