//! Real-data checks for the fullscreen world colours queued by FUN_0042F270.

use std::num::NonZeroU64;

use v2k_game::main_base_abort::MainBaseAbortControllerStorage;
use v2k_game::session::GameSession;

#[v2k_test_support::retail_test]
fn every_level_sky_colour_resolves_in_the_master_palette() {
    let dir = v2k_test_support::retail_dir();
    for level in 13u32..=50 {
        let mut session = GameSession::init(&dir).unwrap();
        session.load_auxiliary_ovl(3, 0).unwrap();
        session.load_level_by_id(level, 0).unwrap();
        let desc = session.cache.level_desc().unwrap();
        let palette = session.cache.master_color_palette().unwrap();
        let entry = palette
            .get(desc.sky_color_index as usize)
            .unwrap_or_else(|| {
                panic!(
                    "level {level}: sky colour {} outside {}-entry master palette",
                    desc.sky_color_index,
                    palette.len()
                )
            });
        eprintln!(
            "level {level}: sky={} rgb=({},{},{}) model={}",
            desc.sky_color_index, entry.r, entry.g, entry.b, desc.sky_model
        );
        assert!(v2k_game::sky::build_sky_background(&session.cache).is_some());
        assert!(v2k_game::sky::build_sky_background_for_index(
            &session.cache,
            desc.main_base_abort_sky_color_index,
        )
        .is_some());
        if level == 13 {
            assert_eq!(
                (
                    desc.sky_color_index,
                    desc.sky_model,
                    desc.main_base_abort_sky_color_index,
                    desc.main_base_abort_sky_model,
                ),
                (27, 305, 32, 0),
            );
            let normal = v2k_game::sky::build_sky_background(&session.cache).unwrap();
            let abort = v2k_game::sky::build_sky_background_for_index(
                &session.cache,
                desc.main_base_abort_sky_color_index,
            )
            .unwrap();
            assert_ne!(normal.color, abort.color);
            let abort_entry = palette
                .get(desc.main_base_abort_sky_color_index as usize)
                .unwrap();
            assert_eq!((abort_entry.r, abort_entry.g, abort_entry.b), (0, 0, 0),);
            eprintln!(
                "level {level}: abort sky rgb=({},{},{})",
                abort_entry.r, abort_entry.g, abort_entry.b,
            );
            assert_eq!(
                abort.color,
                [
                    abort_entry.r as f32 / 255.0,
                    abort_entry.g as f32 / 255.0,
                    abort_entry.b as f32 / 255.0,
                ]
            );

            let mut progress = v2k_game::power_up_contact::PlayerCampaignProgress::new();
            progress.set_current_control_slot(Some(1));
            let mut controller = MainBaseAbortControllerStorage::from_loaded_level(
                NonZeroU64::new(1).unwrap(),
                desc,
                false,
                &mut progress,
            )
            .unwrap()
            .0;
            let normal_request = controller.submitted_frame_request();
            assert_eq!(
                (
                    normal_request.callback_address,
                    normal_request.retained_dword_0xac,
                    normal_request.word_0xb0,
                    normal_request.word_0xb2,
                    normal_request.dword_0xb4,
                    normal_request.dword_0xb8,
                    normal_request.dword_0xbc,
                ),
                (0x0042_E860, 0, 27, 305, 0x650, 21, 8),
            );
            assert_eq!(
                v2k_game::sky::submitted_sky_model_id(&session.cache, Some(normal_request),),
                Some(305),
            );
            let abort_request = controller
                .commit_post_terrain_abort(controller.world_control_lease())
                .unwrap();
            assert_eq!(
                (
                    abort_request.callback_address,
                    abort_request.retained_dword_0xac,
                    abort_request.word_0xb0,
                    abort_request.word_0xb2,
                    abort_request.dword_0xb4,
                    abort_request.dword_0xb8,
                    abort_request.dword_0xbc,
                ),
                (0x0042_E860, 0, 32, 0, 0x650, 21, 8),
            );
            assert_eq!(
                v2k_game::sky::submitted_sky_model_id(&session.cache, Some(abort_request),),
                None,
            );
        }
        if level == 14 {
            assert_eq!((desc.sky_color_index, desc.sky_model), (27, 306));
        }
        if desc.sky_model != 0 {
            let model = session
                .cache
                .global_model(desc.sky_model as usize)
                .unwrap_or_else(|| {
                    panic!("level {level}: sky model {} unresolved", desc.sky_model)
                });
            eprintln!("level {level}: sky model name={:?}", model.name);
        }
    }
}
