//! Live Type-60 construction shared by class 49 and hard-water entry.

use super::{
    actor_animation_runtime_from_constructor, actor_common_axis_descriptor_from_constructor,
    base_factory_runtime_from_constructor, entity_sub_n_runtime_from_constructor,
    main_base_abort_actor_allocation_identity, model_for_constructor_state, raw_position_world,
    sub_a_propulsion_runtime_from_constructor, sub_g_06070_runtime_from_constructor,
    sub_h_external_frame_runtime_from_constructor, sub_j_attachment_runtime_from_constructor,
    Entity, EntityKind, EntityManager,
};
use crate::actor_task_dispatcher::ActorTaskRuntime;
use crate::actor_task_owner::{ActorTaskOwner, ActorTaskSlot};
use crate::entity_behavior::{select_initial_behavior, BehaviorContextRuntime, BehaviorWeightRule};
use crate::entity_initializer::{
    resolve_entity_initializer_with_selected_behavior, EntityInitializerRequest,
    ResourceDomainRelation,
};
use crate::main_base_abort::MainBaseAbortActorLease;
use crate::type60_exploding_ring::{
    exact_type60_ring_metadata, HardWaterType60ConstructionRequest, HostType60Allocator,
    Type60Allocator, Type60ConstructionOutcome, Type60ConstructionReceipt,
    Type60ConstructionRequest, Type60ExplodingRingTaskState, TYPE60_RING_ENTITY_TYPE,
    TYPE60_RING_FAILED_TASK_CONTROL_OUTPUT_1_RAW, TYPE60_RING_INITIAL_CONTROL_OUTPUT_1_RAW,
};
use crate::world_fx::WorldFx;
use v2k_formats::terrain::TerrainGrid;

impl EntityManager {
    /// Construct one real hard-water Type-60 actor.
    ///
    /// This seam owns only generic construction and its selector/failure
    /// boundaries. The caller retains `FUN_004141D0`'s later sound-17 and
    /// entering-body velocity writes so their synchronous order stays
    /// explicit.
    pub fn construct_hard_water_type60_ring(
        &mut self,
        request: HardWaterType60ConstructionRequest,
        terrain: &TerrainGrid,
        world_fx: &mut WorldFx,
    ) -> Type60ConstructionOutcome {
        self.construct_type60_exploding_ring_with_allocator(
            request.into_generic(),
            terrain,
            world_fx,
            &mut HostType60Allocator,
        )
    }

    pub(crate) fn construct_type60_exploding_ring_with_allocator(
        &mut self,
        request: Type60ConstructionRequest,
        terrain: &TerrainGrid,
        world_fx: &mut WorldFx,
        allocator: &mut impl Type60Allocator,
    ) -> Type60ConstructionOutcome {
        debug_assert_eq!(request.entity_type(), TYPE60_RING_ENTITY_TYPE);
        debug_assert_eq!(request.rotation_raw(), [0; 3]);
        let Some(metadata) = self
            .type_metadata
            .get(TYPE60_RING_ENTITY_TYPE as usize)
            .filter(|metadata| exact_type60_ring_metadata(metadata))
            .cloned()
        else {
            return Type60ConstructionOutcome::RejectedBeforeSelector;
        };
        // Missing host metadata is admission failure, not an entered retail
        // allocation. Once admitted,104B0 consumes456C20 before body/component
        // allocation; every subsequent allocator rejection retains this stamp.
        let construction_stamp_at_0xb4 = self.begin_common_body_attempt();
        if !allocator.prepare_components(request) {
            return Type60ConstructionOutcome::RejectedBeforeSelector;
        }
        let mut selector_rng_word = None;
        let selection = select_initial_behavior(
            metadata
                .initializer
                .as_ref()
                .expect("exact Type-60 metadata retains its initializer")
                .behavior_choices
                .as_ref(),
            |rule| match rule {
                BehaviorWeightRule::Always => 1,
                _ => unreachable!("exact Type-60 metadata has one Always choice"),
            },
            || {
                let word = world_fx.next_shared_retail_random_u16();
                selector_rng_word = Some(word);
                u32::from(word)
            },
        )
        .expect("exact Type-60 behavior row is valid")
        .expect("the singleton positive Type-60 choice is selected");
        let selector_rng_word =
            selector_rng_word.expect("retail selector draws once even for singleton Always");
        if !allocator.prepare_behavior_context() {
            return Type60ConstructionOutcome::RejectedAfterSelector { selector_rng_word };
        }
        let selected_context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
            .expect("exact class-48 selection constructs a fresh context");
        let resolution = resolve_entity_initializer_with_selected_behavior(
            EntityInitializerRequest {
                metadata: Some(&metadata),
                spawn_param: request.payload_raw(),
                authored_position_raw: request.position_raw(),
                terrain: Some(terrain),
                resource_domain: ResourceDomainRelation::Current,
            },
            Some(selection),
        );
        let model_overrides = request.model_overrides();
        let model_slots = std::array::from_fn(|slot| {
            model_overrides[slot].or(Some(usize::from(metadata.model_slots[slot])))
        });
        let state_flags = resolution.state_flags;
        let model_index = model_for_constructor_state(model_slots, state_flags);
        debug_assert_eq!(model_index, Some(request.presentation_model_id()));
        let entity_id = self.next_entity_id;
        let mut entity = Entity {
            id: entity_id,
            construction_stamp_at_0xb4,
            authored_spawn_index: None,
            kind: EntityKind::from_type(TYPE60_RING_ENTITY_TYPE),
            entity_type: TYPE60_RING_ENTITY_TYPE,
            authored_follow_beacon_priority_raw: None,
            power_up_payload_packed: None,
            auto_pilot_payload_packed: None,
            factory_type61_birth_provenance: None,
            type60_construction_provenance: Some(request.provenance()),
            main_base_type54_sea_delta_source:
                crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            position: raw_position_world(request.position_raw()),
            heading: 0.0,
            pitch_roll_raw: [0; 2],
            physical_body_basis_q31: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            velocity: [0.0; 3],
            surface_lifetime_timer_ms_at_0x48:
                crate::entity_collision_state::RetailRuntimeValue::Known(0),
            mass_raw: metadata.mass_raw,
            capability_flags: metadata.capability_flags,
            attached_to: None,
            model_slots,
            model_index,
            collision: super::EntityCollisionRuntimeState::from_constructor(
                Some(&metadata),
                request.initial_damage_buffer_raw(),
                state_flags,
            ),
            initial_behavior: crate::entity_collision_state::RetailRuntimeValue::Known(Some(
                selection,
            )),
            current_behavior_context: crate::entity_collision_state::RetailRuntimeValue::Known(
                Some(selected_context),
            ),
            authored_radial_emitter: None,
            sub_n_runtime: entity_sub_n_runtime_from_constructor(
                Some(&metadata),
                false,
                request.position_raw(),
                None,
            ),
            base_factory_runtime: base_factory_runtime_from_constructor(
                TYPE60_RING_ENTITY_TYPE,
                Some(&metadata),
                None,
            ),
            actor_animation_runtime: actor_animation_runtime_from_constructor(Some(&metadata)),
            sub_a_propulsion_runtime: sub_a_propulsion_runtime_from_constructor(Some(&metadata)),
            sub_g_06070_runtime: sub_g_06070_runtime_from_constructor(Some(&metadata)),
            intro2_type13_common_mover_runtime: None,
            native_type13_allocation: None,
            intro2_type13_aim_runtime: None,
            intro2_type16_aim_runtime: None,
            intro2_type58_aim_runtime: None,
            intro2_type94_aim_runtime: None,
            intro2_flyer_aim_runtime: None,
            sub_h_external_frame_runtime: sub_h_external_frame_runtime_from_constructor(Some(
                &metadata,
            )),
            sub_j_attachment_runtime: sub_j_attachment_runtime_from_constructor(
                Some(&metadata),
                None,
                None,
            ),
            actor_common_axis_descriptor: actor_common_axis_descriptor_from_constructor(Some(
                &metadata,
            )),
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
            native_type43_runtime: None,
            native_type43_aim_runtime: None,
            native_type38_runtime: None,
            native_type38_aim_runtime: None,
            native_type18_runtime: None,
            native_type18_aim_runtime: None,
            native_type28_runtime: None,
            native_type76_runtime: None,
            native_type76_aim_runtime: None,
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
            native_type123_anchor_raw_at_0x90:
                crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            native_type86_runtime: None,
            native_type86_anchor_raw_at_0x90:
                crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            intro2_type9_runtime: None,
            ordinary_type9_native_receipt: None,
            main_base_runtime: None,
            type17_sub_d_frame_owner: None,
            type17_sub_d_runtime: None,
            type8_sub_d_frame_owner: None,
            type8_wander_anchor_raw_at_0x90:
                crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            type8_sub_d_runtime: None,
            type47_immutable_anchor_raw_at_0x90:
                crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            active: true,
        };
        let prepared = allocator.prepare_primary_task(Type60ExplodingRingTaskState::new());
        let (task_id, control_output_1_raw) = match prepared {
            Some(prepared) => {
                let task_id = entity.actor_tasks.replace_prepared(
                    ActorTaskSlot::Primary,
                    prepared.map(ActorTaskRuntime::ExplodingRing),
                );
                (Some(task_id), TYPE60_RING_INITIAL_CONTROL_OUTPUT_1_RAW)
            }
            None => {
                entity.publish_behavior_initializer_failure_fallback(selected_context);
                (None, TYPE60_RING_FAILED_TASK_CONTROL_OUTPUT_1_RAW)
            }
        };
        let allocated_id = self.allocate_live_entity_id();
        debug_assert_eq!(allocated_id, entity_id);
        self.append_live_entity(entity);
        world_fx.materialize_type60_exploding_ring_raw(
            entity_id,
            request.position_raw(),
            request.presentation_model_id(),
            control_output_1_raw,
        );
        let actor = MainBaseAbortActorLease {
            entity_id,
            allocation_identity: main_base_abort_actor_allocation_identity(
                self.allocation_generation,
                entity_id,
            ),
        };
        let receipt = match task_id {
            Some(task_id) => Type60ConstructionReceipt::linked_with_primary(
                actor,
                request.provenance(),
                task_id,
                selector_rng_word,
            ),
            None => Type60ConstructionReceipt::linked_after_primary_allocation_failure(
                actor,
                request.provenance(),
                selector_rng_word,
            ),
        };
        Type60ConstructionOutcome::ActorLinked(receipt)
    }
}

#[cfg(test)]
mod tests {
    use super::super::world_position_raw;
    use super::*;
    use crate::actor_task_owner::PreparedActorTask;
    use crate::entity_collision_state::{
        EntityInitializerSpec, EntityTypeRuntimeMetadata, RetailRuntimeValue,
    };
    use crate::type60_exploding_ring::{
        HardWaterType60Severity, Type60ConstructionProvenance, Type60ConstructorRngDisposition,
        Type60InitializerDisposition, TYPE60_COMPONENT_TOPOLOGY,
        TYPE60_RING_ALTERNATE_BEHAVIOR_CLASS, TYPE60_RING_CAPABILITY_FLAGS,
        TYPE60_RING_COMMON_AXIS_DESCRIPTOR, TYPE60_RING_DAMAGE_PROFILE,
        TYPE60_RING_INITIALIZER_STATE_RAW, TYPE60_RING_INITIAL_BEHAVIOR_CLASS,
        TYPE60_RING_INITIAL_HEALTH_RAW, TYPE60_RING_MASS_RAW, TYPE60_RING_MODEL_ID,
    };
    use crate::world_fx::{HARD_WATER_ENTRY_SPLASH_MID_MODEL_ID, HARD_WATER_ENTRY_SPLASH_MODEL_ID};
    use v2k_formats::collision::BehaviorChoice;
    use v2k_formats::terrain::{TerrainCell, GRID_SIZE};

    fn exact_type60_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            cured_model_presentation_sound_id: RetailRuntimeValue::Unresolved,
            common_world_effects: RetailRuntimeValue::Unresolved,
            detailed_sound_policy: RetailRuntimeValue::Unresolved,
            model_slots: [TYPE60_RING_MODEL_ID as u16; 4],
            mass_raw: TYPE60_RING_MASS_RAW,
            capability_flags: TYPE60_RING_CAPABILITY_FLAGS,
            initial_health_raw: Some(TYPE60_RING_INITIAL_HEALTH_RAW),
            damage_profile: Some(TYPE60_RING_DAMAGE_PROFILE),
            accepted_hit_presentation_sound_id: RetailRuntimeValue::Known(None),
            infected_model_presentation_sound_id: RetailRuntimeValue::Known(None),
            death_sound_id: RetailRuntimeValue::Known(None),
            target_warning_sound_id: RetailRuntimeValue::Unresolved,
            constructor_sound_attachment_id: RetailRuntimeValue::Known(None),
            generic_hit_sound_id: RetailRuntimeValue::Known(None),
            search_attack_optional_prelude_sound_id: RetailRuntimeValue::Unresolved,
            search_attack_aim_sound_id: RetailRuntimeValue::Unresolved,
            search_attack_aim_sound_period_raw: RetailRuntimeValue::Unresolved,
            run_away_optional_sound_id: RetailRuntimeValue::Unresolved,
            run_away_sound_period_raw: RetailRuntimeValue::Unresolved,
            terrain_contact_task_lifetime_ms: RetailRuntimeValue::Unresolved,
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(None),
            sub_b_lateral_descriptor: RetailRuntimeValue::Known(None),
            sub_c_lift_descriptor: RetailRuntimeValue::Known(None),
            sub_d_steering_descriptor: RetailRuntimeValue::Known(None),
            sub_f_swimming_descriptor: RetailRuntimeValue::Known(None),
            model_variable_count_raw: RetailRuntimeValue::Unresolved,
            projectile_emitter_descriptor: RetailRuntimeValue::Known(None),
            sub_n_payload: None,
            status_component_descriptor: RetailRuntimeValue::Known(None),
            actor_animation_descriptor: RetailRuntimeValue::Known(None),
            sub_h_external_frame_descriptor: RetailRuntimeValue::Known(None),
            sub_j_attachment_descriptor: RetailRuntimeValue::Known(None),
            common_mover_topology: RetailRuntimeValue::Known(TYPE60_COMPONENT_TOPOLOGY),
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: TYPE60_RING_INITIALIZER_STATE_RAW,
                common_axis_descriptor: TYPE60_RING_COMMON_AXIS_DESCRIPTOR,
                behavior_choices: vec![BehaviorChoice {
                    weight_rule_id: 1,
                    weight_multiplier: 1,
                    behavior_class_id: u32::from(TYPE60_RING_INITIAL_BEHAVIOR_CLASS),
                }]
                .into_boxed_slice(),
                behavior_rule_ref: 1,
                alternate_behavior_class_ref: u32::from(TYPE60_RING_ALTERNATE_BEHAVIOR_CLASS),
            }),
            common_mover_gkl_payloads: RetailRuntimeValue::Unresolved,
        }
    }

    fn manager() -> EntityManager {
        let mut metadata = vec![EntityTypeRuntimeMetadata::default(); 61];
        metadata[TYPE60_RING_ENTITY_TYPE as usize] = exact_type60_metadata();
        EntityManager::from_entities_with_type_metadata_for_test(Vec::new(), metadata, true)
    }

    fn terrain() -> TerrainGrid {
        TerrainGrid {
            header: [0; 5],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0,
                };
                GRID_SIZE * GRID_SIZE
            ],
        }
    }

    #[test]
    fn hard_water_construction_retains_provenance_four_slot_model_and_primary_lease() {
        let terrain = terrain();
        let mut manager = manager();
        manager.set_common_body_stamp_counter_for_test(2);
        let mut world_fx = WorldFx::new();
        let mut rng_oracle = WorldFx::new();
        let expected_moderate_word = rng_oracle.next_shared_retail_random_u16();
        let expected_severe_word = rng_oracle.next_shared_retail_random_u16();

        let moderate_position = [0x0123, -0x0200, 0x0345];
        let moderate = manager.construct_hard_water_type60_ring(
            HardWaterType60ConstructionRequest::new(
                moderate_position,
                HardWaterType60Severity::Moderate,
            ),
            &terrain,
            &mut world_fx,
        );
        let Type60ConstructionOutcome::ActorLinked(moderate_receipt) = moderate else {
            panic!("host construction must link the moderate actor")
        };
        assert_eq!(
            moderate_receipt.constructor_rng(),
            Type60ConstructorRngDisposition::SelectorWord(expected_moderate_word)
        );
        assert_eq!(
            moderate_receipt.provenance(),
            Type60ConstructionProvenance::HardWaterModerate
        );
        let moderate_task = moderate_receipt
            .primary_task_lease()
            .expect("host construction publishes Primary");
        let moderate_id = moderate_receipt.actor().entity_id;
        {
            let entity = manager
                .entities
                .iter()
                .find(|entity| entity.id == moderate_id)
                .expect("moderate actor remains linked");
            assert_eq!(
                entity.type60_construction_provenance(),
                Some(Type60ConstructionProvenance::HardWaterModerate)
            );
            assert_eq!(
                entity.construction_stamp_at_0xb4,
                RetailRuntimeValue::Known(0x0800)
            );
            assert_eq!(
                entity.model_slots,
                [Some(HARD_WATER_ENTRY_SPLASH_MID_MODEL_ID); 4]
            );
            assert_eq!(
                entity.model_index,
                Some(HARD_WATER_ENTRY_SPLASH_MID_MODEL_ID)
            );
            let Some(ActorTaskRuntime::ExplodingRing(state)) =
                entity.actor_tasks.task_state(moderate_task.task_id())
            else {
                panic!("Primary owns exact Exploding Ring state")
            };
            assert_eq!(state.control_output_1_raw(), u16::MAX);
            assert_eq!(state.next_callback_sequence(), 1);
        }
        assert!(manager
            .type60_exploding_ring_entity_mut(moderate_id)
            .is_some());

        let moderate_ring = world_fx.exploding_rings()[0];
        assert_eq!(moderate_ring.associated_entity_id(), moderate_id);
        assert_eq!(moderate_ring.model_id, HARD_WATER_ENTRY_SPLASH_MID_MODEL_ID);
        assert_eq!(
            world_position_raw(moderate_ring.position),
            moderate_position
        );
        assert_eq!(moderate_ring.control_output_1_raw(), u16::MAX);

        let severe_position = [-0x0456, -0x0300, 0x0789];
        let severe = manager.construct_hard_water_type60_ring(
            HardWaterType60ConstructionRequest::new(
                severe_position,
                HardWaterType60Severity::Severe,
            ),
            &terrain,
            &mut world_fx,
        );
        let Type60ConstructionOutcome::ActorLinked(severe_receipt) = severe else {
            panic!("host construction must link the severe actor")
        };
        assert_eq!(
            severe_receipt.constructor_rng(),
            Type60ConstructorRngDisposition::SelectorWord(expected_severe_word)
        );
        assert_eq!(
            severe_receipt.provenance(),
            Type60ConstructionProvenance::HardWaterSevere
        );
        assert!(severe_receipt.primary_task_lease().is_some());
        let severe_id = severe_receipt.actor().entity_id;
        let entity = manager
            .entities
            .iter()
            .find(|entity| entity.id == severe_id)
            .expect("severe actor remains linked");
        assert_eq!(
            entity.model_slots,
            [Some(HARD_WATER_ENTRY_SPLASH_MODEL_ID); 4]
        );
        assert_eq!(entity.model_index, Some(HARD_WATER_ENTRY_SPLASH_MODEL_ID));
        assert_eq!(
            entity.type60_construction_provenance(),
            Some(Type60ConstructionProvenance::HardWaterSevere)
        );
        let severe_ring = world_fx.exploding_rings()[1];
        assert_eq!(
            entity.construction_stamp_at_0xb4,
            RetailRuntimeValue::Known(0x0801)
        );
        assert_eq!(
            manager.next_common_body_ordinal(),
            RetailRuntimeValue::Known(2)
        );
        assert_eq!(severe_ring.associated_entity_id(), severe_id);
        assert_eq!(severe_ring.model_id, HARD_WATER_ENTRY_SPLASH_MODEL_ID);
        assert_eq!(world_position_raw(severe_ring.position), severe_position);
    }

    struct RejectBeforeSelector;

    impl Type60Allocator for RejectBeforeSelector {
        fn prepare_components(&mut self, _request: Type60ConstructionRequest) -> bool {
            false
        }

        fn prepare_behavior_context(&mut self) -> bool {
            panic!("pre-selector rejection cannot prepare behavior context")
        }

        fn prepare_primary_task(
            &mut self,
            _state: Type60ExplodingRingTaskState,
        ) -> Option<PreparedActorTask<Type60ExplodingRingTaskState>> {
            panic!("pre-selector rejection cannot prepare Primary")
        }
    }

    struct RejectAfterSelector;

    impl Type60Allocator for RejectAfterSelector {
        fn prepare_components(&mut self, _request: Type60ConstructionRequest) -> bool {
            true
        }

        fn prepare_behavior_context(&mut self) -> bool {
            false
        }

        fn prepare_primary_task(
            &mut self,
            _state: Type60ExplodingRingTaskState,
        ) -> Option<PreparedActorTask<Type60ExplodingRingTaskState>> {
            panic!("post-selector rejection cannot prepare Primary")
        }
    }

    struct RejectPrimary;

    impl Type60Allocator for RejectPrimary {
        fn prepare_components(&mut self, _request: Type60ConstructionRequest) -> bool {
            true
        }

        fn prepare_behavior_context(&mut self) -> bool {
            true
        }

        fn prepare_primary_task(
            &mut self,
            _state: Type60ExplodingRingTaskState,
        ) -> Option<PreparedActorTask<Type60ExplodingRingTaskState>> {
            None
        }
    }

    #[test]
    fn construction_failures_preserve_exact_selector_and_link_boundaries() {
        let terrain = terrain();
        let mut missing_metadata =
            EntityManager::from_entities_with_type_metadata_for_test(Vec::new(), Vec::new(), true);
        missing_metadata.set_common_body_stamp_counter_for_test(27);
        let mut missing_fx = WorldFx::new();
        let mut missing_oracle = WorldFx::new();
        assert_eq!(
            missing_metadata.construct_hard_water_type60_ring(
                HardWaterType60ConstructionRequest::new(
                    [0x0100, -0x0200, 0x0300],
                    HardWaterType60Severity::Severe,
                ),
                &terrain,
                &mut missing_fx,
            ),
            Type60ConstructionOutcome::RejectedBeforeSelector
        );
        assert_eq!(
            missing_fx.next_shared_retail_random_u16(),
            missing_oracle.next_shared_retail_random_u16(),
            "unavailable metadata fails before selector RNG"
        );
        assert_eq!(
            missing_metadata.next_common_body_ordinal(),
            RetailRuntimeValue::Known(0),
            "host metadata admission is not an entered104B0 allocation"
        );

        let request = HardWaterType60ConstructionRequest::new(
            [0x0100, -0x0200, 0x0300],
            HardWaterType60Severity::Severe,
        )
        .into_generic();

        let mut before_manager = manager();
        before_manager.set_common_body_stamp_counter_for_test(27);
        let mut before_fx = WorldFx::new();
        let mut before_oracle = WorldFx::new();
        assert_eq!(
            before_manager.construct_type60_exploding_ring_with_allocator(
                request,
                &terrain,
                &mut before_fx,
                &mut RejectBeforeSelector,
            ),
            Type60ConstructionOutcome::RejectedBeforeSelector
        );
        assert!(before_manager.entities.is_empty());
        assert_eq!(
            before_manager.next_common_body_ordinal(),
            RetailRuntimeValue::Known(1),
            "body/component failure retains its pre-selector stamp attempt"
        );
        assert!(before_fx.exploding_rings().is_empty());
        assert_eq!(
            before_fx.next_shared_retail_random_u16(),
            before_oracle.next_shared_retail_random_u16(),
            "component rejection consumes no selector word"
        );

        let mut after_manager = manager();
        after_manager.set_common_body_stamp_counter_for_test(27);
        let mut after_fx = WorldFx::new();
        let mut after_oracle = WorldFx::new();
        let expected_after_word = after_oracle.next_shared_retail_random_u16();
        let expected_after_next = after_oracle.next_shared_retail_random_u16();
        assert_eq!(
            after_manager.construct_type60_exploding_ring_with_allocator(
                request,
                &terrain,
                &mut after_fx,
                &mut RejectAfterSelector,
            ),
            Type60ConstructionOutcome::RejectedAfterSelector {
                selector_rng_word: expected_after_word,
            }
        );
        assert!(after_manager.entities.is_empty());
        assert_eq!(
            after_manager.next_common_body_ordinal(),
            RetailRuntimeValue::Known(1)
        );
        assert!(after_fx.exploding_rings().is_empty());
        assert_eq!(
            after_fx.next_shared_retail_random_u16(),
            expected_after_next,
            "context rejection retains the singleton selector draw"
        );

        let mut primary_manager = manager();
        primary_manager.set_common_body_stamp_counter_for_test(27);
        let mut primary_fx = WorldFx::new();
        let mut primary_oracle = WorldFx::new();
        let expected_primary_word = primary_oracle.next_shared_retail_random_u16();
        let expected_primary_next = primary_oracle.next_shared_retail_random_u16();
        let outcome = primary_manager.construct_type60_exploding_ring_with_allocator(
            request,
            &terrain,
            &mut primary_fx,
            &mut RejectPrimary,
        );
        let Type60ConstructionOutcome::ActorLinked(receipt) = outcome else {
            panic!("Primary allocation failure still links fallback actor")
        };
        assert_eq!(
            receipt.constructor_rng(),
            Type60ConstructorRngDisposition::SelectorWord(expected_primary_word)
        );
        assert_eq!(receipt.primary_task_lease(), None);
        assert_eq!(
            receipt.initializer(),
            Type60InitializerDisposition::FallbackPublishedAfterPrimaryAllocationFailure {
                control_output_1_raw: 0,
            }
        );
        assert_eq!(primary_manager.entities.len(), 1);
        let entity = &primary_manager.entities[0];
        assert_eq!(
            entity.construction_stamp_at_0xb4,
            RetailRuntimeValue::Known(0x6c00)
        );
        assert_eq!(
            primary_manager.next_common_body_ordinal(),
            RetailRuntimeValue::Known(1),
            "Primary fallback publishes the original attempted body exactly once"
        );
        assert_eq!(
            entity.type60_construction_provenance(),
            Some(Type60ConstructionProvenance::HardWaterSevere)
        );
        assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| entity.actor_task_state(slot).is_none()));
        let ring = primary_fx.exploding_rings()[0];
        assert_eq!(ring.associated_entity_id(), entity.id);
        assert_eq!(ring.model_id, HARD_WATER_ENTRY_SPLASH_MODEL_ID);
        assert_eq!(ring.control_output_1_raw(), 0);
        assert_eq!(
            primary_fx.next_shared_retail_random_u16(),
            expected_primary_next
        );
    }

    #[test]
    fn partial_manager_does_not_manufacture_a_native_ring_stamp() {
        let mut manager = manager();
        let outcome = manager.construct_hard_water_type60_ring(
            HardWaterType60ConstructionRequest::new([0; 3], HardWaterType60Severity::Moderate),
            &terrain(),
            &mut WorldFx::new(),
        );
        assert!(matches!(outcome, Type60ConstructionOutcome::ActorLinked(_)));
        assert_eq!(
            manager.next_common_body_ordinal(),
            RetailRuntimeValue::Unresolved
        );
        assert_eq!(
            manager.entities[0].construction_stamp_at_0xb4,
            RetailRuntimeValue::Unresolved
        );
    }
}
