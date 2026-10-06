use super::*;
use crate::{
    entity::{AuthoredWorldConstruction, EntityConstructionResources},
    gameplay_notifications::GameplayNotifications,
    intro2_common_dying::publish_intro2_common_standard_death,
    session::GameSession,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
    world_fx::WorldFx,
};

enum BirthStage {
    Acquiring,
    Following,
}

struct Fixture {
    session: GameSession,
    entities: EntityManager,
    scheduler: SpecializedActorTaskScheduler,
    fx: WorldFx,
    damage: StaticDamageScheduler,
    notifications: GameplayNotifications,
    id: u32,
    tick: u32,
}

impl Fixture {
    fn load(stage: BirthStage) -> Option<Self> {
        let (mut session, metadata) = super::super::tests::fixture()?;
        session.load_level_by_id(13, 1).unwrap();
        // Choose a real Follow birth from a bounded process RNG prehistory.
        // No synthetic behavior context, task or allocation receipt is used.
        for prehistory in 0..32 {
            let mut fx = WorldFx::new();
            for _ in 0..prehistory {
                fx.next_shared_retail_random_u16();
            }
            let mut entities = EntityManager::from_authored_world(
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
            let selected = entities.iter_all().find(|entity| entity.entity_type == 17
                && matches!(entity.initial_behavior, RetailRuntimeValue::Known(Some(selection)) if selection.program.class_id == 33))
                .map(|entity| entity.id);
            let Some(id) = selected else { continue };
            let mut owner = Intro2Type17Owner::adopt(&entities, id).unwrap();
            let mut tick = 0;
            if matches!(stage, BirthStage::Following) {
                let mut following = false;
                for frame in 1..=128 {
                    tick = frame;
                    let result = tick_intro2_type17(
                        &mut entities,
                        owner,
                        Intro2Type17Frame {
                            capture_tasks: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler::default(),
                            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                            resources: &session.cache,
                            world_fx: &mut fx,
                            elapsed_micros: 20_000,
                            retail_tick: tick,
                        },
                    );
                    assert!(
                        !matches!(
                            result.outcome,
                            Intro2Type17Outcome::Blocked { .. }
                                | Intro2Type17Outcome::Pending { .. }
                        ),
                        "{:?}",
                        result.outcome
                    );
                    owner = result.retained_owner.expect("live shared Type17");
                    if matches!(
                        entities
                            .entity_mut(id)
                            .unwrap()
                            .actor_task_state(ActorTaskSlot::Primary),
                        Some(ActorTaskRuntime::FollowBeaconsFollowing(_))
                    ) {
                        following = true;
                        break;
                    }
                }
                assert!(
                    following,
                    "real Follow birth never acquired an authored beacon"
                );
            }
            let mut scheduler = SpecializedActorTaskScheduler::default();
            scheduler.register_intro2_type17(owner);
            return Some(Self {
                session,
                entities,
                scheduler,
                fx,
                damage: StaticDamageScheduler::default(),
                notifications: GameplayNotifications::new(),
                id,
                tick,
            });
        }
        panic!("no actual Type17 Follow birth in bounded canonical prehistory");
    }

    fn entity(&self) -> &Entity {
        self.entities
            .iter_all()
            .find(|entity| entity.id == self.id)
            .unwrap()
    }

    fn task(&self) -> crate::follow_beacons::FollowBeaconsFollowingTaskState {
        let Some(ActorTaskRuntime::FollowBeaconsFollowing(task)) =
            self.entity().actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("real Following task")
        };
        *task
    }

    fn resolve(&mut self) -> Type17ContactOutcome {
        resolve_type17_static_contact(
            &mut Intro2ContactFrame {
                entities: &mut self.entities,
                resources: &mut self.session.cache,
                world_fx: &mut self.fx,
                static_damage: &mut self.damage,
                notifications: &mut self.notifications,
                retail_tick: self.tick,
                actor_tasks: &mut self.scheduler,
            },
            self.id,
        )
    }

    fn find_contact(
        &mut self,
        center: [i16; 3],
        expected_cell: [u8; 2],
        model_id: u16,
    ) -> ([i16; 3], StaticModelContact) {
        self.locate_contact(center, expected_cell, model_id)
            .unwrap_or_else(|| {
                panic!("native model256 must overlap authored pen model{model_id} near {center:?}")
            })
    }

    fn locate_contact(
        &mut self,
        center: [i16; 3],
        expected_cell: [u8; 2],
        model_id: u16,
    ) -> Option<([i16; 3], StaticModelContact)> {
        // Locate overlap using the native owner's actual incoming body basis.
        // The neighborhood is confined to the previously proven pen cells.
        for dx in [0i16, -64, 64, -128, 128, -256, 256, -384, 384, -512, 512] {
            for dz in [0i16, -64, 64, -128, 128, -256, 256, -384, 384, -512, 512] {
                for dy in [0i16, -64, 64, -128, 128, -256, 256, -384, 384] {
                    let position = [
                        center[0].wrapping_add(dx),
                        center[1].wrapping_add(dy),
                        center[2].wrapping_add(dz),
                    ];
                    let entity = self.entity();
                    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
                        panic!()
                    };
                    let model = self
                        .session
                        .cache
                        .global_model(entity.model_index.unwrap())
                        .unwrap();
                    let contact = scan_deepest_static_contact(StaticContactQuery {
                        terrain: self.session.cache.terrain().unwrap(),
                        terrain_objects: self.session.cache.terrain_objects().unwrap(),
                        model_pool: &self.session.cache,
                        tick: self.tick,
                        active_model: model,
                        active_model_to_world_basis: basis
                            .orientation_world_from_model()
                            .map(|row| row.map(f64::from)),
                        active_anim_vars: &entity.presentation_anim_vars(self.tick),
                        position_raw: position,
                    })
                    .unwrap();
                    if let Some(contact) = contact.filter(|contact| {
                        contact.cell == expected_cell && contact.model_id == model_id
                    }) {
                        self.entities
                            .entity_mut(self.id)
                            .unwrap()
                            .set_motion_raw(position, [0; 3]);
                        return Some((position, contact));
                    }
                }
            }
        }
        None
    }
}

#[v2k_test_support::retail_test]
fn world20_living_and_dying_spiders_complete_kind4_contact_with_exact_task_custody() {
    for (burn, dying) in [(false, false), (true, false), (true, true)] {
        let Some((mut session, metadata)) = super::super::tests::fixture() else {
            return;
        };
        session.load_level_by_id(20, 1).unwrap();
        let mut fx = WorldFx::new();
        let entities = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                level: session.cache.level_desc().unwrap(),
                logical_world_index: 8,
                type_metadata: &metadata,
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| {
                        session.cache.global_model(id).map(|model| model.radius)
                    }),
                },
                player_arrival: Some(crate::entity::AuthoredPlayerArrival {
                    position_raw: [19712, -500, 14848],
                    heading_raw: 0x4000,
                }),
                retail_tick: 0,
            },
            &mut fx,
        )
        .unwrap();
        let id = entities
            .iter_all()
            .find(|entity| entity.entity_type == 17)
            .unwrap()
            .id;
        let mut scheduler = SpecializedActorTaskScheduler::default();
        scheduler.register_intro2_type17(Intro2Type17Owner::adopt(&entities, id).unwrap());
        let mut world = Fixture {
            session,
            entities,
            scheduler,
            fx,
            damage: StaticDamageScheduler::new(),
            notifications: GameplayNotifications::new(),
            id,
            tick: 0,
        };
        if dying {
            let owner =
                publish_intro2_common_standard_death(&mut world.entities, id, &mut world.fx)
                    .unwrap()
                    .expect("actual native Class12 publication");
            world.scheduler.register_intro2_common_dying(owner);
        }
        let terrain = world.session.cache.terrain().unwrap();
        let objects = world.session.cache.terrain_objects().unwrap();
        let candidates: Vec<_> = terrain
            .cells
            .iter()
            .enumerate()
            .filter_map(|(index, cell)| {
                if cell.attribute == 0 || cell.terrain_type & 0x08 != 0 {
                    return None;
                }
                let descriptor = objects.records.get(usize::from(cell.attribute))?;
                (descriptor.kind_index == 4).then_some((
                    [
                        ((index / 256) as u16 * 256 + 127) as i16,
                        (i16::from(cell.height as i8) * 32),
                        ((index % 256) as u16 * 256 + 127) as i16,
                    ],
                    [(index / 256) as u8, (index % 256) as u8],
                    descriptor.model_id_for(cell.terrain_type),
                ))
            })
            .collect();
        let (position, contact) = candidates
            .into_iter()
            .find_map(|(center, cell, model)| world.locate_contact(center, cell, model))
            .expect("world20 authors a solid kind4 fuel object");
        //11760's-256 response adds one-eighth rebound, not a full reflection.
        // Derive the search bound from the actual plane so every candidate's
        // components fit signed words; measure damage with the current mass.
        let max_normal_axis = contact
            .normal_q12
            .iter()
            .map(|axis| i32::from(*axis).abs())
            .max()
            .unwrap();
        assert!(max_normal_axis > 0);
        let max_inward_speed =
            (i32::from(i16::MAX) * 4096 / max_normal_axis).min(i32::from(i16::MAX));
        let velocity = (1..=max_inward_speed)
            .find_map(|speed| {
                let before = contact
                    .normal_q12
                    .map(|axis| (-i32::from(axis) * speed / 4096) as i16);
                let mut after = before;
                let mut after_position = position;
                apply_contact_response_raw(&mut after_position, &mut after, contact);
                let impact = velocity_delta_impact_raw(before, after, world.entity().mass_raw);
                let desired = if burn { 4500..6500 } else { 1..2000 };
                desired.contains(&impact).then_some(before)
            })
            .unwrap_or_else(|| panic!("controlled inward speed must reach burn={burn}, dying={dying}, mass={}, contact={contact:?}", world.entity().mass_raw));
        let before_cell = world.session.cache.terrain().unwrap().cells
            [usize::from(contact.cell[0]) * 256 + usize::from(contact.cell[1])];
        world
            .entities
            .entity_mut(id)
            .unwrap()
            .set_motion_raw(position, velocity);
        let before_primary = world
            .entity()
            .actor_task_state(ActorTaskSlot::Primary)
            .cloned();
        let before_sub_a = world.entity().sub_a_propulsion_runtime;
        let before_basis = world.entity().physical_body_basis_q31();
        let mut expected_fx = world.fx.fork_for_main_base_abort_transaction();
        if burn {
            // 4337E0's kind4 listener always consumes the sound62 rate word,
            // after direction-table scatter; it has no chance draw.
            expected_fx.next_shared_retail_random_u16();
        }
        let result = world.resolve();
        let Type17ContactOutcome::Applied(applied) = result else {
            panic!("{result:?}")
        };
        assert_eq!(applied.contact.kind_index, 4);
        if burn {
            assert!(matches!(
                applied.static_damage,
                Some(StaticDamageOutcome::ImmediateBurn { sample: None, .. })
            ));
        } else {
            assert_eq!(
                applied.static_damage,
                Some(StaticDamageOutcome::NoDamage { severity_raw: 0 })
            );
        }
        let after_cell = world.session.cache.terrain().unwrap().cells
            [usize::from(contact.cell[0]) * 256 + usize::from(contact.cell[1])];
        assert_eq!(after_cell.attribute, before_cell.attribute);
        assert_eq!(
            after_cell.terrain_type,
            before_cell.terrain_type | if burn { 0x08 } else { 0 }
        );
        assert!(
            applied.actor_damage.is_some(),
            "the static hit must return to the actor recipient"
        );
        assert_eq!(world.damage.active_program_count(), 0);
        assert!(!world.scheduler.intro2_type17_has_pending_prefix(id));
        assert!(world
            .scheduler
            .prepare_native_actor_mutation(&world.entities, id));
        if dying {
            assert!(matches!(
                before_primary.as_ref(),
                Some(ActorTaskRuntime::CommonDying(_))
            ));
            assert_eq!(
                world.entity().actor_task_state(ActorTaskSlot::Primary),
                before_primary.as_ref()
            );
            assert_eq!(world.entity().sub_a_propulsion_runtime, before_sub_a);
            assert_eq!(world.entity().physical_body_basis_q31(), before_basis);
            assert_eq!(
                world.fx.next_shared_retail_random_u16(),
                expected_fx.next_shared_retail_random_u16(),
                "Class12 contact consumes only the kind4 burn-listener sound word"
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn native_following_contacts_ordinary_and_mirrored_fences_without_rebuilding_basis() {
    for (center, cell, model) in [
        ([-26753, -85, -26945], [152, 150], 518),
        ([-24961, 3, -26881], [157, 151], 532),
        ([-24705, 91, -27265], [158, 149], 542),
    ] {
        let Some(mut world) = Fixture::load(BirthStage::Following) else {
            return;
        };
        let (position, _) = world.find_contact(center, cell, model);
        let before = world.task();
        let basis = world.entity().physical_body_basis_q31();
        let mut expected_fx = world.fx.fork_for_main_base_abort_transaction();
        let x = expected_fx.next_shared_retail_random_u16();
        let z = expected_fx.next_shared_retail_random_u16();
        let Type17ContactOutcome::Applied(applied) = world.resolve() else {
            panic!("native Following contact")
        };
        assert_eq!(applied.contact.cell, cell);
        assert_eq!(applied.contact.model_id, model);
        assert_eq!(applied.impact_raw, 0);
        assert!(applied.static_damage.is_none() && applied.actor_damage.is_none());
        let after = world.task();
        assert_eq!(after.target_id(), before.target_id());
        assert_eq!(after.elapsed_ms(), before.elapsed_ms());
        assert_eq!(
            after.private_state().direction,
            -before.private_state().direction
        );
        assert_eq!(after.private_state().reversal_timer_ms, 2500);
        assert_eq!(
            after.private_state().target_position_raw,
            [
                position[0].wrapping_add((x >> 6) as i16 - 0x200),
                before.private_state().target_position_raw[1],
                position[2].wrapping_add((z >> 6) as i16 - 0x200)
            ]
        );
        assert_eq!(world.entity().physical_body_basis_q31(), basis);
        assert_eq!(
            world.fx.next_shared_retail_random_u16(),
            expected_fx.next_shared_retail_random_u16()
        );
        assert!(world
            .scheduler
            .prepare_native_actor_mutation(&world.entities, world.id));
    }
}

#[v2k_test_support::retail_test]
fn breached_cell_stops_colliding_and_acquiring_hook_preserves_its_secondary_and_clocks() {
    let Some(mut world) = Fixture::load(BirthStage::Following) else {
        return;
    };
    let (position, _) = world.find_contact([-24961, 3, -26881], [157, 151], 532);
    world
        .session
        .cache
        .or_level_terrain_type_bits([157, 151], 0x08);
    assert_eq!(
        world
            .session
            .cache
            .global_model(533)
            .unwrap()
            .collision_radius_raw,
        0
    );
    let mut expected_fx = world.fx.fork_for_main_base_abort_transaction();
    let before = world.task();
    match world.resolve() {
        Type17ContactOutcome::Miss => {
            assert_eq!(world.entity().position_raw(), position);
            assert_eq!(world.task(), before);
        }
        Type17ContactOutcome::Applied(applied) => {
            // This actual body basis can also overlap the neighboring intact
            // model540. That legitimate contact must survive the532->533 breach.
            assert_ne!(applied.contact.cell, [157, 151]);
            assert_ne!(applied.contact.model_id, 533);
            expected_fx.next_shared_retail_random_u16();
            expected_fx.next_shared_retail_random_u16();
        }
        other => panic!("breach contact: {other:?}"),
    }
    assert_eq!(
        world.fx.next_shared_retail_random_u16(),
        expected_fx.next_shared_retail_random_u16()
    );

    let mut acquiring = Fixture::load(BirthStage::Acquiring).unwrap();
    acquiring.find_contact([-24961, 3, -26881], [157, 151], 532);
    let mut expected_fx = acquiring.fx.fork_for_main_base_abort_transaction();
    let secondary = acquiring
        .entity()
        .actor_task_state(ActorTaskSlot::Secondary)
        .cloned();
    let primary = acquiring
        .entity()
        .actor_task_state(ActorTaskSlot::Primary)
        .cloned()
        .unwrap();
    assert!(matches!(
        acquiring.resolve(),
        Type17ContactOutcome::Applied(_)
    ));
    expected_fx.next_shared_retail_random_u16();
    expected_fx.next_shared_retail_random_u16();
    assert_eq!(
        acquiring
            .entity()
            .actor_task_state(ActorTaskSlot::Secondary),
        secondary.as_ref()
    );
    let (ActorTaskRuntime::SharedRetarget(before), Some(ActorTaskRuntime::SharedRetarget(after))) = (
        primary,
        acquiring.entity().actor_task_state(ActorTaskSlot::Primary),
    ) else {
        panic!("acquiring Primary remains SharedRetarget")
    };
    assert_eq!(after.elapsed_ms(), before.elapsed_ms());
    assert_eq!(after.lifetime_ms(), before.lifetime_ms());
    assert_eq!(
        after.private_state().direction,
        -before.private_state().direction
    );
    acquiring
        .entities
        .entity_mut(acquiring.id)
        .unwrap()
        .set_position_raw([0, 30000, 0]);
    assert_eq!(acquiring.resolve(), Type17ContactOutcome::Miss);
    assert_eq!(
        acquiring.fx.next_shared_retail_random_u16(),
        expected_fx.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn pursuit_and_run_away_contact_preserve_their_typed_lifetime_route_and_audio() {
    for class in [9, 10] {
        let Some(mut world) = Fixture::load(BirthStage::Acquiring) else {
            return;
        };
        let target = world
            .entities
            .iter_all()
            .find(|entity| {
                entity.id != world.id
                    && entity.capability_flags & if class == 9 { 0xc00 } else { 1 } != 0
            })
            .unwrap()
            .id;
        let position = world.entity().position_raw();
        world
            .entities
            .entity_mut(target)
            .unwrap()
            .set_position_raw(position);
        let mut acquired = false;
        for _ in 0..128 {
            super::super::behavior::reselect(
                &mut world.entities,
                world.id,
                world.tick,
                &mut world.fx,
                super::super::behavior::ReselectionEntry::Impact,
            )
            .unwrap();
            let mut owner = Intro2Type17Owner::adopt(&world.entities, world.id).unwrap();
            let RetailRuntimeValue::Known(Some(context)) = world.entity().current_behavior_context
            else {
                panic!()
            };
            if context.active_style().style_address()
                != if class == 9 { 0x4c7ff0 } else { 0x4c7618 }
            {
                continue;
            }
            for _ in 0..32 {
                world.tick += 1;
                let tick = tick_intro2_type17(
                    &mut world.entities,
                    owner,
                    Intro2Type17Frame {
                        capture_tasks: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler::default(),
                        notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                        resources: &world.session.cache,
                        world_fx: &mut world.fx,
                        elapsed_micros: 20_000,
                        retail_tick: world.tick,
                    },
                );
                assert!(
                    !matches!(
                        tick.outcome,
                        Intro2Type17Outcome::Blocked { .. } | Intro2Type17Outcome::Pending { .. }
                    ),
                    "{:?}",
                    tick.outcome
                );
                owner = tick.retained_owner.unwrap();
                acquired = matches!(
                    (
                        class,
                        world.entity().actor_task_state(ActorTaskSlot::Primary)
                    ),
                    (9, Some(ActorTaskRuntime::CapturePeoplePursuit(_)))
                        | (10, Some(ActorTaskRuntime::RunAway(_)))
                );
                if acquired {
                    break;
                }
            }
            if acquired {
                world.scheduler.register_intro2_type17(owner);
                break;
            }
        }
        assert!(acquired, "class{class} did not acquire actual nearby actor");
        world.find_contact([-24961, 3, -26881], [157, 151], 532);
        let mut expected = world
            .entity()
            .actor_task_state(ActorTaskSlot::Primary)
            .cloned()
            .unwrap();
        let Type17StaticTaskHook::WanderPrivate(private_before) =
            contact_task_hook(world.entity()).unwrap()
        else {
            panic!("living task retains02CA0")
        };
        let position = world.entity().position_raw();
        let mut expected_fx = world.fx.fork_for_main_base_abort_transaction();
        let x = expected_fx.next_shared_retail_random_u16();
        let z = expected_fx.next_shared_retail_random_u16();
        let mut after = private_before;
        after.direction = after.direction.wrapping_neg();
        after.reversal_timer_ms = 2500;
        after.target_position_raw[0] = position[0].wrapping_add((x >> 6) as i16 - 0x200);
        after.target_position_raw[2] = position[2].wrapping_add((z >> 6) as i16 - 0x200);
        match &mut expected {
            ActorTaskRuntime::CapturePeoplePursuit(task) => {
                let mut stage = task.stage_callback();
                *stage.private_state_mut() = after;
                stage.commit(task);
            }
            ActorTaskRuntime::RunAway(task) => {
                let mut stage = task.stage_callback();
                *stage.private_state_mut() = after;
                stage.commit(task);
            }
            _ => unreachable!(),
        }
        let basis = world.entity().physical_body_basis_q31();
        assert!(matches!(world.resolve(), Type17ContactOutcome::Applied(_)));
        assert_eq!(
            world.entity().actor_task_state(ActorTaskSlot::Primary),
            Some(&expected)
        );
        assert_eq!(world.entity().physical_body_basis_q31(), basis);
        assert_eq!(
            world.fx.next_shared_retail_random_u16(),
            expected_fx.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn native_fence_damage_publishes_unvisited_class12_and_late_failure_parks_custody() {
    for fail_profile in [false, true] {
        let Some(mut world) = Fixture::load(BirthStage::Following) else {
            return;
        };
        let (position, contact) = world.find_contact([-24961, 3, -26881], [157, 151], 532);
        let velocity = contact
            .normal_q12
            .map(|axis| (-i32::from(axis) * 2400 / 4096) as i16);
        let entity = world.entities.entity_mut(world.id).unwrap();
        entity.set_motion_raw(position, velocity);
        entity.collision.health_raw = RetailRuntimeValue::Known(1);
        if fail_profile {
            entity.collision.damage_profile = RetailRuntimeValue::Unresolved;
        }
        let result = world.resolve();
        if fail_profile {
            assert!(
                matches!(
                    result,
                    Type17ContactOutcome::Blocked {
                        reason: Type17ContactBlock::Damage(_),
                        committed_prefix: true
                    }
                ),
                "{result:?}"
            );
            assert!(world.scheduler.intro2_type17_has_pending_prefix(world.id));
            world
                .entities
                .entity_mut(world.id)
                .unwrap()
                .set_motion_raw(position, velocity);
            let mut expected_fx = world.fx.fork_for_main_base_abort_transaction();
            assert!(matches!(
                world.resolve(),
                Type17ContactOutcome::Blocked {
                    reason: Type17ContactBlock::Runtime("completed contact owner"),
                    committed_prefix: false
                }
            ));
            assert_eq!(
                world.fx.next_shared_retail_random_u16(),
                expected_fx.next_shared_retail_random_u16()
            );
        } else {
            let Type17ContactOutcome::Applied(applied) = result else {
                panic!("{result:?}")
            };
            assert!(applied.impact_raw > 2000);
            assert!(applied.static_damage.is_some());
            assert!(applied.actor_damage.unwrap().death_publication.is_some());
            let Some(ActorTaskRuntime::CommonDying(task)) =
                world.entity().actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("Class12 owns new Primary")
            };
            assert_eq!(
                task.elapsed_ms(),
                0,
                "late contact cannot visit new class12 in the completed actor pass"
            );
            assert!(world
                .scheduler
                .prepare_native_actor_mutation(&world.entities, world.id));
        }
    }
}
