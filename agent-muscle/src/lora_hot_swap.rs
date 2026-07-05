//! Hot-swap LoRA adapter paths without restarting muscle serve (Phase 9 P1).

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Default)]
pub struct LoraAdapterRegistry {
    active: Arc<RwLock<Option<PathBuf>>>,
    staging: Arc<RwLock<Option<PathBuf>>>,
}

impl LoraAdapterRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stage(&self, path: PathBuf) {
        if let Ok(mut guard) = self.staging.write() {
            *guard = Some(path);
        }
    }

    pub fn promote_staging(&self) -> Option<PathBuf> {
        let next = self.staging.read().ok()?.clone()?;
        if !next.exists() {
            return None;
        }
        if let Ok(mut active) = self.active.write() {
            *active = Some(next.clone());
        }
        if let Ok(mut staging) = self.staging.write() {
            *staging = None;
        }
        Some(next)
    }

    pub fn active_path(&self) -> Option<PathBuf> {
        self.active.read().ok()?.clone()
    }

    pub fn hot_swap(&self, path: impl AsRef<Path>) -> bool {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            return false;
        }
        self.stage(path);
        self.promote_staging().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn hot_swap_promotes_existing_adapter() {
        let dir = std::env::temp_dir().join(format!("lora-hot-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let adapter = dir.join("adapter.safetensors");
        fs::write(&adapter, b"stub").unwrap();
        let reg = LoraAdapterRegistry::new();
        assert!(reg.hot_swap(&adapter));
        assert_eq!(reg.active_path(), Some(adapter));
        let _ = fs::remove_dir_all(dir);
    }
}
