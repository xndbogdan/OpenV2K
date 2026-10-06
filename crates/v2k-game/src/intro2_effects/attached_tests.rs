use super::*;
use crate::{
    intro2_type47_live::world::native_intro2_fixture,
    playing_particle_host::{
        PlayingParticleActorEvent, PlayingParticleActorResponse, PlayingParticleHost,
    },
    world_fx::{
        begin_attached_particle_owner_update, DescriptorParticleRequest, ParticleAttachmentOwner,
        ParticleOwnerAtBirth,
    },
};

fn run_class84(playing: bool, living: bool) {
    let Some((mut session, mut entities, _)) = native_intro2_fixture() else {
        return;
    };
    let target = entities
        .iter_all()
        .find(|entity| entity.entity_type == 9)
        .unwrap();
    let id = target.id;
    let position_raw = target.position_raw();
    let source_id = entities
        .iter_all()
        .find(|entity| entity.entity_type == 57)
        .unwrap()
        .id;
    if living {
        entities.entity_mut(id).unwrap().collision.health_raw = RetailRuntimeValue::Known(50_000);
    }
    // Controlled source-backed prior actor tail, not an invented birth B2.
    // 12DA0 clears this field before the later particle pass. Its constructor
    // deliberately leaves allocator bytes unresolved until that first tail.
    crate::entity_scheduler::commit_common_scheduler_post_callback(
        &mut entities.entity_mut(id).unwrap().collision,
    );
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(scheduler.adopt_intro2_type9(&mut entities), 13);
    let mut fx = WorldFx::new();
    fx.materialize_descriptor_particle_request(
        DescriptorParticleRequest {
            source_class: 68,
            position_raw,
            velocity_raw: [700, -200, 900],
            owner: Some(ParticleOwnerAtBirth {
                entity_id: source_id,
                entity_type: 57,
            }),
            suppresses_impact_damage: false,
        },
        ParticleEnvironment::Dry,
        0,
    )
    .unwrap();
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let first = update_intro2_effects(Intro2EffectsFrame {
        cache: &mut session.cache,
        entities: &mut entities,
        world_fx: &mut fx,
        static_damage: &mut static_damage,
        scheduler: &mut scheduler,
        notifications: &mut notifications,
        elapsed_micros: 0,
        retail_tick: 25,
    });
    assert!(
        first.particles.blocked_attached_updates.is_empty(),
        "{first:?}"
    );
    assert!(
        first.entity_deliveries.iter().any(|delivery| {
            delivery.impact.target_entity_id == id
                && matches!(
                    delivery.steps.as_slice(),
                    [Intro2EntityImpactStep::NativeType9Damage(
                        crate::ordinary_type9_impact::NativeType9ImpactOutcome::Applied(_)
                    )]
                )
        }),
        "{first:?}"
    );
    let RetailRuntimeValue::Known(health_before_attached) =
        entities.entity_mut(id).unwrap().collision.health_raw
    else {
        panic!("health")
    };
    assert_eq!(health_before_attached > 0, living);
    let attached: Vec<_> = fx
        .test_particles_in_virgin_birth_order()
        .into_iter()
        .filter(|p| p.source_class == 84)
        .collect();
    assert_eq!(attached.len(), 1);
    assert_eq!(
        attached[0].age_ticks, 0.0,
        "earlier physical birth waits for the next visit"
    );
    assert_eq!(attached[0].attached_owner_handle, Some(id));
    assert_eq!(
        entities
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .masked(0x8000),
        RetailRuntimeValue::Known(if living { 0x8000 } else { 0 })
    );

    let before_stamp = entities
        .entity_mut(id)
        .unwrap()
        .collision
        .last_hit_presentation_tick_at_0x34;
    let mut player_hull = crate::player_hull::PlayerHull::default();
    let second = if playing {
        let projection = intro2_particle_collision_models(&session.cache, &entities, 26);
        let attachment_owners = entities
            .iter_all()
            .map(ParticleAttachmentOwner::from_entity)
            .collect();
        let owner_motions = entities
            .iter_all()
            .map(|entity| ParticleOwnerMotion {
                owner_id: entity.id,
                velocity: entity.velocity,
            })
            .collect();
        fx.update_with_traversal_host(
            ParticleTraversalTiming {
                elapsed_micros: 20_000,
                retail_tick: 26,
            },
            &mut PlayingParticleHost {
                cache: &mut session.cache,
                static_damage: &mut static_damage,
                owner_motions,
                collision_models: projection.models,
                attachment_owners,
                on_actor_event: |cache: &mut ResourceCache,
                                 static_damage: &mut StaticDamageScheduler,
                                 world_fx: &mut WorldFx,
                                 event| {
                    match event {
                        PlayingParticleActorEvent::AttachedUpdate(request) => {
                            PlayingParticleActorResponse::AttachedUpdate(
                                begin_attached_particle_owner_update(&mut entities, request),
                            )
                        }
                        PlayingParticleActorEvent::Impact(_) => {
                            panic!("the isolated postdeath attachment has no model contact")
                        }
                        PlayingParticleActorEvent::AttachedDamage(request) => {
                            assert!(living);
                            let result = crate::attached_particle_damage::apply_attached_particle_damage(
                                crate::attached_particle_damage::AttachedParticleDamageFrame {
                                    resources: cache, entities: &mut entities, world_fx,
                                    static_damage, scheduler: &mut scheduler,
                                    notifications: &mut notifications, retail_tick: 26,
                                    world: crate::attached_particle_damage::AttachedParticleDamageWorld::Playing { player_hull: &mut player_hull, extra_lives: crate::entity_collision_state::RetailRuntimeValue::Known(0) },
                                }, request);
                            PlayingParticleActorResponse::AttachedDamage {
                                result,
                                refresh: crate::playing_particle_host::PlayingParticleRefresh {
                                    collision: crate::world_fx::ParticleCollisionCacheRefresh::Replace(
                                        intro2_particle_collision_models(cache, &entities, 26).models),
                                    owner_motions: entities.iter_all().map(|entity| ParticleOwnerMotion {
                                        owner_id: entity.id, velocity: entity.velocity,
                                    }).collect(),
                                    attachment_owners: entities.iter_all().map(ParticleAttachmentOwner::from_entity).collect(),
                                },
                            }
                        }
                        PlayingParticleActorEvent::AttachedCascadeOwner(request) => {
                            assert!(living);
                            PlayingParticleActorResponse::AttachedCascadeOwner(
                                crate::world_fx::sample_attached_particle_cached_owner(&entities, request))
                        }
                    }
                },
                on_terrain_event: |_: &mut StaticDamageScheduler, _: &mut WorldFx, _| {
                    crate::world_fx::ParticleTerrainResponse::Unhandled
                },
            },
        )
    } else {
        update_intro2_effects(Intro2EffectsFrame {
            cache: &mut session.cache,
            entities: &mut entities,
            world_fx: &mut fx,
            static_damage: &mut static_damage,
            scheduler: &mut scheduler,
            notifications: &mut notifications,
            elapsed_micros: 20_000,
            retail_tick: 26,
        })
        .particles
    };
    assert!(second.blocked_attached_updates.is_empty(), "{second:?}");
    let [observed] = second.attached_updates.as_slice() else {
        panic!("{second:?}")
    };
    assert_eq!(observed.particle_class, 84);
    assert_eq!(observed.mass_increment_raw, 63);
    assert_eq!(observed.damage_request.target_handle, id);
    assert_eq!(
        (
            observed.damage_request.ratio_numerator,
            observed.damage_request.ratio_denominator
        ),
        (1, 255)
    );
    let corpse = entities.entity_mut(id).unwrap();
    assert_eq!(
        corpse.collision.health_raw,
        RetailRuntimeValue::Known(health_before_attached - observed.filtered_damage_raw)
    );
    assert_eq!(observed.filtered_damage_raw > 0, living);
    assert_eq!(
        corpse.collision.last_hit_presentation_tick_at_0x34, before_stamp,
        "direct15040 never replays11180/10EB0 presentation stamp"
    );
    assert_eq!(
        corpse.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(63)
    );
    assert_eq!(
        crate::entity_scheduler::common_scheduler_callback_mass(
            corpse.mass_raw,
            corpse.collision.animation_offset_at_0xb2
        ),
        RetailRuntimeValue::Known(corpse.mass_raw.wrapping_add(63))
    );
    // The next actual common actor tail owns the clear; the particle adapter
    // must not clear its contribution immediately after publishing it.
    crate::entity_scheduler::commit_common_scheduler_post_callback(&mut corpse.collision);
    assert_eq!(
        corpse.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(0)
    );
}

#[v2k_test_support::retail_test]
fn intro2_class68_actual_hit_runs_complete_postdeath_class84_callback() {
    run_class84(false, false);
}

#[v2k_test_support::retail_test]
fn playing_host_mutates_real_b2_for_actual_class68_postdeath_attachment() {
    run_class84(true, false);
}

#[v2k_test_support::retail_test]
fn intro2_actual_class68_hit_then_enabled_class84_runs_complete_direct_damage() {
    run_class84(false, true);
}

#[v2k_test_support::retail_test]
fn playing_actual_class68_hit_then_enabled_class84_runs_complete_direct_damage() {
    run_class84(true, true);
}
