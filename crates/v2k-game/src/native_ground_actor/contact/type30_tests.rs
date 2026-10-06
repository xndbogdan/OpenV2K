//! Actual Type30 lease and Class26 C890 phase; geometry uses the shared host.
use super::*;
use crate::{
    entity_behavior::{behavior_program, BehaviorContextRuntime, BehaviorSelection},
    gameplay_notifications::GameplayNotifications,
    native_type30::{self, tests::native_actor_fixture},
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
    world_fx::WorldFx,
};

#[v2k_test_support::retail_test]
fn native_type30_class5_search7_and_furniture26_static_hooks_keep_own_graph() {
    for class in [5, 7, 26] {
        let (mut session, metadata, mut manager, id) = native_actor_fixture();
        let entity = manager.entity_mut(id).unwrap();
        let RetailRuntimeValue::Known(Some(birth)) = entity.initial_behavior else {
            panic!("birth")
        };
        let selection = BehaviorSelection {
            program: behavior_program(class).unwrap(),
            choice_index: match class {
                5 => 0,
                7 => 1,
                26 => 2,
                _ => unreachable!(),
            },
            ..birth
        };
        assert!(<native_type30::profile::Type30Profile as super::super::sealed::Sealed>::publish_acquiring(
            entity,&metadata[30],selection,BehaviorContextRuntime::from_fresh_weighted_selection(selection).unwrap(),&mut ||0));
        let hook = contact_task_hook(entity).unwrap();
        assert!(matches!(
            (class, hook),
            (26, NativeGroundStaticTaskHook::Furniture)
                | (5 | 7, NativeGroundStaticTaskHook::WanderPrivate(_))
        ));
        if class != 26 {
            continue;
        }
        let bank_before = entity.native_type30_runtime.clone();
        let primary_before = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
        let others: Vec<_> = manager
            .iter_all()
            .filter(|entity| entity.id != id)
            .map(|entity| entity.id)
            .collect();
        for other in others {
            manager.entity_mut(other).unwrap().active = false;
        }
        let mut fx = WorldFx::new();
        let mut control = WorldFx::new();
        control.next_shared_retail_random_u16();
        control.next_shared_retail_random_u16();
        let mut tasks = SpecializedActorTaskScheduler::new();
        tasks.register_type30(native_type30::Type30Owner::adopt(&manager, id).unwrap());
        reselect_after_furniture_contact(
            &mut Intro2ContactFrame {
                entities: &mut manager,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut StaticDamageScheduler::default(),
                notifications: &mut GameplayNotifications::new(),
                retail_tick: 251,
                actor_tasks: &mut tasks,
            },
            id,
        )
        .unwrap();
        let entity = manager.entity_mut(id).unwrap();
        assert_ne!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            primary_before
        );
        assert_eq!(entity.native_type30_runtime, bank_before);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            control.next_shared_retail_random_u16()
        );
        assert!(
            tasks.prepare_native_actor_mutation(&manager, id),
            "C890 registers actual replacement before11760"
        );
    }
}
