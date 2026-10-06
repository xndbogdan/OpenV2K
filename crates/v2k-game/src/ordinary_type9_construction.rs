//! Shared native Type-9 construction, independent of a captured world/spawn.
//!
//! `104B0 -> 09A80 -> D4A0 -> 25680 -> ABE0` owns the authored state,
//! component construction, immutable grounded anchor and four-way selector.
//! The process constructor supplies Sub-D's real allocation and surface result;
//! this owner never borrows a captured seed or first-query cache reset.

use v2k_formats::{levels::EntitySpawn, terrain::TerrainGrid};

use crate::{
    actor_animation::ActorAnimationController,
    actor_task_owner::ActorTaskSlot,
    common_mover::{
        sub_d::{Type9SubDFrameOwner, Type9SubDRuntime},
        SubAPropulsionRuntime,
    },
    descriptor_contact::DESCRIPTOR_CONTACT_CALLBACK_ADDRESS,
    entity::{Entity, EntityManager},
    entity_behavior::translate_state_policy,
    entity_collision_state::{
        EntityPairCallbackRuntimeState, EntityTypeRuntimeMetadata, PairOrientationPolicy,
        RetailRuntimeValue, RetailStateWord, ACTIVE_MODEL_SLOT_HIGH_STATE_BIT, SURFACE_STATE_MASK,
    },
    job_nearby::JobCapacityState,
    main_base_abort::MainBaseAbortActorLease,
    main_base_type9_abort::{
        exact_level_one_type9_metadata, LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR,
        LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR, LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR,
    },
    ordinary_type9_initial_production::{
        compose_fresh_level1_type9_initial_production, FreshLevel1Type9InitialProductionFailure,
        FreshLevel1Type9LinkReady, FreshLevel1Type9ProductionCandidate,
        InfallibleFreshLevel1Type9ProductionRuntime,
    },
    ordinary_type9_initial_selection::FreshLevel1Type9EntityRef,
    ordinary_type9_live::{
        FreshLevel1OrdinaryType9Admission, OrdinaryType9PendingInitialSelection,
    },
    world_fx::WorldFx,
};

#[cfg(test)]
#[path = "ordinary_type9_construction_tests.rs"]
mod cross_world_tests;

/// Constructor-only proof. Task graphs and mutable component contents retain
/// their own leases; this receipt authenticates the allocation and birth inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OrdinaryType9NativeReceipt {
    allocation: MainBaseAbortActorLease,
    spawn_index: usize,
    rotation_raw: [i16; 3],
    pre_publication_state: RetailStateWord,
    active_slot: usize,
    initial_sub_a: SubAPropulsionRuntime,
    components: OrdinaryType9PendingInitialSelection,
}

impl OrdinaryType9NativeReceipt {
    pub(crate) const fn initial_components(self) -> OrdinaryType9PendingInitialSelection {
        self.components
    }
    pub(crate) const fn authored_spawn_index(self) -> usize {
        self.spawn_index
    }
    pub(crate) const fn owner_id(self) -> u32 {
        self.allocation.entity_id
    }
    pub(crate) const fn pre_publication_state(self) -> RetailStateWord {
        self.pre_publication_state
    }
    pub(crate) const fn rotation_raw(self) -> [i16; 3] {
        self.rotation_raw
    }
    pub(crate) const fn active_slot(self) -> usize {
        self.active_slot
    }
    pub(crate) const fn initial_sub_a(self) -> SubAPropulsionRuntime {
        self.initial_sub_a
    }

    pub(crate) fn authenticates_entity(self, entity: &Entity) -> bool {
        entity.id == self.allocation.entity_id
            && entity.entity_type == 9
            && entity.authored_spawn_index == Some(self.spawn_index)
            && entity.model_slots == [Some(558); 4]
            && entity.capability_flags == 0x1804
            && entity.ordinary_type9_native_receipt == Some(self)
    }

    pub(crate) fn authenticates_birth(self, entity: &Entity) -> bool {
        self.authenticates_entity(entity)
            && entity.collision.state_flags_at_0x08 == self.pre_publication_state
            && entity.rotation_heading_pitch_roll_raw() == self.rotation_raw
            && entity.ordinary_type9_pending_initial_selection == Some(self.components)
            && self.components.immutable_anchor_raw_at_0x90()
                == RetailRuntimeValue::Known(entity.position_raw())
            && entity.sub_a_propulsion_runtime
                == RetailRuntimeValue::Known(Some(self.initial_sub_a))
    }
}

/// Entry points without a retained task lease must re-observe manager custody.
pub(crate) fn ordinary_type9_native_allocation_authenticates(
    manager: &EntityManager,
    id: u32,
) -> bool {
    let Some(entity) = manager.iter_all().find(|entity| entity.id == id) else {
        return false;
    };
    let Some(receipt) = entity.ordinary_type9_native_receipt else {
        return false;
    };
    receipt.authenticates_entity(entity)
        && manager
            .main_base_abort_actor_observation(id)
            .is_some_and(|observation| observation.lease == receipt.allocation)
}

pub(crate) struct OrdinaryType9ConstructionRequest<'a> {
    pub entity: Entity,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub spawn: &'a EntitySpawn,
    pub preceding: &'a [Entity],
    pub terrain: &'a TerrainGrid,
    pub allocation: MainBaseAbortActorLease,
    pub sub_d_frame_owner: Type9SubDFrameOwner,
    pub sub_d_runtime: Type9SubDRuntime,
    /// `104B0` compares the authored Y against the current sea/wave surface.
    /// Supplied by the shared process constructor, before D4A0 grounds Y.
    pub constructor_surface_bits: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OrdinaryType9ConstructionError {
    Identity,
    Metadata,
    AlreadyPublished,
    Prefix,
    Surface,
    Components,
    Production(FreshLevel1Type9InitialProductionFailure),
}

pub(crate) fn construct_ordinary_type9(
    request: OrdinaryType9ConstructionRequest<'_>,
    world_fx: &mut WorldFx,
) -> Result<FreshLevel1Type9LinkReady, OrdinaryType9ConstructionError> {
    let OrdinaryType9ConstructionRequest {
        mut entity,
        metadata,
        spawn,
        preceding,
        terrain,
        allocation,
        sub_d_frame_owner,
        sub_d_runtime,
        constructor_surface_bits,
    } = request;
    if entity.id != allocation.entity_id
        || !entity.active
        || entity.entity_type != 9
        || spawn.entity_type != 9
        || entity.authored_spawn_index != Some(spawn.index)
        || entity.model_slots != [Some(558); 4]
        || entity.rotation_heading_pitch_roll_raw() != spawn.rotation.map(|word| word as i16)
    {
        return Err(OrdinaryType9ConstructionError::Identity);
    }
    if !exact_level_one_type9_metadata(metadata) {
        return Err(OrdinaryType9ConstructionError::Metadata);
    }
    if entity.ordinary_type9_native_receipt.is_some()
        || entity.intro2_type9_runtime.is_some()
        || entity.ordinary_type9_pending_initial_selection.is_some()
        || entity.ordinary_type9_selected_component_runtime.is_some()
        || entity.main_base_type9_death_component_runtime.is_some()
        || entity.initial_behavior != RetailRuntimeValue::Unresolved
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .iter()
            .any(|&slot| entity.actor_task_state(slot).is_some())
    {
        return Err(OrdinaryType9ConstructionError::AlreadyPublished);
    }
    // The persistent player can precede authored records without a spawn index.
    // Every authored predecessor must still retain its actual list order.
    if preceding.iter().any(|actor| actor.id == entity.id || actor.authored_spawn_index.is_some_and(|index| index >= spawn.index))
        || preceding.windows(2).any(|pair| matches!((pair[0].authored_spawn_index, pair[1].authored_spawn_index), (Some(a), Some(b)) if a >= b)) {
        return Err(OrdinaryType9ConstructionError::Prefix);
    }
    if constructor_surface_bits & !SURFACE_STATE_MASK != 0 {
        return Err(OrdinaryType9ConstructionError::Surface);
    }
    let authored = spawn.position_raw();
    let cell = terrain
        .cell(
            usize::from((authored[0] as u16) >> 8),
            usize::from((authored[2] as u16) >> 8),
        )
        .ok_or(OrdinaryType9ConstructionError::Surface)?;
    let active_slot = if cell.terrain_type & 0x10 != 0 { 2 } else { 0 };
    let anchor = [
        authored[0],
        terrain.bilinear_height_raw(authored[0], authored[2]),
        authored[2],
    ];
    if entity.collision.health_raw != RetailRuntimeValue::Known(1500)
        || entity.collision.recent_relation_id_at_0x60 != RetailRuntimeValue::Known(None)
        || entity.sub_a_propulsion_runtime
            != RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::pending_constructor_rng(
                LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR,
            )))
        || entity.actor_animation_runtime
            != RetailRuntimeValue::Known(Some(
                ActorAnimationController::from_descriptor(LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR)
                    .expect("audited Type9 Sub-I"),
            ))
        || sub_d_runtime != Type9SubDRuntime::from_constructor()
    {
        return Err(OrdinaryType9ConstructionError::Components);
    }
    let mut state =
        RetailStateWord::exact(0x0607_8801 | if spawn.param != 0 { 0x0100_0000 } else { 0 });
    state.overwrite(SURFACE_STATE_MASK, constructor_surface_bits);
    state.overwrite(
        ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
        if active_slot == 2 {
            ACTIVE_MODEL_SLOT_HIGH_STATE_BIT
        } else {
            0
        },
    );
    let policy = translate_state_policy(
        metadata
            .initializer
            .as_ref()
            .expect("exact metadata")
            .initializer_state_flags_raw,
    );
    state.overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    entity.collision.state_flags_at_0x08 = state;
    entity.set_position_raw(anchor);
    entity.actor_common_axis_descriptor =
        RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR);
    entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    entity.model_index = Some(558);
    entity.collision.pair_callbacks = EntityPairCallbackRuntimeState::audited_local(
        Some(DESCRIPTOR_CONTACT_CALLBACK_ADDRESS),
        RetailRuntimeValue::Known(PairOrientationPolicy::LiveHeadingWithAuthoredPitchRoll {
            pitch_raw: spawn.rotation[1],
            roll_raw: spawn.rotation[2],
        }),
    );
    // Explicit native allocation policy for the uninitialized transient +B2.
    // Unlike the old fixture, no captured first-scheduler state is installed.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    entity.collision.fresh_level1_type9_first_scheduler_pending = false;
    let initial_sub_a = SubAPropulsionRuntime::from_20450_constructor(
        LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR,
        world_fx.next_shared_retail_random_u16(),
    );
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(initial_sub_a));
    let components = OrdinaryType9PendingInitialSelection::from_native_components(
        anchor,
        sub_d_frame_owner,
        sub_d_runtime,
    );
    entity.ordinary_type9_pending_initial_selection = Some(components);
    let receipt = OrdinaryType9NativeReceipt {
        allocation,
        spawn_index: spawn.index,
        rotation_raw: spawn.rotation.map(|word| word as i16),
        pre_publication_state: state,
        active_slot,
        initial_sub_a,
        components,
    };
    entity.ordinary_type9_native_receipt = Some(receipt);
    let candidates: Vec<_> = preceding
        .iter()
        .map(|actor| {
            FreshLevel1Type9ProductionCandidate::new(
                FreshLevel1Type9EntityRef {
                    id: actor.id,
                    entity_type: actor.entity_type,
                    position_raw: actor.position_raw(),
                    state_flags_raw: actor.collision.state_flags_at_0x08,
                    capability_flags: RetailRuntimeValue::Known(actor.capability_flags),
                    attached_entity_handle: actor.collision.recent_relation_id_at_0x60,
                },
                match actor.base_factory_runtime {
                    RetailRuntimeValue::Known(Some(state)) => {
                        RetailRuntimeValue::Known(Some(JobCapacityState {
                            current_jobs_raw: i32::from(state.current_scientists),
                            capacity_raw: i32::from(state.required_scientists),
                        }))
                    }
                    RetailRuntimeValue::Known(None) => RetailRuntimeValue::Known(None),
                    RetailRuntimeValue::Unresolved => RetailRuntimeValue::Unresolved,
                },
            )
        })
        .collect();
    compose_fresh_level1_type9_initial_production(
        entity,
        FreshLevel1OrdinaryType9Admission::from_native(receipt),
        metadata,
        &candidates,
        &mut InfallibleFreshLevel1Type9ProductionRuntime::new(world_fx),
    )
    .map_err(OrdinaryType9ConstructionError::Production)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        common_mover::sub_d::{
            construct_native_sub_d, SubDAllocationCounter, ORDINARY_TYPE9_SUB_D,
        },
        entity::EntityKind,
        entity_collision_state::{EntityCollisionRuntimeState, BODY_BASIS_REBUILT_STATE_BIT},
        ordinary_type9_initial_production::FreshLevel1Type9InitialProductionBranch,
        ordinary_type9_wander_initializer::OrdinaryType9WanderInitializerOutcome,
        session::GameSession,
    };

    fn corpus() -> Option<(GameSession, EntityTypeRuntimeMetadata, EntitySpawn)> {
        let data = v2k_test_support::retail_dir();
        let mut session = GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(14, 1).unwrap();
        let metadata =
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(9).unwrap());
        let spawn = session
            .cache
            .level_desc()
            .unwrap()
            .entities
            .iter()
            .find(|spawn| spawn.entity_type == 9)
            .unwrap()
            .clone();
        Some((session, metadata, spawn))
    }

    // Controlled common-construction boundary, before native component/behavior
    // publication. Full authored-list loading is covered by the manager tests.
    fn provisional(metadata: &EntityTypeRuntimeMetadata, spawn: &EntitySpawn, id: u32) -> Entity {
        let mut entity = Entity::unresolved_port_entity(id, EntityKind::Unknown(9), 9);
        entity.authored_spawn_index = Some(spawn.index);
        entity.model_slots = [Some(558); 4];
        entity.model_index = Some(558);
        entity.mass_raw = metadata.mass_raw;
        entity.capability_flags = metadata.capability_flags;
        entity.collision = EntityCollisionRuntimeState::from_constructor(
            Some(metadata),
            spawn.initial_damage_buffer_raw,
            RetailStateWord::unknown(),
        );
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
            SubAPropulsionRuntime::pending_constructor_rng(LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR),
        ));
        entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(None);
        entity.actor_animation_runtime = RetailRuntimeValue::Known(Some(
            ActorAnimationController::from_descriptor(LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR).unwrap(),
        ));
        entity.set_position_raw(spawn.position_raw());
        entity.set_rotation_heading_pitch_roll_raw(spawn.rotation.map(|word| word as i16));
        entity
    }

    #[v2k_test_support::retail_test]
    fn native_birth_retains_authored_pose_slot_and_real_component_rng_order() {
        let Some((mut session, metadata, mut spawn)) = corpus() else {
            return;
        };
        // Exercise inputs outside all accepted Level1/Intro2 capture identities.
        spawn.index = 77;
        spawn.param = 1;
        spawn.rotation = [0x2000, 0x0800, 0x0400];
        let raw = spawn.position_raw();
        let terrain = session.cache.level_terrain_mut().unwrap();
        terrain.cells
            [usize::from((raw[0] as u16) >> 8) * 256 + usize::from((raw[2] as u16) >> 8)]
        .terrain_type |= 0x10;
        let native = construct_native_sub_d(
            &mut SubDAllocationCounter::from_next_seed(0xb7),
            ORDINARY_TYPE9_SUB_D,
        );
        let mut expected_fx = WorldFx::new();
        let constructor_word = expected_fx.next_shared_retail_random_u16();
        let selector_word = expected_fx.next_shared_retail_random_u16();
        let initializer_word = expected_fx.next_shared_retail_random_u16();
        let mut fx = WorldFx::new();
        let link = construct_ordinary_type9(
            OrdinaryType9ConstructionRequest {
                entity: provisional(&metadata, &spawn, 42),
                metadata: &metadata,
                spawn: &spawn,
                preceding: &[],
                terrain,
                allocation: MainBaseAbortActorLease {
                    entity_id: 42,
                    allocation_identity: 0x1_0000_002a,
                },
                sub_d_frame_owner: native.frame_owner,
                sub_d_runtime: native.runtime,
                constructor_surface_bits: 0,
            },
            &mut fx,
        )
        .unwrap();
        let (entity, owner, messages) = link.into_parts();
        let receipt = entity.ordinary_type9_native_receipt.unwrap();
        assert_eq!(
            entity.rotation_heading_pitch_roll_raw(),
            spawn.rotation.map(|word| word as i16)
        );
        assert_eq!(
            entity.collision.active_model_slot(),
            RetailRuntimeValue::Known(2)
        );
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(0x0100_0000),
            RetailRuntimeValue::Known(0x0100_0000)
        );
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(BODY_BASIS_REBUILT_STATE_BIT),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            entity.physical_body_basis_q31,
            RetailRuntimeValue::Unresolved
        );
        assert!(!entity.collision.fresh_level1_type9_first_scheduler_pending);
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            receipt.initial_components().sub_d_frame_owner,
            native.frame_owner
        );
        assert_eq!(
            receipt.initial_sub_a(),
            SubAPropulsionRuntime::from_20450_constructor(
                LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR,
                constructor_word
            )
        );
        let FreshLevel1Type9InitialProductionBranch::Wander(
            OrdinaryType9WanderInitializerOutcome::Published {
                selected,
                constructor,
                ..
            },
        ) = owner.branch()
        else {
            panic!("empty prefix must select Wander");
        };
        assert_eq!(selected.selector_random_word, u32::from(selector_word));
        assert_eq!(constructor.random_sample_low16, initializer_word);
        assert!(messages.is_empty());
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            expected_fx.next_shared_retail_random_u16()
        );
    }

    #[v2k_test_support::retail_test]
    fn native_birth_keeps_player_nearby_and_complete_attract_attention_publication() {
        let Some((session, metadata, spawn)) = corpus() else {
            return;
        };
        let terrain = session.cache.terrain().unwrap();
        let raw = spawn.position_raw();
        let anchor = [raw[0], terrain.bilinear_height_raw(raw[0], raw[2]), raw[2]];
        let mut player = Entity::unresolved_port_entity(1, EntityKind::Player, 46);
        player.capability_flags = 1;
        player.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        player.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        player.base_factory_runtime = RetailRuntimeValue::Known(None);
        player.set_position_raw(anchor);
        let preceding = [player];
        let mut found = false;
        for warmup in 0..32 {
            let mut fx = WorldFx::new();
            for _ in 0..warmup {
                fx.next_shared_retail_random_u16();
            }
            let native = construct_native_sub_d(
                &mut SubDAllocationCounter::from_next_seed(0x91),
                ORDINARY_TYPE9_SUB_D,
            );
            let link = construct_ordinary_type9(
                OrdinaryType9ConstructionRequest {
                    entity: provisional(&metadata, &spawn, 42),
                    metadata: &metadata,
                    spawn: &spawn,
                    preceding: &preceding,
                    terrain,
                    allocation: MainBaseAbortActorLease {
                        entity_id: 42,
                        allocation_identity: 0x2_0000_002a,
                    },
                    sub_d_frame_owner: native.frame_owner,
                    sub_d_runtime: native.runtime,
                    constructor_surface_bits: 0,
                },
                &mut fx,
            )
            .unwrap();
            let (entity, owner, _) = link.into_parts();
            if !matches!(
                owner.branch(),
                FreshLevel1Type9InitialProductionBranch::AttractAttention(_)
            ) {
                continue;
            }
            assert!(entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .is_some());
            assert!(entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Tertiary)
                .is_some());
            assert_eq!(entity.ordinary_type9_selected_component_runtime.unwrap().kind(), crate::ordinary_type9_live::OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished);
            assert!(entity.ordinary_type9_pending_initial_selection.is_none());
            found = true;
            break;
        }
        assert!(
            found,
            "source weighted selector must retain its player-nearby branch"
        );
    }
}
