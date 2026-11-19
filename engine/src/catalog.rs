use base64::Engine as _;
use nix_settings_gui::widgets::style_catalog;
use serde_json::{json, Value};

fn preview_data(bytes: &'static [u8]) -> String {
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

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
                    "preview": entry.preview.map(preview_data),
                    "previews": entry.previews.iter().map(|bytes| preview_data(bytes)).collect::<Vec<_>>(),
                })
            })
            .collect(),
    ))
}

pub fn menu_providers(top_level: bool) -> Value {
    nix_settings_gui::headless::menu_providers(top_level)
}

pub fn menu_field_help(name: &str) -> Option<&'static str> {
    nix_settings_gui::headless::menu_field_help(name)
}
