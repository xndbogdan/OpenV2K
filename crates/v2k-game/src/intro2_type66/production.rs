//! World-independent side effects of the receipt-bound `19010` child machine.
//!
//! Constructor refusal is retained as a blocked prefix. The existing adapters
//! do not report a retail allocation failure separately from missing runtime
//! prerequisites, so `None` must not masquerade as a successful failed attempt.

use super::{Intro2Type66Block as Block, Intro2Type66Frame};
use crate::{
    entity::EntityManager,
    entity_collision_state::RetailRuntimeValue,
    factory_production::{
        FACTORY_HIGH_THRESHOLD_DIRECT_TEXT_ID, FACTORY_MATERIALISER_ENTITY_TYPE,
        FACTORY_OUTPUT_CONVERSION_HUD_RESOURCE_ID, FACTORY_OUTPUT_CONVERSION_SOUND_ID,
        FACTORY_PICKUP_ENTITY_TYPE,
    },
    factory_production_live::{
        FactoryEntitySpawnResult, FactoryProductionAction as Action,
        FactoryProductionActionPhase as Phase, FactoryProductionEntityVersion, FactorySpawnRole,
        FACTORY_ENTITY_LIFETIME_OFFSET,
    },
    factory_production_owner::{FactoryProductionOwnerActionPhase, FactoryProductionOwnerResume},
};

pub(super) fn apply_action(
    manager: &mut EntityManager,
    frame: &mut Intro2Type66Frame<'_>,
    factory: FactoryProductionEntityVersion,
    action: Action,
) -> Result<FactoryProductionOwnerResume, Block> {
    let phase = FactoryProductionOwnerActionPhase::Production(action.phase());
    let acknowledged = FactoryProductionOwnerResume::Acknowledged { phase };
    match action {
        Action::SpawnEntity {
            phase: Phase::SpawnPickup,
            role: FactorySpawnRole::Pickup,
            request,
        } if request.entity_type == FACTORY_PICKUP_ENTITY_TYPE => {
            let spawned = manager
                .append_factory_pickup(factory, request, frame.world_fx)
                .ok_or(Block::Runtime("pickup constructor prerequisites"))?;
            Ok(FactoryProductionOwnerResume::SpawnCompleted {
                phase,
                result: FactoryEntitySpawnResult::Spawned(spawned),
            })
        }
        Action::SpawnEntity {
            phase: Phase::SpawnConvertedOutput,
            role: FactorySpawnRole::ConvertedOutput,
            request,
        } => {
            let terrain = frame
                .resources
                .terrain()
                .ok_or(Block::Runtime("worker terrain"))?;
            let spawned = manager
                .append_factory_converted_output(
                    factory,
                    request,
                    frame.retail_tick,
                    terrain,
                    frame.world_fx,
                )
                .ok_or(Block::Runtime("converted worker constructor prerequisites"))?;
            // BA40 may publish event16 during a diver's constructor. The
            // native factory owns this live phase, before the Type93 append.
            frame
                .notifications
                .drain_attract_attention_receipts(manager, frame.retail_tick as i32)
                .map_err(|_| Block::Runtime("converted worker notification receipt"))?;
            Ok(FactoryProductionOwnerResume::SpawnCompleted {
                phase,
                result: FactoryEntitySpawnResult::Spawned(spawned),
            })
        }
        Action::SpawnEntity {
            phase: Phase::SpawnMaterialiser,
            role: FactorySpawnRole::Materialiser,
            request,
        } if request.entity_type == FACTORY_MATERIALISER_ENTITY_TYPE => {
            let terrain = frame
                .resources
                .terrain()
                .ok_or(Block::Runtime("materialiser terrain"))?;
            let spawned = manager
                .append_factory_output_materialiser(factory, request, terrain, frame.world_fx)
                .ok_or(Block::Runtime("materialiser constructor prerequisites"))?;
            Ok(FactoryProductionOwnerResume::SpawnCompleted {
                phase,
                result: FactoryEntitySpawnResult::Spawned(spawned),
            })
        }
        Action::LinkOwner {
            phase: Phase::LinkPickupOwner,
            spawned,
            owner_factory,
        } if owner_factory == factory => {
            // The source ignores a fresh-child lookup miss and continues.
            manager.link_factory_pickup_owner(spawned, owner_factory);
            Ok(acknowledged)
        }
        Action::LinkOwner {
            phase: Phase::LinkConvertedOutputOwner,
            spawned,
            owner_factory,
        } if owner_factory == factory => {
            manager.link_factory_converted_output_owner(spawned, owner_factory);
            Ok(acknowledged)
        }
        Action::QueryPickupPresence {
            phase: Phase::QueryPickupPresence,
            pickup_handle,
        } => {
            let presence = crate::factory_activation_live::live_factory_pickup_presence(
                manager,
                pickup_handle,
            )
            .map_err(|_| Block::Runtime("pickup presence"))?;
            Ok(FactoryProductionOwnerResume::PickupPresence { phase, presence })
        }
        Action::QueueHudResource {
            phase: Phase::QueueConversionHudResource,
            resource_id: FACTORY_OUTPUT_CONVERSION_HUD_RESOURCE_ID,
        } => {
            frame
                .notifications
                .queue_factory_output_conversion(frame.retail_tick as i32);
            Ok(acknowledged)
        }
        Action::LinkMaterialiserOutput {
            phase: Phase::LinkMaterialiserOutput,
            materialiser,
            output,
        } => {
            // 19010 disposes 08F00's return and proceeds to sound even on miss.
            manager.link_factory_materialiser_output(materialiser, output, frame.world_fx);
            Ok(acknowledged)
        }
        Action::PlayConversionPositionalSound {
            phase: Phase::PlayConversionPositionalSound,
            request,
        } if request.sound_id == FACTORY_OUTPUT_CONVERSION_SOUND_ID
            && request.gain_raw_16_16 == 0x1_0000
            && request.rate_raw_16_16 == 0x1_0000 =>
        {
            frame.world_fx.queue_fixed_positional_sound_raw_at_rate(
                request.sound_id,
                request.position_raw,
                request.rate_raw_16_16 as u32,
            );
            Ok(acknowledged)
        }
        Action::EmitDirectText {
            direct_text_id: FACTORY_HIGH_THRESHOLD_DIRECT_TEXT_ID,
            sub_parameter: 0,
            ..
        } => {
            frame
                .notifications
                .queue_factory_product_ready(frame.retail_tick as i32);
            Ok(acknowledged)
        }
        Action::ClearFactoryLifetime {
            factory: requested,
            lifetime_offset: FACTORY_ENTITY_LIFETIME_OFFSET,
            value_raw: 0,
            ..
        } if requested == factory => {
            let entity = manager
                .entity_mut(factory.entity_id)
                .ok_or(Block::Allocation)?;
            if !entity.factory_owner_version_matches(factory) {
                return Err(Block::Runtime("factory lifetime receipt"));
            }
            let RetailRuntimeValue::Known(Some(base)) = &mut entity.base_factory_runtime else {
                return Err(Block::Runtime("factory lifetime Sub-M"));
            };
            base.live_owner
                .as_mut()
                .ok_or(Block::Runtime("factory lifetime owner"))?
                .lifetime_at_0x74_raw = 0;
            Ok(acknowledged)
        }
        _ => Err(Block::UnsupportedProduction(phase)),
    }
}
