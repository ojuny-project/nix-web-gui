use std::collections::HashMap;

use nix_settings_gui::dsl::WidgetKind;
use nix_settings_gui::metadata::{self, Target};
use nix_settings_gui::model::Record;
use nix_settings_gui::{edit, scan};
use serde_json::{json, Value};

/// One open settings root and its staged edits.
pub struct SettingsSession {
    pub root: String,
    pub scanned: scan::ScanResult,
    staged: HashMap<String, String>,
    target: Option<Target>,
    target_checked: bool,
    descriptors: HashMap<String, Value>,
    child_records: HashMap<String, Record>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewEdit {
    pub file: String,
    pub start: usize,
    pub end: usize,
    pub before: Option<String>,
    pub after: String,
}

impl SettingsSession {
    pub fn open(root: impl Into<String>) -> Result<Self, String> {
        let root = root.into();
        let scanned = scan::scan_root(&root)?;
        Ok(Self {
            root,
            scanned,
            staged: HashMap::new(),
            target: None,
            target_checked: false,
            descriptors: HashMap::new(),
            child_records: HashMap::new(),
        })
    }

    pub fn reload(&mut self) -> Result<(), String> {
        self.scanned = scan::scan_root(&self.root)?;
        self.target = None;
        self.target_checked = false;
        self.descriptors.clear();
        self.child_records.clear();
        Ok(())
    }

    pub fn discard(&mut self) -> Result<(), String> {
        self.staged.clear();
        self.reload()
    }

    pub fn menus(&self) -> &[nix_settings_gui::model::MenuModel] {
        &self.scanned.menus
    }

    pub fn hosts(&self) -> Vec<String> {
        scan::hosts_from_targets(&self.scanned.targets)
    }

    pub fn menus_filtered(
        &self,
        host: Option<&str>,
    ) -> Vec<&nix_settings_gui::model::MenuModel> {
        match host {
            Some(host) => self.scanned.menus_for(host),
            None => self.scanned.menus.iter().collect(),
        }
    }

    pub fn records(&self) -> impl Iterator<Item = &Record> {
        self.scanned
            .menus
            .iter()
            .flat_map(|menu| menu.records.iter())
    }

    pub fn record(&self, path: &str) -> Option<&Record> {
        self.records()
            .find(|record| record.path() == path)
            .or_else(|| self.child_records.get(path))
    }

    pub fn set(&mut self, path: &str, value: impl Into<String>) -> Result<(), String> {
        if self.record(path).is_none() {
            return Err(format!("unknown option path: {path}"));
        }
        self.staged.insert(path.to_string(), value.into());
        Ok(())
    }

    pub fn reset(&mut self, path: &str) -> bool {
        self.staged.remove(path).is_some()
    }

    pub fn staged(&self) -> &HashMap<String, String> {
        &self.staged
    }

    pub fn target(&mut self) -> Option<Target> {
        if !self.target_checked {
            self.target_checked = true;
            self.target = discover_target(&self.root);
        }
        self.target.clone()
    }

    pub fn describe(&mut self, path: &str) -> Result<Value, String> {
        if let Some(cached) = self.descriptors.get(path) {
            return Ok(cached.clone());
        }

        let record = self
            .record(path)
            .cloned()
            .ok_or_else(|| format!("unknown option path: {path}"))?;
        let target = self.target();
        let mut parent_cache = HashMap::new();
        let descriptor = self.describe_record(&record, target.as_ref(), &mut parent_cache, 0);
        self.descriptors.insert(path.to_string(), descriptor.clone());
        Ok(descriptor)
    }

    pub fn describe_page(&mut self, file: &str) -> Result<Vec<Value>, String> {
        let target = self.target();
        self.describe_page_inner(file, target.as_ref())
    }

    /// Describe a page without nixd or eval metadata. Used by tests and by
    /// callers that already know the target is unavailable.
    pub fn describe_page_local(&mut self, file: &str) -> Result<Vec<Value>, String> {
        self.describe_page_inner(file, None)
    }

    fn describe_page_inner(
        &mut self,
        file: &str,
        target: Option<&Target>,
    ) -> Result<Vec<Value>, String> {
        let records = self
            .scanned
            .menus
            .iter()
            .find(|menu| menu.file == file)
            .map(|menu| menu.records.clone())
            .ok_or_else(|| format!("unknown page: {file}"))?;
        let mut parent_cache = HashMap::new();
        let mut out = Vec::new();

        for record in &records {
            let path = record.path();
            if let Some(cached) = self.descriptors.get(&path) {
                out.push(cached.clone());
                continue;
            }
            let descriptor = self.describe_record(record, target, &mut parent_cache, 0);
            self.descriptors.insert(path, descriptor.clone());
            out.push(descriptor);
        }

        Ok(out)
    }

    fn describe_record(
        &mut self,
        record: &Record,
        target: Option<&Target>,
        parent_cache: &mut HashMap<String, Vec<metadata::ChildOption>>,
        depth: usize,
    ) -> Value {
        let path = record.path();
        let mut type_str = None;
        let mut description = record.spec.description.clone();
        let mut read_only = false;

        if let Some(target) = target {
            let target_arg = target.as_arg();
            if let Ok(info) = nix_settings_gui::nixd::query_option(&self.root, &target_arg, &path) {
                if info.type_str.is_some() || !info.description.is_empty() {
                    type_str = info.type_str;
                    if description.is_none() && !info.description.is_empty() {
                        description = Some(info.description);
                    }
                }
            }

            if type_str.is_none() {
                if let Some((name, parent)) = split_path(&path) {
                    let key = parent.join(".");
                    let options = parent_cache.entry(key).or_insert_with(|| {
                        metadata::enumerate(&self.root, target, &parent).unwrap_or_default()
                    });
                    if let Some(option) = options.iter().find(|option| option.name == name) {
                        type_str = Some(option.type_str.clone());
                        read_only = option.read_only;
                        if description.is_none() && !option.description.is_empty() {
                            description = Some(option.description.clone());
                        }
                    }
                }
            }
        }

        let widget = crate::render::choose_widget(record, type_str.as_deref());
        let mut choices = crate::render::resolve_choices(&record.spec, type_str.as_deref());
        if widget == WidgetKind::Facter && choices.is_empty() {
            choices.push("auto".to_string());
            choices.extend(nix_settings_gui::widgets::facter_detect::profile_choices());
        }
        let mut out = crate::api::record_json(record);
        if let Some(type_str) = &type_str {
            out["type"] = json!(type_str);
        }
        if let Some(description) = &description {
            if out["description"].is_null() {
                out["description"] = json!(description);
            }
        }
        out["widget"] = json!(crate::render::widget_name(widget));
        out["choices"] = json!(choices);
        out["readonly"] = json!(record.spec.readonly || read_only);
        out["depth"] = json!(depth);
        if out["label"].is_null() {
            if let Some(name) = path.rsplit('.').next() {
                out["label"] = json!(name);
            }
        }

        if is_composite_widget(widget) && depth < 4 {
            let children = self.describe_children(record, target, parent_cache, depth + 1);
            if !children.is_empty() {
                out["children"] = json!(children);
            }
        }

        out
    }

    fn describe_children(
        &mut self,
        record: &Record,
        target: Option<&Target>,
        parent_cache: &mut HashMap<String, Vec<metadata::ChildOption>>,
        depth: usize,
    ) -> Vec<Value> {
        let parent_segments = record.path_segments().to_vec();
        let mut out = Vec::new();
        for mut child in nix_settings_gui::scan::scan_value(&record.value) {
            child.byte_start += record.byte_start;
            child.byte_end += record.byte_start;
            if let Some((start, end)) = child.comment_span {
                child.comment_span = Some((start + record.byte_start, end + record.byte_start));
            }
            child.file = record.file.clone();
            if child.origin.is_empty() {
                child.origin = record.origin.clone();
            }
            let mut segments = parent_segments.clone();
            segments.extend(child.segments.clone());
            child.segments = segments;
            child.option_path = None;
            let path = child.path();
            self.child_records.insert(path.clone(), child.clone());
            let descriptor = self.describe_record(&child, target, parent_cache, depth);
            self.descriptors.insert(path, descriptor.clone());
            out.push(descriptor);
        }
        out
    }

    pub fn add_option(
        &mut self,
        file: &str,
        origin: &str,
        option_path: &str,
        widget: &str,
        label: &str,
        value: &str,
    ) -> Result<(), String> {
        if !crate::paths::is_editable(std::path::Path::new(&self.root), std::path::Path::new(file)) {
            return Err(format!("cannot edit {file}: outside the configuration root"));
        }
        let widget = crate::render::parse_widget(widget)
            .ok_or_else(|| format!("unknown widget: {widget}"))?;
        let literal = crate::render::literal_for_widget(widget, value);
        let mut block = String::from("/* @option\n");
        block.push_str(&format!("   widget = \"{}\";\n", crate::render::widget_name(widget)));
        if !label.trim().is_empty() {
            block.push_str(&format!("   label = \"{}\";\n", label.trim()));
        }
        block.push_str("*/\n");
        block.push_str(&format!("{option_path} = {literal};"));
        edit::insert_assignment(file, origin, &block)?;
        self.reload()
    }

    pub fn insert_snippet(
        &mut self,
        file: &str,
        name: &str,
        offset: Option<usize>,
        values: &[(String, String)],
    ) -> Result<(), String> {
        if !crate::paths::is_editable(std::path::Path::new(&self.root), std::path::Path::new(file)) {
            return Err(format!("cannot edit {file}: outside the configuration root"));
        }

        let (marker_offset, body) = {
            let menu = self
                .scanned
                .menus
                .iter()
                .find(|menu| menu.file == file)
                .ok_or_else(|| format!("unknown page: {file}"))?;
            let marker = menu
                .inserts
                .iter()
                .find(|marker| marker.name == name && offset.map_or(true, |value| marker.offset == value))
                .ok_or_else(|| format!("unknown insert marker: {name}"))?;
            let snippet = menu
                .snippets
                .iter()
                .find(|snippet| snippet.name == name)
                .ok_or_else(|| format!("unknown snippet: {name}"))?;
            (marker.offset, snippet.body.clone())
        };

        let rendered = crate::authoring::render_body(&body, values);
        edit::insert_at(file, marker_offset, &rendered)?;
        self.reload()
    }

    pub fn pending_edits(&self) -> Result<Vec<edit::PendingEdit>, String> {
        let mut edits = Vec::new();
        for (path, value) in &self.staged {
            let record = self
                .record(path)
                .ok_or_else(|| format!("unknown option path: {path}"))?;
            edits.push(edit::PendingEdit {
                file: record.file.clone(),
                start: record.byte_start,
                end: record.byte_end,
                value: value.clone(),
            });
        }
        Ok(edits)
    }

    pub fn preview(&self) -> Result<Vec<PreviewEdit>, String> {
        self.pending_edits()?
            .into_iter()
            .map(|edit| {
                let before = std::fs::read_to_string(&edit.file)
                    .ok()
                    .and_then(|text| text.get(edit.start..edit.end).map(str::to_string));
                Ok(PreviewEdit {
                    file: edit.file,
                    start: edit.start,
                    end: edit.end,
                    before,
                    after: edit.value,
                })
            })
            .collect()
    }

    pub fn apply(&mut self) -> Result<usize, String> {
        let edits = self.pending_edits()?;
        if edits.is_empty() {
            return Ok(0);
        }
        let files = edit::apply(&edits, &self.root)?;
        self.staged.clear();
        self.reload()?;
        Ok(files)
    }
}

fn is_composite_widget(widget: WidgetKind) -> bool {
    matches!(
        widget,
        WidgetKind::Submodule
            | WidgetKind::HorizontalSubmodule
            | WidgetKind::Users
            | WidgetKind::Monitors
            | WidgetKind::Docker
            | WidgetKind::Menu
            | WidgetKind::Bar
            | WidgetKind::Dock
            | WidgetKind::GridConfiguration
            | WidgetKind::Plugins
            | WidgetKind::Keybinds
    )
}

fn discover_target(root: &str) -> Option<Target> {
    let configs = metadata::configs(root).ok()?;
    if let Some(host) = configs.nixos.first() {
        return Some(Target::Nixos(host.clone()));
    }
    if let Some(user) = configs.home.first() {
        return Some(Target::Home(user.clone()));
    }
    configs.flake_parts.then_some(Target::FlakeParts)
}

fn split_path(path: &str) -> Option<(String, Vec<String>)> {
    let segments: Vec<String> = path
        .split('.')
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect();
    let (name, parent) = segments.split_last()?;
    Some((name.clone(), parent.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_and_reloads_a_fixture() {
        let root = format!(
            "{}/../tests/fixtures/plain-flake",
            env!("CARGO_MANIFEST_DIR")
        );
        let mut session = SettingsSession::open(root).unwrap();
        assert!(!session.menus().is_empty());
        session.reload().unwrap();
    }

    #[test]
    fn describes_submodule_children() {
        let root = format!(
            "{}/../tests/fixtures/plain-flake",
            env!("CARGO_MANIFEST_DIR")
        );
        let mut session = SettingsSession::open(root).unwrap();
        let file = session.menus()[0].file.clone();
        let descriptors = session.describe_page_local(&file).unwrap();
        let parent = descriptors
            .iter()
            .find(|descriptor| descriptor["path"] == "services.example")
            .expect("submodule descriptor");
        let children = parent["children"].as_array().expect("children");
        assert!(children
            .iter()
            .any(|child| child["path"] == "services.example.enable"));
        assert!(session.record("services.example.enable").is_some());

        let table = descriptors
            .iter()
            .find(|descriptor| descriptor["path"] == "services.exampleRows")
            .expect("table descriptor");
        assert_eq!(table["table"]["rows"].as_array().unwrap().len(), 1);
    }

    /// Gate: annotations inside lists, map, mkMerge, and `lib.mkIf` bodies stay
    /// invisible. Plain annotated assignments remain visible. A local patch
    /// that adds list or apply-node descent fails here.
    #[test]
    fn gate_limitation_no_list_descent() {
        let root = format!(
            "{}/../tests/fixtures/limitations",
            env!("CARGO_MANIFEST_DIR")
        );
        let session = SettingsSession::open(root).unwrap();
        let paths: Vec<String> = session.records().map(|record| record.path()).collect();
        assert!(!paths.iter().any(|path| path == "networking.hostName"));
        assert!(!paths.iter().any(|path| path == "networking.domain"));
        assert!(!paths.iter().any(|path| path == "time.timeZone"));
        assert!(paths.iter().any(|path| path == "i18n.defaultLocale"));
    }
}
