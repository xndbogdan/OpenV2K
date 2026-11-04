use super::*;

#[test]
fn cells_are_sample_points_inside_the_sphere_x_outer() {
    // Offsets -500, -244, 12 and 268: -500 never fits (its square alone is
    // the radius's), so a flat world at the ground takes the 3 x 3 rest.
    let writes = infection_radial_cells([0x1080, 0, 0x2080], 500, true, |_| 0);
    assert_eq!(writes.len(), 9);
    assert_eq!(
        writes[0],
        InfectionCellWrite::at([0x1080 - 244, 0x2080 - 244], true)
    );
    assert_eq!(
        writes[1],
        InfectionCellWrite::at([0x1080 - 244, 0x2080 + 12], true)
    );
    assert_eq!(
        writes[8],
        InfectionCellWrite::at([0x1080 + 268, 0x2080 + 268], true)
    );

    // 400 above the ground leaves only the points within 300 horizontally.
    let raised = infection_radial_cells([0x1080, 400, 0x2080], 500, false, |_| 0);
    assert_eq!(raised.len(), 5);
    assert!(raised.iter().all(|write| !write.infected));

    // The ground is the cell's height byte << 5: 400 above byte 12 is 16.
    let ground = infection_radial_cells([0x1080, 400, 0x2080], 500, true, |_| 12);
    assert_eq!(ground.len(), 9);
}

#[test]
fn cells_wrap_the_world_words() {
    let writes = infection_radial_cells([0x0010, 0, 0x7FF0], 500, true, |_| 0);
    assert_eq!(writes.len(), 9);
    assert_eq!(
        writes[0].position_raw,
        [(0x0010u16.wrapping_sub(244)) as i16, 0x7FF0 - 244]
    );
    assert_eq!(writes[0].cell, [0xFF, 0x7E]);
    assert_eq!(writes[8].cell, [0x01, 0x80]);
}

#[test]
fn bodies_need_the_state_and_the_range() {
    let infect = ParticleInfectionRadial {
        origin_raw: [0, 0, 0],
        infected: true,
    };
    let cure = ParticleInfectionRadial {
        infected: false,
        ..infect
    };
    let known = RetailRuntimeValue::Known;
    assert_eq!(
        body_eligible([300, 300, 0], known(0x8800), infect),
        Ok(true)
    );
    assert_eq!(
        body_eligible([200, 200, 200], known(0x8800), infect),
        Ok(true)
    );
    assert_eq!(
        body_eligible([300, 300, 300], known(0x8800), infect),
        Ok(false)
    );
    assert_eq!(body_eligible([0, 0, 0], known(0xA800), infect), Ok(false));
    assert_eq!(body_eligible([0, 0, 0], known(0xA800), cure), Ok(true));
    // With 0x1000, or without 0x0800, the walk passes it by; 0x4000 (dying)
    // is outside the mask.
    assert_eq!(body_eligible([0, 0, 0], known(0x9800), infect), Ok(false));
    assert_eq!(body_eligible([0, 0, 0], known(0x8000), infect), Ok(false));
    assert_eq!(
        body_eligible([0, 0, 0], known(0x8804 | 0x4000), infect),
        Ok(true)
    );
    assert_eq!(
        body_eligible([0, 0, 0], RetailRuntimeValue::Unresolved, infect),
        Err(())
    );
    assert_eq!(
        body_eligible([600, 0, 0], RetailRuntimeValue::Unresolved, infect),
        Ok(false)
    );
}

/// Retail lab (Level 1, master code, F9, the Virus Bomb held for 60
/// ticks): three bombs ran `FUN_00441850` from `FUN_00442B40`'s free at
/// these origins; `FUN_00433720` took these X/Z words in this order, and the
/// radar drew once for each cell that changed: all seven, then the second
/// bomb's first three (the rest were the first bomb's), then all nine.
#[v2k_test_support::retail_test]
fn level_one_bombs_write_retails_cells() {
    let (mut session, _manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(13);
    let bombs: [([u16; 3], &[(u16, u16)], usize); 3] = [
        (
            [0x4D8A, 0xFED1, 0x3EC8],
            &[
                (0x4C96, 0x3ED4),
                (0x4C96, 0x3FD4),
                (0x4D96, 0x3DD4),
                (0x4D96, 0x3ED4),
                (0x4D96, 0x3FD4),
                (0x4E96, 0x3ED4),
                (0x4E96, 0x3FD4),
            ],
            7,
        ),
        (
            [0x4C75, 0xFE3F, 0x3E63],
            &[
                (0x4B81, 0x3E6F),
                (0x4B81, 0x3F6F),
                (0x4C81, 0x3D6F),
                (0x4C81, 0x3E6F),
                (0x4C81, 0x3F6F),
                (0x4D81, 0x3D6F),
                (0x4D81, 0x3E6F),
                (0x4D81, 0x3F6F),
            ],
            3,
        ),
        (
            [0x4925, 0xFC7D, 0x396C],
            &[
                (0x4831, 0x3878),
                (0x4831, 0x3978),
                (0x4831, 0x3A78),
                (0x4931, 0x3878),
                (0x4931, 0x3978),
                (0x4931, 0x3A78),
                (0x4A31, 0x3878),
                (0x4A31, 0x3978),
                (0x4A31, 0x3A78),
            ],
            9,
        ),
    ];
    for (origin, expected, changes) in bombs {
        let terrain = session.cache.level_terrain().unwrap();
        let writes = infection_radial_cells(origin.map(|word| word as i16), 500, true, |[x, z]| {
            terrain.cell(usize::from(x), usize::from(z)).unwrap().height as i8
        });
        let words: Vec<_> = writes
            .iter()
            .map(|write| (write.position_raw[0] as u16, write.position_raw[1] as u16))
            .collect();
        assert_eq!(words, expected);
        let changed = writes
            .into_iter()
            .filter(|&write| {
                crate::infection_listeners::write_infection(&mut session.cache, &mut fx, write)
                    .unwrap()
            })
            .count();
        assert_eq!(changed, changes);
    }
}
