//! World32's native Type78 uses the shared E100 owner without losing F70.
use super::*;
use crate::common_mover::{
    type9_surface::ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT,
    type9_tail::{apply_type9_environment_drag_raw, Type9ModeZeroDrag},
};

#[v2k_test_support::retail_test]
fn native_type78_steady_wind_preserves_task_rng_and_pre_wind_physical_basis() {
    const DT: u32 = 100_000;
    let mut fx = WorldFx::new();
    let (session, mut manager) =
        crate::native_type86::tests::load(32, &mut fx).expect("canonical retail corpus");
    let mut oracle_fx = WorldFx::new();
    let (_, mut oracle) =
        crate::native_type86::tests::load(32, &mut oracle_fx).expect("canonical retail corpus");
    let environment = manager.common_environment_physics();
    assert_eq!(environment.runtime_wind_mode, 1);
    assert_eq!(environment.current_wind_raw, [-300, 0, -300]);
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 78)
        .unwrap()
        .id;
    let terrain = session.cache.level_terrain().unwrap();
    // A controlled exposed pose, using the real terrain and authored sea-side
    // gate, makes angular and linear wind observable on a genuine allocation.
    let vector = environment.current_wind_raw;
    let position = (-30_000i32..=30_000)
        .step_by(1024)
        .find_map(|x| {
            (-30_000i32..=30_000).step_by(1024).find_map(|z| {
                let (x, z) = (x as i16, z as i16);
                let floor =
                    terrain
                        .bilinear_height_raw(x, z)
                        .max(terrain.bilinear_height_raw(
                            x.wrapping_sub(vector[0] >> 1),
                            z.wrapping_sub(vector[2] >> 1),
                        ))
                        .max(terrain.bilinear_height_raw(
                            x.wrapping_sub(vector[0]),
                            z.wrapping_sub(vector[2]),
                        ));
                let y = i32::from(if environment.wind_above_sea {
                    floor.max(terrain.sea_level_raw())
                } else {
                    floor
                }) + 1024;
                let y = i16::try_from(y).ok()?;
                ((terrain.sea_level_raw() < y) == environment.wind_above_sea).then_some([x, y, z])
            })
        })
        .expect("canonical exposed wind-side terrain");
    for actors in [&mut manager, &mut oracle] {
        let entity = actors.entity_mut(id).unwrap();
        // Suppress only unrelated E370 and visibility-gated sound; the real
        // person task and its private-state/random prefix still execute.
        let flags = COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
            | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
            | ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT;
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(flags | 0x800, flags);
        entity.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        entity.set_motion_raw(position, [113, -271, 389]);
        entity.set_rotation_heading_pitch_roll_raw([3200, 450, -700]);
    }
    let owner = Type86Owner::take_birth(manager.entity_mut(id).unwrap()).unwrap();
    let mut oracle_owner = Type86Owner::take_birth(oracle.entity_mut(id).unwrap()).unwrap();
    let metadata = oracle.type_runtime_metadata(78).unwrap().clone();
    let RetailRuntimeValue::Known(mass) = common_scheduler_callback_mass(
        metadata.mass_raw,
        oracle
            .entity_mut(id)
            .unwrap()
            .collision
            .animation_offset_at_0xb2,
    ) else {
        panic!("native callback mass")
    };
    oracle.entity_mut(id).unwrap().mass_raw = mass;
    assert!(!task::run_task(
        &mut oracle,
        &mut oracle_owner,
        &task::TaskFrame {
            metadata: &metadata,
            terrain,
            elapsed_micros: DT,
            global_elapsed_micros: DT,
            scheduler_mode: 0,
        },
        &mut oracle_fx
    )
    .unwrap());
    let entry = oracle.entity_mut(id).unwrap();
    let [heading, pitch, roll] = entry.rotation_heading_pitch_roll_raw();
    let basis = Type9BodyBasis::from_angle_words(heading, pitch, roll);
    let mut angles = [heading, pitch, roll];
    let mut velocity = entry.velocity_raw();
    let mut no_wind_velocity = velocity;
    apply_type9_environment_drag_raw(
        &mut no_wind_velocity,
        DT,
        Type9ModeZeroDrag {
            callback_mass_raw: std::num::NonZeroU16::new(mass).unwrap(),
            strength: environment.drag_strength,
        },
    );
    apply_common_wind_drag_raw(
        &mut velocity,
        &mut angles,
        CommonWindDrag::from_level(session.cache.level_desc().unwrap(), environment).unwrap(),
        CommonWindDragFrame {
            terrain,
            position_raw: entry.position_raw(),
            basis,
            callback_mass_raw: std::num::NonZeroU16::new(mass).unwrap(),
            elapsed_micros: DT,
        },
    );
    assert_ne!(velocity, no_wind_velocity);
    assert_ne!(angles, [heading, pitch, roll]);
    let mut position = entry.position_raw();
    let mut ground_flags = 0;
    apply_type9_ground_snap_raw(&mut position, &mut velocity, &mut ground_flags, terrain);
    let state = bits(
        entry,
        COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
        "master motion",
    )
    .unwrap();
    let motion =
        plan_common_master_motion(position, velocity, (state & !0x800000) | ground_flags, DT);
    let tick = tick_type86(
        &mut manager,
        owner,
        Type86Frame {
            resources: &session.cache,
            world_fx: &mut fx,
            elapsed_micros: DT,
            global_elapsed_micros: DT,
            retail_tick: 5,
        },
    );
    assert!(
        matches!(
            tick.outcome,
            Type86Outcome::Advanced {
                terminal: false,
                ..
            }
        ),
        "{:?}",
        tick.outcome
    );
    let actual = manager.entity_mut(id).unwrap();
    assert_eq!(actual.position_raw(), motion.position_after_raw);
    assert_eq!(actual.velocity_raw(), motion.velocity_after_raw);
    assert_eq!(actual.rotation_heading_pitch_roll_raw(), angles);
    assert_eq!(
        actual.physical_body_basis_q31(),
        RetailRuntimeValue::Known(basis)
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        oracle_fx.next_shared_retail_random_u16()
    );
}
