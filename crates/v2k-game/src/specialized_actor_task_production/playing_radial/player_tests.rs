use super::*;
use crate::{
    damage::{DamagePacket, TYPE_46_DAMAGE_PROFILE},
    entity::{EntityConstructionResources, PlayerCheckedDamageBlock},
    entity_collision_state::{
        EntityTypeRuntimeMetadata, RetailRuntimeValue, RetailStateWord,
        CHECKED_DAMAGE_ENABLED_STATE_BIT, DYING_STATE_BIT, PAIR_COLLISION_FIXED_STATE_BIT,
        PAIR_COLLISION_INELIGIBLE_STATE_BIT, RADIAL_DAMAGE_ALTERNATE_ELIGIBLE_STATE_BIT,
    },
    radial_damage::scale_radial_damage,
    session::GameSession,
    static_damage::StaticDamageScheduler,
};

struct Fixture {
    session: GameSession,
    entities: EntityManager,
    hull: PlayerHull,
    fx: WorldFx,
    notifications: GameplayNotifications,
}

impl Fixture {
    fn new(health: i32, buffer: i32, flags: u32) -> Self {
        let root = v2k_test_support::retail_dir();
        let mut session = GameSession::init(&root).expect("retail corpus required");
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(13, 1).unwrap();
        let metadata = session
            .cache
            .global_entity_model_table()
            .iter()
            .enumerate()
            .map(|(kind, model_slots)| {
                session
                    .cache
                    .global_entity_type(kind)
                    .map(EntityTypeRuntimeMetadata::from_section12)
                    .unwrap_or(EntityTypeRuntimeMetadata {
                        model_slots: *model_slots,
                        ..Default::default()
                    })
            })
            .collect::<Vec<_>>();
        let mut fx = WorldFx::new();
        let mut entities = EntityManager::from_fresh_new_game_level_with_type_metadata(
            session.cache.level_desc().unwrap(),
            &metadata,
            EntityConstructionResources::new(
                session.cache.terrain(),
                session.cache.terrain_objects(),
            ),
            0,
            &mut fx,
        )
        .unwrap();
        let player = entities.player_mut().unwrap();
        assert_eq!(player.model_slots, [Some(41), Some(67), Some(41), Some(67)]);
        player.collision.state_flags_at_0x08 = RetailStateWord::exact(flags);
        player.collision.health_raw = RetailRuntimeValue::Known(health);
        player.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(buffer);
        player.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
        player.set_position_raw([0, 10_000, 0]);
        player.set_velocity_raw([i16::MAX, -203, 901]);
        let mut hull = PlayerHull::default();
        hull.health_raw = health;
        hull.pre_health_damage_buffer_raw = buffer;
        hull.dying = flags & DYING_STATE_BIT != 0;
        let mut fx = WorldFx::new();
        // A reached active-world death has the genuine full-rate history;
        // leave cold-start pacing policy in the production constructor.
        for _ in 0..8 {
            fx.advance_frame_pacing(20_000);
        }
        Self {
            session,
            entities,
            hull,
            fx,
            notifications: GameplayNotifications::new(),
        }
    }

    fn visit(
        &mut self,
        origin_raw: [i16; 3],
        template: RadialDamageTemplate,
        lives: RetailRuntimeValue<u8>,
    ) -> Result<bool, PlayerRadialBlock> {
        let id = self.entities.player().unwrap().id;
        player::apply_player_radial_target(
            &mut self.entities,
            id,
            player::PlayerRadialContext {
                player_hull: &mut self.hull,
                extra_lives: lives,
                origin_raw,
                template,
                world_fx: &mut self.fx,
                notifications: &mut self.notifications,
                retail_tick: 81,
                resources: &self.session.cache,
            },
        )
    }
}

fn template() -> RadialDamageTemplate {
    RadialDamageTemplate {
        inner_radius_raw: 128,
        outer_radius_raw: 256,
        impulse_raw: 128,
        packet: DamagePacket {
            channels: [1, 3],
            amounts_raw: [8_000, 1_000],
        },
        trailing_raw: [-2, 0x12345678],
    }
}

#[v2k_test_support::retail_test]
fn playing_radial_player_retains_full_falloff_outer_and_impulse_policy() {
    for (distance, flags, mass) in [
        (0, CHECKED_DAMAGE_ENABLED_STATE_BIT, 100),
        (128, CHECKED_DAMAGE_ENABLED_STATE_BIT, 100),
        (192, CHECKED_DAMAGE_ENABLED_STATE_BIT, 100),
        (256, CHECKED_DAMAGE_ENABLED_STATE_BIT, 100),
        (
            128,
            CHECKED_DAMAGE_ENABLED_STATE_BIT | PAIR_COLLISION_FIXED_STATE_BIT,
            100,
        ),
        (128, CHECKED_DAMAGE_ENABLED_STATE_BIT, 1),
        (
            128,
            CHECKED_DAMAGE_ENABLED_STATE_BIT | PAIR_COLLISION_INELIGIBLE_STATE_BIT,
            100,
        ),
        (
            128,
            CHECKED_DAMAGE_ENABLED_STATE_BIT
                | PAIR_COLLISION_INELIGIBLE_STATE_BIT
                | RADIAL_DAMAGE_ALTERNATE_ELIGIBLE_STATE_BIT,
            100,
        ),
        (128, 0, 100),
    ] {
        let mut f = Fixture::new(40_000, 0, flags);
        f.entities.player_mut().unwrap().mass_raw = mass;
        let before = f.entities.player().unwrap().velocity_raw();
        let origin = [-distance, 10_000, 0];
        let eligible = flags & CHECKED_DAMAGE_ENABLED_STATE_BIT != 0
            && (flags & PAIR_COLLISION_INELIGIBLE_STATE_BIT == 0
                || flags & RADIAL_DAMAGE_ALTERNATE_ELIGIBLE_STATE_BIT != 0);
        let impulse = mass >= 2 && flags & PAIR_COLLISION_FIXED_STATE_BIT == 0;
        let expected = eligible
            .then(|| scale_radial_damage(template(), origin, [0, 10_000, 0], impulse))
            .flatten();
        assert_eq!(
            f.visit(origin, template(), RetailRuntimeValue::Known(2)),
            Ok(expected.is_some())
        );
        let damage = expected.map_or(0, |scaled| {
            scaled.packet.filtered_raw(Some(&TYPE_46_DAMAGE_PROFILE))
        });
        assert_eq!(f.hull.health_raw, 40_000 - damage);
        let expected_velocity = expected
            .and_then(|scaled| scaled.impulse_vector_raw)
            .map_or(before, |impulse| {
                std::array::from_fn(|axis| before[axis].wrapping_add(impulse[axis]))
            });
        assert_eq!(
            f.entities.player().unwrap().velocity_raw(),
            expected_velocity
        );
        assert_eq!(
            f.entities
                .player()
                .unwrap()
                .collision
                .last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(17)
        );
        assert!(f.entities.player_dying_contact_runtime().is_none());
    }
}

#[v2k_test_support::retail_test]
fn playing_radial_player_dying_and_filtered_zero_keep_native_asymmetries() {
    let mut f = Fixture::new(
        0,
        10_000,
        CHECKED_DAMAGE_ENABLED_STATE_BIT | DYING_STATE_BIT,
    );
    assert_eq!(
        f.visit([0, 10_000, 0], template(), RetailRuntimeValue::Known(1)),
        Ok(true)
    );
    assert_eq!(
        (f.hull.health_raw, f.hull.pre_health_damage_buffer_raw),
        (0, 6_000)
    );
    assert_eq!(f.fx.particle_count(), 0);
    assert!(f.fx.take_positional_sounds().is_empty());
    assert!(f.entities.player_dying_contact_runtime().is_none());
    for distance in [0, 192] {
        let mut f = Fixture::new(
            40_000,
            0,
            CHECKED_DAMAGE_ENABLED_STATE_BIT | PAIR_COLLISION_FIXED_STATE_BIT,
        );
        f.entities.player_mut().unwrap().capability_flags |= 8;
        let zero = RadialDamageTemplate {
            packet: DamagePacket {
                channels: [5, 0],
                amounts_raw: [1000, 0],
            },
            trailing_raw: [46, 0],
            ..template()
        };
        let result = f.visit([-distance, 10_000, 0], zero, RetailRuntimeValue::Known(1));
        if distance == 0 {
            assert!(matches!(
                result,
                Err(PlayerRadialBlock {
                    reason: PlayerRadialBlockReason::Damage(
                        PlayerCheckedDamageBlock::PlayerFeedback
                    ),
                    target_prefix_committed: false,
                    ..
                })
            ));
        } else {
            assert_eq!(
                result,
                Ok(true),
                "14E10 omits checked zero-filter feedback but keeps255E0"
            );
        }
        assert_eq!(f.hull.health_raw, 40_000);
        assert_eq!(f.fx.particle_count(), 0);
    }
}

#[v2k_test_support::retail_test]
fn playing_radial_player_missing_context_blocks_without_numeric_fallback_or_impulse() {
    let mut f = Fixture::new(1, 0, CHECKED_DAMAGE_ENABLED_STATE_BIT);
    let before = f.entities.player().unwrap().velocity_raw();
    let collision = f.entities.player().unwrap().collision.clone();
    let result = f.visit(
        [-128, 10_000, 0],
        template(),
        RetailRuntimeValue::Unresolved,
    );
    assert!(matches!(
        result,
        Err(PlayerRadialBlock {
            reason: PlayerRadialBlockReason::ExtraLivesUnavailable,
            target_prefix_committed: false,
            ..
        })
    ));
    assert_eq!(f.entities.player().unwrap().velocity_raw(), before);
    assert_eq!(f.entities.player().unwrap().collision, collision);
    assert_eq!(f.fx.particle_count(), 0);
}

#[v2k_test_support::retail_test]
fn playing_radial_player_death_finishes_native_constructor_before_return() {
    for lives in [0, 2] {
        let mut f = Fixture::new(1, 0, CHECKED_DAMAGE_ENABLED_STATE_BIT);
        f.entities
            .player_mut()
            .unwrap()
            .collision
            .generic_hit_sound_id = RetailRuntimeValue::Known(Some(33));
        f.entities.player_mut().unwrap().collision.death_sound_id =
            RetailRuntimeValue::Known(Some(34));
        assert_eq!(
            f.visit([0, 10_000, 0], template(), RetailRuntimeValue::Known(lives)),
            Ok(true)
        );
        assert_eq!(f.hull.health_raw, 0);
        assert!(f.hull.dying);
        assert_eq!(f.entities.player().unwrap().model_index, Some(67));
        assert!(f.entities.player_dying_contact_runtime().is_some());
        assert_eq!(f.fx.particle_count(), 11);
        let sounds =
            f.fx.take_positional_sounds()
                .iter()
                .map(|sound| sound.sound_id)
                .collect::<Vec<_>>();
        assert_eq!(sounds[..2], [33, 34]);
        let mut expected_notifications = GameplayNotifications::new();
        expected_notifications.queue_player_destroyed(lives, 81);
        assert_eq!(f.notifications, expected_notifications);
    }
}

#[v2k_test_support::retail_test]
fn attached_native_class49_nested_radial_completes_real_player_death_before_return() {
    let (mut session, mut entities, _) = crate::intro2_gun_turret::authored_tests::fixture(15);
    entities.cleanup_pending_actor_deferred_destroys();
    let turret = entities
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(1))
        .unwrap()
        .id;
    let metadata =
        EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(46).unwrap());
    entities.place_or_spawn_player_at_campaign_arrival(
        Some(&metadata),
        [200, 10_000, -300],
        0,
        session.cache.terrain(),
    );
    let player_id = entities.player().unwrap().id;
    let ids = entities.retail_live_order_ids().collect::<Vec<_>>();
    for id in ids {
        let entity = entities.entity_mut(id).unwrap();
        entity.collision.state_flags_at_0x08 = RetailStateWord::exact(if id == turret {
            0x0c06_8000
        } else if id == player_id {
            CHECKED_DAMAGE_ENABLED_STATE_BIT
        } else {
            0
        });
        entity.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
    }
    entities
        .entity_mut(turret)
        .unwrap()
        .set_position_raw([100, 10_000, -300]);
    entities.entity_mut(turret).unwrap().collision.health_raw = RetailRuntimeValue::Known(1);
    let player = entities.player_mut().unwrap();
    player.collision.health_raw = RetailRuntimeValue::Known(1);
    player.current_behavior_context = RetailRuntimeValue::Known(Some(
        crate::entity_behavior::BehaviorContextRuntime::from_fresh_weighted_selection(
            crate::entity_behavior::BehaviorSelection {
                choice_index: 0,
                program: crate::entity_behavior::audited_behavior_program(24).unwrap(),
            },
        )
        .unwrap(),
    ));
    let mut hull = PlayerHull::default();
    hull.health_raw = 1;
    hull.pre_health_damage_buffer_raw = 0;
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_intro2_gun_turret(&entities);
    let mut fx = WorldFx::new();
    for _ in 0..8 {
        fx.advance_frame_pacing(20_000);
    }
    let mut notifications = GameplayNotifications::new();
    let mut static_damage = StaticDamageScheduler::new();
    let result = crate::attached_particle_damage::apply_attached_particle_damage(
        crate::attached_particle_damage::AttachedParticleDamageFrame {
            entities: &mut entities,
            resources: &mut session.cache,
            world_fx: &mut fx,
            static_damage: &mut static_damage,
            scheduler: &mut scheduler,
            notifications: &mut notifications,
            retail_tick: 81,
            world: crate::attached_particle_damage::AttachedParticleDamageWorld::Playing {
                player_hull: &mut hull,
                extra_lives: RetailRuntimeValue::Known(2),
            },
        },
        crate::world_fx::AttachedParticleDamageRequest {
            target_handle: turret,
            packet_va: 0x004c_c048,
            source_entity_type_raw: (-5_i32) as u32,
            current_emitter: None,
            ratio_numerator: 255,
            ratio_denominator: 255,
        },
    );
    assert!(matches!(result,Ok(damage) if damage>0), "{result:?}");
    assert!(hull.dying);
    assert_eq!(entities.player().unwrap().model_index, Some(67));
    assert!(entities.player_dying_contact_runtime().is_some());
    assert!(fx
        .test_particles_in_virgin_birth_order()
        .iter()
        .any(|particle| matches!(particle.source_class, 16 | 18)));
    assert!(entities
        .pending_actor_deferred_destroy_ids()
        .contains(&turret));
    // This native actor fixture does not load the System5 text pool. Assert
    // the actual notification slot, independently of presentation resources.
    let mut expected_notifications = GameplayNotifications::new();
    expected_notifications.queue_player_destroyed(2, 81);
    assert_eq!(notifications, expected_notifications);
}
