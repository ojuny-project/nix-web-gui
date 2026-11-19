use nix_settings_gui::dsl::{OptionSpec, WidgetKind};

/// One child entry template for a composite widget. The returned text is the
/// attribute value, without the `key =` prefix for attribute sets.
pub fn child_template(
    widget: WidgetKind,
    key: &str,
    mode: Option<(u32, u32, f64)>,
    label: Option<&str>,
    spec: &OptionSpec,
) -> Result<String, String> {
    let (width, height, refresh) = mode.unwrap_or((1920, 1080, 60.0));
    let template = match widget {
        WidgetKind::Users => {
            "{ description = \"\"; isNormalUser = true; extraGroups = [ \"networkmanager\" ]; hashedPassword = null; }"
                .to_string()
        }
        WidgetKind::Keybinds => {
            let key_field = spec.bind_key_field.as_deref();
            let fields = spec
                .bind_fields
                .clone()
                .unwrap_or_else(|| vec!["modifier".to_string(), "keys".to_string()]);
            let types = spec.bind_field_types.clone().unwrap_or_default();
            let mut parts = Vec::new();
            if let Some(key_field) = key_field {
                parts.push(format!("{key_field} = {};", crate::render::quote(key.trim())));
            }
            for (index, field) in fields.iter().enumerate() {
                if Some(field.as_str()) == key_field {
                    continue;
                }
                let kind = types
                    .get(index)
                    .map(|kind| kind.as_str())
                    .unwrap_or(if field == "modifier" || field == "keys" {
                        "list"
                    } else {
                        "str"
                    });
                let literal = match kind.to_ascii_lowercase().as_str() {
                    "bool" | "boolean" => "false",
                    "int" | "integer" => "0",
                    "float" | "number" => "0.0",
                    "list" | "string-list" | "strings" => "[ ]",
                    _ => "\"\"",
                };
                parts.push(format!("{field} = {literal};"));
            }
            format!("{{ {} }}", parts.join(" "))
        }
        WidgetKind::Monitors => format!(
            "{{ width = {width}; height = {height}; refresh = {refresh}; xPos = 0; yPos = 0; scale = 1.0; }}"
        ),
        WidgetKind::Docker => {
            let field = |value: &Option<String>, default: &str| -> Option<String> {
                match value.as_deref() {
                    Some("") => None,
                    Some(name) => Some(name.to_string()),
                    None => Some(default.to_string()),
                }
            };
            let default = spec.container_auto_start_default.unwrap_or(true);
            let mut parts = Vec::new();
            if let Some(name) = field(&spec.container_image, "image") {
                parts.push(format!("{name} = \"\";"));
            }
            if let Some(name) = field(&spec.container_ports, "ports") {
                parts.push(format!("{name} = [ ];"));
            }
            if let Some(name) = field(&spec.container_volumes, "volumes") {
                parts.push(format!("{name} = [ ];"));
            }
            if let Some(name) = field(&spec.container_environment, "environment") {
                parts.push(format!("{name} = {{ }};"));
            }
            if let Some(name) = field(&spec.container_auto_start, "autoStart") {
                parts.push(format!("{name} = {default};"));
            }
            format!("{{ {} }}", parts.join(" "))
        }
        WidgetKind::Menu => format!(
            "{{ label = {}; }}",
            crate::render::quote(label.unwrap_or("New entry"))
        ),
        WidgetKind::Plugins => "{ enable = true; }".to_string(),
        WidgetKind::List => {
            if key.trim().is_empty() {
                "{ }".to_string()
            } else {
                format!("{{ id = {}; }}", crate::render::quote(key.trim()))
            }
        }
        WidgetKind::GridConfiguration => {
            if spec.grid_enabled {
                "{ enabled = true; }".to_string()
            } else {
                "{ }".to_string()
            }
        }
        WidgetKind::Submodule
        | WidgetKind::HorizontalSubmodule
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
        assert!(child_template(WidgetKind::Users, "", None, None, &OptionSpec::default())
            .unwrap()
            .contains("isNormalUser = true"));
        assert!(child_template(WidgetKind::Keybinds, "", None, None, &OptionSpec::default())
            .unwrap()
            .contains("modifier = [ ]"));
    }

    #[test]
    fn follows_configurable_bind_and_container_schemas() {
        let spec = nix_settings_gui::dsl::OptionSpec {
            bind_fields: Some(vec![
                "name".to_string(),
                "keys".to_string(),
                "action".to_string(),
            ]),
            bind_field_types: Some(vec![
                "str".to_string(),
                "list".to_string(),
                "str".to_string(),
            ]),
            bind_key_field: Some("name".to_string()),
            container_image: Some("imageRef".to_string()),
            container_ports: Some("expose".to_string()),
            container_volumes: Some("mounts".to_string()),
            container_environment: Some("env".to_string()),
            container_auto_start: Some("startOnBoot".to_string()),
            container_auto_start_default: Some(false),
            ..nix_settings_gui::dsl::OptionSpec::default()
        };
        let bind = child_template(WidgetKind::Keybinds, "terminal", None, None, &spec).unwrap();
        assert!(bind.contains("name = \"terminal\";"), "{bind}");
        assert!(bind.contains("keys = [ ];"), "{bind}");
        assert!(bind.contains("action = \"\";"), "{bind}");

        let container = child_template(WidgetKind::Docker, "web", None, None, &spec).unwrap();
        assert!(container.contains("imageRef = \"\";"), "{container}");
        assert!(container.contains("expose = [ ];"), "{container}");
        assert!(container.contains("startOnBoot = false;"), "{container}");
    }

    #[test]
    fn quotes_unusual_keys() {
        assert_eq!(attr_path("alice").unwrap(), "alice");
        assert_eq!(attr_path("tools.htop").unwrap(), "tools.htop");
        assert_eq!(attr_path("my user").unwrap(), "\"my user\"");
        assert!(attr_path("a..b").is_err());
    }
}
