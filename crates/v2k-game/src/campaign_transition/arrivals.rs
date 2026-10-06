//! Route-specific player poses, distinct from the destination's authored actors.

use super::{WarpArrival, CAMPAIGN_ARRIVAL_HEADING_RAW, RETAIL_CONTROLLER_DEFAULT_ARRIVAL};
use crate::session::GameSession;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CampaignArrivalEntry {
    pub source_level_id: u32,
    pub record_index: usize,
    pub destination_level_id: u32,
    pub arrival: WarpArrival,
}

/// Direct OVL loading has no retail campaign source. Select the first incoming
/// authored record in source/record order, with the executable controller pose
/// retained explicitly for arenas (which have no campaign edges). New Game,
/// saved games and real campaign handoffs do not use this convenience policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectWorldEntry {
    AuthoredRoute(CampaignArrivalEntry),
    RetailControllerDefault(WarpArrival),
}

impl DirectWorldEntry {
    pub fn arrival(self) -> WarpArrival {
        match self {
            Self::AuthoredRoute(entry) => entry.arrival,
            Self::RetailControllerDefault(arrival) => arrival,
        }
    }
}

#[derive(Debug, Default)]
pub struct CampaignArrivalCatalog {
    entries: Vec<CampaignArrivalEntry>,
}

impl CampaignArrivalCatalog {
    /// Read every ordinary tier-selected world without replacing resident
    /// resources. Logical1..36 map to OVL13..48;49/50 are cinematics.
    pub fn load(session: &GameSession, variant: u32) -> Result<Self, String> {
        let mut entries = Vec::new();
        for source_level_id in 13..=48 {
            let state = session
                .load_ovl_by_id(source_level_id, variant)
                .map_err(|error| format!("campaign OVL {source_level_id}: {error}"))?;
            let level = state
                .level
                .ok_or_else(|| format!("campaign OVL {source_level_id}: missing Section13"))?;
            for (record_index, record) in level.campaign_records.iter().enumerate() {
                let flags = record.flags() & 0xff;
                if flags & 0x80 != 0 {
                    continue;
                }
                if flags != 0x10 {
                    return Err(format!("campaign OVL {source_level_id} record {record_index}: unsupported normal flags {flags:#x}"));
                }
                let transition = record.marker_transition().expect("marker flag");
                let destination_level_id = transition.destination_global_level()
                    .filter(|id| (13..=48).contains(id))
                    .ok_or_else(|| format!("campaign OVL {source_level_id} record {record_index}: invalid logical destination {}", transition.destination_logical_level))?;
                entries.push(CampaignArrivalEntry {
                    source_level_id,
                    record_index,
                    destination_level_id,
                    arrival: WarpArrival {
                        position_raw: transition.arrival_raw,
                        heading_raw: CAMPAIGN_ARRIVAL_HEADING_RAW,
                    },
                });
            }
        }
        Ok(Self { entries })
    }

    pub fn entries(&self) -> &[CampaignArrivalEntry] {
        &self.entries
    }

    pub fn direct_world_entry(&self, destination_level_id: u32) -> Option<DirectWorldEntry> {
        if !(13..=48).contains(&destination_level_id) {
            return None;
        }
        Some(
            match self
                .entries
                .iter()
                .find(|entry| entry.destination_level_id == destination_level_id)
            {
                Some(entry) => DirectWorldEntry::AuthoredRoute(*entry),
                None => {
                    DirectWorldEntry::RetailControllerDefault(RETAIL_CONTROLLER_DEFAULT_ARRIVAL)
                }
            },
        )
    }
}
