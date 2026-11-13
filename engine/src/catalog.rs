use nix_settings_gui::widgets::style_catalog;
use serde_json::{json, Value};

pub fn kind_for_widget(widget: &str) -> Option<&'static str> {
    match widget {
        "font" => Some("fonts"),
        "icontheme" | "icon-theme" => Some("icons"),
        "cursortheme" | "cursor-theme" => Some("cursors"),
        _ => None,
    }
}

pub fn package_for(kind: &str, value: &str) -> Option<&'static str> {
    let entries = match kind {
        "fonts" => style_catalog::fonts(),
        "icons" => style_catalog::icon_themes(),
        "cursors" => style_catalog::cursor_themes(),
        _ => return None,
    };
    entries
        .iter()
        .find(|entry| entry.value == value)
        .map(|entry| entry.package)
}

pub fn styles(kind: &str) -> Result<Value, String> {
    let entries = match kind {
        "fonts" => style_catalog::fonts(),
        "icons" => style_catalog::icon_themes(),
        "cursors" => style_catalog::cursor_themes(),
        other => return Err(format!("unknown style catalog: {other}")),
    };
    Ok(Value::Array(
        entries
            .iter()
            .map(|entry| {
                json!({
                    "label": entry.label,
                    "value": entry.value,
                    "package": entry.package,
                    "monospace": entry.monospace,
                    "variant": entry.variant,
                })
            })
            .collect(),
    ))
}
