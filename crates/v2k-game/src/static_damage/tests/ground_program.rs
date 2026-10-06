use super::*;

#[test]
fn fireball_ground_program_is_the_exact_two_record_cell_only_program() {
    let program = static_program(FIREBALL_GROUND_PROGRAM_VA).unwrap();
    assert_eq!(
        program
            .records()
            .iter()
            .map(|record| *record.raw_words())
            .collect::<Vec<_>>(),
        [[0, 11, 1], [0, 0, 0]]
    );
    assert!(
        static_program(0x004c_9910).is_none(),
        "following pointer is not a program"
    );

    let mut scheduler = StaticDamageScheduler::new();
    let cell = [255, 0];
    assert_eq!(
        scheduler.submit_fireball_ground_program(cell),
        StaticGroundProgramOutcome::Started
    );
    assert_eq!(scheduler.active_program_count(), 1);
    // A missing static descriptor/model is allowed by both records. Neither
    // zero elapsed time nor unknown optional static data suppresses opcode11.
    assert_eq!(
        scheduler.advance(0, |_| None),
        [StaticDamageAction::LowerTerrainLight { cell, amount: 1 }]
    );
    assert_eq!(scheduler.active_program_count(), 0);
    assert!(scheduler
        .advance(0, |_| panic!("retired program looked up a static"))
        .is_empty());
}

#[test]
fn fireball_ground_program_shares_dedup_with_hit_crater_and_in_flight_nodes() {
    let cell = [12, 34];
    let mut scheduler = StaticDamageScheduler::new();
    assert_eq!(
        scheduler.submit_crater_destruction(cell, KIND_1_STATIC_OBJECT),
        StaticCraterOutcome::Started
    );
    assert_eq!(
        scheduler.submit_fireball_ground_program(cell),
        StaticGroundProgramOutcome::Duplicate
    );
    scheduler.advance(500_000, |_| {
        Some(target(cell, KIND_1_STATIC_OBJECT, 0, 0, 16).state)
    });

    assert_eq!(
        scheduler.submit_fireball_ground_program(cell),
        StaticGroundProgramOutcome::Started
    );
    assert!(matches!(
        scheduler.submit_hit(
            target(cell, KIND_1_STATIC_OBJECT, 0, 0, 16),
            DamagePacket::collision(5_000),
            &mut || panic!("deterministic hit drew RNG")
        ),
        StaticDamageOutcome::Duplicate { .. }
    ));
    assert_eq!(
        scheduler.submit_crater_destruction(cell, KIND_1_STATIC_OBJECT),
        StaticCraterOutcome::Duplicate
    );
    scheduler.begin_advance(40_000);
    let batch = scheduler.next_node_batch(|_| None).unwrap();
    assert_eq!(
        scheduler.submit_fireball_ground_program(cell),
        StaticGroundProgramOutcome::Duplicate
    );
    let (token, actions) = batch.into_parts();
    assert_eq!(
        actions,
        [StaticDamageAction::LowerTerrainLight { cell, amount: 1 }]
    );
    assert!(scheduler.complete_node_batch(token));
    assert!(scheduler.next_node_batch(|_| None).is_none());
    assert_eq!(
        scheduler.submit_fireball_ground_program(cell),
        StaticGroundProgramOutcome::Started
    );
}

#[test]
fn fireball_ground_program_appended_during_static_pass_waits_and_keeps_fifo_order() {
    let mut scheduler = StaticDamageScheduler::new();
    let cells = [[255, 0], [0, 255], [255, 255]];
    for cell in &cells[..2] {
        assert_eq!(
            scheduler.submit_fireball_ground_program(*cell),
            StaticGroundProgramOutcome::Started
        );
    }
    scheduler.begin_advance(125_000);
    let first = scheduler.next_node_batch(|_| None).unwrap();
    assert_eq!(first.source_cell(), cells[0]);
    assert_eq!(
        scheduler.submit_fireball_ground_program(cells[2]),
        StaticGroundProgramOutcome::Started
    );
    assert!(scheduler.complete_node_batch(first.into_parts().0));
    let second = scheduler.next_node_batch(|_| None).unwrap();
    assert_eq!(second.source_cell(), cells[1]);
    assert!(scheduler.complete_node_batch(second.into_parts().0));
    assert!(scheduler.next_node_batch(|_| None).is_none());
    assert_eq!(scheduler.active_program_count(), 1);
    assert_eq!(
        scheduler.advance(0, |_| None),
        [StaticDamageAction::LowerTerrainLight {
            cell: cells[2],
            amount: 1
        }]
    );
}
