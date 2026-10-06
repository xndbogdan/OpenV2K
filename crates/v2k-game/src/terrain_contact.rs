//! Retail player contact with the bare Section-10 terrain heightfield.
//!
//! `FUN_00412870` submits the live, oriented player model's authored collision
//! spheres to the terrain callback table. The common type-vtable at
//! `0x004C8A30` binds bare-terrain slot `+0x0C` to `FUN_0040D7F0`. That
//! trampoline invokes optional behavior-style callback `+0x10`, re-resolves a
//! surviving entity, and then unconditionally runs
//! `FUN_004141D0(..., 0x7fff_ffff, 2, 1)`: separate the hull, remove its inward
//! normal velocity, and submit channel-1 impact damage. Type 46's Player
//! Control style callback `FUN_00448280` adds feedback and a material-specific
//! damage request; it provides no clamp or impact immunity. This module owns
//! the common physical/damage tail. The water plane is deliberately absent:
//! retail lets the craft cross it, while the terrain below remains solid.

use crate::damage::{DamageDeliveryRecord, DamagePacket};
#[cfg(test)]
use crate::entity_behavior::behavior_program;
use crate::entity_behavior::BehaviorContextRuntime;
use crate::entity_collision_state::RetailRuntimeValue;
use crate::player::VehicleMode;
use crate::player_contact_style::{
    apply_player_dying_surface_contact, player_surface_style, PlayerContactStyleBlock,
    PlayerDyingSurfaceContactFrame, PlayerDyingSurfaceContactOutcome, PlayerSurfaceStyle,
};
use crate::player_hull::{HullDamageOutcome, PlayerHull};
use crate::whole_body_surface::nearest_cell_material_code;
use v2k_formats::models::{AnimVars, ModelCollisionError, ModelEntry};
use v2k_formats::terrain::TerrainGrid;

#[cfg(test)]
const PLAYER_CONTROL_BEHAVIOR_CLASS_ID: u32 = 24;
/// Hover Player Control style installed at entity behavior-context `+0x14`.
pub const PLAYER_CONTROL_HOVER_STYLE_ADDRESS: u32 = 0x004C_D940;
/// VTOL Player Control style installed when the craft changes mode.
pub const PLAYER_CONTROL_VTOL_STYLE_ADDRESS: u32 = 0x004C_D988;
/// Shared bare-terrain callback at style slot `+0x10` in both exact styles.
pub const PLAYER_CONTROL_TERRAIN_CALLBACK_ADDRESS: u32 = 0x0044_8280;
/// Retail's default settings dword at `FUN_0043C7C0() + 0x40`.
///
/// The port has no force-feedback backend or setting yet. Retaining the
/// authored scale in the request still exposes the exact strength that would
/// have been submitted instead of silently discarding the callback branch.
pub const DEFAULT_PLAYER_TERRAIN_HAPTIC_SCALE_RAW: i32 = 15;
const PLAYER_TERRAIN_HAPTIC_INWARD_THRESHOLD_RAW: i32 = 0x200;
const PLAYER_TERRAIN_HAPTIC_STRENGTH_MAX_RAW: i32 = 0xff;
const PLAYER_TERRAIN_DAMAGE_RESPONSE_SELECTOR: u8 = 7;

/// Exact delivery record built by Player Control's bare-terrain callback when
/// the nearest terrain material maps to response selector 7.
pub const PLAYER_CONTROL_TERRAIN_DAMAGE_DELIVERY: DamageDeliveryRecord = DamageDeliveryRecord {
    packet: DamagePacket {
        channels: [3, 0],
        amounts_raw: [15_000, 0],
    },
    source_entity_type_raw: (-3_i32) as u32,
    owner_handle: 0,
};

/// Deepest authored player-model contact against the triangulated heightfield.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainModelContact {
    pub normal_q12: [i16; 3],
    pub penetration_raw: i32,
}

/// Inputs owned by the optional behavior-style `+0x10` callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerTerrainStyleCallbackRequest {
    /// Current per-instance descriptor authority at terrain-dispatch entry.
    ///
    /// The port's behavior context retains the initially published Hover style,
    /// while retail changes the installed style pointer with the craft mode.
    /// [`vehicle_mode`](Self::vehicle_mode) is therefore the current style
    /// authority and this context authenticates the class-24 descriptor only.
    pub behavior_context: RetailRuntimeValue<Option<BehaviorContextRuntime>>,
    /// Current PlayerCraft mode, mapping to retail's exact Hover or VTOL style.
    pub vehicle_mode: VehicleMode,
    /// Handle passed to the style callback.
    pub entity_handle: u32,
    /// Entity currently bound at the local controller's retail `+0x68`.
    /// `None` represents a missing controller allocation.
    pub controlled_entity_handle: Option<u32>,
    /// Runtime table built by [`v2k_formats::levels::LevelDescriptor::ground_response_selectors`].
    pub ground_response_selectors: [u8; 8],
    /// Settings dword multiplied into the unsupported force-feedback request.
    pub haptic_scale_raw: i32,
}

pub enum PlayerTerrainStyleCallback<'a> {
    PlayerControl(PlayerTerrainStyleCallbackRequest),
    DyingBounce(PlayerDyingSurfaceContactFrame<'a>),
}

impl From<PlayerTerrainStyleCallbackRequest> for PlayerTerrainStyleCallback<'_> {
    fn from(request: PlayerTerrainStyleCallbackRequest) -> Self {
        Self::PlayerControl(request)
    }
}

/// Force-feedback command which retail submits for a sufficiently fast inward
/// terrain strike. The port deliberately reports it without pretending that
/// its current input backend can play the effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerTerrainUnsupportedHaptic {
    pub inward_speed_raw: i32,
    pub haptic_scale_raw: i32,
    pub strength_raw: i32,
}

/// Exact Player Control style work completed before the common terrain tail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerControlTerrainStyleOutcome {
    /// Current craft mode which selected the exact retail style below.
    pub vehicle_mode: VehicleMode,
    pub style_address: u32,
    /// Q12 normal dot signed-8.8 velocity before the common response.
    pub normal_velocity_raw: i32,
    pub unsupported_haptic: Option<PlayerTerrainUnsupportedHaptic>,
    /// Low-three-bit terrain material from the nearest wrapped X-major cell.
    pub material_code: u8,
    pub response_selector: u8,
    pub damage_delivery: Option<DamageDeliveryRecord>,
    pub damage_raw: i32,
    pub hull_damage: HullDamageOutcome,
}

/// Optional style-dispatch result preceding every common player response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerTerrainStyleCallbackOutcome {
    /// A known-null behavior context has no style callback.
    Absent,
    /// The live descriptor was unavailable or was not the exact audited Player
    /// Control descriptor. Its callback must not be inferred from entity type.
    Unauthenticated,
    /// Player Control was active, but `FUN_00443AF0` rejected the callback
    /// handle before feedback or material damage.
    ControllerMismatch {
        entity_handle: u32,
        controlled_entity_handle: Option<u32>,
    },
    PlayerControl(PlayerControlTerrainStyleOutcome),
    DyingBounce(PlayerDyingSurfaceContactOutcome),
}

/// Player-visible state changes produced by one terrain response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerTerrainContactOutcome {
    pub contact: TerrainModelContact,
    pub position_before_raw: [i16; 3],
    pub position_after_raw: [i16; 3],
    pub velocity_before_raw: [i16; 3],
    pub velocity_after_raw: [i16; 3],
    pub style_callback: PlayerTerrainStyleCallbackOutcome,
    pub collision_impact_raw: i32,
    pub collision_packet: DamagePacket,
    pub collision_damage_raw: i32,
    pub hull_damage: HullDamageOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainContactError {
    CollisionProgram(ModelCollisionError),
    StyleCallback(PlayerContactStyleBlock),
    NativeRuntime(&'static str),
    NativeDeath(crate::entity::PlayerCheckedDamageBlock),
}

/// Retained D7F0 contact suffix. Consuming it after the style packet's synchronous
/// death constructor prevents replay and lets 141D0 read the current velocity.
pub struct PlayerTerrainContactContinuation {
    contact: TerrainModelContact,
    position_before_raw: [i16; 3],
    velocity_before_raw: [i16; 3],
    style_callback: PlayerTerrainStyleCallbackOutcome,
}

/// Convenience complete dispatch for callers without native death custody.
pub fn resolve_player_terrain_contact<'a>(
    terrain: &TerrainGrid,
    active_model: &ModelEntry,
    active_model_to_world_basis: [[f64; 3]; 3],
    active_anim_vars: &AnimVars,
    style_request: impl Into<PlayerTerrainStyleCallback<'a>>,
    position_raw: &mut [i16; 3],
    velocity_raw: &mut [i16; 3],
    hull: &mut PlayerHull,
) -> Result<Option<PlayerTerrainContactOutcome>, TerrainContactError> {
    begin_player_terrain_contact(
        terrain,
        active_model,
        active_model_to_world_basis,
        active_anim_vars,
        style_request,
        position_raw,
        velocity_raw,
        hull,
    )
    .map(|continuation| {
        continuation.map(|continuation| continuation.finish(position_raw, velocity_raw, hull))
    })
}

/// Resolve the oriented player model against bare terrain and apply retail's
/// mode-1 physical response atomically to the supplied motion/hull state.
pub fn begin_player_terrain_contact<'a>(
    terrain: &TerrainGrid,
    active_model: &ModelEntry,
    active_model_to_world_basis: [[f64; 3]; 3],
    active_anim_vars: &AnimVars,
    style_request: impl Into<PlayerTerrainStyleCallback<'a>>,
    position_raw: &mut [i16; 3],
    velocity_raw: &mut [i16; 3],
    hull: &mut PlayerHull,
) -> Result<Option<PlayerTerrainContactContinuation>, TerrainContactError> {
    let Some(hit) = active_model
        .collide_terrain_raw_oriented(
            terrain,
            *position_raw,
            active_model_to_world_basis,
            active_anim_vars,
        )
        .map_err(TerrainContactError::CollisionProgram)?
    else {
        return Ok(None);
    };

    let contact = TerrainModelContact {
        normal_q12: hit
            .normal
            .map(|component| (component * 4096.0).round() as i16),
        penetration_raw: hit.penetration_raw as i32,
    };
    begin_player_terrain_contact_with_style_raw(
        terrain,
        contact,
        style_request.into(),
        position_raw,
        velocity_raw,
        hull,
    )
    .map(Some)
}

#[cfg(test)]
fn apply_player_terrain_contact_raw(
    terrain: &TerrainGrid,
    contact: TerrainModelContact,
    style_request: PlayerTerrainStyleCallbackRequest,
    position_raw: &mut [i16; 3],
    velocity_raw: &mut [i16; 3],
    hull: &mut PlayerHull,
) -> PlayerTerrainContactOutcome {
    begin_player_terrain_contact_with_style_raw(
        terrain,
        contact,
        style_request.into(),
        position_raw,
        velocity_raw,
        hull,
    )
    .expect("Player Control callback is representable")
    .finish(position_raw, velocity_raw, hull)
}

fn begin_player_terrain_contact_with_style_raw(
    terrain: &TerrainGrid,
    contact: TerrainModelContact,
    style_request: PlayerTerrainStyleCallback<'_>,
    position_raw: &mut [i16; 3],
    velocity_raw: &mut [i16; 3],
    hull: &mut PlayerHull,
) -> Result<PlayerTerrainContactContinuation, TerrainContactError> {
    let position_before_raw = *position_raw;
    let velocity_before_raw = *velocity_raw;
    let style_callback = match style_request {
        PlayerTerrainStyleCallback::PlayerControl(request) => apply_player_terrain_style_callback(
            terrain,
            position_before_raw,
            velocity_before_raw,
            contact,
            request,
            hull,
        ),
        PlayerTerrainStyleCallback::DyingBounce(frame) => {
            PlayerTerrainStyleCallbackOutcome::DyingBounce(
                apply_player_dying_surface_contact(
                    frame,
                    position_before_raw,
                    contact.normal_q12,
                    velocity_raw,
                )
                .map_err(TerrainContactError::StyleCallback)?,
            )
        }
    };
    Ok(PlayerTerrainContactContinuation {
        contact,
        position_before_raw,
        velocity_before_raw,
        style_callback,
    })
}

impl PlayerTerrainContactContinuation {
    pub fn style_callback(&self) -> PlayerTerrainStyleCallbackOutcome {
        self.style_callback
    }

    pub fn finish(
        self,
        position_raw: &mut [i16; 3],
        velocity_raw: &mut [i16; 3],
        hull: &mut PlayerHull,
    ) -> PlayerTerrainContactOutcome {
        let velocity_before_response_raw = *velocity_raw;
        apply_common_solid_terrain_motion_raw(position_raw, velocity_raw, self.contact);
        let collision_impact_raw = hull
            .profile()
            .impact_raw(velocity_before_response_raw, *velocity_raw);
        let collision_packet = DamagePacket::collision(collision_impact_raw);
        let collision_damage_raw = collision_packet.filtered_raw(Some(&hull.profile().damage));
        let hull_damage = hull.apply_damage_raw(collision_damage_raw);
        PlayerTerrainContactOutcome {
            contact: self.contact,
            position_before_raw: self.position_before_raw,
            position_after_raw: *position_raw,
            velocity_before_raw: self.velocity_before_raw,
            velocity_after_raw: *velocity_raw,
            style_callback: self.style_callback,
            collision_impact_raw,
            collision_packet,
            collision_damage_raw,
            hull_damage,
        }
    }
}

fn apply_player_terrain_style_callback(
    terrain: &TerrainGrid,
    position_raw: [i16; 3],
    velocity_raw: [i16; 3],
    contact: TerrainModelContact,
    request: PlayerTerrainStyleCallbackRequest,
    hull: &mut PlayerHull,
) -> PlayerTerrainStyleCallbackOutcome {
    match request.behavior_context {
        RetailRuntimeValue::Known(None) => return PlayerTerrainStyleCallbackOutcome::Absent,
        RetailRuntimeValue::Known(Some(context))
            if player_surface_style(context) == Some(PlayerSurfaceStyle::Null) =>
        {
            return PlayerTerrainStyleCallbackOutcome::Absent
        }
        RetailRuntimeValue::Known(Some(context)) if is_player_control_descriptor(context) => {}
        RetailRuntimeValue::Known(Some(_)) | RetailRuntimeValue::Unresolved => {
            return PlayerTerrainStyleCallbackOutcome::Unauthenticated;
        }
    }
    if request.controlled_entity_handle != Some(request.entity_handle) {
        return PlayerTerrainStyleCallbackOutcome::ControllerMismatch {
            entity_handle: request.entity_handle,
            controlled_entity_handle: request.controlled_entity_handle,
        };
    }

    let normal_velocity_raw = q12_dot(contact.normal_q12, velocity_raw);
    let inward_speed_raw = normal_velocity_raw.wrapping_neg();
    let unsupported_haptic =
        (inward_speed_raw > PLAYER_TERRAIN_HAPTIC_INWARD_THRESHOLD_RAW).then(|| {
            let strength_raw = request.haptic_scale_raw.wrapping_mul(
                inward_speed_raw.wrapping_sub(PLAYER_TERRAIN_HAPTIC_INWARD_THRESHOLD_RAW),
            ) / 15;
            PlayerTerrainUnsupportedHaptic {
                inward_speed_raw,
                haptic_scale_raw: request.haptic_scale_raw,
                strength_raw: strength_raw.min(PLAYER_TERRAIN_HAPTIC_STRENGTH_MAX_RAW),
            }
        });

    let material_code = nearest_cell_material_code(terrain, position_raw[0], position_raw[2]);
    let response_selector = request.ground_response_selectors[usize::from(material_code)];
    let (damage_delivery, damage_raw, hull_damage) =
        if response_selector == PLAYER_TERRAIN_DAMAGE_RESPONSE_SELECTOR {
            let delivery = PLAYER_CONTROL_TERRAIN_DAMAGE_DELIVERY;
            let damage_raw = delivery.packet.filtered_raw(Some(&hull.profile().damage));
            (
                Some(delivery),
                damage_raw,
                hull.apply_damage_raw(damage_raw),
            )
        } else {
            (None, 0, HullDamageOutcome::default())
        };

    PlayerTerrainStyleCallbackOutcome::PlayerControl(PlayerControlTerrainStyleOutcome {
        vehicle_mode: request.vehicle_mode,
        style_address: player_control_style_address(request.vehicle_mode),
        normal_velocity_raw,
        unsupported_haptic,
        material_code,
        response_selector,
        damage_delivery,
        damage_raw,
        hull_damage,
    })
}

const fn player_control_style_address(vehicle_mode: VehicleMode) -> u32 {
    match vehicle_mode {
        VehicleMode::Hover => PLAYER_CONTROL_HOVER_STYLE_ADDRESS,
        VehicleMode::Vtol => PLAYER_CONTROL_VTOL_STYLE_ADDRESS,
    }
}

fn is_player_control_descriptor(context: BehaviorContextRuntime) -> bool {
    player_surface_style(context) == Some(PlayerSurfaceStyle::PlayerControl)
}

fn q12_dot(lhs_q12: [i16; 3], rhs_raw: [i16; 3]) -> i32 {
    i32::from(lhs_q12[1])
        .wrapping_mul(i32::from(rhs_raw[1]))
        .wrapping_add(i32::from(lhs_q12[2]).wrapping_mul(i32::from(rhs_raw[2])))
        .wrapping_add(i32::from(lhs_q12[0]).wrapping_mul(i32::from(rhs_raw[0])))
        >> 12
}

/// Arithmetic retained from `FUN_004141D0`'s solid-terrain response.
///
/// Presentation uses the combined response before its final `>> 1`; collision
/// damage uses the velocity difference after this motion response. Neither
/// presentation nor damage is submitted by this kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonSolidTerrainMotionOutcome {
    /// `None` means velocity was already directed out of the terrain.
    pub inward_speed_raw: Option<i32>,
    /// Inward speed plus its signed-Q31 `0x7fff_ffff` product.
    pub combined_response_raw: Option<i32>,
}

/// `FUN_004141D0`'s solid-terrain motion with response `0x7fff_ffff`.
///
/// Position separation always precedes the inward-velocity test. The caller
/// owns the grounded state write, impact sound, material response and checked
/// channel-1 damage in their native order. This is not the water-entry branch.
pub fn apply_common_solid_terrain_motion_raw(
    position_raw: &mut [i16; 3],
    velocity_raw: &mut [i16; 3],
    contact: TerrainModelContact,
) -> CommonSolidTerrainMotionOutcome {
    for (position, &normal) in position_raw.iter_mut().zip(&contact.normal_q12) {
        let correction_raw = (i32::from(normal).wrapping_mul(contact.penetration_raw) >> 12) as i16;
        *position = position.wrapping_add(correction_raw);
    }

    let inward_raw = contact.normal_q12.iter().zip(velocity_raw.iter()).fold(
        0i32,
        |sum, (&normal, &velocity)| {
            sum.wrapping_add(i32::from(normal).wrapping_mul(i32::from(velocity)))
        },
    ) >> 12;
    if inward_raw >= 0 {
        return CommonSolidTerrainMotionOutcome {
            inward_speed_raw: None,
            combined_response_raw: None,
        };
    }

    let inward_speed_raw = inward_raw.wrapping_neg();
    let combined_response_raw = inward_speed_raw.wrapping_add(q31_mul(inward_speed_raw, i32::MAX));
    let response_raw = combined_response_raw >> 1;
    for (velocity, &normal) in velocity_raw.iter_mut().zip(&contact.normal_q12) {
        let correction_raw = (i32::from(normal).wrapping_mul(response_raw) >> 12) as i16;
        *velocity = velocity.wrapping_add(correction_raw);
    }
    CommonSolidTerrainMotionOutcome {
        inward_speed_raw: Some(inward_speed_raw),
        combined_response_raw: Some(combined_response_raw),
    }
}

fn q31_mul(lhs: i32, rhs: i32) -> i32 {
    ((i64::from(lhs) * i64::from(rhs)) >> 31) as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_behavior::{BehaviorContextRuntime, BehaviorSelection};
    use v2k_formats::terrain::{TerrainCell, GRID_SIZE};

    const PLAYER_HANDLE: u32 = 0x047e_0001;

    fn flat_terrain(material: u8) -> TerrainGrid {
        TerrainGrid {
            header: [0; 5],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: material,
                };
                GRID_SIZE * GRID_SIZE
            ],
        }
    }

    fn behavior_context(class_id: u32) -> BehaviorContextRuntime {
        let program = behavior_program(class_id).expect("audited behavior program");
        BehaviorContextRuntime::from_published_weighted_selection(BehaviorSelection {
            choice_index: 0,
            program,
        })
        .expect("canonical initial style")
    }

    fn style_request(
        behavior_context: RetailRuntimeValue<Option<BehaviorContextRuntime>>,
        ground_response_selectors: [u8; 8],
    ) -> PlayerTerrainStyleCallbackRequest {
        PlayerTerrainStyleCallbackRequest {
            behavior_context,
            vehicle_mode: VehicleMode::Hover,
            entity_handle: PLAYER_HANDLE,
            controlled_entity_handle: Some(PLAYER_HANDLE),
            ground_response_selectors,
            haptic_scale_raw: DEFAULT_PLAYER_TERRAIN_HAPTIC_SCALE_RAW,
        }
    }

    fn upward_contact(penetration_raw: i32) -> TerrainModelContact {
        TerrainModelContact {
            normal_q12: [0, 4096, 0],
            penetration_raw,
        }
    }

    #[test]
    fn terrain_response_separates_hull_and_removes_inward_normal_motion() {
        let mut position = [100, 500, -200];
        let mut velocity = [321, -2048, -456];

        apply_common_solid_terrain_motion_raw(&mut position, &mut velocity, upward_contact(87));

        assert_eq!(position, [100, 587, -200]);
        assert_eq!(velocity, [321, -1, -456]);
    }

    #[test]
    fn terrain_response_keeps_outward_velocity_but_still_separates() {
        let mut position = [0, 10, 0];
        let mut velocity = [12, 34, 56];

        apply_common_solid_terrain_motion_raw(&mut position, &mut velocity, upward_contact(5));

        assert_eq!(position, [0, 15, 0]);
        assert_eq!(velocity, [12, 34, 56]);
    }

    #[test]
    fn terrain_response_projects_along_sloped_q12_normal() {
        let mut position = [0, 0, 0];
        let mut velocity = [-800, -800, 77];
        let contact = TerrainModelContact {
            normal_q12: [2896, 2896, 0],
            penetration_raw: 100,
        };

        apply_common_solid_terrain_motion_raw(&mut position, &mut velocity, contact);

        assert_eq!(position, [70, 70, 0]);
        assert!(velocity[0].abs() <= 2);
        assert!(velocity[1].abs() <= 2);
        assert_eq!(velocity[2], 77);
    }

    #[test]
    fn player_control_haptic_uses_q12_inward_speed_strict_threshold_and_cap() {
        let terrain = flat_terrain(0);
        let request = style_request(
            RetailRuntimeValue::Known(Some(behavior_context(PLAYER_CONTROL_BEHAVIOR_CLASS_ID))),
            [0; 8],
        );

        let mut hull = PlayerHull::default();
        let below_threshold = apply_player_terrain_style_callback(
            &terrain,
            [0; 3],
            [0, -0x1ff, 0],
            upward_contact(1),
            request,
            &mut hull,
        );
        let PlayerTerrainStyleCallbackOutcome::PlayerControl(below_threshold) = below_threshold
        else {
            panic!("exact Player Control style must be admitted");
        };
        assert_eq!(below_threshold.normal_velocity_raw, -0x1ff);
        assert_eq!(below_threshold.unsupported_haptic, None);

        let at_threshold = apply_player_terrain_style_callback(
            &terrain,
            [0; 3],
            [0, -0x200, 0],
            upward_contact(1),
            request,
            &mut hull,
        );
        let PlayerTerrainStyleCallbackOutcome::PlayerControl(at_threshold) = at_threshold else {
            panic!("exact Player Control style must be admitted");
        };
        assert_eq!(at_threshold.normal_velocity_raw, -0x200);
        assert_eq!(at_threshold.unsupported_haptic, None);

        let one_above = apply_player_terrain_style_callback(
            &terrain,
            [0; 3],
            [0, -0x201, 0],
            upward_contact(1),
            request,
            &mut hull,
        );
        let PlayerTerrainStyleCallbackOutcome::PlayerControl(one_above) = one_above else {
            panic!("exact Player Control style must be admitted");
        };
        assert_eq!(
            one_above.unsupported_haptic,
            Some(PlayerTerrainUnsupportedHaptic {
                inward_speed_raw: 0x201,
                haptic_scale_raw: 15,
                strength_raw: 1,
            })
        );

        let capped = apply_player_terrain_style_callback(
            &terrain,
            [0; 3],
            [0, -0x300, 0],
            upward_contact(1),
            request,
            &mut hull,
        );
        let PlayerTerrainStyleCallbackOutcome::PlayerControl(capped) = capped else {
            panic!("exact Player Control style must be admitted");
        };
        assert_eq!(capped.unsupported_haptic.unwrap().strength_raw, 0xff);
    }

    #[test]
    fn hover_and_vtol_modes_admit_exact_styles_with_shared_terrain_callback() {
        assert_eq!(
            player_control_style_address(VehicleMode::Hover),
            PLAYER_CONTROL_HOVER_STYLE_ADDRESS
        );
        assert_eq!(
            player_control_style_address(VehicleMode::Vtol),
            PLAYER_CONTROL_VTOL_STYLE_ADDRESS
        );
        assert_eq!(PLAYER_CONTROL_TERRAIN_CALLBACK_ADDRESS, 0x0044_8280);

        let terrain = flat_terrain(0);
        let context = behavior_context(PLAYER_CONTROL_BEHAVIOR_CLASS_ID);
        // BehaviorContextRuntime currently retains the initial Hover style;
        // PlayerCraft.mode is the current retail style-pointer authority.
        assert_eq!(
            context.active_style().style_address(),
            PLAYER_CONTROL_HOVER_STYLE_ADDRESS
        );

        for vehicle_mode in [VehicleMode::Hover, VehicleMode::Vtol] {
            let request = PlayerTerrainStyleCallbackRequest {
                vehicle_mode,
                ..style_request(RetailRuntimeValue::Known(Some(context)), [0; 8])
            };
            let mut hull = PlayerHull::default();
            let outcome = apply_player_terrain_style_callback(
                &terrain,
                [0; 3],
                [0, -0x201, 0],
                upward_contact(1),
                request,
                &mut hull,
            );
            let PlayerTerrainStyleCallbackOutcome::PlayerControl(outcome) = outcome else {
                panic!("both exact Player Control mode styles must be admitted");
            };
            assert_eq!(outcome.vehicle_mode, vehicle_mode);
            assert_eq!(
                outcome.style_address,
                player_control_style_address(vehicle_mode)
            );
        }
    }

    #[test]
    fn player_control_dot_preserves_sloped_axis_order_and_wrapping_sum() {
        let sloped_normal = [2_896, 2_896, -1_024];
        let velocity = [-800, -900, 777];
        let expected = i32::from(sloped_normal[1])
            .wrapping_mul(i32::from(velocity[1]))
            .wrapping_add(i32::from(sloped_normal[2]).wrapping_mul(i32::from(velocity[2])))
            .wrapping_add(i32::from(sloped_normal[0]).wrapping_mul(i32::from(velocity[0])))
            >> 12;
        assert_eq!(q12_dot(sloped_normal, velocity), expected);

        let wrapping_normal = [i16::MAX; 3];
        let wrapping_velocity = [i16::MAX; 3];
        let product = i32::from(i16::MAX).wrapping_mul(i32::from(i16::MAX));
        assert_eq!(
            q12_dot(wrapping_normal, wrapping_velocity),
            product.wrapping_add(product).wrapping_add(product) >> 12
        );
    }

    #[test]
    fn selector_seven_uses_nearest_wrapped_x_major_material_and_exact_delivery() {
        let mut terrain = flat_terrain(0);
        // x=+0x80 rounds to cell 1; z=-0x81 rounds across the torus to 255.
        terrain.cells[GRID_SIZE + 255].terrain_type = 0x0d;
        let mut selectors = [0; 8];
        selectors[5] = 7;
        let request = style_request(
            RetailRuntimeValue::Known(Some(behavior_context(PLAYER_CONTROL_BEHAVIOR_CLASS_ID))),
            selectors,
        );
        let mut position = [0x80, 100, -0x81];
        let mut velocity = [0, -0x201, 0];
        let mut hull = PlayerHull::default();

        let outcome = apply_player_terrain_contact_raw(
            &terrain,
            upward_contact(3),
            request,
            &mut position,
            &mut velocity,
            &mut hull,
        );

        let PlayerTerrainStyleCallbackOutcome::PlayerControl(style) = outcome.style_callback else {
            panic!("exact Player Control style must be admitted");
        };
        assert_eq!(style.material_code, 5);
        assert_eq!(style.response_selector, 7);
        assert_eq!(
            style.damage_delivery.unwrap().raw_dwords(),
            [3, 0, 15_000, 0, 0xffff_fffd, 0]
        );
        assert_eq!(style.damage_raw, 30_000);
        assert_eq!(style.hull_damage.health_lost_raw, 30_000);
        assert_eq!(hull.health_raw, 10_000);

        // FUN_0040D7F0 retains FUN_004141D0 after the style damage.
        assert_eq!(position, [0x80, 103, -0x81]);
        assert_eq!(velocity, [0, -1, 0]);
        assert_eq!(outcome.collision_damage_raw, 0);

        let terrain = flat_terrain(7);
        let mut selectors = [0; 8];
        selectors[7] = 6;
        let request = style_request(
            RetailRuntimeValue::Known(Some(behavior_context(PLAYER_CONTROL_BEHAVIOR_CLASS_ID))),
            selectors,
        );
        let mut hull = PlayerHull::default();
        let no_damage = apply_player_terrain_style_callback(
            &terrain,
            [0; 3],
            [0, -0x201, 0],
            upward_contact(1),
            request,
            &mut hull,
        );
        let PlayerTerrainStyleCallbackOutcome::PlayerControl(no_damage) = no_damage else {
            panic!("exact Player Control style must be admitted");
        };
        assert_eq!(no_damage.material_code, 7);
        assert_eq!(no_damage.response_selector, 6);
        assert_eq!(no_damage.damage_delivery, None);
        assert_eq!(no_damage.damage_raw, 0);
        assert_eq!(hull.health_raw, 40_000);
    }

    #[test]
    fn style_and_controller_mismatches_skip_only_the_optional_prefix() {
        let terrain = flat_terrain(0);
        let selectors = [7; 8];
        let cases = [
            style_request(
                RetailRuntimeValue::Known(Some(behavior_context(0))),
                selectors,
            ),
            PlayerTerrainStyleCallbackRequest {
                controlled_entity_handle: Some(PLAYER_HANDLE.wrapping_add(1)),
                ..style_request(
                    RetailRuntimeValue::Known(Some(behavior_context(
                        PLAYER_CONTROL_BEHAVIOR_CLASS_ID,
                    ))),
                    selectors,
                )
            },
        ];

        for (index, request) in cases.into_iter().enumerate() {
            let mut position = [0, 10, 0];
            let mut velocity = [0, -0x201, 0];
            let mut hull = PlayerHull::default();
            let outcome = apply_player_terrain_contact_raw(
                &terrain,
                upward_contact(2),
                request,
                &mut position,
                &mut velocity,
                &mut hull,
            );

            if index == 0 {
                assert_eq!(
                    outcome.style_callback,
                    PlayerTerrainStyleCallbackOutcome::Unauthenticated
                );
            } else {
                assert_eq!(
                    outcome.style_callback,
                    PlayerTerrainStyleCallbackOutcome::ControllerMismatch {
                        entity_handle: PLAYER_HANDLE,
                        controlled_entity_handle: Some(PLAYER_HANDLE.wrapping_add(1)),
                    }
                );
            }
            assert_eq!(hull.health_raw, 40_000);
            assert_eq!(position, [0, 12, 0]);
            assert_eq!(velocity, [0, -1, 0]);
        }
    }

    #[test]
    fn lethal_style_damage_still_runs_common_separation_suffix() {
        let terrain = flat_terrain(0);
        let request = style_request(
            RetailRuntimeValue::Known(Some(behavior_context(PLAYER_CONTROL_BEHAVIOR_CLASS_ID))),
            [7; 8],
        );
        let mut position = [4, 20, 8];
        let mut velocity = [0, -0x201, 0];
        let mut hull = PlayerHull::default();
        hull.health_raw = 20_000;

        let outcome = apply_player_terrain_contact_raw(
            &terrain,
            upward_contact(6),
            request,
            &mut position,
            &mut velocity,
            &mut hull,
        );

        let PlayerTerrainStyleCallbackOutcome::PlayerControl(style) = outcome.style_callback else {
            panic!("exact Player Control style must be admitted");
        };
        assert_eq!(style.damage_raw, 30_000);
        assert!(style.hull_damage.destroyed_now);
        assert!(hull.dying);
        assert_eq!(position, [4, 26, 8]);
        assert_eq!(velocity, [0, -1, 0]);
    }

    #[test]
    fn fast_terrain_response_crosses_the_player_collision_damage_threshold() {
        let mut position = [0, 100, 0];
        let mut velocity = [0, -4096, 0];
        let velocity_before = velocity;
        apply_common_solid_terrain_motion_raw(&mut position, &mut velocity, upward_contact(1));

        let mut hull = PlayerHull::default();
        let impact = hull.profile().impact_raw(velocity_before, velocity);
        let damage = hull.collision_damage_raw(velocity_before, velocity);
        let outcome = hull.apply_damage_raw(damage);

        assert!(impact > 6_000);
        assert!(damage > 0);
        assert_eq!(outcome.health_lost_raw, damage);
    }
}
