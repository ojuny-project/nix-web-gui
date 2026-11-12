use nix_settings_gui::model::{MenuModel, Record};
use serde_json::{json, Value};

pub fn record_json(record: &Record) -> Value {
    json!({
        "path": record.path(),
        "file": record.file,
        "value": record.value,
        "label": record.spec.label,
        "description": record.spec.description,
        "widget": record.spec.widget.map(|widget| format!("{widget:?}").to_lowercase()),
        "choices": record.spec.choices,
        "disabledFeatures": record.spec.disabled_features,
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
        "columns": record.spec.columns,
        "keyEditable": record.spec.key_editable,
    })
}

pub fn menu_json(menu: &MenuModel) -> Value {
    json!({
        "file": menu.file,
        "displayname": menu.display_name(),
        "description": menu.spec.description,
        "category": menu.spec.category,
        "icon": menu.spec.icon,
        "order": menu.order(),
        "systems": menu.spec.systems,
        "hidden": menu.spec.hidden,
        "records": menu.records.iter().map(record_json).collect::<Vec<_>>(),
    })
}

pub fn page_json(menu: &MenuModel) -> Value {
    json!({
        "id": menu.file,
        "title": menu.display_name(),
        "description": menu.spec.description,
        "category": menu.spec.category,
        "icon": menu.spec.icon,
        "order": menu.order(),
        "systems": menu.spec.systems,
        "hidden": menu.spec.hidden,
        "records": menu.records.iter().map(|record| record.path()).collect::<Vec<_>>(),
    })
}
