//! Shared direct 415040/14E90 phase and synchronous native death publication.
//! Radial callers retain their own eligibility, impulse and mutation-custody
//! policy; attached callers enter directly after their B2/follow prefix.

use crate::entity::{
    DynamicRadialDeathPublication, DynamicRadialLiveBlockReason, DynamicRadialLiveCallbacks,
    DynamicRadialRuntimeField, DynamicRadialUnresolvedReason, EntityManager,
};
use crate::entity_collision_state::RetailRuntimeValue;
use crate::gameplay_notifications::GameplayNotifications;
use crate::intro2_type8::NativeWorkerProfile;
use crate::live_actor_checked_damage::{
    apply_live_actor_checked_damage, LiveActorDamageError, LiveActorDamageFeedback,
    LiveActorDamageOutcome, LiveActorDamageRequest, LiveActorDeathResult,
};
use crate::main_base_type9_abort::MainBaseType9ResultScreenState;
use crate::native_type86::NativeFourChoiceProfile;
use crate::ordinary_type47_death_live::{
    publish_fresh_level_one_type47_standard_death, Type47StandardDeathOutcome,
};
use crate::ordinary_type9_standard_death::OrdinaryType9StandardDeathEntry;
use crate::world_fx::WorldFx;

pub(crate) struct NativeCheckedDamageContext<'a> {
    pub world_fx: &'a mut WorldFx,
    pub retail_tick: u32,
    pub notifications: &'a mut GameplayNotifications,
    pub callbacks: &'a mut dyn DynamicRadialLiveCallbacks,
}

pub(crate) fn apply_native_actor_checked_damage(
    manager: &mut EntityManager,
    request: LiveActorDamageRequest<'_>,
    context: NativeCheckedDamageContext<'_>,
) -> Result<
    LiveActorDamageOutcome<DynamicRadialDeathPublication>,
    LiveActorDamageError<DynamicRadialLiveBlockReason, DynamicRadialDeathPublication>,
> {
    let NativeCheckedDamageContext {
        world_fx,
        retail_tick,
        notifications,
        callbacks,
    } = context;
    let id = request.entity_id;
    let kind = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .map_or(0, |entity| entity.entity_type);
    // Preserve explicit14E90 feedback's sink/tick and the death callback's
    // existing frame sink/tick. A radial/direct request without feedback
    // binds its real frame sink once and lends it to synchronous death.
    let (feedback, mut death_notifications) = match request.feedback {
        Some(feedback) => (feedback, Some(notifications)),
        None => (
            LiveActorDamageFeedback {
                notifications,
                retail_tick,
            },
            None,
        ),
    };
    let request = LiveActorDamageRequest {
        entity_id: request.entity_id,
        delivery: request.delivery,
        entry: request.entry,
        ratio_numerator: request.ratio_numerator,
        ratio_denominator: request.ratio_denominator,
        feedback: Some(feedback),
    };
    let result = apply_live_actor_checked_damage(
        manager,
        world_fx,
        request,
        |manager, world_fx, feedback| {
            let feedback = feedback.expect("native checked frame owns feedback");
            let notifications = match death_notifications.as_deref_mut() {
                Some(notifications) => notifications,
                None => &mut *feedback.notifications,
            };
            if let Some(result) =
                callbacks.standard_death(manager, id, kind, world_fx, retail_tick, notifications)
            {
                result
            } else {
                dispatch_standard_death(
                    manager,
                    id,
                    kind,
                    world_fx,
                    retail_tick,
                    notifications,
                    callbacks,
                )
            }
        },
    );
    let publication = match &result {
        Ok(outcome) => outcome.death_publication,
        Err(error) => error.death_publication,
    };
    if let Some(publication) = publication {
        callbacks.retain_death_publication(publication);
    }
    result
}

fn dispatch_standard_death(
    manager: &mut EntityManager,
    id: u32,
    kind: u32,
    world_fx: &mut WorldFx,
    retail_tick: u32,
    notifications: &mut GameplayNotifications,
    callbacks: &mut dyn DynamicRadialLiveCallbacks,
) -> Result<LiveActorDeathResult<DynamicRadialDeathPublication>, DynamicRadialLiveBlockReason> {
    match kind {
        56 if crate::native_type56::manager_allocation_authenticates(manager, id) => {
            crate::native_type56::death::begin_type56_standard_death(manager, id, world_fx)
                .map(|result| LiveActorDeathResult {
                    returned_nonzero: result.returned_nonzero,
                    publication: result.publication.map(|terminal| match terminal {
                        crate::native_ground_actor::NativeGroundTerminalPublication::CommonDying(owner) =>
                            DynamicRadialDeathPublication::Intro2Class12(owner),
                        crate::native_ground_actor::NativeGroundTerminalPublication::Deferred(receipt) =>
                            DynamicRadialDeathPublication::NativeGroundDeferred(receipt),
                    }),
                })
                .map_err(DynamicRadialLiveBlockReason::NativeType56)
        }
        6 if crate::main_base_runtime::main_base_manager_allocation_authenticates(manager, id) => {
            crate::main_base_runtime::publish_main_base_standard_death(manager, id, world_fx)
                .map(|result| LiveActorDeathResult {
                    returned_nonzero: result.returned_nonzero,
                    publication: result.owner.map(DynamicRadialDeathPublication::MainBase),
                })
                .map_err(DynamicRadialLiveBlockReason::MainBase)
        }
        10 | 5 if crate::intro2_type10::type10_manager_allocation_authenticates(manager, id) => {
            crate::intro2_type10::death::publish_intro2_type10_standard_death(manager, id, world_fx)
                .map(|owner| LiveActorDeathResult {
                    returned_nonzero: owner.is_some(),
                    publication: owner.map(DynamicRadialDeathPublication::Intro2Type10Tumble),
                })
                .map_err(DynamicRadialLiveBlockReason::Intro2Type10)
        }
        57 if manager
            .iter_all()
            .find(|entity| entity.id == id)
            .is_some_and(crate::intro2_type57::intro2_type57_allocation_authenticates) =>
        {
            crate::intro2_type57::death::publish_intro2_type57_standard_death(manager, id, world_fx)
                .map(|owner| LiveActorDeathResult {
                    returned_nonzero: owner.is_some(),
                    publication: owner.map(DynamicRadialDeathPublication::Intro2Type57Tumble),
                })
                .map_err(DynamicRadialLiveBlockReason::Intro2Type57)
        }
        76 | 77 if crate::native_type76::manager_allocation_authenticates(manager, id) => {
            crate::intro2_common_dying::publish_intro2_common_standard_death(manager, id, world_fx)
                .map(|owner| LiveActorDeathResult {
                    returned_nonzero: owner.is_some(),
                    publication: owner.map(DynamicRadialDeathPublication::Intro2Class12),
                })
                .map_err(DynamicRadialLiveBlockReason::Intro2Class12)
        }
        30 if crate::native_type30::manager_allocation_authenticates(manager, id) => {
            crate::intro2_common_dying::publish_intro2_common_standard_death(manager, id, world_fx)
                .map(|owner| LiveActorDeathResult {
                    returned_nonzero: owner.is_some(),
                    publication: owner.map(DynamicRadialDeathPublication::Intro2Class12),
                })
                .map_err(DynamicRadialLiveBlockReason::Intro2Class12)
        }
        58 if crate::intro2_type58::type58_manager_allocation_authenticates(manager, id) => {
            crate::intro2_common_dying::publish_intro2_common_standard_death(manager, id, world_fx)
                .map(|owner| LiveActorDeathResult {
                    returned_nonzero: owner.is_some(),
                    publication: owner.map(DynamicRadialDeathPublication::Intro2Class12),
                })
                .map_err(DynamicRadialLiveBlockReason::Intro2Class12)
        }
        94 if manager
            .iter_all()
            .find(|entity| entity.id == id)
            .is_some_and(crate::intro2_type94::intro2_type94_allocation_authenticates) =>
        {
            crate::intro2_common_dying::publish_intro2_common_standard_death(manager, id, world_fx)
                .map(|owner| LiveActorDeathResult {
                    returned_nonzero: owner.is_some(),
                    publication: owner.map(DynamicRadialDeathPublication::Intro2Class12),
                })
                .map_err(DynamicRadialLiveBlockReason::Intro2Class12)
        }
        16 if manager
            .iter_all()
            .find(|entity| entity.id == id)
            .is_some_and(crate::intro2_type16::intro2_type16_allocation_authenticates) =>
        {
            crate::intro2_common_dying::publish_intro2_common_standard_death(manager, id, world_fx)
                .map(|owner| LiveActorDeathResult {
                    returned_nonzero: owner.is_some(),
                    publication: owner.map(DynamicRadialDeathPublication::Intro2Class12),
                })
                .map_err(DynamicRadialLiveBlockReason::Intro2Class12)
        }
        17 | 122 | 18 | 28
            if crate::intro2_type17::type17_manager_allocation_authenticates(manager, id)
                || crate::native_type122::type122_manager_allocation_authenticates(manager, id)
                || crate::native_type18::manager_allocation_authenticates(manager, id)
                || crate::native_type28::manager_allocation_authenticates(manager, id) =>
        {
            if let Some(tasks) = callbacks.capture_task_custody() {
                return crate::native_actor_capture::publish_native_captor_standard_death(
                    manager,
                    id,
                    &mut crate::intro2_type17::capture::CaptureContext {
                        resources: None,
                        tasks,
                        world_fx,
                        notifications,
                        retail_tick,
                        result_screen:
                            crate::main_base_type9_abort::MainBaseType9ResultScreenState::NotShown,
                        hive_dying: Default::default(),
                    },
                )
                .map(|owner| LiveActorDeathResult {
                    returned_nonzero: owner.is_some(),
                    publication: owner.map(DynamicRadialDeathPublication::Intro2Class12),
                })
                .map_err(DynamicRadialLiveBlockReason::Type17Capture);
            }
            // Detached callers can still exercise null cleanup. Occupied
            // Capture styles require their real child owner at the DB80 call.
            if manager
                .iter_all()
                .find(|entity| entity.id == id)
                .is_some_and(|entity| {
                    matches!(entity.current_behavior_context,
                    RetailRuntimeValue::Known(Some(context))
                        if context.active_style().death_callback_policy()
                            == crate::entity_behavior::DeathCallbackPolicy::CapturePeopleCleanup)
                })
            {
                return Err(DynamicRadialLiveBlockReason::Type17Capture(
                    crate::intro2_type17::capture::CaptureBlock::new(
                        "radial Capture child custody",
                    ),
                ));
            }
            crate::intro2_common_dying::publish_intro2_common_standard_death(manager, id, world_fx)
                .map(|owner| LiveActorDeathResult {
                    returned_nonzero: owner.is_some(),
                    publication: owner.map(DynamicRadialDeathPublication::Intro2Class12),
                })
                .map_err(DynamicRadialLiveBlockReason::Intro2Class12)
        }
        26 if manager
            .iter_all()
            .find(|entity| entity.id == id)
            .is_some_and(
                crate::intro2_type26_defecate_virus::intro2_type26_allocation_authenticates,
            ) =>
        {
            crate::intro2_common_dying::publish_intro2_common_standard_death(manager, id, world_fx)
                .map(|owner| LiveActorDeathResult {
                    returned_nonzero: owner.is_some(),
                    publication: owner.map(DynamicRadialDeathPublication::Intro2Class12),
                })
                .map_err(DynamicRadialLiveBlockReason::Intro2Class12)
        }
        66 if manager
            .iter_all()
            .find(|entity| entity.id == id)
            .is_some_and(crate::intro2_type66::intro2_type66_allocation_authenticates) =>
        {
            crate::intro2_type66::death::publish_intro2_type66_standard_death(manager, id, world_fx)
                .map(|death| LiveActorDeathResult {
                    returned_nonzero: death.returned_nonzero,
                    publication: death.owner.map(DynamicRadialDeathPublication::Intro2Type66),
                })
                .map_err(DynamicRadialLiveBlockReason::Intro2Type66)
        }
        53 => {
            crate::intro2_common_dying::publish_intro2_common_standard_death(manager, id, world_fx)
                .map(|owner| LiveActorDeathResult {
                    returned_nonzero: owner.is_some(),
                    publication: owner.map(DynamicRadialDeathPublication::Intro2Class12),
                })
                .map_err(DynamicRadialLiveBlockReason::Intro2Class12)
        }
        47 if manager
            .iter_all()
            .find(|entity| entity.id == id)
            .is_some_and(crate::intro2_type47_live::intro2_type47_cohort_runtime_authenticates) =>
        {
            crate::intro2_common_dying::publish_intro2_common_standard_death(manager, id, world_fx)
                .map(|owner| LiveActorDeathResult {
                    returned_nonzero: owner.is_some(),
                    publication: owner.map(DynamicRadialDeathPublication::Intro2Class12),
                })
                .map_err(DynamicRadialLiveBlockReason::Intro2Class12)
        }
        47 => {
            let metadata = manager
                .type_runtime_metadata(kind)
                .cloned()
                .ok_or_else(|| {
                    DynamicRadialLiveBlockReason::Runtime(
                        DynamicRadialUnresolvedReason::MissingRuntimeField(
                            DynamicRadialRuntimeField::CurrentBehaviorStyle,
                        ),
                    )
                })?;
            match publish_fresh_level_one_type47_standard_death(manager, id, &metadata, world_fx)
                .map_err(DynamicRadialLiveBlockReason::Type47Death)?
            {
                Type47StandardDeathOutcome::Published(publication) => Ok(LiveActorDeathResult {
                    returned_nonzero: true,
                    publication: Some(DynamicRadialDeathPublication::Type47Class12(
                        publication.owner,
                    )),
                }),
                Type47StandardDeathOutcome::AlreadyDyingNoOp
                | Type47StandardDeathOutcome::RemoteOwnedNoOp => Ok(LiveActorDeathResult {
                    returned_nonzero: false,
                    publication: None,
                }),
            }
        }
        type_id
            if NativeWorkerProfile::from_entity_type(type_id).is_some()
                && manager
                    .iter_all()
                    .find(|entity| entity.id == id)
                    .is_some_and(crate::intro2_type8::intro2_type8_allocation_authenticates) =>
        {
            crate::intro2_type8::impact::run_intro2_type8_standard_death(manager, id, world_fx)
                .map(|result| LiveActorDeathResult {
                    returned_nonzero: result.returned_nonzero,
                    publication: result
                        .publication
                        .map(DynamicRadialDeathPublication::Intro2Type8),
                })
                .map_err(DynamicRadialLiveBlockReason::Intro2Type8)
        }
        123 if manager
            .iter_all()
            .find(|entity| entity.id == id)
            .is_some_and(crate::native_type123::native_type123_allocation_authenticates) =>
        {
            crate::native_type123::impact::run_native_type123_standard_death(manager, id, world_fx)
                .map(|result| LiveActorDeathResult {
                    returned_nonzero: result.returned_nonzero,
                    publication: result
                        .publication
                        .map(DynamicRadialDeathPublication::NativeType123),
                })
                .map_err(DynamicRadialLiveBlockReason::NativeType123)
        }
        type_id
            if NativeFourChoiceProfile::from_entity_type(type_id).is_some()
                && manager
                    .iter_all()
                    .find(|entity| entity.id == id)
                    .is_some_and(crate::native_type86::native_type86_allocation_authenticates) =>
        {
            crate::native_type86::impact::run_native_type86_standard_death(manager, id, world_fx)
                .map(|result| LiveActorDeathResult {
                    returned_nonzero: result.returned_nonzero,
                    publication: result
                        .publication
                        .map(DynamicRadialDeathPublication::NativeType86),
                })
                .map_err(DynamicRadialLiveBlockReason::NativeType86)
        }
        67 => {
            let dying_model = manager
                .entity_mut(id)
                .and_then(|entity| entity.apply_hive_dying_initializer());
            match dying_model {
                Some(dying_model) => {
                    crate::hive_death::emit_hive_dying_surface_burst_for_entity(
                        manager,
                        world_fx,
                        id,
                        callbacks
                            .hive_dying_burst_model_extent_raw(dying_model)
                            .unwrap_or(0),
                        callbacks.hive_dying_burst_sea_level_raw(),
                    );
                    Ok(LiveActorDeathResult {
                        returned_nonzero: true,
                        publication: None,
                    })
                }
                None => Err(DynamicRadialLiveBlockReason::UnsupportedDeath {
                    entity_type: 67,
                    alternate_class: Some(46),
                }),
            }
        }
        9 => {
            let native_intro2 = manager
                .iter_all()
                .find(|entity| entity.id == id)
                .is_some_and(crate::intro2_type9::intro2_type9_allocation_authenticates);
            let death = manager
                .publish_ordinary_type9_standard_death(
                    id,
                    OrdinaryType9StandardDeathEntry::GenericDeath,
                    MainBaseType9ResultScreenState::NotShown,
                    world_fx,
                    retail_tick as i32,
                    Some(notifications),
                )
                .map_err(DynamicRadialLiveBlockReason::Type9Death)?;
            let publication = manager
                .main_base_abort_actor_observation(id)
                .map(|observation| observation.lease)
                .and_then(|lease| {
                    crate::ordinary_type9_outer_tail::outer_tail_class14_exploding_task_lease(
                        manager, lease,
                    )
                })
                .map(|lease| {
                    if native_intro2 {
                        DynamicRadialDeathPublication::Intro2Type9Class14(lease)
                    } else {
                        DynamicRadialDeathPublication::Type9Class14(lease)
                    }
                });
            use crate::ordinary_type9_standard_death::OrdinaryType9StandardDeathOutcome as Death;
            Ok(LiveActorDeathResult {
                returned_nonzero: matches!(
                    death,
                    Death::Published { .. } | Death::InitializerFallback { .. }
                ),
                publication,
            })
        }
        _ => Err(DynamicRadialLiveBlockReason::UnsupportedDeath {
            entity_type: kind,
            alternate_class: manager
                .type_runtime_metadata(kind)
                .and_then(|metadata| metadata.initializer.as_ref())
                .map(|initializer| initializer.alternate_behavior_class_ref),
        }),
    }
}

/// Radial and direct attachment share native fish custody, including the
/// completed class2 allocation until14990. Type16/Type57 remain direct-only;
/// their primary hit wrappers do not participate in this source call.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeMutationCaller {
    Radial,
    Attached,
}

pub(crate) fn prepare_native_actor_damage_mutation(
    manager: &EntityManager,
    id: u32,
    caller: NativeMutationCaller,
    callbacks: &mut dyn DynamicRadialLiveCallbacks,
) -> bool {
    if crate::native_type40::death::finished_terminal_authenticates(manager, id)
        || crate::native_type56::death::finished_terminal_authenticates(manager, id)
        // 4566E0 has no dying test: a finished BAC0/BD20/BC90 corpse stays a
        // radial target until 14990, and only its Finished receipt stands in
        // for the retired task custody.
        || crate::class49_death::finished_terminal_hit_authenticates(manager, id)
    {
        return true;
    }
    // 14AE0 and 43FA3D share the 8000/800/1000 gate without a deferred-removal
    // exclusion, so a later radial reaches a finished class49 allocation just
    // as a particle does. Only its completed receipt replaces task custody.
    if crate::class49_death::finished_terminal_hit_authenticates(manager, id) {
        return true;
    }
    if crate::shared_fish::death::completed_shared_fish_death(manager, id) {
        // Both entries can consume an existing corpse buffer before reading
        // dying. Its completed terminal receipt replaces the retired task.
        return true;
    }
    let native_main_base_valid =
        crate::main_base_runtime::main_base_manager_allocation_authenticates(manager, id);
    let native_type9_valid =
        crate::ordinary_type9_construction::ordinary_type9_native_allocation_authenticates(
            manager, id,
        );
    let native_type17_valid =
        crate::intro2_type17::type17_manager_allocation_authenticates(manager, id);
    let native_type26_valid =
        crate::intro2_type26_defecate_virus::type26_manager_allocation_authenticates(manager, id);
    let native_type47_valid =
        crate::shared_type47::type47_manager_allocation_authenticates(manager, id);
    let native_type53_valid =
        crate::intro2_type53::type53_manager_allocation_authenticates(manager, id);
    let native_type58_valid =
        crate::intro2_type58::type58_manager_allocation_authenticates(manager, id);
    let native_type122_valid =
        crate::native_type122::type122_manager_allocation_authenticates(manager, id);
    let native_type18_valid = crate::native_type18::manager_allocation_authenticates(manager, id);
    let native_type28_valid = crate::native_type28::manager_allocation_authenticates(manager, id);
    let native_type76_valid = crate::native_type76::manager_allocation_authenticates(manager, id);
    let native_type30_valid = crate::native_type30::manager_allocation_authenticates(manager, id);
    let native_type40_valid = crate::native_type40::manager_allocation_authenticates(manager, id);
    let native_type56_valid = crate::native_type56::manager_allocation_authenticates(manager, id);
    let native_type43_valid = crate::native_type43::manager_allocation_authenticates(manager, id);
    let native_type38_valid = crate::native_type38::manager_allocation_authenticates(manager, id);
    let native_cleansing_valid = crate::cleansing_vehicle::allocation_authenticates(manager, id);
    let native_gun_turret_valid =
        crate::intro2_gun_turret::intro2_gun_turret_manager_allocation_authenticates(manager, id);
    let native_power_up_valid = crate::native_type61::allocation_authenticates(manager, id);
    let native_type13_valid = crate::class49_death::allocation_authenticates(manager, id)
        && manager
            .iter_all()
            .any(|entity| entity.id == id && entity.entity_type == 13);
    let Some(entity) = manager.iter_all().find(|entity| entity.id == id) else {
        return false;
    };
    let native_weapon = entity.native_entity_weapon_runtime.is_some();
    let native_weapon_valid = native_weapon
        && crate::native_entity_weapons::entity_authenticates(entity)
        && crate::class49_death::allocation_authenticates(manager, id);
    let direct_extra = caller == NativeMutationCaller::Attached
        && (entity.intro2_type16_runtime.is_some()
            || entity.intro2_type57_runtime.is_some()
            || entity.shared_fish_runtime.is_some());
    let direct_extra_valid = if entity.intro2_type16_runtime.is_some() {
        crate::intro2_type16::intro2_type16_allocation_authenticates(entity)
    } else if entity.intro2_type57_runtime.is_some() {
        crate::intro2_type57::intro2_type57_allocation_authenticates(entity)
    } else {
        crate::shared_fish::allocation_authenticates(manager, id)
    };
    let native_actor = direct_extra
        || native_weapon
        || entity.shared_fish_runtime.is_some()
        || native_type13_valid
        || entity.intro2_type8_runtime.is_some()
        || entity.native_type123_runtime.is_some()
        || entity.native_type86_runtime.is_some()
        || entity.ordinary_type9_native_receipt.is_some()
        || entity.main_base_runtime.is_some()
        || entity.intro2_type9_runtime.is_some()
        || entity.intro2_type66_runtime.is_some()
        || entity.intro2_type17_runtime.is_some()
        || entity.native_type26_allocation.is_some()
        || entity.intro2_type53_runtime.is_some()
        || entity.intro2_type58_runtime.is_some()
        || entity.native_type122_runtime.is_some()
        || entity.native_type18_runtime.is_some()
        || entity.native_type28_runtime.is_some()
        || entity.native_type76_runtime.is_some()
        || entity.native_type30_runtime.is_some()
        || entity.native_type40_runtime.is_some()
        || entity.native_type56_runtime.is_some()
        || entity.native_type43_runtime.is_some()
        || entity.native_type38_runtime.is_some()
        || entity.intro2_type94_runtime.is_some()
        || entity.native_type47_construction.is_some()
        || entity.intro2_type10_runtime.is_some()
        || entity.intro2_gun_turret_runtime.is_some()
        || entity.cleansing_vehicle_runtime.is_some()
        || crate::native_type61::has_native_allocation(entity)
        || matches!(
            entity.actor_task_state(crate::actor_task_owner::ActorTaskSlot::Primary),
            Some(crate::actor_task_dispatcher::ActorTaskRuntime::CommonDying(
                _
            ))
        );
    let native_allocation = if native_weapon {
        native_weapon_valid
    } else if entity.shared_fish_runtime.is_some() {
        crate::shared_fish::allocation_authenticates(manager, id)
    } else if direct_extra {
        direct_extra_valid
    } else if native_type13_valid {
        true
    } else if entity.main_base_runtime.is_some() {
        native_main_base_valid
    } else if entity.ordinary_type9_native_receipt.is_some() {
        native_type9_valid
    } else if entity.intro2_type17_runtime.is_some() {
        native_type17_valid
    } else if entity.native_type26_allocation.is_some() {
        native_type26_valid
    } else if entity.native_type47_construction.is_some() {
        native_type47_valid
    } else if entity.intro2_type53_runtime.is_some() {
        native_type53_valid
    } else if entity.intro2_type58_runtime.is_some() {
        native_type58_valid
    } else if entity.native_type122_runtime.is_some() {
        native_type122_valid
    } else if entity.native_type18_runtime.is_some() {
        native_type18_valid
    } else if entity.native_type28_runtime.is_some() {
        native_type28_valid
    } else if entity.native_type76_runtime.is_some() {
        native_type76_valid
    } else if entity.native_type30_runtime.is_some() {
        native_type30_valid
    } else if entity.native_type40_runtime.is_some() {
        native_type40_valid
    } else if entity.native_type56_runtime.is_some() {
        native_type56_valid
    } else if entity.native_type43_runtime.is_some() {
        native_type43_valid
    } else if entity.native_type38_runtime.is_some() {
        native_type38_valid
    } else if entity.intro2_type94_runtime.is_some() {
        crate::intro2_type94::intro2_type94_allocation_authenticates(entity)
    } else if entity.intro2_type8_runtime.is_some() {
        crate::intro2_type8::intro2_type8_allocation_authenticates(entity)
    } else if entity.native_type123_runtime.is_some() {
        crate::native_type123::native_type123_allocation_authenticates(entity)
    } else if entity.native_type86_runtime.is_some() {
        crate::native_type86::native_type86_allocation_authenticates(entity)
    } else if entity.intro2_type9_runtime.is_some() {
        crate::intro2_type9::intro2_type9_allocation_authenticates(entity)
    } else if entity.intro2_type66_runtime.is_some() {
        crate::intro2_type66::intro2_type66_allocation_authenticates(entity)
    } else if entity.intro2_type10_runtime.is_some() {
        crate::intro2_type10::type10_manager_allocation_authenticates(manager, id)
    } else if entity.cleansing_vehicle_runtime.is_some() {
        native_cleansing_valid
    } else if entity.intro2_gun_turret_runtime.is_some() {
        native_gun_turret_valid
    } else if crate::native_type61::has_native_allocation(entity) {
        native_power_up_valid
    } else {
        true
    };
    let pending_terminal = (entity.intro2_type10_runtime.is_some()
        && crate::intro2_type10::death::terminal_is_pending(entity))
        || crate::class49_death::terminal_is_pending(entity);
    !native_actor
        || (native_allocation
            && (callbacks.active_terminal_call(manager, id)
                || (!pending_terminal && callbacks.before_native_actor_mutation(manager, id))))
}

#[cfg(test)]
mod feedback_tests {
    use super::*;
    use crate::{
        damage::{DamageDeliveryRecord, DamagePacket},
        live_actor_checked_damage::LiveActorDamageEntry,
    };

    struct ObserveDeathContext;

    impl DynamicRadialLiveCallbacks for ObserveDeathContext {
        fn before_native_actor_mutation(&mut self, _: &EntityManager, _: u32) -> bool {
            true
        }

        fn standard_death(
            &mut self,
            _: &mut EntityManager,
            _: u32,
            kind: u32,
            _: &mut WorldFx,
            retail_tick: u32,
            notifications: &mut GameplayNotifications,
        ) -> Option<
            Result<
                LiveActorDeathResult<DynamicRadialDeathPublication>,
                DynamicRadialLiveBlockReason,
            >,
        > {
            assert_eq!(kind, 30);
            // Observe the actual death callback's frame without substituting
            // for its native Class12 publication.
            notifications.queue_hull_low(retail_tick as i32);
            None
        }
    }

    #[v2k_test_support::retail_test]
    fn genuine_type30_native_death_preserves_explicit_feedback_or_lends_its_frame() {
        for explicit in [false, true] {
            let (_session, mut manager, mut fx) =
                crate::native_type122::construction_tests::native_fixture_with_player(19);
            let id = manager
                .iter_all()
                .find(|entity| entity.entity_type == 30)
                .unwrap()
                .id;
            let player_id = manager.player().unwrap().id;
            manager
                .entity_mut(id)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(0x0100_0000, 0x0100_0000);
            let mut frame_notifications = GameplayNotifications::new();
            let mut supplied_notifications = GameplayNotifications::new();
            let mut callbacks = ObserveDeathContext;
            let outcome = apply_native_actor_checked_damage(
                &mut manager,
                LiveActorDamageRequest {
                    entity_id: id,
                    delivery: DamageDeliveryRecord {
                        packet: DamagePacket {
                            channels: [2, 3],
                            amounts_raw: [14000, 12000],
                        },
                        source_entity_type_raw: 46,
                        owner_handle: player_id,
                    },
                    entry: LiveActorDamageEntry::Unchecked,
                    ratio_numerator: 0,
                    ratio_denominator: 0,
                    feedback: explicit.then_some(LiveActorDamageFeedback {
                        notifications: &mut supplied_notifications,
                        retail_tick: 321,
                    }),
                },
                NativeCheckedDamageContext {
                    world_fx: &mut fx,
                    retail_tick: 654,
                    notifications: &mut frame_notifications,
                    callbacks: &mut callbacks,
                },
            )
            .expect("genuine Type30 native Class12 and player feedback complete");
            assert!(matches!(
                outcome.death_publication,
                Some(DynamicRadialDeathPublication::Intro2Class12(_))
            ));
            assert!(crate::native_type30::manager_allocation_authenticates(
                &manager, id
            ));
            let mut expected_frame = GameplayNotifications::new();
            expected_frame.queue_hull_low(654);
            let mut expected_supplied = GameplayNotifications::new();
            if explicit {
                expected_supplied.queue_player_kill(321);
            } else {
                expected_frame.queue_player_kill(654);
            }
            assert_eq!(frame_notifications, expected_frame);
            assert_eq!(supplied_notifications, expected_supplied);
        }
    }
}
