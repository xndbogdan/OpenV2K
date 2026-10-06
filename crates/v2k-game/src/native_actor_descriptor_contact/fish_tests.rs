use super::*;
use crate::{
    entity::{AuthoredWorldConstruction, EntityConstructionResources},
    entity_behavior::{BehaviorContextRuntime, BehaviorDescriptorIdentity},
    gameplay_notifications::GameplayNotifications,
    session::GameSession,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
    world_fx::WorldFx,
};

struct FishFixture {
    session: GameSession,
    entities: EntityManager,
    fx: WorldFx,
    scheduler: SpecializedActorTaskScheduler,
    damage: StaticDamageScheduler,
    notifications: GameplayNotifications,
}

impl FishFixture {
    fn native(world: u32) -> Self {
        let data = v2k_test_support::retail_dir();
        assert!(data.join("PRELOAD.DAT").is_file(), "retail data required");
        let mut session = GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(world, 1).unwrap();
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
        let entities = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                logical_world_index: world as i32 - 12,
                level: session.cache.level_desc().unwrap(),
                type_metadata: &metadata,
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| session.cache.global_model(id).map(|m| m.radius)),
                },
                player_arrival: None,
                retail_tick: 321,
            },
            &mut fx,
        )
        .unwrap();
        Self {
            session,
            entities,
            fx,
            scheduler: SpecializedActorTaskScheduler::new(),
            damage: StaticDamageScheduler::new(),
            notifications: GameplayNotifications::new(),
        }
    }

    fn invoke(
        &mut self,
        id: u32,
        opposite: u32,
    ) -> Result<NativeDescriptorContactOutcome, NativeDescriptorContactBlock> {
        resolve_native_actor_descriptor_contact(
            &mut Intro2ContactFrame {
                entities: &mut self.entities,
                resources: &mut self.session.cache,
                world_fx: &mut self.fx,
                static_damage: &mut self.damage,
                notifications: &mut self.notifications,
                retail_tick: 321,
                actor_tasks: &mut self.scheduler,
            },
            id,
            opposite,
            ActorTaskSlot::Primary,
        )
    }

    fn next_word(&self) -> u16 {
        self.fx
            .fork_for_main_base_abort_transaction()
            .next_shared_retail_random_u16()
    }
}

#[v2k_test_support::retail_test]
fn native_fish_contact_reverses_d_and_f_without_a_speed_rng_or_animation_ticks() {
    for world in [23, 30] {
        // The constructor's authored choices produce class5 retarget or
        // class6 Wander. The fourth case reaches AF50 through actual class13
        // acquisition on two native same-species allocations.
        for (kind, selection_word, route) in [
            (22, 65535, false),
            (24, 65535, false),
            (124, 65535, false),
            (22, 0, true),
        ] {
            let mut f = FishFixture::native(world);
            let id = f
                .entities
                .iter_all()
                .find(|e| e.entity_type == kind)
                .unwrap()
                .id;
            let opposite = f
                .entities
                .iter_all()
                .find(|e| e.id != id && e.entity_type == 22)
                .unwrap()
                .id;
            let metadata = f.entities.type_runtime_metadata(kind).unwrap().clone();
            let selection =
                crate::shared_fish_tasks::select_behavior(&metadata, &mut || selection_word)
                    .unwrap();
            let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection).unwrap();
            crate::shared_fish_tasks::publish_selection(
                f.entities.entity_mut(id).unwrap(),
                &metadata,
                selection,
                context,
            )
            .unwrap();
            if route {
                let ids: Vec<_> = f
                    .entities
                    .iter_all()
                    .filter(|e| e.entity_type == kind)
                    .map(|e| e.id)
                    .collect();
                for other in ids {
                    f.entities
                        .entity_mut(other)
                        .unwrap()
                        .set_position_raw([20000, 0, 20000]);
                }
                f.entities
                    .entity_mut(id)
                    .unwrap()
                    .set_position_raw([100, 0, 100]);
                f.entities
                    .entity_mut(opposite)
                    .unwrap()
                    .set_position_raw([101, 0, 100]);
                crate::shared_fish_tasks::acquire(&mut f.entities, id, &mut || 0).unwrap();
                assert!(matches!(
                    f.entities
                        .entity_mut(id)
                        .unwrap()
                        .actor_task_state(ActorTaskSlot::Primary),
                    Some(ActorTaskRuntime::FishTargetRoute(_))
                ));
            }
            f.scheduler.adopt_shared_fish(&f.entities);
            let position = actor(&f.entities, id).unwrap().position_raw();
            f.entities
                .entity_mut(opposite)
                .unwrap()
                .set_position_raw(position);
            let before = actor(&f.entities, id).unwrap();
            let task_id = before
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .unwrap();
            let task = *before.actor_tasks.task_state(task_id).unwrap();
            let [heading, pitch, roll] = before.rotation_heading_pitch_roll_raw();
            let basis = before.physical_body_basis_q31();
            let velocity = before.velocity_raw();
            let before_context = before.current_behavior_context;
            assert!(
                matches!(before_context, RetailRuntimeValue::Known(Some(context))
                if matches!(context.descriptor(),BehaviorDescriptorIdentity::Named(_)))
            );
            let mut expected_runtime = before.shared_fish_runtime.clone().unwrap();
            let mut expected_fx = f.fx.fork_for_main_base_abort_transaction();
            let expected =
                crate::common_mover::target_prelude::plan_common_mover_descriptor_effect(
                    crate::common_mover::target_prelude::CommonMoverDescriptorEffectRequest {
                        target_state: private_state(&task).unwrap(),
                        controlled_position_raw: position,
                        heading_raw: heading as u16,
                        roll_raw: roll as u16,
                        topology: contact_topology(before, &metadata).unwrap(),
                    },
                    || u32::from(expected_fx.next_shared_retail_random_u16()),
                )
                .unwrap();
            assert_eq!(
                expected.rng_draw_count, 2,
                "B/D/F consumes only X/Z retarget draws"
            );
            assert_eq!(expected.sub_a_runtime, None);
            expected_runtime.sub_d_runtime.last_yaw_step_raw =
                expected.sub_d_reversal_write.unwrap().step_raw as i16;
            expected_runtime.sub_f.reversal_raw = u8::from(expected.sub_f_reverse_write.unwrap());
            assert_eq!(
                f.invoke(id, opposite),
                Ok(NativeDescriptorContactOutcome::Applied { rng_draws: 2 })
            );
            let after = actor(&f.entities, id).unwrap();
            let mut expected_task = task;
            commit_private(&mut expected_task, expected.target_state);
            assert_eq!(after.actor_tasks.task_state(task_id), Some(&expected_task));
            assert_eq!(
                after.shared_fish_runtime.as_ref(),
                Some(&expected_runtime),
                "contact changes only D's last step and F's reversal byte"
            );
            assert_eq!(after.current_behavior_context, before_context);
            assert_eq!(after.position_raw(), position);
            assert_eq!(after.velocity_raw(), velocity);
            assert_eq!(after.physical_body_basis_q31(), basis);
            assert_eq!(
                after.rotation_heading_pitch_roll_raw(),
                [expected.heading_raw as i16, pitch, expected.roll_raw as i16]
            );
            assert_eq!(
                after.sub_a_propulsion_runtime,
                RetailRuntimeValue::Known(None)
            );
            assert_eq!(f.next_word(), expected_fx.next_shared_retail_random_u16());
            assert!(f.scheduler.prepare_native_actor_mutation(&f.entities, id));
        }
    }
}

#[v2k_test_support::retail_test]
fn fish_contact_preserves_negative_gate_and_rejects_unresolved_or_parked_owner_before_rng() {
    let mut f = FishFixture::native(30);
    f.scheduler.adopt_shared_fish(&f.entities);
    let id = f
        .entities
        .iter_all()
        .find(|e| e.entity_type == 22)
        .unwrap()
        .id;
    let opposite = f
        .entities
        .iter_all()
        .find(|e| e.entity_type == 22 && e.id != id)
        .unwrap()
        .id;
    let before = actor(&f.entities, id).unwrap();
    let RetailRuntimeValue::Known(basis) = before.physical_body_basis_q31() else {
        panic!()
    };
    let axis = (0..3)
        .max_by_key(|&axis| i64::from(basis.forward[axis]).abs())
        .unwrap();
    assert_ne!(basis.forward[axis], 0);
    let position = before.position_raw();
    let mut behind = position;
    behind[axis] = behind[axis].wrapping_sub(if basis.forward[axis] > 0 { 64 } else { -64 });
    let runtime = before.shared_fish_runtime.clone();
    f.entities
        .entity_mut(opposite)
        .unwrap()
        .set_position_raw(behind);
    f.entities.entity_mut(id).unwrap().sub_a_propulsion_runtime = RetailRuntimeValue::Unresolved;
    let rng = f.next_word();
    assert_eq!(
        f.invoke(id, opposite),
        Ok(NativeDescriptorContactOutcome::Behind)
    );
    f.entities
        .entity_mut(opposite)
        .unwrap()
        .set_position_raw(position);
    assert_eq!(
        f.invoke(id, opposite),
        Err(NativeDescriptorContactBlock {
            reason: NativeDescriptorContactError::Runtime("fish descriptor Sub-A absence"),
            committed_prefix: false,
        })
    );
    assert_eq!(actor(&f.entities, id).unwrap().shared_fish_runtime, runtime);
    assert_eq!(f.next_word(), rng);
    f.entities.entity_mut(id).unwrap().sub_a_propulsion_runtime = RetailRuntimeValue::Known(None);
    assert!(f.scheduler.park_native_contact_prefix(&f.entities, id));
    assert_eq!(
        f.invoke(id, opposite),
        Err(NativeDescriptorContactBlock {
            reason: NativeDescriptorContactError::Runtime("completed descriptor owner"),
            committed_prefix: false,
        })
    );
    assert_eq!(actor(&f.entities, id).unwrap().shared_fish_runtime, runtime);
    assert_eq!(f.next_word(), rng);
}
