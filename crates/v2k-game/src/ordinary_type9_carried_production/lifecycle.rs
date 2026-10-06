//! Carried Type9's complete `162B0 -> 16750/CE90 -> 10C10` boundary.
//!
//! CE90 really selects and initializes a living graph before 10C10 replaces
//! it with class14. Its selector/constructor RNG and presentation cannot be
//! bypassed merely because the intermediate graph never gets a task visit.

use super::*;
use crate::{
    intro2_common_dying::Intro2CommonDyingBlock,
    main_base_type9_abort::LEVEL_ONE_TYPE9_DEATH_SOUND_ID,
    ordinary_type9_cargo::{
        commit_release, prepare_release, Type9CargoReleasePosition, Type9CargoReleaseRequest,
    },
    ordinary_type9_go_to_job_initializer::OrdinaryType9GoToJobCandidateEvidence,
    ordinary_type9_outer_tail::outer_tail_class14_exploding_task_lease,
    ordinary_type9_root_reselection::OrdinaryType9RootEntityRef,
    ordinary_type9_standard_death::OrdinaryType9StandardDeathEntry,
};

pub(super) struct CarriedSurfaceExpiry {
    candidates: Vec<OrdinaryType9RootEntityRef>,
    jobs: Vec<OrdinaryType9GoToJobCandidateEvidence>,
}

impl CarriedSurfaceExpiry {
    pub(super) fn request<'a>(
        &'a self,
        metadata: &'a EntityTypeRuntimeMetadata,
    ) -> Type9CargoReleaseRequest<'a> {
        Type9CargoReleaseRequest {
            metadata,
            candidates: &self.candidates,
            job_evidence: &self.jobs,
            position: Type9CargoReleasePosition::Retained,
        }
    }

    /// Resolve every query before the scheduler can draw or publish a prefix.
    /// Only this child's None/Sub-I, basis, velocity and timer change before
    /// CE90; prepare_release replaces its own candidate with the released
    /// state, so these immutable other-actor inputs remain valid at commit.
    pub(super) fn prepare(
        manager: &EntityManager,
        owner: &Type9CarriedOwner,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Option<Self> {
        if manager
            .pending_actor_deferred_destroy_ids()
            .contains(&owner.entity_id())
        {
            return None;
        }
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == owner.entity_id())?;
        // The 10C10 owner validates this immutable cue after CE90. Admit its
        // exact known/unresolved matrix here so a foreign cue cannot first
        // consume release RNG and carrying custody before death rejects it.
        let death_sound_matches = match entity.collision.death_sound_id {
            RetailRuntimeValue::Known(Some(sound_id)) => sound_id == LEVEL_ONE_TYPE9_DEATH_SOUND_ID,
            RetailRuntimeValue::Unresolved => {
                metadata.death_sound_id
                    == RetailRuntimeValue::Known(Some(LEVEL_ONE_TYPE9_DEATH_SOUND_ID))
            }
            RetailRuntimeValue::Known(None) => false,
        };
        if !death_sound_matches {
            return None;
        }
        let entities = manager
            .retail_live_order_ids()
            .map(|id| {
                manager
                    .iter_all()
                    .find(|entity| entity.id == id)
                    .expect("live-list entity")
            })
            .collect::<Vec<_>>();
        let expiry = Self {
            candidates: entities
                .iter()
                .map(|entity| OrdinaryType9RootEntityRef {
                    id: entity.id,
                    entity_type: entity.entity_type,
                    position_raw: entity.position_raw(),
                    state_flags_raw: entity.collision.state_flags_at_0x08,
                    capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
                    attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
                })
                .collect(),
            jobs: entities
                .iter()
                .map(|entity| OrdinaryType9GoToJobCandidateEvidence {
                    candidate_id: entity.id,
                    state_flags: entity.collision.state_flags_at_0x08,
                    capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
                    capacity: match entity.base_factory_runtime {
                        RetailRuntimeValue::Known(Some(state)) => {
                            RetailRuntimeValue::Known(Some(crate::job_nearby::JobCapacityState {
                                current_jobs_raw: i32::from(state.current_scientists),
                                capacity_raw: i32::from(state.required_scientists),
                            }))
                        }
                        RetailRuntimeValue::Known(None) => RetailRuntimeValue::Known(None),
                        RetailRuntimeValue::Unresolved => RetailRuntimeValue::Unresolved,
                    },
                })
                .collect(),
        };
        prepare_release(entity, owner, &expiry.request(metadata)).ok()?;
        Some(expiry)
    }

    pub(super) fn commit(
        self,
        manager: &mut EntityManager,
        owner: Type9CarriedOwner,
        metadata: &EntityTypeRuntimeMetadata,
        world_fx: &mut WorldFx,
        notifications: &mut GameplayNotifications,
        retail_tick: u32,
    ) -> Result<Option<MainBaseType9ExplodingTaskLease>, Intro2CommonDyingBlock> {
        let entity_id = owner.entity_id();
        let actor = owner.actor;
        let entity = manager
            .ordinary_type9_selected_entity_mut(entity_id)
            .expect("synchronous carrying callback retains its allocation");
        let request = self.request(metadata);
        let prepared = prepare_release(entity, &owner, &request)
            .expect("None/Sub-I and E100 preserve every preflighted release input");
        let mut sounds = Vec::new();
        let _temporary_living_publication = commit_release(
            entity,
            owner,
            prepared,
            request,
            || u32::from(world_fx.next_shared_retail_random_u16()),
            |text| {
                notifications
                    .queue_attract_attention_resource_text(text, retail_tick as i32)
                    .expect("canonical Type9 resource receipt");
            },
            |sound| sounds.push(sound),
        )
        .expect("preflighted CE90 initializes the actual weighted root");
        for sound in sounds {
            world_fx.queue_fixed_positional_sound_raw(sound.global_sound_id, sound.position_raw);
        }
        // 16750 has now completed, including its style callback. Calling the
        // generic 10C10 entry avoids applying that release prefix a second time.
        manager
            .publish_ordinary_type9_standard_death(
                entity_id,
                OrdinaryType9StandardDeathEntry::GenericDeath,
                MainBaseType9ResultScreenState::NotShown,
                world_fx,
                retail_tick as i32,
                Some(notifications),
            )
            .map_err(|_| Intro2CommonDyingBlock::Runtime("carried Type9 standard death"))?;
        Ok(outer_tail_class14_exploding_task_lease(manager, actor))
    }
}
