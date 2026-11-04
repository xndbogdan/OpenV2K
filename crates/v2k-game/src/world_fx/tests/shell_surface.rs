use super::*;

/// Retail lab (Level 1, master code, F9, the Virus Bomb fired at tick 664,
/// world clock 312): `FUN_0044E770` launched it at `(19850, -596, 14422)`
/// with `(0, -282, 1762)`, and `FUN_00440A60` added class 77's `+0x2A` 50.
/// Each row is the record at its `FUN_0043F260` entry from clock 313 on: Y
/// and Z after that tick's integration, and the vertical word before its
/// gravity. It falls 29 a tick, meets the sea at clock 329 (E300 bounces it:
/// Y on the wave, the word halved and reversed), settles at 9 or 10 a tick
/// and rides the beach up the bilinear ground, and its life ends at clock
/// 363 at `(19850, -303, 16072)`, where `FUN_00441850` runs.
const RETAIL: [(i16, i16, i16); 50] = [
    (-601, 14455, -232),
    (-606, 14488, -261),
    (-612, 14521, -290),
    (-619, 14554, -319),
    (-626, 14587, -348),
    (-634, 14620, -377),
    (-642, 14653, -406),
    (-651, 14686, -435),
    (-660, 14719, -464),
    (-670, 14752, -493),
    (-680, 14785, -522),
    (-691, 14818, -551),
    (-703, 14851, -580),
    (-715, 14884, -609),
    (-728, 14917, -638),
    (-741, 14950, -667),
    (-755, 14983, -696),
    (-754, 15016, 362),
    (-761, 15049, -166),
    (-754, 15082, 97),
    (-754, 15115, -34),
    (-752, 15148, 31),
    (-755, 15181, -1),
    (-757, 15214, 15),
    (-759, 15247, 7),
    (-761, 15280, 11),
    (-763, 15313, 9),
    (-765, 15346, 10),
    (-768, 15379, 9),
    (-763, 15412, 10),
    (-753, 15445, 9),
    (-743, 15478, 10),
    (-733, 15511, 9),
    (-724, 15544, 10),
    (-714, 15577, 9),
    (-704, 15610, 10),
    (-694, 15643, 9),
    (-669, 15676, 10),
    (-640, 15709, 9),
    (-611, 15742, 10),
    (-582, 15775, 9),
    (-553, 15808, 10),
    (-524, 15841, 9),
    (-496, 15874, 10),
    (-467, 15907, 9),
    (-440, 15940, 10),
    (-412, 15973, 9),
    (-385, 16006, 10),
    (-358, 16039, 9),
    (-330, 16072, 10),
];

#[v2k_test_support::retail_test]
fn level_one_virus_bomb_follows_retails_flight_bounce_and_slide() {
    let (session, _manager, _) =
        crate::native_type122::construction_tests::native_fixture_with_player(13);
    let context = TerrainCollisionContext::from_current_level_cache(&session.cache).unwrap();
    let mut fx = WorldFx::new();
    fx.frame_pacing.count_scale_q16 = 0x1_0000;
    fx.last_retail_tick = Some(312);
    let bomb = descriptor_test_particle(
        77,
        raw_position_to_world([19850, -596, 14422]),
        [0, -282, 1762],
        None,
    );
    let slot = fx.particles.allocate(bomb).unwrap();
    let mut previous_y = None;
    for (index, &(y, z, vertical)) in RETAIL.iter().enumerate() {
        let clock = 313 + index as u32;
        let particle = fx.particles.slots[slot].unwrap();
        let velocity = particle
            .velocity
            .map(|v| world_velocity_component_to_raw(v) as i16);
        assert_eq!(velocity, [0, vertical, 1762], "word before clock {clock}");
        if let Some(previous_y) = previous_y {
            // 40120's integration from the previous tick's final Y.
            let step = (i32::from(vertical) * (20_000 >> 5) >> 15) as i16;
            assert_eq!(previous_y + step, y, "Y integrated at clock {clock}");
        }
        fx.update(ParticleUpdateRequest::terrain(20_000, clock, context));
        let particle = fx.particles.slots[slot].unwrap();
        assert_eq!(particle.source_class, 77);
        let position = world_position_to_raw(particle.position);
        assert_eq!([position[0], position[2]], [19850, z], "clock {clock}");
        previous_y = Some(position[1]);
    }
    let outcome = fx.update(ParticleUpdateRequest::terrain(20_000, 363, context));
    let [tail] = outcome.held_bomb_tails.as_slice() else {
        panic!("{:?}", outcome.held_bomb_tails)
    };
    assert_eq!(world_position_to_raw(tail.position), [19850, -303, 16072]);
}

/// `FUN_00440A60` adds class 77's `+0x2A` (+50) to the launch's vertical
/// word, as it does for every particle weapon's class.
#[test]
fn a_bomb_launch_takes_its_class_vertical_bias() {
    let mut event = primary_event(-4.0, [0.0; 3]);
    let descriptor = particle_descriptor(77).unwrap();
    event.projectile.spec = crate::primary_weapon::PrimaryProjectileSpec {
        projectile_class: 77,
        sprite_id: first_particle_frame(77).sprite_id,
        speed_raw: 2000,
        speed_world_per_second: crate::primary_weapon::raw_velocity_to_world_per_second(2000),
        lifetime_ticks: descriptor.lifetime_ticks(),
    };
    event.projectile.velocity_world_per_second = raw_velocity_to_world([0, -282, 1762]);
    let mut fx = WorldFx::new();
    fx.queue_primary_fire_batch(&[event], None);
    fx.process_pending();
    let bomb = fx
        .test_particles_in_virgin_birth_order()
        .into_iter()
        .find(|particle| particle.source_class == 77)
        .unwrap();
    assert_eq!(
        bomb.velocity
            .map(|v| world_velocity_component_to_raw(v) as i16),
        [0, -232, 1762]
    );
}
