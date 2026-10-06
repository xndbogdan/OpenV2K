use super::*;
use crate::{
    damage::{DamagePacket, FUN_0043F780_DAMAGE_PACKET},
    entity::{
        AuthoredPlayerArrival, AuthoredWorldConstruction, EntityConstructionResources,
        EntityManager,
    },
    entity_collision_state::EntityTypeRuntimeMetadata,
    gameplay_notifications::GameplayNotifications,
    native_entity_weapons::{EntityWeaponConstructionRequest, EntityWeaponKind},
    player_hull::PlayerHull,
    session::GameSession,
    shared_actor_impact::{apply_playing_actor_particle_hit, SharedActorImpactOutcome},
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
    world_fx::{BallisticDamageRequest, WorldFx},
};

struct World {
    session: GameSession,
    manager: EntityManager,
    fx: WorldFx,
    tasks: SpecializedActorTaskScheduler,
    statics: StaticDamageScheduler,
    hull: PlayerHull,
    notifications: GameplayNotifications,
    id: u32,
}
impl World {
    fn new(kind: EntityWeaponKind) -> Self {
        let data = v2k_test_support::retail_dir();
        assert!(
            data.join("PRELOAD.DAT").is_file(),
            "normal-tier corpus required"
        );
        let mut session = GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(17, 1).unwrap();
        let metadata: Vec<_> = session
            .cache
            .global_entity_model_table()
            .iter()
            .enumerate()
            .map(|(id, _)| {
                EntityTypeRuntimeMetadata::from_section12(
                    session.cache.global_entity_type(id).unwrap(),
                )
            })
            .collect();
        let mut fx = WorldFx::new();
        let mut manager = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                level: session.cache.level_desc().unwrap(),
                logical_world_index: 5,
                type_metadata: &metadata,
                resources: EntityConstructionResources {
                    terrain: session.cache.level_terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| {
                        session.cache.global_model(id).map(|model| model.radius)
                    }),
                },
                player_arrival: Some(AuthoredPlayerArrival {
                    position_raw: [1000, 1024, 2000],
                    heading_raw: 0,
                }),
                retail_tick: 0,
            },
            &mut fx,
        )
        .unwrap();
        manager.cleanup_pending_actor_deferred_destroys();
        let owner = manager
            .construct_entity_weapon(
                EntityWeaponConstructionRequest {
                    kind,
                    source_actor_id: manager.player().unwrap().id,
                    position_raw: [0, 10000, 0],
                    velocity_raw: [0; 3],
                    rotation_raw: [0; 3],
                },
                &session.cache,
                &mut fx,
                0,
            )
            .unwrap();
        let id = owner.entity_id();
        for current in manager.retail_live_order_ids().collect::<Vec<_>>() {
            manager
                .entity_mut(current)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(u32::MAX, if current == id { 0x0406_8005 } else { 0 });
        }
        let mut tasks = SpecializedActorTaskScheduler::default();
        tasks.register_native_weapon(&manager, owner).unwrap();
        Self {
            session,
            manager,
            fx,
            tasks,
            statics: StaticDamageScheduler::new(),
            hull: PlayerHull::default(),
            notifications: GameplayNotifications::new(),
            id,
        }
    }
    fn hit(&mut self, class: u8, channel: u8, amount: i32) -> NativeWeaponImpactOutcome {
        let source_owner_id = self.manager.player().map(|entity| entity.id);
        let outcome = apply_playing_actor_particle_hit(
            PlayingActorImpactFrame {
                resources: &mut self.session.cache,
                entities: &mut self.manager,
                world_fx: &mut self.fx,
                scheduler: &mut self.tasks,
                notifications: &mut self.notifications,
                static_damage: &mut self.statics,
                player_hull: &mut self.hull,
                extra_lives: RetailRuntimeValue::Known(1),
                retail_tick: 501,
            },
            ParticleEntityImpact {
                source_particle_class: class,
                target_entity_id: self.id,
                impact_position_argument_va: if class == 5 { 0x004d_cf48 } else { 0 },
                position_world: [0.0; 3],
                velocity_raw: [0, 0, 8192],
                damage: Some(BallisticDamageRequest {
                    packet: if class == 5 {
                        FUN_0043F780_DAMAGE_PACKET
                    } else {
                        DamagePacket {
                            channels: [i32::from(channel), 0],
                            amounts_raw: [amount, 0],
                        }
                    },
                    source_entity_type_at_birth: Some(46),
                    source_owner_id,
                }),
            },
        );
        let Some(SharedActorImpactOutcome::NativeWeapon(outcome)) = outcome else {
            panic!("{outcome:?}")
        };
        outcome
    }
}

#[v2k_test_support::retail_test]
fn primary_and_static_route_keep_distinct_impulse_with_filtered_packets() {
    for kind in [EntityWeaponKind::Rocket, EntityWeaponKind::Grenade] {
        let mut primary = World::new(kind);
        let mut static_hit = World::new(kind);
        // Impact channel2 below its threshold exercises11030 without killing.
        let direct = primary.hit(16, 2, 100);
        let amplified = static_hit.hit(52, 2, 100);
        assert!(
            matches!(direct, NativeWeaponImpactOutcome::Applied(ref outcome) if outcome.filtered_damage_raw == 0),
            "{direct:?}"
        );
        assert!(
            matches!(amplified, NativeWeaponImpactOutcome::Applied(ref outcome) if outcome.filtered_damage_raw == 0),
            "{amplified:?}"
        );
        let a = primary
            .manager
            .entity_mut(primary.id)
            .unwrap()
            .velocity_raw()[2];
        let b = static_hit
            .manager
            .entity_mut(static_hit.id)
            .unwrap()
            .velocity_raw()[2];
        assert!(a > 0 && b > a, "native11180 has8x impulse: {a}/{b}");
        assert_eq!(
            primary.fx.next_shared_retail_random_u16(),
            static_hit.fx.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn lethal_native_particle_finishes_once_and_allows_completed_terminal_rehit() {
    for kind in [EntityWeaponKind::Rocket, EntityWeaponKind::Grenade] {
        let mut world = World::new(kind);
        let first = world.hit(16, 2, 5000);
        assert!(
            matches!(first, NativeWeaponImpactOutcome::Applied(_)),
            "{first:?}"
        );
        assert!(crate::class49_death::finished_terminal_hit_authenticates(
            &world.manager,
            world.id
        ));
        assert!(world
            .tasks
            .native_weapon_owner(&world.manager, world.id)
            .is_none());
        let pending = world.manager.pending_actor_deferred_destroy_ids().to_vec();
        let rings = world
            .manager
            .iter_all()
            .filter(|entity| entity.entity_type == 60)
            .count();
        let second = world.hit(16, 2, 5000);
        assert!(
            matches!(second, NativeWeaponImpactOutcome::Applied(_)),
            "{second:?}"
        );
        assert_eq!(world.manager.pending_actor_deferred_destroy_ids(), pending);
        assert_eq!(
            world
                .manager
                .iter_all()
                .filter(|entity| entity.entity_type == 60)
                .count(),
            rings
        );
        assert_eq!(rings, usize::from(kind == EntityWeaponKind::Grenade));
    }
}

#[v2k_test_support::retail_test]
fn infection_keeps_null_style_hooks_and_live_task_custody() {
    let mut world = World::new(EntityWeaponKind::Rocket);
    let owner = world
        .tasks
        .native_weapon_owner(&world.manager, world.id)
        .unwrap();
    let outcome = world.hit(5, 6, 2000);
    assert!(
        matches!(outcome, NativeWeaponImpactOutcome::Applied(_)),
        "{outcome:?}"
    );
    assert_eq!(
        world.tasks.native_weapon_owner(&world.manager, world.id),
        Some(owner)
    );
    assert_eq!(
        world
            .manager
            .entity_mut(world.id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .masked(0x2000),
        RetailRuntimeValue::Known(0x2000)
    );
}
