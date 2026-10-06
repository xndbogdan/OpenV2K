//! Shared authentication for a selected class-45 root predecessor.
//!
//! Root applications consume the same two exact Attract Attention graphs.
//! Keeping that authority here prevents a new destination from weakening the
//! accepted initial/Candidate/Cue or Target Route shape.

use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskOwner, ActorTaskSlot},
    attract_attention::{
        ATTRACT_ATTENTION_CUE_LIFETIME_MS, ATTRACT_ATTENTION_INITIAL_STYLE,
        ATTRACT_ATTENTION_WANDER_LIFETIME_MS,
    },
    entity_behavior::{
        audited_behavior_style, behavior_program, ActiveBehaviorStyle, BehaviorChoiceListSource,
        BehaviorContextRuntime, BehaviorDescriptorIdentity,
    },
    entity_collision_state::RetailRuntimeValue,
    ordinary_type9_initial_selection::LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID,
    ordinary_type9_live::OrdinaryType9SelectedRuntimeKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OrdinaryType9AttractAttentionPredecessorAuthError {
    ContextMismatch,
    TaskGraphMismatch,
}

pub(crate) fn authenticate_ordinary_type9_attract_attention_predecessor(
    actor_tasks: &ActorTaskOwner<ActorTaskRuntime>,
    context: BehaviorContextRuntime,
    selected_kind: OrdinaryType9SelectedRuntimeKind,
) -> Result<(), OrdinaryType9AttractAttentionPredecessorAuthError> {
    let program = behavior_program(u32::from(LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID))
        .expect("Attract Attention is statically audited");
    if context.descriptor() != BehaviorDescriptorIdentity::Named(program)
        || context.choice_list_source()
            != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
    {
        return Err(OrdinaryType9AttractAttentionPredecessorAuthError::ContextMismatch);
    }

    let graph_matches = match selected_kind {
        OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished => {
            if context.active_style() != ActiveBehaviorStyle::Audited(program.initial_style)
                || context.style_table_index_raw_at_0x10() != program.initial_style_table_index_raw
            {
                return Err(OrdinaryType9AttractAttentionPredecessorAuthError::ContextMismatch);
            }
            let primary_matches = matches!(
                actor_tasks.state_in_slot(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::SharedRetarget(task))
                    if task.lifetime_ms() == ATTRACT_ATTENTION_WANDER_LIFETIME_MS
            );
            let secondary_matches = match actor_tasks.state_in_slot(ActorTaskSlot::Secondary) {
                None => true,
                Some(ActorTaskRuntime::AttractAttentionCandidate(candidate)) => {
                    candidate.constructor_filter_override_raw()
                        == ATTRACT_ATTENTION_INITIAL_STYLE.initializer_argument
                }
                Some(_) => false,
            };
            let tertiary_matches = matches!(
                actor_tasks.state_in_slot(ActorTaskSlot::Tertiary),
                Some(ActorTaskRuntime::AttractAttentionCue(cue))
                    if cue.lifetime_ms() == ATTRACT_ATTENTION_CUE_LIFETIME_MS
            );
            primary_matches && secondary_matches && tertiary_matches
        }
        OrdinaryType9SelectedRuntimeKind::AttractAttentionTargetRoutePublished => {
            let target_style =
                audited_behavior_style(u32::from(LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID), 1)
                    .expect("Attract Attention target style is statically audited");
            if context.active_style() != ActiveBehaviorStyle::Audited(*target_style)
                || context.style_table_index_raw_at_0x10() != 1
            {
                return Err(OrdinaryType9AttractAttentionPredecessorAuthError::ContextMismatch);
            }
            let RetailRuntimeValue::Known(Some(target_id)) = context.target_handle_at_0x08() else {
                return Err(OrdinaryType9AttractAttentionPredecessorAuthError::TaskGraphMismatch);
            };
            matches!(
                actor_tasks.state_in_slot(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::AttractAttentionTargetRoute(task))
                    if task.target_id() == Some(target_id)
            ) && actor_tasks
                .state_in_slot(ActorTaskSlot::Secondary)
                .is_none()
                && actor_tasks.state_in_slot(ActorTaskSlot::Tertiary).is_none()
        }
        _ => false,
    };
    if graph_matches {
        Ok(())
    } else {
        Err(OrdinaryType9AttractAttentionPredecessorAuthError::TaskGraphMismatch)
    }
}
