//! Native worker E100 routing: exact current wind, retained F70 and RNG order.
use super::*;
use crate::common_mover::{
    type9_surface::ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT,
    type9_tail::{apply_type9_environment_drag_raw, Type9ModeZeroDrag},
};

const DT: u32 = 100_000;

fn actor(manager: &EntityManager, id: u32) -> &Entity {
    manager.iter_all().find(|entity| entity.id == id).unwrap()
}

fn prepare(manager: &mut EntityManager, id: u32, position: [i16; 3]) {
    let entity = manager.entity_mut(id).unwrap();
    // Isolate E100 from the separately covered E370 lifecycle. The original
    // surface-disable flag and hidden healthy sound gate both remain genuine.
    let set = COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
        | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
        | ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT;
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(set | 0x800, set);
    entity.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
    entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    entity.set_motion_raw(position, [113, -271, 389]);
    entity.set_rotation_heading_pitch_roll_raw([3200, 450, -700]);
}

#[v2k_test_support::retail_test]
fn native_type116_routes_still_steady_and_gust_wind_after_task_before_ground_and_motion() {
    for (level, expected_mode, expected_vector, flat_water) in [
        (42, 0, [0, 0, 0], false),
        (46, 1, [1000, 1000, 1000], false),
        (47, 2, [2000, 0, 0], false),
        (46, 1, [1000, 1000, 1000], true),
        (47, 2, [2000, 0, 0], true),
    ] {
        let mut fx = WorldFx::new();
        let (mut session, mut manager) = crate::intro2_type8::authored_tests::load(level, &mut fx)
            .expect("canonical retail corpus");
        let mut oracle_fx = WorldFx::new();
        let (_, mut oracle) = crate::intro2_type8::authored_tests::load(level, &mut oracle_fx)
            .expect("canonical retail corpus");
        manager.advance_environment_frame(&mut fx, DT);
        oracle.advance_environment_frame(&mut oracle_fx, DT);
        let environment = manager.common_environment_physics();
        assert_eq!(environment.runtime_wind_mode, expected_mode, "world{level}");
        assert_eq!(
            environment.configured_wind_raw, expected_vector,
            "world{level}"
        );
        if expected_mode == 2 {
            assert_ne!(environment.current_wind_raw, expected_vector);
            assert_ne!(environment.current_wind_raw, [0; 3]);
        }
        let id = manager
            .iter_all()
            .find(|entity| entity.entity_type == 116)
            .unwrap()
            .id;
        // World46's authored sea=-4096/+80=0 keeps normal ground actors on
        // the ordinary-drag side. Keep that real case, then separately use
        // controlled flat water to exercise vector/angular routing with the
        // same native allocation and unchanged authored wind descriptor.
        if flat_water {
            let terrain = session.cache.level_terrain_mut().unwrap();
            terrain.header[0] = 0;
            for cell in &mut terrain.cells {
                cell.height = (-128i8) as u8;
            }
        }
        let terrain = session.cache.level_terrain().unwrap();
        let position = if flat_water {
            [
                128,
                if environment.wind_above_sea {
                    1024
                } else {
                    -1024
                },
                128,
            ]
        } else {
            actor(&manager, id).position_raw()
        };
        prepare(&mut manager, id, position);
        prepare(&mut oracle, id, position);
        let owner = Intro2Type8Owner::take_birth(manager.entity_mut(id).unwrap()).unwrap();
        let mut oracle_owner =
            Intro2Type8Owner::take_birth(oracle.entity_mut(id).unwrap()).unwrap();
        let metadata = oracle.type_runtime_metadata(116).unwrap().clone();
        let RetailRuntimeValue::Known(mass) = common_scheduler_callback_mass(
            metadata.mass_raw,
            actor(&oracle, id).collision.animation_offset_at_0xb2,
        ) else {
            panic!("native callback mass")
        };
        oracle.entity_mut(id).unwrap().mass_raw = mass;
        // Run the actual task independently to retain its real D/Sub-I and
        // random prefix. Only the surrounding F70/E100/DF70 tail is predicted.
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
            &mut oracle_fx,
        )
        .unwrap());
        let entry = actor(&oracle, id);
        let [heading, pitch, roll] = entry.rotation_heading_pitch_roll_raw();
        let basis = Type9BodyBasis::from_angle_words(heading, pitch, roll);
        let mut angles = [heading, pitch, roll];
        let mut velocity = entry.velocity_raw();
        let mut mode_zero_velocity = velocity;
        apply_type9_environment_drag_raw(
            &mut mode_zero_velocity,
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
        if expected_mode == 0 || (!flat_water && level == 46) {
            assert_eq!(velocity, mode_zero_velocity);
            assert_eq!(angles, [heading, pitch, roll]);
        } else if flat_water {
            assert_ne!(
                velocity, mode_zero_velocity,
                "world{level} must exercise wind response"
            );
            assert_ne!(
                angles,
                [heading, pitch, roll],
                "world{level} must exercise angular response"
            );
        }
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
        let tick = tick_intro2_type8(
            &mut manager,
            owner,
            Intro2Type8Frame {
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
                Intro2Type8Outcome::Advanced {
                    terminal: false,
                    ..
                }
            ),
            "world{level}: {:?}",
            tick.outcome
        );
        let actual = actor(&manager, id);
        assert_eq!(
            actual.position_raw(),
            motion.position_after_raw,
            "world{level}"
        );
        assert_eq!(
            actual.velocity_raw(),
            motion.velocity_after_raw,
            "world{level}"
        );
        assert_eq!(
            actual.rotation_heading_pitch_roll_raw(),
            angles,
            "world{level}"
        );
        assert_eq!(
            actual.physical_body_basis_q31(),
            RetailRuntimeValue::Known(basis)
        );
        assert_eq!(
            manager.common_environment_physics(),
            environment,
            "actors must not advance gust phase"
        );
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle_fx.next_shared_retail_random_u16(),
            "world{level} environment tail must not add random draws"
        );
    }
}

#[v2k_test_support::retail_test]
fn native_worker_wind_rejects_changed_environment_foreign_or_pending_owner_before_writes() {
    for case in 0..3 {
        let mut fx = WorldFx::new();
        let (session, mut manager) = crate::intro2_type8::authored_tests::load(46, &mut fx)
            .expect("canonical retail corpus");
        let id = manager
            .iter_all()
            .find(|entity| entity.entity_type == 116)
            .unwrap()
            .id;
        let mut owner = Intro2Type8Owner::take_birth(manager.entity_mut(id).unwrap()).unwrap();
        match case {
            0 => manager.set_runtime_wind_mode_for_test(0),
            1 => {
                let (_, mut foreign) =
                    crate::intro2_type8::authored_tests::load(46, &mut WorldFx::new()).unwrap();
                owner = Intro2Type8Owner::take_birth(foreign.entity_mut(id).unwrap()).unwrap();
            }
            _ => owner.pending = true,
        }
        let snapshot = |manager: &EntityManager| {
            let entity = actor(manager, id);
            (
                entity.position_raw(),
                entity.velocity_raw(),
                entity.rotation_heading_pitch_roll_raw(),
                entity.physical_body_basis_q31(),
                entity.collision.callback_scheduler_accumulator_us_at_0x6c,
                entity.collision.subject_scan_gate_at_0x70,
                entity.current_behavior_context,
            )
        };
        let before = snapshot(&manager);
        let mut oracle = fx.fork_for_main_base_abort_transaction();
        let tick = tick_intro2_type8(
            &mut manager,
            owner,
            Intro2Type8Frame {
                resources: &session.cache,
                world_fx: &mut fx,
                elapsed_micros: DT,
                global_elapsed_micros: DT,
                retail_tick: 5,
            },
        );
        match case {
            0 => assert_eq!(
                tick.outcome,
                Intro2Type8Outcome::Blocked {
                    entity_id: id,
                    reason: Intro2Type8Block::Runtime("world wind custody"),
                    prefix_committed: false
                }
            ),
            1 => assert_eq!(tick.outcome, Intro2Type8Outcome::Dropped { entity_id: id }),
            _ => assert_eq!(tick.outcome, Intro2Type8Outcome::Pending { entity_id: id }),
        }
        assert_eq!(snapshot(&manager), before);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
    }
}
