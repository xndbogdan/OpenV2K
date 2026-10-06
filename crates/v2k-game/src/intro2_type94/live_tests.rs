use super::*;
use crate::{
    entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    intro2_type47_live::world::native_intro2_fixture, sub_h_external_frame::SubHSurfacePolicy,
    world_fx::WorldFx,
};

#[v2k_test_support::retail_test]
fn native_type94_water_actor_advances_real_owner_in_both_scheduler_modes() {
    for detailed in [true, false] {
        let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(43))
            .unwrap()
            .id;
        let entity = manager.entity_mut(id).unwrap();
        assert!(intro2_type94_allocation_authenticates(entity));
        let before = entity.position_raw();
        let RetailRuntimeValue::Known(Some(h)) = &entity.sub_h_external_frame_runtime else {
            panic!()
        };
        assert_eq!(h.surface_policy(), SubHSurfacePolicy::TerrainAndWater);
        entity.collision.state_flags_at_0x08.overwrite(
            u32::MAX,
            if detailed {
                0x68000 | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
            } else {
                0x68000 & !SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
            },
        );
        let mut owner = Intro2Type94Owner::adopt(&manager, id).unwrap();
        let mut fx = WorldFx::new();
        let mut elapsed = 0_u32;
        let mut advanced = 0;
        for index in 0..96 {
            let dt = [1_000, 16_667, 40_000, 125_000][index % 4];
            elapsed += dt;
            let tick = tick_intro2_type94(
                &mut manager,
                owner,
                Intro2Type94Frame {
                    resources: &mut session.cache,
                    world_fx: &mut fx,
                    elapsed_micros: dt,
                    retail_tick: elapsed / 20_000,
                },
            );
            match tick.outcome {
                Intro2Type94Outcome::Advanced { .. } => advanced += 1,
                Intro2Type94Outcome::Waiting { .. } => {}
                other => panic!("detailed={detailed} visit={index}: {other:?}"),
            }
            assert!(tick.replacement_common_dying_owner.is_none());
            owner = tick
                .retained_owner
                .expect("living allocation retains its exact owner");
        }
        assert!(advanced > 0);
        let entity = manager.entity_mut(id).unwrap();
        assert_ne!(entity.position_raw(), before);
        assert_eq!(entity.model_slots, [Some(272); 4]);
        assert_eq!(
            entity.collision.health_raw,
            RetailRuntimeValue::Known(10000)
        );
        assert_eq!(
            entity.surface_lifetime_timer_ms_at_0x48,
            RetailRuntimeValue::Known(0)
        );
        let RetailRuntimeValue::Known(Some(h)) = &entity.sub_h_external_frame_runtime else {
            panic!()
        };
        assert_eq!(h.surface_policy(), SubHSurfacePolicy::TerrainAndWater);
    }
}
