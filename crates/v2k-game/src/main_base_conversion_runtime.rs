//! Native worker replacement planning/apply owners for Main Base conversion.
//!
//! The ordered pair visit remains in [`crate::main_base_conversion`]. This
//! module deliberately does not classify contact or own a source's
//! descriptor-component/physical suffix. It closes every fallible world and
//! initializer prerequisite before event 1, then retains the one shared-RNG
//! selection and supported worker append for the ordered apply phase.

use crate::entity::{
    EntityManager, MainBaseScientistSpawnError, MainBaseScientistSpawnRequest,
    MainBaseScientistSpawned,
};
use crate::entity_behavior::{
    behavior_program, select_initial_behavior, BehaviorSelectionError, BehaviorWeightRule,
    MAX_BEHAVIOR_CHOICES,
};
use crate::entity_collision_state::RetailRuntimeValue;
use crate::intro2_type8::NativeWorkerProfile;
use crate::job_nearby::{
    evaluate_job_nearby, JobAwareBehaviorSelectionError, JobCapacityState, JobNearbyCandidate,
    JobNearbyEvaluationRequest, JobNearbyOwner,
};
use crate::main_base_conversion::MainBaseReplacementSpawn;
use crate::native_type86::birth::FourChoiceRootPlan;
use crate::world_fx::WorldFx;
use crate::wrapped_axis_range::WrappedAxisRange;
use v2k_formats::{collision::BehaviorChoice, terrain::TerrainGrid};

const FIRST_WORLD_SCIENTIST_ENTITY_TYPE: u32 = 8;
const FIRST_WORLD_SOURCE_ENTITY_TYPE: u32 = 9;
const FIRST_WORLD_MAIN_BASE_ENTITY_TYPE: u32 = 6;

/// Why a replacement action could not be closed before the visit mutated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MainBaseReplacementPreflightError {
    UnsupportedReplacementType { replacement_type: u32 },
    SourceUnavailable { source_id: u32 },
    SourceTypeMismatch { source_id: u32, entity_type: u32 },
    SourceDestroyAlreadyPending { source_id: u32 },
    MainBaseUnavailable { main_base_id: u32 },
    ScientistMetadataUnavailable,
    ScientistInitializerUnavailable,
    BehaviorSelection(JobAwareBehaviorSelectionError),
    ScientistConstructor(MainBaseScientistSpawnError),
    NegativeEffectiveWeight { choice_index: usize, weight: i32 },
    NoBehaviorSelected,
}

/// Immutable action closed before event 1/deferred destruction.
#[derive(Debug, PartialEq, Eq)]
pub struct PreparedFirstWorldMainBaseReplacement {
    request: MainBaseReplacementSpawn,
    constructor: PreparedReplacementConstructor,
    constructor_surface_bits: Option<u32>,
}

/// Constructor ownership remains explicit: the diver's complete four-choice
/// graph cannot be reduced to the ordinary workers' JobNearby/Always selector.
#[derive(Debug, PartialEq, Eq)]
enum PreparedReplacementConstructor {
    TwoChoice {
        behavior_choices: Box<[BehaviorChoice]>,
        job_nearby: bool,
        prepared_go_to_job: Option<crate::go_to_job::GoToJobSetupPlan>,
    },
    Diver(FourChoiceRootPlan),
}

/// Close all non-allocation state used by worker construction without touching
/// shared RNG or the live list.
pub fn prepare_first_world_main_base_replacement(
    entities: &EntityManager,
    terrain: Option<&TerrainGrid>,
    request: MainBaseReplacementSpawn,
    retail_tick: u32,
) -> Result<PreparedFirstWorldMainBaseReplacement, MainBaseReplacementPreflightError> {
    if request.replacement_type != 7
        && NativeWorkerProfile::from_entity_type(request.replacement_type).is_none()
    {
        return Err(
            MainBaseReplacementPreflightError::UnsupportedReplacementType {
                replacement_type: request.replacement_type,
            },
        );
    }

    let source = entities
        .iter_all()
        .find(|entity| entity.id == request.source_entity_id)
        .ok_or(MainBaseReplacementPreflightError::SourceUnavailable {
            source_id: request.source_entity_id,
        })?;
    if (source.entity_type != FIRST_WORLD_SOURCE_ENTITY_TYPE
        || source.ordinary_type9_native_receipt.is_some())
        && !crate::main_base_person_contact::native_person_allocation_authenticates(
            entities, source.id,
        )
    {
        return Err(MainBaseReplacementPreflightError::SourceTypeMismatch {
            source_id: source.id,
            entity_type: source.entity_type,
        });
    }
    if entities
        .pending_main_base_conversion_destroy_ids()
        .contains(&request.source_entity_id)
    {
        return Err(
            MainBaseReplacementPreflightError::SourceDestroyAlreadyPending {
                source_id: request.source_entity_id,
            },
        );
    }
    if !entities.iter_all().any(|entity| {
        entity.id == request.main_base_entity_id
            && entity.entity_type == FIRST_WORLD_MAIN_BASE_ENTITY_TYPE
    }) {
        return Err(MainBaseReplacementPreflightError::MainBaseUnavailable {
            main_base_id: request.main_base_entity_id,
        });
    }

    let prepare_surface = || {
        terrain
            .map(|terrain| {
                crate::entity_initializer::constructor_surface_bits_at_tick(
                    request.position_raw,
                    terrain,
                    retail_tick,
                    entities.common_environment_physics().waves_enabled,
                )
                .ok_or(MainBaseReplacementPreflightError::ScientistConstructor(
                    MainBaseScientistSpawnError::ScientistTerrainUnavailable,
                ))
            })
            .transpose()
    };
    if request.replacement_type == 7 {
        let root = entities
            .prepare_diver_root(request.position_raw, terrain)
            .map_err(MainBaseReplacementPreflightError::ScientistConstructor)?;
        return Ok(PreparedFirstWorldMainBaseReplacement {
            request,
            constructor: PreparedReplacementConstructor::Diver(root),
            constructor_surface_bits: prepare_surface()?,
        });
    }

    let (scientist_capability_flags, range, behavior_choices) = {
        let metadata = entities
            .main_base_scientist_metadata(request.replacement_type)
            .ok_or(MainBaseReplacementPreflightError::ScientistMetadataUnavailable)?;
        let initializer = metadata
            .initializer
            .as_ref()
            .ok_or(MainBaseReplacementPreflightError::ScientistInitializerUnavailable)?;
        if request.replacement_type != FIRST_WORLD_SCIENTIST_ENTITY_TYPE {
            crate::intro2_type8::validate_worker_metadata(request.replacement_type, metadata)
                .map_err(|_| {
                    MainBaseReplacementPreflightError::ScientistConstructor(
                        MainBaseScientistSpawnError::NativeWorkerPublication,
                    )
                })?;
        }
        (
            metadata.capability_flags,
            WrappedAxisRange::from_raw(initializer.common_axis_descriptor.strict_axis_limit_raw),
            initializer.behavior_choices.to_vec(),
        )
    };

    let constructor_surface_bits = prepare_surface()?;
    let birth_position_raw = entities
        .type8_scientist_birth_position_raw(request.replacement_type, request.position_raw, terrain)
        .map_err(MainBaseReplacementPreflightError::ScientistConstructor)?;
    let candidates = entities
        .iter_all()
        .map(|entity| JobNearbyCandidate {
            id: entity.id,
            position_raw: entity.position_raw(),
            state_flags: entity.collision.state_flags_at_0x08,
            capacity: match entity.base_factory_runtime {
                RetailRuntimeValue::Known(Some(state)) => {
                    RetailRuntimeValue::Known(Some(JobCapacityState {
                        current_jobs_raw: i32::from(state.current_scientists),
                        capacity_raw: i32::from(state.required_scientists),
                    }))
                }
                RetailRuntimeValue::Known(None) => RetailRuntimeValue::Known(None),
                RetailRuntimeValue::Unresolved => RetailRuntimeValue::Unresolved,
            },
        })
        .collect::<Vec<_>>();
    if behavior_choices.len() > MAX_BEHAVIOR_CHOICES {
        return Err(MainBaseReplacementPreflightError::BehaviorSelection(
            JobAwareBehaviorSelectionError::Selection(BehaviorSelectionError::TooManyChoices {
                count: behavior_choices.len(),
            }),
        ));
    }
    for choice in &behavior_choices {
        match BehaviorWeightRule::from_raw(choice.weight_rule_id) {
            Some(BehaviorWeightRule::Always | BehaviorWeightRule::JobNearby) => {}
            _ => {
                return Err(MainBaseReplacementPreflightError::BehaviorSelection(
                    JobAwareBehaviorSelectionError::UnsupportedWeightRule {
                        raw: choice.weight_rule_id,
                    },
                ));
            }
        }
    }
    let job_nearby = evaluate_job_nearby(JobNearbyEvaluationRequest {
        // The replacement is not published in the intrusive list until
        // after selection. Reusing the source id only skips the pending
        // source, which cannot itself supply a factory job.
        owner: JobNearbyOwner {
            id: request.source_entity_id,
            position_raw: birth_position_raw,
            capability_flags: RetailRuntimeValue::Known(scientist_capability_flags),
        },
        candidates_in_intrusive_order: &candidates,
        range,
    })
    .map_err(|source| {
        MainBaseReplacementPreflightError::BehaviorSelection(
            JobAwareBehaviorSelectionError::JobNearby(source),
        )
    })?;
    let mut total = 0_i32;
    let mut go_to_job_can_be_selected = false;
    let mut wander_near_can_be_selected = false;
    for (choice_index, choice) in behavior_choices.iter().enumerate() {
        let enabled = match BehaviorWeightRule::from_raw(choice.weight_rule_id) {
            Some(BehaviorWeightRule::Always) => 1,
            Some(BehaviorWeightRule::JobNearby) => i32::from(job_nearby),
            _ => unreachable!("the bounded rule set was validated above"),
        };
        let weight = enabled.wrapping_mul(choice.weight_multiplier as i32);
        if weight < 0 {
            return Err(MainBaseReplacementPreflightError::NegativeEffectiveWeight {
                choice_index,
                weight,
            });
        }
        if weight > 0 && behavior_program(choice.behavior_class_id).is_none() {
            return Err(MainBaseReplacementPreflightError::BehaviorSelection(
                JobAwareBehaviorSelectionError::Selection(
                    BehaviorSelectionError::UnknownBehaviorClass {
                        raw: choice.behavior_class_id,
                    },
                ),
            ));
        }
        if weight > 0 && choice.behavior_class_id == 54 {
            go_to_job_can_be_selected = true;
        }
        if weight > 0 && choice.behavior_class_id == 6 {
            wander_near_can_be_selected = true;
        }
        total = total.wrapping_add(weight);
    }
    if total <= 0 {
        return Err(MainBaseReplacementPreflightError::NoBehaviorSelected);
    }
    if wander_near_can_be_selected
        && !entities.type8_scientist_wander_near_constructor_ready(request.replacement_type)
    {
        return Err(MainBaseReplacementPreflightError::ScientistConstructor(
            MainBaseScientistSpawnError::WanderNearSubARuntimeUnavailable,
        ));
    }

    let prepared_go_to_job = go_to_job_can_be_selected
        .then(|| {
            entities
                .prepare_type8_scientist_go_to_job(request.replacement_type, birth_position_raw)
                .map_err(MainBaseReplacementPreflightError::ScientistConstructor)
        })
        .transpose()?;

    Ok(PreparedFirstWorldMainBaseReplacement {
        request,
        constructor: PreparedReplacementConstructor::TwoChoice {
            behavior_choices: behavior_choices.into_boxed_slice(),
            job_nearby,
            prepared_go_to_job,
        },
        constructor_surface_bits,
    })
}

/// Apply one preflighted replacement after its source destruction has been
/// queued. The common-body stamp precedes Sub-D/Sub-A construction and the
/// shared selector; the chosen graph then consumes its own constructor words.
/// A later failure cannot refund the body attempt or the caller's destruction
/// prefix.
pub fn apply_prepared_first_world_main_base_replacement(
    entities: &mut EntityManager,
    terrain: Option<&TerrainGrid>,
    world_fx: &mut WorldFx,
    plan: PreparedFirstWorldMainBaseReplacement,
) -> MainBaseScientistSpawned {
    let PreparedFirstWorldMainBaseReplacement {
        request,
        constructor,
        constructor_surface_bits,
    } = plan;
    entities
        .preflight_main_base_scientist_append(request.source_entity_id, request.main_base_entity_id)
        .expect("replacement source destruction and live Main Base precede104B0");
    let spawned = match constructor {
        PreparedReplacementConstructor::Diver(root) => entities
            .append_main_base_diver(request, terrain, constructor_surface_bits, root, world_fx)
            .expect("prepared diver prerequisites stay valid through local apply"),
        PreparedReplacementConstructor::TwoChoice {
            behavior_choices,
            job_nearby,
            prepared_go_to_job,
        } => {
            let body = entities.begin_type8_body_construction(request.replacement_type, world_fx);
            let selected_behavior = select_initial_behavior(
                &behavior_choices,
                |rule| match rule {
                    BehaviorWeightRule::Always => 1,
                    BehaviorWeightRule::JobNearby => i32::from(job_nearby),
                    _ => unreachable!("the prepared rule set is bounded"),
                },
                || u32::from(world_fx.next_shared_retail_random_u16()),
            )
            .expect("prepared behavior records stay valid through local apply")
            .expect("positive prepared weights always select one behavior");

            entities
                .append_main_base_scientist(
                    MainBaseScientistSpawnRequest {
                        entity_type: request.replacement_type,
                        body,
                        source_id: request.source_entity_id,
                        main_base_id: request.main_base_entity_id,
                        position_raw: request.position_raw,
                        selected_behavior,
                        prepared_go_to_job,
                        constructor_surface_bits,
                        terrain,
                    },
                    || u32::from(world_fx.next_shared_retail_random_u16()),
                )
                .expect("prepared replacement prerequisites stay valid through local apply")
        }
    };
    world_fx
        .queue_cargo_transfer_particle(spawned.replacement_position, Some(spawned.replacement_id));
    spawned
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor_task_owner::ActorTaskSlot;
    use crate::entity::MainBaseConversionDestroyQueueOutcome;
    use crate::entity_collision_state::{
        CommonMoverComponentTopology, EntityInitializerSpec, EntityTypeRuntimeMetadata,
    };
    use v2k_formats::collision::{BehaviorChoice, SubAPropulsionDescriptor};
    use v2k_formats::levels::{EntitySpawn, LevelDescriptor};

    const SOURCE_ID: u32 = 1;
    const MAIN_BASE_ID: u32 = 2;
    const SOURCE_ENTITY_TYPE: u32 = 9;
    const MAIN_BASE_ENTITY_TYPE: u32 = 6;
    const SOURCE_CONVERSION_CAPABILITY_BIT: u32 = 0x800;
    const SCIENTIST_JOB_CAPABILITY_BIT: u32 = 0x400;

    fn exact_initializer(behavior_choices: Vec<BehaviorChoice>) -> EntityInitializerSpec {
        EntityInitializerSpec {
            initializer_state_flags_raw: 0,
            common_axis_descriptor: Default::default(),
            behavior_choices: behavior_choices.into_boxed_slice(),
            behavior_rule_ref: 0,
            alternate_behavior_class_ref: 0,
        }
    }

    fn level(entity_types: &[u32]) -> LevelDescriptor {
        let entities = entity_types
            .iter()
            .copied()
            .into_iter()
            .enumerate()
            .map(|(index, entity_type)| EntitySpawn {
                index,
                entity_type,
                pos_data_1: [128, 0, 128, 0],
                pos_data_2: [0; 4],
                param: 0,
                rotation: [0; 3],
                extra: [0; 40],
                initial_damage_buffer_raw: 0,
                model_overrides: [0; 4],
                has_animation: false,
                anim_frames: 0,
                animation: None,
                has_config: false,
                config: None,
            })
            .collect::<Vec<_>>();
        LevelDescriptor {
            raw_header: [0; 0xd0],
            name: "Main Base conversion runtime test".into(),
            world_style: 0,
            terrain_sprite_base: 0,
            sky_color_index: 0,
            sky_model: 0,
            main_base_abort_sky_color_index: 0,
            main_base_abort_sky_model: 0,
            terrain_draw_depth: 30,
            sub_count: entities.len() as u32,
            campaign_record_count: 0,
            entities,
            campaign_records: Vec::new(),
        }
    }

    fn entities_with_choices_and_topology(
        behavior_choices: Vec<BehaviorChoice>,
        exact_go_to_job_topology: bool,
    ) -> EntityManager {
        let mut metadata = vec![EntityTypeRuntimeMetadata::default(); 10];
        metadata[SOURCE_ENTITY_TYPE as usize] = EntityTypeRuntimeMetadata {
            capability_flags: SOURCE_CONVERSION_CAPABILITY_BIT,
            status_component_descriptor: RetailRuntimeValue::Known(None),
            actor_animation_descriptor: RetailRuntimeValue::Known(None),
            initializer: Some(exact_initializer(Vec::new())),
            ..EntityTypeRuntimeMetadata::default()
        };
        metadata[MAIN_BASE_ENTITY_TYPE as usize] = EntityTypeRuntimeMetadata {
            status_component_descriptor: RetailRuntimeValue::Known(None),
            actor_animation_descriptor: RetailRuntimeValue::Known(None),
            initializer: Some(exact_initializer(Vec::new())),
            ..EntityTypeRuntimeMetadata::default()
        };
        metadata[FIRST_WORLD_SCIENTIST_ENTITY_TYPE as usize] = EntityTypeRuntimeMetadata {
            capability_flags: SCIENTIST_JOB_CAPABILITY_BIT,
            status_component_descriptor: RetailRuntimeValue::Known(None),
            actor_animation_descriptor: RetailRuntimeValue::Known(None),
            sub_a_propulsion_descriptor: exact_go_to_job_topology
                .then_some(RetailRuntimeValue::Known(Some(SubAPropulsionDescriptor {
                    acceleration_raw: 1,
                    overspeed_correction_raw: 2,
                    target_speed_base_raw: 0x0300,
                })))
                .unwrap_or(RetailRuntimeValue::Unresolved),
            common_mover_topology: exact_go_to_job_topology
                .then_some(RetailRuntimeValue::Known(CommonMoverComponentTopology {
                    sub_a: true,
                    ..CommonMoverComponentTopology::default()
                }))
                .unwrap_or(RetailRuntimeValue::Unresolved),
            initializer: Some(exact_initializer(behavior_choices)),
            ..EntityTypeRuntimeMetadata::default()
        };
        EntityManager::from_level_with_type_metadata(
            &level(&[SOURCE_ENTITY_TYPE, MAIN_BASE_ENTITY_TYPE]),
            &metadata,
            None,
        )
    }

    fn entities_with_choices(behavior_choices: Vec<BehaviorChoice>) -> EntityManager {
        entities_with_choices_and_topology(behavior_choices, false)
    }

    fn authored_choices() -> Vec<BehaviorChoice> {
        vec![
            BehaviorChoice {
                weight_rule_id: 13,
                weight_multiplier: 100,
                behavior_class_id: 54,
            },
            BehaviorChoice {
                weight_rule_id: 1,
                weight_multiplier: 1,
                behavior_class_id: 6,
            },
        ]
    }

    fn entities() -> EntityManager {
        entities_with_choices_and_topology(authored_choices(), true)
    }

    fn request(source_id: u32, main_base_id: u32) -> MainBaseReplacementSpawn {
        MainBaseReplacementSpawn {
            source_entity_id: source_id,
            main_base_entity_id: main_base_id,
            replacement_type: FIRST_WORLD_SCIENTIST_ENTITY_TYPE,
            position_raw: [0x1200, -0x0200, 0x3400],
        }
    }

    fn diver_fixture() -> (EntityManager, TerrainGrid, MainBaseReplacementSpawn) {
        let data = v2k_test_support::retail_dir();
        let mut session = crate::session::GameSession::init(&data).expect("retail corpus required");
        session.load_auxiliary_ovl(3, 1).unwrap();
        let metadata = EntityTypeRuntimeMetadata::from_section12(
            session
                .cache
                .global_entity_type(7)
                .expect("canonical diver"),
        );
        // Preserve the existing source/Base transaction fixture; only its
        // unpublished replacement uses the complete canonical Type7 record.
        let mut entities = entities();
        *entities.type_runtime_metadata_mut_for_test(7).unwrap() = metadata;
        let terrain = TerrainGrid {
            header: [512, 0, 0, 0, 0],
            cells: vec![
                v2k_formats::terrain::TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0,
                };
                v2k_formats::terrain::GRID_SIZE * v2k_formats::terrain::GRID_SIZE
            ],
        };
        let request = MainBaseReplacementSpawn {
            replacement_type: 7,
            ..request(SOURCE_ID, MAIN_BASE_ID)
        };
        (entities, terrain, request)
    }

    #[v2k_test_support::retail_test]
    fn type7_replacement_uses_its_complete_native_constructor_and_single_outer_particle() {
        let (mut entities, terrain, request) = diver_fixture();
        let before = entities.retail_live_order_ids().collect::<Vec<_>>();
        let plan =
            prepare_first_world_main_base_replacement(&entities, Some(&terrain), request, 123)
                .unwrap();
        assert!(matches!(
            &plan.constructor,
            PreparedReplacementConstructor::Diver(_)
        ));
        assert_eq!(entities.retail_live_order_ids().collect::<Vec<_>>(), before);
        assert!(entities
            .pending_main_base_conversion_destroy_ids()
            .is_empty());
        assert_eq!(
            entities.queue_main_base_conversion_destroy(SOURCE_ID),
            MainBaseConversionDestroyQueueOutcome::Queued {
                source_id: SOURCE_ID
            }
        );
        let mut expected = WorldFx::new();
        for _ in 0..3 {
            let _ = expected.next_shared_retail_random_u16();
        }
        let mut fx = WorldFx::new();
        let spawned = apply_prepared_first_world_main_base_replacement(
            &mut entities,
            Some(&terrain),
            &mut fx,
            plan,
        );
        let diver = entities
            .iter_all()
            .find(|entity| entity.id == spawned.replacement_id)
            .unwrap();
        assert_eq!(diver.entity_type, 7);
        assert_eq!(diver.authored_spawn_index, None);
        assert_eq!(diver.model_slots, [Some(1249); 4]);
        assert_eq!(
            diver.position_raw(),
            [request.position_raw[0], 0, request.position_raw[2]]
        );
        assert_eq!(
            diver.collision.recent_relation_id_at_0x60,
            RetailRuntimeValue::Known(Some(MAIN_BASE_ID))
        );
        assert!(
            crate::native_type86::native_type86_manager_allocation_authenticates(
                &entities,
                spawned.replacement_id
            )
        );
        assert!(matches!(
            diver.actor_task_state(ActorTaskSlot::Primary),
            Some(crate::actor_task_dispatcher::ActorTaskRuntime::OrdinaryType9Wander(_))
        ));
        assert_eq!(fx.next_sub_d_allocation_seed(), 1);
        assert_eq!(
            fx.pending_event_count(),
            1,
            "the outer callback queues operation33 once"
        );
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16(),
            "20450, selector and class6 suffix each draw once"
        );
    }

    #[v2k_test_support::retail_test]
    fn type7_invalid_descriptor_fails_before_the_conversion_prefix() {
        let (mut entities, terrain, request) = diver_fixture();
        entities
            .type_runtime_metadata_mut_for_test(7)
            .unwrap()
            .actor_animation_descriptor = RetailRuntimeValue::Unresolved;
        let before = entities.retail_live_order_ids().collect::<Vec<_>>();
        assert!(matches!(
            prepare_first_world_main_base_replacement(&entities, Some(&terrain), request, 123),
            Err(MainBaseReplacementPreflightError::ScientistConstructor(_))
        ));
        assert_eq!(entities.retail_live_order_ids().collect::<Vec<_>>(), before);
        assert!(entities
            .pending_main_base_conversion_destroy_ids()
            .is_empty());
    }

    #[test]
    fn successful_class6_apply_consumes_component_selector_and_task_draws() {
        let mut entities = entities();
        let callback_position = request(SOURCE_ID, MAIN_BASE_ID).position_raw;
        let plan = prepare_first_world_main_base_replacement(
            &entities,
            None,
            request(SOURCE_ID, MAIN_BASE_ID),
            0,
        )
        .expect("proven replacement preflight");
        assert_eq!(
            entities.queue_main_base_conversion_destroy(SOURCE_ID),
            MainBaseConversionDestroyQueueOutcome::Queued {
                source_id: SOURCE_ID
            }
        );
        let mut expected_rng = WorldFx::new();
        let _component_sub_a_word = expected_rng.next_shared_retail_random_u16();
        let _consumed_selection_word = expected_rng.next_shared_retail_random_u16();
        let consumed_constructor_word = expected_rng.next_shared_retail_random_u16();
        let expected_next_word = expected_rng.next_shared_retail_random_u16();
        let mut world_fx = WorldFx::new();

        let spawned = apply_prepared_first_world_main_base_replacement(
            &mut entities,
            None,
            &mut world_fx,
            plan,
        );

        assert_eq!(spawned.source_id, SOURCE_ID);
        assert_eq!(spawned.main_base_id, MAIN_BASE_ID);
        let replacement = entities
            .iter_all()
            .find(|entity| entity.id == spawned.replacement_id)
            .expect("tail-appended replacement");
        assert_eq!(replacement.entity_type, FIRST_WORLD_SCIENTIST_ENTITY_TYPE);
        // This focused task fixture has no Sub-D descriptor. Construction
        // must not invent a captured first-world allocation for missing data.
        assert!(replacement.type8_sub_d_frame_owner.is_none());
        assert!(replacement.type8_sub_d_runtime.is_none());
        assert_eq!(world_fx.next_sub_d_allocation_seed(), 0);
        assert_eq!(
            replacement.physical_body_basis_q31(),
            RetailRuntimeValue::Known(
                crate::common_mover::type9_attitude::Type9BodyBasis::from_angle_words(0, 0, 0)
            ),
        );
        assert_eq!(replacement.position_raw(), callback_position);
        assert_eq!(
            replacement.collision.recent_relation_id_at_0x60,
            RetailRuntimeValue::Known(Some(MAIN_BASE_ID))
        );
        let RetailRuntimeValue::Known(Some(selection)) = replacement.initial_behavior else {
            panic!("fallback scientist behavior must resolve exactly");
        };
        assert_eq!(selection.program.class_id, 6);
        assert!(matches!(
            replacement.actor_task_state(ActorTaskSlot::Primary),
            Some(crate::actor_task_dispatcher::ActorTaskRuntime::OrdinaryType9Wander(state))
                if state.elapsed_ms() == 0
        ));
        assert_eq!(replacement.actor_task_state(ActorTaskSlot::Secondary), None);
        assert_eq!(replacement.actor_task_state(ActorTaskSlot::Tertiary), None);
        let RetailRuntimeValue::Known(Some(sub_a)) = replacement.sub_a_propulsion_runtime else {
            panic!("class-6 constructor must bind the replacement's own Sub-A runtime")
        };
        assert_eq!(
            sub_a.target_speed_raw(),
            RetailRuntimeValue::Known(crate::common_mover::shared_initializer_target_speed_raw(
                0x0300,
                consumed_constructor_word,
            ))
        );
        assert_eq!(sub_a.direction_multiplier(), 1);
        assert_eq!(sub_a.drive_scale_percent(), 100);
        assert_eq!(world_fx.pending_event_count(), 1);
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            expected_next_word,
            "20450 precedes the selector and class-6 task/Sub-A suffix draws"
        );
    }

    #[test]
    fn successful_class54_apply_uses_the_preflighted_one_shot_task_plan() {
        let mut entities = entities_with_choices_and_topology(
            vec![BehaviorChoice {
                weight_rule_id: 1,
                weight_multiplier: 1,
                behavior_class_id: 54,
            }],
            true,
        );
        let plan = prepare_first_world_main_base_replacement(
            &entities,
            None,
            request(SOURCE_ID, MAIN_BASE_ID),
            0,
        )
        .expect("class-54 constructor preflight");
        assert_eq!(
            entities.queue_main_base_conversion_destroy(SOURCE_ID),
            MainBaseConversionDestroyQueueOutcome::Queued {
                source_id: SOURCE_ID
            }
        );
        let mut expected_rng = WorldFx::new();
        let _component_sub_a_word = expected_rng.next_shared_retail_random_u16();
        let _selection_word = expected_rng.next_shared_retail_random_u16();
        let _go_to_job_constructor_word = expected_rng.next_shared_retail_random_u16();
        let expected_next_word = expected_rng.next_shared_retail_random_u16();
        let mut world_fx = WorldFx::new();

        let spawned = apply_prepared_first_world_main_base_replacement(
            &mut entities,
            None,
            &mut world_fx,
            plan,
        );

        let replacement = entities
            .iter_all()
            .find(|entity| entity.id == spawned.replacement_id)
            .expect("tail-appended replacement");
        // This focused task fixture has no Sub-D descriptor. Construction
        // must not invent a captured first-world allocation for missing data.
        assert!(replacement.type8_sub_d_frame_owner.is_none());
        assert!(replacement.type8_sub_d_runtime.is_none());
        assert_eq!(world_fx.next_sub_d_allocation_seed(), 0);
        assert_eq!(
            replacement.physical_body_basis_q31(),
            RetailRuntimeValue::Known(
                crate::common_mover::type9_attitude::Type9BodyBasis::from_angle_words(0, 0, 0)
            ),
        );
        assert!(matches!(
            replacement.actor_task_state(ActorTaskSlot::Primary),
            Some(crate::actor_task_dispatcher::ActorTaskRuntime::GoToJob(_))
        ));
        let RetailRuntimeValue::Known(Some(sub_a)) = replacement.sub_a_propulsion_runtime else {
            panic!("class-54 constructor must bind the replacement's own Sub-A runtime");
        };
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(0x0400));
        assert_eq!(sub_a.direction_multiplier(), 1);
        assert_eq!(sub_a.drive_scale_percent(), 100);
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            expected_next_word,
            "20450 precedes the selector and class-54 task-constructor draws"
        );
    }

    #[test]
    fn wrong_source_fails_during_read_only_preflight() {
        let entities = entities();
        assert_eq!(
            prepare_first_world_main_base_replacement(
                &entities,
                None,
                request(0x04c2_0001, MAIN_BASE_ID),
                0,
            ),
            Err(MainBaseReplacementPreflightError::SourceUnavailable {
                source_id: 0x04c2_0001,
            })
        );
    }

    #[test]
    fn unsupported_rules_unknown_programs_and_zero_weights_fail_in_preflight() {
        let cases = [
            (
                vec![BehaviorChoice {
                    weight_rule_id: 2,
                    weight_multiplier: 1,
                    behavior_class_id: 6,
                }],
                MainBaseReplacementPreflightError::BehaviorSelection(
                    JobAwareBehaviorSelectionError::UnsupportedWeightRule { raw: 2 },
                ),
            ),
            (
                vec![BehaviorChoice {
                    weight_rule_id: 1,
                    weight_multiplier: 1,
                    behavior_class_id: 255,
                }],
                MainBaseReplacementPreflightError::BehaviorSelection(
                    JobAwareBehaviorSelectionError::Selection(
                        BehaviorSelectionError::UnknownBehaviorClass { raw: 255 },
                    ),
                ),
            ),
            (
                vec![BehaviorChoice {
                    weight_rule_id: 1,
                    weight_multiplier: 0,
                    behavior_class_id: 6,
                }],
                MainBaseReplacementPreflightError::NoBehaviorSelected,
            ),
        ];

        for (choices, expected) in cases {
            let entities = entities_with_choices(choices);
            assert_eq!(
                prepare_first_world_main_base_replacement(
                    &entities,
                    None,
                    request(SOURCE_ID, MAIN_BASE_ID),
                    0,
                ),
                Err(expected),
            );
            assert!(entities
                .pending_main_base_conversion_destroy_ids()
                .is_empty());
        }
    }

    #[test]
    fn class54_constructor_dependencies_fail_before_destroy_or_shared_rng() {
        let entities = entities_with_choices_and_topology(
            vec![BehaviorChoice {
                weight_rule_id: 1,
                weight_multiplier: 1,
                behavior_class_id: 54,
            }],
            false,
        );
        let before_order = entities.retail_live_order_ids().collect::<Vec<_>>();
        let mut untouched_rng = WorldFx::new();
        let mut expected_rng = WorldFx::new();
        let expected_first_word = expected_rng.next_shared_retail_random_u16();

        assert_eq!(
            prepare_first_world_main_base_replacement(
                &entities,
                None,
                request(SOURCE_ID, MAIN_BASE_ID),
                0,
            ),
            Err(MainBaseReplacementPreflightError::ScientistConstructor(
                MainBaseScientistSpawnError::GoToJobSetup(
                    crate::go_to_job::GoToJobSetupError::ConstructorSuffix(
                        crate::go_to_job::GoToJobConstructorSuffixError::
                            UnresolvedComponentTopology,
                    ),
                ),
            )),
        );
        assert_eq!(
            entities.retail_live_order_ids().collect::<Vec<_>>(),
            before_order,
            "failed constructor preflight must not publish a replacement"
        );
        assert!(
            entities
                .pending_main_base_conversion_destroy_ids()
                .is_empty(),
            "event/deferred-destroy apply has not begun"
        );
        assert_eq!(
            untouched_rng.next_shared_retail_random_u16(),
            expected_first_word,
            "class-54 preflight never owns the selector RNG draw"
        );
    }

    #[test]
    fn class6_constructor_dependencies_fail_before_destroy_or_shared_rng() {
        let entities = entities_with_choices_and_topology(
            vec![BehaviorChoice {
                weight_rule_id: 1,
                weight_multiplier: 1,
                behavior_class_id: 6,
            }],
            false,
        );
        let before_order = entities.retail_live_order_ids().collect::<Vec<_>>();
        let mut untouched_rng = WorldFx::new();
        let mut expected_rng = WorldFx::new();
        let expected_first_word = expected_rng.next_shared_retail_random_u16();

        assert_eq!(
            prepare_first_world_main_base_replacement(
                &entities,
                None,
                request(SOURCE_ID, MAIN_BASE_ID),
                0,
            ),
            Err(MainBaseReplacementPreflightError::ScientistConstructor(
                MainBaseScientistSpawnError::WanderNearSubARuntimeUnavailable,
            )),
        );
        assert_eq!(
            entities.retail_live_order_ids().collect::<Vec<_>>(),
            before_order
        );
        assert!(entities
            .pending_main_base_conversion_destroy_ids()
            .is_empty());
        assert_eq!(
            untouched_rng.next_shared_retail_random_u16(),
            expected_first_word,
            "class-6 topology preflight never owns the selector RNG draw"
        );
    }

    #[test]
    fn missing_main_base_fails_before_any_source_mutation() {
        let mut metadata = vec![EntityTypeRuntimeMetadata::default(); 10];
        metadata[SOURCE_ENTITY_TYPE as usize] = EntityTypeRuntimeMetadata {
            capability_flags: SOURCE_CONVERSION_CAPABILITY_BIT,
            status_component_descriptor: RetailRuntimeValue::Known(None),
            actor_animation_descriptor: RetailRuntimeValue::Known(None),
            initializer: Some(exact_initializer(Vec::new())),
            ..EntityTypeRuntimeMetadata::default()
        };
        metadata[FIRST_WORLD_SCIENTIST_ENTITY_TYPE as usize] = EntityTypeRuntimeMetadata {
            capability_flags: SCIENTIST_JOB_CAPABILITY_BIT,
            status_component_descriptor: RetailRuntimeValue::Known(None),
            actor_animation_descriptor: RetailRuntimeValue::Known(None),
            initializer: Some(exact_initializer(authored_choices())),
            ..EntityTypeRuntimeMetadata::default()
        };
        let entities = EntityManager::from_level_with_type_metadata(
            &level(&[SOURCE_ENTITY_TYPE]),
            &metadata,
            None,
        );

        assert_eq!(
            prepare_first_world_main_base_replacement(
                &entities,
                None,
                request(SOURCE_ID, MAIN_BASE_ID),
                0,
            ),
            Err(MainBaseReplacementPreflightError::MainBaseUnavailable {
                main_base_id: MAIN_BASE_ID,
            }),
        );
        assert!(entities
            .pending_main_base_conversion_destroy_ids()
            .is_empty());
    }

    #[test]
    fn unresolved_nearby_factory_capacity_fails_before_shared_rng_is_owned() {
        const FACTORY_TYPE: u32 = 17;
        const FACTORY_ID: u32 = 3;
        let mut metadata = vec![EntityTypeRuntimeMetadata::default(); 18];
        for entity_type in [SOURCE_ENTITY_TYPE, MAIN_BASE_ENTITY_TYPE, FACTORY_TYPE] {
            metadata[entity_type as usize] = EntityTypeRuntimeMetadata {
                capability_flags: (entity_type == SOURCE_ENTITY_TYPE)
                    .then_some(SOURCE_CONVERSION_CAPABILITY_BIT)
                    .unwrap_or(0),
                status_component_descriptor: if entity_type == FACTORY_TYPE {
                    RetailRuntimeValue::Unresolved
                } else {
                    RetailRuntimeValue::Known(None)
                },
                actor_animation_descriptor: RetailRuntimeValue::Known(None),
                initializer: Some(exact_initializer(Vec::new())),
                ..EntityTypeRuntimeMetadata::default()
            };
        }
        metadata[FIRST_WORLD_SCIENTIST_ENTITY_TYPE as usize] = EntityTypeRuntimeMetadata {
            capability_flags: SCIENTIST_JOB_CAPABILITY_BIT,
            status_component_descriptor: RetailRuntimeValue::Known(None),
            actor_animation_descriptor: RetailRuntimeValue::Known(None),
            initializer: Some(exact_initializer(authored_choices())),
            ..EntityTypeRuntimeMetadata::default()
        };
        let entities = EntityManager::from_level_with_type_metadata(
            &level(&[SOURCE_ENTITY_TYPE, MAIN_BASE_ENTITY_TYPE, FACTORY_TYPE]),
            &metadata,
            None,
        );

        assert_eq!(
            prepare_first_world_main_base_replacement(
                &entities,
                None,
                request(SOURCE_ID, MAIN_BASE_ID),
                0,
            ),
            Err(MainBaseReplacementPreflightError::BehaviorSelection(
                JobAwareBehaviorSelectionError::JobNearby(
                    crate::job_nearby::JobNearbyEvaluationError::CandidateCapacityUnresolved {
                        id: FACTORY_ID,
                    },
                ),
            )),
        );
        assert!(entities
            .pending_main_base_conversion_destroy_ids()
            .is_empty());
    }
}
