use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The shared settings GUI config. The web worker uses the same file as the
/// desktop app so rebuild commands and runtime preferences stay in one place.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    #[serde(default)]
    pub flake_path: Option<String>,
    #[serde(default)]
    pub terminal_command: Option<String>,
    #[serde(default)]
    pub rebuild_command: Option<String>,
    #[serde(default)]
    pub recent_flake_paths: Vec<String>,
    #[serde(default = "default_true")]
    pub live_apply: bool,
    #[serde(default)]
    pub reload_mango_on_rebuild: bool,
    #[serde(default)]
    pub reload_dms_on_rebuild: bool,
    #[serde(default)]
    pub expand_details_by_default: bool,
    #[serde(default)]
    pub auto_git_sync: bool,
}

fn default_true() -> bool {
    true
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            flake_path: None,
            terminal_command: None,
            rebuild_command: None,
            recent_flake_paths: Vec::new(),
            live_apply: true,
            reload_mango_on_rebuild: false,
            reload_dms_on_rebuild: false,
            expand_details_by_default: false,
            auto_git_sync: false,
        }
    }
}

impl WebConfig {
    pub fn path() -> PathBuf {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home().join(".config"));
        base.join("nix-settings-gui").join("config.json")
    }

    pub fn load() -> Self {
        let mut config: Self = std::fs::read_to_string(Self::path())
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        if config.flake_path.is_none() {
            config.flake_path = system_flake_path();
        }
        config
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create config dir: {error}"))?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|error| error.to_string())?;
        std::fs::write(&path, json).map_err(|error| format!("cannot write config: {error}"))
    }

    pub fn remember(&mut self, flake: &str) {
        self.flake_path = Some(flake.to_string());
        self.recent_flake_paths.retain(|path| path != flake);
        self.recent_flake_paths.insert(0, flake.to_string());
        self.recent_flake_paths.truncate(10);
    }
}

fn system_flake_path() -> Option<String> {
    let text = std::fs::read_to_string("/etc/ojuny/flake-path").ok()?;
    let path = text.trim();
    if path.is_empty() || !PathBuf::from(path).join("flake.nix").is_file() {
        return None;
    }
    Some(path.to_string())
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_enable_live_apply() {
        let config = WebConfig::default();
        assert!(config.live_apply);
        assert!(!config.auto_git_sync);
    }

    #[test]
    fn missing_auto_git_sync_deserializes_to_off() {
        let config: WebConfig = serde_json::from_str("{}").unwrap();
        assert!(!config.auto_git_sync);
    }

    #[test]
    fn remembers_unique_recents() {
        let mut config = WebConfig::default();
        config.remember("/tmp/a");
        config.remember("/tmp/b");
        config.remember("/tmp/a");
        assert_eq!(config.recent_flake_paths, vec!["/tmp/a", "/tmp/b"]);
    }
}
