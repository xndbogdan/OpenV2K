use super::*;
use crate::session::GameSession;
use v2k_formats::terrain::GRID_SIZE;

fn object_cells(session: &GameSession) -> Vec<([u8; 2], u32)> {
    let objects = session.cache.terrain_objects().unwrap();
    session
        .cache
        .level_terrain()
        .unwrap()
        .cells
        .iter()
        .enumerate()
        .filter(|(_, cell)| cell.attribute != 0)
        .filter_map(|(index, cell)| {
            let kind = objects.records.get(usize::from(cell.attribute))?.kind_index;
            Some(([(index / GRID_SIZE) as u8, (index % GRID_SIZE) as u8], kind))
        })
        .collect()
}

fn covered(session: &GameSession, cell: [u8; 2]) -> bool {
    let linear = usize::from(cell[1]) * GRID_SIZE + usize::from(cell[0]);
    let packed = session
        .cache
        .level_terrain_radar()
        .unwrap()
        .packed_coverage()[linear >> 1];
    (if linear & 1 == 0 {
        packed & 0x0F
    } else {
        packed >> 4
    }) != 0
}

/// `FUN_00427C30` from the kind table: a tree infected emits its class 5 at
/// the cell's X centre, 0x80 short of the low Z edge and half the infected
/// model's extent up; cured, its class 6 at the cell centre on the ground.
#[v2k_test_support::retail_test]
fn an_infected_or_cured_object_emits_its_kinds_carriers() {
    let (mut session, _manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(13);
    let (cell, kind) = object_cells(&session)
        .into_iter()
        .find(|&(cell, kind)| {
            let words = static_kind_descriptor(kind).unwrap().raw_words();
            words[6] == 5
                && words[7] == 6
                && session
                    .cache
                    .level_terrain()
                    .unwrap()
                    .cell(cell[0].into(), cell[1].into())
                    .unwrap()
                    .terrain_type
                    & 0x18
                    == 0
        })
        .expect("a world-13 object whose kind spreads both carriers");
    let words = *static_kind_descriptor(kind).unwrap().raw_words();
    let ground = session
        .cache
        .level_terrain()
        .unwrap()
        .cell(cell[0].into(), cell[1].into())
        .copied()
        .unwrap();
    let height = i16::from(ground.height as i8) * 0x20;
    let object = &session.cache.terrain_objects().unwrap().records[usize::from(ground.attribute)];
    let extent = session
        .cache
        .global_model(usize::from(object.model_id_for(0x10)))
        .unwrap()
        .radius;
    let x = (u16::from(cell[0]) << 8) as i16;
    let z = (u16::from(cell[1]) << 8) as i16;

    // The births are exactly the expected request's, under the same pacing.
    let mut twin = fx.fork_for_main_base_abort_transaction();
    twin.emit_static_object_infection_effect_raw(
        [x + 0x80, height + (extent >> 1) as i16, z - 0x80],
        5,
        words[4],
    );
    let before = fx.particle_count();
    let infect = InfectionCellWrite::at_cell(cell, true);
    assert_eq!(
        write_infection(&mut session.cache, &mut fx, infect),
        Ok(true)
    );
    assert!(fx.particle_count() > before);
    assert_eq!(
        fx.test_particles_in_virgin_birth_order(),
        twin.test_particles_in_virgin_birth_order()
    );
    assert!(fx
        .test_particles_in_virgin_birth_order()
        .iter()
        .all(|particle| particle.source_class == 5
            && particle.owner_id == Some(0)
            && !particle.suppresses_impact_damage));

    // An unchanged bit notifies nothing.
    let count = fx.particle_count();
    assert_eq!(
        write_infection(&mut session.cache, &mut fx, infect),
        Ok(false)
    );
    assert_eq!(fx.particle_count(), count);

    let mut twin = fx.fork_for_main_base_abort_transaction();
    twin.emit_static_object_infection_effect_raw([x + 0x80, height, z + 0x80], 6, words[4]);
    let cure = InfectionCellWrite::at_cell(cell, false);
    assert_eq!(write_infection(&mut session.cache, &mut fx, cure), Ok(true));
    assert!(fx.particle_count() > count);
    assert_eq!(
        fx.test_particles_in_virgin_birth_order(),
        twin.test_particles_in_virgin_birth_order()
    );
}

/// A network client's carriers are born harmless.
#[v2k_test_support::retail_test]
fn a_network_clients_object_carriers_are_harmless() {
    let (mut session, _manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(13);
    let (cell, _) = object_cells(&session)
        .into_iter()
        .find(|&(_, kind)| static_kind_descriptor(kind).unwrap().raw_words()[7] == 6)
        .unwrap();
    session
        .cache
        .apply_level_infection_writes(&[InfectionCellWrite::at_cell(cell, true)])
        .unwrap();
    fx.set_network_client(true);
    write_infection(
        &mut session.cache,
        &mut fx,
        InfectionCellWrite::at_cell(cell, false),
    )
    .unwrap();
    let born = fx.test_particles_in_virgin_birth_order();
    assert!(!born.is_empty());
    assert!(born
        .iter()
        .all(|particle| particle.source_class == 6 && particle.suppresses_impact_damage));
}

/// `FUN_0044A850` redraws the written cell: one shared word when the radar
/// covers it, none otherwise.
#[v2k_test_support::retail_test]
fn the_radar_redraw_draws_one_word_for_a_covered_cell() {
    for level in 13..=48 {
        let (mut session, _manager, mut fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(level);
        session
            .cache
            .initialize_level_terrain_radar(&mut || 0)
            .unwrap();
        let objects = object_cells(&session);
        let bare = |cell: [u8; 2]| !objects.iter().any(|&(object, _)| object == cell);
        let mut cells = (0..=255u8).flat_map(|x| (0..=255u8).map(move |z| [x, z]));
        let Some(lit) = cells
            .clone()
            .find(|&cell| covered(&session, cell) && bare(cell))
        else {
            continue;
        };
        let dark = cells
            .find(|&cell| !covered(&session, cell) && bare(cell))
            .unwrap();

        let state = fx.shared_retail_random_state();
        let revision = session.cache.level_terrain_radar().unwrap().revision();
        write_infection(
            &mut session.cache,
            &mut fx,
            InfectionCellWrite::at_cell(dark, true),
        )
        .unwrap();
        assert_eq!(fx.shared_retail_random_state(), state);
        assert_ne!(
            session.cache.level_terrain_radar().unwrap().revision(),
            revision
        );

        let mut expected = fx.shared_retail_random_state();
        crate::retail_rng::retail_random_u16(&mut expected);
        write_infection(
            &mut session.cache,
            &mut fx,
            InfectionCellWrite::at_cell(lit, true),
        )
        .unwrap();
        assert_eq!(fx.shared_retail_random_state(), expected);
        return;
    }
    panic!("no world has radar coverage");
}
