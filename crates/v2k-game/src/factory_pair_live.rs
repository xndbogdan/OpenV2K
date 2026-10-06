//! Factory-as-subject pair walker for Working Factory intake.
//!
//! Retail `FUN_00411A80` visits the factory after the actor-task pass. The
//! caller selects one actual type-66 allocation, then this adapter classifies
//! workers whose Primary Go-To-Job still targets that factory.
//! Contact uses the shared oriented Section-8 probe. Arrival is never inferred
//! from distance. Applying the delivery journal and the remaining `0xA300`
//! pair visit stays with the scheduler owner.

use crate::active_pair::{ActivePairContact, ActivePairUnresolved};
use crate::actor_task_dispatcher::ActorTaskRuntime;
use crate::actor_task_owner::ActorTaskSlot;
use crate::entity::{Entity, EntityManager};
use crate::entity_behavior::PairContactCallbackPolicy;
use crate::entity_collision_state::{RetailRuntimeValue, DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT};
use crate::player_active_contact::classify_oriented_active_pair_contact;
use v2k_formats::models::CollisionModelPool;

const SCIENTIST_FACTORY_CAPABILITY_BIT: u32 = 0x400;
const FACTORY_CALLBACK_STATE_MASK: u32 = DYING_STATE_BIT | REMOTE_OWNED_STATE_BIT;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactoryPairUnresolved {
    FactoryBehaviorUnavailable { factory_id: u32 },
    FactoryBehaviorMismatch { factory_id: u32 },
    FactoryOwner(crate::factory_activation_live::FactoryActivationLiveError),
    ActivePair(ActivePairUnresolved),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryPairRequest {
    pub factory_id: u32,
    pub retail_tick: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryPairContact {
    pub scientist_id: u32,
    pub contact: ActivePairContact,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactoryPairPass {
    pub factory_id: u32,
    pub contacts: Vec<FactoryPairContact>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactoryPairPassResult {
    FactoryAbsent,
    Resolved(FactoryPairPass),
}

/// Classify factory/scientist contacts. Does not apply delivery or infer range.
pub fn resolve_factory_pair_arrivals<P: CollisionModelPool + ?Sized>(
    entities: &EntityManager,
    model_pool: &P,
    request: FactoryPairRequest,
) -> Result<FactoryPairPassResult, FactoryPairUnresolved> {
    let FactoryPairRequest {
        factory_id,
        retail_tick,
    } = request;
    let Some(factory) = entities
        .iter_all()
        .find(|entity| entity.id == factory_id && entity.active)
    else {
        return Ok(FactoryPairPassResult::FactoryAbsent);
    };
    {
        match factory.current_behavior_context {
            RetailRuntimeValue::Known(Some(context))
                if context.active_style().pair_contact_callback_policy()
                    == PairContactCallbackPolicy::LifterDelivery => {}
            RetailRuntimeValue::Known(_) => {
                return Err(FactoryPairUnresolved::FactoryBehaviorMismatch { factory_id })
            }
            RetailRuntimeValue::Unresolved => {
                return Err(FactoryPairUnresolved::FactoryBehaviorUnavailable { factory_id })
            }
        }
    }
    crate::factory_activation_live::validate_factory_delivery_owner(entities, factory_id)
        .map_err(FactoryPairUnresolved::FactoryOwner)?;

    let candidate_ids = entities
        .retail_live_order_ids()
        .filter(|id| *id != factory_id)
        .collect::<Vec<_>>();
    let mut contacts = Vec::new();
    for candidate_id in candidate_ids {
        let eligible = {
            let Some(candidate) = entities.iter_all().find(|entity| entity.id == candidate_id)
            else {
                continue;
            };
            scientist_is_factory_arrival_candidate(entities, factory_id, candidate)
        };
        if !eligible {
            continue;
        }
        let factory = entities
            .iter_all()
            .find(|entity| entity.id == factory_id)
            .expect("factory remains live during the pair walk");
        let candidate = entities
            .iter_all()
            .find(|entity| entity.id == candidate_id)
            .expect("candidate was just observed");
        // Factory owns the LifterDelivery callback, but its Section-8 program
        // uses target-side `0x8F` boxes. The pair query walker only emits
        // `0x8E` spheres, so the scientist is the query model and the factory
        // is the target. Distance is still not used.
        match classify_oriented_active_pair_contact(
            crate::player_active_contact::OrientedActivePairContactRequest {
                subject: candidate,
                candidate: factory,
                subject_entry: &crate::player_active_contact::active_pair_body_from_entity(
                    candidate, model_pool,
                ),
                retail_tick,
            },
            model_pool,
        ) {
            Ok(Some(contact)) => contacts.push(FactoryPairContact {
                scientist_id: candidate_id,
                contact,
            }),
            Ok(None) => {}
            Err(source) => return Err(FactoryPairUnresolved::ActivePair(source)),
        }
    }
    Ok(FactoryPairPassResult::Resolved(FactoryPairPass {
        factory_id,
        contacts,
    }))
}

fn scientist_is_factory_arrival_candidate(
    entities: &EntityManager,
    factory_id: u32,
    scientist: &Entity,
) -> bool {
    if entities
        .pending_factory_scientist_destroy_ids()
        .contains(&scientist.id)
    {
        return false;
    }
    if !scientist.active || scientist.capability_flags & SCIENTIST_FACTORY_CAPABILITY_BIT == 0 {
        return false;
    }
    let Some(ActorTaskRuntime::GoToJob(state)) = scientist.actor_task_state(ActorTaskSlot::Primary)
    else {
        return false;
    };
    if state.target_id() != Some(factory_id) {
        return false;
    }
    match scientist
        .collision
        .state_flags_at_0x08
        .masked(FACTORY_CALLBACK_STATE_MASK)
    {
        RetailRuntimeValue::Known(0)
            if scientist.collision.state_flags_at_0x08.known_value_bits() != 0 =>
        {
            true
        }
        _ => false,
    }
}
