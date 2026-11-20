use nix_settings_gui::dsl::WidgetKind;
use nix_settings_gui::model::{MenuModel, Record};
use nix_settings_gui::scan;
use serde_json::{json, Map, Value};

pub fn record_json(record: &Record) -> Value {
    json!({
        "path": record.path(),
        "file": record.file,
        "value": record.value,
        "label": record.spec.label,
        "description": record.spec.description,
        "widget": record.spec.widget.map(crate::render::widget_name),
        "choices": record.spec.choices,
        "disabledFeatures": record.spec.disabled_features,
        "list": nix_settings_gui::value::list_items(&record.value),
        "advanced": record.spec.advanced,
        "hidden": record.spec.hidden,
        "readonly": record.spec.readonly,
        "group": record.spec.group,
        "groupOrder": record.spec.group_order,
        "order": record.spec.order,
        "sourceOrder": record.byte_start,
        "min": record.spec.min,
        "max": record.spec.max,
        "step": record.spec.step,
        "digits": record.spec.digits,
        "unit": record.spec.unit,
        "rows": record.spec.rows,
        "placeholder": record.spec.placeholder,
        "maxColumns": record.spec.max_columns,
        "monospace": record.spec.monospace,
        "variant": record.spec.variant,
        "columns": record.spec.columns,
        "bindFields": record.spec.bind_fields,
        "bindFieldTypes": record.spec.bind_field_types,
        "bindPreview": record.spec.bind_preview,
        "bindSort": record.spec.bind_sort,
        "bindKeyField": record.spec.bind_key_field,
        "presetOption": record.spec.preset_option,
        "reportPath": record.spec.report_path,
        "profileCatalog": record.spec.profile_catalog,
        "gridRegistry": record.spec.grid_registry,
        "gridColumns": record.spec.grid_columns,
        "gridEnabled": record.spec.grid_enabled,
        "containerImage": record.spec.container_image,
        "containerPorts": record.spec.container_ports,
        "containerVolumes": record.spec.container_volumes,
        "containerEnvironment": record.spec.container_environment,
        "containerAutoStart": record.spec.container_auto_start,
        "containerAutoStartDefault": record.spec.container_auto_start_default,
        "keyEditable": record.spec.key_editable,
        "applyCommand": record.spec.apply_command,
        "origin": record.origin,
        "visibleWhen": record.spec.visible_when.as_ref().map(visibility_json),
        "table": table_json(record),
    })
}

fn table_json(record: &Record) -> Value {
    let list = match record.spec.widget {
        Some(WidgetKind::TableAttrset) => false,
        Some(WidgetKind::TableList) => true,
        _ => return Value::Null,
    };
    let rows = scan::scan_table(&record.value, list);
    json!({
        "list": list,
        "columns": record.spec.columns.clone().unwrap_or_default(),
        "columnTypes": record.spec.column_types.clone().unwrap_or_default(),
        "choices": record.spec.choices.clone().unwrap_or_default(),
        "keyEditable": record.spec.key_editable,
        "firstValueEditable": record.spec.first_value_editable,
        "rows": rows.iter().map(|row| {
            let mut fields = Map::new();
            let mut raw_expressions = Map::new();
            for (key, value) in &row.fields {
                match crate::render::table_string_value(value) {
                    Some(text) => {
                        fields.insert(key.clone(), json!(text));
                    }
                    None => {
                        if crate::render::table_cell_is_raw(value) {
                            raw_expressions.insert(key.clone(), Value::Bool(true));
                        }
                        fields.insert(key.clone(), json!(value));
                    }
                }
            }
            json!({
                "key": row.key,
                "label": row.spec.label,
                "hidden": row.spec.hidden,
                "visibleWhen": row.spec.visible_when.as_ref().map(visibility_json),
                "fields": fields,
                "rawExpressions": raw_expressions,
            })
        }).collect::<Vec<_>>(),
    })
}

fn visibility_json(visibility: &nix_settings_gui::dsl::Visibility) -> Value {
    json!({
        "path": visibility.path,
        "equals": visibility.equals,
        "equalsAny": visibility.equals_any,
        "notEquals": visibility.not_equals,
    })
}

pub fn menu_json(menu: &MenuModel) -> Value {
    json!({
        "file": menu.file,
        "displayname": menu.display_name(),
        "origin": menu.origins.first(),
        "description": menu.spec.description,
        "category": menu.spec.category,
        "icon": menu.spec.icon,
        "order": menu.order(),
        "systems": menu.spec.systems,
        "hidden": menu.spec.hidden,
        "hasMenu": menu.has_menu,
        "records": menu.records.iter().map(record_json).collect::<Vec<_>>(),
        "widgetPresets": menu.widget_presets.iter().map(|(preset, fields)| json!({
            "id": preset.id,
            "label": preset.label,
            "description": preset.description,
            "icon": preset.icon,
            "sections": preset.sections,
            "hasSettings": preset.has_settings,
            "defaults": preset.defaults.iter()
                .map(|(key, value)| (key.clone(), value.to_nix()))
                .collect::<std::collections::BTreeMap<_, _>>(),
            "fields": fields.iter().map(|field| json!({
                "name": field.name,
                "widget": field.widget.map(|widget| format!("{widget:?}").to_lowercase()),
                "label": field.label,
                "description": field.description,
                "choices": field.choices,
                "min": field.min,
                "max": field.max,
                "step": field.step,
                "unit": field.unit,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "inserts": menu.inserts.iter().map(|marker| json!({
            "name": marker.name,
            "group": marker.group,
            "label": marker.label,
            "offset": marker.offset,
            "line": marker.line,
        })).collect::<Vec<_>>(),
        "snippets": menu.snippets.iter().map(|snippet| json!({
            "name": snippet.name,
            "label": snippet.label,
            "placeholders": snippet.placeholders.iter().map(|placeholder| json!({
                "name": placeholder.name,
                "label": placeholder.label,
                "default": placeholder.default,
                "hint": placeholder.hint,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

pub fn page_json(menu: &MenuModel) -> Value {
    json!({
        "id": menu.file,
        "title": menu.display_name(),
        "origin": menu.origins.first(),
        "description": menu.spec.description,
        "category": menu.spec.category,
        "icon": menu.spec.icon,
        "order": menu.order(),
        "systems": menu.spec.systems,
        "hidden": menu.spec.hidden,
        "hasMenu": menu.has_menu,
        "records": menu.records.iter().map(|record| record.path()).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_non_string_table_cells_as_raw() {
        let mut record = Record::default();
        record.spec.widget = Some(WidgetKind::TableList);
        record.spec.columns = Some(vec![
            "name".to_string(),
            "icon".to_string(),
            "enabled".to_string(),
            "width".to_string(),
        ]);
        record.spec.column_types = Some(vec![
            "str".to_string(),
            "icon".to_string(),
            "bool".to_string(),
            "int".to_string(),
        ]);
        record.value =
            r#"[ { name = "one"; icon = ./icons/one.png; enabled = true; width = 42; } ]"#
                .to_string();

        let table = table_json(&record);
        let row = &table["rows"][0];
        assert_eq!(row["fields"]["name"], "one");
        assert_eq!(row["fields"]["icon"], "./icons/one.png");
        assert_eq!(row["rawExpressions"]["icon"], true);
        assert_eq!(row["rawExpressions"]["enabled"], true);
        assert_eq!(row["rawExpressions"]["width"], true);
        assert!(row["rawExpressions"].get("name").is_none());
    }
}
