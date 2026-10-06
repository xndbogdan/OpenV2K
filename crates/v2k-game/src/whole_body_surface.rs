//! Pure whole-body terrain/water classification and fallback-response planning.
//!
//! Retail separates these phases. `FUN_004129B0` first classifies the active
//! model's collision sphere against the animated water surface and returns an
//! above-to-water edge after writing the entity's surface-state bits. On that
//! edge the outer walker plays the type-record sound and then chooses exactly
//! one response: a non-null type/style callback, or `FUN_004141D0` as the
//! fallback. Consequently a caller must commit the returned classification
//! state before dispatching either exclusive response branch.

use crate::entity_collision_state::{
    RetailRuntimeValue, RetailStateWord, FULLY_ABOVE_SURFACE_STATE_BIT,
    FULLY_BELOW_SURFACE_STATE_BIT, SURFACE_STATE_MASK,
};
use v2k_formats::terrain::{wave_surface_raw, TerrainGrid};

/// `FUN_004141D0` uses the ordinary fixed-four response at this velocity and
/// above. More-negative values select the hard water-entry operation.
pub(crate) const WATER_ENTRY_HARD_VELOCITY_THRESHOLD_RAW: i16 = -1000;
/// The hard operation switches its model payload from `0x84` to `0x82` only
/// below this strict boundary.
pub(crate) const WATER_ENTRY_SEVERE_VELOCITY_THRESHOLD_RAW: i16 = -1750;

/// Inputs read by `FUN_004129B0` for one entity.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WholeBodySurfaceClassificationRequest<'a> {
    /// Live entity state before classification.
    pub state_before: RetailStateWord,
    /// Complete wrapping 256x256 terrain grid.
    pub terrain: &'a TerrainGrid,
    /// Entity center in retail signed-8.8 position words.
    pub position_raw: [i16; 3],
    /// Active Section-8 model collision radius from header `+0x0A`.
    pub collision_radius_raw: u16,
    /// Wrapping `DAT_004FED60` tick used by the animated water surface.
    pub retail_tick: u32,
    /// `DAT_004FECE4`, copied from the current Section-13 descriptor +0x84.
    /// This is independent of whether the sea plane is visible.
    pub waves_enabled: bool,
    /// Static sea plane as a signed-8.8 raw word. `None` represents a level
    /// whose water pass is disabled and therefore preserves the prior state.
    pub static_sea_level_raw: Option<i16>,
}

/// Contact data produced only by a known above-to-intersecting/below edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WholeBodySurfaceEntryContact {
    pub position_raw: [i16; 3],
    pub surface_y_raw: i16,
    /// Low three bits of the nearest rounded terrain cell's material.
    pub material_code: u8,
}

/// Proposed result of `FUN_004129B0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WholeBodySurfaceClassification {
    /// State with the surface bits updated on wet cells. Dry cells preserve the
    /// complete input, including unknown-bit provenance.
    pub state_after: RetailStateWord,
    /// The one-shot edge which permits the caller to choose its resolved
    /// type/style callback or the common fallback response.
    pub entry_contact: Option<WholeBodySurfaceEntryContact>,
}

/// Classify an entity collision sphere against the authored animated surface.
///
/// The wet/dry gate uses the current integer terrain cell. The response
/// material is sampled from the independently rounded nearest cell. Sphere
/// comparisons are strict, so exact top/bottom equality means intersection.
pub(crate) fn classify_whole_body_surface(
    request: WholeBodySurfaceClassificationRequest<'_>,
) -> WholeBodySurfaceClassification {
    let Some(static_sea_level_raw) = request.static_sea_level_raw else {
        return WholeBodySurfaceClassification {
            state_after: request.state_before,
            entry_contact: None,
        };
    };

    let terrain_y_raw = current_cell_height_raw(
        request.terrain,
        request.position_raw[0],
        request.position_raw[2],
    );
    if static_sea_level_raw <= terrain_y_raw {
        return WholeBodySurfaceClassification {
            state_after: request.state_before,
            entry_contact: None,
        };
    }

    let surface_y_raw = if request.waves_enabled {
        wave_surface_raw(
            request.position_raw[0],
            request.position_raw[2],
            request.retail_tick as i32,
            static_sea_level_raw,
            terrain_y_raw,
        )
    } else {
        static_sea_level_raw
    };
    let center = i32::from(request.position_raw[1]);
    let radius = i32::from(request.collision_radius_raw);
    let surface = i32::from(surface_y_raw);
    let next_surface_bits = if center + radius < surface {
        FULLY_BELOW_SURFACE_STATE_BIT
    } else if surface < center - radius {
        FULLY_ABOVE_SURFACE_STATE_BIT
    } else {
        0
    };

    // Retail tests only the prior above bit, not equality of the complete
    // two-bit field. This matters for a raw state containing both bits.
    let entered_surface = matches!(
        request.state_before.masked(FULLY_ABOVE_SURFACE_STATE_BIT),
        RetailRuntimeValue::Known(FULLY_ABOVE_SURFACE_STATE_BIT)
    ) && next_surface_bits != FULLY_ABOVE_SURFACE_STATE_BIT;

    let mut state_after = request.state_before;
    state_after.overwrite(SURFACE_STATE_MASK, next_surface_bits);
    let entry_contact = entered_surface.then(|| WholeBodySurfaceEntryContact {
        position_raw: request.position_raw,
        surface_y_raw,
        material_code: nearest_cell_material_code(
            request.terrain,
            request.position_raw[0],
            request.position_raw[2],
        ),
    });

    WholeBodySurfaceClassification {
        state_after,
        entry_contact,
    }
}

/// Authored common presentation selected by `FUN_004141D0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WholeBodySurfaceResponse {
    /// Material-selected fixed-four particle response.
    SurfaceBurst { response_selector: u8 },
    /// Class-0x3C water ring, sound 17, and signed velocity damping.
    HardImpact { severe: bool },
}

/// Inputs for the common fallback when the type/style callback is null.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WholeBodySurfaceFallbackResponseRequest {
    pub contact: WholeBodySurfaceEntryContact,
    /// Signed Y velocity immediately before the common response.
    pub vertical_velocity_raw: i16,
    /// Section-13 material-to-response table installed at `DAT_004FEC68`.
    pub water_response_selectors: [u8; 8],
}

/// Proposed common response and resulting Y velocity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WholeBodySurfaceFallbackResponsePlan {
    /// `None` is selector 7's total early return.
    pub response: Option<WholeBodySurfaceResponse>,
    pub vertical_velocity_after_raw: i16,
}

/// Plan `FUN_004141D0`'s fallback whole-body water-entry response.
pub(crate) fn plan_fallback_whole_body_surface_response(
    request: WholeBodySurfaceFallbackResponseRequest,
) -> WholeBodySurfaceFallbackResponsePlan {
    let response_selector =
        request.water_response_selectors[usize::from(request.contact.material_code & 7)];
    if response_selector == 7 {
        return WholeBodySurfaceFallbackResponsePlan {
            response: None,
            vertical_velocity_after_raw: request.vertical_velocity_raw,
        };
    }

    if request.vertical_velocity_raw >= WATER_ENTRY_HARD_VELOCITY_THRESHOLD_RAW {
        WholeBodySurfaceFallbackResponsePlan {
            response: Some(WholeBodySurfaceResponse::SurfaceBurst { response_selector }),
            vertical_velocity_after_raw: request.vertical_velocity_raw,
        }
    } else {
        WholeBodySurfaceFallbackResponsePlan {
            response: Some(WholeBodySurfaceResponse::HardImpact {
                severe: request.vertical_velocity_raw < WATER_ENTRY_SEVERE_VELOCITY_THRESHOLD_RAW,
            }),
            // Retail's signed SAR rounds negative odd values toward negative
            // infinity; Rust's signed right shift has the same semantics.
            vertical_velocity_after_raw: request.vertical_velocity_raw >> 1,
        }
    }
}

/// Exact current-cell signed height byte consumed by `FUN_004129B0`.
fn current_cell_height_raw(terrain: &TerrainGrid, x_raw: i16, z_raw: i16) -> i16 {
    let x = usize::from((x_raw as u16) >> 8);
    let z = usize::from((z_raw as u16) >> 8);
    i16::from(terrain.cell(x, z).expect("complete 256x256 terrain").height as i8) * 0x20
}

/// Exact nearest-cell material packing used for the response contact.
pub(crate) fn nearest_cell_material_code(terrain: &TerrainGrid, x_raw: i16, z_raw: i16) -> u8 {
    let x = usize::from((x_raw.wrapping_add(0x80) as u16) >> 8);
    let z = usize::from((z_raw.wrapping_add(0x80) as u16) >> 8);
    terrain
        .cell(x, z)
        .expect("complete 256x256 terrain")
        .terrain_type
        & 7
}

#[cfg(test)]
mod tests {
    use super::*;
    use v2k_formats::terrain::{TerrainCell, GRID_SIZE};

    const UNRELATED_STATE_BIT: u32 = 0x0000_0080;

    fn flat_terrain(height: i8, material: u8) -> TerrainGrid {
        TerrainGrid {
            header: [0; 5],
            cells: vec![
                TerrainCell {
                    height: height as u8,
                    attribute: 0,
                    terrain_type: material,
                };
                GRID_SIZE * GRID_SIZE
            ],
        }
    }

    fn classify(
        state_before: RetailStateWord,
        center_y_raw: i16,
        collision_radius_raw: u16,
    ) -> WholeBodySurfaceClassification {
        let terrain = flat_terrain(-128, 3);
        classify_whole_body_surface(WholeBodySurfaceClassificationRequest {
            state_before,
            terrain: &terrain,
            position_raw: [0, center_y_raw, 0],
            collision_radius_raw,
            retail_tick: 0,
            waves_enabled: true,
            static_sea_level_raw: Some(0),
        })
    }

    fn contact(material_code: u8) -> WholeBodySurfaceEntryContact {
        WholeBodySurfaceEntryContact {
            position_raw: [0; 3],
            surface_y_raw: 0,
            material_code,
        }
    }

    fn plan(
        vertical_velocity_raw: i16,
        response_selector: u8,
    ) -> WholeBodySurfaceFallbackResponsePlan {
        plan_fallback_whole_body_surface_response(WholeBodySurfaceFallbackResponseRequest {
            contact: contact(5),
            vertical_velocity_raw,
            water_response_selectors: [response_selector; 8],
        })
    }

    #[test]
    fn disabled_wave_global_uses_static_sea_on_a_wet_cell() {
        let terrain = flat_terrain(-128, 3);
        let tick = (0..1024)
            .find(|tick| wave_surface_raw(512, 1792, *tick, 0, -4096) > 1)
            .expect("a genuine positive wave at this fixed coordinate");
        let request = WholeBodySurfaceClassificationRequest {
            state_before: RetailStateWord::from_known_bits(
                FULLY_ABOVE_SURFACE_STATE_BIT,
                SURFACE_STATE_MASK,
            ),
            terrain: &terrain,
            position_raw: [512, 1, 1792],
            collision_radius_raw: 0,
            retail_tick: tick as u32,
            static_sea_level_raw: Some(0),
            waves_enabled: false,
        };
        let flat = classify_whole_body_surface(request);
        assert_eq!(
            flat.state_after.masked(SURFACE_STATE_MASK),
            RetailRuntimeValue::Known(FULLY_ABOVE_SURFACE_STATE_BIT)
        );
        assert!(flat.entry_contact.is_none());
        let wave = classify_whole_body_surface(WholeBodySurfaceClassificationRequest {
            waves_enabled: true,
            ..request
        });
        assert_eq!(
            wave.state_after.masked(SURFACE_STATE_MASK),
            RetailRuntimeValue::Known(FULLY_BELOW_SURFACE_STATE_BIT)
        );
        assert_eq!(
            wave.entry_contact.unwrap().surface_y_raw,
            wave_surface_raw(512, 1792, tick, 0, -4096)
        );
    }

    #[test]
    fn dry_cells_and_disabled_water_preserve_complete_state() {
        let state_before = RetailStateWord::from_known_bits(
            FULLY_ABOVE_SURFACE_STATE_BIT | UNRELATED_STATE_BIT,
            SURFACE_STATE_MASK | UNRELATED_STATE_BIT,
        );
        let dry = flat_terrain(1, 0);

        for static_sea_level_raw in [None, Some(0)] {
            let result = classify_whole_body_surface(WholeBodySurfaceClassificationRequest {
                state_before,
                terrain: &dry,
                position_raw: [0, 0, 0],
                collision_radius_raw: 10,
                retail_tick: 0,
                waves_enabled: true,
                static_sea_level_raw,
            });
            assert_eq!(result.state_after, state_before);
            assert_eq!(result.entry_contact, None);
        }
    }

    #[test]
    fn unknown_first_classification_becomes_known_without_synthesizing_an_edge() {
        let result = classify(RetailStateWord::unknown(), 0, 10);
        assert_eq!(
            result.state_after.masked(SURFACE_STATE_MASK),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(result.entry_contact, None);
    }

    #[test]
    fn prior_above_bit_presence_drives_edge_even_if_both_bits_were_set() {
        let state_before = RetailStateWord::exact(
            FULLY_ABOVE_SURFACE_STATE_BIT | FULLY_BELOW_SURFACE_STATE_BIT | UNRELATED_STATE_BIT,
        );
        let result = classify(state_before, 0, 10);

        assert!(result.entry_contact.is_some());
        assert_eq!(
            result.state_after.masked(SURFACE_STATE_MASK),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            result.state_after.masked(UNRELATED_STATE_BIT),
            RetailRuntimeValue::Known(UNRELATED_STATE_BIT)
        );
    }

    #[test]
    fn known_above_bit_drives_edge_when_the_below_bit_is_unresolved() {
        let state_before = RetailStateWord::from_known_bits(
            FULLY_ABOVE_SURFACE_STATE_BIT,
            FULLY_ABOVE_SURFACE_STATE_BIT,
        );
        let result = classify(state_before, 0, 10);

        assert!(result.entry_contact.is_some());
        assert_eq!(
            result.state_after.masked(SURFACE_STATE_MASK),
            RetailRuntimeValue::Known(0)
        );
    }

    #[test]
    fn sphere_boundaries_are_strict() {
        let unknown = RetailStateWord::unknown();

        assert_eq!(
            classify(unknown, 11, 10)
                .state_after
                .masked(SURFACE_STATE_MASK),
            RetailRuntimeValue::Known(FULLY_ABOVE_SURFACE_STATE_BIT)
        );
        assert_eq!(
            classify(unknown, 10, 10)
                .state_after
                .masked(SURFACE_STATE_MASK),
            RetailRuntimeValue::Known(0),
            "bottom equality intersects"
        );
        assert_eq!(
            classify(unknown, -10, 10)
                .state_after
                .masked(SURFACE_STATE_MASK),
            RetailRuntimeValue::Known(0),
            "top equality intersects"
        );
        assert_eq!(
            classify(unknown, -11, 10)
                .state_after
                .masked(SURFACE_STATE_MASK),
            RetailRuntimeValue::Known(FULLY_BELOW_SURFACE_STATE_BIT)
        );
    }

    #[test]
    fn entry_material_uses_nearest_rounded_wrapping_cell() {
        let mut terrain = flat_terrain(-128, 1);
        terrain.cells[GRID_SIZE + 1].terrain_type = 5;
        terrain.cells[0].terrain_type = 6;
        let above =
            RetailStateWord::from_known_bits(FULLY_ABOVE_SURFACE_STATE_BIT, SURFACE_STATE_MASK);

        let rounded = classify_whole_body_surface(WholeBodySurfaceClassificationRequest {
            state_before: above,
            terrain: &terrain,
            position_raw: [0x00c0, 0, 0x00c0],
            collision_radius_raw: u16::MAX,
            retail_tick: 0,
            waves_enabled: true,
            static_sea_level_raw: Some(0),
        });
        assert_eq!(rounded.entry_contact.unwrap().material_code, 5);

        let wrapped = classify_whole_body_surface(WholeBodySurfaceClassificationRequest {
            state_before: above,
            terrain: &terrain,
            position_raw: [-0x0080, 0, -0x0080],
            collision_radius_raw: u16::MAX,
            retail_tick: 0,
            waves_enabled: true,
            static_sea_level_raw: Some(0),
        });
        assert_eq!(wrapped.entry_contact.unwrap().material_code, 6);
    }

    #[test]
    fn selector_seven_suppresses_every_response_and_velocity_change() {
        assert_eq!(
            plan(-1751, 7),
            WholeBodySurfaceFallbackResponsePlan {
                response: None,
                vertical_velocity_after_raw: -1751,
            }
        );
    }

    #[test]
    fn velocity_thresholds_match_the_retail_strict_comparisons() {
        assert_eq!(
            plan(-1000, 6),
            WholeBodySurfaceFallbackResponsePlan {
                response: Some(WholeBodySurfaceResponse::SurfaceBurst {
                    response_selector: 6,
                }),
                vertical_velocity_after_raw: -1000,
            }
        );
        assert_eq!(
            plan(-1001, 6),
            WholeBodySurfaceFallbackResponsePlan {
                response: Some(WholeBodySurfaceResponse::HardImpact { severe: false }),
                vertical_velocity_after_raw: -501,
            }
        );
        assert_eq!(
            plan(-1750, 6),
            WholeBodySurfaceFallbackResponsePlan {
                response: Some(WholeBodySurfaceResponse::HardImpact { severe: false }),
                vertical_velocity_after_raw: -875,
            }
        );
        assert_eq!(
            plan(-1751, 6),
            WholeBodySurfaceFallbackResponsePlan {
                response: Some(WholeBodySurfaceResponse::HardImpact { severe: true }),
                vertical_velocity_after_raw: -876,
            }
        );
    }

    #[test]
    fn hard_response_uses_signed_arithmetic_half_for_odd_negative_velocity() {
        assert_eq!(plan(-1003, 6).vertical_velocity_after_raw, -502);
    }
}
