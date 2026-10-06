//! Type6's resource-independent first10C10 ->19750 ->AC60 ->25730 entry.
use super::*;
use crate::{
    base_factory_progression::{ProgressiveDeathBegin, PROGRESSION_REVIVE_HEALTH_RAW},
    entity_collision_state::{DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT},
    world_fx::WorldFx,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseDeathBlock {
    pub reason: MainBaseError,
    pub committed_prefix: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseDeathOutcome {
    pub returned_nonzero: bool,
    pub owner: Option<MainBaseOwner>,
    pub selector_word: Option<u16>,
}

pub fn publish_main_base_standard_death(
    manager: &mut EntityManager,
    entity_id: u32,
    world_fx: &mut WorldFx,
) -> Result<MainBaseDeathOutcome, MainBaseDeathBlock> {
    let block = |reason, committed_prefix| MainBaseDeathBlock {
        reason,
        committed_prefix,
    };
    if !main_base_manager_allocation_authenticates(manager, entity_id) {
        return Err(block(MainBaseError::Identity, false));
    }
    let entity = manager
        .entity_mut(entity_id)
        .ok_or(block(MainBaseError::Identity, false))?;
    let RetailRuntimeValue::Known(remote) = entity
        .collision
        .state_flags_at_0x08
        .masked(REMOTE_OWNED_STATE_BIT)
    else {
        return Err(block(MainBaseError::Runtime("remote ownership"), false));
    };
    if remote != 0 {
        return Ok(MainBaseDeathOutcome {
            returned_nonzero: false,
            owner: None,
            selector_word: None,
        });
    }
    let RetailRuntimeValue::Known(dying) =
        entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT)
    else {
        return Err(block(MainBaseError::Runtime("dying selector"), false));
    };
    if dying != 0 {
        return Ok(MainBaseDeathOutcome {
            returned_nonzero: true,
            owner: None,
            selector_word: None,
        });
    }
    //10C68/10C6B are committed before sound/attachment/context consumers.
    entity.collision.health_raw = RetailRuntimeValue::Known(0);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
    if entity.collision.death_sound_id != RetailRuntimeValue::Known(None)
        || entity.collision.constructor_sound_attachment_id_at_0x8c
            != RetailRuntimeValue::Known(None)
    {
        return Err(block(
            MainBaseError::Runtime("death sound/attachment"),
            true,
        ));
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(block(MainBaseError::ComponentStorage, true));
    };
    if context.active_style().style_address() != 0x004C_9480 {
        return Err(block(MainBaseError::ComponentStorage, true));
    }
    let RetailRuntimeValue::Known(Some(base)) = entity.base_factory_runtime else {
        return Err(block(MainBaseError::ComponentStorage, true));
    };
    let ProgressiveDeathBegin::Revived { state } = base.progressive_death.begin() else {
        return Err(block(
            MainBaseError::Runtime("terminal callback requires world resources"),
            true,
        ));
    };
    let metadata = manager
        .type_runtime_metadata(6)
        .ok_or(block(MainBaseError::Metadata, true))?;
    super::native::authenticate_metadata(metadata).map_err(|error| block(error, true))?;
    let entity = manager.entity_mut(entity_id).unwrap();
    entity.collision.health_raw = RetailRuntimeValue::Known(PROGRESSION_REVIVE_HEALTH_RAW);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, 0);
    let RetailRuntimeValue::Known(Some(base)) = &mut entity.base_factory_runtime else {
        unreachable!()
    };
    base.progressive_death = state;
    let publication = super::native::reselect_main_base(entity, &mut || {
        u32::from(world_fx.next_shared_retail_random_u16())
    })
    .map_err(|error| block(error, true))?;
    let owner = MainBaseOwner::adopt(manager, entity_id).map_err(|error| block(error, true))?;
    Ok(MainBaseDeathOutcome {
        returned_nonzero: false,
        owner: Some(owner),
        selector_word: Some(publication.selector_word as u16),
    })
}
