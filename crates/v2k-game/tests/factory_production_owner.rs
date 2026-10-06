use v2k_game::base_factory_progression::{
    ProgressiveDeathState, ProgressiveModelEffect, PROGRESSION_REVIVE_HEALTH_RAW,
};
use v2k_game::factory_production::{
    FactoryProductionPhase, FactoryProductionRuntime, FactorySection13Config, FACTORY_CONFIG_BYTES,
};
use v2k_game::factory_production_live::{
    FactoryEntitySpawnResult, FactoryProductionAction, FactoryProductionActionPhase,
    FactoryProductionEntityVersion, FactoryProductionExternalBlock, FactoryProductionProtocolError,
    FactoryProductionTransactionId,
};
use v2k_game::factory_production_owner::{
    FactoryProductionOwnerAction, FactoryProductionOwnerActionPhase,
    FactoryProductionOwnerAdmissionRejection, FactoryProductionOwnerExternalBlock,
    FactoryProductionOwnerFrameRequest, FactoryProductionOwnerMachine, FactoryProductionOwnerPoll,
    FactoryProductionOwnerProtocolError, FactoryProductionOwnerResume,
    FactoryProductionOwnerTransactionId, IssuedFactoryProductionOwnerAction,
    FACTORY_OWNER_CAPACITY_DIRECT_TEXT_ID, FACTORY_OWNER_CAPACITY_HUD_RESOURCE_ID,
    FACTORY_OWNER_ENTRY_FLAGS_AT_0X84_OR_MASK, FACTORY_OWNER_UNDERSTAFFED_DIRECT_TEXT_ID,
};

fn write_i32(raw: &mut [u8; FACTORY_CONFIG_BYTES], offset: usize, value: i32) {
    raw[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn production(
    capacity: i32,
    scientists: i32,
    health: i32,
    phase: FactoryProductionPhase,
) -> FactoryProductionRuntime {
    let mut raw = [0; FACTORY_CONFIG_BYTES];
    write_i32(&mut raw, 0x00, 0x1234);
    write_i32(&mut raw, 0x04, capacity);
    write_i32(&mut raw, 0x08, 100);
    write_i32(&mut raw, 0x0c, 20);
    write_i32(&mut raw, 0x10, 30);
    write_i32(&mut raw, 0x14, 1_000_000);
    let mut production = FactoryProductionRuntime::from_retail_template(
        FactorySection13Config::decode(&raw),
        health,
    );
    production.current_scientists_raw = scientists;
    production.remaining_stock_raw = 2;
    production.phase = phase;
    production
}

fn request(production: FactoryProductionRuntime) -> FactoryProductionOwnerFrameRequest {
    FactoryProductionOwnerFrameRequest {
        factory: FactoryProductionEntityVersion {
            entity_id: 0x04b7_0001,
            allocation_identity: 0x1111,
            state_version: 7,
        },
        position_raw: [100, -200, 300],
        pickup_spawn_offset_raw: [10, 20, -30],
        current_health_raw: 100,
        maximum_health_raw: 100,
        progressive_death: ProgressiveDeathState::idle(0),
        production,
        animation_state_raw: 0,
        elapsed_micros: 1,
        world_style_raw: 1,
        phase1_presentation_enabled: false,
        suppress_status_publication: false,
    }
}

fn owner_id(raw: u64) -> FactoryProductionOwnerTransactionId {
    FactoryProductionOwnerTransactionId::new(raw).unwrap()
}

fn child_id(raw: u64) -> FactoryProductionTransactionId {
    FactoryProductionTransactionId::new(raw).unwrap()
}

fn machine_with(
    owner_raw: u64,
    child_raw: u64,
    request: FactoryProductionOwnerFrameRequest,
) -> FactoryProductionOwnerMachine {
    FactoryProductionOwnerMachine::preflight(owner_id(owner_raw), child_id(child_raw), request)
        .unwrap()
}

fn issue(machine: &mut FactoryProductionOwnerMachine) -> IssuedFactoryProductionOwnerAction {
    match machine.poll() {
        FactoryProductionOwnerPoll::Action(issued) => issued,
        other => panic!("expected action, got {other:?}"),
    }
}

fn acknowledge(
    machine: &mut FactoryProductionOwnerMachine,
    issued: IssuedFactoryProductionOwnerAction,
) {
    let phase = issued.action.phase();
    machine
        .resume(
            issued.receipt,
            FactoryProductionOwnerResume::Acknowledged { phase },
        )
        .unwrap();
}

#[test]
fn owner_and_child_transaction_ids_must_be_distinct() {
    let request = request(production(2, 2, 100, FactoryProductionPhase::Producing));
    assert_eq!(
        FactoryProductionOwnerMachine::preflight(owner_id(7), child_id(7), request),
        Err(FactoryProductionOwnerAdmissionRejection::TransactionIdsMustBeDistinct)
    );
}

#[test]
fn above_capacity_state_follows_retail_staffed_branch_instead_of_failing_admission() {
    let mut runtime = production(2, 3, 100, FactoryProductionPhase::Producing);
    runtime.production_progress_micros_raw = 99;
    let mut machine = machine_with(8, 9, request(runtime));
    let issued = issue(&mut machine);
    assert!(matches!(
        issued.action,
        FactoryProductionOwnerAction::Production {
            phase: FactoryProductionOwnerActionPhase::Production(
                FactoryProductionActionPhase::SpawnPickup
            ),
            ..
        }
    ));
    assert_eq!(issued.before_action.production.current_scientists_raw, 3);
}

#[test]
fn above_capacity_health_loss_preserves_capacity_effect_order() {
    let runtime = production(2, 4, 100, FactoryProductionPhase::Producing);
    let mut frame = request(runtime);
    frame.current_health_raw = 49;
    frame.elapsed_micros = 0;
    let mut machine = machine_with(12, 13, frame);

    let countdown = issue(&mut machine);
    assert!(matches!(
        countdown.action,
        FactoryProductionOwnerAction::PublishCountdownSeconds {
            phase: FactoryProductionOwnerActionPhase::PublishDamageLossCapacityCountdown,
            remaining_seconds_raw: 0,
        }
    ));
    assert_eq!(countdown.before_action.production.current_scientists_raw, 2);
    acknowledge(&mut machine, countdown);

    let text = issue(&mut machine);
    assert!(matches!(
        text.action,
        FactoryProductionOwnerAction::EmitDirectText {
            phase: FactoryProductionOwnerActionPhase::EmitDamageLossCapacityDirectText,
            direct_text_id: FACTORY_OWNER_CAPACITY_DIRECT_TEXT_ID,
            ..
        }
    ));
    acknowledge(&mut machine, text);

    let hud = issue(&mut machine);
    assert!(matches!(
        hud.action,
        FactoryProductionOwnerAction::QueueHudResource {
            phase: FactoryProductionOwnerActionPhase::QueueDamageLossCapacityHudResource,
            resource_id: FACTORY_OWNER_CAPACITY_HUD_RESOURCE_ID,
        }
    ));
    acknowledge(&mut machine, hud);
    assert_eq!(machine.production().current_scientists_raw, 1);
}

#[test]
fn cached_health_loss_uses_strict_threshold_and_retail_double_decrement() {
    let mut runtime = production(2, 2, 100, FactoryProductionPhase::Producing);
    runtime.health_loss_accumulator_raw = 1;
    let mut frame = request(runtime);
    frame.current_health_raw = 50;
    frame.elapsed_micros = 0;
    frame.suppress_status_publication = true;

    let mut machine = machine_with(1, 2, frame);
    let issued = issue(&mut machine);
    assert_eq!(issued.before_action.production.current_scientists_raw, 0);
    assert_eq!(
        issued.before_action.production.health_loss_accumulator_raw,
        0
    );
    assert_eq!(issued.before_action.production.cached_health_raw, 50);
    assert_eq!(
        issued.before_action.entry_flags_at_0x84_or_mask,
        FACTORY_OWNER_ENTRY_FLAGS_AT_0X84_OR_MASK
    );

    let mut runtime = production(2, 2, 100, FactoryProductionPhase::Producing);
    runtime.health_loss_accumulator_raw = 0;
    let mut frame = request(runtime);
    frame.current_health_raw = 50;
    frame.elapsed_micros = 0;
    frame.suppress_status_publication = true;
    let mut machine = machine_with(3, 4, frame);
    let issued = issue(&mut machine);
    assert_eq!(issued.before_action.production.current_scientists_raw, 2);
    assert_eq!(
        issued.before_action.production.health_loss_accumulator_raw,
        50
    );
}

#[test]
fn understaffed_expiry_orders_text_death_and_countdown_and_rejects_wrong_receipts() {
    let mut runtime = production(2, 1, 100, FactoryProductionPhase::Producing);
    runtime.understaffed_countdown_micros_raw = 900_000;
    let mut frame = request(runtime);
    frame.elapsed_micros = 100_000;

    let mut machine = machine_with(10, 11, frame);
    let text = issue(&mut machine);
    assert!(matches!(
        text.action,
        FactoryProductionOwnerAction::EmitDirectText {
            phase: FactoryProductionOwnerActionPhase::EmitUnderstaffedDirectText,
            direct_text_id: FACTORY_OWNER_UNDERSTAFFED_DIRECT_TEXT_ID,
            sub_parameter: 0,
        }
    ));
    assert_eq!(
        text.before_action
            .production
            .understaffed_countdown_micros_raw,
        1_000_000
    );

    let mut other = machine_with(20, 21, frame);
    let wrong = issue(&mut other);
    let failure = machine
        .resume(
            wrong.receipt,
            FactoryProductionOwnerResume::Acknowledged {
                phase: FactoryProductionOwnerActionPhase::EmitUnderstaffedDirectText,
            },
        )
        .unwrap_err();
    assert!(matches!(
        failure.error,
        FactoryProductionOwnerProtocolError::ReceiptTransactionMismatch { .. }
    ));
    assert_eq!(
        machine.poll(),
        FactoryProductionOwnerPoll::Awaiting(
            FactoryProductionOwnerActionPhase::EmitUnderstaffedDirectText
        )
    );

    acknowledge(&mut machine, text);
    let death = issue(&mut machine);
    assert!(matches!(
        death.action,
        FactoryProductionOwnerAction::InvokeUnderstaffedDeath { .. }
    ));
    acknowledge(&mut machine, death);

    let countdown = issue(&mut machine);
    assert!(matches!(
        countdown.action,
        FactoryProductionOwnerAction::PublishCountdownSeconds {
            phase: FactoryProductionOwnerActionPhase::PublishUnderstaffedCountdown,
            remaining_seconds_raw: 0,
        }
    ));
    assert_eq!(
        countdown.before_action.current_health_raw,
        PROGRESSION_REVIVE_HEALTH_RAW
    );
    assert_eq!(
        countdown.before_action.progressive_death.elapsed_micros_raw,
        1
    );
    acknowledge(&mut machine, countdown);
    let status = issue(&mut machine);
    assert!(matches!(
        status.action,
        FactoryProductionOwnerAction::PublishStatus { .. }
    ));
}

#[test]
fn owner_blocks_are_terminal_and_retain_the_durable_prefix() {
    let mut runtime = production(2, 1, 100, FactoryProductionPhase::Producing);
    runtime.understaffed_countdown_micros_raw = 900_000;
    let mut frame = request(runtime);
    frame.elapsed_micros = 100_000;
    let mut machine = machine_with(30, 31, frame);

    let issued = issue(&mut machine);
    let phase = issued.action.phase();
    machine
        .resume(
            issued.receipt,
            FactoryProductionOwnerResume::Blocked {
                phase,
                reason: FactoryProductionOwnerExternalBlock::DirectTextUnavailable,
            },
        )
        .unwrap();
    let block = match machine.poll() {
        FactoryProductionOwnerPoll::Blocked(block) => block,
        other => panic!("expected block, got {other:?}"),
    };
    assert_eq!(
        block.phase,
        FactoryProductionOwnerActionPhase::EmitUnderstaffedDirectText
    );
    assert_eq!(
        block.production.understaffed_countdown_micros_raw,
        1_000_000
    );
    assert_eq!(machine.poll(), FactoryProductionOwnerPoll::Blocked(block));
}

#[test]
fn damaged_repair_preserves_fixed_point_remainder_and_capacity_effect_order() {
    let mut runtime = production(2, 2, 100, FactoryProductionPhase::Producing);
    runtime.cached_health_raw = 99;
    runtime.repair_rate_raw = 2_048;
    runtime.repair_rate_remainder_raw = 0;
    runtime.health_loss_accumulator_raw = 5;
    runtime.understaffed_countdown_micros_raw = 500;
    let mut frame = request(runtime);
    frame.current_health_raw = 99;
    frame.elapsed_micros = 256;

    let mut machine = machine_with(40, 41, frame);
    let countdown = issue(&mut machine);
    assert!(matches!(
        countdown.action,
        FactoryProductionOwnerAction::PublishCountdownSeconds {
            phase: FactoryProductionOwnerActionPhase::PublishRepairCompleteCountdown,
            remaining_seconds_raw: 0,
        }
    ));
    assert_eq!(countdown.before_action.current_health_raw, 100);
    assert_eq!(
        countdown.before_action.production.repair_rate_remainder_raw,
        0
    );
    assert_eq!(
        countdown
            .before_action
            .production
            .health_loss_accumulator_raw,
        4
    );
    // FUN_00418C20 has clamped staffing, but its countdown write follows its
    // countdown/text calls and therefore is not committed yet.
    assert_eq!(
        countdown
            .before_action
            .production
            .understaffed_countdown_micros_raw,
        244
    );
    acknowledge(&mut machine, countdown);

    let text = issue(&mut machine);
    assert!(matches!(
        text.action,
        FactoryProductionOwnerAction::EmitDirectText {
            phase: FactoryProductionOwnerActionPhase::EmitRepairCapacityDirectText,
            direct_text_id: FACTORY_OWNER_CAPACITY_DIRECT_TEXT_ID,
            sub_parameter: 0,
        }
    ));
    acknowledge(&mut machine, text);

    let hud = issue(&mut machine);
    assert!(matches!(
        hud.action,
        FactoryProductionOwnerAction::QueueHudResource {
            phase: FactoryProductionOwnerActionPhase::QueueRepairCapacityHudResource,
            resource_id: FACTORY_OWNER_CAPACITY_HUD_RESOURCE_ID,
        }
    ));
    assert_eq!(
        hud.before_action
            .production
            .understaffed_countdown_micros_raw,
        0
    );
}

#[test]
fn full_health_staffed_owner_maps_each_child_action_once() {
    let mut runtime = production(2, 2, 100, FactoryProductionPhase::Producing);
    runtime.production_progress_micros_raw = 99;
    let frame = request(runtime);
    let mut machine = machine_with(50, 51, frame);

    let spawn = issue(&mut machine);
    assert!(spawn.receipt.is_nested_production());
    assert!(matches!(
        spawn.action,
        FactoryProductionOwnerAction::Production {
            phase: FactoryProductionOwnerActionPhase::Production(
                FactoryProductionActionPhase::SpawnPickup
            ),
            action: FactoryProductionAction::SpawnEntity { .. },
        }
    ));
    assert_eq!(
        machine.poll(),
        FactoryProductionOwnerPoll::Awaiting(FactoryProductionOwnerActionPhase::Production(
            FactoryProductionActionPhase::SpawnPickup
        ))
    );

    let failure = machine
        .resume(
            spawn.receipt,
            FactoryProductionOwnerResume::Acknowledged {
                phase: FactoryProductionOwnerActionPhase::Production(
                    FactoryProductionActionPhase::SpawnPickup,
                ),
            },
        )
        .unwrap_err();
    assert_eq!(
        failure.error,
        FactoryProductionOwnerProtocolError::ChildProtocol(
            FactoryProductionProtocolError::ResponseKindMismatch {
                phase: FactoryProductionActionPhase::SpawnPickup,
            }
        )
    );
    machine
        .resume(
            failure.receipt,
            FactoryProductionOwnerResume::SpawnCompleted {
                phase: FactoryProductionOwnerActionPhase::Production(
                    FactoryProductionActionPhase::SpawnPickup,
                ),
                result: FactoryEntitySpawnResult::Failed,
            },
        )
        .unwrap();

    let clear = issue(&mut machine);
    assert_eq!(clear.receipt.action_sequence(), 2);
    assert!(matches!(
        clear.action,
        FactoryProductionOwnerAction::Production {
            phase: FactoryProductionOwnerActionPhase::Production(
                FactoryProductionActionPhase::ClearFactoryLifetime
            ),
            action: FactoryProductionAction::ClearFactoryLifetime { .. },
        }
    ));
    acknowledge(&mut machine, clear);
    assert_eq!(
        machine.production().phase,
        FactoryProductionPhase::Delivering
    );
}

#[test]
fn progressive_death_delegates_effects_and_terminal_destruction_before_tail() {
    let runtime = production(2, 2, 100, FactoryProductionPhase::Producing);
    let mut effect_frame = request(runtime);
    effect_frame.progressive_death = ProgressiveDeathState {
        elapsed_micros_raw: 1,
        config_flags_at_0x18: 0,
    };
    effect_frame.elapsed_micros = 100_000;
    effect_frame.suppress_status_publication = true;
    let mut effect_machine = machine_with(60, 61, effect_frame);
    let effect = issue(&mut effect_machine);
    assert!(matches!(
        effect.action,
        FactoryProductionOwnerAction::DispatchProgressiveModelEffect {
            effect: ProgressiveModelEffect { threshold: 0x19 },
            ..
        }
    ));
    acknowledge(&mut effect_machine, effect);
    assert!(matches!(
        effect_machine.poll(),
        FactoryProductionOwnerPoll::Complete(_)
    ));

    let mut terminal_frame = request(runtime);
    terminal_frame.progressive_death = ProgressiveDeathState {
        elapsed_micros_raw: 100_001,
        config_flags_at_0x18: 0x20,
    };
    terminal_frame.elapsed_micros = 100_000;
    terminal_frame.animation_state_raw = 3;
    let mut terminal_machine = machine_with(62, 63, terminal_frame);
    let pickup = issue(&mut terminal_machine);
    assert_eq!(
        pickup.action,
        FactoryProductionOwnerAction::QueueDeferredDestroy {
            phase: FactoryProductionOwnerActionPhase::QueueProgressivePickupDeferredDestroy,
            entity_id: 0,
        }
    );
    assert_eq!(
        pickup.before_action.progressive_death.elapsed_micros_raw,
        -1
    );
    acknowledge(&mut terminal_machine, pickup);

    let factory = issue(&mut terminal_machine);
    assert!(matches!(
        factory.action,
        FactoryProductionOwnerAction::InvokeProgressiveFactoryDeath {
            phase: FactoryProductionOwnerActionPhase::InvokeProgressiveFactoryDeath,
            ..
        }
    ));
    acknowledge(&mut terminal_machine, factory);

    let presentation = issue(&mut terminal_machine);
    assert!(matches!(
        presentation.action,
        FactoryProductionOwnerAction::DispatchProgressiveDeathPresentation { .. }
    ));
    acknowledge(&mut terminal_machine, presentation);

    let animation = issue(&mut terminal_machine);
    assert!(matches!(
        animation.action,
        FactoryProductionOwnerAction::ApplyAnimationTransition {
            transition: v2k_game::factory_production_live::FactoryAnimationTransition {
                previous_state_raw: 3,
                next_state_raw: 0,
                ..
            },
            ..
        }
    ));
    acknowledge(&mut terminal_machine, animation);
    let completion = match terminal_machine.poll() {
        FactoryProductionOwnerPoll::Complete(completion) => completion,
        other => panic!("expected completion, got {other:?}"),
    };
    assert_eq!(completion.production.current_scientists_raw, 0);
    assert_eq!(completion.production.scientist_capacity_raw, 0);
    assert!(completion.status_published);
}

#[test]
fn progressive_factory_death_ack_commits_zero_after_revived_action_snapshots() {
    let mut runtime = production(2, 2, 100, FactoryProductionPhase::Producing);
    runtime.spawned_pickup_handle = 77;
    let mut frame = request(runtime);
    frame.current_health_raw = PROGRESSION_REVIVE_HEALTH_RAW;
    frame.progressive_death = ProgressiveDeathState {
        elapsed_micros_raw: 100_001,
        config_flags_at_0x18: 0x20,
    };
    frame.elapsed_micros = 100_000;
    frame.suppress_status_publication = true;
    let mut machine = machine_with(64, 65, frame);

    let pickup = issue(&mut machine);
    assert_eq!(
        pickup.action,
        FactoryProductionOwnerAction::QueueDeferredDestroy {
            phase: FactoryProductionOwnerActionPhase::QueueProgressivePickupDeferredDestroy,
            entity_id: 77,
        }
    );
    assert_eq!(
        pickup.before_action.current_health_raw,
        PROGRESSION_REVIVE_HEALTH_RAW
    );
    acknowledge(&mut machine, pickup);

    let factory = issue(&mut machine);
    assert!(matches!(
        factory.action,
        FactoryProductionOwnerAction::InvokeProgressiveFactoryDeath { .. }
    ));
    assert_eq!(
        factory.before_action.current_health_raw,
        PROGRESSION_REVIVE_HEALTH_RAW
    );
    acknowledge(&mut machine, factory);

    assert_eq!(machine.current_health_raw(), 0);
    let completion = match machine.poll() {
        FactoryProductionOwnerPoll::Complete(completion) => completion,
        other => panic!("expected completion, got {other:?}"),
    };
    assert_eq!(completion.current_health_raw, 0);
}

#[test]
fn stage31_effect_sees_advanced_clock_before_stage32_writes_terminal_sentinel() {
    let runtime = production(2, 2, 100, FactoryProductionPhase::Producing);
    let mut frame = request(runtime);
    frame.progressive_death = ProgressiveDeathState {
        elapsed_micros_raw: 3_099_999,
        config_flags_at_0x18: 0,
    };
    frame.elapsed_micros = 100_001;
    let mut machine = machine_with(66, 67, frame);

    let effect = issue(&mut machine);
    assert!(matches!(
        effect.action,
        FactoryProductionOwnerAction::DispatchProgressiveModelEffect {
            effect: ProgressiveModelEffect {
                threshold: 0x1_0000
            },
            ..
        }
    ));
    assert_eq!(
        effect.before_action.progressive_death.elapsed_micros_raw,
        3_200_000
    );
    acknowledge(&mut machine, effect);

    let pickup = issue(&mut machine);
    assert!(matches!(
        pickup.action,
        FactoryProductionOwnerAction::QueueDeferredDestroy {
            phase: FactoryProductionOwnerActionPhase::QueueProgressivePickupDeferredDestroy,
            ..
        }
    ));
    assert_eq!(
        pickup.before_action.progressive_death.elapsed_micros_raw,
        -1
    );
}

#[test]
fn nested_block_reason_must_match_the_child_protocol() {
    let mut runtime = production(2, 2, 100, FactoryProductionPhase::Producing);
    runtime.production_progress_micros_raw = 99;
    let mut machine = machine_with(70, 71, request(runtime));
    let spawn = issue(&mut machine);
    let failure = machine
        .resume(
            spawn.receipt,
            FactoryProductionOwnerResume::Blocked {
                phase: FactoryProductionOwnerActionPhase::Production(
                    FactoryProductionActionPhase::SpawnPickup,
                ),
                reason: FactoryProductionOwnerExternalBlock::DirectTextUnavailable,
            },
        )
        .unwrap_err();
    assert!(matches!(
        failure.error,
        FactoryProductionOwnerProtocolError::ResponseKindMismatch { .. }
    ));
    machine
        .resume(
            failure.receipt,
            FactoryProductionOwnerResume::Blocked {
                phase: FactoryProductionOwnerActionPhase::Production(
                    FactoryProductionActionPhase::SpawnPickup,
                ),
                reason: FactoryProductionOwnerExternalBlock::Production(
                    FactoryProductionExternalBlock::EntitySpawnUnavailable,
                ),
            },
        )
        .unwrap();
    assert!(matches!(
        machine.poll(),
        FactoryProductionOwnerPoll::Blocked(_)
    ));
}

#[test]
fn nested_state_commit_failure_is_a_durable_outer_block() {
    let mut runtime = production(2, 2, 100, FactoryProductionPhase::Producing);
    runtime.production_progress_micros_raw = 99;
    let mut machine = machine_with(80, 81, request(runtime));
    let spawn = issue(&mut machine);
    let phase = spawn.action.phase();
    machine
        .resume(
            spawn.receipt,
            FactoryProductionOwnerResume::Blocked {
                phase,
                reason: FactoryProductionOwnerExternalBlock::FactoryStateCommitUnavailable,
            },
        )
        .unwrap();
    let block = match machine.poll() {
        FactoryProductionOwnerPoll::Blocked(block) => block,
        other => panic!("expected durable block, got {other:?}"),
    };
    assert_eq!(block.phase, phase);
    assert_eq!(
        block.reason,
        FactoryProductionOwnerExternalBlock::FactoryStateCommitUnavailable
    );
}
