//! `FUN_00413BE0`'s transform-only pass against retail.
//!
//! A retail lab run (New Game, Intro2 left alone, 20 ms steps) logged each
//! transform-only pass at `0x00413BF6` (the actor's position and body basis
//! rows `+0C..+2F`) and each queued shot at its launch, `0x00414885`. These
//! shots were queued by actors in the broader-detail window only, so not
//! drawn; each launched with its time word `+20` zero, so its origin is the
//! muzzle the pass stamped. The Level-1 ant (Type47) carries its gun on its
//! root model, so its muzzle depends only on the logged pose. Type57's and
//! Type10's guns sit on children turned by joint words the log does not hold.

use v2k_formats::models::AnimVars;
use v2k_game::model_tree::ActorTransformPass;
use v2k_game::session::GameSession;
use v2k_game::sub_h_external_frame::SubHRuntimeState;

struct RetailShot {
    tick: u32,
    entity_type: usize,
    model_id: usize,
    position_raw: [i16; 3],
    basis_q31: [i32; 9],
    muzzle_raw: [i16; 3],
}

const SHOTS: [RetailShot; 2] = [
    RetailShot {
        tick: 1435,
        entity_type: 47,
        model_id: 302,
        position_raw: [-16373, 58, 5668],
        basis_q31: [
            -1048903680,
            138280960,
            1868300288,
            259063808,
            2131558400,
            -12451840,
            -1855389696,
            219480064,
            -1057882112,
        ],
        muzzle_raw: [-16528, 65, 5597],
    },
    RetailShot {
        tick: 1502,
        entity_type: 47,
        model_id: 302,
        position_raw: [-16354, 48, 5680],
        basis_q31: [
            -1311965184,
            195756032,
            1688141824,
            266993664,
            2130182144,
            -39518208,
            -1678376960,
            185860096,
            -1325858816,
        ],
        muzzle_raw: [-16497, 53, 5587],
    },
];

#[v2k_test_support::retail_test]
fn undrawn_actors_shots_leave_from_retails_transform_pass_muzzles() {
    let dir = v2k_test_support::retail_dir();
    {
        let mut session = GameSession::init(&dir).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_auxiliary_ovl(5, 1).unwrap();
        let cache = &session.cache;
        for shot in &SHOTS {
            let record = cache.global_entity_type(shot.entity_type).unwrap();
            let descriptor = record.sub_h_external_frame_descriptor();
            let mut runtime = descriptor
                .as_ref()
                .map(|descriptor| SubHRuntimeState::new(descriptor.records.len()).unwrap());
            let b = shot.basis_q31;
            let stamps = ActorTransformPass {
                cache,
                model_id: shot.model_id,
                vars: &AnimVars::default(),
                origin_raw: shot.position_raw,
                body_axes_q31: [[b[0], b[1], b[2]], [b[3], b[4], b[5]], [b[6], b[7], b[8]]],
                surface: None,
                retail_tick: shot.tick,
                sub_h: runtime.as_mut().zip(descriptor.as_ref()),
                emitter: record.projectile_emitter_descriptor(),
                fallback: None,
            }
            .run()
            .unwrap();
            assert_eq!(stamps, [Some(shot.muzzle_raw), None], "tick {}", shot.tick);
        }
    }
}
