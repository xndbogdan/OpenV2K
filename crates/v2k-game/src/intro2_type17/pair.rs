//! Type17 public entry names for the shared native captor pair program.

pub use crate::native_actor_capture::pair::{
    resolve_native_captor_active_contacts as resolve_type17_active_contacts,
    resolve_native_captor_active_contacts_with_playing as resolve_type17_active_contacts_with_playing,
    CaptureFeedbackPolicy, NativeCaptorPairBehaviorResult as Type17PairBehaviorResult,
    NativeCaptorPairBlock as Type17PairBlock,
    NativeCaptorPairComponentResult as Type17PairComponentResult,
    NativeCaptorPairOutcome as Type17PairOutcome, NativeCaptorPairStage as Type17PairStage,
    NativeCaptorPairVisit as Type17PairVisit,
};

#[cfg(test)]
use crate::{
    active_pair::plan_active_pair_response_and_damage_cap,
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    entity::EntityManager,
    entity_collision_state::{RetailRuntimeValue, DYING_STATE_BIT},
    intro2_contacts::Intro2ContactFrame,
    intro2_radial::Intro2RadialTaskCustody,
    intro2_type17::Intro2Type17Owner,
    native_actor_capture::{
        self as capture,
        pair::{actor, style_address},
    },
    player_active_contact::{active_pair_body_from_entity, classify_oriented_active_pair_contact},
};
#[cfg(test)]
#[path = "pair_physical_tests.rs"]
mod physical_tests;
#[cfg(test)]
#[path = "pair_tests.rs"]
pub(crate) mod tests;
