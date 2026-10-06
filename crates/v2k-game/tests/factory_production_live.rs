use std::num::NonZeroU32;

use v2k_game::factory_production::{
    FactoryProductionPhase, FactoryProductionRuntime, FactorySection13Config, FACTORY_CONFIG_BYTES,
    FACTORY_HIGH_THRESHOLD_DIRECT_TEXT_ID, FACTORY_MATERIALISER_ENTITY_TYPE,
    FACTORY_OUTPUT_CONVERSION_GATE_RAW, FACTORY_OUTPUT_CONVERSION_HUD_RESOURCE_ID,
    FACTORY_OUTPUT_CONVERSION_SOUND_ID, FACTORY_PICKUP_ENTITY_TYPE,
};
use v2k_game::factory_production_live::{
    animation_transition, converted_output_entity_type, factory_status_ratio, published_status,
    FactoryAnimationRange, FactoryEntitySpawnRequest, FactoryEntitySpawnResult,
    FactoryPickupPresence, FactoryPositionalSoundRequest, FactoryProductionAction,
    FactoryProductionActionPhase, FactoryProductionAdmissionRejection,
    FactoryProductionEntityVersion, FactoryProductionExternalBlock, FactoryProductionFrameRequest,
    FactoryProductionMachine, FactoryProductionPoll, FactoryProductionProtocolError,
    FactoryProductionResume, FactoryProductionTransactionId, FactoryRequestedEntityHandle,
    FactorySpawnRole, FactorySpawnedEntity, IssuedFactoryProductionAction,
    FACTORY_CONVERTED_OUTPUT_Z_OFFSET_RAW, FACTORY_ENTITY_DIRTY_FLAG,
    FACTORY_ENTITY_LIFETIME_OFFSET,
};

fn write_word(raw: &mut [u8; FACTORY_CONFIG_BYTES], offset: usize, value: i32) {
    raw[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn production(phase: FactoryProductionPhase) -> FactoryProductionRuntime {
    let mut raw = [0_u8; FACTORY_CONFIG_BYTES];
    write_word(&mut raw, 0x00, 0x1234_5678);
    write_word(&mut raw, 0x04, 2);
    write_word(&mut raw, 0x08, 1_000);
    write_word(&mut raw, 0x0c, 10);
    write_word(&mut raw, 0x10, 20);
    write_word(&mut raw, 0x14, 20);
    let mut production =
        FactoryProductionRuntime::from_retail_template(FactorySection13Config::decode(&raw), 100);
    production.current_scientists_raw = 2;
    production.remaining_stock_raw = 2;
    production.understaffed_countdown_micros_raw = 10;
    production.phase = phase;
    production
}

fn frame(production: FactoryProductionRuntime) -> FactoryProductionFrameRequest {
    FactoryProductionFrameRequest {
        factory: FactoryProductionEntityVersion {
            entity_id: 0x04ab_0001,
            allocation_identity: 7,
            state_version: 11,
        },
        position_raw: [32_760, -32_760, 100],
        pickup_spawn_offset_raw: [10, -10, -200],
        current_health_raw: 100,
        maximum_health_raw: 100,
        progressive_death_elapsed_raw: 0,
        production,
        animation_state_raw: 0,
        elapsed_micros: 1,
        world_style_raw: 1,
        phase1_presentation_enabled: false,
        suppress_status_publication: false,
    }
}

fn transaction(raw: u64) -> FactoryProductionTransactionId {
    FactoryProductionTransactionId::new(raw).unwrap()
}

fn spawned(handle: u32, allocation_identity: u64) -> FactorySpawnedEntity {
    FactorySpawnedEntity {
        entity_id: NonZeroU32::new(handle).unwrap(),
        allocation_identity,
    }
}

fn issue(machine: &mut FactoryProductionMachine) -> IssuedFactoryProductionAction {
    match machine.poll() {
        FactoryProductionPoll::Action(issued) => issued,
        other => panic!("expected action, got {other:?}"),
    }
}

fn acknowledge(machine: &mut FactoryProductionMachine, issued: IssuedFactoryProductionAction) {
    let phase = issued.action.phase();
    machine
        .resume(
            issued.receipt,
            FactoryProductionResume::Acknowledged { phase },
        )
        .unwrap();
}

fn finish_spawn(
    machine: &mut FactoryProductionMachine,
    issued: IssuedFactoryProductionAction,
    result: FactoryEntitySpawnResult,
) {
    let phase = issued.action.phase();
    machine
        .resume(
            issued.receipt,
            FactoryProductionResume::SpawnCompleted { phase, result },
        )
        .unwrap();
}

fn finish_presence(
    machine: &mut FactoryProductionMachine,
    issued: IssuedFactoryProductionAction,
    presence: FactoryPickupPresence,
) {
    let phase = issued.action.phase();
    machine
        .resume(
            issued.receipt,
            FactoryProductionResume::PickupPresence { phase, presence },
        )
        .unwrap();
}

#[test]
fn preflight_routes_every_non_production_owner_path_in_retail_order() {
    let baseline = frame(production(FactoryProductionPhase::Producing));

    let mut request = baseline;
    request.progressive_death_elapsed_raw = 1;
    request.current_health_raw = 99;
    assert_eq!(
        FactoryProductionMachine::preflight(transaction(1), request),
        Err(FactoryProductionAdmissionRejection::ProgressiveDeathActive)
    );

    let mut request = baseline;
    request.current_health_raw = 99;
    assert_eq!(
        FactoryProductionMachine::preflight(transaction(2), request),
        Err(FactoryProductionAdmissionRejection::CachedHealthTransition)
    );

    let mut request = baseline;
    request.current_health_raw = 99;
    request.production.cached_health_raw = 99;
    assert_eq!(
        FactoryProductionMachine::preflight(transaction(3), request),
        Err(FactoryProductionAdmissionRejection::DamagedFactory)
    );

    let mut request = baseline;
    request.production.current_scientists_raw = 1;
    assert_eq!(
        FactoryProductionMachine::preflight(transaction(4), request),
        Err(FactoryProductionAdmissionRejection::UnderstaffedFactory)
    );
}

#[test]
fn phase_zero_success_commits_the_exact_spawn_and_nonrollback_tail_order() {
    let mut production = production(FactoryProductionPhase::Producing);
    production.production_threshold_micros_raw = 4_000_001;
    production.production_progress_micros_raw = 4_000_000;
    let request = frame(production);
    let factory = request.factory;
    let mut machine = FactoryProductionMachine::preflight(transaction(10), request).unwrap();

    let pickup = issue(&mut machine);
    assert_eq!(pickup.before_action.factory, factory);
    assert_eq!(
        pickup
            .before_action
            .production
            .production_progress_micros_raw,
        4_000_001
    );
    assert_eq!(
        pickup.before_action.production.phase,
        FactoryProductionPhase::Producing
    );
    assert_eq!(
        pickup.action,
        FactoryProductionAction::SpawnEntity {
            phase: FactoryProductionActionPhase::SpawnPickup,
            role: FactorySpawnRole::Pickup,
            request: FactoryEntitySpawnRequest {
                requested_handle: FactoryRequestedEntityHandle::Allocate,
                entity_type: FACTORY_PICKUP_ENTITY_TYPE,
                position_raw: [-32_766, 32_766, -100],
                spawn_parameter_6: 0x1234_5678,
            },
        }
    );
    assert_eq!(
        machine.production().production_progress_micros_raw,
        4_000_001
    );
    assert_eq!(machine.production().remaining_stock_raw, 2);

    let child = spawned(0x04ac_0001, 8);
    finish_spawn(
        &mut machine,
        pickup,
        FactoryEntitySpawnResult::Spawned(child),
    );
    let link = issue(&mut machine);
    assert_eq!(
        link.action,
        FactoryProductionAction::LinkOwner {
            phase: FactoryProductionActionPhase::LinkPickupOwner,
            spawned: child,
            owner_factory: factory,
        }
    );
    acknowledge(&mut machine, link);
    assert_eq!(machine.production().spawned_pickup_handle, 0x04ac_0001);
    assert_eq!(machine.production().remaining_stock_raw, 1);

    let text = issue(&mut machine);
    assert_eq!(
        text.action,
        FactoryProductionAction::EmitDirectText {
            phase: FactoryProductionActionPhase::EmitHighThresholdDirectText,
            direct_text_id: FACTORY_HIGH_THRESHOLD_DIRECT_TEXT_ID,
            sub_parameter: 0,
        }
    );
    acknowledge(&mut machine, text);
    assert_eq!(
        machine.production().phase,
        FactoryProductionPhase::Producing
    );
    assert_eq!(
        machine.production().production_progress_micros_raw,
        4_000_001
    );

    let clear = issue(&mut machine);
    assert_eq!(
        clear.action,
        FactoryProductionAction::ClearFactoryLifetime {
            phase: FactoryProductionActionPhase::ClearFactoryLifetime,
            factory,
            lifetime_offset: FACTORY_ENTITY_LIFETIME_OFFSET,
            value_raw: 0,
        }
    );
    acknowledge(&mut machine, clear);
    assert_eq!(
        machine.production().phase,
        FactoryProductionPhase::Delivering
    );

    let animation = issue(&mut machine);
    assert_eq!(
        animation.before_action.production.phase,
        FactoryProductionPhase::Delivering
    );
    assert_eq!(
        animation.action,
        FactoryProductionAction::ApplyAnimationTransition {
            phase: FactoryProductionActionPhase::ApplyAnimationTransition,
            factory,
            transition: animation_transition(0, 3),
        }
    );
    acknowledge(&mut machine, animation);

    let status = issue(&mut machine);
    match status.action {
        FactoryProductionAction::PublishStatus {
            phase: FactoryProductionActionPhase::PublishStatus,
            factory: status_factory,
            status,
        } => {
            assert_eq!(status_factory, factory);
            assert_eq!(status.current_scientists_raw, 2);
            assert_eq!(status.scientist_capacity_raw, 2);
            assert_eq!(status.output_selector_raw, 0x78);
            assert_eq!(status.production_or_cooldown_ratio_raw, u16::MAX);
            assert_eq!(status.delivery_or_cooldown_ratio_raw, 0);
            assert_eq!(status.understaffed_ratio_raw, factory_status_ratio(9, 20));
        }
        other => panic!("expected status publication, got {other:?}"),
    }
    acknowledge(&mut machine, status);

    let dirty = issue(&mut machine);
    assert_eq!(
        dirty.action,
        FactoryProductionAction::MarkEntityDirty {
            phase: FactoryProductionActionPhase::MarkEntityDirty,
            factory,
            flag: FACTORY_ENTITY_DIRTY_FLAG,
        }
    );
    acknowledge(&mut machine, dirty);

    match machine.poll() {
        FactoryProductionPoll::Complete(completion) => {
            assert!(completion.status_published);
            assert!(completion.dirty_flag_written);
            assert_eq!(completion.animation_state_raw, 3);
        }
        other => panic!("expected completion, got {other:?}"),
    }
}

#[test]
fn phase_zero_spawn_failure_still_enters_delivery_and_clears_lifetime() {
    let mut production = production(FactoryProductionPhase::Producing);
    production.production_progress_micros_raw = 999;
    let mut machine =
        FactoryProductionMachine::preflight(transaction(20), frame(production)).unwrap();

    let pickup = issue(&mut machine);
    finish_spawn(&mut machine, pickup, FactoryEntitySpawnResult::Failed);
    assert_eq!(
        machine.production().phase,
        FactoryProductionPhase::Producing
    );
    assert_eq!(machine.production().production_progress_micros_raw, 1_000);
    assert_eq!(machine.production().remaining_stock_raw, 2);
    assert_eq!(machine.production().spawned_pickup_handle, 0);
    let clear = issue(&mut machine);
    assert!(matches!(
        clear.action,
        FactoryProductionAction::ClearFactoryLifetime { .. }
    ));
    acknowledge(&mut machine, clear);
    assert_eq!(
        machine.production().phase,
        FactoryProductionPhase::Delivering
    );
}

#[test]
fn receipts_reject_wrong_responses_and_external_blocks_preserve_prefix_state() {
    let mut production = production(FactoryProductionPhase::Producing);
    production.production_progress_micros_raw = 999;
    let mut machine =
        FactoryProductionMachine::preflight(transaction(30), frame(production)).unwrap();
    let pickup = issue(&mut machine);
    let committed_prefix = pickup.before_action.production;
    let phase = pickup.action.phase();

    let failure = machine
        .resume(
            pickup.receipt,
            FactoryProductionResume::Acknowledged { phase },
        )
        .unwrap_err();
    assert_eq!(
        failure.error,
        FactoryProductionProtocolError::ResponseKindMismatch { phase }
    );
    assert_eq!(machine.poll(), FactoryProductionPoll::Awaiting(phase));

    let failure = machine
        .resume(
            failure.receipt,
            FactoryProductionResume::Blocked {
                phase,
                reason: FactoryProductionExternalBlock::DirtyFlagUnavailable,
            },
        )
        .unwrap_err();
    assert_eq!(
        failure.error,
        FactoryProductionProtocolError::ResponseKindMismatch { phase }
    );
    assert_eq!(machine.poll(), FactoryProductionPoll::Awaiting(phase));

    machine
        .resume(
            failure.receipt,
            FactoryProductionResume::Blocked {
                phase,
                reason: FactoryProductionExternalBlock::EntitySpawnUnavailable,
            },
        )
        .unwrap();
    match machine.poll() {
        FactoryProductionPoll::Blocked(block) => {
            assert_eq!(block.phase, FactoryProductionActionPhase::SpawnPickup);
            assert_eq!(
                block.reason,
                FactoryProductionExternalBlock::EntitySpawnUnavailable
            );
            assert_eq!(block.production.production_progress_micros_raw, 1_000);
            assert_eq!(block.production, committed_prefix);
            assert_eq!(block.production.phase, FactoryProductionPhase::Producing);
            assert!(!block.status_published);
            assert!(!block.dirty_flag_written);
        }
        other => panic!("expected durable block, got {other:?}"),
    }
}

#[test]
fn unavailable_state_commit_blocks_before_the_external_action_and_keeps_recovery_state() {
    let mut production = production(FactoryProductionPhase::Producing);
    production.production_progress_micros_raw = 999;
    let mut machine =
        FactoryProductionMachine::preflight(transaction(35), frame(production)).unwrap();

    let pickup = issue(&mut machine);
    let phase = pickup.action.phase();
    let recovery = pickup.before_action;
    machine
        .resume(
            pickup.receipt,
            FactoryProductionResume::Blocked {
                phase,
                reason: FactoryProductionExternalBlock::FactoryStateCommitUnavailable,
            },
        )
        .unwrap();

    match machine.poll() {
        FactoryProductionPoll::Blocked(block) => {
            assert_eq!(
                block.reason,
                FactoryProductionExternalBlock::FactoryStateCommitUnavailable
            );
            assert_eq!(block.factory, recovery.factory);
            assert_eq!(block.production, recovery.production);
            assert_eq!(block.animation_state_raw, recovery.animation_state_raw);
        }
        other => panic!("expected recovery block, got {other:?}"),
    }
}

#[test]
fn phase_one_presentation_reenables_a_suppressed_publication() {
    let mut production = production(FactoryProductionPhase::Delivering);
    production.delivery_progress_micros_raw = 9;
    let mut request = frame(production);
    request.animation_state_raw = 3;
    request.phase1_presentation_enabled = true;
    request.suppress_status_publication = true;
    let mut machine = FactoryProductionMachine::preflight(transaction(40), request).unwrap();

    let presentation = issue(&mut machine);
    assert!(matches!(
        presentation.action,
        FactoryProductionAction::DispatchPhase1Presentation {
            phase: FactoryProductionActionPhase::DispatchPhase1Presentation,
            ..
        }
    ));
    acknowledge(&mut machine, presentation);
    assert_eq!(
        machine.production().phase,
        FactoryProductionPhase::WaitingForPickup
    );

    let animation = issue(&mut machine);
    acknowledge(&mut machine, animation);
    let status = issue(&mut machine);
    assert!(matches!(
        status.action,
        FactoryProductionAction::PublishStatus { .. }
    ));
    acknowledge(&mut machine, status);
    let dirty = issue(&mut machine);
    assert!(matches!(
        dirty.action,
        FactoryProductionAction::MarkEntityDirty { .. }
    ));
}

#[test]
fn suppressed_status_also_suppresses_the_active_phase_dirty_write() {
    let mut request = frame(production(FactoryProductionPhase::Producing));
    request.suppress_status_publication = true;
    let mut machine = FactoryProductionMachine::preflight(transaction(45), request).unwrap();

    let animation = issue(&mut machine);
    assert!(matches!(
        animation.action,
        FactoryProductionAction::ApplyAnimationTransition { .. }
    ));
    acknowledge(&mut machine, animation);

    match machine.poll() {
        FactoryProductionPoll::Complete(completion) => {
            assert!(!completion.status_published);
            assert!(!completion.dirty_flag_written);
            assert_eq!(completion.production.production_progress_micros_raw, 1);
        }
        other => panic!("expected completion without status tail, got {other:?}"),
    }
}

#[test]
fn phase_two_pickup_loss_starts_cooldown_but_an_active_pickup_does_not() {
    for (presence, expected_phase) in [
        (
            FactoryPickupPresence::Active,
            FactoryProductionPhase::WaitingForPickup,
        ),
        (
            FactoryPickupPresence::Gone,
            FactoryProductionPhase::Cooldown,
        ),
    ] {
        let mut production = production(FactoryProductionPhase::WaitingForPickup);
        production.spawned_pickup_handle = 0x04ac_0001;
        production.production_progress_micros_raw = 1_000;
        production.delivery_progress_micros_raw = 10;
        let mut request = frame(production);
        request.animation_state_raw = 1;
        request.suppress_status_publication = true;
        let mut machine =
            FactoryProductionMachine::preflight(transaction(50 + presence as u64), request)
                .unwrap();

        let query = issue(&mut machine);
        assert_eq!(
            query.action,
            FactoryProductionAction::QueryPickupPresence {
                phase: FactoryProductionActionPhase::QueryPickupPresence,
                pickup_handle: 0x04ac_0001,
            }
        );
        finish_presence(&mut machine, query, presence);
        assert_eq!(machine.production().phase, expected_phase);
        if presence == FactoryPickupPresence::Gone {
            assert_eq!(machine.production().cooldown_remaining_micros_raw, 20);
            assert_eq!(machine.production().production_progress_micros_raw, 0);
            assert_eq!(machine.production().delivery_progress_micros_raw, 0);
        }
    }
}

#[test]
fn conversion_uses_exact_packets_and_keeps_decrements_after_materialiser_failure() {
    let mut production = production(FactoryProductionPhase::WaitingForPickup);
    production.remaining_stock_raw = 0;
    production.production_progress_micros_raw =
        production.production_threshold_micros_raw + FACTORY_OUTPUT_CONVERSION_GATE_RAW;
    let mut request = frame(production);
    request.animation_state_raw = 1;
    request.world_style_raw = 5;
    let factory = request.factory;
    let mut machine = FactoryProductionMachine::preflight(transaction(60), request).unwrap();

    let hud = issue(&mut machine);
    assert_eq!(
        hud.action,
        FactoryProductionAction::QueueHudResource {
            phase: FactoryProductionActionPhase::QueueConversionHudResource,
            resource_id: FACTORY_OUTPUT_CONVERSION_HUD_RESOURCE_ID,
        }
    );
    acknowledge(&mut machine, hud);

    let output_request = FactoryEntitySpawnRequest {
        requested_handle: FactoryRequestedEntityHandle::Allocate,
        entity_type: 0x74,
        position_raw: [
            32_760,
            -32_760,
            100_i16.wrapping_add(FACTORY_CONVERTED_OUTPUT_Z_OFFSET_RAW),
        ],
        spawn_parameter_6: 0,
    };
    let output_spawn = issue(&mut machine);
    assert_eq!(
        output_spawn.action,
        FactoryProductionAction::SpawnEntity {
            phase: FactoryProductionActionPhase::SpawnConvertedOutput,
            role: FactorySpawnRole::ConvertedOutput,
            request: output_request,
        }
    );
    let output = spawned(0x04ad_0001, 9);
    finish_spawn(
        &mut machine,
        output_spawn,
        FactoryEntitySpawnResult::Spawned(output),
    );
    let output_link = issue(&mut machine);
    assert_eq!(
        output_link.action,
        FactoryProductionAction::LinkOwner {
            phase: FactoryProductionActionPhase::LinkConvertedOutputOwner,
            spawned: output,
            owner_factory: factory,
        }
    );
    acknowledge(&mut machine, output_link);
    assert_eq!(machine.production().current_scientists_raw, 1);
    assert_eq!(machine.production().scientist_capacity_raw, 1);

    let materialiser = issue(&mut machine);
    assert_eq!(
        materialiser.action,
        FactoryProductionAction::SpawnEntity {
            phase: FactoryProductionActionPhase::SpawnMaterialiser,
            role: FactorySpawnRole::Materialiser,
            request: FactoryEntitySpawnRequest {
                entity_type: FACTORY_MATERIALISER_ENTITY_TYPE,
                ..output_request
            },
        }
    );
    finish_spawn(&mut machine, materialiser, FactoryEntitySpawnResult::Failed);
    assert_eq!(machine.production().current_scientists_raw, 1);
    assert_eq!(machine.production().scientist_capacity_raw, 1);
    assert!(matches!(
        issue(&mut machine).action,
        FactoryProductionAction::PublishStatus { .. }
    ));
}

#[test]
fn converted_output_failure_keeps_hud_but_not_staffing_or_later_actions() {
    let mut production = production(FactoryProductionPhase::WaitingForPickup);
    production.remaining_stock_raw = 0;
    production.production_progress_micros_raw =
        production.production_threshold_micros_raw + FACTORY_OUTPUT_CONVERSION_GATE_RAW;
    let mut request = frame(production);
    request.animation_state_raw = 1;
    let mut machine = FactoryProductionMachine::preflight(transaction(65), request).unwrap();

    let hud = issue(&mut machine);
    assert!(matches!(
        hud.action,
        FactoryProductionAction::QueueHudResource {
            resource_id: FACTORY_OUTPUT_CONVERSION_HUD_RESOURCE_ID,
            ..
        }
    ));
    acknowledge(&mut machine, hud);
    let output_spawn = issue(&mut machine);
    assert!(matches!(
        output_spawn.action,
        FactoryProductionAction::SpawnEntity {
            role: FactorySpawnRole::ConvertedOutput,
            ..
        }
    ));
    finish_spawn(&mut machine, output_spawn, FactoryEntitySpawnResult::Failed);

    assert_eq!(machine.production().current_scientists_raw, 2);
    assert_eq!(machine.production().scientist_capacity_raw, 2);
    assert!(matches!(
        issue(&mut machine).action,
        FactoryProductionAction::PublishStatus { .. }
    ));
}

#[test]
fn successful_materialiser_link_precedes_the_conversion_positional_sound() {
    let mut production = production(FactoryProductionPhase::WaitingForPickup);
    production.remaining_stock_raw = 0;
    production.production_progress_micros_raw =
        production.production_threshold_micros_raw + FACTORY_OUTPUT_CONVERSION_GATE_RAW;
    let mut request = frame(production);
    request.animation_state_raw = 1;
    let mut machine = FactoryProductionMachine::preflight(transaction(70), request).unwrap();

    let hud = issue(&mut machine);
    acknowledge(&mut machine, hud);
    let output_spawn = issue(&mut machine);
    let output = spawned(0x04ad_0001, 9);
    finish_spawn(
        &mut machine,
        output_spawn,
        FactoryEntitySpawnResult::Spawned(output),
    );
    let output_link = issue(&mut machine);
    acknowledge(&mut machine, output_link);
    let materialiser_spawn = issue(&mut machine);
    let materialiser = spawned(0x04ae_0001, 10);
    finish_spawn(
        &mut machine,
        materialiser_spawn,
        FactoryEntitySpawnResult::Spawned(materialiser),
    );

    let link = issue(&mut machine);
    assert_eq!(
        link.action,
        FactoryProductionAction::LinkMaterialiserOutput {
            phase: FactoryProductionActionPhase::LinkMaterialiserOutput,
            materialiser,
            output,
        }
    );
    acknowledge(&mut machine, link);
    let sound = issue(&mut machine);
    assert_eq!(
        sound.action,
        FactoryProductionAction::PlayConversionPositionalSound {
            phase: FactoryProductionActionPhase::PlayConversionPositionalSound,
            request: FactoryPositionalSoundRequest {
                sound_id: FACTORY_OUTPUT_CONVERSION_SOUND_ID,
                position_raw: [32_760, -32_760, -150],
                gain_raw_16_16: 0x1_0000,
                rate_raw_16_16: 0x1_0000,
            },
        }
    );
}

#[test]
fn conversion_gate_does_not_recheck_after_adding_this_frames_elapsed_time() {
    let mut production = production(FactoryProductionPhase::WaitingForPickup);
    production.remaining_stock_raw = 0;
    production.production_progress_micros_raw =
        production.production_threshold_micros_raw + FACTORY_OUTPUT_CONVERSION_GATE_RAW - 1;
    let mut request = frame(production);
    request.animation_state_raw = 1;
    request.elapsed_micros = 1_000;
    request.suppress_status_publication = true;
    let mut machine = FactoryProductionMachine::preflight(transaction(80), request).unwrap();

    assert!(matches!(machine.poll(), FactoryProductionPoll::Complete(_)));
    assert_eq!(
        machine.production().production_progress_micros_raw,
        production.production_progress_micros_raw + 1_000
    );
}

#[test]
fn selector_ratio_status_and_animation_helpers_match_retail_edges() {
    assert_eq!(
        (0..=7)
            .map(converted_output_entity_type)
            .collect::<Vec<_>>(),
        [0, 8, 0x5b, 0x5a, 0x4f, 0x74, 7, 0]
    );
    assert_eq!(factory_status_ratio(0, 0), u16::MAX);
    assert_eq!(factory_status_ratio(0, 10), 0);
    assert_eq!(factory_status_ratio(10, 10), u16::MAX);
    assert_eq!(factory_status_ratio(5, 10), 32_767);
    assert_eq!(factory_status_ratio(0x80_0000, 0x100_0000), 32_767);

    let mut production = production(FactoryProductionPhase::Cooldown);
    production.cooldown_remaining_micros_raw = 5;
    let status = published_status(production);
    assert_eq!(status.production_or_cooldown_ratio_raw, 16_383);
    assert_eq!(status.delivery_or_cooldown_ratio_raw, 16_383);

    let transition = animation_transition(0, 3);
    assert_eq!(
        transition.primary,
        Some(FactoryAnimationRange {
            start_raw_16_16: 0x1_0000,
            end_raw_16_16: 0x1_0000,
        })
    );
    assert_eq!(
        transition.secondary,
        Some(FactoryAnimationRange {
            start_raw_16_16: 0x1_0000,
            end_raw_16_16: 0x1_0000,
        })
    );
}
