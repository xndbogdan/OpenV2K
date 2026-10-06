use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    damage::DamagePacket,
    entity::{AuthoredWorldConstruction, EntityConstructionResources},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    hive_birth::HIVE_EJECTION_DELAY_US,
    hive_controller::{objective_hostile_present, HIVE_LOCKED_HEALTH_RAW},
    intro2_contacts::Intro2ContactFrame,
    intro2_flyer_contacts::{resolve_native_flyer_contacts, Intro2FlyerContactOutcome},
    intro2_flyer_impact::{apply_native_flyer_particle_hit, NativeFlyerImpactOutcome},
    intro2_flyers_live::Intro2FlyerSchedulerProductionOutcome,
    main_base_abort_world_effects::{
        snapshot_main_base_abort_terrain_geometry, MainBaseAbortWorldEffects,
    },
    native_actor_surface_contact::{
        resolve_native_actor_surface_contact, NativeActorSurfaceContactOutcome,
    },
    native_ground_actor::contact::NativeGroundContactOutcome,
    player_hull::PlayerHull,
    session::GameSession,
    static_damage::StaticDamageScheduler,
    sub_n_runtime::MAIN_BASE_ABORT_SUB_N_TIMER_US,
    world_fx::{BallisticDamageRequest, ParticleEntityImpact},
};

#[derive(Clone, Copy)]
enum FixturePhase {
    Cinematic,
    Playing,
}

struct Fixture {
    session: GameSession,
    manager: EntityManager,
    fx: WorldFx,
    scheduler: SpecializedActorTaskScheduler,
    notifications: GameplayNotifications,
    tally: WorldCompleteTally,
    static_damage: StaticDamageScheduler,
    player_hull: PlayerHull,
    hive: u32,
    aborted: bool,
    retail_tick: u32,
}

impl Fixture {
    fn new() -> Self {
        let data = v2k_test_support::retail_dir();
        assert!(
            data.join("PRELOAD.DAT").is_file(),
            "normal-tier corpus required"
        );
        let mut session = GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(13, 1).unwrap();
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
                logical_world_index: 1,
                type_metadata: &metadata,
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| {
                        session.cache.global_model(id).map(|model| model.radius)
                    }),
                },
                player_arrival: None,
                retail_tick: 0,
            },
            &mut fx,
        )
        .unwrap();
        let hive = manager
            .iter_all()
            .find(|entity| entity.entity_type == 67)
            .unwrap()
            .id;
        // Execute the actual1CF90 alternate-cleanup prefix against the live
        // Section10 allocation. Other actors are then made inactive to isolate
        // the reached component/task lane. This does not claim the full56960
        // sweep or an RNG receipt from a retail failed-world recording.
        let geometry = snapshot_main_base_abort_terrain_geometry(&session.cache).unwrap();
        let mut static_damage = StaticDamageScheduler::new();
        let mut effects =
            MainBaseAbortWorldEffects::new(&geometry, &mut session.cache, &mut static_damage)
                .unwrap();
        let lease = manager
            .main_base_abort_actor_observation(hive)
            .unwrap()
            .lease;
        manager
            .apply_main_base_abort_alternate_cleanup(lease, &mut effects)
            .unwrap();
        let others: Vec<_> = manager
            .iter_all()
            .filter(|entity| entity.id != hive && entity.entity_type != 46)
            .map(|entity| entity.id)
            .collect();
        for id in others {
            manager.entity_mut(id).unwrap().active = false;
        }
        let RetailRuntimeValue::Known(Some(sub_n)) =
            &manager.entity_mut(hive).unwrap().sub_n_runtime
        else {
            panic!()
        };
        assert_eq!(sub_n.cleanup_timer_us(), MAIN_BASE_ABORT_SUB_N_TIMER_US);
        Self {
            session,
            manager,
            fx,
            scheduler: SpecializedActorTaskScheduler::new(),
            notifications: GameplayNotifications::new(),
            tally: WorldCompleteTally::default(),
            static_damage,
            player_hull: PlayerHull::default(),
            hive,
            aborted: true,
            retail_tick: 0,
        }
    }

    fn children(&self) -> Vec<u32> {
        self.manager
            .iter_all()
            .filter(|entity| entity.entity_type == 15)
            .map(|entity| entity.id)
            .collect()
    }

    fn row(&self) -> &crate::hive_birth::HiveBirthRowRuntime {
        &self
            .manager
            .iter_all()
            .find(|entity| entity.id == self.hive)
            .unwrap()
            .authored_radial_emitter
            .as_ref()
            .unwrap()
            .birth_runtime()
            .unwrap()
            .rows()[0]
    }

    fn ejection_timer_us(&self, id: u32) -> Option<i32> {
        self.manager
            .iter_all()
            .find(|entity| entity.id == self.hive)
            .unwrap()
            .authored_radial_emitter
            .as_ref()
            .unwrap()
            .birth_runtime()
            .unwrap()
            .ejection_slots()
            .iter()
            .find(|slot| slot.child_handle == Some(id))
            .map(|slot| slot.timer_us)
    }

    fn tick(&mut self, elapsed_micros: u32) -> SpecializedActorTaskProductionPass {
        self.tick_in_phase(elapsed_micros, FixturePhase::Cinematic)
    }

    fn tick_playing(&mut self, elapsed_micros: u32) -> SpecializedActorTaskProductionPass {
        self.tick_in_phase(elapsed_micros, FixturePhase::Playing)
    }

    fn tick_in_phase(
        &mut self,
        elapsed_micros: u32,
        phase: FixturePhase,
    ) -> SpecializedActorTaskProductionPass {
        self.retail_tick = self.retail_tick.wrapping_add(elapsed_micros / 1_000);
        let position = self
            .manager
            .iter_all()
            .find(|entity| entity.id == self.hive)
            .unwrap()
            .position_raw();
        self.scheduler.tick(
            &mut self.manager,
            SpecializedActorTaskProductionFrame {
                world: match phase {
                    FixturePhase::Cinematic => SpecializedActorTaskWorld::Cinematic,
                    FixturePhase::Playing => SpecializedActorTaskWorld::Playing {
                        player_hull: &mut self.player_hull,
                        extra_lives: RetailRuntimeValue::Known(0),
                    },
                },
                hive_components: Some(HiveComponentProductionContext {
                    world_complete_tally: &mut self.tally,
                    view_detail: RetailViewDetailContext::from_raw(
                        position.map(i32::from),
                        0,
                        (52, 30),
                    ),
                }),
                resources: &mut self.session.cache,
                world_fx: &mut self.fx,
                static_damage: &mut self.static_damage,
                elapsed_micros,
                global_elapsed_micros: elapsed_micros,
                retail_tick: self.retail_tick,
                notification_phase: GameplayNotificationPhase::Playing,
                main_base_abort_active: self.aborted,
            },
            &mut self.notifications,
        )
    }

    fn contact_frame(&mut self) -> Intro2ContactFrame<'_> {
        Intro2ContactFrame {
            entities: &mut self.manager,
            resources: &mut self.session.cache,
            actor_tasks: &mut self.scheduler,
            world_fx: &mut self.fx,
            static_damage: &mut self.static_damage,
            notifications: &mut self.notifications,
            retail_tick: self.retail_tick,
        }
    }

    fn terrain_penetration(&self, id: u32) -> Option<f64> {
        let entity = self
            .manager
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap();
        let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
            panic!("real native task must retain its physical basis")
        };
        self.session
            .cache
            .global_model(276)
            .unwrap()
            .collide_terrain_raw_oriented(
                self.session.cache.level_terrain().unwrap(),
                entity.position_raw(),
                basis
                    .orientation_world_from_model()
                    .map(|row| row.map(f64::from)),
                &entity.presentation_anim_vars(self.retail_tick),
            )
            .unwrap()
            .map(|hit| hit.penetration_raw)
    }

    fn dry_flat_level1_ground(&self) -> [i16; 3] {
        let terrain = self.session.cache.level_terrain().unwrap();
        for x in 2..254 {
            for z in 2..254 {
                let height = terrain.cell(x, z).unwrap().height;
                let floor_raw = i16::from(height as i8) * 32;
                // 129B0 excludes water when the signed cell floor is at or
                // above sea level; it does not require an extra512raw margin.
                if floor_raw < terrain.sea_level_raw() {
                    continue;
                }
                if (x - 1..=x + 1).all(|nx| {
                    (z - 1..=z + 1).all(|nz| terrain.cell(nx, nz).unwrap().height == height)
                }) {
                    return [
                        ((x * 256) + 128) as i16,
                        floor_raw,
                        ((z * 256) + 128) as i16,
                    ];
                }
            }
        }
        panic!("normal-tier Level1 needs an unmodified dry flat ground patch")
    }
}

#[v2k_test_support::retail_test]
fn failed_level1_row_births_an_objective_wasp_and_ticks_its_new_tail_same_pass() {
    let mut f = Fixture::new();
    f.aborted = false;
    let seed = f.fx.next_sub_d_allocation_seed();
    let ordinal = f.manager.next_common_body_ordinal();
    let normal = f.tick(20_000);
    assert!(normal.block.is_none());
    assert!(f.children().is_empty());
    assert_eq!(f.row().timer_us(), 0, "inactive failed-only row freezes");
    assert_eq!(f.fx.next_sub_d_allocation_seed(), seed);
    assert_eq!(f.manager.next_common_body_ordinal(), ordinal);
    f.aborted = true;
    let pass = f.tick(20_000);
    assert!(pass.block.is_none());
    assert!(
        f.scheduler.hive_diagnostics.is_empty(),
        "{:?}",
        f.scheduler.hive_diagnostics
    );
    let children = f.children();
    assert_eq!(children.len(), 1);
    let child = f.manager.entity_mut(children[0]).unwrap();
    assert_eq!(child.authored_spawn_index, None);
    assert_eq!(child.attached_to, None);
    assert_eq!(
        child.collision.recent_relation_id_at_0x60,
        RetailRuntimeValue::Known(Some(f.hive))
    );
    assert_eq!(
        child.collision.state_flags_at_0x08.masked(0x0100_8000),
        RetailRuntimeValue::Known(0x0100_0000),
        "ejection clears only pair admission"
    );
    let Some(ActorTaskRuntime::SharedRetarget(task)) =
        child.actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!()
    };
    assert_eq!(
        task.elapsed_ms(),
        20,
        "new tail is eligible during the birth pass"
    );
    assert!(pass.outcomes.iter().any(|outcome| matches!(outcome,
        SpecializedActorTaskProductionOutcome::Intro2FlyerScheduler(Intro2FlyerSchedulerProductionOutcome::B6c0Visit {entity_id,..}) if *entity_id == children[0])));
    assert_eq!(f.row().produced_count(), 1);
    assert_eq!(
        f.row().children().iter().copied().collect::<Vec<_>>(),
        children
    );
    assert_eq!(
        f.manager.entity_mut(f.hive).unwrap().collision.health_raw,
        RetailRuntimeValue::Known(HIVE_LOCKED_HEALTH_RAW)
    );
    assert_eq!(
        objective_hostile_present(f.manager.iter_all()),
        Ok(true),
        "8000 is not15120's hostile gate"
    );
}

#[v2k_test_support::retail_test]
fn failed_level1_finite_three_births_survive_varied_rng_and_callback_durations() {
    for (prior_draws, delta, visits) in [(0, 20_000, 360), (19, 1_000_000, 14)] {
        let mut f = Fixture::new();
        for _ in 0..prior_draws {
            f.fx.next_shared_retail_random_u16();
        }
        let before_seed = f.fx.next_sub_d_allocation_seed();
        let RetailRuntimeValue::Known(before_ordinal) = f.manager.next_common_body_ordinal() else {
            panic!()
        };
        for _ in 0..visits {
            let pass = f.tick(delta);
            assert!(pass.block.is_none());
            assert!(
                f.scheduler.hive_diagnostics.is_empty(),
                "{:?}",
                f.scheduler.hive_diagnostics
            );
        }
        assert_eq!(
            f.children().len(),
            3,
            "delta{delta}, RNG prefix{prior_draws}"
        );
        assert_eq!(f.row().produced_count(), 3);
        assert_eq!(f.row().children().len(), 3);
        assert_eq!(
            f.fx.next_sub_d_allocation_seed(),
            before_seed.wrapping_add(3)
        );
        assert_eq!(
            f.manager.next_common_body_ordinal(),
            RetailRuntimeValue::Known(before_ordinal.wrapping_add(3))
        );
        assert_eq!(
            f.manager
                .entity_mut(f.hive)
                .unwrap()
                .authored_radial_emitter
                .as_ref()
                .unwrap()
                .controller_state(),
            1
        );
        let frozen_timer = f.row().timer_us();
        for _ in 0..3 {
            f.tick(delta);
        }
        assert_eq!(
            f.row().timer_us(),
            frozen_timer,
            "expired total-cap timer remains unchanged"
        );
        assert_eq!(f.row().produced_count(), 3);
    }
}

#[v2k_test_support::retail_test]
fn failed_level1_large_health_unlock_and_already_unlocked_hive_produce_no_children() {
    for previously_unlocked in [false, true] {
        let mut f = Fixture::new();
        if previously_unlocked {
            f.manager
                .entity_mut(f.hive)
                .unwrap()
                .authored_radial_emitter
                .as_mut()
                .unwrap()
                .enter_vulnerable();
        }
        let seed = f.fx.next_sub_d_allocation_seed();
        let ordinal = f.manager.next_common_body_ordinal();
        let pass = f.tick(if previously_unlocked {
            20_000
        } else {
            5_000_000
        });
        assert!(pass.block.is_none());
        assert!(f.children().is_empty());
        assert_eq!(f.row().produced_count(), 0);
        assert_eq!(f.row().timer_us(), 0);
        assert_eq!(f.fx.next_sub_d_allocation_seed(), seed);
        assert_eq!(f.manager.next_common_body_ordinal(), ordinal);
        assert_eq!(
            f.manager
                .entity_mut(f.hive)
                .unwrap()
                .authored_radial_emitter
                .as_ref()
                .unwrap()
                .controller_state(),
            2
        );
    }
}

#[v2k_test_support::retail_test]
fn failed_level1_native_wasp_restores_pairs_then_uses_full_hit_and_quiet_death_owner() {
    let mut f = Fixture::new();
    for _ in 0..25 {
        f.tick(20_000);
    }
    let id = f.children()[0];
    assert_eq!(
        f.manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .masked(0x8000),
        RetailRuntimeValue::Known(0x8000)
    );
    assert!(f.scheduler.intro2_flyer_completed_owner(&f.manager, id));
    let source_owner_id = f.manager.player().map(|entity| entity.id);
    let impact = |amount| ParticleEntityImpact {
        source_particle_class: 16,
        impact_position_argument_va: 0,
        target_entity_id: id,
        position_world: [0.; 3],
        velocity_raw: [0, 0, 8192],
        damage: Some(BallisticDamageRequest {
            packet: DamagePacket {
                channels: [3, 0],
                amounts_raw: [amount, 0],
            },
            source_entity_type_at_birth: Some(46),
            source_owner_id,
        }),
    };
    let hit = impact(100);
    let lethal = impact(100_000);
    let result = apply_native_flyer_particle_hit(
        crate::shared_actor_impact::SharedActorImpactFrame {
            entities: &mut f.manager,
            resources: &f.session.cache,
            world_fx: &mut f.fx,
            scheduler: &mut f.scheduler,
            notifications: &mut f.notifications,
            retail_tick: f.retail_tick,
        },
        hit,
    );
    assert!(
        matches!(result, NativeFlyerImpactOutcome::Applied(ref result) if result.filtered_damage_raw > 0 && result.death_publication.is_none()),
        "{result:?}"
    );
    assert!(f.scheduler.intro2_flyer_completed_owner(&f.manager, id));
    let result = apply_native_flyer_particle_hit(
        crate::shared_actor_impact::SharedActorImpactFrame {
            entities: &mut f.manager,
            resources: &f.session.cache,
            world_fx: &mut f.fx,
            scheduler: &mut f.scheduler,
            notifications: &mut f.notifications,
            retail_tick: f.retail_tick + 1,
        },
        lethal,
    );
    assert!(
        matches!(result, NativeFlyerImpactOutcome::Applied(ref result) if result.death_publication.is_some()),
        "{result:?}"
    );
    assert!(f.manager.pending_actor_deferred_destroy_ids().contains(&id));
    assert_ne!(
        f.notifications.save_tail_seen_mask()
            & (1 << crate::gameplay_notifications::PLAYER_KILL_HINT_EVENT_ID),
        0
    );
    assert!(!f.scheduler.intro2_flyer_completed_owner(&f.manager, id));
    assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER.into_iter().all(|slot| f
        .manager
        .entity_mut(id)
        .unwrap()
        .actor_task_state(slot)
        .is_none()));
    assert_eq!(
        f.row().produced_count(),
        1,
        "death does not refund the total cap"
    );
}

#[v2k_test_support::retail_test]
fn failed_level1_hive_wasp_playing_contact_restores_gate_and_corrects_solid_burial() {
    // The real OVL13 constructor,1CF90 cleanup and Hive birth/ejection/task
    // lane supply custody. Only the contact stress pose is controlled; this
    // isolated lane is not a full56960 sweep or a retail trajectory oracle.
    for prior_draws in [0, 19] {
        for delta in [16_667, 40_000, 125_000] {
            let mut f = Fixture::new();
            for _ in 0..prior_draws {
                f.fx.next_shared_retail_random_u16();
            }
            let birth = f.tick_playing(delta);
            assert!(birth.block.is_none(), "{:?}", birth.block);
            assert!(f.scheduler.hive_diagnostics.is_empty());
            let id = f.children()[0];
            let child = f.manager.entity_mut(id).unwrap();
            assert_eq!(child.entity_type, 15);
            assert_eq!(child.model_index, Some(276));
            assert_eq!(child.authored_spawn_index, None);
            assert!(matches!(
                child.intro2_flyer_frame_owner.unwrap().birth_provenance,
                crate::intro2_flyers_live::FlyerBirthProvenance::NativeType15(_)
            ));
            assert_eq!(
                child.collision.state_flags_at_0x08.masked(0x8000),
                RetailRuntimeValue::Known(0),
                "the actual Hive ejection suppresses admission"
            );
            let initial_primary = child
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .unwrap();
            let Some(ActorTaskRuntime::SharedRetarget(task)) =
                child.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("the native child must publish its real acquiring task")
            };
            assert_eq!(task.elapsed_ms(), delta / 1_000);
            assert_eq!(task.lifetime_ms(), 500);
            let initial_pose = (child.position_raw(), child.velocity_raw());
            let suppressed = resolve_native_flyer_contacts(&mut f.contact_frame(), id);
            assert_eq!(suppressed.surface, Intro2FlyerContactOutcome::Ineligible);
            assert_eq!(
                suppressed.static_contact,
                Some(NativeGroundContactOutcome::Ineligible)
            );
            assert!(!suppressed.blocks_later_contacts());
            let child = f.manager.entity_mut(id).unwrap();
            assert_eq!((child.position_raw(), child.velocity_raw()), initial_pose);

            //1CA90 ages its ejection slot in microseconds. The task's generic
            // wrapper independently truncates each callback delta to ms.
            let mut ejection_elapsed_us = delta;
            let mut wander_elapsed_ms = delta / 1_000;
            while f
                .manager
                .entity_mut(id)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .masked(0x8000)
                == RetailRuntimeValue::Known(0)
            {
                assert!(ejection_elapsed_us < HIVE_EJECTION_DELAY_US as u32);
                assert_eq!(
                    f.ejection_timer_us(id),
                    Some(HIVE_EJECTION_DELAY_US - ejection_elapsed_us as i32),
                    "the actual1CA90 slot retains the remaining suppression duration"
                );
                let child = f.manager.entity_mut(id).unwrap();
                if child.actor_tasks.task_in_slot(ActorTaskSlot::Primary) == Some(initial_primary) {
                    let Some(ActorTaskRuntime::SharedRetarget(task)) =
                        child.actor_task_state(ActorTaskSlot::Primary)
                    else {
                        panic!("unchanged acquiring wrapper must retain its native task")
                    };
                    assert_eq!(task.elapsed_ms(), wander_elapsed_ms);
                }
                let pass = f.tick_playing(delta);
                assert!(pass.block.is_none(), "{:?}", pass.block);
                assert!(f.scheduler.hive_diagnostics.is_empty());
                ejection_elapsed_us += delta;
                wander_elapsed_ms += delta / 1_000;
            }
            assert!(ejection_elapsed_us >= HIVE_EJECTION_DELAY_US as u32);
            assert!(ejection_elapsed_us - delta < HIVE_EJECTION_DELAY_US as u32);
            assert_eq!(
                f.ejection_timer_us(id),
                None,
                "expired1CA90 clears its slot"
            );
            assert_eq!(
                f.manager
                    .entity_mut(id)
                    .unwrap()
                    .collision
                    .state_flags_at_0x08
                    .masked(0x8000),
                RetailRuntimeValue::Known(0x8000)
            );
            assert!(f.scheduler.intro2_flyer_completed_owner(&f.manager, id));

            let [x, floor, z] = f.dry_flat_level1_ground();
            let descent_raw = (1000_i64 * i64::from(delta) / 1_000_000) as i16;
            let buried_pose = [x, floor - 32 - descent_raw, z];
            let child = f.manager.entity_mut(id).unwrap();
            child.set_position_raw(buried_pose);
            child.set_velocity_raw([0, -1000, 0]);
            let before_penetration = f.terrain_penetration(id).unwrap();
            assert!(before_penetration > 3.0);
            let old = resolve_native_actor_surface_contact(&mut f.contact_frame(), id);
            assert_eq!(old, NativeActorSurfaceContactOutcome::Ineligible);
            assert_eq!(
                f.manager.entity_mut(id).unwrap().position_raw(),
                buried_pose
            );
            assert_eq!(f.terrain_penetration(id), Some(before_penetration));

            let outcome = resolve_native_flyer_contacts(&mut f.contact_frame(), id);
            assert!(
                matches!(
                    outcome.surface,
                    Intro2FlyerContactOutcome::Applied {
                        solid_contact: true,
                        water_entry: false,
                        ..
                    }
                ),
                "RNG prefix{prior_draws}, dt{delta}: {outcome:?}"
            );
            assert!(
                matches!(
                    outcome.static_contact,
                    Some(NativeGroundContactOutcome::Miss | NativeGroundContactOutcome::Applied(_))
                ),
                "the complete source visit must reach its static suffix: {outcome:?}"
            );
            assert!(!outcome.blocks_later_contacts(), "{outcome:?}");
            assert!(f.manager.entity_mut(id).unwrap().position_raw()[1] > buried_pose[1]);
            let after = f.terrain_penetration(id);
            assert!(
                after.is_none_or(|penetration| penetration <= 3.0),
                "RNG prefix{prior_draws}, dt{delta}: residual{after:?}, {outcome:?}"
            );

            if prior_draws == 19 && delta == 125_000 {
                let child = f.manager.entity_mut(id).unwrap();
                child.set_position_raw([x, floor - 32, z]);
                child.set_velocity_raw([0, -12000, 0]);
                child.collision.health_raw = RetailRuntimeValue::Known(1);
                let entry_slot = child.collision.active_model_slot();
                let RetailRuntimeValue::Known(entry_slot) = entry_slot else {
                    panic!("native live model selector")
                };
                assert_eq!(child.model_in_slot(entry_slot), Some(276));
                let lethal = resolve_native_flyer_contacts(&mut f.contact_frame(), id);
                assert!(
                    matches!(lethal.surface, Intro2FlyerContactOutcome::Applied {
                    solid_contact: true, water_entry: false, collision_damage_raw, ..
                } if collision_damage_raw > 0),
                    "{lethal:?}"
                );
                assert!(matches!(lethal.static_contact,
                    Some(NativeGroundContactOutcome::Miss | NativeGroundContactOutcome::Applied(_))),
                    "quiet death must retain entry model276 for this admitted static suffix: {lethal:?}");
                assert!(!lethal.blocks_later_contacts(), "{lethal:?}");
                assert!(
                    crate::native_flying_surface_contact::native_flyer_quiet_death_authenticates(
                        &f.manager, id
                    )
                );
                assert!(f.manager.pending_actor_deferred_destroy_ids().contains(&id));
                assert!(!f.scheduler.intro2_flyer_completed_owner(&f.manager, id));
                let child = f.manager.entity_mut(id).unwrap();
                assert_eq!(child.collision.health_raw, RetailRuntimeValue::Known(0));
                assert_eq!(
                    child
                        .collision
                        .state_flags_at_0x08
                        .masked(crate::entity_collision_state::DYING_STATE_BIT,),
                    RetailRuntimeValue::Known(crate::entity_collision_state::DYING_STATE_BIT),
                );
                let RetailRuntimeValue::Known(terminal_slot) = child.collision.active_model_slot()
                else {
                    panic!("native quiet terminal model selector")
                };
                assert_ne!(terminal_slot, entry_slot);
                assert_eq!(
                    child.model_in_slot(terminal_slot),
                    Some(276),
                    "retail Type15 aliases model276 across its changed terminal selector"
                );
                assert_eq!(
                    child.collision.constructor_sound_attachment_id_at_0x8c,
                    RetailRuntimeValue::Known(None)
                );
                assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
                    .into_iter()
                    .all(|slot| child.actor_task_state(slot).is_none()));
                assert!(f
                    .terrain_penetration(id)
                    .is_none_or(|penetration| penetration <= 3.0));
            }
        }
    }
}
