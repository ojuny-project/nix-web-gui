use nix_settings_gui::dsl::{OptionSpec, WidgetKind};
use nix_settings_gui::model::Record;
use nix_settings_gui::value::{self, ValueKind};
use serde_json::Value;

pub fn widget_name(widget: WidgetKind) -> String {
    let raw = format!("{widget:?}");
    let mut out = String::with_capacity(raw.len() + 4);
    for (index, character) in raw.chars().enumerate() {
        if character.is_ascii_uppercase() {
            if index > 0 {
                out.push('-');
            }
            out.push(character.to_ascii_lowercase());
        } else {
            out.push(character);
        }
    }
    out
}

pub fn parse_widget(name: &str) -> Option<WidgetKind> {
    WidgetKind::parse(name)
}

pub fn choose_widget(record: &Record, type_str: Option<&str>) -> WidgetKind {
    let spec = &record.spec;
    if let Some(widget) = spec.widget {
        if widget == WidgetKind::Spin && spec.min.is_some() && spec.max.is_some() && spec.horizontal
        {
            return WidgetKind::Slider;
        }
        if matches!(widget, WidgetKind::List | WidgetKind::Packages)
            && value::list_items(&record.value).is_none()
        {
            return WidgetKind::Raw;
        }
        return widget;
    }
    if let Some(widget) = import_widget_kind(record) {
        return widget;
    }
    if let Some(widget) = type_str.and_then(widget_from_type) {
        return widget;
    }
    widget_from_kind(value::classify(&record.value))
}

fn import_widget_kind(record: &Record) -> Option<WidgetKind> {
    if record.segments.len() != 1 || record.segments[0] != "imports" {
        return None;
    }
    if record.origin.starts_with("modules.nixos.") {
        Some(WidgetKind::NixosModules)
    } else if record.origin.starts_with("modules.homeManager.") {
        Some(WidgetKind::HmModules)
    } else {
        None
    }
}

pub fn widget_from_type(type_str: &str) -> Option<WidgetKind> {
    let t = type_str.to_lowercase();
    if t.contains("one of") {
        Some(WidgetKind::Dropdown)
    } else if t.contains("list") && t.contains("package") {
        Some(WidgetKind::Packages)
    } else if t.contains("list") {
        Some(WidgetKind::List)
    } else if t.contains("package") {
        Some(WidgetKind::Package)
    } else if t.contains("path") {
        Some(WidgetKind::Path)
    } else if t.contains("bool") {
        Some(WidgetKind::Switch)
    } else if t.contains("float") || t.contains("number") || t.contains("int") {
        Some(WidgetKind::Spin)
    } else if t.contains("submodule") || t.contains("attribute set") || t.contains("attrs") {
        Some(WidgetKind::Submodule)
    } else if t.contains("string") {
        Some(WidgetKind::Text)
    } else {
        None
    }
}

fn widget_from_kind(kind: ValueKind) -> WidgetKind {
    match kind {
        ValueKind::Bool => WidgetKind::Switch,
        ValueKind::Int | ValueKind::Float => WidgetKind::Spin,
        ValueKind::String => WidgetKind::Text,
        ValueKind::List => WidgetKind::List,
        ValueKind::AttrSet => WidgetKind::Submodule,
        _ => WidgetKind::Raw,
    }
}

pub fn resolve_choices(spec: &OptionSpec, type_str: Option<&str>) -> Vec<String> {
    if let Some(choices) = &spec.choices {
        return choices.clone();
    }
    type_str.and_then(enum_choices).unwrap_or_default()
}

fn enum_choices(type_str: &str) -> Option<Vec<String>> {
    let lower = type_str.to_lowercase();
    let index = lower.find("one of")?;
    let mut out = Vec::new();
    let mut chars = type_str[index..].chars().peekable();
    while let Some(character) = chars.next() {
        if character == '"' {
            let mut item = String::new();
            for character in chars.by_ref() {
                if character == '"' {
                    break;
                }
                item.push(character);
            }
            out.push(item);
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

pub fn serialize(record: &Record, widget: WidgetKind, value: &Value) -> Result<String, String> {
    match widget {
        WidgetKind::Switch => Ok(switch_literal(value)?.to_string()),
        WidgetKind::Dropdown | WidgetKind::Radio | WidgetKind::Facter => {
            Ok(choice_literal(string_value(value)?))
        }
        WidgetKind::Spin | WidgetKind::Slider => {
            let number = number_value(value)?;
            Ok(number_literal(number, digits(record)))
        }
        WidgetKind::Text
        | WidgetKind::Multiline
        | WidgetKind::Color
        | WidgetKind::Theme
        | WidgetKind::Font
        | WidgetKind::IconTheme
        | WidgetKind::CursorTheme => Ok(string_literal(string_value(value)?)),
        WidgetKind::Path => Ok(path_literal(string_value(value)?)),
        WidgetKind::Package => Ok(package_literal(string_value(value)?)),
        WidgetKind::List => Ok(list_literal(&string_list(value)?, false)),
        WidgetKind::Packages => Ok(list_literal(&string_list(value)?, true)),
        WidgetKind::Wallpaper | WidgetKind::File => {
            if value.is_null() {
                Ok("null".to_string())
            } else {
                Ok(path_literal(string_value(value)?))
            }
        }
        WidgetKind::TableAttrset => serialize_table(record, false, value),
        WidgetKind::TableList => serialize_table(record, true, value),
        WidgetKind::Raw => Ok(string_value(value)?.to_string()),
        // Structured widgets fall back to a raw Nix editor until their
        // dedicated renderer lands. The browser sends the text verbatim.
        other => value.as_str().map(str::to_string).ok_or_else(|| {
            format!(
                "widget {} needs a structured state",
                widget_name(other)
            )
        }),
    }
}

fn digits(record: &Record) -> u32 {
    record
        .spec
        .digits
        .unwrap_or_else(|| if record.value.contains('.') { 2 } else { 0 })
}

fn switch_literal(value: &Value) -> Result<bool, String> {
    value
        .as_bool()
        .or_else(|| value.as_str().map(|text| text == "true"))
        .ok_or_else(|| "switch value must be a boolean".to_string())
}

fn string_value(value: &Value) -> Result<&str, String> {
    value
        .as_str()
        .ok_or_else(|| "value must be a string".to_string())
}

fn number_value(value: &Value) -> Result<f64, String> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
        .ok_or_else(|| "value must be a number".to_string())
}

fn string_list(value: &Value) -> Result<Vec<String>, String> {
    let items = value
        .as_array()
        .ok_or_else(|| "value must be a list".to_string())?;
    items
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_string)
                .ok_or_else(|| "list items must be strings".to_string())
        })
        .collect()
}

pub fn serialize_table(record: &Record, list: bool, value: &Value) -> Result<String, String> {
    let rows = value
        .get("rows")
        .and_then(Value::as_array)
        .ok_or_else(|| "table value must contain rows".to_string())?;
    let columns = record.spec.columns.clone().unwrap_or_default();
    if columns.is_empty() {
        return Err("table has no columns".to_string());
    }
    let types = record.spec.column_types.clone().unwrap_or_default();
    let mut rendered = Vec::new();

    for row in rows {
        let object = row
            .as_object()
            .ok_or_else(|| "table row must be an object".to_string())?;
        let key = object.get("key").and_then(Value::as_str).unwrap_or_default();
        let fields = object
            .get("fields")
            .and_then(Value::as_object)
            .ok_or_else(|| "table row must contain fields".to_string())?;
        let raw_expressions = object.get("rawExpressions").and_then(Value::as_object);
        let mut body = Vec::new();
        for (index, column) in columns.iter().enumerate() {
            let cell = fields.get(column).cloned().unwrap_or(Value::Null);
            let kind = types.get(index).map(String::as_str).unwrap_or_default();
            let raw = is_raw_expression(raw_expressions, column);
            body.push(format!(
                "{} = {};",
                quote(column),
                table_cell(kind, &cell, raw)?
            ));
        }
        for (column, cell) in fields {
            if columns.iter().any(|declared| declared == column) {
                continue;
            }
            let raw = is_raw_expression(raw_expressions, column);
            body.push(format!("{} = {};", quote(column), table_cell("", cell, raw)?));
        }
        let body = body.join(" ");
        let comment = row_comment(object);
        if list {
            rendered.push(format!("{comment}{{ {body} }}"));
        } else {
            rendered.push(format!("{comment}{} = {{ {body} }};", quote(key)));
        }
    }

    if list {
        Ok(format!("[ {} ]", rendered.join(" ")))
    } else {
        Ok(format!("{{ {} }}", rendered.join(" ")))
    }
}

fn row_comment(row: &serde_json::Map<String, Value>) -> String {
    let mut parts = Vec::new();
    if let Some(label) = row.get("label").and_then(Value::as_str) {
        if !label.is_empty() {
            parts.push(format!("label = {};", string_literal(label)));
        }
    }
    if row.get("hidden").and_then(Value::as_bool) == Some(true) {
        parts.push("hidden = true;".to_string());
    }
    if let Some(visibility) = row.get("visibleWhen").and_then(Value::as_object) {
        if let Some(path) = visibility.get("path").and_then(Value::as_str) {
            parts.push(format!("visibleWhen.path = {};", string_literal(path)));
        }
        if let Some(equals) = visibility.get("equals").and_then(Value::as_str) {
            parts.push(format!("visibleWhen.equals = {equals};"));
        }
        if let Some(equals_any) = visibility.get("equalsAny").and_then(Value::as_array) {
            let values: Vec<String> = equals_any
                .iter()
                .filter_map(Value::as_str)
                .map(string_literal)
                .collect();
            if !values.is_empty() {
                parts.push(format!(
                    "visibleWhen.equals = [ {} ];",
                    values.join(" ")
                ));
            }
        }
        if let Some(not_equals) = visibility.get("notEquals").and_then(Value::as_str) {
            parts.push(format!("visibleWhen.notEquals = {not_equals};"));
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("/* @table-item {} */\n", parts.join(" "))
    }
}

fn is_raw_expression(raw: Option<&serde_json::Map<String, Value>>, column: &str) -> bool {
    raw.and_then(|fields| fields.get(column))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn table_cell(kind: &str, value: &Value, raw: bool) -> Result<String, String> {
    if raw {
        return match value {
            Value::String(text) => Ok(expr_literal(text)),
            Value::Bool(enabled) => Ok(enabled.to_string()),
            Value::Number(number) => Ok(number.to_string()),
            Value::Null => Ok("null".to_string()),
            _ => Err("unsupported table cell value".to_string()),
        };
    }
    match kind {
        "bool" => Ok(switch_literal(value)?.to_string()),
        "int" | "float" => Ok(number_literal(number_value(value)?, 0)),
        "dropdown" => Ok(choice_literal(string_value(value)?)),
        "str" | "icon" => Ok(string_literal(string_value(value)?)),
        _ => match value {
            Value::Bool(enabled) => Ok(enabled.to_string()),
            Value::Number(number) => Ok(number.to_string()),
            Value::String(text) if text.trim().is_empty() => Ok("\"\"".to_string()),
            Value::String(text) => Ok(string_literal(text)),
            Value::Null => Ok("null".to_string()),
            _ => Err("unsupported table cell value".to_string()),
        },
    }
}

pub fn quote(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace("${", "\\${");
    format!("\"{escaped}\"")
}

pub fn string_literal(text: &str) -> String {
    if text.contains('\n') {
        format!("''\n{text}\n''")
    } else {
        quote(text)
    }
}

pub fn choice_literal(raw: &str) -> String {
    let text = raw.trim();
    if matches!(text, "true" | "false" | "null") || text.parse::<f64>().is_ok() {
        text.to_string()
    } else {
        quote(text)
    }
}

pub fn expr_literal(raw: &str) -> String {
    let text = raw.trim();
    if text.is_empty() {
        return "\"\"".to_string();
    }
    let parsed = rnix::Root::parse(text).ok().ok().and_then(|root| root.expr());
    if parsed.is_some() {
        text.to_string()
    } else {
        quote(text)
    }
}

pub fn package_literal(raw: &str) -> String {
    let text = raw.trim();
    if is_bare_package(text) {
        format!("pkgs.{text}")
    } else {
        expr_literal(text)
    }
}

fn is_bare_package(text: &str) -> bool {
    match text.chars().next() {
        Some(character) if character.is_ascii_alphabetic() || character == '_' => {}
        _ => return false,
    }
    text.chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_' || character == '-')
}

pub fn path_literal(raw: &str) -> String {
    let text = raw.trim();
    if text.starts_with("./") || text.starts_with("../") || text.starts_with('/') {
        text.to_string()
    } else {
        quote(text)
    }
}

pub fn number_literal(value: f64, digits: u32) -> String {
    if digits == 0 && value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        format!("{value:.precision$}", precision = digits as usize)
    }
}

pub fn list_literal(items: &[String], _packages: bool) -> String {
    let literals: Vec<String> = items
        .iter()
        .map(|item| expr_literal(item))
        .filter(|item| !item.is_empty() && item != "\"\"")
        .collect();
    value::serialize_list(&literals)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_basic_widgets() {
        let record = Record::default();
        assert_eq!(
            serialize(&record, WidgetKind::Switch, &Value::Bool(true)).unwrap(),
            "true"
        );
        assert_eq!(
            serialize(&record, WidgetKind::Text, &Value::String("hello".into())).unwrap(),
            "\"hello\""
        );
        assert_eq!(
            serialize(&record, WidgetKind::Multiline, &Value::String("a\nb".into())).unwrap(),
            "''\na\nb\n''"
        );
        assert_eq!(
            serialize(&record, WidgetKind::Path, &Value::String("./x".into())).unwrap(),
            "./x"
        );
        assert_eq!(
            serialize(&record, WidgetKind::Package, &Value::String("git".into())).unwrap(),
            "pkgs.git"
        );
        assert_eq!(
            serialize(
                &record,
                WidgetKind::List,
                &serde_json::json!(["one", "two"])
            )
            .unwrap(),
            "[ one two ]"
        );
    }

    #[test]
    fn serializes_table_rows() {
        let mut record = Record::default();
        record.spec.columns = Some(vec!["name".to_string(), "enabled".to_string()]);
        record.spec.column_types = Some(vec!["str".to_string(), "bool".to_string()]);
        let value = serde_json::json!({
            "rows": [{
                "key": "",
                "fields": { "name": "one", "enabled": "true", "extra": "kept" }
            }]
        });
        assert_eq!(
            serialize_table(&record, true, &value).unwrap(),
            "[ { \"name\" = \"one\"; \"enabled\" = true; \"extra\" = \"kept\"; } ]"
        );
    }

    #[test]
    fn preserves_table_item_directives() {
        let mut record = Record::default();
        record.spec.columns = Some(vec!["name".to_string()]);
        let value = serde_json::json!({
            "rows": [{
                "key": "one",
                "label": "First",
                "hidden": true,
                "visibleWhen": { "path": "services.demo", "equals": "true" },
                "fields": { "name": "one" }
            }]
        });
        let rendered = serialize_table(&record, false, &value).unwrap();
        assert!(rendered.contains(
            "/* @table-item label = \"First\"; hidden = true; visibleWhen.path = \"services.demo\"; visibleWhen.equals = true; */"
        ));
        assert!(rendered.contains("\"one\" = { \"name\" = \"one\"; }"));
    }

    #[test]
    fn quotes_choice_labels_but_keeps_verbs() {
        assert_eq!(choice_literal("gnome"), "\"gnome\"");
        assert_eq!(choice_literal("true"), "true");
        assert_eq!(choice_literal("42"), "42");
    }

    #[test]
    fn emits_kebab_case_widget_names() {
        assert_eq!(widget_name(WidgetKind::TableAttrset), "table-attrset");
        assert_eq!(widget_name(WidgetKind::TableList), "table-list");
        assert_eq!(
            widget_name(WidgetKind::HorizontalSubmodule),
            "horizontal-submodule"
        );
        assert_eq!(widget_name(WidgetKind::IconTheme), "icon-theme");
        assert_eq!(widget_name(WidgetKind::GridConfiguration), "grid-configuration");
        assert_eq!(widget_name(WidgetKind::NixosModules), "nixos-modules");
        assert_eq!(widget_name(WidgetKind::Switch), "switch");
    }

    #[test]
    fn serializes_string_and_raw_widgets() {
        let record = Record::default();
        for kind in [
            WidgetKind::Text,
            WidgetKind::Multiline,
            WidgetKind::Color,
            WidgetKind::Theme,
            WidgetKind::Font,
            WidgetKind::IconTheme,
            WidgetKind::CursorTheme,
        ] {
            assert_eq!(
                serialize(&record, kind, &serde_json::json!("value")).unwrap(),
                "\"value\""
            );
        }
        assert_eq!(
            serialize(&record, WidgetKind::Wallpaper, &Value::Null).unwrap(),
            "null"
        );
        for kind in [WidgetKind::Path, WidgetKind::Wallpaper, WidgetKind::File] {
            assert_eq!(
                serialize(&record, kind, &serde_json::json!("./asset.png")).unwrap(),
                "./asset.png"
            );
        }
        for kind in [
            WidgetKind::Submodule,
            WidgetKind::HorizontalSubmodule,
            WidgetKind::GridConfiguration,
            WidgetKind::Keybinds,
            WidgetKind::Monitors,
            WidgetKind::Users,
            WidgetKind::Menu,
            WidgetKind::Plugins,
            WidgetKind::Bar,
            WidgetKind::Dock,
            WidgetKind::Docker,
            WidgetKind::Raw,
        ] {
            assert_eq!(
                serialize(&record, kind, &serde_json::json!("{ }")).unwrap(),
                "{ }"
            );
        }
    }

    #[test]
    fn widget_names_round_trip_through_the_parser() {
        let kinds = [
            WidgetKind::Switch,
            WidgetKind::Dropdown,
            WidgetKind::Radio,
            WidgetKind::Spin,
            WidgetKind::Slider,
            WidgetKind::Text,
            WidgetKind::Multiline,
            WidgetKind::Path,
            WidgetKind::Color,
            WidgetKind::Package,
            WidgetKind::Packages,
            WidgetKind::List,
            WidgetKind::NixosModules,
            WidgetKind::HmModules,
            WidgetKind::Theme,
            WidgetKind::Facter,
            WidgetKind::Font,
            WidgetKind::IconTheme,
            WidgetKind::CursorTheme,
            WidgetKind::Submodule,
            WidgetKind::HorizontalSubmodule,
            WidgetKind::GridConfiguration,
            WidgetKind::TableAttrset,
            WidgetKind::TableList,
            WidgetKind::Keybinds,
            WidgetKind::Monitors,
            WidgetKind::Wallpaper,
            WidgetKind::File,
            WidgetKind::Users,
            WidgetKind::SecureBoot,
            WidgetKind::Menu,
            WidgetKind::Plugins,
            WidgetKind::Bar,
            WidgetKind::Dock,
            WidgetKind::Docker,
            WidgetKind::Raw,
        ];
        for kind in kinds {
            let name = widget_name(kind);
            assert_eq!(
                parse_widget(&name),
                Some(kind),
                "widget name {name} does not parse back"
            );
        }
    }

    #[test]
    fn picks_the_declared_widget_first() {
        let mut record = Record::default();
        record.value = "true".into();
        record.spec.widget = Some(WidgetKind::Text);
        assert_eq!(choose_widget(&record, None), WidgetKind::Text);
    }
}

/// Render an initial text value for the add-option dialog. This mirrors the
/// desktop helper so a new control starts with the same literal shape.
pub fn literal_for_widget(widget: WidgetKind, value: &str) -> String {
    let text = value.trim();
    match widget {
        WidgetKind::Switch | WidgetKind::SecureBoot => {
            if matches!(text, "true" | "false") {
                text.to_string()
            } else {
                "false".to_string()
            }
        }
        WidgetKind::Spin | WidgetKind::Slider => {
            if text.parse::<f64>().is_ok() {
                text.to_string()
            } else {
                "0".to_string()
            }
        }
        WidgetKind::Text
        | WidgetKind::Multiline
        | WidgetKind::Color
        | WidgetKind::Font
        | WidgetKind::IconTheme
        | WidgetKind::CursorTheme
        | WidgetKind::Facter => string_literal(value),
        WidgetKind::Path => path_literal(value),
        WidgetKind::Wallpaper | WidgetKind::File => {
            if text == "null" {
                "null".to_string()
            } else {
                path_literal(value)
            }
        }
        WidgetKind::Dropdown | WidgetKind::Radio => choice_literal(value),
        WidgetKind::Packages => {
            let items = value::list_items(value).unwrap_or_else(|| vec![text.to_string()]);
            list_literal(&items, true)
        }
        WidgetKind::Package => package_literal(value),
        WidgetKind::Users => {
            if text.is_empty() {
                "{ }".to_string()
            } else {
                expr_literal(value)
            }
        }
        _ => expr_literal(value),
    }
}

#[cfg(test)]
mod literal_tests {
    use super::*;

    #[test]
    fn renders_initial_values_for_add_option() {
        assert_eq!(literal_for_widget(WidgetKind::Switch, "true"), "true");
        assert_eq!(literal_for_widget(WidgetKind::Text, "hello"), "\"hello\"");
        assert_eq!(literal_for_widget(WidgetKind::Package, "git"), "pkgs.git");
        assert_eq!(literal_for_widget(WidgetKind::Dropdown, "gnome"), "\"gnome\"");
        assert_eq!(literal_for_widget(WidgetKind::Path, "/tmp/x"), "/tmp/x");
    }
}
