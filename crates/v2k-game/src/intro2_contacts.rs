//! The admitted parts of Intro2's late 11A80 walk, in one intrusive order.
//!
//! Meteor termination, family-owned surface/static contact and admitted
//! active pairs run after physical particles. Static and pair phases retain
//! their own native callbacks, geometry, response and mutation custody.

use crate::{
    entity::EntityManager,
    gameplay_notifications::GameplayNotifications,
    intro2_meteors::{resolve_intro2_meteor_terrain_contact, Intro2MeteorError, Intro2MeteorOwner},
    intro2_radial::{complete_intro2_meteor_death, Intro2MeteorDeathReport, Intro2RadialFrame},
    intro2_type58::contact::{resolve_intro2_type58_static_contact, Intro2Type58ContactOutcome},
    resource_cache::ResourceCache,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
    world_fx::WorldFx,
};

pub struct Intro2ContactFrame<'a> {
    pub entities: &'a mut EntityManager,
    pub resources: &'a mut ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub static_damage: &'a mut StaticDamageScheduler,
    pub notifications: &'a mut GameplayNotifications,
    pub retail_tick: u32,
    pub actor_tasks: &'a mut SpecializedActorTaskScheduler,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2ContactReport {
    Meteor {
        entity_id: u32,
        result: Result<Option<Intro2MeteorDeathReport>, Intro2MeteorError>,
    },
    Type58 {
        entity_id: u32,
        result: Intro2Type58ContactOutcome,
    },
    Flyer {
        entity_id: u32,
        result: crate::intro2_flyer_contacts::Intro2FlyerContactOutcome,
    },
    NativeFlyingSurface {
        entity_id: u32,
        entity_type: u32,
        result: crate::native_flying_surface_contact::NativeFlyingSurfaceContactOutcome,
    },
    Type10 {
        entity_id: u32,
        result: crate::intro2_type10::contact::Intro2Type10ContactOutcome,
    },
    Type57 {
        entity_id: u32,
        result: crate::intro2_type57::contact::Intro2Type57ContactOutcome,
    },
    Type17 {
        entity_id: u32,
        result: crate::intro2_type17::contact::Type17ContactOutcome,
    },
    NativeSurface {
        entity_id: u32,
        entity_type: u32,
        result: crate::native_actor_surface_contact::NativeActorSurfaceContactOutcome,
    },
    Type17Pair {
        entity_id: u32,
        result: crate::intro2_type17::pair::Type17PairOutcome,
    },
    Type47 {
        entity_id: u32,
        result: crate::type47_static_contact::Type47StaticContactOutcome,
    },
    Type53 {
        entity_id: u32,
        result: crate::intro2_type53::contact::Type53ContactOutcome,
    },
    Type122 {
        entity_id: u32,
        result: crate::native_type122::contact::Type122ContactOutcome,
    },
    Type30 {
        entity_id: u32,
        result: crate::native_type30::contact::Type30ContactOutcome,
    },
    Type40 {
        entity_id: u32,
        result: crate::native_type40::contact::Type40ContactOutcome,
    },
    Type56 {
        entity_id: u32,
        result: crate::native_type56::contact::Type56ContactOutcome,
    },
    InsectStatic {
        entity_id: u32,
        entity_type: u32,
        result: crate::native_ground_actor::contact::NativeGroundContactOutcome,
    },
}

pub fn resolve_intro2_contacts(mut frame: Intro2ContactFrame<'_>) -> Vec<Intro2ContactReport> {
    let ids: Vec<_> = frame
        .entities
        .iter_all()
        .filter(|entity| entity.active)
        .map(|entity| entity.id)
        .collect();
    let mut reports = Vec::new();
    for id in ids {
        // Re-read at the current cursor: an earlier meteor can kill or replace
        // a later actor before its own contact visit.
        let Some(entity) = frame
            .entities
            .iter_all()
            .find(|entity| entity.id == id && entity.active)
        else {
            continue;
        };
        if matches!(entity.entity_type, 13 | 10 | 57) {
            let entity_type = entity.entity_type;
            let entry_model_id = match entity.collision.state_flags_at_0x08.masked(0x6000) {
                crate::entity_collision_state::RetailRuntimeValue::Known(bits) => entity
                    .model_in_slot(
                        crate::entity_collision_state::active_model_slot_from_state_flags(bits),
                    ),
                crate::entity_collision_state::RetailRuntimeValue::Unresolved => None,
            };
            let result =
                crate::native_flying_surface_contact::resolve_native_flying_surface_contact(
                    &mut frame, id,
                );
            let ineligible = matches!(
                result,
                crate::native_flying_surface_contact::NativeFlyingSurfaceContactOutcome::Ineligible
            );
            let blocked = matches!(
                result,
                crate::native_flying_surface_contact::NativeFlyingSurfaceContactOutcome::Blocked { .. }
            );
            reports.push(Intro2ContactReport::NativeFlyingSurface {
                entity_id: id,
                entity_type,
                result,
            });
            if blocked {
                continue;
            }
            if !ineligible {
                // Keep the source entry model across damage/death callbacks.
                // Class11 continues only static, without replaying the completed
                // terrain/water walk. Its completion style has null hooks.
                let tumble = frame.entities.iter_all().find(|entity| entity.id == id)
                    .is_some_and(|entity| matches!(entity.current_behavior_context,
                        crate::entity_collision_state::RetailRuntimeValue::Known(Some(context))
                            if matches!(context.active_style().style_address(), 0x4c7f60 | 0x4c7fa8)));
                let mut static_blocked = false;
                if tumble {
                    let entry_model_id = entry_model_id.expect("admitted surface entry model");
                    if entity_type == 10 {
                        let result = crate::intro2_type10::contact::resolve_intro2_type10_tumble_static_continuation(
                            &mut frame, id, entry_model_id);
                        static_blocked |= matches!(
                            result,
                            crate::intro2_type10::contact::Intro2Type10ContactOutcome::Blocked { .. }
                        );
                        reports.push(Intro2ContactReport::Type10 {
                            entity_id: id,
                            result,
                        });
                    } else if entity_type == 57 {
                        let result = crate::intro2_type57::contact::resolve_intro2_type57_tumble_static_continuation(
                            &mut frame, id, entry_model_id);
                        static_blocked |= matches!(
                            result,
                            crate::intro2_type57::contact::Intro2Type57ContactOutcome::Blocked { .. }
                        );
                        reports.push(Intro2ContactReport::Type57 {
                            entity_id: id,
                            result,
                        });
                    }
                } else {
                    let result = crate::native_ground_actor::contact::resolve_flying_static_contact_continuation(
                        &mut frame, id, entry_model_id.expect("admitted surface entry model"));
                    static_blocked |= matches!(
                        result,
                        crate::native_ground_actor::contact::NativeGroundContactOutcome::Blocked { .. }
                    );
                    reports.push(Intro2ContactReport::InsectStatic {
                        entity_id: id,
                        entity_type,
                        result,
                    });
                }
                if static_blocked {
                    continue;
                }
                let result = crate::intro2_type17::pair::resolve_type17_active_contacts(
                    &mut frame,
                    id,
                    crate::intro2_type17::pair::CaptureFeedbackPolicy::Cinematic,
                );
                reports.push(Intro2ContactReport::Type17Pair {
                    entity_id: id,
                    result,
                });
                continue;
            }
        }
        let Some(entity) = frame
            .entities
            .iter_all()
            .find(|entity| entity.id == id && entity.active)
        else {
            continue;
        };
        if let Ok(owner) = Intro2MeteorOwner::adopt_published(entity) {
            let result = resolve_intro2_meteor_terrain_contact(
                frame.entities,
                id,
                frame.resources,
                frame.world_fx,
            )
            .map(|receipt| {
                receipt.map(|receipt| {
                    complete_intro2_meteor_death(
                        Intro2RadialFrame {
                            active_terminal_calls: Vec::new(),
                            entities: frame.entities,
                            resources: frame.resources,
                            world_fx: frame.world_fx,
                            static_damage: frame.static_damage,
                            notifications: frame.notifications,
                            retail_tick: frame.retail_tick,
                            actor_tasks: frame.actor_tasks,
                        },
                        receipt,
                    )
                })
            });
            if matches!(
                result,
                Ok(Some(Intro2MeteorDeathReport::Applied {
                    finalized: true,
                    ..
                }))
            ) {
                frame.actor_tasks.retire_intro2_meteor(owner);
            }
            reports.push(Intro2ContactReport::Meteor {
                entity_id: id,
                result,
            });
        } else if matches!(entity.entity_type, 16 | 26) {
            let entity_type = entity.entity_type;
            let surface = crate::native_actor_surface_contact::resolve_native_actor_surface_contact(
                &mut frame, id,
            );
            let blocked = matches!(
                surface,
                crate::native_actor_surface_contact::NativeActorSurfaceContactOutcome::Blocked { .. }
            );
            reports.push(Intro2ContactReport::NativeSurface {
                entity_id: id,
                entity_type,
                result: surface,
            });
            if blocked {
                continue;
            }
            let result =
                crate::native_ground_actor::contact::resolve_insect_static_contact(&mut frame, id);
            let blocked = matches!(
                result,
                crate::native_ground_actor::contact::NativeGroundContactOutcome::Blocked { .. }
            );
            reports.push(Intro2ContactReport::InsectStatic {
                entity_id: id,
                entity_type,
                result,
            });
            if blocked {
                continue;
            }
        } else if crate::intro2_type94::intro2_type94_allocation_authenticates(entity) {
            // Type94's owned world phase already preserves water Sub-C and H.
            // Late static contact consumes neither; do not substitute a flyer
            // or ground surface kernel for its own six-foot water actor.
            let result =
                crate::native_ground_actor::contact::resolve_insect_static_contact(&mut frame, id);
            let blocked = matches!(
                result,
                crate::native_ground_actor::contact::NativeGroundContactOutcome::Blocked { .. }
            );
            reports.push(Intro2ContactReport::InsectStatic {
                entity_id: id,
                entity_type: 94,
                result,
            });
            if blocked {
                continue;
            }
        } else if entity.entity_type == 13 {
            let result =
                crate::native_ground_actor::contact::resolve_flying_static_contact(&mut frame, id);
            reports.push(Intro2ContactReport::InsectStatic {
                entity_id: id,
                entity_type: 13,
                result,
            });
        } else if entity.entity_type == 10 {
            let result =
                crate::intro2_type10::contact::resolve_intro2_type10_tumble_contact(&mut frame, id);
            let living = matches!(
                result,
                crate::intro2_type10::contact::Intro2Type10ContactOutcome::Ineligible
            );
            reports.push(Intro2ContactReport::Type10 {
                entity_id: id,
                result,
            });
            if living {
                let result = crate::native_ground_actor::contact::resolve_flying_static_contact(
                    &mut frame, id,
                );
                reports.push(Intro2ContactReport::InsectStatic {
                    entity_id: id,
                    entity_type: 10,
                    result,
                });
            }
        } else if entity.entity_type == 17 && entity.intro2_type17_runtime.is_some() {
            let entity_type = entity.entity_type;
            let result = crate::native_actor_surface_contact::resolve_native_actor_surface_contact(
                &mut frame, id,
            );
            let blocked = matches!(
                result,
                crate::native_actor_surface_contact::NativeActorSurfaceContactOutcome::Blocked { .. }
            );
            reports.push(Intro2ContactReport::NativeSurface {
                entity_id: id,
                entity_type,
                result,
            });
            if blocked {
                continue;
            }
            let result =
                crate::intro2_type17::contact::resolve_type17_static_contact(&mut frame, id);
            let blocked = matches!(
                result,
                crate::intro2_type17::contact::Type17ContactOutcome::Blocked { .. }
            );
            reports.push(Intro2ContactReport::Type17 {
                entity_id: id,
                result,
            });
            if blocked {
                continue;
            }
        } else if entity.entity_type == 57 {
            let result =
                crate::intro2_type57::contact::resolve_intro2_type57_tumble_contact(&mut frame, id);
            let living = matches!(
                result,
                crate::intro2_type57::contact::Intro2Type57ContactOutcome::Ineligible
            );
            reports.push(Intro2ContactReport::Type57 {
                entity_id: id,
                result,
            });
            if living {
                let result = crate::native_ground_actor::contact::resolve_flying_static_contact(
                    &mut frame, id,
                );
                reports.push(Intro2ContactReport::InsectStatic {
                    entity_id: id,
                    entity_type: 57,
                    result,
                });
            }
        } else if entity.entity_type == 58 {
            let entity_type = entity.entity_type;
            let result = crate::native_actor_surface_contact::resolve_native_actor_surface_contact(
                &mut frame, id,
            );
            let blocked = matches!(
                result,
                crate::native_actor_surface_contact::NativeActorSurfaceContactOutcome::Blocked { .. }
            );
            reports.push(Intro2ContactReport::NativeSurface {
                entity_id: id,
                entity_type,
                result,
            });
            if blocked {
                continue;
            }
            let result = resolve_intro2_type58_static_contact(&mut frame, id);
            let blocked = matches!(result, Intro2Type58ContactOutcome::Blocked { .. });
            reports.push(Intro2ContactReport::Type58 {
                entity_id: id,
                result,
            });
            if blocked {
                continue;
            }
        } else if entity.entity_type == 122 && entity.native_type122_runtime.is_some() {
            let entity_type = entity.entity_type;
            let result = crate::native_actor_surface_contact::resolve_native_actor_surface_contact(
                &mut frame, id,
            );
            let blocked = matches!(
                result,
                crate::native_actor_surface_contact::NativeActorSurfaceContactOutcome::Blocked { .. }
            );
            reports.push(Intro2ContactReport::NativeSurface {
                entity_id: id,
                entity_type,
                result,
            });
            if blocked {
                continue;
            }
            let result =
                crate::native_type122::contact::resolve_type122_static_contact(&mut frame, id);
            let blocked = matches!(
                result,
                crate::native_type122::contact::Type122ContactOutcome::Blocked { .. }
            );
            reports.push(Intro2ContactReport::Type122 {
                entity_id: id,
                result,
            });
            if blocked {
                continue;
            }
        } else if entity.entity_type == 30 && entity.native_type30_runtime.is_some() {
            let entity_type = entity.entity_type;
            let result = crate::native_actor_surface_contact::resolve_native_actor_surface_contact(
                &mut frame, id,
            );
            let blocked = matches!(
                result,
                crate::native_actor_surface_contact::NativeActorSurfaceContactOutcome::Blocked { .. }
            );
            reports.push(Intro2ContactReport::NativeSurface {
                entity_id: id,
                entity_type,
                result,
            });
            if blocked {
                continue;
            }
            let result =
                crate::native_type30::contact::resolve_type30_static_contact(&mut frame, id);
            let blocked = matches!(
                result,
                crate::native_type30::contact::Type30ContactOutcome::Blocked { .. }
            );
            reports.push(Intro2ContactReport::Type30 {
                entity_id: id,
                result,
            });
            if blocked {
                continue;
            }
        } else if entity.entity_type == 40 && entity.native_type40_runtime.is_some() {
            let entity_type = entity.entity_type;
            let result = crate::native_actor_surface_contact::resolve_native_actor_surface_contact(
                &mut frame, id,
            );
            let blocked = matches!(
                result,
                crate::native_actor_surface_contact::NativeActorSurfaceContactOutcome::Blocked { .. }
            );
            reports.push(Intro2ContactReport::NativeSurface {
                entity_id: id,
                entity_type,
                result,
            });
            if blocked {
                continue;
            }
            let result =
                crate::native_type40::contact::resolve_type40_static_contact(&mut frame, id);
            let blocked = matches!(
                result,
                crate::native_type40::contact::Type40ContactOutcome::Blocked { .. }
            );
            reports.push(Intro2ContactReport::Type40 {
                entity_id: id,
                result,
            });
            if blocked {
                continue;
            }
        } else if entity.entity_type == 56 && entity.native_type56_runtime.is_some() {
            let entity_type = entity.entity_type;
            let result = crate::native_actor_surface_contact::resolve_native_actor_surface_contact(
                &mut frame, id,
            );
            let blocked = matches!(
                result,
                crate::native_actor_surface_contact::NativeActorSurfaceContactOutcome::Blocked { .. }
            );
            reports.push(Intro2ContactReport::NativeSurface {
                entity_id: id,
                entity_type,
                result,
            });
            if blocked {
                continue;
            }
            let result =
                crate::native_type56::contact::resolve_type56_static_contact(&mut frame, id);
            let blocked = matches!(
                result,
                crate::native_type56::contact::Type56ContactOutcome::Blocked { .. }
            );
            reports.push(Intro2ContactReport::Type56 {
                entity_id: id,
                result,
            });
            if blocked {
                continue;
            }
        } else if matches!(entity.entity_type, 15 | 87) {
            let entity_type = entity.entity_type;
            let outcome =
                crate::intro2_flyer_contacts::resolve_native_flyer_contacts(&mut frame, id);
            let blocked = outcome.blocks_later_contacts();
            reports.push(Intro2ContactReport::Flyer {
                entity_id: id,
                result: outcome.surface,
            });
            if let Some(result) = outcome.static_contact {
                reports.push(Intro2ContactReport::InsectStatic {
                    entity_id: id,
                    entity_type,
                    result,
                });
            }
            if blocked {
                continue;
            }
        } else if entity.entity_type == 47 {
            // Newant static contact: without this pass ants walk through
            // live structures. The 02CA0 task-hook retarget stays open.
            let metadata = frame.entities.type_runtime_metadata(47).cloned();
            if let Some(metadata) = metadata {
                let result = crate::type47_static_contact::resolve_intro2_type47_static_contact(
                    frame.entities,
                    id,
                    &metadata,
                    crate::type47_static_contact::Type47StaticContactFrame {
                        resources: frame.resources,
                        static_damage: frame.static_damage,
                        world_fx: frame.world_fx,
                        scheduler: frame.actor_tasks,
                        retail_tick: frame.retail_tick,
                    },
                );
                reports.push(Intro2ContactReport::Type47 {
                    entity_id: id,
                    result,
                });
            }
        } else if entity.entity_type == 53 && entity.intro2_type53_runtime.is_some() {
            // Model302 static contact through the owned Type53 11AD0 scan,
            // including Chase primaries with a coexisting Tertiary Aim.
            let result =
                crate::intro2_type53::contact::resolve_type53_static_contact(&mut frame, id);
            let blocked = matches!(
                result,
                crate::intro2_type53::contact::Type53ContactOutcome::Blocked { .. }
            );
            reports.push(Intro2ContactReport::Type53 {
                entity_id: id,
                result,
            });
            if blocked {
                continue;
            }
        }
        let result = crate::intro2_type17::pair::resolve_type17_active_contacts(
            &mut frame,
            id,
            crate::intro2_type17::pair::CaptureFeedbackPolicy::Cinematic,
        );
        reports.push(Intro2ContactReport::Type17Pair {
            entity_id: id,
            result,
        });
    }
    reports
}

#[cfg(test)]
mod tests;
