//! Native dragon custody at the actual inline radial cursor and hit boundary.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskSlot, PreparedActorTask},
    damage::DamagePacket,
    entity::{DynamicRadialLiveOutcome, DynamicRadialLivePhase},
    entity_collision_state::RetailRuntimeValue,
    intro2_radial::{
        apply_intro2_radial_damage, Intro2RadialFrame, Intro2RadialReport, Intro2RadialTaskCustody,
    },
    intro2_type10::{Intro2Type10Owner, Intro2Type10TumbleOutcome},
    intro2_type47_live::world::native_intro2_fixture,
    radial_damage::RadialDamageTemplate,
    resource_cache::ResourceCache,
    static_damage::StaticDamageScheduler,
    world_fx::{BallisticDamageRequest, ParticleEntityImpact, WorldFx},
};

const ORIGIN: [i16; 3] = [0, 20_000, 0];

fn prepare(manager: &mut EntityManager, spawn: usize, offset: i16) -> u32 {
    let id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(spawn))
        .unwrap()
        .id;
    let entity = manager.entity_mut(id).unwrap();
    entity.set_motion_raw([offset, ORIGIN[1], 0], [0; 3]);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(u32::MAX, 0x0206_8000);
    entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    entity.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    id
}

fn template(lethal: bool) -> RadialDamageTemplate {
    RadialDamageTemplate {
        inner_radius_raw: 64,
        outer_radius_raw: 128,
        impulse_raw: if lethal { 0 } else { 1000 },
        packet: DamagePacket {
            channels: [1, 0],
            amounts_raw: [if lethal { 50000 } else { 10000 }, 0],
        },
        trailing_raw: [-1, 0],
    }
}

fn radial(
    manager: &mut EntityManager,
    resources: &mut ResourceCache,
    fx: &mut WorldFx,
    custody: &mut dyn Intro2RadialTaskCustody,
    lethal: bool,
) -> DynamicRadialLiveOutcome {
    let report = apply_intro2_radial_damage(
        &mut Intro2RadialFrame {
            active_terminal_calls: Vec::new(),
            entities: manager,
            resources,
            world_fx: fx,
            static_damage: &mut StaticDamageScheduler::new(),
            notifications: &mut GameplayNotifications::new(),
            retail_tick: 3000,
            actor_tasks: custody,
        },
        ORIGIN,
        template(lethal),
    );
    let Intro2RadialReport::Applied {
        static_deliveries,
        dynamic,
    } = report
    else {
        panic!("{report:?}")
    };
    assert!(
        static_deliveries.is_empty(),
        "high fixture excludes static cells"
    );
    dynamic
}

fn age(manager: &EntityManager, id: u32) -> u32 {
    match manager
        .iter_all()
        .find(|e| e.id == id)
        .unwrap()
        .actor_task_state(ActorTaskSlot::Primary)
    {
        Some(ActorTaskRuntime::TumbleOutOfSky(task)) => task.elapsed_ms(),
        other => panic!("expected native class11: {other:?}"),
    }
}

fn frame<'a>(
    resources: &'a mut ResourceCache,
    fx: &'a mut WorldFx,
    static_damage: &'a mut StaticDamageScheduler,
) -> SpecializedActorTaskProductionFrame<'a> {
    SpecializedActorTaskProductionFrame {
        world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
        hive_components: None,
        notification_phase: crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
        resources,
        world_fx: fx,
        static_damage,
        elapsed_micros: 20000,
        global_elapsed_micros: 20000,
        retail_tick: 3001,
        main_base_abort_active: false,
    }
}

#[v2k_test_support::retail_test]
fn native_type10_inline_radial_replaces_both_slots_but_only_unvisited_tumble_runs_this_pass() {
    for reverse_registration in [false, true] {
        let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let earlier = prepare(&mut manager, 55, 0);
        let later = prepare(&mut manager, 56, 0);
        let old_tasks = [earlier, later].map(|id| {
            manager
                .entity_mut(id)
                .unwrap()
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
        });
        let mut owners: Vec<_> = [earlier, later]
            .map(|id| {
                SpecializedActorTaskOwner::Intro2Type10(
                    Intro2Type10Owner::adopt(&manager, id).unwrap(),
                )
            })
            .into_iter()
            .collect();
        if reverse_registration {
            owners.reverse();
        }
        // These are the two real storage locations used during the production
        // intrusive walk. The first actor has been visited; the second remains
        // at the live cursor. Radial delivery itself performs both deaths.
        let (mut pending, mut retained): (Vec<_>, Vec<_>) = owners
            .into_iter()
            .partition(|owner| owner.entity_id() == later);
        let mut fx = WorldFx::new();
        let mut oracle = WorldFx::new();
        let result = radial(
            &mut manager,
            &mut session.cache,
            &mut fx,
            &mut Intro2RadialCursorCustody {
                pending: &mut pending,
                retained: &mut retained,
                remaining_live_ids: &[later],
            },
            true,
        );
        assert_eq!(result.blocked, None);
        assert_eq!(result.completed_target_ids, [earlier, later]);
        assert_eq!(result.death_publications.len(), 2);
        assert!(
            matches!(pending.as_slice(), [SpecializedActorTaskOwner::Intro2Type10Tumble(owner)] if owner.entity_id() == later)
        );
        assert!(
            matches!(retained.as_slice(), [SpecializedActorTaskOwner::Intro2Type10Tumble(owner)] if owner.entity_id() == earlier)
        );
        assert_eq!([age(&manager, earlier), age(&manager, later)], [0, 0]);
        for (index, id) in [earlier, later].into_iter().enumerate() {
            let e = manager.entity_mut(id).unwrap();
            assert_ne!(
                e.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                old_tasks[index]
            );
            assert!(e
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Secondary)
                .is_none());
            assert!(e
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Tertiary)
                .is_none());
            assert_eq!(e.collision.health_raw, RetailRuntimeValue::Known(0));
            oracle.next_shared_retail_random_u16(); // own404360/06070 G draw
        }
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.owners = pending;
        let mut static_damage = StaticDamageScheduler::new();
        let pass = scheduler.tick(
            &mut manager,
            frame(&mut session.cache, &mut fx, &mut static_damage),
            &mut GameplayNotifications::new(),
        );
        assert_eq!(pass.block, None);
        assert!(
            matches!(pass.outcomes.as_slice(), [SpecializedActorTaskProductionOutcome::Intro2Type10Tumble(
            Intro2Type10TumbleOutcome::Advanced { entity_id, detailed: true, .. })] if *entity_id == later)
        );
        assert_eq!([age(&manager, earlier), age(&manager, later)], [0, 20]);
        scheduler.owners.extend(retained);
        let pass = scheduler.tick(
            &mut manager,
            frame(&mut session.cache, &mut fx, &mut static_damage),
            &mut GameplayNotifications::new(),
        );
        assert_eq!(
            pass.outcomes
                .iter()
                .map(SpecializedActorTaskProductionOutcome::entity_id)
                .collect::<Vec<_>>(),
            [earlier, later]
        );
        assert!(pass.outcomes.iter().all(|outcome| matches!(
            outcome,
            SpecializedActorTaskProductionOutcome::Intro2Type10Tumble(
                Intro2Type10TumbleOutcome::Advanced { .. }
            )
        )));
        assert_eq!([age(&manager, earlier), age(&manager, later)], [20, 40]);
        assert_eq!(scheduler.owners.len(), 2);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16(),
            "no old living callback, duplicate death constructor, or extra earlier visit"
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type10_radial_rejects_missing_stale_pending_and_executing_task_custody() {
    for boundary in 0..4 {
        let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = prepare(&mut manager, 55, 16);
        let owner = Intro2Type10Owner::adopt(&manager, id).unwrap();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        if boundary != 0 {
            scheduler.register_intro2_type10(owner);
        }
        if boundary == 1 {
            let entity = manager.entity_mut(id).unwrap();
            let task = entity
                .actor_task_state(ActorTaskSlot::Primary)
                .unwrap()
                .clone();
            entity
                .actor_tasks
                .replace_prepared(ActorTaskSlot::Primary, PreparedActorTask::new(task));
        } else if boundary == 2 {
            scheduler.park_intro2_type10_external_prefix(id);
        } else if boundary == 3 {
            let entity = manager.entity_mut(id).unwrap();
            let visit = crate::actor_task_owner::ActorTaskVisit {
                slot: ActorTaskSlot::Primary,
                task_id: entity
                    .actor_tasks
                    .task_in_slot(ActorTaskSlot::Primary)
                    .unwrap(),
            };
            assert_eq!(
                entity.actor_tasks.begin_exact_visit_with(visit, |_| ()),
                Some(())
            );
        }
        let mut fx = WorldFx::new();
        let mut oracle = WorldFx::new();
        let result = radial(
            &mut manager,
            &mut session.cache,
            &mut fx,
            &mut scheduler,
            false,
        );
        let block = result.blocked.unwrap();
        assert_eq!(block.target_id, id);
        assert_eq!(block.phase, DynamicRadialLivePhase::MutationCustody);
        assert!(!block.target_prefix_committed);
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(entity.velocity_raw(), [0; 3]);
        assert_eq!(
            entity.collision.health_raw,
            RetailRuntimeValue::Known(32000)
        );
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type10_late_radial_buffer_failure_parks_impulse_for_scheduler_hit_and_radial_reentry() {
    for cursor_storage in 0..3 {
        let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = prepare(&mut manager, 55, 16);
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .pre_health_damage_buffer_raw = RetailRuntimeValue::Unresolved;
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.register_intro2_type10(Intro2Type10Owner::adopt(&manager, id).unwrap());
        let mut fx = WorldFx::new();
        let mut oracle = WorldFx::new();
        let result = if cursor_storage == 0 {
            radial(
                &mut manager,
                &mut session.cache,
                &mut fx,
                &mut scheduler,
                false,
            )
        } else {
            let mut pending = if cursor_storage == 1 {
                std::mem::take(&mut scheduler.owners)
            } else {
                Vec::new()
            };
            let mut retained = if cursor_storage == 2 {
                std::mem::take(&mut scheduler.owners)
            } else {
                Vec::new()
            };
            let result = radial(
                &mut manager,
                &mut session.cache,
                &mut fx,
                &mut Intro2RadialCursorCustody {
                    pending: &mut pending,
                    retained: &mut retained,
                    remaining_live_ids: if cursor_storage == 1 {
                        std::slice::from_ref(&id)
                    } else {
                        &[]
                    },
                },
                false,
            );
            scheduler.owners.extend(pending);
            scheduler.owners.extend(retained);
            result
        };
        let block = result.blocked.unwrap();
        assert_eq!(block.target_id, id);
        assert_eq!(block.phase, DynamicRadialLivePhase::Buffer);
        assert!(block.target_prefix_committed);
        assert!(scheduler.intro2_type10_has_pending_prefix(id));
        let velocity = manager.entity_mut(id).unwrap().velocity_raw();
        assert!(
            velocity[0] > 0,
            "the committed outward impulse is the failure prefix"
        );
        assert_eq!(
            manager.entity_mut(id).unwrap().collision.health_raw,
            RetailRuntimeValue::Known(32000)
        );
        let second = radial(
            &mut manager,
            &mut session.cache,
            &mut fx,
            &mut scheduler,
            false,
        );
        assert_eq!(
            second.blocked.unwrap().phase,
            DynamicRadialLivePhase::MutationCustody
        );
        let hit = crate::intro2_type10::impact::apply_intro2_type10_particle_hit(
            &mut manager,
            &session.cache,
            &mut fx,
            &mut scheduler,
            ParticleEntityImpact {
                source_particle_class: 16,
                impact_position_argument_va: 0,
                target_entity_id: id,
                position_world: [0.; 3],
                velocity_raw: [0, 0, 8192],
                damage: Some(BallisticDamageRequest {
                    packet: DamagePacket {
                        channels: [1, 0],
                        amounts_raw: [10000, 0],
                    },
                    source_entity_type_at_birth: Some(34),
                    source_owner_id: Some(35),
                }),
            },
            3001,
        );
        assert!(
            matches!(
                hit,
                crate::intro2_type10::impact::Intro2Type10ImpactOutcome::Blocked {
                    reason: crate::intro2_type10::impact::Intro2Type10ImpactBlock::Runtime(
                        "pending actor prefix"
                    ),
                    committed_prefix: false,
                }
            ),
            "{hit:?}"
        );
        let pass = scheduler.tick(
            &mut manager,
            frame(
                &mut session.cache,
                &mut fx,
                &mut StaticDamageScheduler::new(),
            ),
            &mut GameplayNotifications::new(),
        );
        assert!(
            matches!(pass.outcomes.as_slice(), [SpecializedActorTaskProductionOutcome::Intro2Type10(
            crate::intro2_type10::Intro2Type10Outcome::Pending { entity_id })] if *entity_id == id)
        );
        assert_eq!(manager.entity_mut(id).unwrap().velocity_raw(), velocity);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16(),
            "the failed buffer read, rejected reentries, and parked scheduler consume no RNG"
        );
    }
}
