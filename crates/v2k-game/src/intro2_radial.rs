//! Intro2's shared static-then-dynamic radial traversal (FUN_004566E0).
//!
//! This runs at the callback site, before the next actor or contact visit.
//! The original live-list walk has no player allocation in Intro2.

use crate::entity::{DynamicRadialLiveOutcome, DynamicRadialLiveRequest, EntityManager};
use crate::gameplay_notifications::GameplayNotifications;
use crate::intro2_meteors::{
    claim_intro2_meteor_radial, finish_intro2_meteor_death, MeteorTerminalReceipt,
};
use crate::radial_damage::RadialDamageTemplate;
use crate::resource_cache::ResourceCache;
use crate::static_damage::{scan_static_radial, StaticDamageOutcome, StaticDamageScheduler};
use crate::static_damage_live::{
    resolve_current_static_damage_target, CurrentStaticDamageLookupError,
};
use crate::world_fx::WorldFx;

/// Radial calls can occur between frames or inside the actor live cursor.
/// Both owners authenticate before writing and adopt each completed death
/// before the next radial call can encounter that allocation again.
pub trait Intro2RadialTaskCustody: crate::native_ground_actor::NativeGroundTaskCustody {
    fn prepare_native_actor_mutation(&mut self, entities: &EntityManager, entity_id: u32) -> bool;
    fn retain_dynamic_result(&mut self, result: &DynamicRadialLiveOutcome);
    fn park_class49_terminal(&mut self, entities: &EntityManager, entity_id: u32);
    fn finish_class49_terminal(
        &mut self,
        receipt: crate::class49_death::Class49TerminalReceipt,
        ring: Option<crate::type60_exploding_ring_production::Type60ExplodingRingProductionOwner>,
    );
}

pub struct Intro2RadialFrame<'a> {
    /// Sealed claims in the currently executing native call stack. A nested
    /// blast may revisit an ancestor before its deferred removal; an unrelated
    /// later call may not borrow a failed terminal's pending custody.
    pub active_terminal_calls: Vec<Intro2RadialTerminalCall>,
    pub entities: &'a mut EntityManager,
    pub resources: &'a mut ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub static_damage: &'a mut StaticDamageScheduler,
    pub notifications: &'a mut GameplayNotifications,
    pub retail_tick: u32,
    pub actor_tasks: &'a mut dyn Intro2RadialTaskCustody,
}

#[derive(Debug, Clone, Copy)]
pub enum Intro2RadialTerminalCall {
    Type10(crate::intro2_type10::death::Intro2Type10TerminalReceipt),
    Type57(crate::intro2_type57::death::Intro2Type57TerminalReceipt),
    Class49(crate::class49_death::Class49TerminalReceipt),
}
impl Intro2RadialTerminalCall {
    pub(crate) fn authenticates(self, manager: &EntityManager, id: u32) -> bool {
        match self {
            Self::Type10(receipt) => {
                receipt.entity_id == id
                    && crate::intro2_type10::death::active_terminal_receipt(manager, &receipt)
            }
            Self::Type57(receipt) => {
                receipt.entity_id == id
                    && crate::intro2_type57::death::active_terminal_receipt(manager, &receipt)
            }
            Self::Class49(receipt) => {
                receipt.entity_id == id
                    && crate::class49_death::active_terminal_receipt(manager, &receipt)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2RadialReport {
    StaticLookupBlocked(CurrentStaticDamageLookupError),
    Applied {
        static_deliveries: Vec<StaticDamageOutcome>,
        dynamic: DynamicRadialLiveOutcome,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2MeteorDeathReport {
    StaleReceipt,
    StaticLookupBlocked(CurrentStaticDamageLookupError),
    Applied {
        static_deliveries: Vec<StaticDamageOutcome>,
        dynamic: DynamicRadialLiveOutcome,
        finalized: bool,
    },
}

pub fn complete_intro2_meteor_death(
    mut frame: Intro2RadialFrame<'_>,
    receipt: MeteorTerminalReceipt,
) -> Intro2MeteorDeathReport {
    if !claim_intro2_meteor_radial(frame.entities, &receipt) {
        return Intro2MeteorDeathReport::StaleReceipt;
    }
    match apply_intro2_radial_damage(&mut frame, receipt.position_raw, receipt.radial_damage) {
        Intro2RadialReport::StaticLookupBlocked(error) => {
            Intro2MeteorDeathReport::StaticLookupBlocked(error)
        }
        Intro2RadialReport::Applied {
            static_deliveries,
            dynamic,
        } => {
            let finalized =
                dynamic.completed() && finish_intro2_meteor_death(frame.entities, receipt);
            Intro2MeteorDeathReport::Applied {
                static_deliveries,
                dynamic,
                finalized,
            }
        }
    }
}

/// Consume one caller-owned radial action exactly once. Both a static program
/// and meteor death use this traversal; neither is a particle hit callback.
pub fn apply_intro2_radial_damage(
    frame: &mut Intro2RadialFrame<'_>,
    origin_raw: [i16; 3],
    template: RadialDamageTemplate,
) -> Intro2RadialReport {
    let Intro2RadialFrame {
        active_terminal_calls,
        entities,
        resources,
        world_fx,
        static_damage,
        notifications,
        retail_tick,
        actor_tasks,
    } = frame;
    let static_deliveries = match apply_static_radial_damage(
        resources,
        static_damage,
        world_fx,
        origin_raw,
        template,
    ) {
        Ok(deliveries) => deliveries,
        Err(error) => return Intro2RadialReport::StaticLookupBlocked(error),
    };
    let dynamic = entities.apply_dynamic_radial_damage_live(DynamicRadialLiveRequest {
        origin_raw,
        template,
        world_fx,
        retail_tick: *retail_tick,
        notifications,
        callbacks: &mut Intro2RadialCallbacks {
            active_terminal_calls,
            resources,
            static_damage,
            actor_tasks: &mut **actor_tasks,
        },
    });
    // Death receipts were adopted at their exact callback boundary so nested
    // scans already saw their new owners. Retain only a final blocked prefix.
    actor_tasks.retain_dynamic_result(&DynamicRadialLiveOutcome {
        blocked: dynamic.blocked.clone(),
        ..Default::default()
    });
    Intro2RadialReport::Applied {
        static_deliveries,
        dynamic,
    }
}

/// The shared 4566E0 static prefix precedes either world's dynamic owner.
pub(crate) fn apply_static_radial_damage(
    resources: &mut ResourceCache,
    static_damage: &mut StaticDamageScheduler,
    world_fx: &mut WorldFx,
    origin_raw: [i16; 3],
    template: RadialDamageTemplate,
) -> Result<Vec<StaticDamageOutcome>, CurrentStaticDamageLookupError> {
    let mut lookup_error = None;
    let hits = resources.terrain().map_or_else(Vec::new, |terrain| {
        scan_static_radial(terrain, origin_raw, template, |cell| {
            match resolve_current_static_damage_target(resources, cell) {
                Ok(target) => target.map(|target| target.state),
                Err(error) => {
                    lookup_error.get_or_insert(error);
                    None
                }
            }
        })
    });
    if let Some(error) = lookup_error {
        return Err(error);
    }
    let mut static_deliveries = Vec::new();
    for hit in hits {
        let outcome = static_damage.submit_hit(hit.target, hit.packet, &mut || {
            world_fx.next_shared_retail_random_u16()
        });
        match outcome {
            StaticDamageOutcome::BurnedKind10Transition { cell, .. } => {
                resources.apply_burned_kind_10_transition(cell);
            }
            StaticDamageOutcome::ImmediateBurn { cell, .. } => {
                crate::static_terrain_burn::apply_immediate_static_burn(cell, resources, world_fx)
                    .map_err(CurrentStaticDamageLookupError::BurnCallback)?;
            }
            _ => {}
        }
        static_deliveries.push(outcome);
    }
    Ok(static_deliveries)
}

pub(crate) struct Intro2RadialCallbacks<'a> {
    pub(crate) active_terminal_calls: &'a [Intro2RadialTerminalCall],
    pub(crate) resources: &'a mut ResourceCache,
    pub(crate) static_damage: &'a mut StaticDamageScheduler,
    pub(crate) actor_tasks: &'a mut dyn Intro2RadialTaskCustody,
}

impl crate::entity::DynamicRadialLiveCallbacks for Intro2RadialCallbacks<'_> {
    fn capture_task_custody(
        &mut self,
    ) -> Option<&mut dyn crate::intro2_type17::capture::CaptureTaskCustody> {
        Some(self.actor_tasks)
    }
    fn retain_death_publication(
        &mut self,
        publication: crate::entity::DynamicRadialDeathPublication,
    ) {
        self.actor_tasks
            .retain_dynamic_result(&DynamicRadialLiveOutcome {
                death_publications: vec![publication],
                ..Default::default()
            });
    }
    fn active_terminal_call(&self, entities: &EntityManager, id: u32) -> bool {
        self.active_terminal_calls
            .iter()
            .any(|claim| claim.authenticates(entities, id))
    }
    fn before_native_actor_mutation(&mut self, entities: &EntityManager, id: u32) -> bool {
        self.actor_tasks.prepare_native_actor_mutation(entities, id)
    }
    fn hive_dying_burst_model_extent_raw(&self, model_id: usize) -> Option<u16> {
        self.resources
            .global_model(model_id)
            .map(|model| model.radius)
    }
    fn hive_dying_burst_sea_level_raw(&self) -> Option<i16> {
        self.resources
            .terrain()
            .and_then(|terrain| terrain.water_enabled().then(|| terrain.sea_level_raw()))
    }
    fn standard_death(
        &mut self,
        entities: &mut EntityManager,
        id: u32,
        kind: u32,
        world_fx: &mut WorldFx,
        retail_tick: u32,
        notifications: &mut GameplayNotifications,
    ) -> Option<
        Result<
            crate::live_actor_checked_damage::LiveActorDeathResult<
                crate::entity::DynamicRadialDeathPublication,
            >,
            crate::entity::DynamicRadialLiveBlockReason,
        >,
    > {
        if kind == 40 && crate::native_type40::manager_allocation_authenticates(entities, id) {
            return Some(crate::native_type40::death::begin_type40_standard_death(
                entities, id, crate::native_type40::death::Type40Class18Frame {
                    resources: self.resources, world_fx, retail_tick, tasks: self.actor_tasks,
                },
            ).map(|result| crate::live_actor_checked_damage::LiveActorDeathResult {
                returned_nonzero: result.returned_nonzero,
                publication: result.publication.map(|terminal| match terminal {
                    crate::native_ground_actor::NativeGroundTerminalPublication::CommonDying(owner) =>
                        crate::entity::DynamicRadialDeathPublication::Intro2Class12(owner),
                    crate::native_ground_actor::NativeGroundTerminalPublication::Deferred(receipt) =>
                        crate::entity::DynamicRadialDeathPublication::NativeGroundDeferred(receipt),
                }),
            }).map_err(crate::entity::DynamicRadialLiveBlockReason::NativeType40));
        }
        if kind != 49
            && !entities.iter_all().any(|entity| {
                entity.id == id && crate::class49_death::source_profile(entity).is_some()
            })
        {
            return None;
        }
        Some(
            crate::class49_terminal::run_class49_standard_death(
                crate::class49_terminal::Class49TerminalFrame::from_cinematic(Intro2RadialFrame {
                    active_terminal_calls: self.active_terminal_calls.to_vec(),
                    entities,
                    resources: self.resources,
                    world_fx,
                    static_damage: self.static_damage,
                    notifications,
                    retail_tick,
                    actor_tasks: self.actor_tasks,
                }),
                id,
            )
            .map(
                |result| crate::live_actor_checked_damage::LiveActorDeathResult {
                    returned_nonzero: result.returned_nonzero,
                    publication: None,
                },
            )
            .map_err(|error| crate::entity::DynamicRadialLiveBlockReason::Class49(Box::new(error))),
        )
    }
}
