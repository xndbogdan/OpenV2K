use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    entity_collision_state::{RetailRuntimeValue, RetailStateWord},
    intro2_type47_live::world::native_intro2_fixture,
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskFamily, SpecializedActorTaskProductionFrame,
    },
};

struct ContactFixture {
    session: GameSession,
    manager: EntityManager,
    scheduler: SpecializedActorTaskScheduler,
    fx: WorldFx,
    damage: StaticDamageScheduler,
    notifications: GameplayNotifications,
    meteor_ids: Vec<u32>,
}

#[v2k_test_support::retail_test]
fn type58_static_block_stops_its_pair_suffix_but_not_the_late_walk() {
    let Some(mut fixture) = ContactFixture::new() else {
        return;
    };
    let id = fixture
        .manager
        .iter_all()
        .find(|entity| entity.entity_type == 58)
        .unwrap()
        .id;
    assert_eq!(fixture.scheduler.adopt_intro2_type58(&fixture.manager), 1);
    fixture.scheduler.park_intro2_type58_external_prefix(id);
    let entity = fixture.manager.entity_mut(id).unwrap();
    // Living39 disables the bare surface lane; the static lane must still
    // reject the pending owner before proceeding to this actor's pair phase.
    entity.collision.state_flags_at_0x08 = RetailStateWord::exact(0x0406_8000);
    let reports = fixture.contacts();
    assert!(
        reports.iter().any(|report| matches!(report,
            Intro2ContactReport::Type58 {
                entity_id,
                result: Intro2Type58ContactOutcome::Blocked { committed_prefix: false, .. },
            } if *entity_id == id
        )),
        "{reports:?}"
    );
    assert!(
        !reports.iter().any(|report| matches!(report,
            Intro2ContactReport::Type17Pair { entity_id, .. } if *entity_id == id
        )),
        "the pair suffix cannot run after this actor's static failure"
    );
    assert!(
        reports.iter().any(|report| matches!(report,
            Intro2ContactReport::Type17Pair { entity_id, .. } if *entity_id > id
        )),
        "later allocations still receive their own contact visit"
    );

    fixture
        .manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08 = RetailStateWord::exact(0);
    let reports = fixture.contacts();
    assert!(reports.iter().any(|report| matches!(report,
        Intro2ContactReport::Type58 {
            entity_id,
            result: Intro2Type58ContactOutcome::Ineligible,
        } if *entity_id == id
    )));
    assert!(
        reports.iter().any(|report| matches!(report,
            Intro2ContactReport::Type17Pair { entity_id, .. } if *entity_id == id
        )),
        "an ineligible static scan still permits the independent pair phase"
    );
}

#[v2k_test_support::retail_test]
fn type58_surface_block_stops_static_and_pair_suffixes() {
    let Some(mut fixture) = ContactFixture::new() else {
        return;
    };
    let id = fixture
        .manager
        .iter_all()
        .find(|entity| entity.entity_type == 58)
        .unwrap()
        .id;
    fixture
        .manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08 = RetailStateWord::unknown();
    let reports = fixture.contacts();
    assert!(reports.iter().any(|report| matches!(report,
        Intro2ContactReport::NativeSurface {
            entity_id,
            entity_type: 58,
            result: crate::native_actor_surface_contact::NativeActorSurfaceContactOutcome::Blocked {
                committed_prefix: false, ..
            },
        } if *entity_id == id
    )), "{reports:?}");
    assert!(
        !reports.iter().any(|report| matches!(report,
            Intro2ContactReport::Type58 { entity_id, .. }
                | Intro2ContactReport::Type17Pair { entity_id, .. } if *entity_id == id
        )),
        "neither later phase may run after a failed surface entry"
    );
}

impl ContactFixture {
    fn new() -> Option<Self> {
        let (session, mut manager, _) = native_intro2_fixture()?;
        let meteor_ids = manager
            .iter_all()
            .filter_map(|entity| Intro2MeteorOwner::adopt_published(entity).ok())
            .map(|owner| owner.entity_id())
            .collect::<Vec<_>>();
        assert_eq!(meteor_ids.len(), 4);
        // Isolate contact custody from unrelated actor hits. Every allocation,
        // task pair, transform and model still comes from the native scene.
        let ids = manager
            .iter_all()
            .map(|entity| entity.id)
            .collect::<Vec<_>>();
        for id in ids {
            let entity = manager.entity_mut(id).unwrap();
            entity.collision.state_flags_at_0x08 = RetailStateWord::exact(0);
            entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        }
        let mut scheduler = SpecializedActorTaskScheduler::default();
        assert_eq!(scheduler.adopt_intro2_meteors(&manager), 4);
        Some(Self {
            session,
            manager,
            scheduler,
            fx: WorldFx::new(),
            damage: StaticDamageScheduler::default(),
            notifications: GameplayNotifications::new(),
            meteor_ids,
        })
    }

    fn contacts(&mut self) -> Vec<Intro2ContactReport> {
        resolve_intro2_contacts(Intro2ContactFrame {
            entities: &mut self.manager,
            resources: &mut self.session.cache,
            world_fx: &mut self.fx,
            static_damage: &mut self.damage,
            notifications: &mut self.notifications,
            retail_tick: 145,
            actor_tasks: &mut self.scheduler,
        })
    }

    fn collide_first_meteor(&mut self) -> u32 {
        let id = self.meteor_ids[0];
        let entity = self.manager.entity_mut(id).unwrap();
        entity.collision.state_flags_at_0x08 = RetailStateWord::exact(0x0747_8825);
        let [x, _, z] = entity.position_raw();
        let terrain_y = self
            .session
            .cache
            .level_terrain()
            .unwrap()
            .bilinear_height_raw(x, z);
        // Actual model560 collision geometry supplies the separating plane.
        entity.set_motion_raw([x, terrain_y.wrapping_sub(300), z], [0, -10000, 0]);
        id
    }
}

#[v2k_test_support::retail_test]
fn late_finalized_meteor_contact_retires_exact_scheduler_owner_before_next_sweep() {
    let Some(mut fixture) = ContactFixture::new() else {
        return;
    };
    let initial = fixture.contacts();
    assert_eq!(fixture.scheduler.registered_len(), 4);
    assert_eq!(
        initial
            .iter()
            .filter(|report| matches!(
                report,
                Intro2ContactReport::Meteor {
                    result: Ok(None),
                    ..
                }
            ))
            .count(),
        4
    );
    let id = fixture.collide_first_meteor();
    let reports = fixture.contacts();
    assert!(
        reports.iter().any(|report| matches!(report,
            Intro2ContactReport::Meteor {
                entity_id,
                result: Ok(Some(Intro2MeteorDeathReport::Applied { finalized: true, .. })),
            } if *entity_id == id
        )),
        "{reports:?}"
    );
    assert_eq!(fixture.scheduler.family_for(id), None);
    assert_eq!(fixture.scheduler.registered_len(), 3);
    for &other in &fixture.meteor_ids[1..] {
        assert_eq!(
            fixture.scheduler.family_for(other),
            Some(SpecializedActorTaskFamily::Intro2Meteor)
        );
    }
    let entity = fixture.manager.entity_mut(id).unwrap();
    assert!(entity.active, "14990 owns the subsequent allocation sweep");
    for slot in [
        ActorTaskSlot::Primary,
        ActorTaskSlot::Secondary,
        ActorTaskSlot::Tertiary,
    ] {
        assert_eq!(entity.actor_tasks.task_in_slot(slot), None);
    }
    assert_eq!(fixture.manager.pending_actor_deferred_destroy_ids(), [id]);

    let mut rng_oracle = fixture.fx.fork_for_main_base_abort_transaction();
    let repeated = fixture.contacts();
    assert!(!repeated.iter().any(|report| matches!(report,
        Intro2ContactReport::Meteor { entity_id, .. } if *entity_id == id
    )));
    assert_eq!(
        fixture.fx.next_shared_retail_random_u16(),
        rng_oracle.next_shared_retail_random_u16()
    );
    let pass = fixture.scheduler.tick(
        &mut fixture.manager,
        SpecializedActorTaskProductionFrame {
            world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
            hive_components: None,
            notification_phase:
                crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
            resources: &mut fixture.session.cache,
            world_fx: &mut fixture.fx,
            static_damage: &mut fixture.damage,
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
            retail_tick: 146,
            main_base_abort_active: false,
        },
        &mut fixture.notifications,
    );
    assert_eq!(pass.block, None);
    assert!(
        pass.outcomes
            .iter()
            .all(|outcome| outcome.entity_id() != id),
        "{pass:?}"
    );
    assert_eq!(
        fixture.manager.cleanup_pending_actor_deferred_destroys(),
        [id]
    );
}

#[v2k_test_support::retail_test]
fn late_meteor_contact_keeps_scheduler_custody_when_radial_suffix_is_blocked() {
    let Some(mut fixture) = ContactFixture::new() else {
        return;
    };
    let id = fixture.collide_first_meteor();
    let [x, _, z] = fixture.manager.entity_mut(id).unwrap().position_raw();
    let y = fixture
        .session
        .cache
        .level_terrain()
        .unwrap()
        .bilinear_height_raw(x, z);
    let target_id = fixture
        .manager
        .iter_all()
        .find(|entity| entity.entity_type == 94)
        .unwrap()
        .id;
    let target = fixture.manager.entity_mut(target_id).unwrap();
    target.set_position_raw([x, y, z]);
    target.collision.state_flags_at_0x08 = RetailStateWord::unknown();
    let reports = fixture.contacts();
    assert!(reports.iter().any(|report| matches!(report,
        Intro2ContactReport::Meteor {
            entity_id,
            result: Ok(Some(Intro2MeteorDeathReport::Applied { finalized: false, dynamic, .. })),
        } if *entity_id == id && dynamic.blocked.as_ref().is_some_and(|block| block.target_id == target_id)
    )), "{reports:?}");
    assert_eq!(fixture.scheduler.registered_len(), 4);
    assert_eq!(
        fixture.scheduler.family_for(id),
        Some(SpecializedActorTaskFamily::Intro2Meteor)
    );
    assert!(fixture
        .manager
        .pending_actor_deferred_destroy_ids()
        .is_empty());
    assert!(fixture
        .manager
        .entity_mut(id)
        .unwrap()
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .is_some());
    let mut rng_oracle = fixture.fx.fork_for_main_base_abort_transaction();
    let _ = fixture.contacts();
    assert_eq!(
        fixture.fx.next_shared_retail_random_u16(),
        rng_oracle.next_shared_retail_random_u16()
    );
    assert_eq!(
        fixture.scheduler.registered_len(),
        4,
        "an unfinished suffix is not a retirement"
    );
}
