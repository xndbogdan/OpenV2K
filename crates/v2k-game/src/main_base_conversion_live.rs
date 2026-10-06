//! Concrete native Main Base conversion transaction.
//!
//! This adapter binds the saved-next executor to the same active-pair gates
//! and oriented Section-8 probe as ordinary live collision. It admits native
//! descriptor-backed bases and the retained first-world base fixture. The
//! native source suffix authenticates Type9/78/86/95/123 and each actual task slot.
//! Worker outputs 8/79/90/91/116 retain their own initializer and metadata; other
//! profiles fail before event 1 until their exact output owner is implemented.

use crate::active_pair::{ActivePairContact, ActivePairUnresolved};
use crate::entity::{Entity, EntityManager, MainBaseConversionDestroyQueueOutcome};
use crate::entity_behavior::PairContactCallbackPolicy;
use crate::entity_collision_state::{
    PairComponentContactPolicy, RetailRuntimeValue, REMOTE_OWNED_STATE_BIT,
};
use crate::entity_pair_callbacks::{
    PairCallbackEntitySnapshot, MAIN_BASE_CONVERSION_RESOURCE_EVENT_ID,
};
use crate::gameplay_notifications::GameplayNotifications;
use crate::intro2_radial::Intro2RadialTaskCustody;
use crate::main_base_conversion::{
    run_main_base_conversion_pass, MainBaseConversionHost, MainBaseConversionPass,
    MainBaseConversionRequest, MainBaseReplacementSpawn,
};
use crate::main_base_conversion_physical::{
    plan_main_base_type9_physical_bodies, MainBaseType9PhysicalPlan,
    MainBaseType9PhysicalPlanError, FIRST_WORLD_MAIN_BASE_ENTITY_TYPE,
};
use crate::main_base_conversion_runtime::{
    apply_prepared_first_world_main_base_replacement, prepare_first_world_main_base_replacement,
    MainBaseReplacementPreflightError, PreparedFirstWorldMainBaseReplacement,
};
use crate::main_base_person_contact::{
    native_person_allocation_authenticates, plan_native_person_main_base_components,
    NativePersonMainBaseComponentError, NativePersonMainBaseComponentOutcome,
    NativePersonMainBaseComponentPlan,
};
use crate::main_base_type9_component::{
    entity_forward_half_space, plan_type9_main_base_component, Type9MainBaseComponentOutcome,
    Type9MainBaseComponentPlan, Type9MainBaseComponentPlanError,
    FIRST_WORLD_MAIN_BASE_SOURCE_ENTITY_TYPE,
};
use crate::native_type86::NativePersonProfile;
use crate::player_active_contact::{
    active_pair_body_from_entity, classify_oriented_active_pair_contact,
};
use crate::specialized_actor_task_production::SpecializedActorTaskScheduler;
use crate::world_fx::WorldFx;
use v2k_formats::models::CollisionModelPool;
use v2k_formats::terrain::TerrainGrid;

const FIRST_WORLD_MAIN_BASE_SPAWN_INDEX: usize = 6;
const MAIN_BASE_TARGET_CAPABILITY: u32 = 0x0000_0800;
const CALLBACK_DYING_STATE_BIT: u32 = 0x0000_4000;

#[cfg(test)]
mod native_admission_tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirstWorldMainBaseConversionUnresolved {
    MainBaseRuntimeUnavailable,
    CandidateRuntimeUnavailable { entity_id: u32 },
    UnsupportedSourceFamily { entity_id: u32, entity_type: u32 },
    SourceTaskCustodyUnavailable { entity_id: u32 },
    SourceAllocationUnavailable { entity_id: u32 },
    MainBaseBehaviorUnavailable,
    MainBaseBehaviorMismatch,
    CandidateBehaviorUnavailable { entity_id: u32 },
    CandidateBehaviorMismatch { entity_id: u32 },
    CandidateComponentUnavailable { entity_id: u32 },
    CandidateComponentMismatch { entity_id: u32 },
    CallbackStateUnavailable { entity_id: u32 },
    ActivePair(ActivePairUnresolved),
    Replacement(MainBaseReplacementPreflightError),
    Component(Type9MainBaseComponentPlanError),
    NativeComponent(NativePersonMainBaseComponentError),
    Physical(MainBaseType9PhysicalPlanError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstWorldMainBaseConversionSuffixPlan {
    component: MainBaseConversionComponentPlan,
    physical: MainBaseType9PhysicalPlan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MainBaseConversionComponentPlan {
    Native(NativePersonMainBaseComponentPlan),
    CapturedType9(Option<Type9MainBaseComponentPlan>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MainBaseConversionComponentOutcome {
    Native([NativePersonMainBaseComponentOutcome; 3]),
    CapturedType9(Option<Type9MainBaseComponentOutcome>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstWorldMainBaseConversionSuffixOutcome {
    pub component: MainBaseConversionComponentOutcome,
    pub source_velocity_after_raw: [i16; 3],
    pub capped_pair_damage_raw: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirstWorldMainBaseConversionPass {
    MainBaseAbsent,
    Resolved(MainBaseConversionPass<FirstWorldMainBaseConversionSuffixOutcome>),
}

/// Caller-owned context for `4258A0`; `42EB70` reads the current world's
/// Section-13 +0x48 selector, independently of the player craft.
pub struct MainBaseConversionFrame<'a, P: CollisionModelPool + ?Sized> {
    pub entities: &'a mut EntityManager,
    pub actor_tasks: &'a mut SpecializedActorTaskScheduler,
    pub model_pool: &'a P,
    pub terrain: Option<&'a TerrainGrid>,
    pub world_fx: &'a mut WorldFx,
    pub notifications: &'a mut GameplayNotifications,
    pub retail_tick: u32,
    pub world_style: RetailRuntimeValue<u32>,
}

/// Execute the bounded conversion pass with the actual current world selector.
pub fn resolve_first_world_main_base_conversions<P: CollisionModelPool + ?Sized>(
    frame: MainBaseConversionFrame<'_, P>,
) -> Result<
    FirstWorldMainBaseConversionPass,
    crate::main_base_conversion::MainBaseConversionFailure<
        FirstWorldMainBaseConversionUnresolved,
        FirstWorldMainBaseConversionSuffixOutcome,
    >,
> {
    let MainBaseConversionFrame {
        entities,
        actor_tasks,
        model_pool,
        terrain,
        world_fx,
        notifications,
        retail_tick,
        world_style,
    } = frame;
    let Some(main_base_id) = entities
        .iter_all()
        .find(|entity| {
            entity.entity_type == FIRST_WORLD_MAIN_BASE_ENTITY_TYPE
                && (crate::main_base_runtime::main_base_manager_allocation_authenticates(
                    entities, entity.id,
                ) || (entity.main_base_runtime.is_none()
                    && entities.has_captured_first_world_construction()
                    && entity.authored_spawn_index == Some(FIRST_WORLD_MAIN_BASE_SPAWN_INDEX)))
        })
        .map(|entity| entity.id)
    else {
        return Ok(FirstWorldMainBaseConversionPass::MainBaseAbsent);
    };
    let mut host = FirstWorldMainBaseConversionHost {
        entities,
        actor_tasks,
        model_pool,
        terrain,
        world_fx,
        notifications,
        retail_tick,
    };
    run_main_base_conversion_pass(
        &mut host,
        MainBaseConversionRequest {
            main_base_entity_id: main_base_id,
            world_style,
        },
    )
    .map(FirstWorldMainBaseConversionPass::Resolved)
}

struct FirstWorldMainBaseConversionHost<'a, P: CollisionModelPool + ?Sized> {
    entities: &'a mut EntityManager,
    actor_tasks: &'a mut SpecializedActorTaskScheduler,
    model_pool: &'a P,
    terrain: Option<&'a TerrainGrid>,
    world_fx: &'a mut WorldFx,
    notifications: &'a mut GameplayNotifications,
    retail_tick: u32,
}

impl<P: CollisionModelPool + ?Sized> FirstWorldMainBaseConversionHost<'_, P> {
    fn entity(&self, id: u32) -> Option<&Entity> {
        self.entities.iter_all().find(|entity| entity.id == id)
    }
}

impl<P: CollisionModelPool + ?Sized> MainBaseConversionHost
    for FirstWorldMainBaseConversionHost<'_, P>
{
    type Contact = ActivePairContact;
    type ReplacementPlan = PreparedFirstWorldMainBaseReplacement;
    type ContactSuffixPlan = FirstWorldMainBaseConversionSuffixPlan;
    type ContactSuffixOutcome = FirstWorldMainBaseConversionSuffixOutcome;
    type Unresolved = FirstWorldMainBaseConversionUnresolved;

    fn first_live_entity_id(&self) -> Option<u32> {
        self.entities.retail_live_order_ids().next()
    }

    fn next_live_entity_id(&self, entity_id: u32) -> Option<u32> {
        let mut ids = self.entities.retail_live_order_ids();
        ids.find(|id| *id == entity_id)?;
        ids.next()
    }

    fn classify_exact_contact(
        &self,
        main_base_entity_id: u32,
        candidate_id: u32,
    ) -> Result<Option<Self::Contact>, Self::Unresolved> {
        let main_base = self
            .entity(main_base_entity_id)
            .ok_or(FirstWorldMainBaseConversionUnresolved::MainBaseRuntimeUnavailable)?;
        let candidate = self.entity(candidate_id).ok_or(
            FirstWorldMainBaseConversionUnresolved::CandidateRuntimeUnavailable {
                entity_id: candidate_id,
            },
        )?;
        if candidate.capability_flags & MAIN_BASE_TARGET_CAPABILITY == 0
            || self
                .entities
                .pending_main_base_conversion_destroy_ids()
                .contains(&candidate_id)
        {
            return Ok(None);
        }

        // FUN_00411AD0 reaches behavior/component callbacks only after the
        // active-pair gates and oriented model probe find a contact. A distant
        // peasant's unresolved callback must not stop this saved-next scan
        // before a later peasant can actually reach the Main Base.
        let Some(contact) = classify_with_shared_active_pair(
            main_base,
            candidate,
            self.model_pool,
            self.retail_tick,
        )?
        else {
            return Ok(None);
        };

        // 4258DD rejects every remote target before conversion side effects.
        // This bounded adapter performs no component/physical suffix when the
        // callback has no actions; the ordinary pair owner retains that work.
        // A remote native allocation therefore needs no local task custody.
        match candidate
            .collision
            .state_flags_at_0x08
            .masked(REMOTE_OWNED_STATE_BIT)
        {
            RetailRuntimeValue::Known(0) => {}
            RetailRuntimeValue::Known(_) => return Ok(None),
            RetailRuntimeValue::Unresolved => {
                return Err(
                    FirstWorldMainBaseConversionUnresolved::CallbackStateUnavailable {
                        entity_id: candidate.id,
                    },
                );
            }
        }

        // 258A0 gates on capability 800, not Type9. Non-Type9 people must
        // not disappear from the contact census merely because the retained
        // conversion suffix/output adapter does not yet own their profile.
        let captured_type9 = candidate.entity_type == FIRST_WORLD_MAIN_BASE_SOURCE_ENTITY_TYPE
            && candidate.ordinary_type9_native_receipt.is_none()
            && self.entities.has_captured_first_world_construction();
        if !captured_type9 && !native_person_allocation_authenticates(self.entities, candidate.id) {
            if matches!(candidate.entity_type, 9 | 123)
                || NativePersonProfile::from_entity_type(candidate.entity_type).is_some()
            {
                return Err(
                    FirstWorldMainBaseConversionUnresolved::SourceAllocationUnavailable {
                        entity_id: candidate.id,
                    },
                );
            }
            return Err(
                FirstWorldMainBaseConversionUnresolved::UnsupportedSourceFamily {
                    entity_id: candidate.id,
                    entity_type: candidate.entity_type,
                },
            );
        }

        match main_base.current_behavior_context {
            RetailRuntimeValue::Known(Some(context))
                if context.active_style().pair_contact_callback_policy()
                    == PairContactCallbackPolicy::MainBaseConversion => {}
            RetailRuntimeValue::Known(_) => {
                return Err(FirstWorldMainBaseConversionUnresolved::MainBaseBehaviorMismatch)
            }
            RetailRuntimeValue::Unresolved => {
                return Err(FirstWorldMainBaseConversionUnresolved::MainBaseBehaviorUnavailable)
            }
        }
        validate_candidate_behavior_callback(candidate_id, candidate.current_behavior_context)?;
        if native_person_allocation_authenticates(self.entities, candidate_id) {
            if !self
                .actor_tasks
                .main_base_person_completed_owner(self.entities, candidate_id)
            {
                return Err(
                    FirstWorldMainBaseConversionUnresolved::SourceTaskCustodyUnavailable {
                        entity_id: candidate_id,
                    },
                );
            }
            // Native plans inspect all three actual task slots; a null callback
            // is distinct from the retained first-world slot-zero fixture.
            return Ok(Some(contact));
        }
        match candidate.collision.pair_callbacks.component_contact {
            RetailRuntimeValue::Known(
                [PairComponentContactPolicy::DescriptorContact, PairComponentContactPolicy::None, PairComponentContactPolicy::None],
            ) => {}
            RetailRuntimeValue::Known(_) => {
                return Err(
                    FirstWorldMainBaseConversionUnresolved::CandidateComponentMismatch {
                        entity_id: candidate_id,
                    },
                )
            }
            RetailRuntimeValue::Unresolved => {
                return Err(
                    FirstWorldMainBaseConversionUnresolved::CandidateComponentUnavailable {
                        entity_id: candidate_id,
                    },
                )
            }
        }

        Ok(Some(contact))
    }

    fn target_snapshot(
        &self,
        candidate_id: u32,
    ) -> Result<PairCallbackEntitySnapshot, Self::Unresolved> {
        let candidate = self.entity(candidate_id).ok_or(
            FirstWorldMainBaseConversionUnresolved::CandidateRuntimeUnavailable {
                entity_id: candidate_id,
            },
        )?;
        let mask = CALLBACK_DYING_STATE_BIT | REMOTE_OWNED_STATE_BIT;
        let state_flags_raw = match candidate.collision.state_flags_at_0x08.masked(mask) {
            RetailRuntimeValue::Known(flags) => flags,
            RetailRuntimeValue::Unresolved => {
                return Err(
                    FirstWorldMainBaseConversionUnresolved::CallbackStateUnavailable {
                        entity_id: candidate_id,
                    },
                )
            }
        };
        Ok(PairCallbackEntitySnapshot {
            id: candidate.id,
            position_raw: candidate.position_raw(),
            state_flags_raw,
            capability_flags_raw: candidate.capability_flags,
        })
    }

    fn plan_main_base_replacement(
        &self,
        request: MainBaseReplacementSpawn,
    ) -> Result<Self::ReplacementPlan, Self::Unresolved> {
        prepare_first_world_main_base_replacement(
            self.entities,
            self.terrain,
            request,
            self.retail_tick,
        )
        .map_err(FirstWorldMainBaseConversionUnresolved::Replacement)
    }

    fn plan_contact_suffix(
        &self,
        main_base_entity_id: u32,
        candidate_id: u32,
        contact: &Self::Contact,
    ) -> Result<Self::ContactSuffixPlan, Self::Unresolved> {
        let main_base = self
            .entity(main_base_entity_id)
            .ok_or(FirstWorldMainBaseConversionUnresolved::MainBaseRuntimeUnavailable)?;
        let candidate = self.entity(candidate_id).ok_or(
            FirstWorldMainBaseConversionUnresolved::CandidateRuntimeUnavailable {
                entity_id: candidate_id,
            },
        )?;
        let component = if native_person_allocation_authenticates(self.entities, candidate_id) {
            MainBaseConversionComponentPlan::Native(
                plan_native_person_main_base_components(
                    self.entities,
                    candidate_id,
                    main_base_entity_id,
                )
                .map_err(FirstWorldMainBaseConversionUnresolved::NativeComponent)?,
            )
        } else {
            MainBaseConversionComponentPlan::CapturedType9(
                plan_type9_main_base_component(
                    candidate,
                    entity_forward_half_space(candidate, main_base.position_raw()),
                )
                .map_err(FirstWorldMainBaseConversionUnresolved::Component)?,
            )
        };

        let main_base_body = active_pair_body_from_entity(main_base, self.model_pool);
        let mut source_body = active_pair_body_from_entity(candidate, self.model_pool);
        // Suffix commit compares the live source after
        // `queue_main_base_conversion_destroy`. That owner applies
        // `FUN_00410B70`'s full `0x0016_0000` write, not a pending-bit or.
        source_body
            .collision
            .state_flags_at_0x08
            .apply_deferred_destroy_pending_write();
        let physical = plan_main_base_type9_physical_bodies(main_base_body, source_body, *contact)
            .map_err(FirstWorldMainBaseConversionUnresolved::Physical)?;
        Ok(FirstWorldMainBaseConversionSuffixPlan {
            component,
            physical,
        })
    }

    fn queue_resource_notification(&mut self, event_id: u8) {
        debug_assert_eq!(event_id, MAIN_BASE_CONVERSION_RESOURCE_EVENT_ID);
        self.notifications
            .queue_main_base_conversion(self.retail_tick as i32);
    }

    fn queue_deferred_destroy(&mut self, entity_id: u32) {
        if native_person_allocation_authenticates(self.entities, entity_id) {
            // Preflight proved completed custody. Consume Type9's retained
            // observation before 410B70 changes the source's state words.
            assert!(self
                .actor_tasks
                .prepare_native_actor_mutation(self.entities, entity_id));
        }
        assert_eq!(
            self.entities.queue_main_base_conversion_destroy(entity_id),
            MainBaseConversionDestroyQueueOutcome::Queued {
                source_id: entity_id
            },
            "preflighted source must accept the deferred destroy exactly once"
        );
    }

    fn spawn_main_base_replacement(&mut self, plan: Self::ReplacementPlan) -> Option<u32> {
        let spawned = apply_prepared_first_world_main_base_replacement(
            self.entities,
            self.terrain,
            self.world_fx,
            plan,
        );
        self.notifications
            .drain_attract_attention_receipts(self.entities, self.retail_tick as i32)
            .expect("native constructor receipts contain the audited BA40 event");
        Some(spawned.replacement_id)
    }

    fn apply_contact_suffix(
        &mut self,
        candidate_id: u32,
        plan: Self::ContactSuffixPlan,
    ) -> Self::ContactSuffixOutcome {
        let source_velocity_after_raw = plan.physical.resolved_source().velocity_raw;
        let capped_pair_damage_raw = plan.physical.capped_pair_damage_raw();
        let component = self
            .entities
            .commit_main_base_conversion_suffix(
                plan.physical.expected_main_base().id,
                candidate_id,
                plan.component,
                &plan.physical,
            )
            .unwrap_or_else(|source| {
                panic!("preflighted Main Base suffix became stale during local apply: {source:?}")
            });
        FirstWorldMainBaseConversionSuffixOutcome {
            component,
            source_velocity_after_raw,
            capped_pair_damage_raw,
        }
    }
}

fn validate_candidate_behavior_callback(
    entity_id: u32,
    behavior: RetailRuntimeValue<Option<crate::entity_behavior::BehaviorContextRuntime>>,
) -> Result<(), FirstWorldMainBaseConversionUnresolved> {
    match behavior {
        RetailRuntimeValue::Known(None) => Ok(()),
        RetailRuntimeValue::Known(Some(context))
            if context.active_style().pair_contact_callback_policy()
                == PairContactCallbackPolicy::None =>
        {
            Ok(())
        }
        RetailRuntimeValue::Known(Some(_)) => {
            Err(FirstWorldMainBaseConversionUnresolved::CandidateBehaviorMismatch { entity_id })
        }
        RetailRuntimeValue::Unresolved => {
            Err(FirstWorldMainBaseConversionUnresolved::CandidateBehaviorUnavailable { entity_id })
        }
    }
}

fn classify_with_shared_active_pair<P: CollisionModelPool + ?Sized>(
    main_base: &Entity,
    candidate: &Entity,
    model_pool: &P,
    retail_tick: u32,
) -> Result<Option<ActivePairContact>, FirstWorldMainBaseConversionUnresolved> {
    classify_oriented_active_pair_contact(
        crate::player_active_contact::OrientedActivePairContactRequest {
            subject: main_base,
            candidate,
            subject_entry: &crate::player_active_contact::active_pair_body_from_entity(
                main_base, model_pool,
            ),
            retail_tick,
        },
        model_pool,
    )
    .map_err(FirstWorldMainBaseConversionUnresolved::ActivePair)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_behavior::{
        audited_behavior_style, behavior_program, BehaviorContextRuntime, BehaviorSelection,
    };
    use crate::entity_collision_state::RetailStateWord;

    const SOURCE_ID: u32 = 0x04ac_0001;

    #[test]
    fn captured_go_to_job_style_and_known_absence_skip_candidate_behavior_stage() {
        let go_to_job_program = behavior_program(54).expect("captured class-54 program");
        assert_eq!(go_to_job_program.initial_style.frame_address, 0x004c_8788);
        assert_eq!(
            validate_candidate_behavior_callback(
                SOURCE_ID,
                RetailRuntimeValue::Known(Some(
                    BehaviorContextRuntime::from_published_weighted_selection(BehaviorSelection {
                        choice_index: 0,
                        program: go_to_job_program,
                    },)
                    .expect("Go To Job is a weighted catalog behavior"),
                )),
            ),
            Ok(())
        );
        assert_eq!(
            validate_candidate_behavior_callback(SOURCE_ID, RetailRuntimeValue::Known(None)),
            Ok(())
        );
    }

    #[test]
    fn unresolved_or_non_null_candidate_behavior_fails_closed() {
        assert_eq!(
            validate_candidate_behavior_callback(SOURCE_ID, RetailRuntimeValue::Unresolved),
            Err(
                FirstWorldMainBaseConversionUnresolved::CandidateBehaviorUnavailable {
                    entity_id: SOURCE_ID,
                },
            )
        );
        let non_null = BehaviorContextRuntime::named_audited(
            behavior_program(9).expect("Capture People behavior"),
            1,
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Unresolved,
            *audited_behavior_style(9, 1).expect("Capture People variant 1"),
        )
        .expect("Capture People variant 1 is audited");
        assert_eq!(
            validate_candidate_behavior_callback(
                SOURCE_ID,
                RetailRuntimeValue::Known(Some(non_null)),
            ),
            Err(
                FirstWorldMainBaseConversionUnresolved::CandidateBehaviorMismatch {
                    entity_id: SOURCE_ID,
                },
            )
        );
    }

    #[test]
    fn suffix_preflight_uses_the_queued_destroy_write_on_first_scheduler_type9() {
        let mut planned = RetailStateWord::exact(0x0046_8805);
        planned.apply_deferred_destroy_pending_write();
        assert_eq!(
            planned.masked(0x0016_0000),
            RetailRuntimeValue::Known(0x0010_0000)
        );
        assert_eq!(
            planned.masked(0x0006_0000),
            RetailRuntimeValue::Known(0),
            "Wander's first-scheduler word sets 0x00060000; FUN_00410B70 clears it"
        );
    }
}
