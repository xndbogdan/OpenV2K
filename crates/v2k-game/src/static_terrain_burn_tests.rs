use super::*;
use crate::{
    session::GameSession,
    static_damage::{StaticCraterOutcome, StaticDamageAction},
    static_damage_live::resolve_current_static_damage_target,
};
use v2k_formats::{levels::parse_level, terrain::GRID_SIZE};

// Independent V200001 oracle, second 436E00 from 427899 at tick436.
// X-major x133..151, z115..133. It already includes the earlier hut crater.
// 20260919-intro2-second-crater-ttd in the accepted capture ledger records
// origin[-29056,-224,31872], radius2048, depth20, sea=-847.
const BEFORE: [[i8; 19]; 19] = [
    [
        -29, -30, -30, -31, -30, -27, -24, -22, -19, -18, -17, -10, -11, -2, 1, 0, -3, -8, -12,
    ],
    [
        -30, -31, -31, -31, -29, -24, -18, -18, -18, -15, -14, -10, -11, -3, -1, -1, -1, -5, -8,
    ],
    [
        -31, -31, -31, -29, -25, -18, -14, -16, -14, -7, -11, -10, -10, -5, -2, -1, 2, 0, -4,
    ],
    [
        -31, -31, -29, -26, -21, -17, -11, -10, -7, -4, -6, -9, -8, -6, -4, -1, 3, 3, 2,
    ],
    [
        -30, -29, -26, -22, -17, -16, -6, -4, -4, -4, -6, -8, -6, -6, -6, -3, -2, 4, 4,
    ],
    [
        -28, -27, -24, -21, -17, -14, -10, -7, -6, -6, -7, -7, -7, -7, -7, -6, -3, 2, 6,
    ],
    [
        -25, -24, -22, -20, -17, -14, -12, -10, -8, -8, -8, -8, -8, -8, -7, -6, -6, 1, 10,
    ],
    [
        -23, -22, -21, -19, -17, -15, -13, -12, -10, -9, -9, -5, -6, -6, -4, -2, -5, 1, 6,
    ],
    [
        -21, -20, -20, -18, -17, -15, -14, -12, -11, -10, -7, -10, -13, -14, -12, -8, -2, -1, 3,
    ],
    [
        -19, -19, -19, -18, -17, -15, -14, -13, -12, -7, -11, -16, -20, -22, -19, -14, -8, -1, -2,
    ],
    [
        -18, -18, -18, -17, -16, -15, -14, -13, -12, -8, -13, -20, -25, -26, -24, -18, -11, -1, 1,
    ],
    [
        -17, -17, -17, -16, -15, -14, -13, -12, -11, -8, -14, -22, -26, -28, -25, -20, -11, 7, 8,
    ],
    [
        -16, -16, -16, -15, -14, -13, -11, -10, -9, -6, -13, -20, -25, -26, -24, -18, -10, 5, 9,
    ],
    [
        -15, -15, -14, -14, -13, -12, -10, -9, -8, -4, -10, -16, -20, -21, -19, -14, -5, 8, 8,
    ],
    [
        -14, -14, -13, -13, -12, -11, -10, -9, -8, -8, -5, -10, -13, -13, -11, -7, 0, 2, 6,
    ],
    [
        -13, -11, -11, -11, -11, -10, -9, -8, -7, -6, -6, -2, -3, -4, -2, 0, -3, 1, 5,
    ],
    [
        -13, -11, -11, -11, -11, -10, -9, -8, -7, -7, -6, -6, -6, -5, -5, -4, -2, 0, 4,
    ],
    [
        -14, -12, -11, -11, -11, -10, -9, -8, -8, -7, -6, -5, -5, -5, -5, -3, -2, 0, 2,
    ],
    [
        -15, -14, -12, -12, -11, -10, -9, -9, -8, -7, -6, -5, -5, -5, -5, -4, -3, -1, 1,
    ],
];
const AFTER: [[i8; 19]; 19] = [
    [
        -29, -30, -30, -31, -30, -27, -24, -22, -19, -18, -17, -10, -11, -2, 1, 0, -3, -8, -12,
    ],
    [
        -30, -31, -31, -31, -29, -24, -18, -18, -18, -15, -14, -10, -11, -3, -1, -1, -1, -5, -8,
    ],
    [
        -31, -31, -31, -29, -25, -18, -11, -12, -10, -2, -7, -6, -7, -5, -2, -1, 2, 0, -4,
    ],
    [
        -31, -31, -29, -26, -19, -13, -7, -8, -6, -4, -5, -7, -4, -2, -2, -1, 3, 3, 2,
    ],
    [
        -30, -29, -26, -20, -13, -13, -7, -9, -10, -11, -12, -12, -7, -3, -2, -1, -2, 4, 4,
    ],
    [
        -28, -27, -24, -17, -14, -14, -14, -17, -18, -19, -19, -17, -14, -10, -4, -2, -3, 2, 6,
    ],
    [
        -25, -24, -19, -16, -17, -14, -19, -22, -24, -25, -24, -22, -19, -14, -8, -2, -3, 1, 10,
    ],
    [
        -23, -22, -17, -17, -17, -17, -22, -26, -28, -29, -28, -24, -21, -16, -9, 0, -1, 1, 6,
    ],
    [
        -21, -20, -16, -17, -17, -19, -24, -28, -30, -31, -30, -28, -24, -19, -13, -7, 2, -1, 3,
    ],
    [
        -19, -19, -14, -18, -17, -20, -25, -29, -31, -32, -31, -29, -25, -22, -19, -14, -3, -1, -2,
    ],
    [
        -18, -18, -14, -16, -16, -19, -24, -28, -30, -31, -30, -28, -25, -26, -24, -17, -7, -1, 1,
    ],
    [
        -17, -17, -13, -14, -15, -17, -22, -26, -28, -29, -28, -26, -26, -28, -25, -18, -7, 7, 8,
    ],
    [
        -16, -16, -13, -11, -14, -14, -19, -22, -24, -24, -24, -22, -25, -26, -24, -14, -7, 5, 9,
    ],
    [
        -15, -15, -14, -10, -10, -12, -14, -17, -19, -17, -19, -17, -20, -21, -16, -10, -5, 8, 8,
    ],
    [
        -14, -14, -13, -11, -8, -8, -10, -12, -13, -14, -11, -12, -13, -10, -7, -5, 0, 2, 6,
    ],
    [
        -13, -11, -11, -11, -9, -6, -5, -6, -6, -6, -5, 0, 1, 0, 0, 0, -3, 1, 5,
    ],
    [
        -13, -11, -11, -11, -11, -10, -6, -4, -3, -2, -2, -2, -3, -5, -5, -4, -2, 0, 4,
    ],
    [
        -14, -12, -11, -11, -11, -10, -9, -8, -8, -7, -6, -5, -5, -5, -5, -3, -2, 0, 2,
    ],
    [
        -15, -14, -12, -12, -11, -10, -9, -9, -8, -7, -6, -5, -5, -5, -5, -4, -3, -1, 1,
    ],
];

fn fixture() -> Option<(GameSession, EntityManager, WorldFx)> {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    let terrain = session.cache.level_terrain_mut().unwrap();
    for (dx, row) in BEFORE.iter().enumerate() {
        for (dz, &height) in row.iter().enumerate() {
            let cell = &mut terrain.cells[(133 + dx) * GRID_SIZE + 115 + dz];
            cell.height = height as u8;
            // This test isolates the callback; child insertion is checked below.
            cell.attribute = 0;
        }
    }
    let cell = &mut terrain.cells[142 * GRID_SIZE + 124];
    cell.attribute = 30;
    // 4337E0 has already set08 at the capture's callback-entry boundary.
    cell.terrain_type = 0x2a & !0x08;
    let empty_level = parse_level(&[0; 0xd0]).unwrap();
    let manager = EntityManager::from_level(&empty_level, &[], None);
    let mut world_fx = WorldFx::default();
    for _ in 0..8 {
        world_fx.advance_frame_pacing(20_000);
    }
    session
        .cache
        .initialize_level_terrain_radar(&mut || world_fx.next_shared_retail_random_u16())
        .unwrap();
    Some((session, manager, world_fx))
}

#[v2k_test_support::retail_test]
fn burn_callback_matches_second_retail_crater_and_opens_foreground_water() {
    let Some((mut session, mut manager, mut fx)) = fixture() else {
        return;
    };
    let mut scheduler = StaticDamageScheduler::new();
    let result = apply_static_terrain_burn(
        [142, 124],
        &mut session.cache,
        &mut manager,
        &mut fx,
        &mut scheduler,
    )
    .unwrap();
    assert!(result.burn_changed && result.crater_applied && result.terrain_changed);
    let terrain = session.cache.level_terrain().unwrap();
    assert_eq!(terrain.sea_level_raw(), -847);
    let mut changed = 0;
    let mut wet_before = 0;
    let mut wet_after = 0;
    for (dx, row) in AFTER.iter().enumerate() {
        for (dz, &expected) in row.iter().enumerate() {
            let cell = terrain.cell(133 + dx, 115 + dz).unwrap();
            assert_eq!(
                cell.height as i8,
                expected,
                "retail cell {},{}",
                133 + dx,
                115 + dz
            );
            changed += usize::from(BEFORE[dx][dz] != expected);
            wet_before += usize::from(i16::from(BEFORE[dx][dz]) * 32 < -847);
            wet_after += usize::from(i16::from(expected) * 32 < -847);
        }
    }
    assert_eq!((changed, wet_before, wet_after), (164, 23, 44));
    assert_eq!(terrain.cell(142, 124).unwrap().terrain_type, 0x0c);
    let heights = terrain
        .cells
        .iter()
        .map(|cell| cell.height)
        .collect::<Vec<_>>();
    let count = scheduler.active_program_count();
    let particle_count = fx.particle_count();
    let sounds = fx.take_positional_sounds();
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].sound_id, 62);
    let repeated = apply_static_terrain_burn(
        [142, 124],
        &mut session.cache,
        &mut manager,
        &mut fx,
        &mut scheduler,
    )
    .unwrap();
    assert_eq!(repeated, StaticTerrainBurnOutcome::default());
    assert_eq!(scheduler.active_program_count(), count);
    assert_eq!(fx.particle_count(), particle_count);
    assert!(fx.take_positional_sounds().is_empty());
    assert_eq!(
        session
            .cache
            .level_terrain()
            .unwrap()
            .cells
            .iter()
            .map(|cell| cell.height)
            .collect::<Vec<_>>(),
        heights
    );
}

#[v2k_test_support::retail_test]
fn burned_static_suffix_uses_authored_class93_count_and_cached_surface_origin() {
    let Some((mut session, mut manager, mut fx)) = fixture() else {
        return;
    };
    let result = apply_static_terrain_burn(
        [142, 124],
        &mut session.cache,
        &mut manager,
        &mut fx,
        &mut StaticDamageScheduler::new(),
    )
    .unwrap();
    assert!(result.crater_applied && result.burst_emitted);
    assert_eq!(
        static_kind_descriptor(27)
            .unwrap()
            .burn_burst()
            .unwrap()
            .scatter_count,
        160
    );
    let particles = fx.test_particles_in_virgin_birth_order();
    assert_eq!(particles.len(), 161);
    assert!(particles[..160]
        .iter()
        .all(|particle| particle.source_class == 93));
    assert!(particles.iter().all(|particle| particle.owner_id == Some(0)
        && particle.source_entity_type_at_birth == Some(0)
        && !particle.suppresses_impact_damage));
    let surface = particles.last().unwrap();
    assert_eq!(
        surface.source_class, 18,
        "cached pre-crater Y remains above sea even though the new ground is below it"
    );
    assert_eq!(
        surface
            .position
            .map(|value| (value * 256.0).round() as i32 as i16),
        [-29056, -224, 31872]
    );
    let sounds = fx.take_positional_sounds();
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].sound_id, 62);
    assert_eq!(sounds[0].position, surface.position);
}

#[v2k_test_support::retail_test]
fn shared_radial_kind4_fuel_burn_runs_listener_once_across_authored_worlds() {
    use crate::{damage::PRIMARY_PROJECTILE_DAMAGE_PACKET, radial_damage::RadialDamageTemplate};
    let dir = v2k_test_support::retail_dir();
    let mut models = std::collections::BTreeSet::new();
    for level in [13, 14, 50] {
        let mut session = GameSession::init(&dir).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(level, 1).unwrap();
        let terrain = session.cache.level_terrain().unwrap();
        let (index, current) = terrain
            .cells
            .iter()
            .enumerate()
            .find(|(_, cell)| {
                cell.terrain_type & 8 == 0
                    && cell.attribute != 0
                    && session.cache.terrain_objects().unwrap().records[usize::from(cell.attribute)]
                        .kind_index
                        == 4
            })
            .map(|(index, cell)| (index, *cell))
            .expect("authored kind4 fuel object");
        let cell = [(index / GRID_SIZE) as u8, (index % GRID_SIZE) as u8];
        let object =
            &session.cache.terrain_objects().unwrap().records[usize::from(current.attribute)];
        models.insert(object.model_id_for(current.terrain_type));
        let origin = [
            (u16::from(cell[0]) * 256 + 128) as i16,
            terrain.bilinear_height_raw(
                (u16::from(cell[0]) * 256 + 128) as i16,
                (u16::from(cell[1]) * 256 + 128) as i16,
            ),
            (u16::from(cell[1]) * 256 + 128) as i16,
        ];
        let heights: Vec<_> = terrain.cells.iter().map(|cell| cell.height).collect();
        let template = RadialDamageTemplate {
            inner_radius_raw: 256,
            outer_radius_raw: 256,
            impulse_raw: 0,
            packet: PRIMARY_PROJECTILE_DAMAGE_PACKET,
            trailing_raw: [0; 2],
        };
        let mut fx = WorldFx::new();
        for _ in 0..8 {
            fx.advance_frame_pacing(20_000);
        }
        let mut scheduler = StaticDamageScheduler::new();
        let reports = crate::intro2_radial::apply_static_radial_damage(
            &mut session.cache,
            &mut scheduler,
            &mut fx,
            origin,
            template,
        )
        .unwrap();
        assert!(reports.iter().any(|report| matches!(report,
            crate::static_damage::StaticDamageOutcome::ImmediateBurn { cell: actual, .. } if *actual == cell)));
        let particles = fx.test_particles_in_virgin_birth_order();
        assert_eq!(
            particles
                .iter()
                .filter(|particle| particle.source_class == 93)
                .count(),
            80
        );
        assert!(particles.iter().all(|particle| particle.owner_id == Some(0)
            && particle.source_entity_type_at_birth == Some(0)));
        assert_eq!(fx.take_positional_sounds()[0].sound_id, 62);
        assert_eq!(scheduler.active_program_count(), 0);
        assert_eq!(
            session
                .cache
                .level_terrain()
                .unwrap()
                .cells
                .iter()
                .map(|cell| cell.height)
                .collect::<Vec<_>>(),
            heights,
            "kind4 does not own either crater"
        );
        let count = fx.particle_count();
        let repeated = apply_immediate_static_burn(cell, &mut session.cache, &mut fx).unwrap();
        assert_eq!(repeated, StaticTerrainBurnOutcome::default());
        assert_eq!(fx.particle_count(), count);
        assert!(fx.take_positional_sounds().is_empty());
    }
    assert!(
        !models.is_empty(),
        "authored fuel model was exercised in all three worlds"
    );
}

#[v2k_test_support::retail_test]
fn secondary_crater_runs_null_program_listener_before_outer_burst_and_preserves_hut_programs() {
    let Some((mut session, mut manager, mut fx)) = fixture() else {
        return;
    };
    let fuel_attribute = session
        .cache
        .terrain_objects()
        .unwrap()
        .records
        .iter()
        .position(|record| record.kind_index == 4)
        .unwrap() as u8;
    // The hut instances are genuine Section9 kind29/model364/365 objects,
    // distinct from the Type66 dynamic hut and the kind27 crater owner.
    assert_eq!(
        session.cache.terrain_objects().unwrap().records[29].kind_index,
        29
    );
    assert_eq!(
        session.cache.terrain_objects().unwrap().records[29].model_ids,
        [364, 365, 364, 365]
    );
    let terrain = session.cache.level_terrain_mut().unwrap();
    terrain.cells[141 * GRID_SIZE + 124].attribute = fuel_attribute;
    for [x, z] in [[138, 127], [139, 124]] {
        terrain.cells[x * GRID_SIZE + z].attribute = 29;
        terrain.cells[x * GRID_SIZE + z].terrain_type &= !8;
    }
    let mut scheduler = StaticDamageScheduler::new();
    assert_eq!(
        scheduler.submit_crater_destruction([142, 124], 27),
        StaticCraterOutcome::Started
    );
    let result = apply_static_terrain_burn(
        [142, 124],
        &mut session.cache,
        &mut manager,
        &mut fx,
        &mut scheduler,
    )
    .unwrap();
    assert!(result.crater_applied && result.burst_emitted);
    assert_eq!(
        scheduler.active_program_count(),
        3,
        "the source crater and two hut programs retain their authored FIFO timing"
    );
    let particles = fx.test_particles_in_virgin_birth_order();
    assert_eq!(
        particles
            .iter()
            .take(80)
            .filter(|particle| particle.source_class == 93)
            .count(),
        80
    );
    assert_eq!(
        particles.len(),
        200,
        "later kind27 scatter observes earlier listener allocations"
    );
    let sounds = fx.take_positional_sounds();
    assert_eq!(
        sounds
            .iter()
            .map(|sound| sound.sound_id)
            .collect::<Vec<_>>(),
        [62, 62]
    );
    assert_ne!(sounds[0].position, sounds[1].position);
    assert_eq!(
        session
            .cache
            .level_terrain()
            .unwrap()
            .cell(141, 124)
            .unwrap()
            .terrain_type
            & 8,
        8
    );
    for [x, z] in [[138, 127], [139, 124]] {
        assert_eq!(
            session
                .cache
                .level_terrain()
                .unwrap()
                .cell(x, z)
                .unwrap()
                .terrain_type
                & 8,
            0,
            "kind29 stays intact until its 500-ms authored burn record"
        );
    }
}

#[v2k_test_support::retail_test]
fn synchronous_crater_retains_in_flight_dedup_and_pre_crater_radial_origin() {
    let Some((mut session, mut manager, mut fx)) = fixture() else {
        return;
    };
    let mut scheduler = StaticDamageScheduler::new();
    assert_eq!(
        scheduler.submit_crater_destruction([142, 124], 27),
        StaticCraterOutcome::Started
    );
    scheduler.begin_advance(2_100_000);
    let batch = scheduler
        .next_node_batch(|cell| {
            resolve_current_static_damage_target(&session.cache, cell)
                .unwrap()
                .map(|target| target.state)
        })
        .unwrap();
    let (token, actions) = batch.into_parts();
    let mut callback_ran = false;
    for action in actions {
        match action {
            StaticDamageAction::SetBurned { cell } => {
                let result = apply_static_terrain_burn(
                    cell,
                    &mut session.cache,
                    &mut manager,
                    &mut fx,
                    &mut scheduler,
                )
                .unwrap();
                assert!(result.crater_applied);
                callback_ran = true;
                assert!(scheduler.contains_cell(cell));
                assert_eq!(
                    scheduler.active_program_count(),
                    1,
                    "the crater must not restart its in-flight source"
                );
            }
            StaticDamageAction::Radial { origin_raw, .. } => {
                assert!(callback_ran);
                assert_eq!(
                    session
                        .cache
                        .level_terrain()
                        .unwrap()
                        .cell(142, 124)
                        .unwrap()
                        .height as i8,
                    -32
                );
                assert_eq!(
                    origin_raw,
                    [-29056, -224, 31744],
                    "281A0 caches pre-crater Y and north-edge Z before its opcode loop"
                );
            }
            _ => {}
        }
    }
    assert!(callback_ran);
    assert!(scheduler.complete_node_batch(token));
    assert!(!scheduler.contains_cell([142, 124]));
}

#[v2k_test_support::retail_test]
fn catalog_has_one_authored_burn_crater_and_empty_cells_only_change_burn_bit() {
    let kinds = (0..32)
        .filter(|&kind| {
            static_kind_descriptor(kind)
                .unwrap()
                .burn_crater()
                .is_some()
        })
        .collect::<Vec<_>>();
    assert_eq!(kinds, [27]);
    let Some((mut session, mut manager, mut fx)) = fixture() else {
        return;
    };
    let original = session
        .cache
        .level_terrain()
        .unwrap()
        .cell(133, 115)
        .unwrap()
        .height;
    let result = apply_static_terrain_burn(
        [133, 115],
        &mut session.cache,
        &mut manager,
        &mut fx,
        &mut StaticDamageScheduler::new(),
    )
    .unwrap();
    assert!(result.burn_changed && !result.crater_applied);
    assert_eq!(
        session
            .cache
            .level_terrain()
            .unwrap()
            .cell(133, 115)
            .unwrap()
            .height,
        original
    );
}
