use super::*;

#[test]
fn kind2_exact_descriptor_filters_both_slots_before_its_zero_chance_gate() {
    // Read-only retail PE E9BE7A83: 4C9E90 descriptor and4C9C58 profile.
    //11760 submits [1,0,impact,0,type,owner];27950 passes the first four
    //words to255E0 before testing severity, chance or the null program.
    let descriptor = static_kind_descriptor(KIND_2_STATIC_OBJECT).unwrap();
    assert_eq!(
        *descriptor.raw_words(),
        [0x004c9c58, 0, 0, 0, 10_000, 0, 0, 0, 1, 0x004e2037, 0x0000ff00]
    );
    let catalog = admitted_static_catalog(KIND_2_STATIC_OBJECT).unwrap();
    assert_eq!(
        catalog.profile.thresholds_raw,
        [0, 4000, 4000, 4000, 4000, 0, 0]
    );
    assert_eq!(
        catalog.profile.multipliers_q8,
        [0, 256, 256, 256, 256, 0, 0]
    );
    assert_eq!((catalog.chance_gate_raw, catalog.program_va), (0, 0));
    let mut scheduler = StaticDamageScheduler::new();
    let static_target = target([117, 73], KIND_2_STATIC_OBJECT, 0x43, 0, 100);
    for packet in [
        DamagePacket::collision(0),
        DamagePacket::collision(3999),
        DamagePacket::collision(4000),
        crate::damage::PRIMARY_PROJECTILE_DAMAGE_PACKET,
        crate::damage::BALLISTIC_PARTICLE_DAMAGE_PACKET,
    ] {
        assert_eq!(
            scheduler.submit_hit(static_target, packet, &mut || panic!("zero severity")),
            StaticDamageOutcome::NoDamage { severity_raw: 0 }
        );
    }
    for (packet, severity_raw) in [
        (DamagePacket::collision(4001), 1),
        (
            DamagePacket {
                channels: [2, 4],
                amounts_raw: [4001, 4002],
            },
            3,
        ),
    ] {
        assert_eq!(
            scheduler.submit_hit(static_target, packet, &mut || panic!("zero chance gate")),
            StaticDamageOutcome::ImmediateBurn {
                cell: static_target.cell,
                severity_raw,
                sample: None
            }
        );
    }
    assert_eq!(scheduler.active_program_count(), 0);
    // Similar catalogue rows remain unadmitted; this is exactly kind2.
    assert!(!static_damage_kind_is_admitted(5));
}

#[test]
fn kind2_burned_halving_and_existing_cell_preserve_null_program_order() {
    let mut scheduler = StaticDamageScheduler::new();
    let cell = [117, 73];
    scheduler.submit_fireball_ground_program(cell);
    let unburned = target(cell, KIND_2_STATIC_OBJECT, 0x43, 0, 100);
    assert_eq!(
        scheduler.submit_hit(unburned, DamagePacket::collision(4003), &mut || panic!(
            "zero gate"
        )),
        StaticDamageOutcome::ImmediateBurn {
            cell,
            severity_raw: 3,
            sample: None
        }
    );
    assert_eq!(
        scheduler.active_program_count(),
        1,
        "immediate ignition precedes cell dedup"
    );
    let burned = StaticDamageTarget {
        state: StaticDamageTargetState {
            terrain_type: 0x4b,
            ..unburned.state
        },
        ..unburned
    };
    assert_eq!(
        scheduler.submit_hit(burned, DamagePacket::collision(4001), &mut || panic!(
            "halved to zero"
        )),
        StaticDamageOutcome::NoDamage { severity_raw: 0 }
    );
    assert_eq!(
        scheduler.submit_hit(burned, DamagePacket::collision(4003), &mut || panic!(
            "zero gate"
        )),
        StaticDamageOutcome::BurnedIgnored {
            severity_raw: 1,
            sample: None
        }
    );
    assert_eq!(scheduler.active_program_count(), 1);
}

#[test]
fn completed_world_hive_burns_kind22_without_sampling_or_a_program_node() {
    let descriptor = static_kind_descriptor(KIND_22_STATIC_OBJECT).unwrap();
    assert_eq!(descriptor.damage_profile_va(), 0x004c_9bb0);
    assert_eq!(descriptor.chance_gate_raw(), 8_000);
    assert_eq!(descriptor.timed_program_va(), 0);
    let mut scheduler = StaticDamageScheduler::new();
    // Overlay14's infected, unburned cell reached by 425F60 during Slot00 load.
    let cell = [195, 110];
    let infected = target(cell, KIND_22_STATIC_OBJECT, 0x73, 0, 0);
    let packet = DamagePacket {
        channels: [1, 4],
        amounts_raw: [10_000, 8_000],
    };
    assert_eq!(
        scheduler.submit_hit(infected, packet, &mut || panic!("severity exceeds gate")),
        StaticDamageOutcome::ImmediateBurn {
            cell,
            severity_raw: 10_000,
            sample: None
        }
    );
    assert_eq!(scheduler.active_program_count(), 0);
    // Infected (0x10) does not halve damage; burned (0x08) does, before RNG.
    let burned = StaticDamageTarget {
        state: StaticDamageTargetState {
            terrain_type: 0x7b,
            ..infected.state
        },
        ..infected
    };
    assert!(matches!(
        scheduler.submit_hit(burned, packet, &mut || 0),
        StaticDamageOutcome::BurnedIgnored {
            severity_raw: 5_000,
            sample: Some(_)
        }
    ));
}

#[test]
fn kind4_filter_and_chance_precede_its_null_program_burn_without_a_fifo_node() {
    let descriptor = static_kind_descriptor(KIND_4_STATIC_OBJECT).unwrap();
    assert_eq!(descriptor.damage_profile_va(), 0x004c_9de0);
    assert_eq!(descriptor.chance_gate_raw(), 500);
    assert_eq!(descriptor.timed_program_va(), 0);
    assert!(static_damage_kind_is_admitted(KIND_4_STATIC_OBJECT));
    let mut scheduler = StaticDamageScheduler::new();
    let unburned = target([23, 45], KIND_4_STATIC_OBJECT, 0, 0, 300);
    assert_eq!(
        scheduler.submit_hit(unburned, DamagePacket::collision(4000), &mut || panic!(
            "threshold hit"
        )),
        StaticDamageOutcome::NoDamage { severity_raw: 0 }
    );
    // Severity250 samples gate500. Equality accepts and one greater rejects.
    for (roll, accepted) in [(32767, true), (32768, false)] {
        let mut draws = 0;
        let result = scheduler.submit_hit(unburned, DamagePacket::collision(4250), &mut || {
            draws += 1;
            roll
        });
        let sample = StaticDamageChanceSample {
            cutoff: 32767,
            roll,
        };
        assert_eq!(
            result,
            if accepted {
                StaticDamageOutcome::ImmediateBurn {
                    cell: unburned.cell,
                    severity_raw: 250,
                    sample: Some(sample),
                }
            } else {
                StaticDamageOutcome::ChanceRejected {
                    severity_raw: 250,
                    sample,
                }
            }
        );
        assert_eq!(draws, 1);
        assert_eq!(scheduler.active_program_count(), 0);
    }
    for packet in [
        DamagePacket::collision(4500),
        DamagePacket {
            channels: [2, 0],
            amounts_raw: [2000, 0],
        },
    ] {
        let result =
            scheduler.submit_hit(unburned, packet, &mut || panic!("deterministic severity"));
        assert!(matches!(
            result,
            StaticDamageOutcome::ImmediateBurn { sample: None, .. }
        ));
        assert_eq!(scheduler.active_program_count(), 0);
    }
}

#[test]
fn kind4_burned_halving_still_samples_and_immediate_burn_bypasses_cell_dedup() {
    let mut scheduler = StaticDamageScheduler::new();
    let cell = [23, 45];
    assert_eq!(
        scheduler.submit_fireball_ground_program(cell),
        StaticGroundProgramOutcome::Started
    );
    let unburned = target(cell, KIND_4_STATIC_OBJECT, 0, 0, 300);
    assert_eq!(
        scheduler.submit_hit(unburned, DamagePacket::collision(4500), &mut || panic!(
            "gate equality"
        )),
        StaticDamageOutcome::ImmediateBurn {
            cell,
            severity_raw: 500,
            sample: None
        }
    );
    assert_eq!(
        scheduler.active_program_count(),
        1,
        "the existing node is neither replaced nor duplicated"
    );
    let burned = StaticDamageTarget {
        state: StaticDamageTargetState {
            terrain_type: 0x08,
            ..unburned.state
        },
        ..unburned
    };
    let mut draws = 0;
    assert_eq!(
        scheduler.submit_hit(burned, DamagePacket::collision(4500), &mut || {
            draws += 1;
            0
        }),
        StaticDamageOutcome::BurnedIgnored {
            severity_raw: 250,
            sample: Some(StaticDamageChanceSample {
                cutoff: 32767,
                roll: 0
            }),
        }
    );
    assert_eq!(draws, 1);
    assert_eq!(scheduler.active_program_count(), 1);
}
