//! Real Level-1 fence geometry through the fresh Type-17 Following owner.

use v2k_game::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    damage::DamagePacket,
    entity::{Entity, EntityConstructionResources, EntityManager},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    follow_beacons::FollowBeaconsFollowingTaskState,
    gameplay_notifications::GameplayNotifications,
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler,
    },
    static_damage::{StaticDamageOutcome, StaticDamageScheduler},
    type17_common_dying_production::Type17CommonDyingProductionOutcome,
    type17_follow_beacons_live::Type17FollowBeaconsFollowingOwner,
    type17_follow_beacons_production::{
        tick_type17_follow_beacons_scheduler_owner, Type17FollowBeaconsProductionFrame,
        Type17FollowBeaconsSchedulerOwner,
    },
    type17_static_contact_live::{
        resolve_type17_follow_static_contact, Type17StaticContactApplied, Type17StaticContactFrame,
        Type17StaticContactOutcome,
    },
    world_fx::WorldFx,
};

struct Fixture {
    session: GameSession,
    manager: EntityManager,
    metadata: EntityTypeRuntimeMetadata,
    owner: Type17FollowBeaconsFollowingOwner,
    static_damage: StaticDamageScheduler,
    world_fx: WorldFx,
}

impl Fixture {
    fn load() -> Self {
        let dir = v2k_test_support::retail_dir();
        let mut session = GameSession::init(&dir).expect("PRELOAD");
        session
            .load_auxiliary_ovl(3, 1)
            .expect("normal system tier");
        session.load_level_by_id(13, 1).expect("normal Level 1");
        let metadata: Vec<_> = session
            .cache
            .global_entity_model_table()
            .iter()
            .copied()
            .enumerate()
            .map(|(entity_type, model_slots)| {
                session
                    .cache
                    .global_entity_type(entity_type)
                    .map(EntityTypeRuntimeMetadata::from_section12)
                    .unwrap_or(EntityTypeRuntimeMetadata {
                        model_slots,
                        ..EntityTypeRuntimeMetadata::default()
                    })
            })
            .collect();
        let mut world_fx = WorldFx::new();
        let mut manager = EntityManager::from_fresh_new_game_level_with_type_metadata(
            session.cache.level_desc().expect("Level-1 spawns"),
            &metadata,
            EntityConstructionResources::new(
                session.cache.terrain(),
                session.cache.terrain_objects(),
            ),
            0,
            &mut world_fx,
        )
        .expect("fresh-New-Game birth publication");
        let spider = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(18))
            .expect("authored spawn 18");
        let mut owner = Type17FollowBeaconsSchedulerOwner::adopt_published(spider, &metadata[17])
            .expect("published acquisition");
        let mut static_damage = StaticDamageScheduler::new();
        // First visit publishes the real scored beacon and B70 task; the
        // second runs the authenticated mover and publishes its Q31 basis.
        for retail_tick in 0..2 {
            let tick = tick_type17_follow_beacons_scheduler_owner(
                &mut manager,
                owner,
                Type17FollowBeaconsProductionFrame {
                    resources: &mut session.cache,
                    static_damage: &mut static_damage,
                    world_fx: &mut world_fx,
                    elapsed_micros: 19_500,
                    retail_tick,
                },
            );
            owner = tick
                .retained_owner
                .unwrap_or_else(|| panic!("fixture owner lost: {:?}", tick.outcome));
        }
        let Type17FollowBeaconsSchedulerOwner::Following(owner) = owner else {
            panic!("fresh spawn 18 did not acquire its authored beacon");
        };
        let fixture = Self {
            session,
            manager,
            metadata: metadata[17].clone(),
            owner,
            static_damage,
            world_fx,
        };
        assert!(matches!(
            fixture.entity().physical_body_basis_q31(),
            RetailRuntimeValue::Known(_)
        ));
        assert_eq!(fixture.entity().model_slots, [Some(256); 4]);
        assert_eq!(fixture.entity().mass_raw, 100);
        fixture
    }

    fn entity(&self) -> &Entity {
        self.manager
            .iter_all()
            .find(|entity| entity.id == self.owner.entity_id())
            .expect("retained spider")
    }

    fn task(&self) -> FollowBeaconsFollowingTaskState {
        let Some(ActorTaskRuntime::FollowBeaconsFollowing(task)) =
            self.entity().actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("retained Following task");
        };
        *task
    }

    fn health_raw(&self) -> i32 {
        let RetailRuntimeValue::Known(health) = self.entity().collision.health_raw else {
            panic!("authenticated spider health");
        };
        health
    }

    fn place(&mut self, position_raw: [i16; 3], velocity_raw: [i16; 3]) {
        self.manager
            .entity_mut(self.owner.entity_id())
            .expect("spider motion owner")
            .set_motion_raw(position_raw, velocity_raw);
    }

    fn resolve(&mut self) -> Type17StaticContactOutcome {
        resolve_type17_follow_static_contact(
            &mut self.manager,
            self.owner,
            &self.metadata,
            Type17StaticContactFrame {
                resources: &mut self.session.cache,
                static_damage: &mut self.static_damage,
                world_fx: &mut self.world_fx,
                retail_tick: 2,
            },
        )
        .expect("authenticated Type-17 static suffix")
    }

    fn contact(&mut self) -> Type17StaticContactApplied {
        match self.resolve() {
            Type17StaticContactOutcome::Applied(applied) => applied,
            outcome => panic!("expected authored fence contact, got {outcome:?}"),
        }
    }
}

#[v2k_test_support::retail_test]
fn ordinary_and_mirrored_pen_models_run_exact_two_word_task_hook() {
    // These fixed poses were checked against the canonical raw model-256
    // spheres and the real Section-10 fence cells, using the first mover's
    // published Q31 body basis. No substitute collision shape is involved.
    for (position, cell, model_id) in [
        ([-26753, -85, -26945], [152, 150], 518),
        ([-24961, 3, -26881], [157, 151], 532),
        ([-24705, 91, -27265], [158, 149], 542),
    ] {
        let mut fixture = Fixture::load();
        let mut control = Fixture::load();
        fixture.place(position, [0; 3]);
        let before = fixture.task();
        let health = fixture.health_raw();
        let basis = fixture.entity().physical_body_basis_q31();
        let sub_a_before = fixture.entity().sub_a_propulsion_runtime;
        let x_word = control.world_fx.next_shared_retail_random_u16();
        let z_word = control.world_fx.next_shared_retail_random_u16();
        let applied = fixture.contact();
        assert_eq!(applied.contact.cell, cell);
        assert_eq!(applied.contact.model_id, model_id);
        assert_eq!(applied.contact.kind_index, 9);
        assert!(applied.contact.penetration_raw > 0);
        assert_eq!(applied.impact_raw, 0);
        assert_eq!(applied.static_damage, None);
        assert_eq!(applied.actor_damage, None);
        let after = fixture.task();
        assert_eq!(after.target_id(), before.target_id());
        assert_eq!(after.elapsed_ms(), before.elapsed_ms());
        assert_eq!(
            after.private_state().direction,
            -before.private_state().direction
        );
        assert_eq!(after.private_state().reversal_timer_ms, 2_500);
        assert_eq!(
            after.private_state().target_position_raw,
            [
                position[0].wrapping_add((x_word >> 6) as i16 - 0x200),
                before.private_state().target_position_raw[1],
                position[2].wrapping_add((z_word >> 6) as i16 - 0x200),
            ]
        );
        let (RetailRuntimeValue::Known(Some(before_a)), RetailRuntimeValue::Known(Some(after_a))) =
            (sub_a_before, fixture.entity().sub_a_propulsion_runtime)
        else {
            panic!("authenticated Sub-A runtime");
        };
        assert_eq!(
            after_a.direction_multiplier(),
            after.private_state().direction
        );
        assert_eq!(after_a.target_speed_raw(), before_a.target_speed_raw());
        assert_eq!(
            after_a.drive_scale_percent(),
            before_a.drive_scale_percent()
        );
        assert_eq!(fixture.entity().physical_body_basis_q31(), basis);
        assert_eq!(fixture.health_raw(), health);
        assert_eq!(
            fixture.world_fx.next_shared_retail_random_u16(),
            control.world_fx.next_shared_retail_random_u16(),
            "contact must consume exactly two words"
        );
        assert_eq!(fixture.static_damage.active_program_count(), 0);
        assert!(fixture.world_fx.take_positional_sounds().is_empty());
    }
}

#[v2k_test_support::retail_test]
fn inward_fence_response_stops_at_intact_geometry_and_real_breach_releases_it() {
    let mut fixture = Fixture::load();
    let mut control = Fixture::load();
    let position = [-24961, 3, -26881];
    let velocity = [-64, 0, 0];
    fixture.place(position, velocity);
    let applied = fixture.contact();
    assert_eq!(applied.contact.cell, [157, 151]);
    assert_eq!(applied.contact.model_id, 532);
    assert_eq!(applied.contact.response_raw, -256);
    assert!(applied.position_after_raw[0] > position[0]);
    assert!(applied.velocity_after_raw[0] > velocity[0]);
    assert_eq!(fixture.entity().position_raw(), applied.position_after_raw);
    assert_eq!(fixture.entity().velocity_raw(), applied.velocity_after_raw);
    assert!(applied.impact_raw > 0 && applied.impact_raw < 4_000);
    assert!(matches!(
        applied.static_damage,
        Some(StaticDamageOutcome::NoDamage { .. })
    ));
    assert!(applied.actor_damage.is_some());
    assert_eq!(fixture.health_raw(), control.health_raw());
    control.world_fx.next_shared_retail_random_u16();
    control.world_fx.next_shared_retail_random_u16();

    // Select the authored destroyed model 533 in the existing cell. Keep its
    // attribute and neighboring intact fence cells unchanged.
    let cache = &mut fixture.session.cache;
    let before_cell = *cache.level_terrain().unwrap().cell(157, 151).unwrap();
    assert!(cache.or_level_terrain_type_bits([157, 151], 0x08));
    let after_cell = *cache.level_terrain().unwrap().cell(157, 151).unwrap();
    assert_eq!(after_cell.attribute, before_cell.attribute);
    let descriptor = &cache.terrain_objects().unwrap().records[after_cell.attribute as usize];
    assert_eq!(descriptor.model_id_for(before_cell.terrain_type), 532);
    assert_eq!(descriptor.model_id_for(after_cell.terrain_type), 533);
    assert_eq!(cache.global_model(533).unwrap().collision_radius_raw, 0);
    fixture.place(position, velocity);
    let task = fixture.task();
    assert_eq!(fixture.resolve(), Type17StaticContactOutcome::Miss);
    assert_eq!(fixture.entity().position_raw(), position);
    assert_eq!(fixture.entity().velocity_raw(), velocity);
    assert_eq!(fixture.task(), task);
    assert_eq!(
        fixture.world_fx.next_shared_retail_random_u16(),
        control.world_fx.next_shared_retail_random_u16(),
        "breached geometry must not consume another hook's RNG"
    );
}

#[v2k_test_support::retail_test]
fn hard_fence_contact_filters_same_impact_for_static_cell_and_spider_health() {
    let mut fixture = Fixture::load();
    fixture.place([-24961, 3, -26881], [-2_400, 0, 0]);
    let health_before = fixture.health_raw();
    let applied = fixture.contact();
    let filtered = DamagePacket::collision(applied.impact_raw)
        .filtered_raw(fixture.metadata.damage_profile.as_ref());
    assert!(filtered > 0 && filtered < health_before);
    assert_eq!(fixture.health_raw(), health_before - filtered);
    assert!(applied.actor_damage.is_some());
    assert!(matches!(
        applied.static_damage,
        Some(StaticDamageOutcome::Started { severity_raw, .. }) if severity_raw > 0
    ));
    assert!(fixture.static_damage.contains_cell([157, 151]));
    assert_eq!(fixture.static_damage.active_program_count(), 1);
}

#[v2k_test_support::retail_test]
fn scheduler_contacts_after_motion_and_keeps_new_dying_owner_for_next_pass() {
    let mut fixture = Fixture::load();
    let position = [-24961, 3, -26881];
    fixture.place(position, [-2_400, 0, 0]);
    // A previously wounded spider reaches the same real fence. Preserve its
    // authenticated birth, task, metadata and all callback fields.
    fixture
        .manager
        .entity_mut(fixture.owner.entity_id())
        .unwrap()
        .collision
        .health_raw = RetailRuntimeValue::Known(1);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(
        scheduler.adopt_fresh_level1_type17_follow_beacons(&fixture.manager),
        1
    );
    let mut notifications = GameplayNotifications::new();
    let pass = scheduler.tick(
        &mut fixture.manager,
        SpecializedActorTaskProductionFrame {
            world:
                v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
            hive_components: None,
            notification_phase:
                v2k_game::gameplay_notifications::GameplayNotificationPhase::Playing,
            resources: &mut fixture.session.cache,
            world_fx: &mut fixture.world_fx,
            static_damage: &mut fixture.static_damage,
            elapsed_micros: 1_000,
            global_elapsed_micros: 40_000,
            retail_tick: 2,
            main_base_abort_active: false,
        },
        &mut notifications,
    );
    let [SpecializedActorTaskProductionOutcome::Type17FollowBeacons(outcome)] =
        pass.outcomes.as_slice()
    else {
        panic!(
            "only the existing Follow receipt may run: {:?}",
            pass.outcomes
        );
    };
    let dying = outcome
        .published_common_dying()
        .unwrap_or_else(|| panic!("collision should publish Common Dying: {outcome:?}"));
    assert_eq!(dying.entity_id(), fixture.owner.entity_id());
    let v2k_game::type17_follow_beacons_production::Type17FollowBeaconsSchedulerProductionOutcome::Following {
        visit, ..
    } = outcome else {
        unreachable!();
    };
    let Some(Ok(Type17StaticContactOutcome::Applied(contact))) = &visit.static_contact else {
        panic!("committed mover must enter its static suffix: {visit:?}");
    };
    assert_ne!(
        contact.position_before_raw, position,
        "scan follows master integration"
    );
    assert_eq!(contact.contact.cell, [157, 151]);
    assert_eq!(fixture.entity().position_raw(), contact.position_after_raw);
    assert_eq!(fixture.health_raw(), 0);
    assert_eq!(scheduler.registered_len(), 1);

    let next = scheduler.tick(
        &mut fixture.manager,
        SpecializedActorTaskProductionFrame {
            world:
                v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
            hive_components: None,
            notification_phase:
                v2k_game::gameplay_notifications::GameplayNotificationPhase::Playing,
            resources: &mut fixture.session.cache,
            world_fx: &mut fixture.world_fx,
            static_damage: &mut fixture.static_damage,
            elapsed_micros: 1_000,
            global_elapsed_micros: 41_000,
            retail_tick: 2,
            main_base_abort_active: false,
        },
        &mut notifications,
    );
    assert!(
        matches!(
            next.outcomes.as_slice(),
            [SpecializedActorTaskProductionOutcome::Type17CommonDying(
                Type17CommonDyingProductionOutcome::Advanced { .. }
                    | Type17CommonDyingProductionOutcome::SchedulerWaiting { .. }
            )]
        ),
        "next pass must own the published class-12 task: {:?}",
        next.outcomes
    );
    assert_eq!(scheduler.registered_len(), 1);
}
