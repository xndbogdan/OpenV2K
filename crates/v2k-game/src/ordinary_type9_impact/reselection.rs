//! Shared C690 -> AC60 replacement of an admitted native Type9 hit graph.

use super::*;
use crate::{
    entity_collision_state::EntityTypeRuntimeMetadata,
    guard_location_owner::acquisition::GuardLocationEntityRef,
    ordinary_type9_attract_attention_initializer::OrdinaryType9AttractAttentionAllocationDecision,
    ordinary_type9_current_task::OrdinaryType9CurrentTaskPublication,
    ordinary_type9_go_to_job_initializer::{
        OrdinaryType9GoToJobAllocationDecision, OrdinaryType9GoToJobCandidateEvidence,
    },
    ordinary_type9_live::OrdinaryType9SelectedRuntimeKind,
    ordinary_type9_root_attract_attention_application::{
        apply_ordinary_type9_root_attract_attention_from_predecessor,
        OrdinaryType9RootAttractAttentionApplicationOutcome,
        OrdinaryType9RootAttractAttentionPredecessorKind,
    },
    ordinary_type9_root_go_to_job_application::{
        apply_ordinary_type9_root_go_to_job_from_predecessor,
        OrdinaryType9RootGoToJobApplicationOutcome, OrdinaryType9RootGoToJobPredecessorKind,
    },
    ordinary_type9_root_reselection::{
        plan_ordinary_type9_root_reselection, OrdinaryType9RootReselectionRequest,
        OrdinaryType9RootSelection,
    },
    ordinary_type9_root_run_away_application::{
        apply_ordinary_type9_root_run_away_from_predecessor,
        OrdinaryType9RootRunAwayApplicationOutcome, OrdinaryType9RootRunAwayPredecessorKind,
    },
    ordinary_type9_root_wander_application::{
        apply_ordinary_type9_root_wander, OrdinaryType9RootWanderApplicationOutcome,
    },
    ordinary_type9_run_away_initializer::OrdinaryType9RunAwayAllocationDecision,
    ordinary_type9_wander_initializer::OrdinaryType9WanderAllocationDecision,
};

pub(super) fn reselect(
    manager: &mut EntityManager,
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    world_fx: &mut WorldFx,
    notifications: &mut GameplayNotifications,
    retail_tick: u32,
) -> Result<OrdinaryType9CurrentTaskPublication, NativeType9ImpactBlock> {
    use NativeType9ImpactBlock as Block;
    let candidates: Vec<_> = manager
        .retail_live_order_ids()
        .filter_map(|id| manager.iter_all().find(|entity| entity.id == id))
        .map(candidate)
        .collect();
    let jobs: Vec<_> = manager
        .retail_live_order_ids()
        .filter_map(|id| manager.iter_all().find(|entity| entity.id == id))
        .map(|entity| OrdinaryType9GoToJobCandidateEvidence {
            candidate_id: entity.id,
            state_flags: entity.collision.state_flags_at_0x08,
            capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
            capacity: crate::ordinary_type9_wander_production::job_capacity_from_entity(entity),
        })
        .collect();
    let entity = manager.entity_mut(id).unwrap();
    let kind = entity
        .ordinary_type9_selected_component_runtime
        .ok_or(Block::Runtime("selected components"))?
        .kind();
    let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
        return Err(Block::Runtime("common axis"));
    };
    let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
        return Err(Block::Runtime("Sub-A"));
    };
    let plan = plan_ordinary_type9_root_reselection(
        OrdinaryType9RootReselectionRequest {
            active_model_id: entity.model_index.ok_or(Block::Runtime("active model"))?,
            metadata,
            owner: candidate(entity),
            current_context: entity.current_behavior_context,
            candidates_in_intrusive_order: &candidates,
        },
        || u32::from(world_fx.next_shared_retail_random_u16()),
    )
    .map_err(Block::Reselection)?;
    let OrdinaryType9RootSelection::Weighted { selection, .. } = plan.selection() else {
        // Native death changes to a class14 style whose two hit slots are
        // null before returning. A dying bit with a living selected graph is
        // not an authenticated completed native callback boundary.
        return Err(Block::Runtime(
            "dying selected graph without class14 publication",
        ));
    };
    let visits = super::task_visits(entity);
    let mut next_random = || u32::from(world_fx.next_shared_retail_random_u16());
    match selection.program.class_id {
        6 => {
            match apply_ordinary_type9_root_wander(
                entity,
                metadata,
                plan,
                visits,
                |_| OrdinaryType9WanderAllocationDecision::Prepared,
                &mut next_random,
            )
            .map_err(|_| Block::Initializer("Wander predecessor"))?
            {
                OrdinaryType9RootWanderApplicationOutcome::Published { owner, .. } => {
                    Ok(OrdinaryType9CurrentTaskPublication::Wander(owner))
                }
                _ => Err(Block::Initializer("Wander allocation")),
            }
        }
        10 => {
            let predecessor = match kind {
                OrdinaryType9SelectedRuntimeKind::WanderNearPublished => {
                    OrdinaryType9RootRunAwayPredecessorKind::Wander {
                        actor_common_axis: axis,
                    }
                }
                OrdinaryType9SelectedRuntimeKind::GoToJobPublished => {
                    OrdinaryType9RootRunAwayPredecessorKind::GoToJob {
                        actor_common_axis: axis,
                    }
                }
                OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished
                | OrdinaryType9SelectedRuntimeKind::AttractAttentionTargetRoutePublished => {
                    OrdinaryType9RootRunAwayPredecessorKind::AttractAttention {
                        actor_common_axis: axis,
                    }
                }
                OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished
                | OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished => {
                    OrdinaryType9RootRunAwayPredecessorKind::RunAway
                }
                _ => return Err(Block::Runtime("non-native predecessor family")),
            };
            match apply_ordinary_type9_root_run_away_from_predecessor(
                entity,
                metadata,
                plan,
                visits,
                sub_a,
                predecessor,
                |_| OrdinaryType9RunAwayAllocationDecision::Prepared,
                &mut next_random,
            )
            .map_err(|_| Block::Initializer("Run Away predecessor"))?
            {
                OrdinaryType9RootRunAwayApplicationOutcome::Published { owner, .. } => {
                    Ok(OrdinaryType9CurrentTaskPublication::RunAway(owner))
                }
                _ => Err(Block::Initializer("Run Away allocation")),
            }
        }
        54 => {
            let predecessor = match kind {
                OrdinaryType9SelectedRuntimeKind::WanderNearPublished => {
                    OrdinaryType9RootGoToJobPredecessorKind::Wander
                }
                OrdinaryType9SelectedRuntimeKind::GoToJobPublished => {
                    OrdinaryType9RootGoToJobPredecessorKind::GoToJob
                }
                OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished
                | OrdinaryType9SelectedRuntimeKind::AttractAttentionTargetRoutePublished => {
                    OrdinaryType9RootGoToJobPredecessorKind::AttractAttention
                }
                OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished
                | OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished => {
                    OrdinaryType9RootGoToJobPredecessorKind::RunAway
                }
                _ => return Err(Block::Runtime("non-native predecessor family")),
            };
            match apply_ordinary_type9_root_go_to_job_from_predecessor(
                entity,
                metadata,
                plan,
                visits,
                axis,
                sub_a,
                &jobs,
                predecessor,
                |_| OrdinaryType9GoToJobAllocationDecision::Prepared,
                &mut next_random,
            )
            .map_err(|_| Block::Initializer("Go To Job predecessor"))?
            {
                OrdinaryType9RootGoToJobApplicationOutcome::Published { owner, .. } => {
                    Ok(OrdinaryType9CurrentTaskPublication::GoToJob(owner))
                }
                _ => Err(Block::Initializer("Go To Job allocation")),
            }
        }
        45 => {
            let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
                return Err(Block::Runtime("Sub-I"));
            };
            let predecessor = match kind {
                OrdinaryType9SelectedRuntimeKind::WanderNearPublished => {
                    OrdinaryType9RootAttractAttentionPredecessorKind::Wander
                }
                OrdinaryType9SelectedRuntimeKind::GoToJobPublished => {
                    OrdinaryType9RootAttractAttentionPredecessorKind::GoToJob
                }
                OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished
                | OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished => {
                    OrdinaryType9RootAttractAttentionPredecessorKind::RunAway
                }
                OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished
                | OrdinaryType9SelectedRuntimeKind::AttractAttentionTargetRoutePublished => {
                    OrdinaryType9RootAttractAttentionPredecessorKind::AttractAttention
                }
                _ => return Err(Block::Runtime("non-native predecessor family")),
            };
            let outcome = apply_ordinary_type9_root_attract_attention_from_predecessor(
                entity,
                metadata,
                plan,
                visits,
                axis,
                sub_a,
                animation,
                predecessor,
                |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
                &mut next_random,
                |request| {
                    notifications
                        .queue_attract_attention_resource_text(request, retail_tick as i32)
                        .expect("class45 emits the canonical event-0x10 request");
                },
                |_| {},
            )
            .map_err(|_| Block::Initializer("Attract Attention predecessor"))?;
            let (committed, publication) = match outcome {
                OrdinaryType9RootAttractAttentionApplicationOutcome::Published {
                    committed,
                    owners,
                    publication,
                } => {
                    (committed, Ok(OrdinaryType9CurrentTaskPublication::AttractAttention {
                        publication,
                        initial_owners: owners,
                    }))
                }
                OrdinaryType9RootAttractAttentionApplicationOutcome::InitializerFallbackPublished {
                    committed, ..
                } => {
                    (committed, Err(Block::Initializer("Attract Attention allocation")))
                }
            };
            // Text was submitted at BA40's callback before Cue/Primary
            // preparation. Preserve its prefix even if a later allocation
            // fails; a live hit never queues a load-time text receipt.
            if let Some(sound) = committed.positional_sound {
                world_fx
                    .queue_fixed_positional_sound_raw(sound.global_sound_id, sound.position_raw);
            }
            publication
        }
        _ => Err(Block::Runtime("selector outside canonical Type9 choices")),
    }
}

fn candidate(entity: &Entity) -> GuardLocationEntityRef {
    GuardLocationEntityRef {
        id: entity.id,
        entity_type: entity.entity_type,
        position_raw: entity.position_raw(),
        state_flags_raw: entity.collision.state_flags_at_0x08,
        capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
    }
}
