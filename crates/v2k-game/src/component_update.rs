//! Shared detailed/coarse component-update mode selected by `FUN_00412DA0`.
//!
//! This is scheduler state, not an Alien-Hive-emitter policy. Keeping it in a
//! neutral module lets independently recovered component callbacks share the
//! proven data shape without coupling their behavior.

/// Which common entity-update path reached a retained component.
///
/// `FUN_00412DA0` selects the detailed `FUN_0040DCA0` path when live state bit
/// `0x02000000` is set and the coarse `FUN_0040E870` path otherwise. Component
/// callbacks receive those paths as integer modes zero and one respectively;
/// callbacks which branch on zero/nonzero should match the enum rather than
/// inventing additional numeric modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentUpdateMode {
    Detailed,
    Coarse,
}

impl ComponentUpdateMode {
    pub const fn retail_value(self) -> u32 {
        match self {
            Self::Detailed => 0,
            Self::Coarse => 1,
        }
    }
}
