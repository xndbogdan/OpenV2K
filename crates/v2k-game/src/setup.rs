//! Pre-SDL installation discovery, retail-file validation and disc installation.
//!
//! This owner reads original assets without changing their layout. A successful
//! structural check admits the complete retail corpus; it does not assert that
//! every decoded actor has an implemented gameplay program.

mod disc;
mod install;
mod validation;

pub use disc::{DiscImage, DiscSummary};
pub use install::{install_launcher, InstallRequest, InstallResult, SetupProgress};
pub use validation::{validate_installation, AssetIssue, IssueSeverity, ValidationReport};

use std::path::{Path, PathBuf};

pub type SetupResult<T> = Result<T, String>;
pub const MANIFEST_FILENAME: &str = "v2000-installation.json";

#[derive(Debug, Clone, Default)]
pub struct DiscoveryRequest {
    pub executable: PathBuf,
    pub explicit: Option<PathBuf>,
    /// Setup hints only: they must not make an unrelated executable folder ready.
    pub remembered: Option<PathBuf>,
    /// Registry and working-directory hints for the setup browser.
    pub registry_candidates: Vec<PathBuf>,
}

/// Default startup validates the executable's own directory, including a wholly
/// empty directory. Saved/registry hints never substitute a remote installation.
/// Explicit data roots remain available for the diagnostic command-line path.
pub fn discover_installation(request: &DiscoveryRequest) -> Option<ValidationReport> {
    if let Some(root) = &request.explicit {
        return Some(validate_installation(root));
    }
    request.executable.parent().map(validate_installation)
}

/// A verified browser default for choosing where to install the portable copy.
/// This hint never authorizes Play from the current executable's own folder.
pub fn discover_setup_hint(request: &DiscoveryRequest) -> Option<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    for candidate in request
        .explicit
        .iter()
        .chain(request.remembered.iter())
        .chain(request.registry_candidates.iter())
        .take(64)
    {
        let Ok(root) = std::fs::canonicalize(candidate) else {
            continue;
        };
        if seen.insert(root.clone()) && validate_installation(&root).is_ready() {
            return Some(root);
        }
    }
    None
}

pub(crate) fn find_child(parent: &Path, name: &str) -> Option<PathBuf> {
    std::fs::read_dir(parent)
        .ok()?
        .filter_map(Result::ok)
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(name)
        })
        .map(|entry| entry.path())
}

pub fn expected_overlay_names() -> impl Iterator<Item = String> {
    (0..4).flat_map(|tier| (0..53).map(move |level| format!("{tier}X{level}XX.OVL")))
}
