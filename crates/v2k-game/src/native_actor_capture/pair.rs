//! Native captor17/122, independent58 and bounded insect/Type66 11AD0 pairs,
//! after the same subject's static contact.
//!
//! The intrusive next chain and actual oriented Section-8 query select contacts.
//! C910's A300 return suppresses only response/damage: the opposite behavior
//! and both freshly read A900 task chains still execute. This lane owns the
//! pairs incident to a native captor, native-Type47 versus bound-hive
//! pairs, native Type9 peers, and authenticated native insects versus owned Type66 buildings in
//! either intrusive seat. Player pairs remain with the player
//! adapter. Follow, RunAway and Class12 retain the same contact program as
//! Capture, with each participant's current callbacks. Each successful phase
//! commits immediately; a later evidence block retains and parks that prefix.

use crate::intro2_type17::{Intro2Type17Block, Intro2Type17Owner};
use crate::native_actor_capture::{self as capture, carry_tasks, NativeCaptorProfile};
use crate::{
    active_pair::{
        plan_active_pair_response_and_damage_cap, ActivePairBody, ActivePairContact,
        ActivePairUnresolved,
    },
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    entity::{Entity, EntityManager},
    entity_behavior::PairContactCallbackPolicy,
    entity_collision_state::{
        RetailRuntimeValue, DYING_STATE_BIT, PAIR_COLLISION_FIXED_STATE_BIT, REMOTE_OWNED_STATE_BIT,
    },
    guard_location_owner::acquisition::{
        select_guard_location_candidate, GuardLocationCandidateFilter, GuardLocationCandidateRange,
        GuardLocationCandidateRequest, GuardLocationCandidateSelection, GuardLocationSearchContext,
    },
    intro2_contacts::Intro2ContactFrame,
    intro2_radial::Intro2RadialTaskCustody,
    intro2_type8::NativeWorkerProfile,
    native_type86::NativeFourChoiceProfile,
    player_active_contact::{active_pair_body_from_entity, classify_oriented_active_pair_contact},
    type17_initial_behavior_live::fresh_type17_candidate_ref,
};

/// Controller phase at the actual call site of 4568B0, independent of actor data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureFeedbackPolicy {
    Gameplay,
    Cinematic,
}

/// Local player authorities required by a nested Playing Class49 blast.
/// An unavailable controller lives byte cannot authorize its death message.
pub struct PlayingPlayerContact<'a> {
    pub hull: &'a mut crate::player_hull::PlayerHull,
    pub extra_lives: RetailRuntimeValue<u8>,
}

impl PlayingPlayerContact<'_> {
    fn reborrow(&mut self) -> PlayingPlayerContact<'_> {
        PlayingPlayerContact {
            hull: self.hull,
            extra_lives: self.extra_lives,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeCaptorPairBlock {
    Runtime(&'static str),
    UnresolvedBehavior {
        entity_id: u32,
        entity_type: u32,
    },
    BodyCustody {
        entity_id: u32,
    },
    Geometry(ActivePairUnresolved),
    Capture(capture::CaptureBlock),
    Task(Intro2Type17Block),
    GroundTask(crate::native_ground_actor::NativeGroundActorBlock),
    CaptureTask(carry_tasks::CaptureTaskBlock),
    Component(crate::native_actor_descriptor_contact::NativeDescriptorContactBlock),
    Damage(
        crate::live_actor_checked_damage::LiveActorDamageError<
            crate::intro2_common_dying::Intro2CommonDyingBlock,
            crate::intro2_common_dying::Intro2CommonDyingOwner,
        >,
    ),
    FlyingDamage(
        crate::live_actor_checked_damage::LiveActorDamageError<
            crate::native_flying_surface_contact::NativeFlyingSurfaceDeathBlock,
            crate::native_flying_surface_contact::NativeFlyingSurfaceDeathPublication,
        >,
    ),
    FactoryDamage(
        crate::live_actor_checked_damage::LiveActorDamageError<
            crate::intro2_type66::death::Intro2Type66DeathBlock,
            crate::intro2_type66::Intro2Type66Owner,
        >,
    ),
    /// Lethal ordinary Type97 active-pair Class49 delivery
    /// (`FUN_00410C10` -> BAF0 -> radial -> ring). Nonlethal checked damage
    /// completes through the same 15040 path without consulting the Playing
    /// context; only the lethal death closure requires it.
    TurretPairDamage(
        crate::live_actor_checked_damage::LiveActorDamageError<
            crate::class49_terminal::Class49TerminalBlock,
            (),
        >,
    ),
    /// Lethal Type97 pair without the explicit Playing player-hull context.
    /// The checked health prefix is already committed; this preserves the
    /// cinematic fail-closed boundary instead of inventing a hull.
    MissingPlayingContext {
        entity_id: u32,
    },
    WeaponTerminal(crate::class49_terminal::Class49TerminalBlock),
    NativeWeaponPairDamage(
        crate::live_actor_checked_damage::LiveActorDamageError<
            crate::entity::DynamicRadialLiveBlockReason,
            crate::entity::DynamicRadialDeathPublication,
        >,
    ),
    WeaponPlayerDamage(crate::entity::PlayerCheckedDamageBlock),
    UnsupportedBehavior {
        entity_id: u32,
        style: u32,
    },
    HiveImpact(crate::hive_impact::HiveImpactBlock),
    /// Authored Type3/27 Rolling Boulder currently lacks native construction,
    /// completed motion, component and terminal custody. A Hive latch alone
    /// cannot authorize that actor's remaining 11AD0 physical/damage suffix.
    UnsupportedHiveImpactCounterpart {
        entity_id: u32,
        entity_type: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCaptorPairBehaviorResult {
    Null,
    CaptureAcceptedA300,
    PowerUpUnsupportedRecipientA300,
}

pub use crate::native_actor_descriptor_contact::NativeDescriptorContactOutcome as NativeCaptorPairComponentResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCaptorPairStage {
    Sound(u16),
    Behavior {
        owner: u32,
        style: u32,
        result: NativeCaptorPairBehaviorResult,
    },
    /// 4BE328 -> 4C5070[0] -> 42E8E0 is a RET, with no sound or RNG effect.
    DispatchCaptureAccepted,
    /// 4BEAA8 shares the same single-RET vtable entry as capture's A300.
    DispatchPowerUpUnsupportedRecipient,
    Component {
        owner: u32,
        slot: ActorTaskSlot,
        result: NativeCaptorPairComponentResult,
    },
    Physical {
        capped_damage_raw: i32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeCaptorPairVisit {
    pub candidate_id: u32,
    pub contact: ActivePairContact,
    pub stages: Vec<NativeCaptorPairStage>,
    pub physical_suppressed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeCaptorPairOutcome {
    Ineligible,
    Resolved {
        visits: Vec<NativeCaptorPairVisit>,
    },
    Blocked {
        reason: NativeCaptorPairBlock,
        committed_prefix: bool,
        visits: Vec<NativeCaptorPairVisit>,
    },
}

pub fn resolve_native_captor_active_contacts(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    feedback: CaptureFeedbackPolicy,
) -> NativeCaptorPairOutcome {
    resolve_native_captor_active_contacts_with_playing(frame, id, feedback, None)
}

/// Playing active-pair pass with the explicit player/controller context required
/// by lethal ordinary Type97 Class49 delivery. Cinematic callers keep the
/// `None` boundary through the wrapper above; campaign reconstruction never
/// supplies an ordinary task admission through this path.
pub fn resolve_native_captor_active_contacts_with_playing(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    feedback: CaptureFeedbackPolicy,
    playing_player: Option<PlayingPlayerContact<'_>>,
) -> NativeCaptorPairOutcome {
    let subject_entry = frame
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .map(|entity| active_pair_body_from_entity(entity, frame.resources));
    resolve_native_actor_active_contacts_at_entry(
        frame,
        NativeActorPairRequest {
            id,
            feedback,
            playing_player,
            subject_entry: subject_entry.as_ref(),
        },
    )
}

/// The caller snapshots11AD0's subject admission before its static prefix.
/// Only eligibility, domain, model and+70 remain latched;12530 and callbacks
/// still read the current body, relations, basis and task slots.
pub(crate) struct NativeActorPairRequest<'a> {
    pub id: u32,
    pub feedback: CaptureFeedbackPolicy,
    pub playing_player: Option<PlayingPlayerContact<'a>>,
    pub subject_entry: Option<&'a ActivePairBody>,
}

pub(crate) fn resolve_native_actor_active_contacts_at_entry(
    frame: &mut Intro2ContactFrame<'_>,
    request: NativeActorPairRequest<'_>,
) -> NativeCaptorPairOutcome {
    let NativeActorPairRequest {
        id,
        feedback,
        playing_player,
        subject_entry,
    } = request;
    let mut committed = false;
    let mut visits = Vec::new();
    let mut participants = Vec::new();
    match resolve(
        frame,
        id,
        subject_entry,
        feedback,
        playing_player,
        &mut committed,
        &mut visits,
        &mut participants,
    ) {
        Ok(false) => NativeCaptorPairOutcome::Ineligible,
        Ok(true) => NativeCaptorPairOutcome::Resolved { visits },
        Err(reason) => {
            if committed {
                for participant in participants {
                    // The exact retained owner, including Type9's linear
                    // continuations, survives a late failure on either body.
                    if !frame
                        .actor_tasks
                        .park_native_contact_prefix(frame.entities, participant)
                    {
                        frame
                            .actor_tasks
                            .park_intro2_flyer_contact_prefix(participant);
                    }
                }
            }
            NativeCaptorPairOutcome::Blocked {
                reason,
                committed_prefix: committed,
                visits,
            }
        }
    }
}

pub(crate) fn actor(manager: &EntityManager, id: u32) -> Result<&Entity, NativeCaptorPairBlock> {
    manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(NativeCaptorPairBlock::Runtime("pair allocation"))
}

fn bits(entity: &Entity, mask: u32) -> Result<u32, NativeCaptorPairBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(value) => Ok(value),
        RetailRuntimeValue::Unresolved => Err(NativeCaptorPairBlock::Runtime("pair state bits")),
    }
}

fn owns_captor_contact(entity: &Entity) -> bool {
    // Receipt presence selects this lane; the manager authentication below
    // must report a changed body rather than silently discarding its pairs.
    (entity.entity_type == 17 && entity.intro2_type17_runtime.is_some())
        || (entity.entity_type == 122 && entity.native_type122_runtime.is_some())
}

/// Independent Type58 side of an 11AD0 pair.
///
/// Bounded to native Type58 construction with manager-authenticated
/// allocation (ordinary worlds share this receipt through `intro2_type58`).
/// Unlike captors, Type58 owns no C910/J transport: its behavior callbacks
/// are the current descriptor hooks (02CA0 primary, null component slot for
/// TrashFurniture) and its A900 component slots. Admitting it as an
/// independent subject also requires the counterpart to carry valid
/// callbacks; unresolved families stay fail-closed at their existing
/// `UnresolvedBehavior`/`UnsupportedBehavior` boundaries and must not be
/// whitelisted here.
fn owns_independent_type58_contact(manager: &EntityManager, entity: &Entity) -> bool {
    entity.entity_type == 58
        && crate::intro2_type58::type58_manager_allocation_authenticates(manager, entity.id)
}

fn owns_native_weapon_contact(entity: &Entity) -> bool {
    entity.native_entity_weapon_runtime.is_some()
}

/// Native gunner side of the gunner/hive separation pair.
///
/// Bounded to native Type47 construction (ordinary worlds share this receipt
/// through `shared_type47`); the Intro2 replay cohort stays skipped because it
/// retains no native receipt. Guard/Wander, Chase/Aim and Class12 keep their
/// contact-time behavior policies; the visit below reads them live and fails
/// closed on anything unaudited.
fn owns_native_gunner_contact(manager: &EntityManager, entity: &Entity) -> bool {
    entity.entity_type == 47
        && entity.native_type47_construction.is_some()
        && crate::shared_type47::type47_manager_allocation_authenticates(manager, entity.id)
}

/// Bound live-hive side of the gunner/hive separation pair.
///
/// Retail 11AD0 separates every live pair, but this lane can only prove the
/// bound hive: authored radial emitter, fixed body, and the installed
/// Alien-Hive pair policy (null against capability 8, which every gunner
/// carries). Dying, unbound, or re-styled hives stay skipped; the visit below
/// re-checks the live policy before any write.
fn owns_bound_hive_contact(entity: &Entity) -> bool {
    if entity.entity_type != 67 || entity.authored_radial_emitter.is_none() {
        return false;
    }
    if entity
        .collision
        .state_flags_at_0x08
        .masked(PAIR_COLLISION_FIXED_STATE_BIT)
        != RetailRuntimeValue::Known(PAIR_COLLISION_FIXED_STATE_BIT)
    {
        return false;
    }
    matches!(
        entity.current_behavior_context,
        RetailRuntimeValue::Known(Some(context))
            if context.active_style().pair_contact_callback_policy()
                == PairContactCallbackPolicy::Hive
    )
}

/// Admit a native-gunner versus bound-hive pair in either intrusive direction.
///
/// Authored Level-1 gunner spawn 11 rests ~158 raw units outside the hive
/// narrow solid with a +-1024 Guard retarget, so retail's mass-weighted
/// separation is the only thing keeping wanderers out of the mesh. Without
/// this lane a wanderer that enters the fixed hive body is never pushed out
/// and can no longer be hit, blocking the seven-hostile victory gate.
fn is_native_gunner_hive_pair(manager: &EntityManager, first: &Entity, second: &Entity) -> bool {
    (owns_native_gunner_contact(manager, first) && owns_bound_hive_contact(second))
        || (owns_native_gunner_contact(manager, second) && owns_bound_hive_contact(first))
}

fn hive_impact_counterpart(first: &Entity, second: &Entity) -> Option<u32> {
    use crate::hive_impact::{hive_pair_callback_branch, HivePairCallbackBranch};
    [(first, second), (second, first)]
        .into_iter()
        .find_map(|(hive, opposite)| {
            (owns_bound_hive_contact(hive)
                && hive_pair_callback_branch(opposite.capability_flags)
                    == HivePairCallbackBranch::Impact)
                .then_some(opposite.id)
        })
}

fn is_native_insect_factory_pair(manager: &EntityManager, first: &Entity, second: &Entity) -> bool {
    let is_factory =
        |entity: &Entity| crate::intro2_type66::intro2_type66_allocation_authenticates(entity);
    (is_factory(first)
        && crate::native_actor_descriptor_contact::native_insect_allocation_authenticates(
            manager, second.id,
        ))
        || (is_factory(second)
            && crate::native_actor_descriptor_contact::native_insect_allocation_authenticates(
                manager, first.id,
            ))
}

fn is_native_type9_peer_pair(first: &Entity, second: &Entity) -> bool {
    // Receipt presence chooses the lane; completed body custody below checks
    // each allocation and its actual living/carried/Class14 graph. A changed
    // receipt must report a contact block rather than silently lose a pair.
    [first, second].into_iter().all(|entity| {
        entity.entity_type == 9
            && (entity.ordinary_type9_native_receipt.is_some()
                || entity.intro2_type9_runtime.is_some())
    })
}

fn is_native_type40_person_pair(first: &Entity, second: &Entity) -> bool {
    // Type40 owns C910 with a true absent J; it is not a Type17/122 captor.
    // Select by its receipt, then authenticate both completed body owners in
    // resolve before geometry callbacks or physical mutation.
    [(first, second), (second, first)]
        .into_iter()
        .any(|(actor, person)| {
            actor.entity_type == 40
                && actor.native_type40_runtime.is_some()
                && person.capability_flags & 0xc00 != 0
        })
}

fn resolve(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    subject_entry: Option<&ActivePairBody>,
    feedback: CaptureFeedbackPolicy,
    mut playing_player: Option<PlayingPlayerContact<'_>>,
    committed: &mut bool,
    visits: &mut Vec<NativeCaptorPairVisit>,
    participants: &mut Vec<u32>,
) -> Result<bool, NativeCaptorPairBlock> {
    let subject = actor(frame.entities, id)?;
    if !subject.active {
        return Ok(false);
    }
    let subject_entry = subject_entry
        .filter(|entry| entry.id == id)
        .ok_or(NativeCaptorPairBlock::Runtime("subject entry admission"))?;
    let mut candidate_cursor = frame
        .entities
        .retail_live_successor_id(id)
        .map_err(|_| NativeCaptorPairBlock::Runtime("subject intrusive cursor"))?;
    let player = frame.entities.player().map(|entity| entity.id);
    // 11AD0 begins at *subject, the next node. Earlier nodes had their own pass.
    // This lane owns pairs containing a native captor, a native-gunner
    // versus bound-hive pair, native Type9 peers, or a native Type58 participant in either seat.
    // Player-as-subject visits native weapons here; its other pairs retain
    // the player adapter. Captor-as-subject still visits the player candidate.
    // Other unordered pairs remain with their existing subsystem. The caller visits every
    // subject in the same live order, so each unordered pair is visited
    // exactly once from its earlier participant's pass.
    while let Some(candidate_id) = candidate_cursor {
        //11AD0 caches this candidate's successor before its callbacks. A new
        //tail can become visible through a later candidate's current next,
        //but a birth during the old terminal candidate does not extend it.
        candidate_cursor = frame
            .entities
            .retail_live_successor_id(candidate_id)
            .map_err(|_| NativeCaptorPairBlock::Runtime("candidate intrusive cursor"))?;
        let subject = actor(frame.entities, id)?;
        let candidate = actor(frame.entities, candidate_id)?;
        let pair_weapon = [subject, candidate]
            .into_iter()
            .any(owns_native_weapon_contact);
        if player == Some(id) && !pair_weapon {
            continue;
        }
        let pair_captors = [subject, candidate]
            .into_iter()
            .filter(|entity| owns_captor_contact(entity))
            .map(|entity| entity.id)
            .collect::<Vec<_>>();
        let pair_type58 = [subject, candidate]
            .into_iter()
            .any(|entity| owns_independent_type58_contact(frame.entities, entity));
        if pair_captors.is_empty()
            && !is_native_gunner_hive_pair(frame.entities, subject, candidate)
            && !pair_type58
            && !pair_weapon
            && !is_native_insect_factory_pair(frame.entities, subject, candidate)
            && !is_native_type9_peer_pair(subject, candidate)
            && !is_native_type40_person_pair(subject, candidate)
            && hive_impact_counterpart(subject, candidate).is_none()
        {
            continue;
        }
        let Some(contact) = classify_oriented_active_pair_contact(
            crate::player_active_contact::OrientedActivePairContactRequest {
                subject,
                candidate,
                subject_entry,
                retail_tick: frame.retail_tick,
            },
            frame.resources,
        )
        .map_err(NativeCaptorPairBlock::Geometry)?
        else {
            continue;
        };
        if bits(subject, REMOTE_OWNED_STATE_BIT)? != 0
            || bits(candidate, REMOTE_OWNED_STATE_BIT)? != 0
        {
            return Err(NativeCaptorPairBlock::Runtime("local pair callbacks"));
        }
        if let Some(counterpart) = hive_impact_counterpart(subject, candidate) {
            let opposite = actor(frame.entities, counterpart)?;
            if !requires_body_custody(opposite) {
                return Err(NativeCaptorPairBlock::UnsupportedHiveImpactCounterpart {
                    entity_id: counterpart,
                    entity_type: opposite.entity_type,
                });
            }
        }
        for &captor in &pair_captors {
            if NativeCaptorProfile::authenticate(frame.entities, captor).is_err() {
                return Err(NativeCaptorPairBlock::Runtime("native pair allocation"));
            }
            if !frame
                .actor_tasks
                .prepare_native_actor_mutation(frame.entities, captor)
            {
                return Err(NativeCaptorPairBlock::Runtime("completed pair owner"));
            }
            if !participants.contains(&captor) {
                participants.push(captor);
            }
        }
        // A null task callback and zero collision damage still allow 12760
        // to move a body. Authenticate both native task owners before any
        // callback or physical write, including a parked Class12 counterpart.
        for owner in [id, candidate_id] {
            let entity = actor(frame.entities, owner)?;
            if requires_body_custody(entity)
                && !crate::native_actor_descriptor_contact::completed_contact_owner(
                    frame.entities,
                    frame.actor_tasks,
                    owner,
                )
            {
                return Err(NativeCaptorPairBlock::BodyCustody { entity_id: owner });
            }
            if requires_body_custody(entity) && !participants.contains(&owner) {
                participants.push(owner);
            }
        }
        visits.push(NativeCaptorPairVisit {
            candidate_id,
            contact,
            stages: Vec::new(),
            physical_suppressed: false,
        });
        let visit = visits.last_mut().unwrap();
        contact_sounds(frame, id, candidate_id, committed, &mut visit.stages)?;
        // D780 resolves the current behavior context for each directional call.
        for (owner, opposite) in [(id, candidate_id), (candidate_id, id)] {
            let style = style_address(actor(frame.entities, owner)?)?;
            let result = behavior(
                frame,
                owner,
                opposite,
                feedback,
                playing_player.as_mut().map(PlayingPlayerContact::reborrow),
                committed,
            )?;
            visit.stages.push(NativeCaptorPairStage::Behavior {
                owner,
                style,
                result,
            });
            match result {
                NativeCaptorPairBehaviorResult::CaptureAcceptedA300 => {
                    visit
                        .stages
                        .push(NativeCaptorPairStage::DispatchCaptureAccepted);
                    visit.physical_suppressed = true;
                }
                NativeCaptorPairBehaviorResult::PowerUpUnsupportedRecipientA300 => {
                    visit
                        .stages
                        .push(NativeCaptorPairStage::DispatchPowerUpUnsupportedRecipient);
                    visit.physical_suppressed = true;
                }
                NativeCaptorPairBehaviorResult::Null => {}
            }
        }
        // A900 re-reads each slot after every prior callback. ADB0 may have
        // replaced the victim by None, and C6B0 installed a new captor Primary.
        for (owner, opposite) in [(id, candidate_id), (candidate_id, id)] {
            for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
                let result = component(frame, owner, opposite, slot, committed)?;
                visit.stages.push(NativeCaptorPairStage::Component {
                    owner,
                    slot,
                    result,
                });
            }
        }
        if !visit.physical_suppressed {
            physical(
                frame,
                id,
                candidate_id,
                contact,
                playing_player.as_mut().map(PlayingPlayerContact::reborrow),
                committed,
                &mut visit.stages,
            )?;
        }
        for captor in pair_captors {
            if !matches!(
                actor(frame.entities, captor)?.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::CommonDying(_))
            ) {
                adopt_captor(frame, captor)?;
            }
        }
    }
    Ok(true)
}

pub(crate) fn style_address(entity: &Entity) -> Result<u32, NativeCaptorPairBlock> {
    match entity.current_behavior_context {
        RetailRuntimeValue::Known(Some(context)) => Ok(context.active_style().style_address()),
        RetailRuntimeValue::Known(None) => Ok(0),
        _ => Err(NativeCaptorPairBlock::UnresolvedBehavior {
            entity_id: entity.id,
            entity_type: entity.entity_type,
        }),
    }
}

fn contact_sounds(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    candidate: u32,
    committed: &mut bool,
    stages: &mut Vec<NativeCaptorPairStage>,
) -> Result<(), NativeCaptorPairBlock> {
    let entity = actor(frame.entities, id)?;
    let row = frame
        .resources
        .global_entity_type(entity.entity_type as usize)
        .ok_or(NativeCaptorPairBlock::Runtime("pair sound metadata"))?;
    let position = entity.position_raw();
    let target_flags = actor(frame.entities, candidate)?.capability_flags;
    for (offset, mask) in [(0x8e, 4), (0x8c, 8)] {
        let sound = u16::from_le_bytes([row.raw_header[offset], row.raw_header[offset + 1]]);
        if sound != 0 && target_flags & mask != 0 {
            frame
                .world_fx
                .queue_fixed_positional_sound_raw(sound, position);
            *committed = true;
            stages.push(NativeCaptorPairStage::Sound(sound));
        }
    }
    Ok(())
}

fn behavior(
    frame: &mut Intro2ContactFrame<'_>,
    owner: u32,
    opposite: u32,
    feedback: CaptureFeedbackPolicy,
    playing_player: Option<PlayingPlayerContact<'_>>,
    committed: &mut bool,
) -> Result<NativeCaptorPairBehaviorResult, NativeCaptorPairBlock> {
    let entity = actor(frame.entities, owner)?;
    if frame
        .entities
        .player()
        .is_some_and(|player| player.id == owner)
        && owns_native_weapon_contact(actor(frame.entities, opposite)?)
    {
        //447D70: the actual local controller bound to this player satisfies
        //43AF0. Capability40 excludes the c00 consumable branch; only the
        //existing no-haptics presentation boundary can remain. It neither
        //changes body/task state nor returns an A300 physics suppression.
        if playing_player.is_none()
            || entity.capability_flags & 1 == 0
            || actor(frame.entities, opposite)?.capability_flags != 0x40
        {
            return Err(NativeCaptorPairBlock::Runtime(
                "local player weapon pair controller",
            ));
        }
        return Ok(NativeCaptorPairBehaviorResult::Null);
    }
    let context = match entity.current_behavior_context {
        RetailRuntimeValue::Known(Some(context)) => context,
        RetailRuntimeValue::Known(None) => return Ok(NativeCaptorPairBehaviorResult::Null),
        RetailRuntimeValue::Unresolved => {
            return Err(NativeCaptorPairBlock::UnresolvedBehavior {
                entity_id: entity.id,
                entity_type: entity.entity_type,
            })
        }
    };
    let style = context.active_style().style_address();
    match context.active_style().pair_contact_callback_policy() {
        PairContactCallbackPolicy::None => Ok(NativeCaptorPairBehaviorResult::Null),
        PairContactCallbackPolicy::UnknownAddress(0x0040_cef0)
            if owns_native_weapon_contact(entity) =>
        {
            let expected_style = if entity.entity_type == 42 {
                0x4c81a0
            } else {
                0x4c8350
            };
            if style != expected_style
                    || !crate::native_actor_descriptor_contact::native_weapon_contact_owner_authenticates(
                        frame.entities, frame.actor_tasks, owner)
                {
                    return Err(NativeCaptorPairBlock::BodyCustody { entity_id: owner });
                }
            *committed = true;
            run_weapon_pair_terminal(frame, owner, playing_player)?;
            //CEF0 always discards10C10's result and returns null. The
            //opposing D780 call, both current A900 chains and12760 remain.
            Ok(NativeCaptorPairBehaviorResult::Null)
        }
        // C910 rejects a non-person before reading any captor state. Other
        // actor profiles using Class9 share this no-op against the captor.
        PairContactCallbackPolicy::CapturePeople
            if actor(frame.entities, opposite)?.capability_flags & 0xc00 == 0
                || bits(actor(frame.entities, opposite)?, DYING_STATE_BIT)? != 0
                || actor(frame.entities, opposite)?
                    .collision
                    .state_flags_at_0x08
                    .masked(u32::MAX)
                    == RetailRuntimeValue::Known(0) =>
        {
            Ok(NativeCaptorPairBehaviorResult::Null)
        }
        PairContactCallbackPolicy::CapturePeople if entity.entity_type == 40 => {
            if !crate::native_type40::manager_allocation_authenticates(frame.entities, owner)
                || style != 0x4c8038
                || entity.sub_j_attachment_runtime != RetailRuntimeValue::Known(None)
                || crate::native_type40::Type40Owner::adopt(frame.entities, owner).is_err()
            {
                return Err(NativeCaptorPairBlock::BodyCustody { entity_id: owner });
            }
            // 418410 returns full for the genuine null J. C910 invokes the
            // contact person's10C10 and discards it at40CAC5: no attachment,
            // transport task or A300 return. Opposing behavior/components and
            // the12760 physical suffix still execute after this null result.
            let result = capture::kill_capture_contact(
                frame.entities,
                opposite,
                &mut capture::CaptureContext {
                    tasks: frame.actor_tasks,
                    world_fx: frame.world_fx,
                    notifications: frame.notifications,
                    retail_tick: frame.retail_tick,
                    result_screen:
                        crate::main_base_type9_abort::MainBaseType9ResultScreenState::NotShown,
                    hive_dying: Default::default(),
                },
            );
            if let Err(error) = result {
                *committed |= error.committed_prefix;
                return Err(NativeCaptorPairBlock::Capture(error));
            }
            *committed = true;
            Ok(NativeCaptorPairBehaviorResult::Null)
        }
        PairContactCallbackPolicy::CapturePeople if matches!(entity.entity_type, 17 | 122) => {
            let RetailRuntimeValue::Known(Some(rows)) = &entity.sub_j_attachment_runtime else {
                return Err(NativeCaptorPairBlock::Runtime("capture capacity"));
            };
            let full = rows.len() >= rows.capacity();
            let result = {
                let mut context = capture::CaptureContext {
                    tasks: frame.actor_tasks,
                    world_fx: frame.world_fx,
                    notifications: frame.notifications,
                    retail_tick: frame.retail_tick,
                    result_screen:
                        crate::main_base_type9_abort::MainBaseType9ResultScreenState::NotShown,
                    hive_dying: Default::default(),
                };
                if full {
                    capture::kill_capture_contact(frame.entities, opposite, &mut context)
                } else {
                    capture::attach_capture_child(frame.entities, owner, opposite, &mut context)
                }
            };
            if let Err(error) = result {
                *committed |= error.committed_prefix;
                return Err(NativeCaptorPairBlock::Capture(error));
            }
            *committed = true;
            if full {
                return Ok(NativeCaptorPairBehaviorResult::Null);
            }
            if feedback == CaptureFeedbackPolicy::Gameplay
                && bits(actor(frame.entities, owner)?, 0x0400_0000)? != 0
            {
                frame
                    .notifications
                    .queue_spider_capture(frame.retail_tick as i32);
            }
            finish_capture_selection(frame, owner)?;
            Ok(NativeCaptorPairBehaviorResult::CaptureAcceptedA300)
        }
        PairContactCallbackPolicy::UnknownAddress(0x0040_cd50) if entity.entity_type == 30 => {
            if style != 0x004c7930
                || entity.capability_flags != 8
                || !crate::native_type30::manager_allocation_authenticates(frame.entities, owner)
                || crate::native_type30::Type30Owner::adopt(frame.entities, owner).is_err()
            {
                return Err(NativeCaptorPairBlock::BodyCustody { entity_id: owner });
            }
            //CD50 forwards selector1 toCD70. The callback reads its owner's
            //capabilities, not the opposite actor's: native Type30 has neither
            //C00 nor1000, so both branches reachCE62's zero return. No task,
            //component, body or process-RNG mutation precedes that return.
            Ok(NativeCaptorPairBehaviorResult::Null)
        }
        PairContactCallbackPolicy::UnknownAddress(0x0040_d0b0)
            if matches!(entity.entity_type, 17 | 122) =>
        {
            let result = capture::execute_capture_delivery(
                frame.entities,
                owner,
                opposite,
                &mut capture::CaptureContext {
                    tasks: frame.actor_tasks,
                    world_fx: frame.world_fx,
                    notifications: frame.notifications,
                    retail_tick: frame.retail_tick,
                    result_screen:
                        crate::main_base_type9_abort::MainBaseType9ResultScreenState::NotShown,
                    hive_dying: Default::default(),
                },
            );
            match result {
                Ok(result) => {
                    *committed = true; // 235D0 is before D0B0's target lookup/filter.
                    if let Some(owner) = result.common_dying_owner {
                        frame.actor_tasks.register_intro2_common_dying(owner);
                    } else {
                        adopt_captor(frame, owner)?;
                    }
                    Ok(NativeCaptorPairBehaviorResult::Null)
                }
                Err(error) => {
                    *committed |= error.committed_prefix;
                    Err(NativeCaptorPairBlock::Capture(error))
                }
            }
        }
        // 259F0 reads the opposing capability words before any hive-owned state.
        // Its consuming operation33/deferred-destroy branch stays explicit;
        // it must never fall through to 1CE10 when both masks match.
        PairContactCallbackPolicy::Hive => {
            use crate::hive_impact::{hive_pair_callback_branch, HivePairCallbackBranch};
            let counterpart = actor(frame.entities, opposite)?;
            match hive_pair_callback_branch(counterpart.capability_flags) {
                HivePairCallbackBranch::NoEffect => Ok(NativeCaptorPairBehaviorResult::Null),
                HivePairCallbackBranch::Consume => {
                    Err(NativeCaptorPairBlock::UnsupportedBehavior {
                        entity_id: owner,
                        style,
                    })
                }
                HivePairCallbackBranch::Impact => {
                    if !requires_body_custody(counterpart) {
                        return Err(NativeCaptorPairBlock::UnsupportedHiveImpactCounterpart {
                            entity_id: opposite,
                            entity_type: counterpart.entity_type,
                        });
                    }
                    let outcome = frame
                        .entities
                        .apply_live_hive_pair_impact(owner, opposite)
                        .map_err(NativeCaptorPairBlock::HiveImpact)?;
                    *committed |= outcome.is_some_and(|impact| impact.forced_death);
                    Ok(NativeCaptorPairBehaviorResult::Null)
                }
            }
        }
        // 425850/4258A0 reject these capability words before reading the
        // destination's resource, relation or factory task state.
        PairContactCallbackPolicy::LifterDelivery
            if actor(frame.entities, opposite)?.capability_flags & 0x400 == 0 =>
        {
            Ok(NativeCaptorPairBehaviorResult::Null)
        }
        PairContactCallbackPolicy::MainBaseConversion
            if actor(frame.entities, opposite)?.capability_flags & 0x800 == 0 =>
        {
            Ok(NativeCaptorPairBehaviorResult::Null)
        }
        PairContactCallbackPolicy::PowerUp
            if actor(frame.entities, opposite)?.capability_flags & 1 == 0 =>
        {
            Ok(NativeCaptorPairBehaviorResult::PowerUpUnsupportedRecipientA300)
        }
        _ => Err(NativeCaptorPairBlock::UnsupportedBehavior {
            entity_id: owner,
            style,
        }),
    }
}

fn finish_capture_selection(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
) -> Result<(), NativeCaptorPairBlock> {
    let entity = frame
        .entities
        .entity_mut(id)
        .ok_or(NativeCaptorPairBlock::Runtime("capture allocation"))?;
    let RetailRuntimeValue::Known(mut axis) = entity.actor_common_axis_descriptor else {
        return Err(NativeCaptorPairBlock::Runtime("capture destination axis"));
    };
    axis.raw_word_at_0x04 = 0x10;
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(axis);
    let mut owner = fresh_type17_candidate_ref(entity);
    owner.attached_entity_handle = entity.collision.recent_relation_id_at_0x60;
    let candidates = frame
        .entities
        .retail_live_order_ids()
        .map(|candidate| {
            fresh_type17_candidate_ref(
                actor(frame.entities, candidate).expect("live order allocation"),
            )
        })
        .collect::<Vec<_>>();
    let selected = select_guard_location_candidate(GuardLocationCandidateRequest {
        owner,
        candidates_in_intrusive_order: &candidates,
        search_context: GuardLocationSearchContext::new(
            GuardLocationCandidateRange::from_raw(axis.strict_axis_limit_raw),
            GuardLocationCandidateFilter::from_raw(axis.raw_word_at_0x04),
        ),
    })
    .map_err(|_| NativeCaptorPairBlock::Runtime("capture destination query"))?;
    let (variant, target) = match selected {
        GuardLocationCandidateSelection::Selected(destination) => (
            2,
            carry_tasks::CaptureTargetWrite::Set(Some(destination.id)),
        ),
        GuardLocationCandidateSelection::TaggedNoCandidate { .. } => {
            (3, carry_tasks::CaptureTargetWrite::Preserve)
        }
    };
    carry_tasks::publish_style(frame.entities, id, variant, target, frame.world_fx)
        .map_err(NativeCaptorPairBlock::CaptureTask)?;
    // The behavior callback has completed its replacement before the same
    // pair's A900 walk reaches the new task. Transfer custody at this boundary.
    adopt_captor(frame, id)?;
    Ok(())
}

fn adopt_captor(frame: &mut Intro2ContactFrame<'_>, id: u32) -> Result<(), NativeCaptorPairBlock> {
    match NativeCaptorProfile::authenticate(frame.entities, id)
        .map_err(NativeCaptorPairBlock::Capture)?
    {
        NativeCaptorProfile::Type17 => {
            let owner = Intro2Type17Owner::adopt(frame.entities, id)
                .map_err(NativeCaptorPairBlock::Task)?;
            frame.actor_tasks.register_intro2_type17(owner);
        }
        NativeCaptorProfile::Type122 => {
            let owner = crate::native_type122::Type122Owner::adopt(frame.entities, id)
                .map_err(NativeCaptorPairBlock::GroundTask)?;
            frame.actor_tasks.register_type122(owner);
        }
    }
    Ok(())
}

fn component(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    opposite: u32,
    slot: ActorTaskSlot,
    committed: &mut bool,
) -> Result<NativeCaptorPairComponentResult, NativeCaptorPairBlock> {
    let result = crate::native_actor_descriptor_contact::resolve_native_actor_descriptor_contact(
        frame, id, opposite, slot,
    )
    .map_err(|error| {
        *committed |= error.committed_prefix;
        NativeCaptorPairBlock::Component(error)
    })?;
    *committed |= matches!(result, NativeCaptorPairComponentResult::Applied { .. });
    Ok(result)
}
fn physical(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    candidate: u32,
    contact: ActivePairContact,
    mut playing_player: Option<PlayingPlayerContact<'_>>,
    committed: &mut bool,
    stages: &mut Vec<NativeCaptorPairStage>,
) -> Result<(), NativeCaptorPairBlock> {
    let weapon_pair = [id, candidate]
        .into_iter()
        .any(|id| actor(frame.entities, id).is_ok_and(owns_native_weapon_contact));
    let plan = plan_active_pair_response_and_damage_cap(
        active_pair_body_from_entity(actor(frame.entities, id)?, frame.resources),
        active_pair_body_from_entity(actor(frame.entities, candidate)?, frame.resources),
        contact,
    )
    .map_err(NativeCaptorPairBlock::Geometry)?;
    for body in [&plan.subject, &plan.candidate] {
        let current = actor(frame.entities, body.id)?;
        if (current.position_raw() != body.position_raw
            || current.velocity_raw() != body.velocity_raw)
            && !requires_body_custody(current)
            && !(weapon_pair
                && frame
                    .entities
                    .player()
                    .is_some_and(|player| player.id == body.id)
                && playing_player.is_some())
        {
            return Err(NativeCaptorPairBlock::BodyCustody { entity_id: body.id });
        }
    }
    for body in [&plan.subject, &plan.candidate] {
        frame
            .entities
            .entity_mut(body.id)
            .unwrap()
            .set_motion_raw(body.position_raw, body.velocity_raw);
    }
    *committed = true;
    stages.push(NativeCaptorPairStage::Physical {
        capped_damage_raw: plan.capped_pair_damage_raw,
    });
    if plan.capped_pair_damage_raw != 0 {
        for (target, source) in [(id, candidate), (candidate, id)] {
            // 11AD0 tests the SOURCE sign immediately before each directional
            // 15040 call; the first damage callback can change the next gate.
            if bits(actor(frame.entities, source)?, REMOTE_OWNED_STATE_BIT)? != 0 {
                continue;
            }
            let source_type = actor(frame.entities, source)?.entity_type;
            let delivery = crate::damage::DamageDeliveryRecord {
                packet: crate::damage::DamagePacket::collision(plan.capped_pair_damage_raw),
                source_entity_type_raw: source_type,
                owner_handle: source,
            };
            if weapon_pair {
                apply_native_pair_checked_damage(
                    frame,
                    target,
                    delivery,
                    playing_player.as_mut().map(PlayingPlayerContact::reborrow),
                    committed,
                )?;
            } else {
                apply_pair_checked_damage(
                    frame,
                    target,
                    delivery,
                    playing_player.as_mut().map(PlayingPlayerContact::reborrow),
                    committed,
                )?;
            }
        }
    }
    Ok(())
}

fn run_weapon_pair_terminal(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    playing_player: Option<PlayingPlayerContact<'_>>,
) -> Result<bool, NativeCaptorPairBlock> {
    use crate::class49_terminal::{
        run_class49_standard_death, Class49TerminalFrame, Class49WorldContext,
    };
    let world = match playing_player {
        Some(player) => Class49WorldContext::Playing {
            scheduler: frame.actor_tasks,
            player_hull: player.hull,
            extra_lives: player.extra_lives,
            active_terminal_calls: Vec::new(),
        },
        None => Class49WorldContext::Cinematic {
            actor_tasks: frame.actor_tasks,
            active_terminal_calls: Vec::new(),
        },
    };
    run_class49_standard_death(
        Class49TerminalFrame {
            entities: frame.entities,
            resources: frame.resources,
            world_fx: frame.world_fx,
            static_damage: frame.static_damage,
            notifications: frame.notifications,
            retail_tick: frame.retail_tick,
            world,
        },
        id,
    )
    .map(|result| result.returned_nonzero)
    .map_err(NativeCaptorPairBlock::WeaponTerminal)
}

// 12760's directional 15040 delivery has no particle/impact prefix. Native
// terminals receive the actual enclosing Playing or Cinematic world custody.
fn apply_native_pair_checked_damage(
    frame: &mut Intro2ContactFrame<'_>,
    target: u32,
    delivery: crate::damage::DamageDeliveryRecord,
    playing_player: Option<PlayingPlayerContact<'_>>,
    committed: &mut bool,
) -> Result<(), NativeCaptorPairBlock> {
    if frame
        .entities
        .player()
        .is_some_and(|player| player.id == target)
    {
        let player = playing_player
            .ok_or(NativeCaptorPairBlock::MissingPlayingContext { entity_id: target })?;
        let RetailRuntimeValue::Known(extra_lives) = player.extra_lives else {
            return Err(NativeCaptorPairBlock::Runtime("weapon pair player lives"));
        };
        return match frame.entities.apply_player_checked_damage(
            crate::entity::PlayerCheckedDamageRequest {
                entry: crate::entity::PlayerDamageEntry::Checked,
                target_id: target,
                delivery,
                ratio_numerator: 0,
                ratio_denominator: 0,
            },
            crate::entity::PlayerCheckedDamageFrame {
                hull: player.hull,
                resources: frame.resources,
                world_fx: frame.world_fx,
                retail_tick: frame.retail_tick,
                notifications: frame.notifications,
                extra_lives,
            },
        ) {
            crate::entity::PlayerCheckedDamageOutcome::Applied { .. }
            | crate::entity::PlayerCheckedDamageOutcome::FilteredOut
            | crate::entity::PlayerCheckedDamageOutcome::Ineligible => Ok(()),
            crate::entity::PlayerCheckedDamageOutcome::Blocked(reason) => {
                Err(NativeCaptorPairBlock::WeaponPlayerDamage(reason))
            }
            crate::entity::PlayerCheckedDamageOutcome::NotPlayer => {
                Err(NativeCaptorPairBlock::Runtime("weapon pair player target"))
            }
        };
    }
    let request = crate::live_actor_checked_damage::LiveActorDamageRequest {
        entity_id: target,
        delivery,
        entry: crate::live_actor_checked_damage::LiveActorDamageEntry::Checked,
        ratio_numerator: 0,
        ratio_denominator: 0,
        feedback: None,
    };
    let result = match playing_player {
        Some(player) => crate::native_checked_damage::apply_native_actor_checked_damage(
            frame.entities, request, crate::native_checked_damage::NativeCheckedDamageContext {
                world_fx: frame.world_fx, retail_tick: frame.retail_tick, notifications: frame.notifications,
                callbacks: &mut crate::specialized_actor_task_production::playing_radial::PlayingNativeCallbacks {
                    scheduler: frame.actor_tasks, resources: frame.resources, static_damage: frame.static_damage,
                    player_hull: player.hull, extra_lives: player.extra_lives, active_terminal_calls: &[],
                },
            }),
        None => crate::native_checked_damage::apply_native_actor_checked_damage(
            frame.entities, request, crate::native_checked_damage::NativeCheckedDamageContext {
                world_fx: frame.world_fx, retail_tick: frame.retail_tick, notifications: frame.notifications,
                callbacks: &mut crate::intro2_radial::Intro2RadialCallbacks {
                    actor_tasks: frame.actor_tasks, resources: frame.resources,
                    static_damage: frame.static_damage, active_terminal_calls: &[],
                },
            }),
    };
    result.map(|_| ()).map_err(|error| {
        *committed |= error.committed_prefix;
        NativeCaptorPairBlock::NativeWeaponPairDamage(error)
    })
}

fn requires_body_custody(entity: &Entity) -> bool {
    entity.native_entity_weapon_runtime.is_some()
        || entity.intro2_type66_runtime.is_some()
        || entity.intro2_type26_sub_d_runtime.is_some()
        || entity.intro2_type13_common_mover_runtime.is_some()
        || entity.intro2_flyer_frame_owner.is_some()
        || entity.intro2_type10_runtime.is_some()
        || entity.intro2_type57_runtime.is_some()
        || entity.intro2_type17_runtime.is_some()
        || entity.intro2_gun_turret_runtime.is_some()
        || crate::intro2_type16::intro2_type16_allocation_authenticates(entity)
        || entity.shared_fish_runtime.is_some()
        || entity.cleansing_vehicle_runtime.is_some()
        || entity.intro2_type8_runtime.is_some()
        || entity.native_type86_runtime.is_some()
        || entity.native_type123_runtime.is_some()
        || entity.ordinary_type9_native_receipt.is_some()
        || crate::intro2_type9::intro2_type9_allocation_authenticates(entity)
        || entity.native_type47_construction.is_some()
        || crate::intro2_type47_live::intro2_type47_cohort_runtime_authenticates(entity)
        || crate::intro2_type53::intro2_type53_allocation_authenticates(entity)
        || crate::intro2_type58::intro2_type58_allocation_authenticates(entity)
        || crate::native_type122::type122_allocation_authenticates(entity)
        || crate::native_type30::allocation_authenticates(entity)
        || crate::native_type40::allocation_authenticates(entity)
        || crate::native_type56::allocation_authenticates(entity)
        || crate::intro2_type94::intro2_type94_allocation_authenticates(entity)
        || matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::CommonDying(_))
        )
}

fn apply_pair_checked_damage(
    frame: &mut Intro2ContactFrame<'_>,
    target: u32,
    delivery: crate::damage::DamageDeliveryRecord,
    playing_player: Option<PlayingPlayerContact<'_>>,
    committed: &mut bool,
) -> Result<(), NativeCaptorPairBlock> {
    let kind = actor(frame.entities, target)?.entity_type;
    if (kind == 40
        && crate::native_type40::manager_allocation_authenticates(frame.entities, target))
        || (kind == 56
            && crate::native_type56::manager_allocation_authenticates(frame.entities, target))
    {
        return apply_native_pair_checked_damage(
            frame,
            target,
            delivery,
            playing_player,
            committed,
        );
    }
    if kind == 66
        && crate::intro2_type66::intro2_type66_allocation_authenticates(actor(
            frame.entities,
            target,
        )?)
    {
        let result = crate::live_actor_checked_damage::apply_live_actor_checked_damage(
            frame.entities,
            frame.world_fx,
            crate::live_actor_checked_damage::LiveActorDamageRequest {
                ratio_numerator: 0,
                ratio_denominator: 0,
                entity_id: target,
                delivery,
                entry: crate::live_actor_checked_damage::LiveActorDamageEntry::Checked,
                feedback: None,
            },
            |manager, world_fx, _| {
                crate::intro2_type66::death::publish_intro2_type66_standard_death(
                    manager, target, world_fx,
                )
                .map(|death| {
                    crate::live_actor_checked_damage::LiveActorDeathResult {
                        returned_nonzero: death.returned_nonzero,
                        publication: death.owner,
                    }
                })
            },
        );
        match result {
            Ok(result) => {
                if let Some(owner) = result.death_publication {
                    frame.actor_tasks.register_intro2_type66(owner);
                }
            }
            Err(error) => {
                *committed |= error.committed_prefix;
                if let Some(owner) = error.death_publication {
                    frame.actor_tasks.register_intro2_type66(owner);
                }
                return Err(NativeCaptorPairBlock::FactoryDamage(error));
            }
        }
        return Ok(());
    }
    if matches!(kind, 13 | 15 | 87 | 10 | 57) {
        let result = crate::live_actor_checked_damage::apply_live_actor_checked_damage(
            frame.entities,
            frame.world_fx,
            crate::live_actor_checked_damage::LiveActorDamageRequest {
                ratio_numerator: 0,
                ratio_denominator: 0,
                entity_id: target,
                delivery,
                entry: crate::live_actor_checked_damage::LiveActorDamageEntry::Checked,
                feedback: None,
            },
            |manager, world_fx, _| {
                crate::native_flying_surface_contact::run_native_flying_standard_death(
                    crate::class49_terminal::Class49TerminalFrame {
                        entities: manager,
                        resources: frame.resources,
                        world_fx,
                        static_damage: frame.static_damage,
                        notifications: frame.notifications,
                        retail_tick: frame.retail_tick,
                        world: crate::class49_terminal::Class49WorldContext::Cinematic {
                            actor_tasks: frame.actor_tasks,
                            active_terminal_calls: Vec::new(),
                        },
                    },
                    target,
                )
            },
        );
        match result {
            Ok(result) => {
                if let Some(owner) = result.death_publication {
                    crate::native_flying_surface_contact::register_native_flying_death(
                        frame.actor_tasks,
                        owner,
                    );
                }
            }
            Err(error) => {
                *committed |= error.committed_prefix;
                if let Some(owner) = error.death_publication {
                    crate::native_flying_surface_contact::register_native_flying_death(
                        frame.actor_tasks,
                        owner,
                    );
                }
                return Err(NativeCaptorPairBlock::FlyingDamage(error));
            }
        }
        return Ok(());
    }
    if kind == 67 {
        let resources = &*frame.resources;
        let sea_level_raw = resources
            .terrain()
            .and_then(|terrain| terrain.water_enabled().then(|| terrain.sea_level_raw()));
        crate::live_actor_checked_damage::apply_live_actor_checked_damage(
            frame.entities,
            frame.world_fx,
            crate::live_actor_checked_damage::LiveActorDamageRequest {
                ratio_numerator: 0,
                ratio_denominator: 0,
                entity_id: target,
                delivery,
                entry: crate::live_actor_checked_damage::LiveActorDamageEntry::Checked,
                feedback: Some(crate::live_actor_checked_damage::LiveActorDamageFeedback {
                    notifications: frame.notifications,
                    retail_tick: frame.retail_tick,
                }),
            },
            |manager, world_fx, _| {
                let dying_model = manager
                    .entity_mut(target)
                    .and_then(|entity| entity.apply_hive_dying_initializer())
                    .ok_or(NativeCaptorPairBlock::Runtime("hive dying initializer"))?;
                let extent = resources
                    .global_model(dying_model)
                    .map(|model| model.radius)
                    .unwrap_or(0);
                crate::hive_death::emit_hive_dying_surface_burst_for_entity(
                    manager,
                    world_fx,
                    target,
                    extent,
                    sea_level_raw,
                );
                Ok(crate::live_actor_checked_damage::LiveActorDeathResult {
                    returned_nonzero: true,
                    publication: None::<()>,
                })
            },
        )
        .map_err(|error| {
            *committed |= error.committed_prefix;
            match error.reason {
                crate::live_actor_checked_damage::LiveActorDamageBlock::Death(block) => block,
                _ => NativeCaptorPairBlock::Runtime("hive checked damage"),
            }
        })?;
        return Ok(());
    }
    if matches!(kind, 9 | 17 | 122 | 123)
        || NativeWorkerProfile::from_entity_type(kind).is_some()
        || NativeFourChoiceProfile::from_entity_type(kind).is_some()
    {
        let completion = capture::apply_capture_pair_checked_damage(
            frame.entities,
            target,
            delivery,
            &mut capture::CaptureContext {
                tasks: frame.actor_tasks,
                world_fx: frame.world_fx,
                notifications: frame.notifications,
                retail_tick: frame.retail_tick,
                result_screen:
                    crate::main_base_type9_abort::MainBaseType9ResultScreenState::NotShown,
                hive_dying: Default::default(),
            },
        )
        .map_err(NativeCaptorPairBlock::Capture)?;
        if let Some(owner) = completion.common_dying_owner {
            frame.actor_tasks.register_intro2_common_dying(owner);
        }
        return Ok(());
    }
    if frame.entities.iter_all().any(|entity| {
        entity.id == target && crate::intro2_gun_turret::is_native_ordinary_gun_turret(entity)
    }) {
        return apply_native_turret_pair_checked_damage(
            frame,
            target,
            delivery,
            playing_player,
            committed,
        );
    }
    if kind == 47
        && !frame
            .actor_tasks
            .prepare_native_actor_mutation(frame.entities, target)
    {
        return Err(NativeCaptorPairBlock::Runtime(
            "completed gunner damage owner",
        ));
    }
    // The generic admission/filter/surviving-health path is independent of a
    // projectile. On lethal damage the existing common Class12 owner must
    // authenticate this allocation; other death programs remain explicit
    // errors, preserving the committed health prefix.
    let checked = crate::live_actor_checked_damage::apply_live_actor_checked_damage(
        frame.entities,
        frame.world_fx,
        crate::live_actor_checked_damage::LiveActorDamageRequest {
            ratio_numerator: 0,
            ratio_denominator: 0,
            entity_id: target,
            delivery,
            entry: crate::live_actor_checked_damage::LiveActorDamageEntry::Checked,
            feedback: Some(crate::live_actor_checked_damage::LiveActorDamageFeedback {
                notifications: frame.notifications,
                retail_tick: frame.retail_tick,
            }),
        },
        |manager, world_fx, _| {
            crate::intro2_common_dying::publish_intro2_common_standard_death(
                manager, target, world_fx,
            )
            .map(
                |owner| crate::live_actor_checked_damage::LiveActorDeathResult {
                    returned_nonzero: owner.is_some(),
                    publication: owner,
                },
            )
        },
    )
    .map_err(|error| {
        *committed |= error.committed_prefix;
        if let Some(owner) = error.death_publication {
            frame.actor_tasks.register_intro2_common_dying(owner);
        }
        if kind == 47 && error.committed_prefix {
            frame.actor_tasks.park_native_type47_external_prefix(target);
        }
        NativeCaptorPairBlock::Damage(error)
    })?;
    if let Some(owner) = checked.death_publication {
        frame.actor_tasks.register_intro2_common_dying(owner);
    }
    Ok(())
}

#[cfg(test)]
#[path = "pair_people_tests.rs"]
mod people_tests;

/// Ordinary Type97 active-pair checked damage through the shared 15040 path.
///
/// The physical cap enters 15040 directly after both participants'
/// displacement/velocity writes. It does not execute 10EB0's hit-tick,
/// impact reaction or infected reselection, nor DAC0/DA00 accepted-hit
/// presentation. Nonlethal damage only writes health/buffer through the
/// authenticated allocation; lethal damage finishes the Playing Class49
/// terminal (BAF0 -> static/dynamic radial -> A860 -> Type60 ring ->
/// deferred removal) with the explicit world/player context. Campaign
/// zero-record reconstruction never borrows this ordinary task admission.
fn apply_native_turret_pair_checked_damage(
    frame: &mut Intro2ContactFrame<'_>,
    target: u32,
    delivery: crate::damage::DamageDeliveryRecord,
    mut playing_player: Option<PlayingPlayerContact<'_>>,
    committed: &mut bool,
) -> Result<(), NativeCaptorPairBlock> {
    use crate::live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageRequest,
    };
    // Ordinary 104B0 turrets only; Intro2 Type92/102 retain their cinematic
    // owners and campaign E/L reconstruction is a distinct origin.
    if !crate::intro2_gun_turret::is_native_ordinary_gun_turret(actor(frame.entities, target)?) {
        return Err(NativeCaptorPairBlock::Runtime("turret pair origin"));
    }
    if !crate::intro2_gun_turret::intro2_gun_turret_manager_allocation_authenticates(
        frame.entities,
        target,
    ) {
        return Err(NativeCaptorPairBlock::Runtime(
            "native turret pair allocation",
        ));
    }
    if !frame
        .actor_tasks
        .prepare_native_actor_mutation(frame.entities, target)
    {
        return Err(NativeCaptorPairBlock::Runtime(
            "completed turret damage owner",
        ));
    }
    // 4C8230 is the living Class29 style with a null pair slot; 4C71E0 is the
    // finished Class49 continuation retained for re-hits. Anything else is an
    // unaudited hit slot, matching the particle-hit style gate.
    let finished =
        crate::class49_death::finished_terminal_hit_authenticates(frame.entities, target);
    let style = style_address(actor(frame.entities, target)?)?;
    if style != if finished { 0x004c_71e0 } else { 0x004c_8230 } {
        return Err(NativeCaptorPairBlock::Runtime(
            "unaudited turret pair style",
        ));
    }
    let had_playing_context = playing_player.is_some();
    let crate::intro2_contacts::Intro2ContactFrame {
        entities,
        resources,
        world_fx,
        static_damage,
        notifications,
        retail_tick,
        actor_tasks,
    } = &mut *frame;
    let retail_tick = *retail_tick;
    let result = apply_live_actor_checked_damage(
        entities,
        world_fx,
        LiveActorDamageRequest {
            ratio_numerator: 0,
            ratio_denominator: 0,
            entity_id: target,
            delivery,
            entry: LiveActorDamageEntry::Checked,
            feedback: Some(crate::live_actor_checked_damage::LiveActorDamageFeedback {
                notifications,
                retail_tick,
            }),
        },
        |manager, world_fx, feedback| {
            let Some(player) = playing_player.as_mut() else {
                // Lethal without the explicit Playing context: preserve the
                // committed health prefix and report the cinematic boundary
                // through the caller's MissingPlayingContext mapping below.
                return Err(
                    crate::class49_terminal::Class49TerminalBlock::PlayingPlayerContextUnavailable,
                );
            };
            // 15040's death reborrows the current feedback context for
            // synchronous child callbacks, like the capture path; the outer
            // notification borrow is already held by the request above.
            let feedback = feedback.expect("pair request retains its notification context");
            crate::class49_terminal::run_class49_standard_death(
                crate::class49_terminal::Class49TerminalFrame {
                    entities: manager,
                    resources,
                    world_fx,
                    static_damage,
                    notifications: feedback.notifications,
                    retail_tick: feedback.retail_tick,
                    world: crate::class49_terminal::Class49WorldContext::Playing {
                        scheduler: actor_tasks,
                        player_hull: player.hull,
                        extra_lives: player.extra_lives,
                        active_terminal_calls: Vec::new(),
                    },
                },
                target,
            )
            .map(
                |result| crate::live_actor_checked_damage::LiveActorDeathResult {
                    returned_nonzero: result.returned_nonzero,
                    publication: None,
                },
            )
        },
    );
    match result {
        Ok(_) => Ok(()),
        Err(error) => {
            *committed |= error.committed_prefix;
            // The terminal parks its own Class49 receipt on failure; the
            // turret's external task prefix also parks like the particle path
            // so a later visit does not replay time, selection or RNG.
            if error.committed_prefix {
                if let Ok(owner) =
                    crate::intro2_gun_turret::Intro2GunTurretOwner::adopt_blocked_prefix(
                        entities, target,
                    )
                {
                    actor_tasks.register_intro2_gun_turret(owner);
                }
                actor_tasks.park_intro2_gun_turret_external_prefix(target);
            }
            if !had_playing_context
                && matches!(
                    &error.reason,
                    crate::live_actor_checked_damage::LiveActorDamageBlock::Death(
                        crate::class49_terminal::Class49TerminalBlock::PlayingPlayerContextUnavailable
                    )
                )
            {
                return Err(NativeCaptorPairBlock::MissingPlayingContext { entity_id: target });
            }
            Err(NativeCaptorPairBlock::TurretPairDamage(error))
        }
    }
}

#[cfg(test)]
#[path = "pair_insect_factory_tests.rs"]
mod insect_factory_tests;

#[cfg(test)]
#[path = "pair_hive_impact_tests.rs"]
mod hive_impact_tests;

#[cfg(test)]
#[path = "pair_ground_damage_tests.rs"]
mod ground_damage_tests;
