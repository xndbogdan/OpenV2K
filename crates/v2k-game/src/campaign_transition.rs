//! Authored campaign routes and the retail player static-contact stamp.
//!
//! Retail does not encode a destination in entity type 111. `FUN_00433BD0`
//! scans Section 10 for static-object kinds `0x16..=0x1A` and materializes a
//! generic type-111/model-16 trigger at each marker. Section 13 owns the route:
//! flag-0x10 campaign records carry the destination, arrival, and marker
//! subtype consumed by `FUN_0042DD10`/`FUN_0042E270`.
//!
//! `427E20 -> 4464B0` records actual model contact before its physical response.
//! `42DD10 -> 42E300 -> 446440` consumes that stamp in campaign-record order,
//! with exact terrain-state selectors and a strict two-tick lifetime. Type111
//! proximity is neither the route predicate nor a source of spawn coordinates.

use crate::power_up_contact::{PlayerCampaignProgress, RETAIL_CONTROL_PAIR_LINK_COLUMNS};
use crate::static_contact::StaticModelContact;
use crate::static_kind_catalog::static_kind_descriptor;
use v2k_formats::anim_frames::TerrainObjectTable;
use v2k_formats::levels::{LevelDescriptor, CAMPAIGN_MARKER_TRANSITION_FLAG};
use v2k_formats::terrain::TerrainGrid;

pub const PEASANT_LEVEL_ID: u32 = 13;
pub const CISTERN_LEVEL_ID: u32 = 30;
pub const CAMPAIGN_ARRIVAL_HEADING_RAW: u16 = 0x4000;

mod arrivals;
pub use arrivals::{CampaignArrivalCatalog, CampaignArrivalEntry, DirectWorldEntry};

const FIRST_WARP_MARKER_KIND: u32 = 0x16;
const LAST_WARP_MARKER_KIND: u32 = 0x1a;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WarpArrival {
    pub position_raw: [i16; 3],
    pub heading_raw: u16,
}

/// `DAT_004D0B30`, copied by the fresh player controller in `443560`.
/// Actual campaign routes and native saves supply their own pose instead.
pub const RETAIL_CONTROLLER_DEFAULT_ARRIVAL: WarpArrival = WarpArrival {
    position_raw: [19712, -500, 14848],
    heading_raw: CAMPAIGN_ARRIVAL_HEADING_RAW,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CampaignWarpRoute {
    pub source_level_id: u32,
    pub marker_cell: [u8; 2],
    pub marker_kind: u32,
    pub marker_subtype: u8,
    pub marker_args: [i8; 2],
    pub destination_logical_level: u32,
    pub destination_level_id: u32,
    pub arrival: WarpArrival,
}

/// `456D10 ->42F140 ->42DE20`: an aborted wreck interior returns the
/// controller's current world, before any authored marker record is examined.
/// Its XYZ comes from retained session `+38/+3C`; `456D10` supplies heading4000.
/// There is no marker identity, pair-link column, or exit-bit write on this path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FailedWorldRetry {
    source_level_id: u32,
    arrival: WarpArrival,
}

impl FailedWorldRetry {
    pub fn new(source_level_id: u32, retained_position_raw: [i16; 3]) -> Option<Self> {
        (13..=48).contains(&source_level_id).then_some(Self {
            source_level_id,
            arrival: WarpArrival {
                position_raw: retained_position_raw,
                heading_raw: CAMPAIGN_ARRIVAL_HEADING_RAW,
            },
        })
    }
}

/// Both selector-zero transitions enter the existing map/loading owner.
/// Their campaign-state and arrival authorities remain distinct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignTransition {
    AuthoredMarker(CampaignWarpRoute),
    FailedWorldRetry(FailedWorldRetry),
}

impl CampaignTransition {
    pub const fn source_level_id(self) -> u32 {
        match self {
            Self::AuthoredMarker(route) => route.source_level_id,
            Self::FailedWorldRetry(retry) => retry.source_level_id,
        }
    }

    pub const fn destination_level_id(self) -> u32 {
        match self {
            Self::AuthoredMarker(route) => route.destination_level_id,
            Self::FailedWorldRetry(retry) => retry.source_level_id,
        }
    }

    pub const fn destination_logical_level(self) -> u32 {
        match self {
            Self::AuthoredMarker(route) => route.destination_logical_level,
            Self::FailedWorldRetry(retry) => retry.source_level_id - 12,
        }
    }

    pub const fn arrival(self) -> WarpArrival {
        match self {
            Self::AuthoredMarker(route) => route.arrival,
            Self::FailedWorldRetry(retry) => retry.arrival,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthoredWarpMarker {
    pub cell: [u8; 2],
    pub attribute: u8,
    pub terrain_type: u8,
    pub kind_index: u32,
    /// Marker selector `1..=5`, corresponding to world-state bits `4..=8`.
    pub subtype: u8,
    /// Global Section-8 model selected for the visible static marker.
    pub model_id: u16,
    /// Exact type-111 constructor position used by `FUN_00433BD0`.
    pub position_raw: [i16; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarpCatalogError {
    MissingDescriptor {
        cell: [u8; 2],
        attribute: u8,
    },
    InvalidDestinationLevel {
        level_id: u32,
        destination_logical_level: u32,
    },
    UnsupportedRouteFlags {
        level_id: u32,
        record_index: usize,
        flags: u32,
    },
    MissingRouteMarker {
        level_id: u32,
        record_index: usize,
        subtype: i8,
    },
}

/// The record index is the pair-link column source, including preceding abort
/// records. It must not be replaced by a compacted route index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthoredCampaignRoute {
    pub record_index: usize,
    pub route: CampaignWarpRoute,
}

pub fn authored_campaign_routes(
    level_id: u32,
    level: &LevelDescriptor,
    terrain: &TerrainGrid,
    terrain_objects: &TerrainObjectTable,
) -> Result<Vec<AuthoredCampaignRoute>, WarpCatalogError> {
    let markers = collect_authored_warp_markers(terrain, terrain_objects)?;
    let mut routes = Vec::new();
    for (record_index, record) in level.campaign_records.iter().enumerate() {
        // 42DE91..42E069 tests AL, not the uninterpreted high bytes. Cistern
        // authors 0x00AA0D10 for its first normal route.
        let flags = record.flags() & 0xff;
        if flags & 0x80 != 0 {
            continue;
        }
        if flags != CAMPAIGN_MARKER_TRANSITION_FLAG {
            return Err(WarpCatalogError::UnsupportedRouteFlags {
                level_id,
                record_index,
                flags,
            });
        }
        let transition = record.marker_transition().expect("flag 0x10");
        let marker = markers
            .iter()
            .find(|marker| i32::from(marker.subtype) == i32::from(transition.marker_subtype))
            .ok_or(WarpCatalogError::MissingRouteMarker {
                level_id,
                record_index,
                subtype: transition.marker_subtype,
            })?;
        let destination_level_id = transition
            .destination_global_level()
            .filter(|id| (13..=48).contains(id))
            .ok_or(WarpCatalogError::InvalidDestinationLevel {
                level_id,
                destination_logical_level: transition.destination_logical_level,
            })?;
        routes.push(AuthoredCampaignRoute {
            record_index,
            route: CampaignWarpRoute {
                source_level_id: level_id,
                marker_cell: marker.cell,
                marker_kind: marker.kind_index,
                marker_subtype: marker.subtype,
                marker_args: transition.marker_args,
                destination_logical_level: transition.destination_logical_level,
                destination_level_id,
                arrival: WarpArrival {
                    position_raw: transition.arrival_raw,
                    heading_raw: CAMPAIGN_ARRIVAL_HEADING_RAW,
                },
            },
        });
    }
    Ok(routes)
}

/// Collect every authored `0x16..=0x1A` terrain marker in retail X-major
/// scan order. The marker subtype selects campaign records; the terrain
/// descriptor and type-111 constructor never encode a destination.
pub fn collect_authored_warp_markers(
    terrain: &TerrainGrid,
    terrain_objects: &TerrainObjectTable,
) -> Result<Vec<AuthoredWarpMarker>, WarpCatalogError> {
    let mut markers = Vec::new();
    for x in 0u16..=255 {
        for z in 0u16..=255 {
            let cell = *terrain
                .cell(usize::from(x), usize::from(z))
                .expect("bounded terrain marker cell");
            if cell.attribute == 0 {
                continue;
            }
            let Some(descriptor) = terrain_objects.records.get(usize::from(cell.attribute)) else {
                return Err(WarpCatalogError::MissingDescriptor {
                    cell: [x as u8, z as u8],
                    attribute: cell.attribute,
                });
            };
            if !(FIRST_WARP_MARKER_KIND..=LAST_WARP_MARKER_KIND).contains(&descriptor.kind_index) {
                continue;
            }

            markers.push(AuthoredWarpMarker {
                cell: [x as u8, z as u8],
                attribute: cell.attribute,
                terrain_type: cell.terrain_type,
                kind_index: descriptor.kind_index,
                subtype: (descriptor.kind_index - FIRST_WARP_MARKER_KIND + 1) as u8,
                model_id: descriptor.model_id_for(cell.terrain_type),
                position_raw: marker_position_raw([x as u8, z as u8], cell.height),
            });
        }
    }
    Ok(markers)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlayerMarkerStamp {
    cell: [u8; 2],
    subtype: u8,
    terrain_states: [i8; 2],
    tick: u32,
}

/// Static-contact route custody. Native Type111 lifetime and sound belong to
/// the entity manager and its positional audio owner.
#[derive(Debug, Default)]
pub struct CampaignWarpRuntime {
    routes: Vec<AuthoredCampaignRoute>,
    marker_stamp: Option<PlayerMarkerStamp>,
}

/// The standalone route probe and the campaign selector have different scan owners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignRouteVisit {
    FirstMatching,
    Record(usize),
}

impl CampaignWarpRuntime {
    pub fn rebuild(
        &mut self,
        level_id: u32,
        level: &LevelDescriptor,
        terrain: &TerrainGrid,
        terrain_objects: &TerrainObjectTable,
    ) -> Result<(), WarpCatalogError> {
        self.routes = authored_campaign_routes(level_id, level, terrain, terrain_objects)?;
        self.marker_stamp = None;
        Ok(())
    }

    /// Forget physical handles after the caller has stopped the global mixer.
    pub fn reset_after_audio_stop(&mut self) {
        self.routes.clear();
        self.marker_stamp = None;
    }

    /// `427E20` mode2 calls `4464B0` before physical separation. The two
    /// selector words come from `427100` contact+18/+14 (terrain bits3/4).
    pub fn observe_player_static_contact(&mut self, contact: StaticModelContact, retail_tick: u32) {
        let Some(kind) = static_kind_descriptor(contact.kind_index) else {
            return;
        };
        if kind.contact_mode_raw() != 2 {
            return;
        }
        let Ok(subtype) = u8::try_from(kind.contact_argument_raw()) else {
            return;
        };
        self.marker_stamp = Some(PlayerMarkerStamp {
            cell: contact.cell,
            subtype,
            terrain_states: [
                ((contact.terrain_type >> 3) & 1) as i8,
                ((contact.terrain_type >> 4) & 1) as i8,
            ],
            tick: retail_tick,
        });
    }

    /// `446440` consumes the first matching stamp once. Failed selector tests
    /// retain it, and the unsigned wrapping age test is strictly `< 2`.
    pub fn poll_route(
        &mut self,
        progress: &mut PlayerCampaignProgress,
        retail_tick: u32,
        visit: CampaignRouteVisit,
    ) -> Option<CampaignWarpRoute> {
        let stamp = self.marker_stamp?;
        if retail_tick.wrapping_sub(stamp.tick) >= 2 {
            return None;
        }
        let authored = self.routes.iter().find(|entry| {
            (match visit {
                CampaignRouteVisit::FirstMatching => true,
                CampaignRouteVisit::Record(index) => entry.record_index == index,
            }) && entry.route.marker_subtype == stamp.subtype
                && entry
                    .route
                    .marker_args
                    .iter()
                    .zip(stamp.terrain_states)
                    .all(|(required, actual)| *required == 2 || *required == actual)
        })?;
        if !commit_route_progress(progress, *authored) {
            return None;
        }
        self.marker_stamp = None;
        Some(CampaignWarpRoute {
            marker_cell: stamp.cell,
            ..authored.route
        })
    }
}

fn commit_route_progress(
    progress: &mut PlayerCampaignProgress,
    authored: AuthoredCampaignRoute,
) -> bool {
    let Some(source_slot) = authored
        .route
        .source_level_id
        .checked_sub(12)
        .map(|slot| slot as usize)
    else {
        return false;
    };
    if progress.current_control_slot() != Some(source_slot) {
        return false;
    }
    let column = authored.record_index as u32 + 1;
    if column <= RETAIL_CONTROL_PAIR_LINK_COLUMNS {
        let _ = progress.set_pair_neighbor_bit(source_slot, column);
    }
    let _ = progress.set_exit_marker_bit(i32::from(authored.route.marker_subtype));
    true
}

#[cfg(test)]
fn marker_kind_for_subtype(subtype: u8) -> u32 {
    FIRST_WARP_MARKER_KIND + u32::from(subtype - 1)
}

fn marker_position_raw(cell: [u8; 2], height: u8) -> [i16; 3] {
    let x = ((u16::from(cell[0]) << 8) | 0x80) as i16;
    let z = ((u16::from(cell[1]) << 8) | 0x80) as i16;
    let y = (i16::from(height as i8) + 0x10) * 0x20;
    [x, y, z]
}

#[cfg(test)]
mod tests;
