use std::num::NonZeroU32;

use v2k_game::entity_pair_callbacks::{
    EntityPairTaggedEffect, LifterSourceSnapshot, PairBehaviorCallbackReturn,
    PairCallbackEntitySnapshot,
};
use v2k_game::factory_delivery::{
    FactoryDeliveryAction, FactoryDeliveryFactorySnapshot, FactoryDeliveryMachine,
    FactoryDeliveryPhase, FactoryDeliveryPoll, FactoryDeliveryRequest, FactoryDeliveryResume,
    FactoryDeliveryScientistSnapshot, FactoryDeliveryTransactionId,
};
use v2k_game::factory_production::{
    FactoryProductionPhase, FactoryProductionRuntime, FactorySection13Config, FACTORY_CONFIG_BYTES,
    FACTORY_OUTPUT_CONVERSION_GATE_RAW,
};
use v2k_game::factory_production_live::{
    FactoryEntitySpawnRequest, FactoryEntitySpawnResult, FactoryPickupPresence,
    FactoryProductionAction, FactoryProductionEntityVersion, FactoryProductionFrameRequest,
    FactoryProductionMachine, FactoryProductionPoll, FactoryProductionResume,
    FactoryProductionTransactionId, FactorySpawnRole, FactorySpawnedEntity,
};

const FACTORY_ID: u32 = 0x04A4_0001;
const SCIENTIST_ID: u32 = 0x0494_0001;
const PICKUP_ID: u32 = 0x0493_0001;
const FACTORY_POSITION_RAW: [i16; 3] = [0x5700, -0x0300, 0x3A00];
const SCIENTIST_POSITION_RAW: [i16; 3] = [0x4FB7, -0x0300, 0x3920];
const FRAME_MICROS: u32 = 20_000;

fn write_i32(raw: &mut [u8; FACTORY_CONFIG_BYTES], offset: usize, value: i32) {
    raw[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn level_one_factory_runtime() -> FactoryProductionRuntime {
    let mut raw = [0_u8; FACTORY_CONFIG_BYTES];
    write_i32(&mut raw, 0x00, 0x0001_F412);
    write_i32(&mut raw, 0x04, 2);
    write_i32(&mut raw, 0x08, 6_000_000);
    write_i32(&mut raw, 0x0C, 5_000_000);
    write_i32(&mut raw, 0x10, -1_000);
    write_i32(&mut raw, 0x14, 0);
    FactoryProductionRuntime::from_retail_template(FactorySection13Config::decode(&raw), 99_999)
}

fn transaction(raw: u64) -> FactoryProductionTransactionId {
    FactoryProductionTransactionId::new(raw).expect("test transaction is nonzero")
}

#[derive(Debug, Default)]
struct FrameObservations {
    pickup_request: Option<FactoryEntitySpawnRequest>,
    converted_output_spawns: usize,
    materialiser_spawns: usize,
}

fn drive_production_frame(
    production: FactoryProductionRuntime,
    animation_state_raw: u32,
    transaction_raw: u64,
    spawn_serial: &mut u32,
) -> (FactoryProductionRuntime, u32, FrameObservations) {
    let request = FactoryProductionFrameRequest {
        factory: FactoryProductionEntityVersion {
            entity_id: FACTORY_ID,
            allocation_identity: 1,
            state_version: transaction_raw,
        },
        position_raw: FACTORY_POSITION_RAW,
        pickup_spawn_offset_raw: [0, 0, 0],
        current_health_raw: 99_999,
        maximum_health_raw: 99_999,
        progressive_death_elapsed_raw: 0,
        production,
        animation_state_raw,
        elapsed_micros: FRAME_MICROS,
        world_style_raw: 1,
        phase1_presentation_enabled: false,
        suppress_status_publication: false,
    };
    let mut machine =
        FactoryProductionMachine::preflight(transaction(transaction_raw), request).unwrap();
    let mut observations = FrameObservations::default();

    loop {
        match machine.poll() {
            FactoryProductionPoll::Action(issued) => {
                let phase = issued.action.phase();
                let response = match issued.action {
                    FactoryProductionAction::SpawnEntity { role, request, .. } => {
                        let handle = match role {
                            FactorySpawnRole::Pickup => {
                                observations.pickup_request = Some(request);
                                PICKUP_ID
                            }
                            FactorySpawnRole::ConvertedOutput => {
                                observations.converted_output_spawns += 1;
                                *spawn_serial = spawn_serial.wrapping_add(1);
                                *spawn_serial
                            }
                            FactorySpawnRole::Materialiser => {
                                observations.materialiser_spawns += 1;
                                *spawn_serial = spawn_serial.wrapping_add(1);
                                *spawn_serial
                            }
                        };
                        FactoryProductionResume::SpawnCompleted {
                            phase,
                            result: FactoryEntitySpawnResult::Spawned(FactorySpawnedEntity {
                                entity_id: NonZeroU32::new(handle).unwrap(),
                                allocation_identity: u64::from(handle),
                            }),
                        }
                    }
                    FactoryProductionAction::QueryPickupPresence { .. } => {
                        FactoryProductionResume::PickupPresence {
                            phase,
                            presence: FactoryPickupPresence::Active,
                        }
                    }
                    _ => FactoryProductionResume::Acknowledged { phase },
                };
                machine.resume(issued.receipt, response).unwrap();
            }
            FactoryProductionPoll::Complete(done) => {
                return (done.production, done.animation_state_raw, observations);
            }
            FactoryProductionPoll::Awaiting(phase) => {
                panic!("test adapter left {phase:?} awaiting a response")
            }
            FactoryProductionPoll::Blocked(block) => {
                panic!("complete test adapter unexpectedly blocked: {block:?}")
            }
        }
    }
}

#[test]
fn accepted_capture_final_scientist_drives_the_exact_first_product_timeline() {
    // 20260731-001224-second-scientist-factory-activation-v2.txt:
    // type-8 0x04940001 reaches type-66 0x04A40001 at tick 0xC7E,
    // changing staff 1 -> 2 before deferred destruction.
    let mut production = level_one_factory_runtime();
    production.current_scientists_raw = 1;
    let request = FactoryDeliveryRequest {
        factory: FactoryDeliveryFactorySnapshot {
            lifter: LifterSourceSnapshot {
                id: FACTORY_ID,
                state_flags_raw: 1,
            },
            allocation_identity: 1,
            state_version: 1,
            type_extension_present: true,
            behavior_state_present: true,
            delivery_enabled: true,
            production,
        },
        scientist: FactoryDeliveryScientistSnapshot {
            entity: PairCallbackEntitySnapshot {
                id: SCIENTIST_ID,
                position_raw: SCIENTIST_POSITION_RAW,
                state_flags_raw: 1,
                capability_flags_raw: 0x400,
            },
            allocation_identity: 2,
            state_version: 1,
        },
    };
    let mut delivery =
        FactoryDeliveryMachine::preflight(FactoryDeliveryTransactionId::new(1).unwrap(), request)
            .unwrap();
    let mut delivery_phases = Vec::new();
    loop {
        match delivery.poll() {
            FactoryDeliveryPoll::Action(issued) => {
                delivery_phases.push(issued.action.phase());
                if let FactoryDeliveryAction::CommitStaffing { commit, .. } = &issued.action {
                    assert_eq!(commit.before.current_scientists_raw, 1);
                    assert_eq!(commit.after.current_scientists_raw, 2);
                    assert!(commit.change.reached_capacity);
                }
                let phase = issued.action.phase();
                delivery
                    .resume(
                        issued.receipt,
                        FactoryDeliveryResume::Acknowledged { phase },
                    )
                    .unwrap();
            }
            FactoryDeliveryPoll::Complete(done) => {
                production = done.factory_runtime;
                assert_eq!(
                    done.callback_return,
                    PairBehaviorCallbackReturn::TaggedA300(EntityPairTaggedEffect::LifterAccepted)
                );
                break;
            }
            other => panic!("unexpected delivery state: {other:?}"),
        }
    }
    assert_eq!(
        delivery_phases,
        [
            FactoryDeliveryPhase::EmitOperation33,
            FactoryDeliveryPhase::QueueDeliveryHudResource,
            FactoryDeliveryPhase::CommitStaffing,
            FactoryDeliveryPhase::QueueCapacityHudResource,
            FactoryDeliveryPhase::QueueDeferredScientistDestroy,
        ]
    );

    // The accepted capture records 300 50-Hz ticks from intake (0xC7E) to
    // type-61 spawn (0xDAA), then 250 ticks to delivery completion (0xEA4).
    let mut animation_state_raw = 0;
    let mut spawn_serial = 0x0500_0000;
    for frame in 1..=299_u64 {
        let (next, animation, observed) =
            drive_production_frame(production, animation_state_raw, frame, &mut spawn_serial);
        production = next;
        animation_state_raw = animation;
        assert!(observed.pickup_request.is_none());
    }
    assert_eq!(production.production_progress_micros_raw, 5_980_000);
    assert_eq!(production.phase, FactoryProductionPhase::Producing);

    let (next, animation, observed) =
        drive_production_frame(production, animation_state_raw, 300, &mut spawn_serial);
    production = next;
    animation_state_raw = animation;
    assert_eq!(
        observed.pickup_request,
        Some(FactoryEntitySpawnRequest {
            requested_handle:
                v2k_game::factory_production_live::FactoryRequestedEntityHandle::Allocate,
            entity_type: 0x3D,
            position_raw: FACTORY_POSITION_RAW,
            spawn_parameter_6: 0x0001_F412,
        })
    );
    assert_eq!(production.spawned_pickup_handle, PICKUP_ID);
    assert_eq!(production.remaining_stock_raw, 0);
    assert_eq!(production.production_progress_micros_raw, 6_000_000);
    assert_eq!(production.phase, FactoryProductionPhase::Delivering);
    assert_eq!(animation_state_raw, 3);

    for frame in 301..=549_u64 {
        let (next, animation, observed) =
            drive_production_frame(production, animation_state_raw, frame, &mut spawn_serial);
        production = next;
        animation_state_raw = animation;
        assert!(observed.pickup_request.is_none());
    }
    assert_eq!(production.delivery_progress_micros_raw, 4_980_000);
    assert_eq!(production.phase, FactoryProductionPhase::Delivering);

    let (production, animation_state_raw, observed) =
        drive_production_frame(production, animation_state_raw, 550, &mut spawn_serial);
    assert!(observed.pickup_request.is_none());
    assert_eq!(production.delivery_progress_micros_raw, 5_000_000);
    assert_eq!(production.phase, FactoryProductionPhase::WaitingForPickup);
    assert_eq!(animation_state_raw, 1);
}

#[test]
fn finite_level_one_factory_ejects_both_staff_after_the_strict_static_gate() {
    let mut production = level_one_factory_runtime();
    production.current_scientists_raw = 2;
    production.remaining_stock_raw = 0;
    production.production_progress_micros_raw = production.production_threshold_micros_raw;
    production.delivery_progress_micros_raw = production.delivery_duration_micros_raw;
    production.phase = FactoryProductionPhase::WaitingForPickup;
    let mut animation_state_raw = 1;
    let mut spawn_serial = 0x0500_0000;

    // FUN_00419010 compares the signed difference against strict 0x3567E1
    // before adding this frame's elapsed time, so 176 20-ms frames only cross
    // the gate; the 177th frame performs the first conversion.
    for frame in 1..=176_u64 {
        let (next, animation, observed) =
            drive_production_frame(production, animation_state_raw, frame, &mut spawn_serial);
        production = next;
        animation_state_raw = animation;
        assert_eq!(observed.converted_output_spawns, 0);
        assert_eq!(observed.materialiser_spawns, 0);
    }
    assert_eq!(
        production.production_progress_micros_raw,
        production.production_threshold_micros_raw + 3_520_000
    );
    assert!(
        production.production_progress_micros_raw - production.production_threshold_micros_raw
            >= FACTORY_OUTPUT_CONVERSION_GATE_RAW
    );

    let (next, animation, observed) =
        drive_production_frame(production, animation_state_raw, 177, &mut spawn_serial);
    production = next;
    animation_state_raw = animation;
    assert_eq!(observed.converted_output_spawns, 1);
    assert_eq!(observed.materialiser_spawns, 1);
    assert_eq!(production.current_scientists_raw, 1);
    assert_eq!(production.scientist_capacity_raw, 1);
    assert_eq!(production.phase, FactoryProductionPhase::WaitingForPickup);

    let (next, animation, observed) =
        drive_production_frame(production, animation_state_raw, 178, &mut spawn_serial);
    production = next;
    animation_state_raw = animation;
    assert_eq!(observed.converted_output_spawns, 1);
    assert_eq!(observed.materialiser_spawns, 1);
    assert_eq!(production.current_scientists_raw, 0);
    assert_eq!(production.scientist_capacity_raw, 0);

    let (production, _, observed) =
        drive_production_frame(production, animation_state_raw, 179, &mut spawn_serial);
    assert_eq!(observed.converted_output_spawns, 0);
    assert_eq!(observed.materialiser_spawns, 0);
    assert_eq!(production.phase, FactoryProductionPhase::IdleEmpty);
    assert_eq!(production.production_progress_micros_raw, 0);
}
