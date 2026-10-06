use super::*;
use crate::{
    common_mover::type9_attitude::Type9BodyBasis,
    entity::EntityManager,
    gameplay_notifications::GameplayNotifications,
    intro2_type47_live::world::native_intro2_fixture,
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskFamily, SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
    world_fx::WorldFx,
};

struct Fixture {
    session: GameSession,
    manager: EntityManager,
    scheduler: SpecializedActorTaskScheduler,
    fx: WorldFx,
    static_damage: StaticDamageScheduler,
    notifications: GameplayNotifications,
    id: u32,
}
impl Fixture {
    fn new() -> Option<Self> {
        Self::with_tumble(true)
    }
    fn with_tumble(tumble: bool) -> Option<Self> {
        let (mut session, mut manager, _) = native_intro2_fixture()?;
        let id = manager
            .iter_all()
            .find(|e| e.authored_spawn_index == Some(55))
            .unwrap()
            .id;
        // Isolate the native source's own radial visit. A second test restores
        // another real allocation to exercise the incomplete radial boundary.
        let ids = manager
            .iter_all()
            .map(|entity| entity.id)
            .collect::<Vec<_>>();
        for id in ids {
            let entity = manager.entity_mut(id).unwrap();
            entity.collision.state_flags_at_0x08.overwrite(0x8000, 0);
        }
        let entity = manager.entity_mut(id).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(u32::MAX, 0x0247_8000);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        entity.physical_body_basis_q31 =
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(0, 0, 0));
        entity.set_motion_raw([0, -10, 0], [0, -600, 0]);
        let mut fx = WorldFx::new();
        for _ in 0..8 {
            fx.advance_frame_pacing(20_000);
        }
        let mut scheduler = SpecializedActorTaskScheduler::default();
        if tumble {
            let owner = publish_intro2_type10_standard_death(&mut manager, id, &mut fx)
                .unwrap()
                .unwrap();
            scheduler.register_intro2_type10_tumble(owner);
        } else {
            scheduler.adopt_intro2_type10(&manager);
        }
        for cell in &mut session.cache.level_terrain_mut().unwrap().cells {
            cell.height = 0;
            cell.attribute = 0;
        }
        Some(Self {
            session,
            manager,
            scheduler,
            fx,
            static_damage: StaticDamageScheduler::default(),
            notifications: GameplayNotifications::default(),
            id,
        })
    }
    fn frame(&mut self) -> Intro2ContactFrame<'_> {
        Intro2ContactFrame {
            entities: &mut self.manager,
            resources: &mut self.session.cache,
            world_fx: &mut self.fx,
            static_damage: &mut self.static_damage,
            notifications: &mut self.notifications,
            retail_tick: 1774,
            actor_tasks: &mut self.scheduler,
        }
    }
    fn run(&mut self) -> Intro2Type10ContactOutcome {
        let id = self.id;
        resolve_intro2_type10_tumble_contact(&mut self.frame(), id)
    }
    fn entity(&mut self) -> &mut Entity {
        self.manager.entity_mut(self.id).unwrap()
    }
}

#[v2k_test_support::retail_test]
fn native_type10_living_search_uses_shared_surface_without_entering_tumble() {
    let Some(mut f) = Fixture::with_tumble(false) else {
        return;
    };
    assert_eq!(f.run(), Intro2Type10ContactOutcome::Ineligible);
    let reports = crate::intro2_contacts::resolve_intro2_contacts(f.frame());
    assert!(reports.iter().any(|report| matches!(report,
    crate::intro2_contacts::Intro2ContactReport::NativeFlyingSurface {entity_id, result, ..}
    if *entity_id == f.id && matches!(result,
        crate::native_flying_surface_contact::NativeFlyingSurfaceContactOutcome::Applied {
            solid_contact: true, water_entry: false, ..
        }))));
    assert!(
        !reports.iter().any(|report| matches!(report,
        crate::intro2_contacts::Intro2ContactReport::Type10 {entity_id, ..} if *entity_id == f.id))
    );
    assert_eq!(
        f.scheduler.family_for(f.id),
        Some(SpecializedActorTaskFamily::Intro2Type10)
    );
    assert!(f.fx.particle_count() > 0);
    assert!(f.entity().position_raw()[1] > -10);
}

#[v2k_test_support::retail_test]
fn native_type10_terrain_terminal_keeps_self_radial_and_post_bac0_physical_response() {
    let Some(mut f) = Fixture::new() else { return };
    let position = f.entity().position_raw();
    let velocity = f.entity().velocity_raw();
    let Intro2Type10ContactOutcome::Applied(report) = f.run() else {
        panic!("terrain callback failed")
    };
    assert!(report.terrain_contact);
    assert!(!report.water_entry);
    let terminal = report.terminal.unwrap();
    assert_eq!(terminal.contact, Intro2Type10TumbleContact::Terrain);
    assert!(terminal.finalized);
    let Intro2RadialReport::Applied { dynamic, .. } = terminal.radial else {
        panic!()
    };
    assert!(dynamic.completed());
    assert_eq!(dynamic.completed_target_ids, [f.id]);
    assert_ne!(f.entity().position_raw(), position);
    assert_ne!(f.entity().velocity_raw(), velocity);
    assert!(report.collision_damage_raw > 0);
    assert_eq!(bits(f.entity(), 0x900000).unwrap(), 0x900000);
    assert_eq!(f.scheduler.family_for(f.id), None);
    assert_eq!(f.manager.pending_actor_deferred_destroy_ids(), [f.id]);
    let count = f.fx.particle_count();
    let position = f.entity().position_raw();
    assert!(matches!(
        f.run(),
        Intro2Type10ContactOutcome::Blocked {
            committed_prefix: false,
            ..
        }
    ));
    assert_eq!(f.entity().position_raw(), position);
    assert_eq!(f.fx.particle_count(), count);
}

#[v2k_test_support::retail_test]
fn native_type10_water_terminal_then_hard_response_adopts_ring_after_task_retirement() {
    let Some(mut f) = Fixture::new() else { return };
    for cell in &mut f.session.cache.level_terrain_mut().unwrap().cells {
        cell.height = (-128i8) as u8;
    }
    f.session.cache.level_terrain_mut().unwrap().header[0] = 0;
    let context = TerrainCollisionContext::from_current_level_cache(&f.session.cache).unwrap();
    let material = context
        .water_response_selectors
        .iter()
        .position(|&selector| selector != 7)
        .unwrap() as u8;
    for cell in &mut f.session.cache.level_terrain_mut().unwrap().cells {
        cell.terrain_type = material;
    }
    let terrain = f.session.cache.level_terrain().unwrap();
    let surface =
        v2k_formats::terrain::wave_surface_raw(0, 0, 1774, terrain.sea_level_raw(), -4096);
    f.entity().set_motion_raw([0, surface, 0], [0, -1001, 0]);
    f.entity()
        .collision
        .state_flags_at_0x08
        .overwrite(0x400000, 0x400000);
    let Intro2Type10ContactOutcome::Applied(report) = f.run() else {
        panic!("water callback failed")
    };
    assert!(!report.terrain_contact);
    assert!(report.water_entry);
    let terminal = report.terminal.unwrap();
    assert!(terminal.finalized);
    assert_eq!(terminal.contact, Intro2Type10TumbleContact::Water);
    assert_eq!(f.scheduler.family_for(f.id), None);
    let ring_id = f
        .manager
        .iter_all()
        .find(|e| e.entity_type == 60 && e.authored_spawn_index.is_none())
        .unwrap()
        .id;
    assert_eq!(
        f.scheduler.family_for(ring_id),
        Some(SpecializedActorTaskFamily::Type60ExplodingRing)
    );
    assert_eq!(
        f.manager.entity_mut(ring_id).unwrap().model_in_slot(0),
        Some(132)
    );
    assert_eq!(f.entity().velocity_raw()[1], -501);
    // The hard-entry suffix queues sound17 even after BAC0 marks the source
    // for deferred removal. Audio observes that request after event draining.
    f.fx.process_pending();
    assert!(f
        .fx
        .take_positional_sounds()
        .iter()
        .any(|sound| sound.sound_id == 17));
}

#[v2k_test_support::retail_test]
fn native_type10_static_scan_runs_without_terrain_bit_and_explodes_before_response() {
    let Some(mut f) = Fixture::new() else { return };
    f.entity()
        .collision
        .state_flags_at_0x08
        .overwrite(0x10000, 0);
    let candidates = f
        .session
        .cache
        .terrain_objects()
        .unwrap()
        .records
        .iter()
        .enumerate()
        .filter(|(index, record)| *index > 0 && record.kind_index == 0)
        .map(|(index, _)| index as u8)
        .collect::<Vec<_>>();
    let mut selected = None;
    'candidate: for attribute in candidates {
        f.session.cache.level_terrain_mut().unwrap().cells[2 * 256 + 3].attribute = attribute;
        for y in [0, 100, 200, 400] {
            let position = [0x27f, y, 0x37f];
            f.entity().set_position_raw(position);
            let entity = f.manager.iter_all().find(|e| e.id == f.id).unwrap();
            let contact = scan_deepest_static_contact(StaticContactQuery {
                terrain: f.session.cache.level_terrain().unwrap(),
                terrain_objects: f.session.cache.terrain_objects().unwrap(),
                model_pool: &f.session.cache,
                tick: 1774,
                active_model: f.session.cache.global_model(351).unwrap(),
                active_model_to_world_basis: Type9BodyBasis::from_angle_words(0, 0, 0)
                    .orientation_world_from_model()
                    .map(|row| row.map(f64::from)),
                active_anim_vars: &entity.presentation_anim_vars(1774),
                position_raw: position,
            })
            .unwrap();
            if let Some(contact) = contact {
                selected = Some(contact);
                break 'candidate;
            }
        }
    }
    let contact = selected.expect("canonical kind0 solid fixture intersects model351");
    f.entity().set_velocity_raw(
        contact
            .normal_q12
            .map(|normal| (-i32::from(normal) * 3000 >> 12) as i16),
    );
    let before = f.entity().velocity_raw();
    let Intro2Type10ContactOutcome::Applied(report) = f.run() else {
        panic!("static callback failed")
    };
    assert!(!report.terrain_contact);
    assert!(!report.water_entry);
    assert_eq!(report.static_contact, Some(contact));
    let terminal = report.terminal.unwrap();
    assert!(terminal.finalized);
    assert_eq!(terminal.contact, Intro2Type10TumbleContact::Static);
    assert_ne!(f.entity().velocity_raw(), before);
    assert_eq!(f.manager.pending_actor_deferred_destroy_ids(), [f.id]);
}

#[v2k_test_support::retail_test]
fn native_type10_incomplete_radial_keeps_terminal_claim_and_never_replays_effect_prefix() {
    let Some(mut f) = Fixture::new() else { return };
    let other = f
        .manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(56))
        .unwrap()
        .id;
    let position = f.entity().position_raw();
    let target = f.manager.entity_mut(other).unwrap();
    target.set_position_raw([position[0] + 100, position[1], position[2]]);
    target
        .collision
        .state_flags_at_0x08
        .overwrite(u32::MAX, 0x0247_8000);
    // Native56 is deliberately not registered in this test's scheduler.
    let outcome = f.run();
    let Intro2Type10ContactOutcome::Blocked {
        reason: Intro2Type10ContactBlock::RadialIncomplete,
        committed_prefix: true,
        report,
    } = outcome
    else {
        panic!("{outcome:?}")
    };
    let terminal = report.terminal.unwrap();
    assert!(!terminal.finalized);
    let Intro2RadialReport::Applied { dynamic, .. } = terminal.radial else {
        panic!()
    };
    assert_eq!(dynamic.blocked.unwrap().target_id, other);
    assert!(f.manager.pending_actor_deferred_destroy_ids().is_empty());
    assert!(terminal_is_pending(f.entity()));
    let count = f.fx.particle_count();
    let mut rng = f.fx.fork_for_main_base_abort_transaction();
    assert!(matches!(
        f.run(),
        Intro2Type10ContactOutcome::Blocked {
            committed_prefix: false,
            ..
        }
    ));
    assert_eq!(f.fx.particle_count(), count);
    assert_eq!(
        f.fx.next_shared_retail_random_u16(),
        rng.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_type10_contact_pending_gate_precedes_geometry_but_ineligible_precedes_owner() {
    let Some(mut f) = Fixture::new() else { return };
    f.scheduler.park_intro2_type10_tumble_contact_prefix(f.id);
    f.entity().physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    assert!(matches!(
        f.run(),
        Intro2Type10ContactOutcome::Blocked {
            reason: Intro2Type10ContactBlock::Runtime("current completed Tumble owner"),
            committed_prefix: false,
            ..
        }
    ));
    f.entity().collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(1);
    assert_eq!(f.run(), Intro2Type10ContactOutcome::Ineligible);
    assert_eq!(f.fx.particle_count(), 0);
}
