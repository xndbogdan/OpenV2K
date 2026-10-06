//! Fail-closed plan for the Level-1 Alien Hive's lethal continuation.
//!
//! Retail does not turn the vulnerable Hive into an exit with a model edit.
//! The ordinary death callback enters `FUN_00425790`, clears task slots 2
//! and 1 through `FUN_0040A7A0`, and calls `FUN_00425F60`. The latter
//! `FUN_00401020`-allocates a slot-0 wrapper whose tick is `FUN_004260F0`,
//! publishes it with `FUN_0040A7A0`, then emits `FUN_00440950`'s shared
//! scatter/surface burst and `FUN_004566E0`'s world radial. Live initializer
//! `FUN_00425760` is the same clear-2/clear-1 pair plus `FUN_00425E10`, whose
//! template differs only in tick `FUN_00425EA0`. Recovered `FUN_004260F0`
//! calls shared `FUN_0041BEB0` then, on detailed ticks, ramps the Sub-K bound
//! u16 (`elapsed_us >> 8`, cap `0xD000`). Live `FUN_00425EA0` writes that same
//! word from a sine of Sub-K `+0x08` (`elapsed_us >> 5`). Type 67 authors
//! Sub-K `[1, 0]`, so `FUN_00424450` binds `FUN_0040a950(entity, 1)` at
//! Sub-K `+0` and those ticks publish the u16 into `AnimVars.dynamic[1]`.
//!
//! The projectile owner applies the entity health/model/style writes, keeps
//! the shared `FUN_0041BEB0` program, reallocates slot 0 onto the dying tick,
//! sets controller state 0 (class-5 spit stops), and switches the Sub-K writer
//! to the dying ramp. Marker-backed Sub-N `+0x4C` arms the `-6_000_000`
//! suction delay. Normal campaign routing consumes actual static-model contact;
//! `1BEB0`'s direct `456D10` interior request is gated by Main Base abort mode.
//! Live fall-in yank `FUN_0041c830` / pair-restore
//! `FUN_0041ca90` stay fail-closed. Sub-N / Sub-K / infection / radial state
//! stay on the component table; this module owns only the 8-byte wrapper
//! identity.

use crate::{
    damage::{
        generic_entity_damage_transition, DamagePacket, GenericEntityDamageStage,
        GenericEntityDamageState, GenericEntityDamageTransition,
    },
    entity_behavior::{audited_behavior_style, BehaviorStyle},
    hive_controller::HIVE_ENTITY_TYPE,
    radial_damage::RadialDamageTemplate,
    world_fx::{
        METEOR_IMPACT_SCATTER_COUNT, METEOR_IMPACT_SOUND_ID, METEOR_SCATTER_CLASS,
        METEOR_SURFACE_CLASS,
    },
};

pub mod component;

/// Retail task slots cleared by `FUN_00425760` / `FUN_00425790`, in call order.
pub const HIVE_DEATH_CLEARED_COMPONENT_SLOTS: [u8; 2] = [2, 1];
/// Slot replaced by `FUN_00425E10` / `FUN_00425F60` after its allocator succeeds.
pub const HIVE_DEATH_COMPONENT_SLOT: u8 = 0;
/// Live `FUN_00425E10` tick copied into the 11-dword `FUN_00401020` template.
pub const HIVE_LIVE_COMPONENT_TICK_ADDRESS: u32 = 0x0042_5EA0;
/// Dying `FUN_00425F60` tick copied into the 11-dword `FUN_00401020` template.
pub const HIVE_DEATH_COMPONENT_TICK_ADDRESS: u32 = 0x0042_60F0;
/// Live slot-0 installer called after the two `FUN_0040A7A0` clears.
pub const HIVE_LIVE_SLOT0_INSTALLER_ADDRESS: u32 = 0x0042_5E10;
/// Dying slot-0 installer called after the two `FUN_0040A7A0` clears.
pub const HIVE_DEATH_SLOT0_INSTALLER_ADDRESS: u32 = 0x0042_5F60;

/// Slot-0 wrapper identity for live `FUN_00425E10` / dying `FUN_00425F60`.
///
/// `FUN_00401020` copies an 11-dword template with a null constructor, the
/// entity's component-table pointer at inner `+0x08`, and one of the tick
/// addresses above. Sub-N, Sub-K, infection, and radial accumulators live on
/// that table, not in this wrapper, so a death realloc keeps them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HiveRadialTaskState {
    tick_callback_address: u32,
}

impl HiveRadialTaskState {
    pub const fn live() -> Self {
        Self {
            tick_callback_address: HIVE_LIVE_COMPONENT_TICK_ADDRESS,
        }
    }

    pub const fn dying() -> Self {
        Self {
            tick_callback_address: HIVE_DEATH_COMPONENT_TICK_ADDRESS,
        }
    }

    pub const fn tick_callback_address(self) -> u32 {
        self.tick_callback_address
    }

    pub const fn is_dying(self) -> bool {
        self.tick_callback_address == HIVE_DEATH_COMPONENT_TICK_ADDRESS
    }
}
/// Exact fixed portion of radial template `0x004C9748`.
///
/// The trailing provenance words are written immediately before dispatch and
/// therefore remain request-specific.
pub fn hive_death_radial_template(hive_entity_id: u32) -> RadialDamageTemplate {
    RadialDamageTemplate {
        trailing_raw: [HIVE_ENTITY_TYPE as i32, hive_entity_id as i32],
        ..HIVE_DEATH_RADIAL_TEMPLATE_BASE
    }
}

/// `FUN_00440950` scatter/surface burst after `FUN_00425790`. Nested
/// `FUN_004566E0` world radial stays with the projectile/abort host.
pub fn emit_hive_dying_surface_burst(
    world_fx: &mut crate::world_fx::WorldFx,
    position_raw: [i16; 3],
    source_extent_raw: u16,
    sea_level_raw: Option<i16>,
    hive_entity_id: u32,
) {
    world_fx.emit_scatter_surface_burst_raw(
        position_raw,
        source_extent_raw,
        sea_level_raw,
        hive_entity_id,
    );
}

pub fn emit_hive_dying_surface_burst_for_entity(
    manager: &crate::entity::EntityManager,
    world_fx: &mut crate::world_fx::WorldFx,
    hive_entity_id: u32,
    source_extent_raw: u16,
    sea_level_raw: Option<i16>,
) {
    let Some(entity) = manager
        .iter_all()
        .find(|entity| entity.id == hive_entity_id)
    else {
        return;
    };
    emit_hive_dying_surface_burst(
        world_fx,
        entity.position_raw(),
        source_extent_raw,
        sea_level_raw,
        hive_entity_id,
    );
}

pub const HIVE_DEATH_RADIAL_TEMPLATE_BASE: RadialDamageTemplate = RadialDamageTemplate {
    inner_radius_raw: 0x0200,
    outer_radius_raw: 0x0400,
    impulse_raw: 200,
    packet: DamagePacket {
        channels: [1, 4],
        amounts_raw: [10_000, 8_000],
    },
    trailing_raw: [0, 0],
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HiveDeathComponentProgram {
    pub cleared_slots: [u8; 2],
    pub replacement_slot: u8,
    pub tick_callback_address: u32,
}

pub const HIVE_DEATH_COMPONENT_PROGRAM: HiveDeathComponentProgram = HiveDeathComponentProgram {
    cleared_slots: HIVE_DEATH_CLEARED_COMPONENT_SLOTS,
    replacement_slot: HIVE_DEATH_COMPONENT_SLOT,
    tick_callback_address: HIVE_DEATH_COMPONENT_TICK_ADDRESS,
};

/// Fixed presentation contract of the shared `FUN_00440950` helper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HiveDeathSurfaceBurstProgram {
    pub scatter_class: u8,
    pub scatter_attempts: usize,
    pub above_sea_surface_class: u8,
    pub source_extent_raw: u16,
    pub randomized_sound_id: usize,
}

pub const fn hive_death_surface_burst_program(
    source_extent_raw: u16,
) -> HiveDeathSurfaceBurstProgram {
    HiveDeathSurfaceBurstProgram {
        scatter_class: METEOR_SCATTER_CLASS,
        scatter_attempts: METEOR_IMPACT_SCATTER_COUNT,
        above_sea_surface_class: METEOR_SURFACE_CLASS,
        source_extent_raw,
        randomized_sound_id: METEOR_IMPACT_SOUND_ID,
    }
}

/// Snapshot admitted by the checked-damage and model-resource adapter.
///
/// The caller supplies the generic arithmetic input and the admitted dying
/// model's extent so this pure planner cannot silently turn a non-Hive, wrong
/// model slot, forged behavior style, survivor hit, or already-dying hit into
/// the continuation. Allocation identity remains an external live-transaction
/// responsibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelOneHiveLethalRequest {
    pub entity_id: u32,
    pub entity_type: u32,
    pub model_slots: [Option<usize>; 4],
    pub active_model_slot: usize,
    pub current_style: BehaviorStyle,
    pub position_raw: [i16; 3],
    /// Slot-3/model-343 resource word `+0x08`, admitted after the generic
    /// damage path sets state bit `0x4000` and selects the dying model.
    pub dying_model_extent_raw: u16,
    pub generic_damage_state: GenericEntityDamageState,
    pub accepted_damage_raw: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelOneHiveEntityDeathTransition {
    pub health_after_raw: i32,
    pub pre_health_buffer_after_raw: i32,
    pub active_model_slot_after: usize,
    pub model_after: usize,
    pub style_after: BehaviorStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelOneHiveDeathPlan {
    pub entity_id: u32,
    pub position_raw: [i16; 3],
    pub generic_damage: GenericEntityDamageTransition,
    pub entity_transition: LevelOneHiveEntityDeathTransition,
    pub component: HiveDeathComponentProgram,
    pub surface_burst: HiveDeathSurfaceBurstProgram,
    pub radial: RadialDamageTemplate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelOneHiveDeathPlanError {
    WrongEntityType,
    InvalidModelSlots,
    WrongActiveModelSlot,
    UnauthenticatedLiveStyle,
    ZeroAcceptedDamage,
    AlreadyDying,
    NonLethal { health_after_raw: i32 },
}

/// Authored live slots 0/2 and dying slots 1/3. Slot 3 is the `0x4000|0x2000`
/// wreck model `FUN_00425790` selects. Level-1 uses 341/343; later worlds may
/// override the ids without changing the slot policy.
pub fn hive_dying_model(model_slots: [Option<usize>; 4]) -> Option<usize> {
    let live = model_slots[0]?;
    let dying = model_slots[1]?;
    if model_slots[2] != Some(live) || model_slots[3] != Some(dying) {
        return None;
    }
    Some(dying)
}

/// Class-46 live style on slot 2 with a complete live/dying slot pair.
pub fn hive_dying_initializer_ready(
    entity_type: u32,
    model_slots: [Option<usize>; 4],
    active_model_slot: usize,
    current_style: BehaviorStyle,
) -> bool {
    entity_type == HIVE_ENTITY_TYPE
        && hive_dying_model(model_slots).is_some()
        && matches!(active_model_slot, 2 | 3)
        && current_style
            == *audited_behavior_style(46, 0)
                .expect("Alien Hive initial style is in the static catalog")
}

/// Recover the complete static lethal plan without mutating world state.
pub fn plan_level_one_hive_lethal_continuation(
    request: LevelOneHiveLethalRequest,
) -> Result<LevelOneHiveDeathPlan, LevelOneHiveDeathPlanError> {
    if request.entity_type != HIVE_ENTITY_TYPE {
        return Err(LevelOneHiveDeathPlanError::WrongEntityType);
    }
    let Some(dying_model) = hive_dying_model(request.model_slots) else {
        return Err(LevelOneHiveDeathPlanError::InvalidModelSlots);
    };
    if request.active_model_slot != 2 {
        return Err(LevelOneHiveDeathPlanError::WrongActiveModelSlot);
    }
    let live_style =
        audited_behavior_style(46, 0).expect("Alien Hive initial style is in the static catalog");
    if request.current_style != *live_style {
        return Err(LevelOneHiveDeathPlanError::UnauthenticatedLiveStyle);
    }
    if request.accepted_damage_raw == 0 {
        return Err(LevelOneHiveDeathPlanError::ZeroAcceptedDamage);
    }

    let generic_damage =
        generic_entity_damage_transition(request.generic_damage_state, request.accepted_damage_raw);
    match generic_damage.stage {
        GenericEntityDamageStage::AlreadyDying => {
            return Err(LevelOneHiveDeathPlanError::AlreadyDying);
        }
        GenericEntityDamageStage::Survived => {
            return Err(LevelOneHiveDeathPlanError::NonLethal {
                health_after_raw: generic_damage.health_after_subtraction_raw,
            });
        }
        GenericEntityDamageStage::DeathDispatchRequired => {}
    }

    let dying_style = *audited_behavior_style(46, 1)
        .expect("captured Alien Hive dying style is in the static catalog");
    let radial = RadialDamageTemplate {
        trailing_raw: [HIVE_ENTITY_TYPE as i32, request.entity_id as i32],
        ..HIVE_DEATH_RADIAL_TEMPLATE_BASE
    };
    let surface_burst = hive_death_surface_burst_program(request.dying_model_extent_raw);

    Ok(LevelOneHiveDeathPlan {
        entity_id: request.entity_id,
        position_raw: request.position_raw,
        generic_damage,
        entity_transition: LevelOneHiveEntityDeathTransition {
            health_after_raw: 0,
            pre_health_buffer_after_raw: generic_damage.pre_health_buffer_after_raw,
            active_model_slot_after: 3,
            model_after: dying_model,
            style_after: dying_style,
        },
        component: HIVE_DEATH_COMPONENT_PROGRAM,
        surface_burst,
        radial,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> LevelOneHiveLethalRequest {
        LevelOneHiveLethalRequest {
            entity_id: 0x04A3_0001,
            entity_type: HIVE_ENTITY_TYPE,
            model_slots: [Some(341), Some(343), Some(341), Some(343)],
            active_model_slot: 2,
            current_style: *audited_behavior_style(46, 0).unwrap(),
            position_raw: [0x1200, 0x0200, -0x3400],
            dying_model_extent_raw: 0x34,
            generic_damage_state: GenericEntityDamageState {
                health_raw: 200,
                pre_health_buffer_raw: 0,
                already_dying: false,
            },
            accepted_damage_raw: 1_800,
        }
    }

    #[test]
    fn captured_lethal_hive_plan_pins_every_static_phase() {
        let plan = plan_level_one_hive_lethal_continuation(request()).unwrap();

        assert_eq!(
            plan.generic_damage.stage,
            GenericEntityDamageStage::DeathDispatchRequired
        );
        assert_eq!(plan.entity_transition.health_after_raw, 0);
        assert_eq!(plan.entity_transition.active_model_slot_after, 3);
        assert_eq!(plan.entity_transition.model_after, 343);
        assert_eq!(
            plan.entity_transition.style_after.frame_address,
            0x004C_9510
        );
        assert_eq!(plan.component.cleared_slots, [2, 1]);
        assert_eq!(plan.component.replacement_slot, 0);
        assert_eq!(plan.component.tick_callback_address, 0x0042_60F0);
        assert_eq!(plan.surface_burst.source_extent_raw, 0x34);
        assert_eq!(plan.radial.inner_radius_raw, 0x0200);
        assert_eq!(plan.radial.outer_radius_raw, 0x0400);
        assert_eq!(plan.radial.impulse_raw, 200);
        assert_eq!(plan.radial.packet.channels, [1, 4]);
        assert_eq!(plan.radial.packet.amounts_raw, [10_000, 8_000]);
        assert_eq!(
            plan.radial.trailing_raw,
            [HIVE_ENTITY_TYPE as i32, 0x04A3_0001]
        );
        assert_eq!(
            hive_death_radial_template(0x04A3_0001).trailing_raw,
            plan.radial.trailing_raw
        );
    }

    #[test]
    fn forged_type_style_slot_and_nonlethal_hits_fail_closed() {
        let mut wrong_type = request();
        wrong_type.entity_type = 66;
        assert_eq!(
            plan_level_one_hive_lethal_continuation(wrong_type),
            Err(LevelOneHiveDeathPlanError::WrongEntityType)
        );

        let mut wrong_slot = request();
        wrong_slot.active_model_slot = 0;
        assert_eq!(
            plan_level_one_hive_lethal_continuation(wrong_slot),
            Err(LevelOneHiveDeathPlanError::WrongActiveModelSlot)
        );

        let mut wrong_style = request();
        wrong_style.current_style = *audited_behavior_style(46, 1).unwrap();
        assert_eq!(
            plan_level_one_hive_lethal_continuation(wrong_style),
            Err(LevelOneHiveDeathPlanError::UnauthenticatedLiveStyle)
        );

        let mut survivor = request();
        survivor.generic_damage_state.health_raw = 2_000;
        assert_eq!(
            plan_level_one_hive_lethal_continuation(survivor),
            Err(LevelOneHiveDeathPlanError::NonLethal {
                health_after_raw: 200
            })
        );
    }

    #[test]
    fn live_and_dying_slot0_ticks_are_the_fun_00401020_template_words() {
        assert_eq!(
            HiveRadialTaskState::live().tick_callback_address(),
            HIVE_LIVE_COMPONENT_TICK_ADDRESS
        );
        assert_eq!(
            HiveRadialTaskState::dying().tick_callback_address(),
            HIVE_DEATH_COMPONENT_TICK_ADDRESS
        );
        assert!(!HiveRadialTaskState::live().is_dying());
        assert!(HiveRadialTaskState::dying().is_dying());
        assert_eq!(HIVE_LIVE_SLOT0_INSTALLER_ADDRESS, 0x0042_5E10);
        assert_eq!(HIVE_DEATH_SLOT0_INSTALLER_ADDRESS, 0x0042_5F60);
        assert_eq!(
            plan_level_one_hive_lethal_continuation(request())
                .unwrap()
                .component
                .tick_callback_address,
            HIVE_DEATH_COMPONENT_TICK_ADDRESS
        );
    }

    #[test]
    fn later_world_slot_ids_still_select_slot_three() {
        let mut later = request();
        later.model_slots = [Some(400), Some(401), Some(400), Some(401)];
        let plan = plan_level_one_hive_lethal_continuation(later).unwrap();
        assert_eq!(plan.entity_transition.active_model_slot_after, 3);
        assert_eq!(plan.entity_transition.model_after, 401);
    }
}
