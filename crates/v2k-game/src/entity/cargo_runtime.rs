//! Native cargo relation callbacks at the player and late materialiser phases.

use super::*;
use crate::gameplay_notifications::GameplayNotifications;
use crate::intro2_type8::NativeWorkerProfile;
use crate::native_type86::NativeFourChoiceProfile;
use crate::ordinary_type9_cargo::{
    self, Type9CargoAttachPlan, Type9CargoReleasePosition, Type9CargoReleasePrepared,
    Type9CargoReleaseRequest, Type9RelationOwner,
};
use crate::ordinary_type9_current_task::OrdinaryType9CurrentTaskAuthority;
use crate::ordinary_type9_go_to_job_initializer::OrdinaryType9GoToJobCandidateEvidence;
use crate::ordinary_type9_root_reselection::OrdinaryType9RootEntityRef;
use crate::specialized_actor_task_production::SpecializedActorTaskScheduler;

#[path = "cargo_attachment.rs"]
mod attachment;
pub use attachment::{PlayerCargoAttachmentError, PlayerCargoAttachmentFrame};

/// One early player-controller phase, before the ordinary actor scheduler.
/// Beam-out constructs its Type93 here; its callback and Sub-J pose write run
/// later through [`EntityManager::update_late_tail_materialisers`].
pub struct PlayerCargoFrame<'a> {
    pub elapsed_micros: u32,
    pub drop_context: Option<CargoDropContext<'a>>,
    pub retail_tick: u32,
    pub scheduler: &'a mut SpecializedActorTaskScheduler,
    pub world_fx: &'a mut WorldFx,
    pub notifications: &'a mut GameplayNotifications,
}

#[derive(Debug)]
pub struct PlayerCargoFrameOutcome {
    pub beam: Option<BeamOutcome>,
    /// Unsupported callbacks stop before relation/task writes. Keep the
    /// identity and diagnostic available instead of silently freezing cargo.
    pub blocked: Vec<(u32, String)>,
}

/// Tail-appended Type93 callbacks after actor task visits, including player
/// drops. Factory children retain their own scheduler authority across
/// `16750`; Type17 missing-relation release keeps its proven null path.
pub struct LateTailMaterialiserFrame<'a> {
    pub elapsed_micros: u32,
    pub terrain: &'a TerrainGrid,
    pub world_fx: &'a mut WorldFx,
    pub scheduler: &'a mut SpecializedActorTaskScheduler,
    pub notifications: &'a mut GameplayNotifications,
    pub retail_tick: u32,
}

impl EntityManager {
    pub fn update_player_cargo(&mut self, frame: PlayerCargoFrame<'_>) -> PlayerCargoFrameOutcome {
        let mut callbacks = LiveCargoCallbacks {
            scheduler: frame.scheduler,
            world_fx: frame.world_fx,
            notifications: frame.notifications,
            retail_tick: frame.retail_tick,
            terrain: frame.drop_context.map(|context| context.terrain),
            blocked: Vec::new(),
        };
        let beam =
            self.tick_beam_with_callbacks(frame.elapsed_micros, frame.drop_context, &mut callbacks);
        PlayerCargoFrameOutcome {
            beam,
            blocked: callbacks.blocked,
        }
    }

    /// Advance every tail-appended materialiser after actor tasks. The child
    /// receives its carrying callback before Type93 publishes its new pose or
    /// releases it. A blocked release retains its Sub-J row and child graph;
    /// the returned identity/reason keeps the custody failure visible.
    pub fn update_late_tail_materialisers(
        &mut self,
        frame: LateTailMaterialiserFrame<'_>,
    ) -> Vec<(u32, String)> {
        let mut callbacks = LiveCargoCallbacks {
            scheduler: frame.scheduler,
            world_fx: frame.world_fx,
            notifications: frame.notifications,
            retail_tick: frame.retail_tick,
            terrain: Some(frame.terrain),
            blocked: Vec::new(),
        };
        self.update_type93_materialisers_with_callbacks(
            frame.elapsed_micros,
            frame.terrain,
            Type93UpdatePhase::LateTailMaterialiser,
            &mut callbacks,
        );
        callbacks.blocked
    }
}

/// The beam geometry owns Sub-J publication. Its callback owner prepares
/// immutable reads first, then commits exactly at 16700/16750, without an
/// intervening actor update or shared-RNG consumer.
pub(super) trait CargoRelationCallbacks {
    type AttachPlan;
    type ReleasePlan;
    fn attach_to_player(
        &mut self,
        manager: &mut EntityManager,
        cargo: u32,
    ) -> Result<(), PlayerCargoAttachmentError>
    where
        Self: Sized,
    {
        attachment::attach_with_callbacks(manager, cargo, self)
    }
    fn spawn_materialiser(
        &mut self,
        manager: &mut EntityManager,
        cargo: u32,
        target: [i16; 3],
        terrain: &TerrainGrid,
    ) -> Option<Type93MaterialiserSpawn>;
    fn prepare_attach(
        &mut self,
        manager: &EntityManager,
        cargo: u32,
        parent: u32,
    ) -> Option<Self::AttachPlan>;
    /// Consume selected task custody before the authenticated Sub-J prefix
    /// changes child state. The prefix still runs before 16700's relation
    /// writes and its behavior callback, as in 18440 -> 16700.
    fn commit_attach(
        &mut self,
        manager: &mut EntityManager,
        plan: Self::AttachPlan,
        publish_sub_j: impl FnOnce(&mut EntityManager),
    );
    fn prepare_release(
        &mut self,
        manager: &EntityManager,
        cargo: u32,
        parent: u32,
        position: Type9CargoReleasePosition,
    ) -> Option<Self::ReleasePlan>;
    fn commit_release(&mut self, manager: &mut EntityManager, plan: Self::ReleasePlan);
    fn report_materialiser_block(&mut self, cargo: u32, reason: &'static str);
}

/// Existing low-level beam/contact fixtures exercise attachment geometry
/// separately from the production scheduler and its behavior authority.
pub(super) struct AttachmentProjectionCallbacks;

impl CargoRelationCallbacks for AttachmentProjectionCallbacks {
    fn report_materialiser_block(&mut self, _cargo: u32, _reason: &'static str) {
        // Geometry fixtures do not enter native Sub-J pose admission.
    }

    fn spawn_materialiser(
        &mut self,
        manager: &mut EntityManager,
        cargo_id: u32,
        target_raw: [i16; 3],
        _terrain: &TerrainGrid,
    ) -> Option<Type93MaterialiserSpawn> {
        let proxy_id = manager.allocate_live_entity_id();

        manager.append_live_entity(Entity {
            id: proxy_id,
            // This low-level geometry fixture is not a104B0 allocation.
            construction_stamp_at_0xb4: RetailRuntimeValue::Unresolved,
            authored_spawn_index: None,
            kind: EntityKind::Unknown(CARGO_DROP_PROXY_ENTITY_TYPE),
            entity_type: CARGO_DROP_PROXY_ENTITY_TYPE,
            authored_follow_beacon_priority_raw: None,
            power_up_payload_packed: None,
            factory_type61_birth_provenance: None,
            type60_construction_provenance: None,
            main_base_type54_sea_delta_source: RetailRuntimeValue::Unresolved,
            position: raw_position_world(target_raw),
            heading: 0.0,
            pitch_roll_raw: [0; 2],
            physical_body_basis_q31: RetailRuntimeValue::Unresolved,
            velocity: [0.0; 3],
            surface_lifetime_timer_ms_at_0x48: RetailRuntimeValue::Known(0),
            // Type 93's exact Section-12 +0x04 mass.
            mass_raw: 1,
            capability_flags: 0,
            attached_to: None,
            model_slots: [None; 4],
            model_index: None,
            collision: EntityCollisionRuntimeState::unresolved_port_entity(0),
            initial_behavior: RetailRuntimeValue::Unresolved,
            current_behavior_context: RetailRuntimeValue::Unresolved,
            authored_radial_emitter: None,
            sub_n_runtime: RetailRuntimeValue::Unresolved,
            base_factory_runtime: RetailRuntimeValue::Unresolved,
            actor_animation_runtime: RetailRuntimeValue::Unresolved,
            sub_a_propulsion_runtime: RetailRuntimeValue::Unresolved,
            sub_g_06070_runtime: RetailRuntimeValue::Unresolved,
            intro2_type13_common_mover_runtime: None,
            native_type13_allocation: None,
            intro2_type13_aim_runtime: None,
            intro2_type16_aim_runtime: None,
            intro2_type58_aim_runtime: None,
            intro2_type94_aim_runtime: None,
            intro2_flyer_aim_runtime: None,
            sub_h_external_frame_runtime: RetailRuntimeValue::Unresolved,
            // The geometry fixture needs the one zero-policy attachment row.
            // Production uses the authenticated Type-93 constructor below.
            sub_j_attachment_runtime: RetailRuntimeValue::Known(Some(type93_attachment_runtime(
                cargo_id,
            ))),
            actor_common_axis_descriptor: RetailRuntimeValue::Unresolved,
            actor_tasks: ActorTaskOwner::new(),
            ordinary_type9_pending_initial_selection: None,
            ordinary_type9_selected_component_runtime: None,
            main_base_type9_death_component_runtime: None,
            ordinary_type47_aim_and_fire_runtime: None,
            native_type61_allocation: None,
            native_type26_allocation: None,
            intro2_type26_sub_d_frame_owner: None,
            intro2_type26_sub_d_runtime: None,
            intro2_type47_sub_d_frame_owner: None,
            intro2_type47_sub_d_runtime: None,
            native_type47_construction: None,
            intro2_flyer_frame_owner: None,
            intro2_type53_runtime: None,
            native_type122_runtime: None,
            native_type30_runtime: None,
            native_type30_aim_runtime: None,
            native_type40_runtime: None,
            native_type40_aim_runtime: None,
            native_type56_runtime: None,
            native_type56_aim_runtime: None,
            native_type122_aim_runtime: None,
            shared_fish_runtime: None,
            cleansing_vehicle_runtime: None,
            intro2_type16_runtime: None,
            intro2_type58_runtime: None,
            intro2_type66_runtime: None,
            intro2_gun_turret_runtime: None,
            intro2_gun_turret_aim_runtime: None,
            class49_death_runtime: None,
            native_entity_weapon_runtime: None,
            intro2_type10_runtime: None,
            intro2_type10_aim_runtime: None,
            intro2_type57_runtime: None,
            intro2_type57_aim_runtime: None,
            intro2_type94_runtime: None,
            intro2_type17_runtime: None,
            native_capture_relation: None,
            intro2_type8_runtime: None,
            native_type123_runtime: None,
            native_type123_anchor_raw_at_0x90: RetailRuntimeValue::Unresolved,
            native_type86_runtime: None,
            native_type86_anchor_raw_at_0x90: RetailRuntimeValue::Unresolved,
            intro2_type9_runtime: None,
            ordinary_type9_native_receipt: None,
            main_base_runtime: None,
            type17_sub_d_frame_owner: None,
            type17_sub_d_runtime: None,
            type8_sub_d_frame_owner: None,
            type8_wander_anchor_raw_at_0x90: RetailRuntimeValue::Unresolved,
            type8_sub_d_runtime: None,
            type47_immutable_anchor_raw_at_0x90: RetailRuntimeValue::Unresolved,
            active: true,
        });
        // The fixture needs the local ownership bit for 12DA0's relation prelude.
        manager
            .entity_mut(proxy_id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(REMOTE_OWNED_STATE_BIT, 0);
        // 443D30 constructs Type 93 before releasing the old player relation.
        // The fresh proxy owns no child until the subsequent 08F00 call.
        if let RetailRuntimeValue::Known(Some(runtime)) = &mut manager
            .entity_mut(proxy_id)
            .unwrap()
            .sub_j_attachment_runtime
        {
            runtime.clear();
        }
        Some(Type93MaterialiserSpawn {
            entity_id: proxy_id,
            construction: Type93MaterialiserConstruction::AttachmentProjection,
        })
    }

    type AttachPlan = (u32, u32);
    type ReleasePlan = (u32, Type9CargoReleasePosition);
    fn prepare_attach(
        &mut self,
        _: &EntityManager,
        cargo: u32,
        parent: u32,
    ) -> Option<Self::AttachPlan> {
        Some((cargo, parent))
    }
    fn commit_attach(
        &mut self,
        manager: &mut EntityManager,
        (cargo, parent): Self::AttachPlan,
        publish_sub_j: impl FnOnce(&mut EntityManager),
    ) {
        publish_sub_j(manager);
        project_relation_attach(manager.entity_mut(cargo).expect("prepared cargo"), parent);
    }
    fn prepare_release(
        &mut self,
        _: &EntityManager,
        cargo: u32,
        _: u32,
        position: Type9CargoReleasePosition,
    ) -> Option<Self::ReleasePlan> {
        Some((cargo, position))
    }
    fn commit_release(
        &mut self,
        manager: &mut EntityManager,
        (cargo, position): Self::ReleasePlan,
    ) {
        if let Type9CargoReleasePosition::Materialiser(raw) = position {
            let entity = manager.entity_mut(cargo).expect("prepared cargo");
            entity.position = raw_position_world(raw);
            if entity.entity_type == SCIENTIST_ENTITY_TYPE {
                entity.type8_wander_anchor_raw_at_0x90 = RetailRuntimeValue::Known(raw);
            }
            project_relation_release_attachment(entity);
        }
    }
}

struct LiveCargoCallbacks<'a> {
    scheduler: &'a mut SpecializedActorTaskScheduler,
    world_fx: &'a mut WorldFx,
    notifications: &'a mut GameplayNotifications,
    retail_tick: u32,
    terrain: Option<&'a TerrainGrid>,
    blocked: Vec<(u32, String)>,
}

enum CargoAttachPlan {
    Cleansing(crate::cleansing_vehicle::cargo::AttachPlan),
    Class0(super::class0_actor::cargo::Class0AttachPlan),
    Type8 {
        cargo: u32,
        parent: u32,
        plan: crate::intro2_type8::cargo::Type8AttachPlan,
    },
    Type9 {
        actor: crate::main_base_abort::MainBaseAbortActorLease,
        parent: u32,
        plan: Type9CargoAttachPlan,
    },
    Type123 {
        cargo: u32,
        parent: u32,
        plan: crate::native_type123::cargo::Type123AttachPlan,
    },
    FourChoice {
        cargo: u32,
        parent: u32,
        plan: crate::native_type86::cargo::Type86AttachPlan,
    },
    Other(u32, u32),
}

enum CargoReleasePlan {
    Cleansing(crate::cleansing_vehicle::cargo::ReleasePlan),
    Class0(super::class0_actor::cargo::Class0ReleasePlan),
    Type8 {
        cargo: u32,
        plan: crate::intro2_type8::cargo::Type8ReleasePlan,
    },
    Type9 {
        cargo: u32,
        metadata: EntityTypeRuntimeMetadata,
        candidates: Vec<OrdinaryType9RootEntityRef>,
        jobs: Vec<OrdinaryType9GoToJobCandidateEvidence>,
        position: Type9CargoReleasePosition,
        prepared: Type9CargoReleasePrepared,
    },
    Type123 {
        cargo: u32,
        plan: crate::native_type123::cargo::Type123ReleasePlan,
    },
    FourChoice {
        cargo: u32,
        plan: crate::native_type86::cargo::Type86ReleasePlan,
    },
    Other(u32, Type9CargoReleasePosition),
}

impl LiveCargoCallbacks<'_> {
    fn blocked<T>(&mut self, cargo: u32, reason: impl std::fmt::Debug) -> Option<T> {
        self.blocked.push((cargo, format!("{reason:?}")));
        None
    }
}

impl CargoRelationCallbacks for LiveCargoCallbacks<'_> {
    type AttachPlan = CargoAttachPlan;
    type ReleasePlan = CargoReleasePlan;

    fn report_materialiser_block(&mut self, cargo: u32, reason: &'static str) {
        self.blocked.push((cargo, reason.to_owned()));
    }

    fn attach_to_player(
        &mut self,
        manager: &mut EntityManager,
        cargo: u32,
    ) -> Result<(), PlayerCargoAttachmentError> {
        let result = if manager.native_class0_construction_present(cargo) {
            attachment::attach_native_class0(manager, cargo, self)
        } else {
            attachment::attach_with_callbacks(manager, cargo, self)
        };
        if let Err(reason) = &result {
            self.blocked.push((cargo, format!("{reason:?}")));
        } else {
            attachment::commit_success_suffix(manager, cargo, self);
        }
        result
    }

    fn spawn_materialiser(
        &mut self,
        manager: &mut EntityManager,
        cargo: u32,
        target: [i16; 3],
        terrain: &TerrainGrid,
    ) -> Option<Type93MaterialiserSpawn> {
        // 443D30 sends remote-owned cargo through 469200 instead of the
        // local Type-93 constructor and release callback. Close this before
        // the constructor's shared-RNG word or any allocation publication.
        let entity = manager.iter_all().find(|entity| entity.id == cargo)?;
        let native_class0 = manager.native_class0_construction_present(cargo);
        if (matches!(entity.entity_type, 9 | 49 | 123)
            || NativeWorkerProfile::from_entity_type(entity.entity_type).is_some()
            || NativeFourChoiceProfile::from_entity_type(entity.entity_type).is_some()
            || native_class0)
            && entity
                .collision
                .state_flags_at_0x08
                .masked(REMOTE_OWNED_STATE_BIT)
                != RetailRuntimeValue::Known(0)
        {
            return self.blocked(cargo, "local cargo ownership unavailable");
        }
        if entity.entity_type == 49 {
            if !self
                .scheduler
                .begin_cleansing_vehicle_external_mutation(manager, cargo)
            {
                return self.blocked(
                    cargo,
                    "cleansing task callback is not at a completed boundary",
                );
            }
            let Some(parent) = entity.attached_to else {
                return self.blocked(cargo, "cleansing drop relation unavailable");
            };
            if let Err(reason) = crate::cleansing_vehicle::cargo::prepare_release(
                manager,
                cargo,
                parent,
                Type9CargoReleasePosition::Retained,
                terrain,
            ) {
                return self.blocked(cargo, reason);
            }
        }
        if native_class0 {
            if !self
                .scheduler
                .prepare_class0_actor_external_mutation(manager, cargo)
            {
                return self.blocked(cargo, "class0 task callback is not at a completed boundary");
            }
            let Some(parent) = entity.attached_to else {
                return self.blocked(cargo, "class0 drop relation unavailable");
            };
            // Close a parked/invalid child callback before allocating an
            // unlinked proxy or consuming its mandatory selector word.
            if let Err(reason) = super::class0_actor::cargo::prepare_release(
                manager,
                cargo,
                parent,
                Type9CargoReleasePosition::Retained,
                terrain,
            ) {
                return self.blocked(cargo, reason);
            }
        }
        let metadata = manager
            .type_runtime_metadata(CARGO_DROP_PROXY_ENTITY_TYPE)?
            .clone();
        let expected_selection = authenticate_type93_materialiser_metadata(&metadata)?;
        let initial_state = resolve_entity_initializer_with_selected_behavior(
            EntityInitializerRequest {
                metadata: Some(&metadata),
                spawn_param: 0,
                authored_position_raw: target,
                terrain: Some(terrain),
                resource_domain: ResourceDomainRelation::Current,
            },
            Some(expected_selection),
        )
        .state_flags;
        let construction_stamp_at_0xb4 = manager.begin_common_body_attempt();
        // 38080's singleton Type-93 behavior selector still consumes a word.
        // This precedes the child's first release selector (CE90 or D1C0), even though only one behavior
        // has a nonzero weight. Use the shared factory/Type-17 data constructor.
        let selected = select_initial_behavior(
            &metadata.initializer.as_ref()?.behavior_choices,
            |rule| i32::from(rule == BehaviorWeightRule::Always),
            || u32::from(self.world_fx.next_shared_retail_random_u16()),
        )
        .ok()
        .flatten()?;
        assert_eq!(selected, expected_selection);
        let proxy_id = manager.next_entity_id;
        let proxy = build_type93_materialiser_entity(
            &metadata,
            proxy_id,
            target,
            selected,
            initial_state,
            construction_stamp_at_0xb4,
        )
        .expect("authenticated Type-93 constructor");
        Some(manager.publish_constructed_type93_materialiser(proxy))
    }

    fn prepare_attach(
        &mut self,
        manager: &EntityManager,
        cargo: u32,
        parent: u32,
    ) -> Option<Self::AttachPlan> {
        let entity = manager.iter_all().find(|entity| entity.id == cargo)?;
        if entity.entity_type == 49 {
            if !self
                .scheduler
                .begin_cleansing_vehicle_external_mutation(manager, cargo)
            {
                return self.blocked(
                    cargo,
                    "cleansing task callback is not at a completed boundary",
                );
            }
            return match crate::cleansing_vehicle::cargo::prepare_attach(manager, cargo, parent) {
                Ok(plan) => Some(CargoAttachPlan::Cleansing(plan)),
                Err(reason) => self.blocked(cargo, reason),
            };
        }
        if manager.native_class0_construction_present(cargo) {
            if !self
                .scheduler
                .prepare_class0_actor_external_mutation(manager, cargo)
            {
                return self.blocked(cargo, "class0 task callback is not at a completed boundary");
            }
            return match super::class0_actor::cargo::prepare_attach(manager, cargo, parent) {
                Ok(plan) => Some(CargoAttachPlan::Class0(plan)),
                Err(reason) => self.blocked(cargo, reason),
            };
        }
        if NativeWorkerProfile::from_entity_type(entity.entity_type).is_some() {
            if !self
                .scheduler
                .begin_intro2_type8_external_mutation(manager, cargo)
            {
                return self.blocked(cargo, "Type8 task callback is not at a completed boundary");
            }
            return match crate::intro2_type8::cargo::prepare_attach(manager, cargo, parent) {
                Ok(plan) => Some(CargoAttachPlan::Type8 {
                    cargo,
                    parent,
                    plan,
                }),
                Err(reason) => self.blocked(cargo, reason),
            };
        }
        // 16700/DBF0/CE70 carries a living Type123 graph exactly like the
        // capture path; the parent here is the player rather than a captor.
        if entity.entity_type == 123 {
            if !self
                .scheduler
                .begin_native_type123_external_mutation(manager, cargo)
            {
                return self.blocked(
                    cargo,
                    "Type123 task callback is not at a completed boundary",
                );
            }
            return match crate::native_type123::cargo::prepare_attach(manager, cargo, parent) {
                Ok(plan) => Some(CargoAttachPlan::Type123 {
                    cargo,
                    parent,
                    plan,
                }),
                Err(reason) => self.blocked(cargo, reason),
            };
        }
        if NativeFourChoiceProfile::from_entity_type(entity.entity_type).is_some() {
            if !self
                .scheduler
                .begin_native_type86_external_mutation(manager, cargo)
            {
                return self.blocked(
                    cargo,
                    "four-choice task callback is not at a completed boundary",
                );
            }
            return match crate::native_type86::cargo::prepare_attach(manager, cargo, parent) {
                Ok(plan) => Some(CargoAttachPlan::FourChoice {
                    cargo,
                    parent,
                    plan,
                }),
                Err(reason) => self.blocked(cargo, reason),
            };
        }
        if entity.entity_type != 9 {
            return Some(CargoAttachPlan::Other(cargo, parent));
        }
        if entity
            .collision
            .state_flags_at_0x08
            .masked(REMOTE_OWNED_STATE_BIT)
            != RetailRuntimeValue::Known(0)
        {
            return self.blocked(cargo, "local Type-9 cargo ownership unavailable");
        }
        let Some(actor) = self.scheduler.prepare_type9_cargo_attach(manager, cargo) else {
            return self.blocked(cargo, "Type-9 task callback is not at a completed boundary");
        };
        let Some(metadata) = manager.type_runtime_metadata(9) else {
            return self.blocked(cargo, "Type-9 metadata unavailable");
        };
        let owner = manager.iter_all().find(|entity| entity.id == parent)?;
        if owner
            .collision
            .state_flags_at_0x08
            .masked(REMOTE_OWNED_STATE_BIT)
            != RetailRuntimeValue::Known(0)
        {
            return self.blocked(cargo, "local Type-9 parent ownership unavailable");
        }
        let relation = Type9RelationOwner {
            id: parent,
            capability_flags: owner.capability_flags,
            position_raw: owner.position_raw(),
        };
        match ordinary_type9_cargo::plan_attach(entity, metadata, relation) {
            Ok(plan) => Some(CargoAttachPlan::Type9 {
                actor,
                parent,
                plan,
            }),
            Err(reason) => self.blocked(cargo, reason),
        }
    }

    fn commit_attach(
        &mut self,
        manager: &mut EntityManager,
        plan: Self::AttachPlan,
        publish_sub_j: impl FnOnce(&mut EntityManager),
    ) {
        match plan {
            CargoAttachPlan::Cleansing(plan) => {
                let cargo = plan.cargo;
                publish_sub_j(manager);
                project_relation_attach(manager.entity_mut(cargo).unwrap(), plan.parent);
                crate::cleansing_vehicle::cargo::commit_attach(
                    manager.entity_mut(cargo).unwrap(),
                    plan,
                    self.world_fx,
                );
                self.scheduler.register_cleansing_vehicle(
                    crate::cleansing_vehicle::CleansingVehicleOwner::adopt(manager, cargo)
                        .expect("completed cleansing attach graph"),
                );
            }
            CargoAttachPlan::Class0(plan) => {
                assert!(self
                    .scheduler
                    .prepare_class0_actor_external_mutation(manager, plan.cargo));
                super::class0_actor::cargo::commit_attach(manager, plan, publish_sub_j);
            }
            CargoAttachPlan::Type8 {
                cargo,
                parent,
                plan,
            } => {
                assert!(self
                    .scheduler
                    .begin_intro2_type8_external_mutation(manager, cargo));
                publish_sub_j(manager);
                let entity = manager.entity_mut(cargo).expect("prepared Type8 cargo");
                project_relation_attach(entity, parent);
                let outcome =
                    crate::intro2_type8::cargo::commit_attach(manager, cargo, plan, self.world_fx);
                self.scheduler
                    .finish_native_actor_attachment(cargo, outcome);
            }
            CargoAttachPlan::Type123 {
                cargo,
                parent,
                plan,
            } => {
                assert!(self
                    .scheduler
                    .begin_native_type123_external_mutation(manager, cargo));
                publish_sub_j(manager);
                let entity = manager.entity_mut(cargo).expect("prepared Type123 cargo");
                project_relation_attach(entity, parent);
                let outcome = crate::native_type123::cargo::commit_attach(
                    manager,
                    cargo,
                    plan,
                    self.world_fx,
                );
                self.scheduler
                    .finish_native_actor_attachment(cargo, outcome);
            }
            CargoAttachPlan::Other(cargo, parent) => {
                AttachmentProjectionCallbacks.commit_attach(manager, (cargo, parent), publish_sub_j)
            }
            CargoAttachPlan::FourChoice {
                cargo,
                parent,
                plan,
            } => {
                assert!(self
                    .scheduler
                    .begin_native_type86_external_mutation(manager, cargo));
                publish_sub_j(manager);
                let entity = manager.entity_mut(cargo).expect("prepared person cargo");
                project_relation_attach(entity, parent);
                let outcome =
                    crate::native_type86::cargo::commit_attach(manager, cargo, plan, self.world_fx);
                self.scheduler
                    .finish_native_actor_attachment(cargo, outcome);
            }
            CargoAttachPlan::Type9 {
                actor,
                parent,
                plan,
            } => {
                assert!(
                    self.scheduler.take_type9_for_cargo(manager, actor),
                    "prepared selected custody"
                );
                publish_sub_j(manager);
                let entity = manager.entity_mut(actor.entity_id).expect("prepared cargo");
                project_relation_attach(entity, parent);
                if let Some((sound, position)) = plan.sound() {
                    self.world_fx
                        .queue_fixed_positional_sound_raw(sound, position);
                }
                let carried = ordinary_type9_cargo::commit_attach(entity, actor, plan)
                    .expect("prepared synchronous Type-9 attachment");
                assert!(self
                    .scheduler
                    .register_type9_carried(carried)
                    .expect("unique carried owner")
                    .is_none());
            }
        }
    }

    fn prepare_release(
        &mut self,
        manager: &EntityManager,
        cargo: u32,
        parent: u32,
        position: Type9CargoReleasePosition,
    ) -> Option<Self::ReleasePlan> {
        let entity = manager.iter_all().find(|entity| entity.id == cargo)?;
        if entity.entity_type == 49 {
            if !self
                .scheduler
                .begin_cleansing_vehicle_external_mutation(manager, cargo)
            {
                return self.blocked(
                    cargo,
                    "cleansing task callback is not at a completed boundary",
                );
            }
            let Some(terrain) = self.terrain else {
                return self.blocked(cargo, "cleansing release terrain unavailable");
            };
            return match crate::cleansing_vehicle::cargo::prepare_release(
                manager, cargo, parent, position, terrain,
            ) {
                Ok(plan) => Some(CargoReleasePlan::Cleansing(plan)),
                Err(reason) => self.blocked(cargo, reason),
            };
        }
        if manager.native_class0_construction_present(cargo) {
            if !self
                .scheduler
                .prepare_class0_actor_external_mutation(manager, cargo)
            {
                return self.blocked(cargo, "class0 task callback is not at a completed boundary");
            }
            let Some(terrain) = self.terrain else {
                return self.blocked(cargo, "class0 release terrain unavailable");
            };
            return match super::class0_actor::cargo::prepare_release(
                manager, cargo, parent, position, terrain,
            ) {
                Ok(plan) => Some(CargoReleasePlan::Class0(plan)),
                Err(reason) => self.blocked(cargo, reason),
            };
        }
        if NativeWorkerProfile::from_entity_type(entity.entity_type).is_some() {
            if !self
                .scheduler
                .begin_intro2_type8_external_mutation(manager, cargo)
            {
                return self.blocked(cargo, "Type8 task callback is not at a completed boundary");
            }
            return match crate::intro2_type8::cargo::prepare_release(
                manager, cargo, parent, position,
            ) {
                Ok(plan) => Some(CargoReleasePlan::Type8 { cargo, plan }),
                Err(reason) => self.blocked(cargo, reason),
            };
        }
        // CE90 reselects the Always-only root at commit time, exactly like the
        // capture path; the parent here is the player rather than a captor.
        if entity.entity_type == 123 {
            if !self
                .scheduler
                .begin_native_type123_external_mutation(manager, cargo)
            {
                return self.blocked(
                    cargo,
                    "Type123 task callback is not at a completed boundary",
                );
            }
            return match crate::native_type123::cargo::prepare_release(
                manager, cargo, parent, position,
            ) {
                Ok(plan) => Some(CargoReleasePlan::Type123 { cargo, plan }),
                Err(reason) => self.blocked(cargo, reason),
            };
        }
        if NativeFourChoiceProfile::from_entity_type(entity.entity_type).is_some() {
            if !self
                .scheduler
                .begin_native_type86_external_mutation(manager, cargo)
            {
                return self.blocked(
                    cargo,
                    "four-choice task callback is not at a completed boundary",
                );
            }
            return match crate::native_type86::cargo::prepare_release(
                manager, cargo, parent, position,
            ) {
                Ok(plan) => Some(CargoReleasePlan::FourChoice { cargo, plan }),
                Err(reason) => self.blocked(cargo, reason),
            };
        }
        if entity.entity_type != 9 {
            return Some(CargoReleasePlan::Other(cargo, position));
        }
        if entity
            .collision
            .state_flags_at_0x08
            .masked(REMOTE_OWNED_STATE_BIT)
            != RetailRuntimeValue::Known(0)
        {
            return self.blocked(cargo, "local Type-9 cargo ownership unavailable");
        }
        if entity.attached_to != Some(parent) {
            return self.blocked(cargo, "cargo relation changed");
        }
        let relation_owner = manager.iter_all().find(|entity| entity.id == parent)?;
        if relation_owner
            .collision
            .state_flags_at_0x08
            .masked(REMOTE_OWNED_STATE_BIT)
            != RetailRuntimeValue::Known(0)
        {
            return self.blocked(cargo, "local Type-9 parent ownership unavailable");
        }
        let Some(carried) = self.scheduler.type9_carried(cargo) else {
            return self.blocked(cargo, "carried Type-9 task authority unavailable");
        };
        if manager
            .main_base_abort_actor_observation(cargo)
            .map(|observation| observation.lease)
            != Some(carried.actor)
        {
            return self.blocked(cargo, "carried Type-9 allocation changed");
        }
        let Some(metadata) = manager.type_runtime_metadata(9) else {
            return self.blocked(cargo, "Type-9 metadata unavailable");
        };
        let candidates: Vec<_> = manager
            .retail_live_order_ids()
            .map(|id| {
                manager
                    .iter_all()
                    .find(|entity| entity.id == id)
                    .expect("live-list entity")
            })
            .map(|entity| OrdinaryType9RootEntityRef {
                id: entity.id,
                entity_type: entity.entity_type,
                position_raw: entity.position_raw(),
                state_flags_raw: entity.collision.state_flags_at_0x08,
                capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
                attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
            })
            .collect();
        let jobs: Vec<_> = manager
            .retail_live_order_ids()
            .map(|id| {
                manager
                    .iter_all()
                    .find(|entity| entity.id == id)
                    .expect("live-list entity")
            })
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
            .collect();
        let request = Type9CargoReleaseRequest {
            metadata,
            candidates: &candidates,
            job_evidence: &jobs,
            position,
        };
        match ordinary_type9_cargo::prepare_release(entity, carried, &request) {
            Ok(prepared) => Some(CargoReleasePlan::Type9 {
                cargo,
                metadata: metadata.clone(),
                candidates,
                jobs,
                position,
                prepared,
            }),
            Err(reason) => self.blocked(cargo, reason),
        }
    }

    fn commit_release(&mut self, manager: &mut EntityManager, plan: Self::ReleasePlan) {
        match plan {
            CargoReleasePlan::Cleansing(plan) => {
                let cargo = plan.cargo;
                crate::cleansing_vehicle::cargo::commit_release(manager, plan, self.world_fx);
                self.scheduler.register_cleansing_vehicle(
                    crate::cleansing_vehicle::CleansingVehicleOwner::adopt(manager, cargo)
                        .expect("completed cleansing release graph"),
                );
            }
            CargoReleasePlan::Class0(plan) => {
                let cargo = plan.cargo;
                assert!(self
                    .scheduler
                    .take_class0_actor_external_mutation(manager, cargo));
                super::class0_actor::cargo::commit_release(manager, plan, self.world_fx);
                let owner = super::class0_actor::Class0ActorOwner::adopt(manager, cargo)
                    .expect("completed class0 release graph");
                self.scheduler.register_class0_actor(owner);
            }
            CargoReleasePlan::Type8 { cargo, plan } => {
                crate::intro2_type8::cargo::commit_release(
                    manager.entity_mut(cargo).expect("prepared Type8 cargo"),
                    plan,
                    self.world_fx,
                );
            }
            CargoReleasePlan::Type123 { cargo, plan } => {
                // Unlike the Type8 arm, the released owner is re-registered:
                // CE90 reselects a live graph that must rejoin the scheduler,
                // following the capture path rather than leaving it stale.
                let owner = crate::native_type123::cargo::commit_release(
                    manager,
                    cargo,
                    plan,
                    self.world_fx,
                );
                self.scheduler.register_native_type123(owner);
            }
            CargoReleasePlan::Other(cargo, position) => {
                AttachmentProjectionCallbacks.commit_release(manager, (cargo, position))
            }
            CargoReleasePlan::FourChoice { cargo, plan } => {
                let owner = crate::native_type86::cargo::commit_release(
                    manager,
                    cargo,
                    plan,
                    self.world_fx,
                );
                self.scheduler.register_native_type86(owner);
                if let Err(error) = self
                    .notifications
                    .drain_attract_attention_receipts(manager, self.retail_tick as i32)
                {
                    self.scheduler.park_native_type86_external_prefix(cargo);
                    self.blocked
                        .push((cargo, format!("release resource notification: {error:?}")));
                }
            }
            CargoReleasePlan::Type9 {
                cargo,
                metadata,
                candidates,
                jobs,
                position,
                prepared,
            } => {
                let carried = self
                    .scheduler
                    .take_type9_carried(cargo)
                    .expect("prepared carried custody");
                let request = Type9CargoReleaseRequest {
                    metadata: &metadata,
                    candidates: &candidates,
                    job_evidence: &jobs,
                    position,
                };
                // Keep a single shared RNG owner. The existing initializer
                // callbacks emit typed presentation receipts without reading
                // or mutating the live entity list.
                let mut texts = Vec::new();
                let mut sounds = Vec::new();
                let publication = ordinary_type9_cargo::commit_release(
                    manager.entity_mut(cargo).expect("prepared cargo"),
                    carried,
                    prepared,
                    request,
                    || u32::from(self.world_fx.next_shared_retail_random_u16()),
                    |request| texts.push(request),
                    |request| sounds.push(request),
                )
                .expect("fully preflighted synchronous Type-9 release");
                for request in texts {
                    self.notifications
                        .queue_attract_attention_resource_text(request, self.retail_tick as i32)
                        .expect("authored Attract resource receipt");
                }
                for request in sounds {
                    debug_assert_eq!((request.gain_16_16, request.rate_16_16), (0x10000, 0x10000));
                    self.world_fx.queue_fixed_positional_sound_raw(
                        request.global_sound_id,
                        request.position_raw,
                    );
                }
                let entity = manager
                    .iter_all()
                    .find(|entity| entity.id == cargo)
                    .expect("published peasant");
                let authority =
                    OrdinaryType9CurrentTaskAuthority::from_publication(entity, publication)
                        .expect("exact release publication");
                self.scheduler
                    .adopt_type9_current_task(manager, authority)
                    .expect("released peasant replaces consumed carried owner");
            }
        }
    }
}

#[cfg(test)]
#[path = "cargo_runtime_tests.rs"]
mod tests;
