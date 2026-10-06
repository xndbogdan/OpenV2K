//! Canonical Man2 artwork contract between Sub-I, movement and model selection.
//!
//! The normal-tier model has explicit `0x2B 0x0081` frame branches. These
//! expectations come from that authored command table and decoded sprite art,
//! not from the direction-bin implementation. Image directions below assume
//! local +X is screen-right and +Z points away from the viewer; scene-level
//! Sub-I reads absolute entity heading +A2; the original chase camera keeps a
//! fixed -Z-facing view. Local-X reflection belongs to the presentation adapter.

use v2k_formats::{
    collision::ActorAnimationDescriptor,
    models::{AnimVars, ModelEntry},
};
use v2k_game::{
    actor_animation::{
        actor_animation_selection, ActorAnimationController, ActorAnimationSelectionInput,
    },
    common_mover::type9_attitude::Type9BodyBasis,
    session::GameSession,
};

fn session() -> GameSession {
    let data = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&data).expect("canonical PRELOAD.DAT is required");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("normal 640x480 system tier is required");
    session
        .load_level_by_id(13, 1)
        .expect("normal-tier first-world corpus is required");
    session
}

fn descriptor(session: &GameSession) -> ActorAnimationDescriptor {
    let record = session.cache.global_entity_type(9).expect("Type 9");
    assert_eq!(record.model_ids, [558; 4]);
    assert_eq!(
        record.subsection("I"),
        Some(&[0x48, 0, 0, 0, 0x48, 0, 1, 4][..])
    );
    let descriptor = record.actor_animation_descriptor().expect("Man2 Sub-I");
    assert_eq!(descriptor.variable_binding, 1);
    assert_eq!(descriptor.frames_per_direction, 4);
    descriptor
}

fn model(session: &GameSession) -> &ModelEntry {
    let model = session.cache.global_model(558).expect("Man2 model 558");
    assert_eq!(model.name.as_deref(), Some("man2"));
    model
}

/// Assert both triangles' material, source texture orientation and physical
/// geometry. A parser UV reversal or lost authored vertex mirror must fail.
fn assert_frame(model: &ModelEntry, vars: &AnimVars, sprite: u16, mirrored: bool) {
    let frame = model.materialize(vars);
    assert!(frame.billboards.is_empty());
    assert!(frame.instances.is_empty());
    assert_eq!(
        frame.face_materials,
        [
            0x8000 | 1359,
            0x8000 | 1359,
            0x8000 | sprite,
            0x8000 | sprite
        ],
        "selector {}",
        vars.dynamic[1]
    );
    assert_eq!(
        frame.triangles,
        [[0, 1, 2], [0, 2, 3], [4, 5, 6], [4, 6, 7]]
    );
    assert_eq!(frame.vertices.len(), 8);
    let left = if mirrored { 71.0 } else { -71.0 };
    assert_eq!(
        &frame.vertices[4..],
        &[
            [left, 165.0, 0.0],
            [-left, 165.0, 0.0],
            [-left, 0.0, 0.0],
            [left, 0.0, 0.0],
        ],
        "selector {} source texture orientation",
        vars.dynamic[1]
    );
    assert_eq!(
        &frame.face_uvs[2..],
        &[
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            [[0.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        ]
    );
}

#[v2k_test_support::retail_test]
fn walking_artwork_tracks_all_eight_body_directions_and_four_phases() {
    let session = session();
    let descriptor = descriptor(&session);
    let model = model(&session);

    // Heading, first selector, source sprites, authored local-X mirrors,
    // physical forward signs. Directions rotate right -> away -> left -> near.
    let directions = [
        (
            0x0000_u16,
            24,
            [1347, 1348, 1349, 1350],
            [false; 4],
            [1, 0, 0],
        ),
        (0x2000, 20, [1351, 1352, 1353, 1354], [true; 4], [1, 0, 1]),
        (
            0x4000,
            16,
            [1355, 1356, 1355, 1356],
            [false, false, true, true],
            [0, 0, 1],
        ),
        (0x6000, 12, [1351, 1352, 1353, 1354], [false; 4], [-1, 0, 1]),
        (0x8000, 8, [1347, 1348, 1349, 1350], [true; 4], [-1, 0, 0]),
        (0xa000, 4, [1343, 1344, 1345, 1346], [false; 4], [-1, 0, -1]),
        (
            0xc000,
            0,
            [1341, 1342, 1341, 1342],
            [false, false, true, true],
            [0, 0, -1],
        ),
        (0xe000, 28, [1343, 1344, 1345, 1346], [true; 4], [1, 0, -1]),
    ];
    for (heading, first_selector, sprites, mirrored, forward_signs) in directions {
        let body = Type9BodyBasis::from_angle_words(heading as i16, 0, 0);
        assert_eq!(
            body.forward.map(i32::signum),
            forward_signs,
            "yaw {heading:04x}"
        );
        let mut controller = ActorAnimationController::from_descriptor(descriptor).unwrap();
        for phase in 0..4 {
            // Zero elapsed publishes phase zero. Subsequent calls cross the
            // exact 80ms countdown once each, without manufacturing state.
            controller.advance_neutral(if phase == 0 { 0 } else { 81_000 }, heading);
            assert_eq!(controller.phase(), phase as u8);
            let mut vars = AnimVars::default();
            controller.publish(&mut vars);
            assert_eq!(vars.dynamic[1], first_selector + phase as i32);
            assert_frame(model, &vars, sprites[phase], mirrored[phase]);
        }
    }
}

#[v2k_test_support::retail_test]
fn help_carry_and_death_select_the_authored_gesture_and_explosion_frames() {
    let session = session();
    let descriptor = descriptor(&session);
    let model = model(&session);
    let mut carried = ActorAnimationController::from_descriptor(descriptor).unwrap();

    for phase in 0..4 {
        let carry = carried.advance(if phase == 0 { 0 } else { 81_000 }, 0x4000, true);
        assert!(!carry.zero_velocity);
        let mut vars = AnimVars::default();
        carried.publish(&mut vars);
        assert_eq!(vars.dynamic[1], 32 + i32::from(phase));
        assert_frame(model, &vars, [1357, 1358, 1357, 1358][phase as usize], true);

        // Mode writers are crate-private production transitions. Exercise the
        // same public selector policy here without fabricating task ownership.
        let help = actor_animation_selection(
            descriptor,
            phase,
            false,
            true,
            ActorAnimationSelectionInput {
                yaw_raw: 0x8000,
                linked_target_valid: false,
            },
        );
        assert_eq!(help.selector, carry.selector);
        assert!(help.zero_velocity);

        let death = actor_animation_selection(
            descriptor,
            phase,
            true,
            true,
            ActorAnimationSelectionInput {
                yaw_raw: 0,
                linked_target_valid: true,
            },
        );
        assert_eq!(death.selector, 38 + u16::from(phase));
        assert!(!death.zero_velocity);
        vars.dynamic[1] = i32::from(death.selector);
        assert_frame(model, &vars, 1360 + u16::from(phase), false);
    }

    // The stream also contains two unmirrored help frames, immediately before
    // death. Their existence must not shift special-mode's first selector 38.
    for (selector, sprite) in [(36, 1357), (37, 1358)] {
        let mut vars = AnimVars::default();
        vars.dynamic[1] = selector;
        assert_frame(model, &vars, sprite, false);
    }
}

#[v2k_test_support::retail_test]
fn man2_consumes_only_the_exact_sub_i_callback_binding() {
    let session = session();
    let mut controller = ActorAnimationController::from_descriptor(descriptor(&session)).unwrap();
    controller.advance_neutral(0, 0);
    let mut vars = AnimVars::default();
    vars.dynamic.fill(0x7654);
    vars.registers.fill(0x1234);
    controller.publish(&mut vars);
    for (index, value) in vars.dynamic.iter().copied().enumerate() {
        assert_eq!(value, if index == 1 { 24 } else { 0x7654 });
    }
    assert_eq!(vars.registers, [0x1234; 64]);
    assert_frame(model(&session), &vars, 1347, false);

    // There is no implicit modulo or clock fallback beyond the authored table.
    vars.dynamic[1] = 42;
    assert_eq!(
        model(&session).materialize(&vars).face_materials,
        [0x8000 | 1359; 2]
    );
}
