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
            let fields: Map<String, Value> = row.fields.iter()
                .map(|(key, value)| (key.clone(), json!(value)))
                .collect();
            json!({
                "key": row.key,
                "label": row.spec.label,
                "hidden": row.spec.hidden,
                "visibleWhen": row.spec.visible_when.as_ref().map(visibility_json),
                "fields": fields,
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
        "records": menu.records.iter().map(record_json).collect::<Vec<_>>(),
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
        "records": menu.records.iter().map(|record| record.path()).collect::<Vec<_>>(),
    })
}
