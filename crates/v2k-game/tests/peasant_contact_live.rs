//! Live selected peasant custody across the player's descriptor-contact pass.

use v2k_game::active_pair::PairCandidateDisposition;
use v2k_game::actor_task_dispatcher::ActorTaskRuntime;
use v2k_game::actor_task_owner::ActorTaskSlot;
use v2k_game::entity::{EntityConstructionResources, EntityManager};
use v2k_game::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
use v2k_game::entity_view_detail::RetailViewDetailContext;
use v2k_game::gameplay_notifications::GameplayNotifications;
use v2k_game::player::PlayerCraft;
use v2k_game::player_hull::PlayerHull;
use v2k_game::session::GameSession;
use v2k_game::specialized_actor_task_production::{
    SpecializedActorTaskProductionFrame, SpecializedActorTaskScheduler,
};
use v2k_game::static_damage::StaticDamageScheduler;
use v2k_game::world_fx::WorldFx;

struct World {
    session: GameSession,
    entities: EntityManager,
    scheduler: SpecializedActorTaskScheduler,
    effects: WorldFx,
    static_damage: StaticDamageScheduler,
    notifications: GameplayNotifications,
    tick: u32,
}

impl World {
    fn new() -> Self {
        let data = v2k_test_support::retail_dir();
        assert!(
            data.join("PRELOAD.DAT").exists(),
            "canonical retail corpus required"
        );
        let mut session = GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(13, 1).unwrap();
        let metadata: Vec<_> = session
            .cache
            .global_entity_model_table()
            .iter()
            .copied()
            .enumerate()
            .map(|(id, model_slots)| {
                session
                    .cache
                    .global_entity_type(id)
                    .map(EntityTypeRuntimeMetadata::from_section12)
                    .unwrap_or(EntityTypeRuntimeMetadata {
                        model_slots,
                        ..Default::default()
                    })
            })
            .collect();
        let mut effects = WorldFx::new();
        let mut entities = EntityManager::from_fresh_new_game_level_with_type_metadata(
            session.cache.level_desc().unwrap(),
            &metadata,
            EntityConstructionResources::new(
                session.cache.terrain(),
                session.cache.terrain_objects(),
            ),
            0,
            &mut effects,
        )
        .unwrap();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(
            scheduler
                .adopt_fresh_level1_type9_selected(&mut entities)
                .unwrap(),
            6
        );
        assert_eq!(
            scheduler.adopt_fresh_level1_type17_follow_beacons(&entities),
            1
        );
        assert_eq!(scheduler.adopt_level_one_factory_arrival(&entities), 1);
        assert_eq!(
            scheduler
                .adopt_fresh_level1_type47_scheduler(&mut entities)
                .unwrap(),
            3
        );
        Self {
            session,
            entities,
            scheduler,
            effects,
            static_damage: StaticDamageScheduler::new(),
            notifications: GameplayNotifications::new(),
            tick: 0,
        }
    }

    fn step(&mut self) {
        self.scheduler.adopt_live_type8_go_to_job(&self.entities);
        self.scheduler
            .adopt_level_one_factory_arrival(&self.entities);
        let claims: Vec<_> = self.scheduler.actor_animation_claims().collect();
        let pass = self.scheduler.tick(
            &mut self.entities,
            SpecializedActorTaskProductionFrame {
                world: v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    v2k_game::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut self.session.cache,
                world_fx: &mut self.effects,
                static_damage: &mut self.static_damage,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: self.tick,
                main_base_abort_active: false,
            },
            &mut self.notifications,
        );
        assert!(
            pass.block.is_none(),
            "pass at {}: {:?}",
            self.tick,
            pass.block
        );
        for outcome in &pass.outcomes {
            let diagnostic = format!("{outcome:?}");
            assert!(
                !diagnostic.contains("Blocked") && !diagnostic.contains("Dropped"),
                "actor at {}: {diagnostic}",
                self.tick
            );
        }
        self.entities
            .advance_unclaimed_actor_animations(20_000, &claims);
        let observed = self.entities.player().unwrap().position;
        let scan = v2k_render::terrain_tiles::scan_dimensions(
            self.session.cache.level_desc().unwrap().terrain_draw_depth,
        );
        self.scheduler.publish_presented_view_detail(
            &mut self.entities,
            RetailViewDetailContext::from_world(
                [observed[0], observed[1] + 8.0, observed[2] - 8.0],
                0.7,
                scan,
            ),
        );
        self.tick += 1;
    }
}

#[derive(Clone, Copy)]
enum ContactSide {
    Forward,
    Rear,
}

fn assert_lighthouse_peasant_continues_after_contact(side: ContactSide, warmup_ticks: u32) {
    let mut world = World::new();
    // This authored singleton starts as selected Wander. Leave its task,
    // heading, component fields and shared process RNG entirely untouched.
    let id = 17;
    let player_start = world.entities.player().unwrap().position;
    // The longer case crosses natural root replacements and exercises current
    // task authority as well as the initial constructor's completed receipt.
    for _ in 0..warmup_ticks {
        world.step();
    }
    let peasant = world
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap();
    assert_eq!(peasant.entity_type, 9);
    assert!(matches!(
        peasant.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::OrdinaryType9Wander(_))
    ));
    let RetailRuntimeValue::Known(basis) = peasant.physical_body_basis_q31() else {
        panic!("live peasant basis");
    };
    let before = peasant.position_raw();
    let velocity_before = peasant.velocity_raw();
    let heading_before = peasant.heading_raw();
    let sub_a_before = peasant.sub_a_propulsion_runtime;
    let position = peasant.position;
    let forward = basis
        .forward
        .map(|component| component as f32 / 2_147_483_648.0);
    // Both approaches overlap the real collision models. The forward side
    // reaches the positive 402DA0 branch; rear contact keeps its lazy no-op,
    // independent of the peasant's authored compass heading.
    let offset = match side {
        ContactSide::Forward => 0.25,
        ContactSide::Rear => -0.25,
    };
    world.entities.player_mut().unwrap().position =
        std::array::from_fn(|axis| position[axis] + offset * forward[axis]);
    let mut hull = PlayerHull::default();
    let contact = world
        .entities
        .resolve_player_active_contacts(
            &world.session.cache,
            v2k_game::player_active_contact::PlayerActivePairFrame {
                resources: &world.session.cache,
                retail_tick: world.tick,
                player_craft: &PlayerCraft::new(),
                player_hull: &mut hull,
                scheduler: &mut world.scheduler,
                world_fx: &mut WorldFx::new(),
                notifications: &mut world.notifications,
                extra_lives: 0,
            },
        )
        .expect("live player/peasant contact");
    assert!(
        contact.core.dispositions.iter().any(|disposition| matches!(
            disposition,
            PairCandidateDisposition::Resolved { candidate_id, .. } if *candidate_id == id
        )),
        "the player must physically contact the peasant"
    );
    let peasant = world
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap();
    assert!(
        peasant.position_raw() != before || peasant.velocity_raw() != velocity_before,
        "physical response must change the retained body even when the descriptor callback is a no-op"
    );
    match side {
        ContactSide::Forward => assert!(
            contact
                .descriptor_contacts
                .iter()
                .any(|outcome| outcome.entity_id == id
                    && outcome.heading_raw_before != outcome.heading_raw_after),
            "positive descriptor contact missing: {contact:?}"
        ),
        ContactSide::Rear => {
            assert!(contact
                .descriptor_contacts
                .iter()
                .all(|outcome| outcome.entity_id != id));
            let peasant = world
                .entities
                .iter_all()
                .find(|entity| entity.id == id)
                .unwrap();
            assert_eq!(peasant.heading_raw(), heading_before);
            assert_eq!(peasant.sub_a_propulsion_runtime, sub_a_before);
        }
    }
    world.entities.player_mut().unwrap().position = player_start;
    let mut moves = 0;
    let mut previous = world
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap()
        .position_raw();
    for _ in 0..100 {
        world.step();
        let current = world
            .entities
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap()
            .position_raw();
        moves += usize::from([current[0], current[2]] != [previous[0], previous[2]]);
        previous = current;
    }
    assert!(moves > 0, "contact must not strand the selected peasant");
    assert!(world
        .scheduler
        .actor_animation_claims()
        .any(|lease| lease.entity_id == id));
}

#[v2k_test_support::retail_test]
fn lighthouse_peasant_keeps_selected_tasks_after_player_descriptor_contact() {
    for warmup_ticks in [10, 1_500] {
        assert_lighthouse_peasant_continues_after_contact(ContactSide::Forward, warmup_ticks);
    }
}

#[v2k_test_support::retail_test]
fn rear_player_contact_keeps_peasant_descriptor_state_and_selected_tasks() {
    for warmup_ticks in [10, 1_500] {
        assert_lighthouse_peasant_continues_after_contact(ContactSide::Rear, warmup_ticks);
    }
}
