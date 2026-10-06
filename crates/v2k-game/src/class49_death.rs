//! Native 10C10 -> class1/BAC0 or class49/BD20. Both run BAF0's static/dynamic
//! radial before A860; only BD20 then constructs a ring. Historical class49
//! custody names cover this shared prefix with an explicit terminal policy.

use crate::{
    actor_task_owner::ActorTaskSlot,
    damage::DamagePacket,
    entity::{Entity, EntityManager},
    entity_behavior::{audited_behavior_program, BehaviorContextRuntime},
    entity_collision_state::{
        RetailRuntimeValue, DEFERRED_DESTROY_PENDING_STATE_BIT, DEFERRED_DESTROY_STATE_WRITE_MASK,
        DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT,
    },
    intro2_gun_turret::Intro2GunTurretProfile,
    main_base_abort::{
        MainBaseAbortActorLease, MainBaseDeferredDestroyCustodyBlock, MainBaseDeferredDestroyEntry,
        MainBaseExternalDeferredDestroyOwner,
    },
    radial_damage::RadialDamageTemplate,
    resource_cache::ResourceCache,
    type60_exploding_ring::{
        HostType60Allocator, Type60Allocator, Type60ConstructionOutcome, Type60ConstructionRequest,
    },
    type60_exploding_ring_production::Type60ExplodingRingProductionOwner,
    world_fx::{ExplodeWithRingBurstRequest, WorldFx},
};
use std::sync::atomic::{AtomicU64, Ordering};

/// Source-backed terminal suffix after the shared BAF0/A860 prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeExplosionPolicy {
    /// BAC0 stages removal immediately after clearing tasks.
    Class1,
    /// BD20 attempts a Type60 ring before staging removal.
    Class49,
}

impl NativeExplosionPolicy {
    pub const fn behavior_class(self) -> u32 {
        match self {
            Self::Class1 => 1,
            Self::Class49 => 49,
        }
    }

    pub const fn style_address(self) -> u32 {
        match self {
            Self::Class1 => 0x004c_7150,
            Self::Class49 => 0x004c_71e0,
        }
    }
}

/// These native bodies share BAF0. Their metadata, current style cleanup,
/// scatter branch and BAC0/BD20 suffix remain independent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeExplosionSourceProfile {
    GunTurret(Intro2GunTurretProfile),
    CleansingVehicle,
    PowerUp61,
    Intro2Type13,
    EntityWeapon(crate::native_entity_weapons::EntityWeaponKind),
}

impl NativeExplosionSourceProfile {
    pub const fn entity_type(self) -> u32 {
        match self {
            Self::GunTurret(profile) => profile.entity_type(),
            Self::CleansingVehicle => 49,
            Self::PowerUp61 => 61,
            Self::Intro2Type13 => 13,
            Self::EntityWeapon(kind) => kind.entity_type() as u32,
        }
    }

    fn authenticates_metadata(
        self,
        metadata: &crate::entity_collision_state::EntityTypeRuntimeMetadata,
    ) -> bool {
        match self {
            Self::GunTurret(profile) => {
                profile.alternate_behavior_class() == self.policy().behavior_class()
                    && crate::intro2_gun_turret::authenticate_metadata(profile, metadata).is_ok()
            }
            Self::CleansingVehicle => {
                crate::cleansing_vehicle::authenticate_metadata(metadata).is_ok()
            }
            Self::PowerUp61 => {
                crate::main_base_type61_abort::exact_level_one_type61_metadata(metadata)
            }
            Self::Intro2Type13 => {
                crate::type13_initial_behavior::authenticate_type13_metadata(metadata).is_ok()
                    && crate::type13_initial_behavior::authenticate_type13_behavior_list(metadata)
                        .is_ok()
                    && metadata.mass_raw == 100
                    && metadata.capability_flags == 8
                    && metadata.model_slots == [291; 4]
            }
            Self::EntityWeapon(kind) => {
                crate::native_entity_weapons::authenticate_weapon_metadata(kind, metadata).is_ok()
            }
        }
    }

    fn null_death_cleanup(self, style: u32) -> bool {
        match self {
            Self::GunTurret(Intro2GunTurretProfile::Type115) => {
                matches!(style, 0x004c_7468 | 0x004c_8230 | 0x004c_74f8)
            }
            Self::GunTurret(_) => matches!(style, 0x004c_8230 | 0x004c_74f8),
            Self::CleansingVehicle => matches!(style, 0x004c_74b0 | 0x004c_85d8 | 0x004c_8620),
            Self::PowerUp61 => style == 0x004c_96c0,
            // DB80's style+28 cleanup is null for both Search variants,
            // Aimless, their completions and the initializer fallback.
            Self::Intro2Type13 => matches!(
                style,
                0x004c_7a50 | 0x004c_7a98 | 0x004c_7ae0 | 0x004c_7930 | 0x004c_7978 | 0x004c_74f8
            ),
            Self::EntityWeapon(kind) => {
                style == 0x004c_74f8
                    || style
                        == match kind {
                            crate::native_entity_weapons::EntityWeaponKind::Rocket => 0x004c_81a0,
                            _ => 0x004c_8350,
                        }
            }
        }
    }

    fn scatter(self) -> (i32, [u8; 2]) {
        match self {
            // BAF0's signed type112..115 branch (40BB92..40BBB3).
            Self::GunTurret(Intro2GunTurretProfile::Type115) => (10, [37, 37]),
            // Empty A/B/N/G with capability40: 40BB77.
            Self::GunTurret(_) | Self::PowerUp61 => (16, [94, 95]),
            // Nonnull A/B takes 40BBB3 before the later type49 branch.
            Self::CleansingVehicle => (10, [37, 37]),
            // Type13 has G: 40BB52 branches to40BBB3 before capability40.
            Self::Intro2Type13 => (10, [37, 37]),
            Self::EntityWeapon(crate::native_entity_weapons::EntityWeaponKind::Rocket) => {
                (10, [37, 37])
            }
            Self::EntityWeapon(_) => (16, [94, 95]),
        }
    }

    pub const fn policy(self) -> NativeExplosionPolicy {
        match self {
            Self::GunTurret(Intro2GunTurretProfile::Type115)
            | Self::Intro2Type13
            | Self::EntityWeapon(crate::native_entity_weapons::EntityWeaponKind::Rocket) => {
                NativeExplosionPolicy::Class1
            }
            _ => NativeExplosionPolicy::Class49,
        }
    }
}

pub(crate) fn source_profile(entity: &Entity) -> Option<NativeExplosionSourceProfile> {
    if crate::native_entity_weapons::entity_authenticates(entity) {
        Some(NativeExplosionSourceProfile::EntityWeapon(
            entity.native_entity_weapon_runtime?.kind,
        ))
    } else if crate::cleansing_vehicle::entity_authenticates(entity) {
        Some(NativeExplosionSourceProfile::CleansingVehicle)
    } else if crate::native_type61::entity_authenticates(entity) {
        Some(NativeExplosionSourceProfile::PowerUp61)
    } else if crate::intro2_gun_turret::intro2_gun_turret_allocation_authenticates(entity) {
        let profile = entity.intro2_gun_turret_runtime?.profile;
        Some(NativeExplosionSourceProfile::GunTurret(profile))
    } else if intro2_type13_explosion_source_authenticates(entity) {
        Some(NativeExplosionSourceProfile::Intro2Type13)
    } else {
        None
    }
}

/// The native Intro2 body retains its D/E/G/K/L constructor storage across
/// BAC0. A generic Type13 spawn has no published G runtime and cannot borrow
/// this terminal profile. The entry caller separately owns its completed graph.
pub(crate) fn intro2_type13_explosion_source_authenticates(entity: &Entity) -> bool {
    crate::intro2_type13_live::authenticate_intro2_type13(entity).is_ok()
        && entity.intro2_type13_common_mover_runtime.is_some()
        && matches!(
            entity.sub_g_06070_runtime,
            RetailRuntimeValue::Known(Some(_))
        )
}

pub(crate) fn allocation_authenticates(manager: &EntityManager, id: u32) -> bool {
    let Some(entity) = manager.iter_all().find(|entity| entity.id == id) else {
        return false;
    };
    match source_profile(entity) {
        Some(NativeExplosionSourceProfile::CleansingVehicle) => {
            crate::cleansing_vehicle::allocation_authenticates(manager, id)
        }
        Some(NativeExplosionSourceProfile::GunTurret(_)) => {
            crate::intro2_gun_turret::intro2_gun_turret_manager_allocation_authenticates(
                manager, id,
            )
        }
        Some(NativeExplosionSourceProfile::PowerUp61) => {
            crate::native_type61::allocation_authenticates(manager, id)
        }
        Some(NativeExplosionSourceProfile::Intro2Type13) => {
            manager.main_base_abort_actor_observation(id).is_some()
        }
        Some(NativeExplosionSourceProfile::EntityWeapon(_)) => manager
            .main_base_abort_actor_observation(id)
            .is_some_and(|observation| {
                observation.lease == entity.native_entity_weapon_runtime.unwrap().allocation
            }),
        None => false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class49DeathBlock {
    Allocation,
    Graph,
    Metadata,
    Runtime(&'static str),
    TerminalReceipt,
    DeferredDestroyCustody(MainBaseDeferredDestroyCustodyBlock),
    DeferredDestroyOwner(MainBaseExternalDeferredDestroyOwner),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Class49TerminalReceipt {
    pub entity_id: u32,
    pub position_raw: [i16; 3],
    pub radial_damage: RadialDamageTemplate,
    profile: NativeExplosionSourceProfile,
    allocation: MainBaseAbortActorLease,
    context: BehaviorContextRuntime,
    continuation_id: u64,
    deferred_destroy_entry: MainBaseDeferredDestroyEntry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalPhase {
    Issued,
    Claimed,
    Finishing,
    Finished,
}

/// Allocation-owned custody, separate from the slots that A860 later clears.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Class49DeathRuntime {
    receipt: Class49TerminalReceipt,
    phase: TerminalPhase,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Class49TerminalCompletion {
    pub ring: Option<Type60ConstructionOutcome>,
    pub ring_owner: Option<Type60ExplodingRingProductionOwner>,
}

static NEXT_TERMINAL_ID: AtomicU64 = AtomicU64::new(1);

/// Retains first-death/radial reentry suppression through Finished and until
/// allocation removal. Particle re-hits use the narrower completed proof below.
pub(crate) fn terminal_is_pending(entity: &Entity) -> bool {
    entity.class49_death_runtime.is_some()
}

/// BD20 finishes before the physical particle walker advances. Its10B70
/// request leaves8000 intact, and43FA3D does not exclude deferred removal,
/// so another particle can reach the same class49 allocation before the sweep.
/// Only the completed receipt can replace the source's live task custody here;
/// an issued, claimed or partially finished continuation remains parked.
pub(crate) fn finished_terminal_hit_authenticates(manager: &EntityManager, id: u32) -> bool {
    let Some(entity) = manager.iter_all().find(|entity| entity.id == id) else {
        return false;
    };
    let Some(runtime) = entity.class49_death_runtime else {
        return false;
    };
    runtime.phase == TerminalPhase::Finished
        && receipt_current(manager, &runtime.receipt)
        && runtime.receipt.context.active_style().style_address()
            == runtime.receipt.profile.policy().style_address()
        && ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| entity.actor_tasks.task_in_slot(slot).is_none())
        && entity
            .collision
            .state_flags_at_0x08
            .masked(DYING_STATE_BIT | DEFERRED_DESTROY_STATE_WRITE_MASK)
            == RetailRuntimeValue::Known(DYING_STATE_BIT | DEFERRED_DESTROY_PENDING_STATE_BIT)
}

fn state_bits(entity: &Entity, mask: u32) -> Result<u32, Class49DeathBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(value) => Ok(value),
        _ => Err(Class49DeathBlock::Runtime("state bits")),
    }
}

/// 416F90 resolves +60, falling back to the source for a null/dangling handle.
fn logical_owner(manager: &EntityManager, entity: &Entity) -> Result<(u32, u8), Class49DeathBlock> {
    let owner = match entity.collision.recent_relation_id_at_0x60 {
        RetailRuntimeValue::Unresolved => {
            return Err(Class49DeathBlock::Runtime("logical owner relation"));
        }
        RetailRuntimeValue::Known(None) => entity,
        RetailRuntimeValue::Known(Some(id)) => {
            let mut candidates = manager.iter_all().filter(|candidate| candidate.id == id);
            match (candidates.next(), candidates.next()) {
                (Some(owner), None) => owner,
                (None, None) => entity,
                _ => return Err(Class49DeathBlock::Runtime("ambiguous logical owner")),
            }
        }
    };
    Ok((owner.id, owner.entity_type as u8))
}

/// Authenticate the native source before committing health/dying/context.
/// Class1/49 is the direct alternate: there is no weighted-selector RNG word.
/// The returned receipt's caller must claim it, execute static then dynamic
/// radial damage synchronously, and finish only after that scan completes.
pub fn begin_class49_standard_death(
    manager: &mut EntityManager,
    id: u32,
    resources: &ResourceCache,
    fx: &mut WorldFx,
    _retail_tick: u32,
) -> Result<Option<Class49TerminalReceipt>, Class49DeathBlock> {
    use Class49DeathBlock as Block;
    let entity = manager
        .iter_all()
        .find(|e| e.id == id)
        .ok_or(Block::Allocation)?;
    if !allocation_authenticates(manager, id) {
        return Err(Block::Allocation);
    }
    if terminal_is_pending(entity)
        || state_bits(entity, REMOTE_OWNED_STATE_BIT | DYING_STATE_BIT)? != 0
    {
        return Ok(None);
    }
    let profile = source_profile(entity).ok_or(Block::Metadata)?;
    let deferred_destroy_entry = manager
        .preflight_main_base_deferred_destroy_entry(id, entity.collision.state_flags_at_0x08)
        .map_err(Block::DeferredDestroyCustody)?;
    if let MainBaseDeferredDestroyEntry::AlreadyPending(owner) = deferred_destroy_entry {
        if profile != NativeExplosionSourceProfile::PowerUp61
            || owner != MainBaseExternalDeferredDestroyOwner::Type61DeferredQueue
        {
            return Err(Block::DeferredDestroyOwner(owner));
        }
    }
    let metadata = manager
        .type_runtime_metadata(profile.entity_type())
        .ok_or(Block::Metadata)?;
    if !profile.authenticates_metadata(metadata) {
        return Err(Block::Metadata);
    }
    if entity.collision.constructor_sound_attachment_id_at_0x8c
        != metadata.constructor_sound_attachment_id
    {
        return Err(Block::Runtime("constructor sound attachment"));
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Graph);
    };
    if !profile.null_death_cleanup(context.active_style().style_address()) {
        return Err(Block::Runtime("unaudited death style"));
    }
    if profile == NativeExplosionSourceProfile::Intro2Type13 {
        crate::intro2_type13_live::Intro2Type13SchedulerOwner::adopt_published(entity)
            .map_err(|_| Block::Graph)?;
        if ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .filter_map(|slot| entity.actor_tasks.task_in_slot(slot))
            .any(|task| {
                entity
                    .actor_tasks
                    .wrapper_flags(task)
                    .is_none_or(|flags| !flags.alive || flags.in_callback)
            })
        {
            return Err(Block::Graph);
        }
        let record = resources.global_entity_type(13).ok_or(Block::Metadata)?;
        if *metadata
            != crate::entity_collision_state::EntityTypeRuntimeMetadata::from_section12(record)
        {
            return Err(Block::Metadata);
        }
    }
    let program =
        audited_behavior_program(profile.policy().behavior_class()).ok_or(Block::Metadata)?;
    let selected = context
        .reselect_audited_type_default(program, 0, program.initial_style)
        .ok_or(Block::Graph)?;
    let flags = state_bits(entity, 0x8000_6000)?;
    let slot =
        crate::entity_collision_state::active_model_slot_from_state_flags(flags | DYING_STATE_BIT);
    let extent = entity.model_slots[slot]
        .and_then(|model| resources.global_model(model))
        .ok_or(Block::Metadata)?
        .radius;
    let (owner_id, owner_type) = logical_owner(manager, entity)?;
    let type_record = resources
        .global_entity_type(profile.entity_type() as usize)
        .ok_or(Block::Metadata)?;
    let death_sound = type_record.death_sound_id();
    let header = &type_record.raw_header;
    let word = |at: usize| i16::from_le_bytes(header[at..at + 2].try_into().unwrap());
    let dword = |at: usize| i32::from_le_bytes(header[at..at + 4].try_into().unwrap());
    let receipt = Class49TerminalReceipt {
        entity_id: id,
        profile,
        position_raw: entity.position_raw(),
        allocation: manager
            .main_base_abort_actor_observation(id)
            .ok_or(Block::Allocation)?
            .lease,
        context: selected,
        continuation_id: NEXT_TERMINAL_ID.fetch_add(1, Ordering::Relaxed),
        deferred_destroy_entry,
        radial_damage: RadialDamageTemplate {
            inner_radius_raw: word(0x50),
            outer_radius_raw: word(0x52),
            impulse_raw: dword(0x54),
            packet: DamagePacket {
                channels: [dword(0x58), dword(0x5c)],
                amounts_raw: [dword(0x60), dword(0x64)],
            },
            trailing_raw: [i32::from(owner_type), owner_id as i32],
        },
    };
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.health_raw = RetailRuntimeValue::Known(0);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
    // 10C10 requests header+90 at fixed gain/rate after setting dying and
    // before DB80 enters BD20. This is distinct from 440950's later sound62
    // and consumes no process RNG. Submit now, before BAF0's immediate cues.
    if let Some(sound) = death_sound {
        fx.emit_fixed_positional_sound_raw(sound, receipt.position_raw);
    }
    // 10C10 then stops +8C before DB80 enters the selected alternate. In
    // particular the native Type61 class23 source owns looping sound44.
    entity.collision.constructor_sound_attachment_id_at_0x8c = RetailRuntimeValue::Known(None);
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(selected));
    entity.class49_death_runtime = Some(Class49DeathRuntime {
        receipt,
        phase: TerminalPhase::Issued,
    });
    let (scatter_count, scatter_classes) = profile.scatter();
    // 440950's solo network step is a no-op; its independent surface/sound
    // suffix follows the profile's exact BAF0 scatter branch.
    fx.emit_explode_with_ring_burst_raw(ExplodeWithRingBurstRequest {
        position_raw: receipt.position_raw,
        source_extent_raw: extent,
        sea_level_raw: resources
            .level_terrain()
            .map(|terrain| terrain.sea_level_raw()),
        logical_owner_entity_id: owner_id,
        logical_owner_entity_type: owner_type,
        scatter_count,
        scatter_classes,
        suppresses_impact_damage: flags & 0x8000_0000 != 0,
    });
    Ok(Some(receipt))
}

fn receipt_current(manager: &EntityManager, receipt: &Class49TerminalReceipt) -> bool {
    manager
        .main_base_abort_actor_observation(receipt.entity_id)
        .is_some_and(|observation| observation.lease == receipt.allocation)
        && manager
            .iter_all()
            .find(|e| e.id == receipt.entity_id)
            .is_some_and(|entity| {
                source_profile(entity) == Some(receipt.profile)
                    && entity.current_behavior_context
                        == RetailRuntimeValue::Known(Some(receipt.context))
                    && entity
                        .class49_death_runtime
                        .is_some_and(|runtime| runtime.receipt == *receipt)
            })
}

pub fn claim_class49_terminal(
    manager: &mut EntityManager,
    receipt: &Class49TerminalReceipt,
) -> bool {
    if !receipt_current(manager, receipt) {
        return false;
    }
    let entity = manager.entity_mut(receipt.entity_id).unwrap();
    if entity.position_raw() != receipt.position_raw {
        return false;
    }
    let runtime = entity.class49_death_runtime.as_mut().unwrap();
    if runtime.phase != TerminalPhase::Issued {
        return false;
    }
    runtime.phase = TerminalPhase::Claimed;
    true
}

pub(crate) fn active_terminal_receipt(
    manager: &EntityManager,
    receipt: &Class49TerminalReceipt,
) -> bool {
    receipt_current(manager, receipt)
        && manager
            .iter_all()
            .find(|e| e.id == receipt.entity_id)
            .is_some_and(|e| {
                e.class49_death_runtime
                    .is_some_and(|runtime| runtime.phase == TerminalPhase::Claimed)
            })
}

pub fn finish_class49_terminal(
    manager: &mut EntityManager,
    receipt: Class49TerminalReceipt,
    resources: &ResourceCache,
    fx: &mut WorldFx,
) -> Result<Class49TerminalCompletion, Class49DeathBlock> {
    finish_with_allocator(manager, receipt, resources, fx, &mut HostType60Allocator)
}

fn finish_with_allocator(
    manager: &mut EntityManager,
    receipt: Class49TerminalReceipt,
    resources: &ResourceCache,
    fx: &mut WorldFx,
    allocator: &mut impl Type60Allocator,
) -> Result<Class49TerminalCompletion, Class49DeathBlock> {
    use Class49DeathBlock as Block;
    if !receipt_current(manager, &receipt) {
        return Err(Block::TerminalReceipt);
    }
    let entity = manager.entity_mut(receipt.entity_id).unwrap();
    if entity.class49_death_runtime.unwrap().phase != TerminalPhase::Claimed {
        return Err(Block::TerminalReceipt);
    }
    entity.class49_death_runtime.as_mut().unwrap().phase = TerminalPhase::Finishing;
    // BAC0 and BD20 clear all slots only after BAF0's radial pass. Only
    // BD20 rereads cached source +08/+96 and attempts the ring allocation.
    for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
        entity.actor_tasks.clear_slot(slot);
    }
    let position = entity.position_raw();
    let ring = if receipt.profile.policy() == NativeExplosionPolicy::Class1
        || state_bits(entity, REMOTE_OWNED_STATE_BIT)? != 0
    {
        None
    } else {
        let terrain = resources
            .level_terrain()
            .ok_or(Block::Runtime("ring terrain"))?;
        Some(manager.construct_type60_exploding_ring_with_allocator(
            Type60ConstructionRequest::class49_at(position),
            terrain,
            fx,
            allocator,
        ))
    };
    let ring_owner = ring
        .and_then(Type60ConstructionOutcome::primary_task_lease)
        .map(Type60ExplodingRingProductionOwner::adopt);
    let entity = manager.entity_mut(receipt.entity_id).unwrap();
    let state = entity.collision.state_flags_at_0x08;
    if manager
        .preflight_main_base_deferred_destroy_entry(receipt.entity_id, state)
        .map_err(Block::DeferredDestroyCustody)?
        != receipt.deferred_destroy_entry
    {
        return Err(Block::Runtime("deferred destroy owner changed"));
    }
    let entity = manager.entity_mut(receipt.entity_id).unwrap();
    entity.class49_death_runtime.as_mut().unwrap().phase = TerminalPhase::Finished;
    // 10B70 is idempotent across the retail shared pending bit/count. A
    // same-frame Type61 pickup keeps its earlier deferred-splice owner.
    manager.commit_main_base_deferred_destroy(receipt.entity_id, receipt.deferred_destroy_entry);
    Ok(Class49TerminalCompletion { ring, ring_owner })
}

#[cfg(test)]
mod tests;
