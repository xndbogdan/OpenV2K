//! Save placement belongs to the running port executable, independently of
//! game-data discovery, working directory, and retail registry import hints.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use super::NUM_SAVE_SLOTS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SaveStoragePolicy {
    /// Historical layout retained by save fixtures and compatibility callers.
    LegacyDataDirectory,
    /// Interactive portable runtime: both native and JSON files beside EXE.
    BesideExecutable,
}

/// Explicit writable location plus read-only legacy/retail import roots.
#[derive(Debug, Clone)]
pub struct SavePathContext {
    pub(super) save_directory: PathBuf,
    pub(super) data_directory: PathBuf,
    pub(super) retail_save_directory: Option<PathBuf>,
    pub(super) storage_policy: SaveStoragePolicy,
}

impl SavePathContext {
    /// Use the actual executable's parent for new save files. The executable
    /// must be absolute, as returned by `std::env::current_exe`, so a relative
    /// working directory can never silently choose the writable location.
    /// Data and registry directories remain read-only import sources.
    pub fn beside_executable(
        executable: &Path,
        data_root: &Path,
        retail_save_directory: Option<&Path>,
    ) -> Result<Self, String> {
        let parent = executable
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty());
        if !executable.is_absolute() || parent.is_none() {
            return Err("Cannot locate save directory beside an absolute executable path".into());
        }
        Ok(Self {
            save_directory: parent.expect("checked executable parent").to_path_buf(),
            data_directory: data_root.to_path_buf(),
            retail_save_directory: retail_save_directory.map(Path::to_path_buf),
            storage_policy: SaveStoragePolicy::BesideExecutable,
        })
    }

    pub fn save_directory(&self) -> &Path {
        &self.save_directory
    }

    pub(super) fn legacy_layout(data_root: &Path, retail_save_directory: Option<&Path>) -> Self {
        Self {
            save_directory: data_root.join("saves"),
            data_directory: data_root.to_path_buf(),
            retail_save_directory: retail_save_directory.map(Path::to_path_buf),
            storage_policy: SaveStoragePolicy::LegacyDataDirectory,
        }
    }

    pub(super) fn native_slot_destination(&self, slot: usize) -> PathBuf {
        if self.storage_policy == SaveStoragePolicy::BesideExecutable {
            return self.save_directory.join(format!("Slot{slot:02}"));
        }
        let roots = [self.data_directory.clone(), self.data_directory.join("..")];
        for root in &roots {
            let path = root.join(format!("Slot{slot:02}"));
            if save_path_is_present(&path) {
                return path;
            }
        }
        let root = roots
            .iter()
            .find(|root| {
                (0..NUM_SAVE_SLOTS)
                    .any(|index| save_path_is_present(&root.join(format!("Slot{index:02}"))))
            })
            .unwrap_or(&self.data_directory);
        root.join(format!("Slot{slot:02}"))
    }

    pub(super) fn checkpoint_shadow_paths(&self, slot: usize) -> Vec<PathBuf> {
        let mut paths = Vec::with_capacity(2);
        if self.storage_policy == SaveStoragePolicy::LegacyDataDirectory {
            paths.push(self.save_directory.join(format!("Slot{slot:02}")));
        }
        paths.push(self.save_directory.join(format!("slot_{slot}.json")));
        paths
    }
}

fn save_path_is_present(path: &Path) -> bool {
    // An unreadable destination must fail in place, never select another root.
    !matches!(fs::metadata(path), Err(error) if error.kind() == ErrorKind::NotFound)
}
