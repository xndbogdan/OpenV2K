//! Pure plans for the bounded behavior/component pair callbacks.
//!
//! `FUN_00411AD0` invokes the subject's current behavior-style `+0x18`
//! callback and interprets its return immediately. An `0xA300` object is
//! dispatched, normalized to null, and clears physical response, so the
//! candidate behavior still runs. A retained non-tagged object suppresses the
//! candidate behavior. Retail then visits the subject and candidate component
//! chains in that order. This module plans individual callbacks; the active
//! pair coordinator owns that cross-callback sequence.
//!
//! The plans below contain ordered external actions but apply none of them.
//! An unresolved prerequisite returns `Err` before any actions escape, so the
//! active-pair owner can close each plan before execution. Execution is not a
//! blanket rollback transaction: Main Base notification and deferred destroy
//! precede the spawn attempt and survive retail spawn failure. Deferred destroy
//! is intentionally an action rather than an immediate intrusive-list edit.

use crate::entity_collision_state::RetailRuntimeValue;

/// Tag tested immediately after each behavior callback by `FUN_00411AD0`.
pub const PAIR_CALLBACK_CANCEL_TAG: u16 = 0xA300;

const CALLBACK_DYING_STATE_BIT: u32 = 0x0000_4000;
const REMOTE_OWNED_STATE_BIT: u32 = 0x8000_0000;
const LIFTER_TARGET_CAPABILITY: u32 = 0x0000_0400;
const MAIN_BASE_TARGET_CAPABILITY: u32 = 0x0000_0800;
const CONSUMABLE_TARGET_CAPABILITIES: u32 = 0x0000_0C00;
const HIVE_IMPACT_CAPABILITY: u32 = 0x0000_2000;
const POWER_UP_RECIPIENT_CAPABILITY: u32 = 0x0000_0001;

/// Deduplicated resource event emitted before an accepted Main Base
/// conversion. It resolves through the retail `DAT_004CAD80` event table.
pub const MAIN_BASE_CONVERSION_RESOURCE_EVENT_ID: u8 = 1;

/// One exact entity lookup used by a callback plan.
///
/// A caller may only construct this after retaining the full current words.
/// Missing entity/type metadata belongs in the surrounding
/// [`RetailRuntimeValue`], not in zeroed fields here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairCallbackEntitySnapshot {
    pub id: u32,
    pub position_raw: [i16; 3],
    pub state_flags_raw: u32,
    pub capability_flags_raw: u32,
}

impl PairCallbackEntitySnapshot {
    const fn is_dying(self) -> bool {
        self.state_flags_raw & CALLBACK_DYING_STATE_BIT != 0
    }

    const fn is_remote_owned(self) -> bool {
        self.state_flags_raw & REMOTE_OWNED_STATE_BIT != 0
    }
}

/// The exact static return object selected by a bounded callback branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityPairTaggedEffect {
    LifterAccepted,
    PowerUpUnsupportedRecipient,
}

impl EntityPairTaggedEffect {
    pub const fn retail_object_address(self) -> u32 {
        match self {
            Self::LifterAccepted => 0x004B_EAA0,
            Self::PowerUpUnsupportedRecipient => 0x004B_EAA8,
        }
    }

    pub const fn tag(self) -> u16 {
        PAIR_CALLBACK_CANCEL_TAG
    }
}

/// Return value of one behavior-style callback, before component callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairBehaviorCallbackReturn {
    Null,
    TaggedA300(EntityPairTaggedEffect),
}

/// Ordered externally-owned work produced by a callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityPairCallbackAction {
    QueueResourceNotification {
        event_id: u8,
    },
    EmitOperation33 {
        position_raw: [i16; 3],
        source_entity_id: u32,
        remote_owned: bool,
    },
    Feedback {
        feedback_id: u32,
    },
    AdvanceFactoryState {
        lifter_entity_id: u32,
        amount: u32,
    },
    RemoteEntityFeedback {
        entity_id: u32,
        channel: u8,
        code: u8,
    },
    QueueDeferredDestroy {
        entity_id: u32,
    },
    /// Attempt to spawn the selected replacement at `position_raw`. On success,
    /// store the Main Base handle in its `+0x60` relation and emit operation
    /// `0x33` from the replacement; on failure, do neither. Earlier actions in
    /// the plan are not rolled back. The output handle does not exist while the
    /// pure plan is built, so those dependent steps remain one action. Retail
    /// tail-appends the successful spawn; if the old tail has not yet had its
    /// `next` saved, the replacement can be visited later in the same pair scan.
    /// This pure action describes the callback only and does not authorize live
    /// topology mutation by the atomic player-contact executor.
    SpawnMainBaseReplacement {
        main_base_entity_id: u32,
        replacement_type: u32,
        position_raw: [i16; 3],
    },
    SetHiveImpactLatch {
        hive_entity_id: u32,
    },
    DispatchEntityDeath {
        entity_id: u32,
    },
}

/// A successfully closed behavior callback. Actions must be applied in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityPairCallbackPlan {
    pub actions: Vec<EntityPairCallbackAction>,
    pub return_value: PairBehaviorCallbackReturn,
}

impl EntityPairCallbackPlan {
    fn null(actions: Vec<EntityPairCallbackAction>) -> Self {
        Self {
            actions,
            return_value: PairBehaviorCallbackReturn::Null,
        }
    }

    fn tagged(actions: Vec<EntityPairCallbackAction>, effect: EntityPairTaggedEffect) -> Self {
        Self {
            actions,
            return_value: PairBehaviorCallbackReturn::TaggedA300(effect),
        }
    }
}

/// Explicit cutover points which must never be reinterpreted as `Null`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityPairCallbackUnresolved {
    LifterTargetState,
    LifterDeliveryState,
    MainBaseTargetState,
    MainBaseWorldStyle,
    HiveTargetState,
    HiveSourceState,
    HiveImpactInputs,
    CaptureTargetState,
    CaptureCapacity,
    CaptureAttachmentTransaction,
    PowerUpRecipientState,
    PowerUpHandlerAvailability,
    PowerUpInventoryHandler,
}

/// Correlated request for Main Base conversion callback `FUN_004258A0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseConversionInput {
    pub main_base_entity_id: u32,
    pub target: RetailRuntimeValue<Option<PairCallbackEntitySnapshot>>,
    /// Current world-style table field `+0x48`, selected through
    /// `FUN_00450C90`. This is not a random choice.
    pub world_style: RetailRuntimeValue<u32>,
}

const fn main_base_replacement_type(world_style: u32) -> u32 {
    match world_style {
        1 => 8,
        2 => 0x5B,
        3 => 0x5A,
        4 => 0x4F,
        5 => 0x74,
        6 => 7,
        _ => 0,
    }
}

/// Plan Main Base conversion callback `FUN_004258A0`.
///
/// The target capability and remote-owner gates precede the world-style
/// lookup. An accepted visit queues resource event 1, deferred-destroys the
/// source, then attempts the dependent spawn/relation/operation-`0x33` action.
/// Spawn failure preserves the first two actions and skips relation/operation.
/// Retail returns null on every branch.
pub fn plan_main_base_conversion(
    input: MainBaseConversionInput,
) -> Result<EntityPairCallbackPlan, EntityPairCallbackUnresolved> {
    let target = match input.target {
        RetailRuntimeValue::Known(Some(target)) => target,
        RetailRuntimeValue::Known(None) => return Ok(EntityPairCallbackPlan::null(Vec::new())),
        RetailRuntimeValue::Unresolved => {
            return Err(EntityPairCallbackUnresolved::MainBaseTargetState)
        }
    };
    if target.capability_flags_raw & MAIN_BASE_TARGET_CAPABILITY == 0 || target.is_remote_owned() {
        return Ok(EntityPairCallbackPlan::null(Vec::new()));
    }

    let world_style = match input.world_style {
        RetailRuntimeValue::Known(world_style) => world_style,
        RetailRuntimeValue::Unresolved => {
            return Err(EntityPairCallbackUnresolved::MainBaseWorldStyle)
        }
    };
    Ok(EntityPairCallbackPlan::null(vec![
        EntityPairCallbackAction::QueueResourceNotification {
            event_id: MAIN_BASE_CONVERSION_RESOURCE_EVENT_ID,
        },
        EntityPairCallbackAction::QueueDeferredDestroy {
            entity_id: target.id,
        },
        EntityPairCallbackAction::SpawnMainBaseReplacement {
            main_base_entity_id: input.main_base_entity_id,
            replacement_type: main_base_replacement_type(world_style),
            position_raw: target.position_raw,
        },
    ]))
}

/// Exact signed occupancy/capacity words read by `FUN_00418D30`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifterFactoryState {
    pub occupancy_raw: i32,
    pub capacity_raw: i32,
}

/// Exact lifter lookup which owns both the callback identity and state word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifterSourceSnapshot {
    pub id: u32,
    pub state_flags_raw: u32,
}

/// Exact helper prerequisites after `FUN_00425850`'s capability gate.
///
/// `Known` means every field and pointer-presence decision below came from the
/// current callback visit. A port that has not retained them must pass
/// `Unresolved`; it must not translate absent port storage to `false`/`None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifterDeliverySnapshot {
    pub lifter: LifterSourceSnapshot,
    pub type_extension_present: bool,
    pub behavior_state_present: bool,
    pub delivery_enabled: bool,
    pub factory_state: Option<LifterFactoryState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LifterDeliveryRejection {
    RemoteOwnedLifter,
    MissingTypeExtension,
    MissingBehaviorState,
    TargetDying,
    TargetInactive,
    LifterDying,
    LifterInactive,
    DeliveryDisabled,
    MissingFactoryState,
    FactoryAtCapacity,
}

fn evaluate_lifter_delivery(
    target: PairCallbackEntitySnapshot,
    delivery: LifterDeliverySnapshot,
) -> Result<(), LifterDeliveryRejection> {
    if delivery.lifter.state_flags_raw & REMOTE_OWNED_STATE_BIT != 0 {
        return Err(LifterDeliveryRejection::RemoteOwnedLifter);
    }
    if !delivery.type_extension_present {
        return Err(LifterDeliveryRejection::MissingTypeExtension);
    }
    if !delivery.behavior_state_present {
        return Err(LifterDeliveryRejection::MissingBehaviorState);
    }
    if target.is_dying() {
        return Err(LifterDeliveryRejection::TargetDying);
    }
    if target.state_flags_raw == 0 {
        return Err(LifterDeliveryRejection::TargetInactive);
    }
    if delivery.lifter.state_flags_raw & CALLBACK_DYING_STATE_BIT != 0 {
        return Err(LifterDeliveryRejection::LifterDying);
    }
    if delivery.lifter.state_flags_raw == 0 {
        return Err(LifterDeliveryRejection::LifterInactive);
    }
    if !delivery.delivery_enabled {
        return Err(LifterDeliveryRejection::DeliveryDisabled);
    }
    let Some(factory) = delivery.factory_state else {
        return Err(LifterDeliveryRejection::MissingFactoryState);
    };
    if factory.occupancy_raw >= factory.capacity_raw {
        return Err(LifterDeliveryRejection::FactoryAtCapacity);
    }
    Ok(())
}

/// Correlated request for Working Factory/lifter contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifterContactInput {
    pub target: RetailRuntimeValue<Option<PairCallbackEntitySnapshot>>,
    pub delivery: RetailRuntimeValue<LifterDeliverySnapshot>,
}

/// Plan the Working Factory/lifter callback `FUN_00425850`.
pub fn plan_lifter_contact(
    input: LifterContactInput,
) -> Result<EntityPairCallbackPlan, EntityPairCallbackUnresolved> {
    let target = match input.target {
        RetailRuntimeValue::Known(Some(target)) => target,
        RetailRuntimeValue::Known(None) => return Ok(EntityPairCallbackPlan::null(Vec::new())),
        RetailRuntimeValue::Unresolved => {
            return Err(EntityPairCallbackUnresolved::LifterTargetState)
        }
    };

    // The wrapper returns before consulting any helper-owned state.
    if target.capability_flags_raw & LIFTER_TARGET_CAPABILITY == 0 {
        return Ok(EntityPairCallbackPlan::null(Vec::new()));
    }

    let delivery = match input.delivery {
        RetailRuntimeValue::Known(delivery) => delivery,
        RetailRuntimeValue::Unresolved => {
            return Err(EntityPairCallbackUnresolved::LifterDeliveryState)
        }
    };
    if evaluate_lifter_delivery(target, delivery).is_err() {
        return Ok(EntityPairCallbackPlan::null(Vec::new()));
    }

    let mut actions = vec![
        EntityPairCallbackAction::EmitOperation33 {
            position_raw: target.position_raw,
            source_entity_id: delivery.lifter.id,
            remote_owned: false,
        },
        EntityPairCallbackAction::Feedback { feedback_id: 5 },
        EntityPairCallbackAction::AdvanceFactoryState {
            lifter_entity_id: delivery.lifter.id,
            amount: 1,
        },
    ];
    if target.is_remote_owned() {
        actions.push(EntityPairCallbackAction::RemoteEntityFeedback {
            entity_id: target.id,
            channel: 1,
            code: 8,
        });
    }
    actions.push(EntityPairCallbackAction::QueueDeferredDestroy {
        entity_id: target.id,
    });
    Ok(EntityPairCallbackPlan::tagged(
        actions,
        EntityPairTaggedEffect::LifterAccepted,
    ))
}

/// Precomputed exact inputs to the final thresholds in `FUN_0041CE10`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HiveImpactSample {
    pub forward_projection_raw: i32,
    pub impact_scale_raw: u16,
}

fn hive_impact_sets_latch(sample: HiveImpactSample) -> bool {
    sample.forward_projection_raw > 500
        && (u32::from(sample.impact_scale_raw).wrapping_mul(sample.forward_projection_raw as u32)
            as i32)
            > 75_000
}

/// Hive callback source identity and the result of its second live lookup.
/// `Known(None)` is the retail null-lookup branch and therefore supplies a
/// false operation-33 remote byte; `Unresolved` is unavailable port state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HiveSourceSnapshot {
    pub id: u32,
    pub state_flags_raw: Option<u32>,
}

/// Correlated request for the two Hive callback branches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HiveContactInput {
    pub source: RetailRuntimeValue<HiveSourceSnapshot>,
    pub target: RetailRuntimeValue<Option<PairCallbackEntitySnapshot>>,
    pub impact: RetailRuntimeValue<Option<HiveImpactSample>>,
}

/// Plan the hive callback `FUN_004259F0`.
///
/// The `0xC00` consume branch precedes the `0x2000` impact branch. Its
/// operation's remote byte comes from the hive/source state, not the target.
pub fn plan_hive_contact(
    input: HiveContactInput,
) -> Result<EntityPairCallbackPlan, EntityPairCallbackUnresolved> {
    let target = match input.target {
        RetailRuntimeValue::Known(Some(target)) => target,
        RetailRuntimeValue::Known(None) => return Ok(EntityPairCallbackPlan::null(Vec::new())),
        RetailRuntimeValue::Unresolved => {
            return Err(EntityPairCallbackUnresolved::HiveTargetState)
        }
    };

    if target.capability_flags_raw & CONSUMABLE_TARGET_CAPABILITIES != 0 {
        let source = match input.source {
            RetailRuntimeValue::Known(source) => source,
            RetailRuntimeValue::Unresolved => {
                return Err(EntityPairCallbackUnresolved::HiveSourceState)
            }
        };
        return Ok(EntityPairCallbackPlan::null(vec![
            EntityPairCallbackAction::EmitOperation33 {
                position_raw: target.position_raw,
                source_entity_id: source.id,
                remote_owned: source
                    .state_flags_raw
                    .is_some_and(|flags| flags & REMOTE_OWNED_STATE_BIT != 0),
            },
            EntityPairCallbackAction::QueueDeferredDestroy {
                entity_id: target.id,
            },
        ]));
    }

    if target.capability_flags_raw & HIVE_IMPACT_CAPABILITY == 0 {
        return Ok(EntityPairCallbackPlan::null(Vec::new()));
    }

    match input.impact {
        RetailRuntimeValue::Known(Some(sample)) if hive_impact_sets_latch(sample) => {
            let source = match input.source {
                RetailRuntimeValue::Known(source) => source,
                RetailRuntimeValue::Unresolved => {
                    return Err(EntityPairCallbackUnresolved::HiveSourceState)
                }
            };
            Ok(EntityPairCallbackPlan::null(vec![
                EntityPairCallbackAction::SetHiveImpactLatch {
                    hive_entity_id: source.id,
                },
            ]))
        }
        RetailRuntimeValue::Known(_) => Ok(EntityPairCallbackPlan::null(Vec::new())),
        RetailRuntimeValue::Unresolved => Err(EntityPairCallbackUnresolved::HiveImpactInputs),
    }
}

/// Plan the currently safe prefix of Capture People `FUN_0040C910`.
///
/// The full-capacity branch dispatches death and returns null. The attachment,
/// relation, feedback, search, and behavior-transition transaction remains one
/// indivisible unresolved boundary.
pub fn plan_capture_people_contact(
    target: RetailRuntimeValue<Option<PairCallbackEntitySnapshot>>,
    capacity_full: RetailRuntimeValue<bool>,
) -> Result<EntityPairCallbackPlan, EntityPairCallbackUnresolved> {
    let target = match target {
        RetailRuntimeValue::Known(Some(target)) => target,
        RetailRuntimeValue::Known(None) => return Ok(EntityPairCallbackPlan::null(Vec::new())),
        RetailRuntimeValue::Unresolved => {
            return Err(EntityPairCallbackUnresolved::CaptureTargetState)
        }
    };
    if target.is_dying()
        || target.state_flags_raw == 0
        || target.capability_flags_raw & CONSUMABLE_TARGET_CAPABILITIES == 0
    {
        return Ok(EntityPairCallbackPlan::null(Vec::new()));
    }

    match capacity_full {
        RetailRuntimeValue::Known(true) => Ok(EntityPairCallbackPlan::null(vec![
            EntityPairCallbackAction::DispatchEntityDeath {
                entity_id: target.id,
            },
        ])),
        RetailRuntimeValue::Known(false) => {
            Err(EntityPairCallbackUnresolved::CaptureAttachmentTransaction)
        }
        RetailRuntimeValue::Unresolved => Err(EntityPairCallbackUnresolved::CaptureCapacity),
    }
}

/// Plan the currently safe prefix of Power Up `FUN_00425AF0`.
pub fn plan_power_up_contact(
    recipient: RetailRuntimeValue<Option<PairCallbackEntitySnapshot>>,
    inventory_handler_present: RetailRuntimeValue<bool>,
) -> Result<EntityPairCallbackPlan, EntityPairCallbackUnresolved> {
    let recipient = match recipient {
        RetailRuntimeValue::Known(Some(recipient)) => recipient,
        RetailRuntimeValue::Known(None) => {
            return Ok(EntityPairCallbackPlan::tagged(
                Vec::new(),
                EntityPairTaggedEffect::PowerUpUnsupportedRecipient,
            ))
        }
        RetailRuntimeValue::Unresolved => {
            return Err(EntityPairCallbackUnresolved::PowerUpRecipientState)
        }
    };
    if recipient.capability_flags_raw & POWER_UP_RECIPIENT_CAPABILITY == 0 {
        return Ok(EntityPairCallbackPlan::tagged(
            Vec::new(),
            EntityPairTaggedEffect::PowerUpUnsupportedRecipient,
        ));
    }

    match inventory_handler_present {
        RetailRuntimeValue::Known(false) => Ok(EntityPairCallbackPlan::null(Vec::new())),
        RetailRuntimeValue::Known(true) => {
            Err(EntityPairCallbackUnresolved::PowerUpInventoryHandler)
        }
        RetailRuntimeValue::Unresolved => {
            Err(EntityPairCallbackUnresolved::PowerUpHandlerAvailability)
        }
    }
}

/// Successful component-contact work. Its callback return is intentionally
/// absent because `FUN_00411AD0` ignores component return values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentPairCallbackPlan {
    pub actions: Vec<EntityPairCallbackAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentPairCallbackUnresolved {
    ForwardHalfSpace,
    DescriptorContact,
}

/// Legacy prefix-only view of component callback `FUN_00402DA0`.
///
/// Callers that own the source/target snapshots and descriptor component state
/// should use [`crate::descriptor_contact::plan_descriptor_contact`] for the
/// complete, RNG-exact transaction. This helper remains useful only where the
/// pair dispatcher has established the half-space result but has not resolved
/// the callback's private/component owners.
pub fn plan_descriptor_component_contact(
    forward_half_space: RetailRuntimeValue<bool>,
) -> Result<ComponentPairCallbackPlan, ComponentPairCallbackUnresolved> {
    match forward_half_space {
        RetailRuntimeValue::Known(false) => Ok(ComponentPairCallbackPlan {
            actions: Vec::new(),
        }),
        RetailRuntimeValue::Known(true) => Err(ComponentPairCallbackUnresolved::DescriptorContact),
        RetailRuntimeValue::Unresolved => Err(ComponentPairCallbackUnresolved::ForwardHalfSpace),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIFTER_ID: u32 = 66;
    const MAIN_BASE_ID: u32 = 6;
    const HIVE_ID: u32 = 67;

    fn entity(id: u32, capabilities: u32) -> PairCallbackEntitySnapshot {
        PairCallbackEntitySnapshot {
            id,
            position_raw: [101, -202, 303],
            state_flags_raw: 1,
            capability_flags_raw: capabilities,
        }
    }

    fn delivery() -> LifterDeliverySnapshot {
        LifterDeliverySnapshot {
            lifter: LifterSourceSnapshot {
                id: LIFTER_ID,
                state_flags_raw: 1,
            },
            type_extension_present: true,
            behavior_state_present: true,
            delivery_enabled: true,
            factory_state: Some(LifterFactoryState {
                occupancy_raw: 2,
                capacity_raw: 3,
            }),
        }
    }

    #[test]
    fn main_base_gates_do_not_consult_the_world_style() {
        let ordinary = entity(9, 0);
        assert_eq!(
            plan_main_base_conversion(MainBaseConversionInput {
                main_base_entity_id: MAIN_BASE_ID,
                target: RetailRuntimeValue::Known(Some(ordinary)),
                world_style: RetailRuntimeValue::Unresolved,
            }),
            Ok(EntityPairCallbackPlan::null(Vec::new()))
        );
        let remote = PairCallbackEntitySnapshot {
            state_flags_raw: ordinary.state_flags_raw | REMOTE_OWNED_STATE_BIT,
            capability_flags_raw: MAIN_BASE_TARGET_CAPABILITY,
            ..ordinary
        };
        assert_eq!(
            plan_main_base_conversion(MainBaseConversionInput {
                main_base_entity_id: MAIN_BASE_ID,
                target: RetailRuntimeValue::Known(Some(remote)),
                world_style: RetailRuntimeValue::Unresolved,
            }),
            Ok(EntityPairCallbackPlan::null(Vec::new()))
        );
        assert_eq!(
            plan_main_base_conversion(MainBaseConversionInput {
                main_base_entity_id: MAIN_BASE_ID,
                target: RetailRuntimeValue::Unresolved,
                world_style: RetailRuntimeValue::Known(1),
            }),
            Err(EntityPairCallbackUnresolved::MainBaseTargetState)
        );
    }

    #[test]
    fn main_base_acceptance_preserves_the_captured_transaction_order() {
        let peasant = PairCallbackEntitySnapshot {
            id: 0x04AC_0001,
            position_raw: [0x50BA, -0x02FE, 0x39EC],
            state_flags_raw: 0x06C6_8825,
            capability_flags_raw: MAIN_BASE_TARGET_CAPABILITY,
        };
        let plan = plan_main_base_conversion(MainBaseConversionInput {
            main_base_entity_id: 0x04B5_0001,
            target: RetailRuntimeValue::Known(Some(peasant)),
            world_style: RetailRuntimeValue::Known(1),
        })
        .unwrap();
        assert_eq!(
            plan,
            EntityPairCallbackPlan::null(vec![
                EntityPairCallbackAction::QueueResourceNotification {
                    event_id: MAIN_BASE_CONVERSION_RESOURCE_EVENT_ID,
                },
                EntityPairCallbackAction::QueueDeferredDestroy {
                    entity_id: peasant.id,
                },
                EntityPairCallbackAction::SpawnMainBaseReplacement {
                    main_base_entity_id: 0x04B5_0001,
                    replacement_type: 8,
                    position_raw: peasant.position_raw,
                },
            ])
        );
    }

    #[test]
    fn main_base_world_style_mapping_includes_the_retail_default() {
        let target = entity(9, MAIN_BASE_TARGET_CAPABILITY);
        for (world_style, expected_type) in [
            (1, 8),
            (2, 0x5B),
            (3, 0x5A),
            (4, 0x4F),
            (5, 0x74),
            (6, 7),
            (0, 0),
            (7, 0),
        ] {
            let plan = plan_main_base_conversion(MainBaseConversionInput {
                main_base_entity_id: MAIN_BASE_ID,
                target: RetailRuntimeValue::Known(Some(target)),
                world_style: RetailRuntimeValue::Known(world_style),
            })
            .unwrap();
            assert_eq!(
                plan.actions[2],
                EntityPairCallbackAction::SpawnMainBaseReplacement {
                    main_base_entity_id: MAIN_BASE_ID,
                    replacement_type: expected_type,
                    position_raw: target.position_raw,
                }
            );
        }
        assert_eq!(
            plan_main_base_conversion(MainBaseConversionInput {
                main_base_entity_id: MAIN_BASE_ID,
                target: RetailRuntimeValue::Known(Some(target)),
                world_style: RetailRuntimeValue::Unresolved,
            }),
            Err(EntityPairCallbackUnresolved::MainBaseWorldStyle)
        );
    }

    #[test]
    fn lifter_capability_gate_does_not_consult_unresolved_helper_state() {
        assert_eq!(
            plan_lifter_contact(LifterContactInput {
                target: RetailRuntimeValue::Known(Some(entity(9, 0))),
                delivery: RetailRuntimeValue::Unresolved,
            }),
            Ok(EntityPairCallbackPlan::null(Vec::new()))
        );
        assert_eq!(
            plan_lifter_contact(LifterContactInput {
                target: RetailRuntimeValue::Unresolved,
                delivery: RetailRuntimeValue::Known(delivery()),
            }),
            Err(EntityPairCallbackUnresolved::LifterTargetState)
        );
        assert_eq!(
            plan_lifter_contact(LifterContactInput {
                target: RetailRuntimeValue::Known(Some(entity(9, LIFTER_TARGET_CAPABILITY,))),
                delivery: RetailRuntimeValue::Unresolved,
            }),
            Err(EntityPairCallbackUnresolved::LifterDeliveryState)
        );
    }

    #[test]
    fn lifter_helper_rejections_follow_the_retail_gate_order() {
        let target = entity(9, LIFTER_TARGET_CAPABILITY);
        let mut state = delivery();
        state.lifter.state_flags_raw = REMOTE_OWNED_STATE_BIT | CALLBACK_DYING_STATE_BIT;
        state.type_extension_present = false;
        assert_eq!(
            evaluate_lifter_delivery(target, state),
            Err(LifterDeliveryRejection::RemoteOwnedLifter)
        );

        let cases = [
            (
                LifterDeliverySnapshot {
                    type_extension_present: false,
                    ..delivery()
                },
                target,
                LifterDeliveryRejection::MissingTypeExtension,
            ),
            (
                LifterDeliverySnapshot {
                    behavior_state_present: false,
                    ..delivery()
                },
                target,
                LifterDeliveryRejection::MissingBehaviorState,
            ),
            (
                delivery(),
                PairCallbackEntitySnapshot {
                    state_flags_raw: CALLBACK_DYING_STATE_BIT,
                    ..target
                },
                LifterDeliveryRejection::TargetDying,
            ),
            (
                delivery(),
                PairCallbackEntitySnapshot {
                    state_flags_raw: 0,
                    ..target
                },
                LifterDeliveryRejection::TargetInactive,
            ),
            (
                LifterDeliverySnapshot {
                    lifter: LifterSourceSnapshot {
                        state_flags_raw: CALLBACK_DYING_STATE_BIT,
                        ..delivery().lifter
                    },
                    ..delivery()
                },
                target,
                LifterDeliveryRejection::LifterDying,
            ),
            (
                LifterDeliverySnapshot {
                    lifter: LifterSourceSnapshot {
                        state_flags_raw: 0,
                        ..delivery().lifter
                    },
                    ..delivery()
                },
                target,
                LifterDeliveryRejection::LifterInactive,
            ),
            (
                LifterDeliverySnapshot {
                    delivery_enabled: false,
                    ..delivery()
                },
                target,
                LifterDeliveryRejection::DeliveryDisabled,
            ),
            (
                LifterDeliverySnapshot {
                    factory_state: None,
                    ..delivery()
                },
                target,
                LifterDeliveryRejection::MissingFactoryState,
            ),
            (
                LifterDeliverySnapshot {
                    factory_state: Some(LifterFactoryState {
                        occupancy_raw: 3,
                        capacity_raw: 3,
                    }),
                    ..delivery()
                },
                target,
                LifterDeliveryRejection::FactoryAtCapacity,
            ),
        ];
        for (state, target, expected) in cases {
            assert_eq!(evaluate_lifter_delivery(target, state), Err(expected));
        }
    }

    #[test]
    fn lifter_acceptance_preserves_action_order_and_tagged_return() {
        let local = entity(9, LIFTER_TARGET_CAPABILITY);
        let plan = plan_lifter_contact(LifterContactInput {
            target: RetailRuntimeValue::Known(Some(local)),
            delivery: RetailRuntimeValue::Known(delivery()),
        })
        .unwrap();
        assert_eq!(
            plan.actions,
            vec![
                EntityPairCallbackAction::EmitOperation33 {
                    position_raw: local.position_raw,
                    source_entity_id: LIFTER_ID,
                    remote_owned: false,
                },
                EntityPairCallbackAction::Feedback { feedback_id: 5 },
                EntityPairCallbackAction::AdvanceFactoryState {
                    lifter_entity_id: LIFTER_ID,
                    amount: 1,
                },
                EntityPairCallbackAction::QueueDeferredDestroy {
                    entity_id: local.id,
                },
            ]
        );
        assert_eq!(
            plan.return_value,
            PairBehaviorCallbackReturn::TaggedA300(EntityPairTaggedEffect::LifterAccepted)
        );
        assert_eq!(
            EntityPairTaggedEffect::LifterAccepted.retail_object_address(),
            0x004B_EAA0
        );
        assert_eq!(
            EntityPairTaggedEffect::LifterAccepted.tag(),
            PAIR_CALLBACK_CANCEL_TAG
        );

        let remote = PairCallbackEntitySnapshot {
            state_flags_raw: local.state_flags_raw | REMOTE_OWNED_STATE_BIT,
            ..local
        };
        let remote_plan = plan_lifter_contact(LifterContactInput {
            target: RetailRuntimeValue::Known(Some(remote)),
            delivery: RetailRuntimeValue::Known(delivery()),
        })
        .unwrap();
        assert_eq!(
            remote_plan.actions[3],
            EntityPairCallbackAction::RemoteEntityFeedback {
                entity_id: remote.id,
                channel: 1,
                code: 8,
            }
        );
        assert!(matches!(
            remote_plan.actions[4],
            EntityPairCallbackAction::QueueDeferredDestroy { .. }
        ));

        let mut signed_capacity = delivery();
        signed_capacity.factory_state = Some(LifterFactoryState {
            occupancy_raw: -1,
            capacity_raw: 0,
        });
        let signed_plan = plan_lifter_contact(LifterContactInput {
            target: RetailRuntimeValue::Known(Some(local)),
            delivery: RetailRuntimeValue::Known(signed_capacity),
        })
        .unwrap();
        assert_eq!(
            signed_plan.return_value,
            PairBehaviorCallbackReturn::TaggedA300(EntityPairTaggedEffect::LifterAccepted)
        );
    }

    #[test]
    fn hive_consume_precedes_impact_and_uses_source_remote_state() {
        let both = entity(17, CONSUMABLE_TARGET_CAPABILITIES | HIVE_IMPACT_CAPABILITY);
        let plan = plan_hive_contact(HiveContactInput {
            source: RetailRuntimeValue::Known(HiveSourceSnapshot {
                id: HIVE_ID,
                state_flags_raw: Some(REMOTE_OWNED_STATE_BIT),
            }),
            target: RetailRuntimeValue::Known(Some(both)),
            impact: RetailRuntimeValue::Unresolved,
        })
        .unwrap();
        assert_eq!(
            plan,
            EntityPairCallbackPlan::null(vec![
                EntityPairCallbackAction::EmitOperation33 {
                    position_raw: both.position_raw,
                    source_entity_id: HIVE_ID,
                    remote_owned: true,
                },
                EntityPairCallbackAction::QueueDeferredDestroy { entity_id: both.id },
            ])
        );

        let no_flags = entity(17, 0);
        assert_eq!(
            plan_hive_contact(HiveContactInput {
                source: RetailRuntimeValue::Unresolved,
                target: RetailRuntimeValue::Known(Some(no_flags)),
                impact: RetailRuntimeValue::Unresolved,
            }),
            Ok(EntityPairCallbackPlan::null(Vec::new()))
        );

        let remote_target = PairCallbackEntitySnapshot {
            state_flags_raw: both.state_flags_raw | REMOTE_OWNED_STATE_BIT,
            ..both
        };
        let local_source_plan = plan_hive_contact(HiveContactInput {
            source: RetailRuntimeValue::Known(HiveSourceSnapshot {
                id: HIVE_ID,
                state_flags_raw: None,
            }),
            target: RetailRuntimeValue::Known(Some(remote_target)),
            impact: RetailRuntimeValue::Unresolved,
        })
        .unwrap();
        assert_eq!(
            local_source_plan.actions[0],
            EntityPairCallbackAction::EmitOperation33 {
                position_raw: remote_target.position_raw,
                source_entity_id: HIVE_ID,
                remote_owned: false,
            }
        );
    }

    #[test]
    fn hive_impact_uses_strict_wrapping_signed_thresholds() {
        let target = entity(17, HIVE_IMPACT_CAPABILITY);
        let run = |sample| {
            plan_hive_contact(HiveContactInput {
                source: RetailRuntimeValue::Known(HiveSourceSnapshot {
                    id: HIVE_ID,
                    state_flags_raw: Some(0),
                }),
                target: RetailRuntimeValue::Known(Some(target)),
                impact: RetailRuntimeValue::Known(sample),
            })
        };
        assert_eq!(run(None), Ok(EntityPairCallbackPlan::null(Vec::new())));
        assert_eq!(
            run(Some(HiveImpactSample {
                forward_projection_raw: 500,
                impact_scale_raw: u16::MAX,
            })),
            Ok(EntityPairCallbackPlan::null(Vec::new()))
        );
        assert_eq!(
            run(Some(HiveImpactSample {
                forward_projection_raw: 501,
                impact_scale_raw: 149,
            })),
            Ok(EntityPairCallbackPlan::null(Vec::new()))
        );
        assert_eq!(
            run(Some(HiveImpactSample {
                forward_projection_raw: 501,
                impact_scale_raw: 150,
            })),
            Ok(EntityPairCallbackPlan::null(vec![
                EntityPairCallbackAction::SetHiveImpactLatch {
                    hive_entity_id: HIVE_ID,
                },
            ]))
        );
        assert_eq!(
            run(Some(HiveImpactSample {
                forward_projection_raw: 65_536,
                impact_scale_raw: 32_768,
            })),
            Ok(EntityPairCallbackPlan::null(Vec::new())),
            "the retail imul wraps before its signed comparison"
        );
        assert_eq!(
            plan_hive_contact(HiveContactInput {
                source: RetailRuntimeValue::Known(HiveSourceSnapshot {
                    id: HIVE_ID,
                    state_flags_raw: Some(0),
                }),
                target: RetailRuntimeValue::Known(Some(target)),
                impact: RetailRuntimeValue::Unresolved,
            }),
            Err(EntityPairCallbackUnresolved::HiveImpactInputs)
        );
    }

    #[test]
    fn capture_prefix_only_dispatches_death_when_capacity_is_full() {
        let eligible = entity(9, CONSUMABLE_TARGET_CAPABILITIES);
        assert_eq!(
            plan_capture_people_contact(
                RetailRuntimeValue::Known(Some(eligible)),
                RetailRuntimeValue::Known(true),
            ),
            Ok(EntityPairCallbackPlan::null(vec![
                EntityPairCallbackAction::DispatchEntityDeath {
                    entity_id: eligible.id,
                },
            ]))
        );
        assert_eq!(
            plan_capture_people_contact(
                RetailRuntimeValue::Known(Some(eligible)),
                RetailRuntimeValue::Known(false),
            ),
            Err(EntityPairCallbackUnresolved::CaptureAttachmentTransaction)
        );
        assert_eq!(
            plan_capture_people_contact(
                RetailRuntimeValue::Known(Some(eligible)),
                RetailRuntimeValue::Unresolved,
            ),
            Err(EntityPairCallbackUnresolved::CaptureCapacity)
        );

        for ineligible in [
            PairCallbackEntitySnapshot {
                capability_flags_raw: 0,
                ..eligible
            },
            PairCallbackEntitySnapshot {
                state_flags_raw: 0,
                ..eligible
            },
            PairCallbackEntitySnapshot {
                state_flags_raw: CALLBACK_DYING_STATE_BIT,
                ..eligible
            },
        ] {
            assert_eq!(
                plan_capture_people_contact(
                    RetailRuntimeValue::Known(Some(ineligible)),
                    RetailRuntimeValue::Unresolved,
                ),
                Ok(EntityPairCallbackPlan::null(Vec::new()))
            );
        }
    }

    #[test]
    fn power_up_prefix_distinguishes_unsupported_and_missing_handler() {
        let unsupported = entity(46, 0);
        let tagged = EntityPairCallbackPlan::tagged(
            Vec::new(),
            EntityPairTaggedEffect::PowerUpUnsupportedRecipient,
        );
        assert_eq!(
            plan_power_up_contact(
                RetailRuntimeValue::Known(Some(unsupported)),
                RetailRuntimeValue::Unresolved,
            ),
            Ok(tagged.clone())
        );
        assert_eq!(
            plan_power_up_contact(
                RetailRuntimeValue::Known(None),
                RetailRuntimeValue::Unresolved,
            ),
            Ok(tagged)
        );

        let supported = entity(46, POWER_UP_RECIPIENT_CAPABILITY);
        assert_eq!(
            plan_power_up_contact(
                RetailRuntimeValue::Known(Some(supported)),
                RetailRuntimeValue::Known(false),
            ),
            Ok(EntityPairCallbackPlan::null(Vec::new()))
        );
        assert_eq!(
            plan_power_up_contact(
                RetailRuntimeValue::Known(Some(supported)),
                RetailRuntimeValue::Known(true),
            ),
            Err(EntityPairCallbackUnresolved::PowerUpInventoryHandler)
        );
        assert_eq!(
            plan_power_up_contact(
                RetailRuntimeValue::Known(Some(supported)),
                RetailRuntimeValue::Unresolved,
            ),
            Err(EntityPairCallbackUnresolved::PowerUpHandlerAvailability)
        );
        assert_eq!(
            EntityPairTaggedEffect::PowerUpUnsupportedRecipient.retail_object_address(),
            0x004B_EAA8
        );
    }

    #[test]
    fn component_contact_keeps_ignored_return_separate_from_behavior_returns() {
        assert_eq!(
            plan_descriptor_component_contact(RetailRuntimeValue::Known(false)),
            Ok(ComponentPairCallbackPlan {
                actions: Vec::new(),
            })
        );
        assert_eq!(
            plan_descriptor_component_contact(RetailRuntimeValue::Known(true)),
            Err(ComponentPairCallbackUnresolved::DescriptorContact)
        );
        assert_eq!(
            plan_descriptor_component_contact(RetailRuntimeValue::Unresolved),
            Err(ComponentPairCallbackUnresolved::ForwardHalfSpace)
        );
    }
}
