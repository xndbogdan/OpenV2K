//! Prepared high-tier System2/3 presentation refresh.
//!
//! The three high overlays share intrinsic assets. Only their Section0/1
//! tables are replaced; raw load receipts authenticate every other section.
//! Low art is a different intrinsic source and cannot enter this transaction.

use std::{fmt, path::PathBuf, sync::Arc};

use v2k_formats::{ovl::OvlFile, system::FixupTable};

use crate::resource_cache::ResourceCache;

/// The authored high layouts; variant0 remains the explicit low-art mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighSystemLayoutTier {
    High640,
    High800,
    High1024,
}

impl HighSystemLayoutTier {
    pub const fn variant(self) -> u32 {
        match self {
            Self::High640 => 1,
            Self::High800 => 2,
            Self::High1024 => 3,
        }
    }

    pub const fn size(self) -> (u32, u32) {
        match self {
            Self::High640 => (640, 480),
            Self::High800 => (800, 600),
            Self::High1024 => (1024, 768),
        }
    }

    pub const fn from_variant(variant: u32) -> Option<Self> {
        match variant {
            1 => Some(Self::High640),
            2 => Some(Self::High800),
            3 => Some(Self::High1024),
            _ => None,
        }
    }
}

/// Read authored words from either a resident cache or an uncommitted stage.
pub trait SystemLayoutSource {
    fn system_data_value(&self, level: u32, index: usize) -> Option<u32>;
    fn system_layout_point(&self, level: u32, index: usize) -> Option<(i16, i16)>;
}

impl SystemLayoutSource for ResourceCache {
    fn system_data_value(&self, level: u32, index: usize) -> Option<u32> {
        ResourceCache::system_data_value(self, level, index)
    }

    fn system_layout_point(&self, level: u32, index: usize) -> Option<(i16, i16)> {
        ResourceCache::system_layout_point(self, level, index)
    }
}

/// Current layout source, separate from LevelState's intrinsic source_path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemLayoutOrigin {
    pub tier: HighSystemLayoutTier,
    pub source_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemLayoutOrigins {
    pub system2: SystemLayoutOrigin,
    pub system3: SystemLayoutOrigin,
}

#[derive(Debug)]
pub enum SystemLayoutRefreshError {
    ResidentUnavailable {
        system_level: u32,
    },
    ResidentSourceUnproven {
        system_level: u32,
    },
    Read {
        system_level: u32,
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        system_level: u32,
        path: PathBuf,
        source: v2k_core::V2kError,
    },
    IntrinsicSectionChanged {
        system_level: u32,
        section: usize,
    },
    InvalidTable {
        system_level: u32,
        section: usize,
    },
    ViewportMismatch {
        tier: HighSystemLayoutTier,
    },
    InvariantScalarChanged {
        index: usize,
    },
    StaleStage {
        system_level: u32,
    },
}

impl fmt::Display for SystemLayoutRefreshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResidentUnavailable { system_level } => {
                write!(f, "System{system_level} is not resident")
            }
            Self::ResidentSourceUnproven { system_level } => write!(
                f,
                "System{system_level} has no authenticated high intrinsic source"
            ),
            Self::Read {
                system_level,
                path,
                source,
            } => write!(
                f,
                "System{system_level} layout read {}: {source}",
                path.display()
            ),
            Self::Parse {
                system_level,
                path,
                source,
            } => write!(
                f,
                "System{system_level} layout parse {}: {source}",
                path.display()
            ),
            Self::IntrinsicSectionChanged {
                system_level,
                section,
            } => write!(
                f,
                "System{system_level} Section{section} differs from loaded intrinsic bytes"
            ),
            Self::InvalidTable {
                system_level,
                section,
            } => write!(
                f,
                "System{system_level} Section{section} has an invalid layout table shape"
            ),
            Self::ViewportMismatch { tier } => {
                write!(f, "System2 viewport does not match {tier:?}")
            }
            Self::InvariantScalarChanged { index } => {
                write!(f, "System3 invariant scalar{index} changed")
            }
            Self::StaleStage { system_level } => {
                write!(f, "System{system_level} changed after layout preparation")
            }
        }
    }
}

impl std::error::Error for SystemLayoutRefreshError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Exact raw headers and payloads, retained once per loaded high system layer.
/// No decoded asset is cloned and a refresh never adds a cache layer.
#[derive(Debug)]
pub(crate) struct IntrinsicSectionReceipt {
    header_value: u32,
    data: Box<[u8]>,
}

#[derive(Debug)]
pub(crate) struct HighSystemLayerReceipt {
    pub intrinsic: Arc<[IntrinsicSectionReceipt]>,
    pub origin: SystemLayoutOrigin,
}

impl HighSystemLayerReceipt {
    pub(crate) fn from_loaded_ovl(
        ovl: &OvlFile,
        tier: HighSystemLayoutTier,
        source_path: PathBuf,
    ) -> Self {
        Self {
            intrinsic: ovl.sections[2..]
                .iter()
                .map(|section| IntrinsicSectionReceipt {
                    header_value: section.header_value,
                    data: section.data.clone().into_boxed_slice(),
                })
                .collect(),
            origin: SystemLayoutOrigin { tier, source_path },
        }
    }
}

#[derive(Debug)]
pub(crate) struct PreparedSystemLayout {
    pub system_level: u32,
    pub data: FixupTable,
    pub points: FixupTable,
    pub origin: SystemLayoutOrigin,
    pub expected_intrinsic: Arc<[IntrinsicSectionReceipt]>,
    pub expected_origin: SystemLayoutOrigin,
    pub expected_data: Vec<u32>,
    pub expected_points: Vec<u32>,
}

/// A complete read-only System2/3 candidate. Build dependent presentation
/// snapshots through SystemLayoutSource before committing this value.
#[derive(Debug)]
pub struct PreparedHighSystemLayouts {
    pub(crate) systems: [PreparedSystemLayout; 2],
    pub(crate) origins: SystemLayoutOrigins,
}

impl PreparedHighSystemLayouts {
    pub fn tier(&self) -> HighSystemLayoutTier {
        self.origins.system2.tier
    }

    pub fn origins(&self) -> &SystemLayoutOrigins {
        &self.origins
    }
}

impl SystemLayoutSource for PreparedHighSystemLayouts {
    fn system_data_value(&self, level: u32, index: usize) -> Option<u32> {
        self.systems
            .iter()
            .find(|system| system.system_level == level)?
            .data
            .entries
            .get(index)
            .copied()
    }

    fn system_layout_point(&self, level: u32, index: usize) -> Option<(i16, i16)> {
        let raw = *self
            .systems
            .iter()
            .find(|system| system.system_level == level)?
            .points
            .entries
            .get(index)?;
        Some((raw as u16 as i16, (raw >> 16) as u16 as i16))
    }
}

pub(crate) fn prepare_system_layout(
    system_level: u32,
    ovl: &OvlFile,
    origin: SystemLayoutOrigin,
    receipt: &HighSystemLayerReceipt,
    resident_data: &FixupTable,
    resident_points: &FixupTable,
) -> Result<PreparedSystemLayout, SystemLayoutRefreshError> {
    use SystemLayoutRefreshError as Error;
    for (index, expected) in receipt.intrinsic.iter().enumerate() {
        let section = &ovl.sections[index + 2];
        if section.header_value != expected.header_value
            || section.data.as_slice() != &*expected.data
        {
            return Err(Error::IntrinsicSectionChanged {
                system_level,
                section: index + 2,
            });
        }
    }
    // Fixed retail high sources: System2 has six lens/scalar words and13
    // points; System3 has18 mixed scalars and35 points. parse_fixup_table
    // alone silently drops trailing bytes, so validate headers/shape first.
    let lengths = if system_level == 2 { [6, 13] } else { [18, 35] };
    for (section, words) in lengths.into_iter().enumerate() {
        let raw = &ovl.sections[section];
        if raw.data.len() != words * 4 || raw.header_value as usize != raw.data.len() {
            return Err(Error::InvalidTable {
                system_level,
                section,
            });
        }
    }
    let data = v2k_formats::sections::parse_fixup_data(ovl).unwrap();
    let points = v2k_formats::sections::parse_fixup_code(ovl).unwrap();
    if system_level == 2 {
        if (data.entries[2], data.entries[3]) != origin.tier.size() {
            return Err(Error::ViewportMismatch { tier: origin.tier });
        }
    } else {
        // Scalars0..5 feed live terrain-radar generation, not presentation.
        // The other listed words are also invariant in all three high files.
        for index in [0, 1, 2, 3, 4, 5, 10, 13, 14, 15, 16, 17] {
            if resident_data.entries.get(index) != data.entries.get(index) {
                return Err(Error::InvariantScalarChanged { index });
            }
        }
    }
    Ok(PreparedSystemLayout {
        system_level,
        data,
        points,
        origin,
        expected_intrinsic: Arc::clone(&receipt.intrinsic),
        expected_origin: receipt.origin.clone(),
        expected_data: resident_data.entries.clone(),
        expected_points: resident_points.entries.clone(),
    })
}
