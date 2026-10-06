//! 443B50: player Sub-J append,16700 callback, then collection success suffix.
//!
//! Type93's08F00 calls16700 directly and deliberately does not use this owner.

use super::*;

pub struct PlayerCargoAttachmentFrame<'a> {
    pub scheduler: &'a mut SpecializedActorTaskScheduler,
    pub world_fx: &'a mut WorldFx,
    pub notifications: &'a mut GameplayNotifications,
    pub retail_tick: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerCargoAttachmentError {
    PlayerUnavailable,
    ChildUnavailable,
    SlotsUnavailable,
    CargoFull,
    UnsupportedCallback {
        entity_type: u32,
    },
    AllocationUnavailable,
    /// A retained, unfinished host callback cannot be entered a second time.
    PendingCallback,
    /// The source has already published18440 and16700 when this is true.
    CallbackUnavailable {
        reason: String,
        prefix_committed: bool,
    },
}

impl EntityManager {
    /// Run443B30/443B50 for a selected native weight. Identity selection and
    /// a possible zero-record birth belong to the caller; neither is retried
    /// when the selected body's callback is unavailable.
    pub fn attach_player_cargo(
        &mut self,
        cargo: u32,
        frame: PlayerCargoAttachmentFrame<'_>,
    ) -> Result<(), PlayerCargoAttachmentError> {
        let entity = self
            .iter_all()
            .find(|entity| entity.id == cargo)
            .ok_or(PlayerCargoAttachmentError::ChildUnavailable)?;
        if matches!(entity.entity_type, 49 | 92 | 96 | 97 | 100 | 123)
            && entity.capability_flags & 0x1000 != 0
        {
            let mut callbacks = LiveCargoCallbacks {
                scheduler: frame.scheduler,
                world_fx: frame.world_fx,
                notifications: frame.notifications,
                retail_tick: frame.retail_tick,
                terrain: None,
                blocked: Vec::new(),
            };
            return callbacks.attach_to_player(self, cargo);
        }
        if entity.entity_type != 68 || !self.native_class0_construction_present(cargo) {
            return Err(PlayerCargoAttachmentError::UnsupportedCallback {
                entity_type: entity.entity_type,
            });
        }
        let mut callbacks = LiveCargoCallbacks {
            scheduler: frame.scheduler,
            world_fx: frame.world_fx,
            notifications: frame.notifications,
            retail_tick: frame.retail_tick,
            terrain: None,
            blocked: Vec::new(),
        };
        callbacks.attach_to_player(self, cargo)
    }
}

fn append_error(error: PlayerSubJAppendError) -> PlayerCargoAttachmentError {
    match error {
        PlayerSubJAppendError::Full => PlayerCargoAttachmentError::CargoFull,
        PlayerSubJAppendError::ChildUnavailable => PlayerCargoAttachmentError::ChildUnavailable,
        PlayerSubJAppendError::RuntimeUnavailable
        | PlayerSubJAppendError::DescriptorUnavailable => {
            PlayerCargoAttachmentError::SlotsUnavailable
        }
    }
}

/// Existing8/9 behavior owners and explicitly separate geometry fixtures.
/// Those callbacks are fully prepared before their synchronous source writes.
pub(super) fn attach_with_callbacks<C: CargoRelationCallbacks>(
    manager: &mut EntityManager,
    cargo: u32,
    callbacks: &mut C,
) -> Result<(), PlayerCargoAttachmentError> {
    let parent = manager
        .player_id
        .ok_or(PlayerCargoAttachmentError::PlayerUnavailable)?;
    let append = manager
        .prepare_player_sub_j_append(cargo)
        .map_err(append_error)?;
    let plan = callbacks
        .prepare_attach(manager, cargo, parent)
        .ok_or_else(|| PlayerCargoAttachmentError::CallbackUnavailable {
            reason: "selected cargo callback unavailable".into(),
            prefix_committed: false,
        })?;
    callbacks.commit_attach(manager, plan, |manager| {
        manager.commit_player_sub_j_append(append);
    });
    Ok(())
}

pub(super) fn attach_native_class0(
    manager: &mut EntityManager,
    cargo: u32,
    callbacks: &mut LiveCargoCallbacks<'_>,
) -> Result<(), PlayerCargoAttachmentError> {
    let parent = manager
        .player_id
        .ok_or(PlayerCargoAttachmentError::PlayerUnavailable)?;
    if !manager.native_class0_allocation_authenticates(cargo) {
        return Err(PlayerCargoAttachmentError::AllocationUnavailable);
    }
    // This is host custody admission, not a retail callback failure. A parked
    // prefix must remain byte-for-byte available to its existing owner.
    if manager.native_class0_has_pending_prefix(cargo) {
        return Err(PlayerCargoAttachmentError::PendingCallback);
    }
    let append = manager
        .prepare_player_sub_j_append(cargo)
        .map_err(append_error)?;
    let callback = (|| {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == cargo)
            .unwrap();
        if entity.entity_type != 68 {
            return Err(Class0ActorError::Runtime("player class0 cargo type"));
        }
        super::super::class0_actor::authenticate_metadata(
            68,
            manager
                .type_runtime_metadata(68)
                .ok_or(Class0ActorError::Metadata)?,
        )?;
        let owner = Class0ActorOwner::adopt(manager, cargo)?;
        if !callbacks
            .scheduler
            .prepare_class0_actor_external_mutation(manager, cargo)
        {
            return Err(Class0ActorError::Runtime("class0 scheduler custody"));
        }
        if entity
            .collision
            .state_flags_at_0x08
            .masked(REMOTE_OWNED_STATE_BIT)
            != RetailRuntimeValue::Known(0)
            || !manager.player().is_some_and(|player| {
                player
                    .collision
                    .state_flags_at_0x08
                    .masked(REMOTE_OWNED_STATE_BIT)
                    == RetailRuntimeValue::Known(0)
            })
        {
            return Err(Class0ActorError::Runtime("local class0 relation"));
        }
        Ok(owner)
    })();
    //18440 checks capacity before duplicate identity.16700 then unconditionally
    // writes1000/+80, even for an already attached first identity match.
    manager.commit_player_sub_j_append(append);
    project_relation_attach(manager.entity_mut(cargo).unwrap(), parent);
    let owner = callback.map_err(|reason| PlayerCargoAttachmentError::CallbackUnavailable {
        reason: format!("{reason:?}"),
        prefix_committed: true,
    })?;
    // DBF0 style4C7468+08 is null: no timer replacement, age reset or RNG draw.
    debug_assert_eq!(Class0ActorOwner::adopt(manager, cargo), Ok(owner));
    Ok(())
}

pub(super) fn commit_success_suffix(
    manager: &mut EntityManager,
    cargo: u32,
    callbacks: &mut LiveCargoCallbacks<'_>,
) {
    let child = manager
        .iter_all()
        .find(|entity| entity.id == cargo)
        .unwrap();
    if child.capability_flags & 0x800 != 0 {
        callbacks
            .notifications
            .queue_cargo_collected(callbacks.retail_tick as i32);
    }
    if child.entity_type == 68 {
        callbacks
            .notifications
            .queue_weight_collected(callbacks.retail_tick as i32);
    }
    manager
        .entity_mut(cargo)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(0x40000, 0);
    let parent = manager.player().expect("443B50 parent");
    if parent.capability_flags & 1 != 0 {
        let parent_id = parent.id;
        let parent_type = parent.entity_type as u8;
        let child = manager.entity_mut(cargo).unwrap();
        child.collision.state_flags_at_0x08.overwrite(0x2000, 0);
        callbacks.world_fx.queue_deferred_cargo_transfer(
            child.position_raw(),
            parent_id,
            parent_type,
        );
    }
}

#[cfg(test)]
#[path = "cargo_attachment_tests.rs"]
mod tests;
