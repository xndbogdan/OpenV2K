use super::*;
use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::intro2_type47_live::world::native_intro2_fixture;
use crate::session::GameSession;

fn fixture(spawn: usize) -> Option<(GameSession, EntityManager, u32)> {
    let (session, mut manager, _) = native_intro2_fixture()?;
    let id = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(spawn))
        .unwrap()
        .id;
    let entity = manager.entity_mut(id).unwrap();
    // Controlled local, detailed visit; the allocation and component custody
    // remain those of this Intro2 birth, including Type26's own Sub-D owner.
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(u32::MAX, 0x68000);
    entity.physical_body_basis_q31 = RetailRuntimeValue::Known(Type9BodyBasis {
        lateral: [i32::MAX, 0, 0],
        up: [0, i32::MAX, 0],
        forward: [0, 0, i32::MAX],
    });
    entity.set_motion_raw([100, -10_000, 300], [7, 8, 9]);
    Some((session, manager, id))
}

fn surface(
    session: &GameSession,
    manager: &mut EntityManager,
    id: u32,
    dt: u32,
    fx: &mut WorldFx,
    death: &mut Option<Intro2CommonDyingOwner>,
) -> Result<(), Intro2CommonDyingBlock> {
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    let metadata = manager
        .type_runtime_metadata(entity.entity_type)
        .unwrap()
        .clone();
    let radius = session
        .cache
        .global_model(entity.model_index.unwrap())
        .unwrap()
        .radius;
    let RetailRuntimeValue::Known(effects) = metadata.common_world_effects else {
        panic!("authored surface profile")
    };
    assert_eq!(effects.surface_selectors, [1, 0]);
    assert_eq!(effects.surface_lifetime_ms, authored_lifetime(manager, id));
    run_living_actor_surface(
        manager,
        id,
        Intro2ActorSurfaceFrame {
            metadata: &metadata,
            terrain: session.cache.level_terrain().unwrap(),
            active_model_extent_raw: radius,
            elapsed_micros: dt,
            retail_tick: 17,
            particle_environment: ParticleEnvironment::Dry,
        },
        fx,
        death,
    )
}

fn authored_lifetime(manager: &EntityManager, id: u32) -> u32 {
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    let RetailRuntimeValue::Known(effects) = manager
        .type_runtime_metadata(entity.entity_type)
        .unwrap()
        .common_world_effects
    else {
        panic!("authored native surface profile")
    };
    assert_eq!(
        effects.surface_lifetime_ms,
        match entity.entity_type {
            26 => 30_000,
            53 => 2_000,
            _ => panic!("fixture requires one of the two admitted native types"),
        }
    );
    effects.surface_lifetime_ms
}

#[v2k_test_support::retail_test]
fn intro2_living_surface_deep_threshold_and_75_percent_preserve_rng_for_both_native_types() {
    for spawn in [10, 38] {
        let Some((session, mut manager, id)) = fixture(spawn) else {
            return;
        };
        let lifetime = authored_lifetime(&manager, id);
        let entity = manager.entity_mut(id).unwrap();
        let radius = session
            .cache
            .global_model(entity.model_index.unwrap())
            .unwrap()
            .radius;
        let threshold =
            session.cache.level_terrain().unwrap().sea_level_raw() - (radius >> 2) as i16;
        entity.set_motion_raw([100, threshold, 300], [7, 8, 9]);
        entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(10);
        let mut fx = WorldFx::new();
        let mut death = None;
        surface(&session, &mut manager, id, 20_999, &mut fx, &mut death).unwrap();
        assert_eq!(
            manager
                .entity_mut(id)
                .unwrap()
                .surface_lifetime_timer_ms_at_0x48,
            RetailRuntimeValue::Known(0),
            "equality is nondeep and timer decay saturates"
        );

        let entity = manager.entity_mut(id).unwrap();
        entity.set_motion_raw([100, threshold - 1, 300], [7, 8, 9]);
        entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(lifetime / 4 - 20);
        surface(&session, &mut manager, id, 20_999, &mut fx, &mut death).unwrap();
        assert_eq!(
            manager
                .entity_mut(id)
                .unwrap()
                .surface_lifetime_timer_ms_at_0x48,
            RetailRuntimeValue::Known(lifetime / 4)
        );
        assert!(death.is_none());
        assert_eq!(fx.particle_count(), 0);
        assert_eq!(fx.pending_event_count(), 0);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            38,
            "75 percent does not enter RNG"
        );
    }
}

#[v2k_test_support::retail_test]
fn intro2_living_surface_first_random_window_commits_timer_and_both_missed_gates() {
    for spawn in [10, 38] {
        let Some((session, mut manager, id)) = fixture(spawn) else {
            return;
        };
        let lifetime = authored_lifetime(&manager, id);
        manager
            .entity_mut(id)
            .unwrap()
            .surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(lifetime / 4 - 20);
        let mut fx = WorldFx::new();
        let mut death = None;
        surface(&session, &mut manager, id, 21_000, &mut fx, &mut death).unwrap();
        assert_eq!(
            manager
                .entity_mut(id)
                .unwrap()
                .surface_lifetime_timer_ms_at_0x48,
            RetailRuntimeValue::Known(lifetime / 4 + 1)
        );
        // 74%: divisor18. First word38 misses the bubble; word7719 misses
        // the independent sound gate. Neither gate rolls the timer back.
        assert_eq!(fx.next_shared_retail_random_u16(), 54006);
        assert_eq!(fx.particle_count(), 0);
        assert_eq!(fx.pending_event_count(), 0);
        assert!(death.is_none());
    }
}

#[v2k_test_support::retail_test]
fn intro2_living_surface_exact_expiry_constructs_before_random_and_overshoot_skips_random() {
    for spawn in [10, 38] {
        for extra_ms in [0, 1] {
            let Some((session, mut manager, id)) = fixture(spawn) else {
                return;
            };
            let lifetime = authored_lifetime(&manager, id);
            let entity = manager.entity_mut(id).unwrap();
            entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(lifetime - 20);
            entity.collision.default_state_flags_at_0xc8 = RetailRuntimeValue::Known(0);
            entity.attached_to = Some(999);
            entity
                .collision
                .state_flags_at_0x08
                .overwrite(0x1000, 0x1000);
            let mut fx = WorldFx::new();
            let mut death = None;
            surface(
                &session,
                &mut manager,
                id,
                20_000 + extra_ms * 1000,
                &mut fx,
                &mut death,
            )
            .unwrap();
            let owner = death.expect("162B0 retains the new class12 owner");
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(
                entity.surface_lifetime_timer_ms_at_0x48,
                RetailRuntimeValue::Known(lifetime + extra_ms)
            );
            assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
            assert_eq!(
                entity.collision.default_state_flags_at_0xc8,
                RetailRuntimeValue::Known(if spawn == 10 { 0x439 } else { 0x39 })
            );
            assert_eq!(entity.attached_to, None);
            assert_eq!(
                entity
                    .collision
                    .state_flags_at_0x08
                    .masked(0x1000 | 0x800 | 0x4000),
                RetailRuntimeValue::Known(0x4800)
            );
            let Some(ActorTaskRuntime::CommonDying(task)) =
                entity.actor_tasks.task_state(owner.visit.task_id)
            else {
                panic!("published Primary")
            };
            assert_eq!(
                task.elapsed_ms(),
                0,
                "new Primary is not visited during the old world tail"
            );
            assert_eq!(entity.velocity_raw(), [7, 500, 9]);
            assert_eq!(fx.particle_count(), 0);
            // Constructor consumes38 first. At exact expiry bubble7719
            // misses divisor2; dying skips the sound gate. Overshoot returns
            // its unsigned wrapped percentage before either random gate.
            assert_eq!(
                fx.next_shared_retail_random_u16(),
                if extra_ms == 0 { 54006 } else { 7719 }
            );
            fx.process_pending();
            assert!(fx
                .take_positional_sounds()
                .iter()
                .all(|sound| sound.sound_id != 106));
            assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
        }
    }
}

#[v2k_test_support::retail_test]
fn intro2_living_surface_expiry_bubble_uses_cached_extent_and_live_native_provenance() {
    for spawn in [10, 38] {
        let Some((session, mut manager, id)) = fixture(spawn) else {
            return;
        };
        let lifetime = authored_lifetime(&manager, id);
        let entity = manager.entity_mut(id).unwrap();
        let entity_type = entity.entity_type as u8;
        let radius = session
            .cache
            .global_model(entity.model_index.unwrap())
            .unwrap()
            .radius;
        entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(lifetime);
        let mut fx = WorldFx::new();
        assert_eq!(fx.next_shared_retail_random_u16(), 38);
        let mut death = None;
        surface(&session, &mut manager, id, 0, &mut fx, &mut death).unwrap();
        assert!(death.is_some());
        // Constructor7719, accepted gate54006, XYZ jitter2437/41623/11797,
        // then X/Z velocity8365/32285. Class42 allocation is synchronous.
        assert_eq!(fx.next_shared_retail_random_u16(), 43218);
        assert_eq!(fx.particle_count(), 1);
        let presentation = fx.prepare_presentation([640, 480], 0x4000, |_| {
            v2k_render::ParticleCenterProjection {
                screen: [320, 240],
                depth_raw: 0x100,
                clip: 0,
            }
        });
        let particle = presentation.particles().next().unwrap().particle;
        assert_eq!(particle.source_class, 42);
        assert_eq!(particle.owner_id, Some(id));
        assert_eq!(particle.source_entity_type_at_birth, Some(entity_type));
        assert_eq!(
            particle.position.map(|value| (value * 256.0) as i16),
            [104, -9919, 300 + radius as i16 - 1 + 23]
        );
        assert_eq!(
            particle
                .velocity
                .map(|value| (value * 256.0 * (1_048_576.0 / 1_000_000.0)).round() as i32),
            [65, 400, 252]
        );
        fx.process_pending();
        assert!(fx
            .take_positional_sounds()
            .iter()
            .all(|sound| sound.sound_id != 106));
    }
}

#[v2k_test_support::retail_test]
fn intro2_living_surface_late_basis_block_preserves_committed_death_and_rng_prefix() {
    let Some((session, mut manager, id)) = fixture(38) else {
        return;
    };
    let entity = manager.entity_mut(id).unwrap();
    entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(2000);
    entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    let mut fx = WorldFx::new();
    assert_eq!(fx.next_shared_retail_random_u16(), 38);
    let mut death = None;
    assert_eq!(
        surface(&session, &mut manager, id, 0, &mut fx, &mut death),
        Err(Intro2CommonDyingBlock::Runtime("surface emission basis"))
    );
    let retained = death.expect("a later failure cannot lose the native class12 owner");
    assert_eq!(retained.entity_id(), id);
    assert_eq!(
        Intro2CommonDyingOwner::adopt(&manager, id).unwrap(),
        retained
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        2437,
        "constructor7719 and successful gate54006 commit before the missing basis"
    );
    assert_eq!(
        manager.entity_mut(id).unwrap().collision.health_raw,
        RetailRuntimeValue::Known(0)
    );
    assert_eq!(fx.particle_count(), 0);
}

#[v2k_test_support::retail_test]
fn intro2_living_surface_missed_bubble_does_not_read_unknown_basis() {
    let Some((session, mut manager, id)) = fixture(38) else {
        return;
    };
    let entity = manager.entity_mut(id).unwrap();
    entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(501);
    entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    let mut fx = WorldFx::new();
    let mut death = None;
    surface(&session, &mut manager, id, 0, &mut fx, &mut death).unwrap();
    assert!(death.is_none());
    assert_eq!(fx.particle_count(), 0);
    assert_eq!(fx.pending_event_count(), 0);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        54006,
        "bubble38 misses divisor18 and the independent sound7719 still executes"
    );
}
