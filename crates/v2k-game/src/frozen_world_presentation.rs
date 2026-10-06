//! Disposable draw owners for a frozen gameplay scene.
//!
//! World presentation writes actor caches, consumes shared RNG, allocates
//! particles and performs destructive particle culling. A paused redraw must
//! therefore retain its own admission snapshot and discard every draw's writes.
//! These owners deliberately do not implement `Clone` or expose a commit path.

use crate::{
    entity::EntityManager, specialized_actor_task_production::SpecializedActorTaskScheduler,
    world_fx::WorldFx,
};

/// The paired live owners read once when entering pause.
pub struct FrozenWorldPresentationSource<'a> {
    pub entities: &'a EntityManager,
    pub world_fx: &'a WorldFx,
    pub specialized_actor_tasks: &'a SpecializedActorTaskScheduler,
}

/// Temporary mutable owners available only during one redraw callback.
pub struct FrozenWorldPresentationFrame<'a> {
    pub entities: &'a mut EntityManager,
    pub world_fx: &'a mut WorldFx,
    pub specialized_actor_tasks: &'a mut SpecializedActorTaskScheduler,
}

/// A retained pause-entry snapshot, independent of subsequent menu RNG/audio.
///
/// Capture this once per pause and drop it on resume or world teardown. Each
/// redraw starts from the same paired allocation/task identities and effects.
/// The caller separately freezes presentation clocks, shield/targetter state
/// and HUD data, supplies zero elapsed time, and omits simulation/audio flushes.
pub struct FrozenWorldPresentationSnapshot {
    entities: EntityManager,
    world_fx: WorldFx,
    specialized_actor_tasks: SpecializedActorTaskScheduler,
}

impl FrozenWorldPresentationSnapshot {
    pub fn capture(source: FrozenWorldPresentationSource<'_>) -> Self {
        // Reuse the complete private deep-copy machinery originally owned by
        // Main Base rollback. Unlike that transaction, presentation never
        // swaps these paired owners back into the live world.
        Self {
            entities: source.entities.fork_for_main_base_abort_transaction(),
            world_fx: source.world_fx.fork_for_main_base_abort_transaction(),
            specialized_actor_tasks: source
                .specialized_actor_tasks
                .fork_for_main_base_abort_transaction(),
        }
    }

    /// Invoke the draw callback exactly once, then discard its mutable owners.
    ///
    /// The unit result and borrowed frame retain no replacement/commit API.
    /// Draw-time caches, particle mutations, queued sounds and RNG draws affect
    /// neither the live source nor this retained snapshot, even after unwinding.
    pub fn redraw(&self, draw: impl FnOnce(FrozenWorldPresentationFrame<'_>)) {
        let mut entities = self.entities.fork_for_main_base_abort_transaction();
        let mut world_fx = self.world_fx.fork_for_main_base_abort_transaction();
        let mut specialized_actor_tasks = self
            .specialized_actor_tasks
            .fork_for_main_base_abort_transaction();
        draw(FrozenWorldPresentationFrame {
            entities: &mut entities,
            world_fx: &mut world_fx,
            specialized_actor_tasks: &mut specialized_actor_tasks,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        entity::{AuthoredWorldConstruction, EntityConstructionResources},
        entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
        session::GameSession,
    };

    fn native_world() -> (EntityManager, WorldFx, SpecializedActorTaskScheduler) {
        let root = v2k_test_support::retail_dir();
        let mut session = GameSession::init(&root).expect("retail fixture data");
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(13, 1).unwrap();
        let metadata: Vec<_> = session
            .cache
            .global_entity_model_table()
            .iter()
            .enumerate()
            .map(|(id, model_slots)| {
                session
                    .cache
                    .global_entity_type(id)
                    .map(EntityTypeRuntimeMetadata::from_section12)
                    .unwrap_or(EntityTypeRuntimeMetadata {
                        model_slots: *model_slots,
                        ..EntityTypeRuntimeMetadata::default()
                    })
            })
            .collect();
        let extent = |id| session.cache.global_model(id).map(|model| model.radius);
        let mut fx = WorldFx::new();
        let entities = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                level: session.cache.level_desc().unwrap(),
                logical_world_index: 1,
                type_metadata: &metadata,
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&extent),
                },
                player_arrival: None,
                retail_tick: 0,
            },
            &mut fx,
        )
        .unwrap();
        let mut scheduler = SpecializedActorTaskScheduler::default();
        assert!(scheduler.adopt_class0_actors(&entities) > 0);
        fx.emit_alien_hive_particle_raw([100, 200, 300], 0);
        fx.queue_fixed_positional_sound_raw(99, [100, 200, 300]);
        (entities, fx, scheduler)
    }

    fn snapshot(
        entities: &EntityManager,
        world_fx: &WorldFx,
        specialized_actor_tasks: &SpecializedActorTaskScheduler,
    ) -> FrozenWorldPresentationSnapshot {
        FrozenWorldPresentationSnapshot::capture(FrozenWorldPresentationSource {
            entities,
            world_fx,
            specialized_actor_tasks,
        })
    }

    #[v2k_test_support::retail_test]
    fn redraw_discards_entity_presentation_effects_rng_and_scheduler_writes() {
        let (entities, mut fx, scheduler) = native_world();
        let frozen = snapshot(&entities, &fx, &scheduler);
        let actor = entities.iter_all().next().unwrap();
        let id = actor.id;
        let position = actor.position_raw();
        let animation = actor.collision.animation_offset_at_0xb2;
        let owner_count = scheduler.registered_len();
        let particle_count = fx.particle_count();
        let pending_count = fx.pending_event_count();
        assert!(particle_count > 0 && pending_count > 0);
        let mut rng_oracle = fx.fork_for_main_base_abort_transaction();
        let expected_word = rng_oracle.next_shared_retail_random_u16();
        let mut calls = 0;

        frozen.redraw(|frame| {
            calls += 1;
            let actor = frame.entities.entity_mut(id).unwrap();
            actor.set_motion_raw([1, 2, 3], [4, 5, 6]);
            actor.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(123);
            for _ in 0..32 {
                frame.world_fx.next_shared_retail_random_u16();
            }
            frame.world_fx.clear();
            frame.specialized_actor_tasks.clear_after_manager_reset();
            assert_eq!(frame.world_fx.particle_count(), 0);
            assert_eq!(frame.specialized_actor_tasks.registered_len(), 0);
        });

        assert_eq!(calls, 1);
        let actor = entities.iter_all().find(|actor| actor.id == id).unwrap();
        assert_eq!(actor.position_raw(), position);
        assert_eq!(actor.collision.animation_offset_at_0xb2, animation);
        assert_eq!(scheduler.registered_len(), owner_count);
        assert_eq!(fx.particle_count(), particle_count);
        assert_eq!(fx.pending_event_count(), pending_count);
        assert_eq!(fx.next_shared_retail_random_u16(), expected_word);
    }

    #[v2k_test_support::retail_test]
    fn each_redraw_restarts_from_pause_entry_even_after_live_menu_changes() {
        let (mut entities, mut fx, mut scheduler) = native_world();
        let frozen = snapshot(&entities, &fx, &scheduler);
        let actor = entities.iter_all().next().unwrap();
        let id = actor.id;
        let position = actor.position_raw();
        let owner_count = scheduler.registered_len();
        let mut first_words = Vec::new();
        frozen.redraw(|frame| {
            first_words.extend((0..16).map(|_| frame.world_fx.next_shared_retail_random_u16()));
            frame
                .entities
                .entity_mut(id)
                .unwrap()
                .set_motion_raw([1; 3], [2; 3]);
            frame.world_fx.clear();
            frame.specialized_actor_tasks.clear_after_manager_reset();
        });
        // Menu sounds share the live process RNG. Neither they nor subsequent
        // source changes may alter another redraw of the retained world.
        for _ in 0..64 {
            fx.next_shared_retail_random_u16();
        }
        fx.clear();
        entities
            .entity_mut(id)
            .unwrap()
            .set_motion_raw([9; 3], [8; 3]);
        scheduler.clear_after_manager_reset();
        let mut next_words = Vec::new();
        frozen.redraw(|frame| {
            next_words.extend((0..16).map(|_| frame.world_fx.next_shared_retail_random_u16()));
            assert_eq!(
                frame.entities.entity_mut(id).unwrap().position_raw(),
                position
            );
            assert_eq!(frame.specialized_actor_tasks.registered_len(), owner_count);
            assert!(frame.world_fx.particle_count() > 0);
            assert!(frame.world_fx.pending_event_count() > 0);
        });
        assert_eq!(next_words, first_words);
    }

    #[v2k_test_support::retail_test]
    fn unwinding_redraw_does_not_damage_the_retained_or_live_world() {
        let (entities, fx, scheduler) = native_world();
        let frozen = snapshot(&entities, &fx, &scheduler);
        let owner_count = scheduler.registered_len();
        let particle_count = fx.particle_count();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            frozen.redraw(|frame| {
                frame.world_fx.clear();
                frame.specialized_actor_tasks.clear_after_manager_reset();
                panic!("aborted presentation");
            });
        }));
        assert!(result.is_err());
        assert_eq!(fx.particle_count(), particle_count);
        assert_eq!(scheduler.registered_len(), owner_count);
        frozen.redraw(|frame| {
            assert_eq!(frame.world_fx.particle_count(), particle_count);
            assert_eq!(frame.specialized_actor_tasks.registered_len(), owner_count);
        });
    }
}
