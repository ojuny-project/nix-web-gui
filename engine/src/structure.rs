use nix_settings_gui::dsl::WidgetKind;

/// One child entry template for a composite widget. The returned text is the
/// attribute value, without the `key =` prefix for attribute sets.
pub fn child_template(
    widget: WidgetKind,
    mode: Option<(u32, u32, f64)>,
    label: Option<&str>,
) -> Result<String, String> {
    let (width, height, refresh) = mode.unwrap_or((1920, 1080, 60.0));
    let template = match widget {
        WidgetKind::Users => {
            "{ description = \"\"; isNormalUser = true; extraGroups = [ \"networkmanager\" ]; hashedPassword = null; }"
                .to_string()
        }
        WidgetKind::Keybinds => "{ modifier = [ ]; keys = [ ]; }".to_string(),
        WidgetKind::Monitors => format!(
            "{{ width = {width}; height = {height}; refresh = {refresh}; xPos = 0; yPos = 0; scale = 1.0; }}"
        ),
        WidgetKind::Docker => {
            "{ image = \"\"; ports = [ ]; volumes = [ ]; environment = { }; autoStart = true; }"
                .to_string()
        }
        WidgetKind::Menu => format!(
            "{{ label = {}; provider = \"apps\"; }}",
            crate::render::quote(label.unwrap_or("New entry"))
        ),
        WidgetKind::Plugins => "{ enable = true; }".to_string(),
        WidgetKind::Submodule
        | WidgetKind::HorizontalSubmodule
        | WidgetKind::GridConfiguration
        | WidgetKind::Bar
        | WidgetKind::Dock => "{ }".to_string(),
        other => {
            return Err(format!(
                "{} rows cannot be added from the web editor",
                crate::render::widget_name(other)
            ))
        }
    };
    Ok(template)
}

/// Render `users.alice` or a label containing unusual characters as a valid
/// Nix attribute path. Each segment is quoted unless it is a plain identifier.
pub fn attr_path(key: &str) -> Result<String, String> {
    let segments: Vec<&str> = key.split('.').collect();
    if segments.iter().any(|segment| segment.is_empty()) {
        return Err("row key has an empty segment".to_string());
    }
    Ok(segments
        .iter()
        .map(|segment| {
            if is_identifier(segment) {
                segment.to_string()
            } else {
                crate::render::quote(segment)
            }
        })
        .collect::<Vec<_>>()
        .join("."))
}

fn is_identifier(text: &str) -> bool {
    let mut characters = text.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    characters.all(|character| character.is_ascii_alphanumeric() || character == '_' || character == '-')
}

/// Outputs reported by the compositor, used to prefill the add-monitor row.
/// Returns an empty list when the compositor or `wlr-randr` is unavailable.
pub fn detect_monitors() -> serde_json::Value {
    let outputs: Vec<serde_json::Value> = nix_settings_gui::monitor_detect::detect()
        .unwrap_or_default()
        .iter()
        .map(|monitor| {
            let mode = monitor.default_mode();
            serde_json::json!({
                "name": monitor.name,
                "make": monitor.make,
                "model": monitor.model,
                "enabled": monitor.enabled,
                "width": mode.map(|mode| mode.width),
                "height": mode.map(|mode| mode.height),
                "refresh": mode.map(|mode| mode.refresh),
            })
        })
        .collect();
    serde_json::json!({ "outputs": outputs })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_users_and_keybind_templates() {
        assert!(child_template(WidgetKind::Users, None, None)
            .unwrap()
            .contains("isNormalUser = true"));
        assert!(child_template(WidgetKind::Keybinds, None, None)
            .unwrap()
            .contains("modifier = [ ]"));
    }

    #[test]
    fn quotes_unusual_keys() {
        assert_eq!(attr_path("alice").unwrap(), "alice");
        assert_eq!(attr_path("tools.htop").unwrap(), "tools.htop");
        assert_eq!(attr_path("my user").unwrap(), "\"my user\"");
        assert!(attr_path("a..b").is_err());
    }
}
