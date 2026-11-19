use nix_settings_gui::metadata;
use serde_json::{json, Value};

/// Plugin registry metadata for the config flake, when a desktop shell input
/// declares one. An empty or failed registry is not fatal for the editor.
pub fn registry(root: &str) -> Value {
    match metadata::dms_plugins(root) {
        Ok(plugins) => json!({
            "plugins": plugins.iter().map(|plugin| json!({
                "id": plugin.id,
                "name": plugin.name,
                "description": plugin.description,
                "category": plugin.category,
                "repo": plugin.repo,
                "settings": plugin.settings,
            })).collect::<Vec<_>>(),
            "error": Value::Null,
        }),
        Err(error) => json!({ "plugins": [], "error": error }),
    }
}

/// The declarative settings QML of one registry plugin, when it ships one.
pub fn settings(root: &str, plugin_id: &str) -> Result<Value, String> {
    metadata::dms_plugin_settings(root, plugin_id)
        .map(|qml| json!({ "qml": qml }))
}

/// Parsed settings controls for one registry plugin.
#[cfg(feature = "ojuny-widgets")]
pub fn schema(root: &str, plugin_id: &str) -> Result<Value, String> {
    let qml = metadata::dms_plugin_settings(root, plugin_id)?;
    Ok(qml
        .as_deref()
        .map(nix_settings_gui::headless::plugin_settings_schema)
        .unwrap_or_else(|| Value::Array(Vec::new())))
}

#[cfg(not(feature = "ojuny-widgets"))]
pub fn schema(_root: &str, _plugin_id: &str) -> Result<Value, String> {
    Err("plugin settings need the Ojuny widgets feature".to_string())
}
