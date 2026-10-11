//! Late11AD0/12CF0 static contact for separately owned A/D insect allocations.
//!
//! This runs after particles, outside12DA0 and its body-basis rebuild. It
//! preserves the original selected plane across02CA0 and delivers11760's
//! collision packet to the static cell before the actor. The owner mirrors
//! [`crate::intro2_type17::contact`] with the Type53 style allowlist,
//! no-Sub-I Sub-A topology and the shared class12 death publisher.
//!
//! Before sharing: Type53 living39 and Class12-28 skip D9B0; Type122
//! living439 and Class12-428 execute it. Each retains its own receipt, metadata
//! and task graph. Class26 Furniture owns the C890 style+1C callback; the
//! remaining admitted style+1C hooks are null. Carrying styles retain their
//! separately authenticated native relation and actual task graph.
//!
//! Wander-based living Primary constructors install02CA0 at task `+0x20`, including
//! the Chase installer `0x403360` (which builds the `0x403490` Chase behavior
//! with its 423030/01430/0x380/0x400 controller and stores02CA0 at site
//! `0x4033C3`). A coexisting Tertiary Aim carries no WanderPrivate record, so
//! only the Primary's shared private state is committed; any other Tertiary
//! graph stays fail-closed.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    damage::{velocity_delta_impact_raw, DamageDeliveryRecord, DamagePacket},
    entity_collision_state::{
        active_model_slot_from_state_flags, ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
        ACTIVE_MODEL_SLOT_LOW_STATE_BIT, REMOTE_OWNED_STATE_BIT,
    },
    follow_beacons::static_contact::{
        plan_wander_private_static_contact, FollowBeaconsStaticContactTopology,
        WanderPrivateStaticContactRequest,
    },
    intro2_common_dying::{Intro2CommonDyingBlock, Intro2CommonDyingOwner},
    intro2_contacts::Intro2ContactFrame,
    intro2_radial::Intro2RadialTaskCustody,
    live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageError,
        LiveActorDamageOutcome, LiveActorDamageRequest, LiveActorDeathResult,
    },
    native_actor_capture::pair::PlayingPlayerContact,
    static_contact::{
        apply_contact_response_raw, scan_deepest_static_contact, static_contact_subject_eligible,
        StaticContactError, StaticContactQuery, StaticModelContact,
    },
    static_damage::StaticDamageOutcome,
    static_damage_live::{resolve_current_static_damage_target, CurrentStaticDamageLookupError},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeStaticActorDeathBlock {
    Common(Intro2CommonDyingBlock),
    Flying(crate::native_flying_surface_contact::NativeFlyingSurfaceDeathBlock),
    /// A class63 carrier's BAF0/BC90 terminal blocked after its prefix.
    AutoPilot(Box<crate::class49_terminal::Class49TerminalBlock>),
    /// An alternate-class1 actor's BAF0/BAC0 terminal blocked after its prefix.
    Class1(Box<crate::class49_terminal::Class49TerminalBlock>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeGroundContactBlock {
    Runtime(&'static str),
    UnsupportedStyle(u32),
    Scan(StaticContactError),
    StaticLookup(CurrentStaticDamageLookupError),
    UnsupportedStaticKind(u32),
    StaticBurn(crate::static_terrain_burn::StaticTerrainBurnFailure),
    Damage(LiveActorDamageError<NativeStaticActorDeathBlock, NativeGroundTerminalPublication>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeGroundContactApplied {
    pub contact: StaticModelContact,
    pub position_before_raw: [i16; 3],
    pub position_after_raw: [i16; 3],
    pub velocity_before_raw: [i16; 3],
    pub velocity_after_raw: [i16; 3],
    pub impact_raw: i32,
    /// D9B0 generic crush precedes11760 and never invokes C690.
    pub crushing_damage: Option<StaticDamageOutcome>,
    /// Class26 C890 delivery/reselection follows the generic D9B0 crush.
    pub furniture_damage: Option<StaticDamageOutcome>,
    pub static_damage: Option<StaticDamageOutcome>,
    pub actor_damage: Option<LiveActorDamageOutcome<NativeGroundTerminalPublication>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeGroundContactOutcome {
    Ineligible,
    Miss,
    Applied(NativeGroundContactApplied),
    Blocked {
        reason: NativeGroundContactBlock,
        committed_prefix: bool,
    },
}

pub(crate) fn resolve_native_ground_static_contact<P: NativeGroundActorProfile>(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
) -> NativeGroundContactOutcome {
    resolve_native_ground_static_contact_with_playing::<P>(frame, id, None)
}

/// Playing's walk lends its player to a terminal profile's BAF0 radial.
pub(crate) fn resolve_native_ground_static_contact_with_playing<P: NativeGroundActorProfile>(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    playing: Option<PlayingPlayerContact<'_>>,
) -> NativeGroundContactOutcome {
    resolve_profile_static_contact(
        frame,
        id,
        StaticActorProfile {
            entity_type: P::ENTITY_TYPE,
            default_flags: P::DEFAULT_FLAGS,
            retained_entry_model_id: None,
            manager_authenticates: P::manager_authenticates,
            metadata_authenticates: P::metadata_authenticates,
            completed_owner: |tasks, manager, id| {
                // A finished BAC0/BC90 corpse keeps only its terminal receipt.
                (P::TERMINAL_DEATH
                    && crate::class49_death::finished_terminal_hit_authenticates(manager, id))
                    || tasks.prepare_native_actor_mutation(manager, id)
            },
            publish_standard_death: |manager, id, context| {
                if P::TERMINAL_DEATH {
                    return P::publish_standard_death(
                        manager,
                        id,
                        &mut NativeGroundDeathContext::Terminal {
                            resources: context.resources,
                            fx: context.world_fx,
                            static_damage: context.static_damage,
                            notifications: context.notifications,
                            retail_tick: context.retail_tick,
                            tasks: context.tasks,
                            player: context.player.as_mut().map(PlayingPlayerContact::reborrow),
                        },
                    )
                    .map_err(NativeStaticActorDeathBlock::Common);
                }
                P::publish_standard_death(
                    manager,
                    id,
                    &mut if P::CAPTURE_POLICY == NativeCapturePolicy::AbsentJ {
                        NativeGroundDeathContext::Split { resources: context.resources, fx: context.world_fx, tick: context.retail_tick, tasks: context.tasks }
                    } else {
                        NativeGroundDeathContext::Capture { resources: context.resources,
                            context: crate::native_actor_capture::CaptureContext {
                                resources: Some(context.resources),
                                tasks: context.tasks, world_fx: context.world_fx,
                                notifications: context.notifications, retail_tick: context.retail_tick,
                                result_screen: crate::main_base_type9_abort::MainBaseType9ResultScreenState::NotShown,
                                hive_dying: Default::default(),
                            }
                        }
                    },
                )
                .map_err(NativeStaticActorDeathBlock::Common)
            },
        },
        playing,
    )
}

/// Native Type16/26/94 share static geometry, A8B0 and D920/11760 phases.
/// Each retained allocation and task graph supplies its own admission.
/// Type16/94 construction remains cinematic-only; Type26 has ordinary receipts.
/// Type94 retains its six-foot H and water Sub-C; this phase does not visit them.
pub fn resolve_insect_static_contact(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
) -> NativeGroundContactOutcome {
    resolve_insect_static_contact_with_playing(frame, id, None)
}

/// Playing's walk lends its player to a terminal static death's radial: the
/// class63 carriers' BC90 and Type43's class1 BAC0 both run BAF0's 4566E0.
pub fn resolve_insect_static_contact_with_playing(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    playing: Option<PlayingPlayerContact<'_>>,
) -> NativeGroundContactOutcome {
    let Some(entity) = frame.entities.iter_all().find(|entity| entity.id == id) else {
        return NativeGroundContactOutcome::Ineligible;
    };
    // Ordinary Type94 construction is not owned by this adapter. Its
    // retained Intro2 allocation, rather than type/model equality, admits it.
    if entity.entity_type == 94
        && !crate::intro2_type94::intro2_type94_allocation_authenticates(entity)
    {
        return NativeGroundContactOutcome::Ineligible;
    }
    let profile = match entity.entity_type {
        16 => StaticActorProfile {
            entity_type: 16,
            default_flags: 0x439,
            retained_entry_model_id: None,
            manager_authenticates: |manager, id| {
                manager
                    .iter_all()
                    .find(|entity| entity.id == id)
                    .is_some_and(crate::intro2_type16::intro2_type16_allocation_authenticates)
            },
            metadata_authenticates: |metadata| {
                crate::intro2_type16::authenticate_metadata(
                    crate::intro2_type16::Type16Row::Type16,
                    metadata,
                )
                .is_ok()
            },
            completed_owner: |tasks, manager, id| tasks.prepare_native_actor_mutation(manager, id),
            publish_standard_death: |manager, id, context| {
                crate::intro2_common_dying::publish_intro2_common_standard_death(
                    manager,
                    id,
                    context.world_fx,
                )
                .map(common_death_result)
                .map_err(NativeStaticActorDeathBlock::Common)
            },
        },
        // Type128 is Type16's row with alternate class63: its death is the
        // shared BAF0/BC90 terminal through this walk's lent radial owner, and
        // a finished corpse keeps its tasks until 14990.
        128 => StaticActorProfile {
            entity_type: 128,
            default_flags: 0x439,
            retained_entry_model_id: None,
            manager_authenticates: |manager, id| {
                manager
                    .iter_all()
                    .find(|entity| entity.id == id)
                    .is_some_and(crate::intro2_type16::intro2_type16_allocation_authenticates)
            },
            metadata_authenticates: |metadata| {
                crate::intro2_type16::authenticate_metadata(
                    crate::intro2_type16::Type16Row::Type128,
                    metadata,
                )
                .is_ok()
            },
            completed_owner: |tasks, manager, id| {
                crate::class49_death::finished_terminal_hit_authenticates(manager, id)
                    || tasks.prepare_native_actor_mutation(manager, id)
            },
            publish_standard_death: |manager, id, context| {
                crate::class49_terminal::run_class49_standard_death(
                    crate::class49_terminal::Class49TerminalFrame {
                        entities: manager,
                        resources: context.resources,
                        world_fx: context.world_fx,
                        static_damage: context.static_damage,
                        notifications: context.notifications,
                        retail_tick: context.retail_tick,
                        world: crate::class49_terminal::Class49WorldContext::for_contact(
                            context.tasks,
                            context.player.as_mut().map(PlayingPlayerContact::reborrow),
                        ),
                    },
                    id,
                )
                .map(|result| LiveActorDeathResult {
                    returned_nonzero: result.returned_nonzero,
                    publication: None,
                })
                .map_err(|error| NativeStaticActorDeathBlock::AutoPilot(Box::new(error)))
            },
        },
        26 => StaticActorProfile {
            entity_type: 26,
            default_flags: 0x439,
            retained_entry_model_id: None,
            manager_authenticates:
                crate::intro2_type26_defecate_virus::type26_manager_allocation_authenticates,
            metadata_authenticates: |metadata| {
                crate::intro2_type26_defecate_virus::authenticate_metadata(metadata).is_ok()
            },
            completed_owner: |tasks, manager, id| tasks.prepare_native_actor_mutation(manager, id),
            publish_standard_death: |manager, id, context| {
                crate::intro2_common_dying::publish_intro2_common_standard_death(
                    manager,
                    id,
                    context.world_fx,
                )
                .map(common_death_result)
                .map_err(NativeStaticActorDeathBlock::Common)
            },
        },
        94 => StaticActorProfile {
            entity_type: 94,
            default_flags: 0x439,
            retained_entry_model_id: None,
            manager_authenticates: |manager, id| {
                manager
                    .iter_all()
                    .find(|entity| entity.id == id)
                    .is_some_and(crate::intro2_type94::intro2_type94_allocation_authenticates)
            },
            metadata_authenticates: |metadata| {
                crate::intro2_type94::authenticate_metadata(metadata).is_ok()
            },
            completed_owner: |tasks, manager, id| tasks.prepare_native_actor_mutation(manager, id),
            publish_standard_death: |manager, id, context| {
                crate::intro2_common_dying::publish_intro2_common_standard_death(
                    manager,
                    id,
                    context.world_fx,
                )
                .map(common_death_result)
                .map_err(NativeStaticActorDeathBlock::Common)
            },
        },
        // The Search styles reverse +C0's 0x20000, so the living shooter
        // keeps 0x8000 and a clear 0x08000000: 11AD0 scans it. Its lethal
        // collision damage publishes class1 through the shared terminal.
        43 => StaticActorProfile {
            entity_type: crate::native_type43::ENTITY_TYPE,
            default_flags: crate::native_type43::DEFAULT_FLAGS,
            retained_entry_model_id: None,
            manager_authenticates: crate::native_type43::manager_allocation_authenticates,
            metadata_authenticates: |metadata| {
                crate::native_type43::authenticate_metadata(metadata).is_ok()
            },
            completed_owner: |tasks, manager, id| {
                crate::class49_death::finished_terminal_hit_authenticates(manager, id)
                    || tasks.prepare_native_actor_mutation(manager, id)
            },
            publish_standard_death: |manager, id, context| {
                crate::class49_terminal::run_class49_standard_death(
                    crate::class49_terminal::Class49TerminalFrame {
                        entities: manager,
                        resources: context.resources,
                        world_fx: context.world_fx,
                        static_damage: context.static_damage,
                        notifications: context.notifications,
                        retail_tick: context.retail_tick,
                        world: crate::class49_terminal::Class49WorldContext::for_contact(
                            context.tasks,
                            context.player.as_mut().map(PlayingPlayerContact::reborrow),
                        ),
                    },
                    id,
                )
                .map(|result| LiveActorDeathResult {
                    returned_nonzero: result.returned_nonzero,
                    publication: None,
                })
                .map_err(|error| NativeStaticActorDeathBlock::Class1(Box::new(error)))
            },
        },
        _ => return NativeGroundContactOutcome::Ineligible,
    };
    resolve_profile_static_contact(frame, id, profile, playing)
}

#[derive(Clone, Copy)]
struct StaticActorProfile {
    entity_type: u32,
    default_flags: u32,
    retained_entry_model_id: Option<usize>,
    manager_authenticates: fn(&EntityManager, u32) -> bool,
    metadata_authenticates: fn(&EntityTypeRuntimeMetadata) -> bool,
    completed_owner: fn(
        &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler,
        &EntityManager,
        u32,
    ) -> bool,
    publish_standard_death: fn(
        &mut EntityManager,
        u32,
        &mut StaticDeathContext<'_>,
    ) -> Result<
        LiveActorDeathResult<NativeGroundTerminalPublication>,
        NativeStaticActorDeathBlock,
    >,
}

/// The static walk owns a concrete scheduler even when a ground family's
/// death callback needs Capture's narrower child-custody interface.
struct StaticDeathContext<'a> {
    resources: &'a mut crate::resource_cache::ResourceCache,
    static_damage: &'a mut crate::static_damage::StaticDamageScheduler,
    tasks: &'a mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler,
    world_fx: &'a mut crate::world_fx::WorldFx,
    notifications: &'a mut crate::gameplay_notifications::GameplayNotifications,
    retail_tick: u32,
    /// Playing's player, lent only by a walk that owns it.
    player: Option<PlayingPlayerContact<'a>>,
}

fn common_death_result(
    owner: Option<Intro2CommonDyingOwner>,
) -> LiveActorDeathResult<NativeGroundTerminalPublication> {
    LiveActorDeathResult {
        returned_nonzero: owner.is_some(),
        publication: owner.map(NativeGroundTerminalPublication::CommonDying),
    }
}

/// Living G-flight families share static geometry and02CA0/019C0, while their
/// actual allocation, completed task owner and alternate death remain distinct.
pub fn resolve_flying_static_contact(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
) -> NativeGroundContactOutcome {
    resolve_flying_static_contact_entry(frame, id, None, None)
}

/// Playing's walk lends its player to a Class1 static death's radial.
pub fn resolve_flying_static_contact_with_playing(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    playing: Option<PlayingPlayerContact<'_>>,
) -> NativeGroundContactOutcome {
    resolve_flying_static_contact_entry(frame, id, None, playing)
}

/// Suffix of the same admitted living11AD0 surface visit. The source model
/// and entry gates survive a synchronous C470 quiet-death callback.
pub(crate) fn resolve_flying_static_contact_continuation(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    entry_model_id: usize,
) -> NativeGroundContactOutcome {
    resolve_flying_static_contact_entry(frame, id, Some(entry_model_id), None)
}

pub(crate) fn resolve_flying_static_contact_continuation_with_playing(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    entry_model_id: usize,
    playing: Option<PlayingPlayerContact<'_>>,
) -> NativeGroundContactOutcome {
    resolve_flying_static_contact_entry(frame, id, Some(entry_model_id), playing)
}

fn resolve_flying_static_contact_entry(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    retained_entry_model_id: Option<usize>,
    playing: Option<PlayingPlayerContact<'_>>,
) -> NativeGroundContactOutcome {
    let Some(entity) = frame.entities.iter_all().find(|entity| entity.id == id) else {
        return NativeGroundContactOutcome::Ineligible;
    };
    let kind = entity.entity_type;
    if !matches!(kind, 13 | 15 | 87 | 10 | 5 | 80 | 126 | 57) {
        return NativeGroundContactOutcome::Ineligible;
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return NativeGroundContactOutcome::Blocked {
            reason: NativeGroundContactBlock::Runtime("flying static style"),
            committed_prefix: false,
        };
    };
    if matches!(context.active_style().style_address(), 0x4c7f60 | 0x4c7fa8) {
        // Existing Class11 C750 terminal adapters own static contact.
        return NativeGroundContactOutcome::Ineligible;
    }
    let Some(metadata) = frame.entities.type_runtime_metadata(kind) else {
        return NativeGroundContactOutcome::Blocked {
            reason: NativeGroundContactBlock::Runtime("flying static metadata"),
            committed_prefix: false,
        };
    };
    let Some(initializer) = metadata.initializer.as_ref() else {
        return NativeGroundContactOutcome::Blocked {
            reason: NativeGroundContactBlock::Runtime("flying static default policy"),
            committed_prefix: false,
        };
    };
    let profile = StaticActorProfile {
        entity_type: kind,
        default_flags: initializer.initializer_state_flags_raw,
        retained_entry_model_id,
        manager_authenticates: |manager, id| {
            manager
                .iter_all()
                .find(|entity| entity.id == id)
                .is_some_and(|entity| match entity.entity_type {
                    13 => crate::intro2_type13_live::authenticate_intro2_type13(entity).is_ok(),
                    10 | 5 | 80 | 126 => {
                        crate::intro2_type10::intro2_type10_allocation_authenticates(entity)
                    }
                    57 => crate::intro2_type57::intro2_type57_allocation_authenticates(entity),
                    15 | 87 => crate::intro2_flyers_live::flyer_identity_authenticates(entity),
                    _ => false,
                })
        },
        // Static019C0 consumes only these common component-presence words.
        // Each constructor/task owner above already authenticates its retained
        // G allocation; no flight-force or K/L callback runs here.
        metadata_authenticates: |metadata| {
            matches!(metadata.common_mover_topology,
            RetailRuntimeValue::Known(topology) if !topology.sub_a && !topology.sub_i && !topology.sub_f && topology.sub_g)
        },
        completed_owner: |tasks, manager, id| {
            let Some(entity) = manager.iter_all().find(|entity| entity.id == id) else {
                return false;
            };
            match entity.entity_type {
                13 => {
                    crate::class49_death::finished_terminal_hit_authenticates(manager, id)
                        || (!tasks.intro2_type13_has_pending_prefix(id)
                            && tasks.intro2_type13_completed_owner(manager, id))
                }
                10 | 5 => {
                    !tasks.intro2_type10_has_pending_prefix(id)
                        && tasks.intro2_type10_completed_owner(manager, id)
                }
                // A finished class63 corpse keeps its tasks until 14990.
                80 | 126 => {
                    crate::class49_death::finished_terminal_hit_authenticates(manager, id)
                        || (!tasks.intro2_type10_has_pending_prefix(id)
                            && tasks.intro2_type10_completed_owner(manager, id))
                }
                57 => {
                    !tasks.intro2_type57_has_pending_prefix(id)
                        && tasks.intro2_type57_completed_owner(manager, id)
                }
                15 | 87 => {
                    crate::native_flying_surface_contact::native_flyer_quiet_death_authenticates(
                        manager, id,
                    ) || tasks.intro2_flyer_completed_owner(manager, id)
                }
                _ => false,
            }
        },
        publish_standard_death: |manager, id, context| {
            use crate::native_flying_surface_contact::{
                register_native_flying_death, run_native_flying_standard_death,
            };
            let result = run_native_flying_standard_death(
                crate::class49_terminal::Class49TerminalFrame {
                    entities: manager,
                    resources: context.resources,
                    world_fx: context.world_fx,
                    static_damage: context.static_damage,
                    notifications: context.notifications,
                    retail_tick: context.retail_tick,
                    world: crate::class49_terminal::Class49WorldContext::for_contact(
                        context.tasks,
                        context.player.as_mut().map(PlayingPlayerContact::reborrow),
                    ),
                },
                id,
            )
            .map_err(NativeStaticActorDeathBlock::Flying)?;
            if let Some(publication) = result.publication {
                register_native_flying_death(context.tasks, publication);
            }
            // Preserve native Class11 or C470 custody; no fabricated Class12
            // owner is returned through the ground family's publication type.
            Ok(LiveActorDeathResult {
                returned_nonzero: result.returned_nonzero,
                publication: None,
            })
        },
    };
    resolve_profile_static_contact(frame, id, profile, playing)
}

fn resolve_profile_static_contact(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    profile: StaticActorProfile,
    playing: Option<PlayingPlayerContact<'_>>,
) -> NativeGroundContactOutcome {
    let mut committed = false;
    match resolve(frame, id, profile, playing, &mut committed) {
        Ok(outcome) => outcome,
        Err(reason) => {
            if committed {
                // A completed death publication has already replaced the living
                // owner. The scheduler parks whichever exact prefix survived.
                if matches!(profile.entity_type, 15 | 87) {
                    if !frame
                        .actor_tasks
                        .park_native_contact_prefix(frame.entities, id)
                    {
                        frame.actor_tasks.park_intro2_flyer_contact_prefix(id);
                    }
                } else {
                    frame
                        .actor_tasks
                        .park_native_contact_prefix(frame.entities, id);
                }
            }
            NativeGroundContactOutcome::Blocked {
                reason,
                committed_prefix: committed,
            }
        }
    }
}

fn resolve(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    profile: StaticActorProfile,
    playing: Option<PlayingPlayerContact<'_>>,
    committed: &mut bool,
) -> Result<NativeGroundContactOutcome, NativeGroundContactBlock> {
    use NativeGroundContactBlock as Block;
    let entity = frame
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Runtime("allocation"))?;
    if entity.entity_type != profile.entity_type {
        return Ok(NativeGroundContactOutcome::Ineligible);
    }
    if !(profile.manager_authenticates)(frame.entities, id) {
        return Err(Block::Runtime("native allocation"));
    }
    let selector = bits(
        entity,
        ACTIVE_MODEL_SLOT_LOW_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
    )?;
    let model_id = match profile.retained_entry_model_id {
        Some(model_id) => model_id,
        None => entity
            .model_in_slot(active_model_slot_from_state_flags(selector))
            .ok_or(Block::Runtime("active model"))?,
    };
    let model = frame
        .resources
        .global_model(model_id)
        .ok_or(Block::Runtime("active model resource"))?;
    if profile.retained_entry_model_id.is_none() {
        match static_contact_subject_eligible(&entity.collision, model.collision_radius_raw) {
            RetailRuntimeValue::Known(false) => return Ok(NativeGroundContactOutcome::Ineligible),
            RetailRuntimeValue::Unresolved => {
                return Err(Block::Runtime("static scan eligibility"))
            }
            RetailRuntimeValue::Known(true) => {}
        }
    }
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        return Err(Block::Runtime("physical basis"));
    };
    let contact = scan_deepest_static_contact(StaticContactQuery {
        terrain: frame
            .resources
            .level_terrain()
            .ok_or(Block::Runtime("terrain"))?,
        terrain_objects: frame
            .resources
            .terrain_objects()
            .ok_or(Block::Runtime("terrain objects"))?,
        model_pool: &*frame.resources,
        tick: frame.retail_tick,
        active_model: model,
        active_model_to_world_basis: basis
            .orientation_world_from_model()
            .map(|row| row.map(f64::from)),
        active_anim_vars: &entity.presentation_anim_vars(frame.retail_tick),
        position_raw: entity.position_raw(),
    })
    .map_err(Block::Scan)?;
    let Some(contact) = contact else {
        return Ok(NativeGroundContactOutcome::Miss);
    };
    // A8B0 is reached only after the static scan hits. The admitted living
    // styles and Class12 below are the same set the native particle entry
    // authenticates; any other style fails closed here.
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Runtime("current style"));
    };
    if ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .into_iter()
        .filter_map(|slot| entity.actor_tasks.task_in_slot(slot))
        .any(|task| {
            entity
                .actor_tasks
                .wrapper_flags(task)
                .is_none_or(|flags| !flags.alive || flags.in_callback)
        })
    {
        return Err(Block::Runtime("completed static task wrappers"));
    }
    if !(matches!(
        context.active_style().style_address(),
        0x4c7a50
            | 0x4c7a98
            | 0x4c7ae0
            | 0x4c7ff0
            | 0x4c8038
            | 0x4c7b28
            | 0x4c7b70
            | 0x4c7ed0
            | 0x4c8080
            | 0x4c80c8
            | 0x4c8110
            | 0x4c8158
    ) || (matches!(profile.entity_type, 16 | 128 | 26 | 56 | 18)
        && context.active_style().style_address() == 0x4c7e88)
        || (matches!(
            profile.entity_type,
            13 | 10 | 5 | 80 | 126 | 57 | 16 | 128 | 15 | 87 | 94 | 38 | 129
        ) && matches!(
            context.active_style().style_address(),
            0x4c7930 | 0x4c7978 | 0x4c74f8
        ))
        || (matches!(profile.entity_type, 15 | 87)
            && profile.retained_entry_model_id.is_some()
            && context.active_style().style_address() == 0x4c7420)
        || (matches!(profile.entity_type, 13 | 43 | 38)
            && context.active_style().style_address() == 0x4c7150
            && crate::class49_death::finished_terminal_hit_authenticates(frame.entities, id))
        || (profile.entity_type == 43 && context.active_style().style_address() == 0x4c74f8)
        || (matches!(profile.entity_type, 80 | 126 | 128 | 129)
            && context.active_style().style_address() == 0x4c7198
            && crate::class49_death::finished_terminal_hit_authenticates(frame.entities, id))
        || (matches!(profile.entity_type, 26 | 18)
            && context.active_style().style_address() == 0x4c7738)
        || (crate::native_type30::allocation_authenticates(entity)
            && matches!(context.active_style().style_address(), 0x4c7930 | 0x4c7738))
        || (profile.entity_type == 56
            && matches!(
                context.active_style().style_address(),
                0x4c7618 | 0x4c7660 | 0x4c74f8
            )))
    {
        return Err(Block::UnsupportedStyle(
            context.active_style().style_address(),
        ));
    }
    if matches!(
        context.active_style().style_address(),
        0x4c8080 | 0x4c80c8 | 0x4c8110 | 0x4c8158
    ) {
        crate::native_actor_capture::NativeCaptorProfile::authenticate(frame.entities, id)
            .map_err(|_| Block::Runtime("native Capture transport profile"))?;
        if crate::native_actor_capture::carry_tasks::carrying_variant(entity).is_none() {
            return Err(Block::Runtime("Capture transport style"));
        }
        crate::native_actor_capture::validate_capture_relation(frame.entities, id)
            .map_err(|_| Block::Runtime("Capture transport relation"))?;
    }
    if !(profile.completed_owner)(frame.actor_tasks, frame.entities, id) {
        return Err(Block::Runtime("completed contact owner"));
    }
    apply_contact(frame, id, profile, contact, playing, committed)
        .map(NativeGroundContactOutcome::Applied)
}

fn apply_contact(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    profile: StaticActorProfile,
    contact: StaticModelContact,
    mut playing: Option<PlayingPlayerContact<'_>>,
    committed: &mut bool,
) -> Result<NativeGroundContactApplied, NativeGroundContactBlock> {
    use NativeGroundContactBlock as Block;
    let metadata = frame
        .entities
        .type_runtime_metadata(profile.entity_type)
        .ok_or(Block::Runtime("metadata"))?;
    if !(profile.metadata_authenticates)(metadata) {
        return Err(Block::Runtime("metadata"));
    }
    let common_mover_topology = metadata.common_mover_topology;
    let type_record = frame
        .resources
        .global_entity_type(profile.entity_type as usize)
        .ok_or(Block::Runtime("type record"))?;
    let entity = frame
        .entities
        .entity_mut(id)
        .ok_or(Block::Runtime("allocation"))?;
    if entity.capability_flags != 8
        || entity.collision.default_state_flags_at_0xc8
            != RetailRuntimeValue::Known(profile.default_flags)
    {
        return Err(Block::Runtime("native ground static callback policy"));
    }
    let cue = u16::from_le_bytes([type_record.raw_header[0x8a], type_record.raw_header[0x8b]]);
    let hook = contact_task_hook(entity)?;
    if cue != 0 {
        *committed = true;
        frame
            .world_fx
            .queue_fixed_positional_sound_raw(cue, entity.position_raw());
    }
    if let NativeGroundStaticTaskHook::WanderPrivate(private_state) = hook {
        let primary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let RetailRuntimeValue::Known(topology) = common_mover_topology else {
            return Err(Block::Runtime("static topology"));
        };
        let sub_a = match entity.sub_a_propulsion_runtime {
            RetailRuntimeValue::Known(Some(a)) if topology.sub_a => Some(a),
            RetailRuntimeValue::Known(None) if !topology.sub_a => None,
            _ => return Err(Block::Runtime("Sub-A runtime")),
        };
        if topology.sub_i || topology.sub_f {
            return Err(Block::Runtime("insect static direction topology"));
        }
        if topology.sub_g
            && !matches!(
                entity.sub_g_06070_runtime,
                RetailRuntimeValue::Known(Some(_))
            )
        {
            return Err(Block::Runtime("Sub-G static direction owner"));
        }
        let plan = plan_wander_private_static_contact(
            WanderPrivateStaticContactRequest {
                private_state,
                controlled_position_raw: entity.position_raw(),
                heading_raw: entity.heading_raw(),
                topology: RetailRuntimeValue::Known(FollowBeaconsStaticContactTopology {
                    sub_i: false,
                    sub_a,
                    sub_f: false,
                    sub_g: topology.sub_g,
                }),
            },
            || u32::from(frame.world_fx.next_shared_retail_random_u16()),
        )
        .expect("authenticated no-I native static direction hook");
        *committed = true;
        match entity.actor_tasks.task_state_mut(primary).unwrap() {
            ActorTaskRuntime::SharedRetarget(task) => {
                task.apply_static_contact_private_state(plan.private_state_after)
            }
            ActorTaskRuntime::DefecateVirusWander(task) => {
                *task = crate::defecate_virus::DefecateVirusWanderTaskState::from_parts(
                    plan.private_state_after,
                    task.elapsed_ms(),
                );
            }
            ActorTaskRuntime::FollowBeaconsFollowing(task)
            | ActorTaskRuntime::CapturePeopleFollowing(task) => {
                let mut stage = task.stage_callback();
                *stage.private_state_mut() = plan.private_state_after;
                stage.commit(task);
            }
            ActorTaskRuntime::RunAway(task) => {
                let mut stage = task.stage_callback();
                *stage.private_state_mut() = plan.private_state_after;
                stage.commit(task);
            }
            ActorTaskRuntime::ChaseTarget(task) => {
                let mut stage = task.stage_callback();
                *stage.private_state_mut() = plan.private_state_after;
                stage.commit(task);
            }
            ActorTaskRuntime::CapturePeoplePursuit(task) => {
                let mut stage = task.stage_callback();
                *stage.private_state_mut() = plan.private_state_after;
                stage.commit(task);
            }
            _ => unreachable!("preflight retains exact task family"),
        }
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(plan.sub_a_runtime);
        if let Some(reverse) = plan.sub_g_reverse_write {
            let RetailRuntimeValue::Known(Some(g)) = &mut entity.sub_g_06070_runtime else {
                unreachable!("preflight native Sub-G");
            };
            g.apply_direction_reverse_write(reverse);
        }
        entity.set_heading_raw(plan.heading_raw);
    }
    // D920 consumes the current effective policy after A8B0. The admitted
    // living styles have zero set masks; Class12 removes2015. Type122 keeps
    // bit400 in both cases (439 / 428), unlike Type53 (39 / 28).
    let RetailRuntimeValue::Known(Some(style)) = entity.current_behavior_context else {
        return Err(Block::Runtime("post-task static style"));
    };
    let effective_flags = match style.active_style().style_address() {
        0x4c7ed0 => profile.default_flags & !0x2015,
        0x4c7a50 | 0x4c7a98 | 0x4c7ae0 | 0x4c7ff0 | 0x4c8038 | 0x4c7b28 | 0x4c7b70 | 0x4c8080
        | 0x4c80c8 | 0x4c8110 | 0x4c8158 | 0x4c7e88 | 0x4c7930 | 0x4c7738 | 0x4c74f8 | 0x4c7420
        | 0x4c7150 | 0x4c7198 | 0x4c7618 | 0x4c7660 => profile.default_flags,
        0x4c7978 => (profile.default_flags | 0x80) & !2,
        address => return Err(Block::UnsupportedStyle(address)),
    };
    let crushing_damage = if effective_flags & 0x400 != 0 {
        // D9B0 ->27950 uses source -5/owner0; static damage consumes only
        // the packet. Retain this provenance rather than substituting the
        // actor's ordinary collision source. The selected plane is unchanged.
        let delivery = DamageDeliveryRecord {
            packet: DamagePacket {
                channels: [1, 0],
                amounts_raw: [40_000, 0],
            },
            source_entity_type_raw: (-5i32) as u32,
            owner_handle: 0,
        };
        *committed = true;
        deliver_static(frame, contact.cell, delivery.packet)?
    } else {
        None
    };
    let furniture_damage = if matches!(hook, NativeGroundStaticTaskHook::Furniture) {
        *committed = true;
        let damage = deliver_static(
            frame,
            contact.cell,
            DamagePacket {
                channels: [1, 0],
                amounts_raw: [40_000, 0],
            },
        )?;
        // C890 calls C690 irrespective of filtering, duplicate registration or
        // the preceding generic crush having already changed this static cell.
        reselect_after_furniture_contact(frame, id)?;
        damage
    } else {
        None
    };
    // C890 has completed above; all other admitted style+1C hooks are null.
    // Re-resolve the survivor after D9B0,
    // then11760 uses the retained plane with the current motion and mass.
    let entity = frame
        .entities
        .entity_mut(id)
        .ok_or(Block::Runtime("post-static-callback allocation"))?;
    let position_before_raw = entity.position_raw();
    let velocity_before_raw = entity.velocity_raw();
    let mut position_after_raw = position_before_raw;
    let mut velocity_after_raw = velocity_before_raw;
    apply_contact_response_raw(&mut position_after_raw, &mut velocity_after_raw, contact);
    *committed = true;
    entity.set_motion_raw(position_after_raw, velocity_after_raw);
    let impact_raw = if bits(entity, REMOTE_OWNED_STATE_BIT)? == 0 {
        velocity_delta_impact_raw(velocity_before_raw, velocity_after_raw, entity.mass_raw)
    } else {
        0
    };
    let mut applied = NativeGroundContactApplied {
        contact,
        position_before_raw,
        position_after_raw,
        velocity_before_raw,
        velocity_after_raw,
        impact_raw,
        crushing_damage,
        furniture_damage,
        static_damage: None,
        actor_damage: None,
    };
    if impact_raw != 0 {
        let delivery = DamageDeliveryRecord {
            packet: DamagePacket::collision(impact_raw),
            source_entity_type_raw: profile.entity_type,
            owner_handle: id,
        };
        applied.static_damage = deliver_static(frame, contact.cell, delivery.packet)?;
        let result = apply_live_actor_checked_damage(
            frame.entities,
            frame.world_fx,
            LiveActorDamageRequest {
                ratio_numerator: 0,
                ratio_denominator: 0,
                entity_id: id,
                delivery,
                entry: LiveActorDamageEntry::Checked,
                feedback: None,
            },
            |manager, world_fx, _feedback| {
                (profile.publish_standard_death)(
                    manager,
                    id,
                    &mut StaticDeathContext {
                        resources: frame.resources,
                        static_damage: frame.static_damage,
                        tasks: frame.actor_tasks,
                        world_fx,
                        notifications: frame.notifications,
                        retail_tick: frame.retail_tick,
                        player: playing.as_mut().map(PlayingPlayerContact::reborrow),
                    },
                )
            },
        );
        match result {
            Ok(result) => {
                if let Some(owner) = result.death_publication {
                    frame.actor_tasks.register_native_ground_terminal(owner);
                }
                applied.actor_damage = Some(result);
            }
            Err(error) => {
                if let Some(owner) = error.death_publication {
                    frame.actor_tasks.register_native_ground_terminal(owner);
                }
                return Err(Block::Damage(error));
            }
        }
    }
    Ok(applied)
}

fn bits(entity: &Entity, mask: u32) -> Result<u32, NativeGroundContactBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        RetailRuntimeValue::Unresolved => {
            Err(NativeGroundContactBlock::Runtime("contact state bits"))
        }
    }
}

#[derive(Clone, Copy)]
enum NativeGroundStaticTaskHook {
    Null,
    Furniture,
    WanderPrivate(crate::wander_near_location::WanderNearPrivateState),
}

fn contact_task_hook(
    entity: &Entity,
) -> Result<NativeGroundStaticTaskHook, NativeGroundContactBlock> {
    use ActorTaskRuntime as Task;
    use NativeGroundContactBlock as Block;
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Runtime("current style"));
    };
    let primary = entity.actor_task_state(ActorTaskSlot::Primary);
    let secondary = entity.actor_task_state(ActorTaskSlot::Secondary);
    let tertiary = entity.actor_task_state(ActorTaskSlot::Tertiary);
    // ADE0 installs Tertiary Aim alongside its Chase Primary. Aim carries no
    // WanderPrivate record, so only the Primary's shared private state is
    // committed; any other Tertiary graph is an unauthenticated layout.
    let chase_tertiary = match tertiary {
        None => true,
        Some(Task::AimAndFire(_)) => matches!(primary, Some(Task::ChaseTarget(_))),
        Some(Task::DefecateVirusTerrain(_)) => {
            matches!(primary, Some(Task::DefecateVirusWander(_)))
        }
        _ => false,
    };
    if !chase_tertiary {
        return Err(Block::Runtime("contact task graph"));
    }
    // 02B10/03650/03B70/03E20 install the identical02CA0 callback at the
    // Primary's +0x20, as does the Chase installer 0x403360 at site 0x4033C3;
    // their acquisition Secondaries retain05FF0's null hook. Only the
    // Primary's shared private record is written; typed task owners preserve
    // their own lifetime, route and audio records when committing this result.
    match (context.active_style().style_address(), primary, secondary) {
        (0x4c7e88, Some(Task::DefecateVirusWander(task)), None) => Ok(
            NativeGroundStaticTaskHook::WanderPrivate(task.private_state()),
        ),
        (0x4c7930 | 0x4c7978, Some(Task::SharedRetarget(task)), None) => Ok(
            NativeGroundStaticTaskHook::WanderPrivate(task.private_state()),
        ),
        (0x4c7738, Some(Task::TrashFurniture(_)), None)
            if entity.entity_type == 26
                || crate::native_type30::allocation_authenticates(entity)
                || crate::native_type18::allocation_authenticates(entity) =>
        {
            Ok(NativeGroundStaticTaskHook::Furniture)
        }
        (0x4c74f8, None, None) => Ok(NativeGroundStaticTaskHook::Null),
        (0x4c7420, None, None) if matches!(entity.entity_type, 15 | 87) => {
            Ok(NativeGroundStaticTaskHook::Null)
        }
        (0x4c7150, None, None) if matches!(entity.entity_type, 13 | 43 | 38) => {
            Ok(NativeGroundStaticTaskHook::Null)
        }
        // BC90 keeps a carrier's living tasks and A8B0 calls each task's +20
        // without a style or dying test. Only the retained Primary's 02CA0
        // writes; acquisition Secondaries keep 05FF0's null hook.
        (0x4c7198, primary, _) => match primary {
            None => Ok(NativeGroundStaticTaskHook::Null),
            Some(Task::SharedRetarget(task)) => Ok(NativeGroundStaticTaskHook::WanderPrivate(
                task.private_state(),
            )),
            Some(Task::ChaseTarget(task)) => Ok(NativeGroundStaticTaskHook::WanderPrivate(
                task.private_state(),
            )),
            Some(Task::DefecateVirusWander(task)) => Ok(NativeGroundStaticTaskHook::WanderPrivate(
                task.private_state(),
            )),
            Some(Task::CapturePeoplePursuit(task)) => Ok(
                NativeGroundStaticTaskHook::WanderPrivate(task.private_state()),
            ),
            _ => Err(Block::Runtime("contact task graph")),
        },
        (0x4c7b28, Some(Task::SharedRetarget(task)), Some(Task::FollowBeaconAcquisition(_))) => Ok(
            NativeGroundStaticTaskHook::WanderPrivate(task.private_state()),
        ),
        (
            0x4c7ff0 | 0x4c7a50 | 0x4c7ae0 | 0x4c7618,
            Some(Task::SharedRetarget(task)),
            Some(Task::TargetAcquisition(_)),
        ) => Ok(NativeGroundStaticTaskHook::WanderPrivate(
            task.private_state(),
        )),
        (0x4c7b70, Some(Task::FollowBeaconsFollowing(task)), None) => Ok(
            NativeGroundStaticTaskHook::WanderPrivate(task.private_state()),
        ),
        (0x4c8038 | 0x4c8080, Some(Task::CapturePeoplePursuit(task)), None) => Ok(
            NativeGroundStaticTaskHook::WanderPrivate(task.private_state()),
        ),
        (0x4c80c8, Some(Task::SharedRetarget(task)), Some(Task::CaptureBeaconAcquisition)) => Ok(
            NativeGroundStaticTaskHook::WanderPrivate(task.private_state()),
        ),
        (0x4c8110, Some(Task::CapturePeopleFollowing(task)), None) => Ok(
            NativeGroundStaticTaskHook::WanderPrivate(task.private_state()),
        ),
        (0x4c8158, Some(Task::SharedRetarget(task)), None) => Ok(
            NativeGroundStaticTaskHook::WanderPrivate(task.private_state()),
        ),
        (0x4c7660, Some(Task::RunAway(task)), None) => Ok(
            NativeGroundStaticTaskHook::WanderPrivate(task.private_state()),
        ),
        (0x4c7a98, Some(Task::ChaseTarget(task)), None) => Ok(
            NativeGroundStaticTaskHook::WanderPrivate(task.private_state()),
        ),
        (0x4c7ed0, Some(Task::CommonDying(_)), None)
            if entity.collision.state_flags_at_0x08.masked(0x4000)
                == RetailRuntimeValue::Known(0x4000) =>
        {
            Ok(NativeGroundStaticTaskHook::Null)
        }
        _ => Err(Block::Runtime("contact task graph")),
    }
}

fn deliver_static(
    frame: &mut Intro2ContactFrame<'_>,
    cell: [u8; 2],
    packet: DamagePacket,
) -> Result<Option<StaticDamageOutcome>, NativeGroundContactBlock> {
    use NativeGroundContactBlock as Block;
    let Some(target) =
        resolve_current_static_damage_target(frame.resources, cell).map_err(Block::StaticLookup)?
    else {
        return Ok(None);
    };
    let outcome = frame.static_damage.submit_hit(target, packet, &mut || {
        frame.world_fx.next_shared_retail_random_u16()
    });
    match outcome {
        StaticDamageOutcome::UnsupportedKind { kind_index } => {
            return Err(Block::UnsupportedStaticKind(kind_index))
        }
        StaticDamageOutcome::BurnedKind10Transition { cell, .. } => {
            frame.resources.apply_burned_kind_10_transition(cell);
        }
        StaticDamageOutcome::ImmediateBurn { cell, .. } => {
            crate::static_terrain_burn::apply_immediate_static_burn(
                cell,
                frame.resources,
                frame.world_fx,
            )
            .map_err(Block::StaticBurn)?;
        }
        _ => {}
    }
    Ok(Some(outcome))
}

/// C890 dispatches the allocation's own C690 selector after its static packet.
/// Type30 has an unconditional Furniture choice and its own source rule list.
fn reselect_after_furniture_contact(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
) -> Result<(), NativeGroundContactBlock> {
    use NativeGroundContactBlock as Block;
    if crate::native_type18::manager_allocation_authenticates(frame.entities, id) {
        // Type18's root weighs rule8, read against the static cell C890 just
        // damaged.
        if super::behavior::reselect::<crate::native_type18::profile::Type18Profile>(
            frame.entities,
            id,
            frame.retail_tick,
            frame.world_fx,
            Some(frame.resources),
            super::behavior::ReselectionEntry::Impact,
        )
        .is_err()
        {
            if let Ok(owner) =
                crate::native_type18::Type18Owner::adopt_blocked_prefix(frame.entities, id)
            {
                frame.actor_tasks.register_type18(owner);
            }
            return Err(Block::Runtime("Type18 static C690 suffix"));
        }
        let owner = crate::native_type18::Type18Owner::adopt(frame.entities, id)
            .map_err(|_| Block::Runtime("Type18 post-static owner"))?;
        frame.actor_tasks.register_type18(owner);
        return Ok(());
    }
    if crate::native_type30::manager_allocation_authenticates(frame.entities, id) {
        if super::behavior::reselect::<crate::native_type30::profile::Type30Profile>(
            frame.entities,
            id,
            frame.retail_tick,
            frame.world_fx,
            Some(frame.resources),
            super::behavior::ReselectionEntry::Impact,
        )
        .is_err()
        {
            if let Ok(owner) =
                crate::native_type30::Type30Owner::adopt_blocked_prefix(frame.entities, id)
            {
                frame.actor_tasks.register_type30(owner);
            }
            return Err(Block::Runtime("Type30 static C690 suffix"));
        }
        let owner = crate::native_type30::Type30Owner::adopt(frame.entities, id)
            .map_err(|_| Block::Runtime("Type30 post-static owner"))?;
        frame.actor_tasks.register_type30(owner);
        Ok(())
    } else {
        crate::intro2_type26_defecate_virus::reselect_after_static_contact(frame, id)
            .map_err(Block::Runtime)
    }
}

#[cfg(test)]
mod type30_tests;
