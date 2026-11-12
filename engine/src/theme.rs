use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use serde::Serialize;
use tokio::sync::broadcast;

pub const DEFAULT_THEME: &str = "everforest-dark-hard";

#[derive(Clone, Debug, Serialize)]
pub struct ThemeState {
    pub name: String,
    pub source: String,
    pub version: u64,
}

pub struct ThemeStore {
    path: PathBuf,
    state: RwLock<ThemeState>,
    tx: broadcast::Sender<ThemeState>,
}

impl ThemeStore {
    pub fn new(path: PathBuf) -> Arc<Self> {
        let name = read_name(&path);
        let source = path.display().to_string();
        let (tx, _) = broadcast::channel(16);
        Arc::new(Self {
            path,
            state: RwLock::new(ThemeState {
                name,
                source,
                version: 1,
            }),
            tx,
        })
    }

    pub fn current(&self) -> ThemeState {
        self.state
            .read()
            .expect("theme store poisoned")
            .clone()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ThemeState> {
        self.tx.subscribe()
    }

    pub fn refresh(&self) -> bool {
        let name = read_name(&self.path);
        let mut state = self.state.write().expect("theme store poisoned");
        if state.name == name {
            return false;
        }
        state.name = name;
        state.version += 1;
        let _ = self.tx.send(state.clone());
        true
    }
}

pub fn read_name(path: &Path) -> String {
    std::fs::read_to_string(path)
        .ok()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| DEFAULT_THEME.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falls_back_to_the_default_theme() {
        assert_eq!(read_name(Path::new("/tmp/nix-web-gui-no-theme")), DEFAULT_THEME);
    }

    #[test]
    fn refresh_emits_only_when_the_name_changes() {
        let path = std::env::temp_dir().join(format!(
            "nix-web-gui-theme-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::write(&path, "catppuccin-mocha\n").unwrap();
        let store = ThemeStore::new(path.clone());
        assert_eq!(store.current().name, "catppuccin-mocha");
        assert!(!store.refresh());
        std::fs::write(&path, "everforest-dark-hard\n").unwrap();
        assert!(store.refresh());
        assert_eq!(store.current().name, "everforest-dark-hard");
        let _ = std::fs::remove_file(path);
    }
}
