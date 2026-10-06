//! Real-data guard for the common Intro2 actor model callback.

use v2k_formats::models::{MaterializedModel, ModelEntry};
use v2k_game::opening::intro_actor_anim_vars;
use v2k_game::search_attack_live::TYPE13_SEARCH_ATTACK_SUB_G_ANIMATION_BINDINGS;
use v2k_game::session::GameSession;

fn intro_actor_session() -> GameSession {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("retail fixture must load");
    // The accepted trace resolves these actors from the normal 640x480
    // system tier. Variant 0 is the authored low-resolution presentation.
    session
        .load_auxiliary_ovl(3, 1)
        .expect("retail fixture must load");
    session
}

fn first_world_actor_session() -> GameSession {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("retail fixture must load");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("retail fixture must load");
    session
        .load_level_by_id(13, 1)
        .expect("retail fixture must load");
    session
}

fn presentation_differs(left: &MaterializedModel, right: &MaterializedModel) -> bool {
    left.vertices != right.vertices
        || left.triangles != right.triangles
        || left.face_materials != right.face_materials
        || left.billboards.len() != right.billboards.len()
        || left.billboards.iter().zip(&right.billboards).any(|(a, b)| {
            (a.vertex, a.slot, a.id, a.size, a.angle, a.textured)
                != (b.vertex, b.slot, b.id, b.size, b.angle, b.textured)
        })
        || left.instances.len() != right.instances.len()
        || left.instances.iter().zip(&right.instances).any(|(a, b)| {
            a.model_id != b.model_id
                || a.attach_slot != b.attach_slot
                || a.attach_pos != b.attach_pos
                || a.orientation != b.orientation
                || a.linked_slots != b.linked_slots
                || a.registers != b.registers
        })
}

fn has_tick_driven_presentation(model: &ModelEntry, entity_type: u32) -> bool {
    let first = model.materialize(&intro_actor_anim_vars(entity_type, 0));
    (1..=256).any(|tick| {
        let frame = model.materialize(&intro_actor_anim_vars(entity_type, tick));
        presentation_differs(&first, &frame)
    })
}

fn model<'a>(session: &'a GameSession, global_id: usize, expected_name: &str) -> &'a ModelEntry {
    let model = session
        .cache
        .global_model(global_id)
        .unwrap_or_else(|| panic!("missing Intro2 model {global_id}"));
    assert_eq!(model.name.as_deref(), Some(expected_name));
    model
}

#[v2k_test_support::retail_test]
fn captured_intro2_ground_actors_consume_the_retail_tick_channel() {
    let session = intro_actor_session();

    for (entity_type, global_id, expected_name) in [(47, 302, "newant"), (26, 267, "stag")] {
        assert!(
            has_tick_driven_presentation(model(&session, global_id, expected_name), entity_type),
            "{expected_name} ignored callback selector zero across 256 retail ticks"
        );
    }
}

#[v2k_test_support::retail_test]
fn ordinary_gameplay_actor_models_identify_their_non_clock_joint_channels() {
    let session = first_world_actor_session();

    for (global_id, expected_name, expected_channels) in
        [(558, "man2", &[1][..]), (256, "spider", &[][..])]
    {
        let model = model(&session, global_id, expected_name);
        let base = model.materialize(&intro_actor_anim_vars(34, 0));
        let mut consumed_channels = Vec::new();
        for index in 0..64 {
            let mut vars = intro_actor_anim_vars(34, 0);
            vars.dynamic[index] = 0x4000;
            if presentation_differs(&base, &model.materialize(&vars)) {
                consumed_channels.push(index);
            }
        }
        assert_eq!(
            consumed_channels, expected_channels,
            "{expected_name} consumed an unexpected animation-variable channel set"
        );
    }
}

#[v2k_test_support::retail_test]
fn captured_flying_insect_uses_authored_sub_g_wing_channels() {
    let session = intro_actor_session();

    let ptersect = model(&session, 291, "ptersect");
    let sub_g = session
        .cache
        .global_entity_type(13)
        .and_then(|record| record.subsection("G"))
        .expect("Ptersect Sub-G");
    assert_eq!(&sub_g[0x24..0x2b], &[1, 2, 3, 4, 5, 8, 9]);
    assert_eq!(
        TYPE13_SEARCH_ATTACK_SUB_G_ANIMATION_BINDINGS,
        [1, 2, 3, 4, 5, 8, 9]
    );
    assert_eq!(
        &sub_g[0x2c..0x38],
        &[0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00],
        "selector one is the authored +/-0x2000 sine channel"
    );
    assert_eq!(&sub_g[0x38..0x44], &[0; 12]);
    assert_eq!(&sub_g[0x44..0x50], &[0; 12]);
    assert_eq!(
        &sub_g[0x50..0x5c],
        &[0x00, 0xf0, 0x00, 0x11, 0xe6, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00],
        "selector four is the authored -0x1000 +/-0x1100 sine channel"
    );
    assert_eq!(
        &sub_g[0x5c..0x68],
        &[0x00, 0x00, 0x00, 0x10, 0xd2, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00],
        "selector five is the authored +/-0x1000 sine channel"
    );
    assert!(!has_tick_driven_presentation(ptersect, 13));
    let base = ptersect.materialize(&intro_actor_anim_vars(13, 0));
    let mut consumed_channels = Vec::new();
    for index in 1..64 {
        let mut vars = intro_actor_anim_vars(13, 0);
        vars.dynamic[index] = 0x4000;
        if presentation_differs(&base, &ptersect.materialize(&vars)) {
            consumed_channels.push(index);
        }
    }
    // The normal-tier model additionally rotates its mounts around Y through
    // selectors 6 and 8. Retail FUN_004671D0 composes these opcode 0x1C
    // rotations with the preceding opcode 0x5C mount orientation.
    assert_eq!(&ptersect.cmd_words[0xd4..0xd7], &[0x1c, 1, 0x86]);
    assert_eq!(&ptersect.cmd_words[0xe5..0xe8], &[0x1c, 1, 0x88]);
    assert_eq!(consumed_channels, [1, 6, 7, 8, 10]);
}
