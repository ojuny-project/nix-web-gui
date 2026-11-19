use std::collections::{HashMap, HashSet};

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
    /// Size and modification time of every scanned file when it was read.
    file_fingerprints: HashMap<String, String>,
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
        let file_fingerprints = fingerprints(&scanned);
        Ok(Self {
            root,
            scanned,
            staged: HashMap::new(),
            target: None,
            target_checked: false,
            descriptors: HashMap::new(),
            child_records: HashMap::new(),
            file_fingerprints,
        })
    }

    pub fn reload(&mut self) -> Result<(), String> {
        self.scanned = scan::scan_root(&self.root)?;
        self.file_fingerprints = fingerprints(&self.scanned);
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

    pub fn declared_inputs(&self) -> &[nix_settings_gui::flake_inputs::DeclaredInput] {
        &self.scanned.declared_inputs
    }

    /// Wallpaper presets generated from the widget's `presetSource` flake
    /// input. The eval can realize store paths, so this is only called when a
    /// wallpaper record declares a preset source.
    fn facter_catalog(
        &self,
        spec: &nix_settings_gui::dsl::OptionSpec,
    ) -> nix_settings_gui::widgets::facter_detect::ProfileCatalog {
        spec.profile_catalog
            .as_deref()
            .and_then(|path| {
                let path = crate::paths::confine(
                    std::path::Path::new(&self.root),
                    std::path::Path::new(path),
                )
                .ok()?;
                let text = std::fs::read_to_string(path).ok()?;
                serde_json::from_str(&text).ok()
            })
            .unwrap_or_else(nix_settings_gui::widgets::facter_detect::ProfileCatalog::builtin)
    }

    pub fn wallpaper_presets(
        &mut self,
        option_path: &str,
    ) -> Result<Vec<nix_settings_gui::metadata::WallpaperPreset>, String> {
        let record = self
            .record(option_path)
            .cloned()
            .ok_or_else(|| format!("unknown option path: {option_path}"))?;
        let source = record
            .spec
            .preset_source
            .clone()
            .ok_or_else(|| "this wallpaper option has no preset source".to_string())?;
        let preset_option = record.spec.preset_option.clone().unwrap_or_default();
        let target = self
            .target()
            .ok_or_else(|| "no NixOS or Home Manager target found".to_string())?;
        let set = metadata::wallpaper_set(
            &self.root,
            &target.as_arg(),
            &preset_option,
            &source,
        )?;
        let mut presets = set.presets;
        presets.extend(set.user);
        Ok(presets)
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
            let catalog = self.facter_catalog(&record.spec);
            choices.extend(nix_settings_gui::widgets::facter_detect::profile_choices(
                &catalog,
            ));
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

        if (is_composite_widget(widget) || is_structured_list(record, widget)) && depth < 6 {
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
        let widget = crate::render::choose_widget(record, None);
        let mut children = if is_row_collection(widget) {
            nix_settings_gui::scan::scan_value_direct(&record.value)
        } else {
            nix_settings_gui::scan::scan_value(&record.value)
        };
        if children.is_empty()
            && nix_settings_gui::value::classify(&record.value)
                == nix_settings_gui::value::ValueKind::List
        {
            children = scan_list_elements(record);
        }
        for mut child in children {
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
        let values = self.current_values();
        let mut edits = Vec::new();
        for (path, value) in &self.staged {
            let record = self
                .record(path)
                .ok_or_else(|| format!("unknown option path: {path}"))?;
            if record.spec.readonly {
                continue;
            }
            if self.disabled_by_ancestor(path) {
                continue;
            }
            if let Some(visibility) = &record.spec.visible_when {
                if !visibility.holds(resolve_dependency(&values, &visibility.path)) {
                    continue;
                }
            }
            edits.push(edit::PendingEdit {
                file: record.file.clone(),
                start: record.byte_start,
                end: record.byte_end,
                value: value.clone(),
            });
        }
        Ok(edits)
    }

    /// The value each record would have after the staged edits are written.
    /// Used to evaluate `visibleWhen` gates on save, so a disabled parent can
    /// never commit its children's staged values.
    fn current_values(&self) -> HashMap<String, String> {
        let mut values = HashMap::new();
        for record in self.records().chain(self.child_records.values()) {
            let path = record.path();
            let value = self
                .staged
                .get(&path)
                .cloned()
                .unwrap_or_else(|| record.value.clone());
            values.insert(path, value);
        }
        values
    }

    fn disabled_by_ancestor(&self, path: &str) -> bool {
        self.records().any(|candidate| {
            let Some(disabled) = &candidate.spec.disabled_features else {
                return false;
            };
            let prefix = format!("{}.", candidate.path());
            if !path.starts_with(&prefix) {
                return false;
            }
            let relative = &path[prefix.len()..];
            disabled
                .iter()
                .any(|feature| relative == feature || relative.starts_with(&format!("{feature}.")))
        })
    }

    /// Fail before any write when a file changed on disk after it was scanned.
    /// Two tabs or two users can then no longer overwrite each other silently.
    fn verify_unchanged(&self, edits: &[edit::PendingEdit]) -> Result<(), String> {
        let mut seen = HashSet::new();
        let mut conflicts = Vec::new();
        for pending in edits {
            if !seen.insert(pending.file.clone()) {
                continue;
            }
            let Some(previous) = self.file_fingerprints.get(&pending.file) else {
                continue;
            };
            let current = file_fingerprint(&pending.file);
            if &current != previous {
                conflicts.push(pending.file.clone());
            }
        }
        if conflicts.is_empty() {
            return Ok(());
        }
        conflicts.sort();
        Err(format!(
            "these files changed on disk since they were opened: {}. Reload before saving.",
            conflicts.join(", ")
        ))
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
        let package_errors = self.package_errors();
        if !package_errors.is_empty() {
            return Err(format!(
                "package validation failed; nothing written:\n{}",
                package_errors.join("\n")
            ));
        }
        let user_errors = self.user_errors();
        if !user_errors.is_empty() {
            return Err(format!(
                "user validation failed; nothing written:\n{}",
                user_errors.join("\n")
            ));
        }
        self.verify_unchanged(&edits)?;
        let files = edit::apply(&edits, &self.root)?;
        self.staged.clear();
        self.reload()?;
        Ok(files)
    }

    /// Ask nixd whether each staged package expression resolves. Missing nixd
    /// infrastructure is not a save failure; real diagnostics are.
    fn package_errors(&self) -> Vec<String> {
        let checks: Vec<(String, String)> = self
            .staged
            .iter()
            .filter_map(|(path, value)| {
                let record = self.record(path)?;
                match crate::render::choose_widget(record, None) {
                    WidgetKind::Package | WidgetKind::Packages => {
                        Some((path.clone(), format!("with pkgs; {value}")))
                    }
                    _ => None,
                }
            })
            .collect();
        if checks.is_empty() {
            return Vec::new();
        }
        nix_settings_gui::nixd::query_errors(Some(&self.root), checks, |_, _| {})
            .unwrap_or_default()
    }

    /// Validate every `users` widget before saving. Mirrors the desktop
    /// invariant: one normal account must keep sudo privileges and a password.
    fn user_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        for record in self
            .records()
            .filter(|record| record.spec.widget == Some(WidgetKind::Users))
        {
            if let Some(error) = validate_users(record, &self.staged) {
                errors.push(error);
            }
        }
        errors
    }

    /// Add one row to an attribute-set or list composite widget such as users,
    /// keybinds, monitors, or docker. The row template is widget-specific.
    pub fn add_child(
        &mut self,
        parent_path: &str,
        key: &str,
        mode: Option<(u32, u32, f64)>,
        defaults: &[(String, String)],
    ) -> Result<(), String> {
        if !self.staged.is_empty() {
            return Err("save or discard staged changes before adding a row".to_string());
        }
        let record = self
            .record(parent_path)
            .cloned()
            .ok_or_else(|| format!("unknown option path: {parent_path}"))?;
        let widget = crate::render::choose_widget(&record, None);
        let kind = nix_settings_gui::value::classify(&record.value);
        let list_strings = kind == nix_settings_gui::value::ValueKind::List
            && nix_settings_gui::value::list_items(&record.value)
                .map(|items| {
                    items.iter().all(|item| {
                        nix_settings_gui::value::classify(item)
                            == nix_settings_gui::value::ValueKind::String
                    })
                })
                .unwrap_or(false);
        let template = if list_strings && widget == WidgetKind::List {
            crate::render::quote(key.trim())
        } else if kind == nix_settings_gui::value::ValueKind::List
            && widget == WidgetKind::List
        {
            let mut parts = vec![format!("id = {};", crate::render::quote(key.trim()))];
            for (field, literal) in defaults {
                parts.push(format!(
                    "{} = {literal};",
                    crate::structure::attr_path(field)?
                ));
            }
            format!("{{ {} }}", parts.join(" "))
        } else if widget == WidgetKind::Menu {
            let mut parts = if defaults.is_empty() {
                vec![format!(
                    "label = {};",
                    crate::render::quote("New entry")
                )]
            } else {
                defaults
                    .iter()
                    .map(|(field, literal)| {
                        Ok(format!(
                            "{} = {literal};",
                            crate::structure::attr_path(field)?
                        ))
                    })
                    .collect::<Result<Vec<_>, String>>()?
            };
            parts.sort();
            format!("{{ {} }}", parts.join(" "))
        } else {
            crate::structure::child_template(widget, key, mode, None, &record.spec)?
        };
        let offset = closing_offset(&record.value, kind)?;
        let insertion = match kind {
            nix_settings_gui::value::ValueKind::AttrSet => {
                if key.trim().is_empty() {
                    return Err("row key is required".to_string());
                }
                let path = crate::structure::attr_path(key.trim())?;
                format!("\n  {path} = {template};\n")
            }
            nix_settings_gui::value::ValueKind::List => format!("\n  {template}\n"),
            _ => {
                return Err(format!(
                    "{} is not an attribute set or list",
                    widget_name(widget)
                ))
            }
        };
        let pending = edit::PendingEdit {
            file: record.file.clone(),
            start: record.byte_start + offset,
            end: record.byte_start + offset,
            value: insertion,
        };
        let mut edits = vec![pending];
        if widget == WidgetKind::Users {
            if let Some(home) = home_add_edit(&record, key.trim()) {
                edits.push(home);
            }
        }
        edit::apply(&edits, &self.root)?;
        self.reload()
    }

    /// Rename one row of a composite attribute set. For the users widget the
    /// matching `home-manager.users` entry is renamed in the same batch.
    pub fn rename_child(
        &mut self,
        parent_path: &str,
        child_path: &str,
        new_key: &str,
    ) -> Result<(), String> {
        if !self.staged.is_empty() {
            return Err("save or discard staged changes before renaming a row".to_string());
        }
        let new_key = new_key.trim();
        if new_key.is_empty() {
            return Err("row key is required".to_string());
        }
        let parent = self
            .record(parent_path)
            .cloned()
            .ok_or_else(|| format!("unknown option path: {parent_path}"))?;
        let widget = crate::render::choose_widget(&parent, None);
        if widget == WidgetKind::Users && !valid_account_name(new_key) {
            return Err(format!("{new_key:?} is not a valid account name"));
        }
        let new_attr = crate::structure::attr_path(new_key)?;
        let parent_segments = parent.path_segments().to_vec();
        let child_segments: Vec<String> = child_path
            .split('.')
            .filter(|segment| !segment.is_empty())
            .map(str::to_string)
            .collect();
        let Some(relative) = child_segments.strip_prefix(parent_segments.as_slice()) else {
            return Err("the row is not inside the selected widget".to_string());
        };
        if relative.len() != 1 {
            return Err("only a direct row of the widget can be renamed".to_string());
        }
        let old_key = relative[0].clone();
        let candidates = scan::scan_value_direct(&parent.value);
        if candidates
            .iter()
            .any(|candidate| candidate.segments == [new_key.to_string()])
        {
            return Err(format!("{new_key} already exists in this list"));
        }
        let child = candidates
            .into_iter()
            .find(|candidate| candidate.segments == relative)
            .ok_or_else(|| "cannot locate the row value".to_string())?;

        let text = std::fs::read_to_string(&parent.file)
            .map_err(|error| format!("cannot read {}: {error}", parent.file))?;
        let start_absolute = parent.byte_start + child.byte_start;
        let end_absolute = parent.byte_start + child.byte_end;
        let (assignments, _) = scan::sibling_assignments(&text, parent.byte_start, parent.byte_end)
            .ok_or_else(|| "cannot locate the widget assignment".to_string())?;
        let (child_assignments, _) = scan::sibling_assignments(&text, start_absolute, end_absolute)
            .ok_or_else(|| "cannot locate the row assignment".to_string())?;
        let assignment = child_assignments
            .iter()
            .filter(|assignment| {
                assignment.value_start <= start_absolute && assignment.value_end >= end_absolute
            })
            .min_by_key(|assignment| assignment.value_end - assignment.value_start)
            .ok_or_else(|| "cannot locate the row assignment".to_string())?;
        let old_text = text
            .get(assignment.full_start..assignment.full_end)
            .ok_or_else(|| "cannot read the row assignment".to_string())?;
        let equals = old_text
            .find('=')
            .ok_or_else(|| "cannot parse the row assignment".to_string())?;
        let renamed = format!("{} {}", new_attr, &old_text[equals..]);
        let mut edits = vec![edit::PendingEdit {
            file: parent.file.clone(),
            start: assignment.full_start,
            end: assignment.full_end,
            value: renamed,
        }];

        if widget == WidgetKind::Users {
            if let Some(home) = assignments.iter().find(|assignment| {
                assignment.segments.len() == 2
                    && matches!(assignment.segments[0].as_str(), "home-manager" | "homeManager")
                    && assignment.segments[1] == "users"
            }) {
                if let Some(home_child) = scan::scan_value_direct(&home.value)
                    .into_iter()
                    .find(|candidate| candidate.segments == [old_key.clone()])
                {
                    let home_start = home.value_start + home_child.byte_start;
                    let home_end = home.value_start + home_child.byte_end;
                    if let Some((home_assignments, _)) =
                        scan::sibling_assignments(&text, home_start, home_end)
                    {
                        if let Some(home_assignment) = home_assignments.iter().find(|assignment| {
                            assignment.value_start <= home_start
                                && assignment.value_end >= home_end
                        }) {
                            if let Some(home_text) = text.get(
                                home_assignment.full_start..home_assignment.full_end,
                            ) {
                                if let Some(home_equals) = home_text.find('=') {
                                    edits.push(edit::PendingEdit {
                                        file: parent.file.clone(),
                                        start: home_assignment.full_start,
                                        end: home_assignment.full_end,
                                        value: format!(
                                            "{} {}",
                                            new_attr,
                                            &home_text[home_equals..]
                                        ),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        edit::apply(&edits, &self.root)?;
        self.reload()
    }
    /// Move one list element by `offset` positions, used by bar and dock tile
    /// sections and horizontal submodule lists.
    pub fn move_child(
        &mut self,
        parent_path: &str,
        child_path: &str,
        offset: isize,
    ) -> Result<(), String> {
        if !self.staged.is_empty() {
            return Err("save or discard staged changes before moving a row".to_string());
        }
        let parent = self
            .record(parent_path)
            .cloned()
            .ok_or_else(|| format!("unknown option path: {parent_path}"))?;
        if nix_settings_gui::value::classify(&parent.value)
            != nix_settings_gui::value::ValueKind::List
        {
            return Err("only list rows can be moved".to_string());
        }
        let parent_segments = parent.path_segments().to_vec();
        let child_segments: Vec<String> = child_path
            .split('.')
            .filter(|segment| !segment.is_empty())
            .map(str::to_string)
            .collect();
        let Some(relative) = child_segments.strip_prefix(parent_segments.as_slice()) else {
            return Err("the row is not inside the selected widget".to_string());
        };
        if relative.len() != 1 {
            return Err("only a direct list element can be moved".to_string());
        }
        let elements = scan_list_elements(&parent);
        let index = elements
            .iter()
            .position(|element| element.segments == relative)
            .ok_or_else(|| "cannot locate the list element".to_string())?;
        let target = index as isize + offset;
        if target < 0 || target as usize >= elements.len() {
            return Err("the row is already at the edge".to_string());
        }
        let target = target as usize;
        if index == target {
            return Ok(());
        }
        let moved = edit::PendingEdit {
            file: parent.file.clone(),
            start: parent.byte_start + elements[index].byte_start,
            end: parent.byte_start + elements[index].byte_end,
            value: elements[target].value.clone(),
        };
        let replaced = edit::PendingEdit {
            file: parent.file.clone(),
            start: parent.byte_start + elements[target].byte_start,
            end: parent.byte_start + elements[target].byte_end,
            value: elements[index].value.clone(),
        };
        edit::apply(&[moved, replaced], &self.root)?;
        self.reload()
    }

    /// Insert a field into an attribute-set row or update it when it already
    /// exists. Used by plugin schema controls and other generated sub-editors.
    pub fn add_field(
        &mut self,
        parent_path: &str,
        field: &str,
        literal: &str,
    ) -> Result<(), String> {
        if !self.staged.is_empty() {
            return Err("save or discard staged changes before adding a field".to_string());
        }
        let path = format!("{parent_path}.{field}");
        if self.record(&path).is_some() {
            self.staged.insert(path, literal.to_string());
            self.apply()?;
            return Ok(());
        }
        let parent = self
            .record(parent_path)
            .cloned()
            .ok_or_else(|| format!("unknown option path: {parent_path}"))?;
        if nix_settings_gui::value::classify(&parent.value)
            != nix_settings_gui::value::ValueKind::AttrSet
        {
            return Err("fields can only be added to an attribute set".to_string());
        }
        let offset = closing_offset(
            &parent.value,
            nix_settings_gui::value::ValueKind::AttrSet,
        )?;
        let field = crate::structure::attr_path(field)?;
        let pending = edit::PendingEdit {
            file: parent.file.clone(),
            start: parent.byte_start + offset,
            end: parent.byte_start + offset,
            value: format!("\n  {field} = {literal};\n"),
        };
        edit::apply(&[pending], &self.root)?;
        self.reload()
    }

    pub fn remove_child(&mut self, parent_path: &str, child_path: &str) -> Result<(), String> {
        if !self.staged.is_empty() {
            return Err("save or discard staged changes before removing a row".to_string());
        }
        let parent = self
            .record(parent_path)
            .cloned()
            .ok_or_else(|| format!("unknown option path: {parent_path}"))?;
        let parent_segments = parent.path_segments().to_vec();
        let child_segments: Vec<String> = child_path
            .split('.')
            .filter(|segment| !segment.is_empty())
            .map(str::to_string)
            .collect();
        let Some(relative) = child_segments.strip_prefix(parent_segments.as_slice()) else {
            return Err("the row is not inside the selected widget".to_string());
        };
        if relative.is_empty() {
            return Err("the selected widget itself cannot be removed".to_string());
        }

        let kind = nix_settings_gui::value::classify(&parent.value);
        let candidates = match kind {
            nix_settings_gui::value::ValueKind::List => scan_list_elements(&parent),
            _ => scan::scan_value_direct(&parent.value),
        };
        let child = candidates
            .into_iter()
            .find(|candidate| candidate.segments == relative)
            .ok_or_else(|| "cannot locate the row value".to_string())?;
        let start_absolute = parent.byte_start + child.byte_start;
        let end_absolute = parent.byte_start + child.byte_end;

        let parent_widget = crate::render::choose_widget(&parent, None);
        let text = std::fs::read_to_string(&parent.file)
            .map_err(|error| format!("cannot read {}: {error}", parent.file))?;
        let (mut start, mut end) = match nix_settings_gui::value::classify(&parent.value) {
            nix_settings_gui::value::ValueKind::AttrSet => {
                let (assignments, _) =
                    scan::sibling_assignments(&text, start_absolute, end_absolute)
                        .ok_or_else(|| "cannot locate the row assignment".to_string())?;
                let assignment = assignments
                    .iter()
                    .find(|assignment| {
                        assignment.value_start <= start_absolute
                            && assignment.value_end >= end_absolute
                    })
                    .ok_or_else(|| "cannot locate the row assignment".to_string())?;
                (assignment.full_start, assignment.full_end)
            }
            nix_settings_gui::value::ValueKind::List => (start_absolute, end_absolute),
            _ => return Err("the parent value is not an attribute set or list".to_string()),
        };
        while start > 0 && matches!(text.as_bytes()[start - 1], b' ' | b'\t') {
            start -= 1;
        }
        while end < text.len() && matches!(text.as_bytes()[end], b' ' | b'\t' | b'\r') {
            end += 1;
        }
        if end < text.len() && text.as_bytes()[end] == b'\n' {
            end += 1;
        }

        let pending = edit::PendingEdit {
            file: parent.file.clone(),
            start,
            end,
            value: String::new(),
        };
        let mut edits = vec![pending];
        if parent_widget == WidgetKind::Users {
            if let Some(key) = relative.last() {
                if let Some(home) = home_remove_edit(&parent, key) {
                    edits.push(home);
                }
            }
        }
        edit::apply(&edits, &self.root)?;
        self.reload()
    }
}

fn home_users_assignment(text: &str, parent: &Record) -> Option<scan::SiblingAssignment> {
    let (assignments, _) = scan::sibling_assignments(text, parent.byte_start, parent.byte_end)?;
    assignments.into_iter().find(|assignment| {
        assignment.segments.len() == 2
            && matches!(
                assignment.segments[0].as_str(),
                "home-manager" | "homeManager"
            )
            && assignment.segments[1] == "users"
    })
}

fn home_add_edit(parent: &Record, key: &str) -> Option<edit::PendingEdit> {
    let text = std::fs::read_to_string(&parent.file).ok()?;
    let home = home_users_assignment(&text, parent)?;
    if nix_settings_gui::value::classify(&home.value)
        != nix_settings_gui::value::ValueKind::AttrSet
    {
        return None;
    }
    let offset = closing_offset(&home.value, nix_settings_gui::value::ValueKind::AttrSet).ok()?;
    let version = scan::scan_value_direct(&home.value)
        .into_iter()
        .find_map(|user| {
            scan::scan_value_direct(&user.value)
                .into_iter()
                .find(|field| field.segments == ["home", "stateVersion"])
                .map(|field| field.value)
        })
        .unwrap_or_else(|| "\"25.05\"".to_string());
    let attr = crate::structure::attr_path(key).ok()?;
    Some(edit::PendingEdit {
        file: parent.file.clone(),
        start: home.value_start + offset,
        end: home.value_start + offset,
        value: format!("\n  {attr} = {{ home.stateVersion = {version}; }};\n"),
    })
}

fn home_remove_edit(parent: &Record, key: &str) -> Option<edit::PendingEdit> {
    let text = std::fs::read_to_string(&parent.file).ok()?;
    let home = home_users_assignment(&text, parent)?;
    let child = scan::scan_value_direct(&home.value)
        .into_iter()
        .find(|child| child.segments == [key.to_string()])?;
    let start = home.value_start + child.byte_start;
    let end = home.value_start + child.byte_end;
    let (assignments, _) = scan::sibling_assignments(&text, start, end)?;
    let assignment = assignments
        .iter()
        .filter(|assignment| assignment.value_start <= start && assignment.value_end >= end)
        .min_by_key(|assignment| assignment.value_end - assignment.value_start)?;
    let mut start = assignment.full_start;
    let mut end = assignment.full_end;
    while start > 0 && matches!(text.as_bytes()[start - 1], b' ' | b'\t') {
        start -= 1;
    }
    while end < text.len() && matches!(text.as_bytes()[end], b' ' | b'\t' | b'\r') {
        end += 1;
    }
    if end < text.len() && text.as_bytes()[end] == b'\n' {
        end += 1;
    }
    Some(edit::PendingEdit {
        file: parent.file.clone(),
        start,
        end,
        value: String::new(),
    })
}

fn widget_name(widget: WidgetKind) -> String {
    crate::render::widget_name(widget)
}

/// Offset of the closing `}` or `]` of a composite value, relative to the
/// value text. A new row is inserted just before it.
fn closing_offset(value: &str, kind: nix_settings_gui::value::ValueKind) -> Result<usize, String> {
    use rnix::SyntaxKind;
    let (node_kind, token_kind) = match kind {
        nix_settings_gui::value::ValueKind::AttrSet => {
            (SyntaxKind::NODE_ATTR_SET, SyntaxKind::TOKEN_R_BRACE)
        }
        nix_settings_gui::value::ValueKind::List => {
            (SyntaxKind::NODE_LIST, SyntaxKind::TOKEN_R_BRACK)
        }
        _ => return Err("value is not an attribute set or list".to_string()),
    };
    let parsed = rnix::Root::parse(value);
    let root = parsed.syntax();
    let node = root
        .descendants()
        .find(|node| node.kind() == node_kind)
        .ok_or_else(|| "cannot locate the composite value".to_string())?;
    let token = node
        .children_with_tokens()
        .filter_map(|element| element.into_token())
        .find(|token| token.kind() == token_kind)
        .ok_or_else(|| "cannot locate the closing delimiter".to_string())?;
    Ok(usize::from(token.text_range().start()))
}

fn fingerprints(scanned: &scan::ScanResult) -> HashMap<String, String> {
    let mut files = HashMap::new();
    for record in scanned.menus.iter().flat_map(|menu| menu.records.iter()) {
        files
            .entry(record.file.clone())
            .or_insert_with(|| file_fingerprint(&record.file));
    }
    files
}

fn file_fingerprint(path: &str) -> String {
    match std::fs::metadata(path) {
        Ok(metadata) => {
            let modified = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_nanos())
                .unwrap_or_default();
            format!("{}:{modified}", metadata.len())
        }
        Err(_) => String::new(),
    }
}

/// Look up a `visibleWhen` governing value, tolerating the `config.` wrapper
/// the scanner can record around annotated module bodies.
fn resolve_dependency<'a>(values: &'a HashMap<String, String>, path: &str) -> Option<&'a str> {
    values
        .get(path)
        .or_else(|| values.get(&format!("config.{path}")))
        .or_else(|| path.strip_prefix("config.").and_then(|rest| values.get(rest)))
        .map(String::as_str)
}

fn validate_users(record: &Record, staged: &HashMap<String, String>) -> Option<String> {
    let users = scan::scan_value_direct(&record.value);
    if users.is_empty() {
        return Some("At least one user is required.".to_string());
    }
    let mut names = HashSet::new();
    let mut uids = HashMap::new();
    let mut admin = false;
    for user in &users {
        let name = user.segments.last()?.trim().to_string();
        if !valid_account_name(&name) {
            return Some(format!(
                "User name {name:?} is invalid. Use letters, digits, underscores, dots, and dashes, starting with a letter or underscore."
            ));
        }
        if !names.insert(name.clone()) {
            return Some(format!("User names must be unique; {name} is repeated."));
        }
        let fields = scan::scan_value_direct(&user.value);
        let field = |key: &str| -> Option<String> {
            let path = format!("{}.{}.{}", record.path(), name, key);
            staged.get(&path).cloned().or_else(|| {
                fields
                    .iter()
                    .find(|field| field.segments.len() == 1 && field.segments[0] == key)
                    .map(|field| field.value.clone())
            })
        };
        if let Some(uid) = field("uid").map(|value| value.trim().to_string()) {
            if !uid.is_empty() {
                match uid.parse::<u32>() {
                    Ok(0) | Err(_) => {
                        return Some(format!("UID for {name} must be a positive number."));
                    }
                    Ok(uid) => {
                        if let Some(other) = uids.insert(uid, name.clone()) {
                            return Some(format!("UID {uid} is used by both {other} and {name}."));
                        }
                    }
                }
            }
        }
        if let Some(shell) = field("shell") {
            let shell = shell.trim();
            if !shell.is_empty() && shell != "null" && !shell.starts_with("pkgs.") {
                return Some(format!(
                    "Shell for {name} must be a pkgs.* package expression, for example pkgs.zsh."
                ));
            }
        }
        let normal = field("isNormalUser").map(|value| value.trim() == "true").unwrap_or(false);
        let groups = field("extraGroups")
            .map(|value| {
                nix_settings_gui::value::list_items(&value)
                    .unwrap_or_default()
                    .iter()
                    .map(|item| unquote(item))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let sudo = groups.iter().any(|group| group == "wheel" || group == "sudo");
        let password = field("hashedPassword")
            .map(|value| {
                let value = value.trim();
                !value.is_empty() && value != "null" && value != "\"\""
            })
            .unwrap_or(false);
        if normal && sudo && password {
            admin = true;
        }
    }
    if !admin {
        return Some(
            "At least one normal user with sudo privileges and a configured password is required."
                .to_string(),
        );
    }
    None
}

fn valid_account_name(name: &str) -> bool {
    let mut characters = name.chars();
    match characters.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => return false,
    }
    characters.all(|character| {
        character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-')
    })
}

fn unquote(value: &str) -> String {
    let text = value.trim();
    if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') {
        text[1..text.len() - 1]
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
    } else if text.starts_with("''") && text.ends_with("''") && text.len() >= 4 {
        text[2..text.len() - 2].to_string()
    } else {
        text.to_string()
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

/// Composite widgets whose value is one attribute set per row. Their children
/// are the rows themselves, not the flattened leaves of every row.
fn is_row_collection(widget: WidgetKind) -> bool {
    matches!(
        widget,
        WidgetKind::Users
            | WidgetKind::Keybinds
            | WidgetKind::Monitors
            | WidgetKind::Docker
            | WidgetKind::Menu
            | WidgetKind::Plugins
            | WidgetKind::GridConfiguration
            | WidgetKind::HorizontalSubmodule
            | WidgetKind::Bar
            | WidgetKind::Dock
    )
}

/// A list of attribute sets, such as a bar section or dock tile list. A list
/// of plain strings or expressions keeps its list editor instead.
fn is_structured_list(record: &Record, widget: WidgetKind) -> bool {
    if widget != WidgetKind::List
        || nix_settings_gui::value::classify(&record.value)
            != nix_settings_gui::value::ValueKind::List
    {
        return false;
    }
    let parsed = rnix::Root::parse(&record.value);
    let root = parsed.syntax();
    root.descendants()
        .find(|node| node.kind() == rnix::SyntaxKind::NODE_LIST)
        .map(|list| {
            list.children().any(|node| {
                nix_settings_gui::value::classify(node.text().to_string().as_str())
                    == nix_settings_gui::value::ValueKind::AttrSet
            })
        })
        .unwrap_or(false)
}

/// Elements of a list value as records, so structured list values can be
/// edited field by field. Annotation scanning never descends into lists; this
/// helper only exposes existing elements, it does not bind `@option` comments.
fn scan_list_elements(record: &Record) -> Vec<Record> {
    use rnix::SyntaxKind;
    let parsed = rnix::Root::parse(&record.value);
    let root = parsed.syntax();
    let Some(list) = root
        .descendants()
        .find(|node| node.kind() == SyntaxKind::NODE_LIST)
    else {
        return Vec::new();
    };
    list.children()
        .enumerate()
        .map(|(index, node)| {
            let range = node.text_range();
            Record {
                segments: vec![format!("[{index}]")],
                option_path: None,
                file: record.file.clone(),
                line: 0,
                col: 0,
                byte_start: usize::from(range.start()),
                byte_end: usize::from(range.end()),
                value: node.text().to_string(),
                wrapper: None,
                origin: record.origin.clone(),
                spec: Default::default(),
                comment_span: None,
            }
        })
        .collect()
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
    fn gate_all_or_nothing_save() {
        let root = std::env::temp_dir().join(format!(
            "nix-web-gui-all-or-nothing-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(root.join("modules")).unwrap();
        let file = root.join("modules/system.nix");
        let original = "/* @menu displayname = \"Test\"; */\n{ ... }: {\n  /* @option widget = \"text\"; label = \"Hostname\"; */\n  networking.hostName = \"ok\";\n}\n";
        std::fs::write(&file, original).unwrap();

        let mut session = SettingsSession::open(root.to_str().unwrap()).unwrap();
        session
            .set("networking.hostName", "\"unterminated")
            .unwrap();
        assert!(session.apply().is_err());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), original);
        let _ = std::fs::remove_dir_all(root);
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

    #[test]
    fn skips_staged_children_hidden_by_their_parent() {
        let root = std::env::temp_dir().join(format!(
            "nix-web-gui-visible-when-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(root.join("modules")).unwrap();
        let file = root.join("modules/system.nix");
        std::fs::write(
            &file,
            "/* @menu displayname = \"Test\"; */\n{ ... }: {\n  /* @option widget = \"switch\"; label = \"Enable\"; */\n  services.demo.enable = true;\n  /* @option widget = \"text\"; label = \"Name\"; visibleWhen.path = \"services.demo.enable\"; visibleWhen.equals = true; */\n  services.demo.name = \"keep\";\n}\n",
        )
        .unwrap();

        let mut session = SettingsSession::open(root.to_str().unwrap()).unwrap();
        session.set("services.demo.enable", "false").unwrap();
        session.set("services.demo.name", "\"changed\"").unwrap();
        assert_eq!(session.apply().unwrap(), 1);

        let saved = std::fs::read_to_string(&file).unwrap();
        assert!(saved.contains("services.demo.enable = false;"));
        assert!(saved.contains("services.demo.name = \"keep\";"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_saving_over_a_file_changed_on_disk() {
        let root = std::env::temp_dir().join(format!(
            "nix-web-gui-stale-file-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(root.join("modules")).unwrap();
        let file = root.join("modules/system.nix");
        std::fs::write(
            &file,
            "/* @menu displayname = \"Test\"; */\n{ ... }: {\n  /* @option widget = \"text\"; label = \"Hostname\"; */\n  networking.hostName = \"before\";\n}\n",
        )
        .unwrap();

        let mut session = SettingsSession::open(root.to_str().unwrap()).unwrap();
        session.set("networking.hostName", "\"mine\"").unwrap();
        std::fs::write(
            &file,
            "/* @menu displayname = \"Test\"; */\n{ ... }: {\n  /* @option widget = \"text\"; label = \"Hostname\"; */\n  networking.hostName = \"changed externally\";\n}\n",
        )
        .unwrap();

        let error = session.apply().unwrap_err();
        assert!(error.contains("changed on disk"), "{error}");
        assert!(std::fs::read_to_string(&file)
            .unwrap()
            .contains("changed externally"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn adds_and_removes_composite_rows() {
        let root = std::env::temp_dir().join(format!(
            "nix-web-gui-structure-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(root.join("modules")).unwrap();
        let file = root.join("modules/system.nix");
        std::fs::write(
            &file,
            "/* @menu displayname = \"Test\"; */\n{ ... }: {\n  /* @option widget = \"users\"; label = \"Users\"; */\n  users.users = {\n    alice = { isNormalUser = true; };\n  };\n  /* @option widget = \"keybinds\"; label = \"Binds\"; */\n  keybinds = {\n    reload = { modifier = [ \"Mod4\" ]; keys = [ \"r\" ]; };\n  };\n  home-manager.users = {\n    alice = { home.stateVersion = \"24.05\"; };\n  };\n}\n",
        )
        .unwrap();

        let mut session = SettingsSession::open(root.to_str().unwrap()).unwrap();
        session.add_child("users.users", "bob", None, &[]).unwrap();
        let saved = std::fs::read_to_string(&file).unwrap();
        assert!(saved.contains("bob ="));
        assert!(saved.matches("bob =").count() >= 2);
        assert!(saved.contains("isNormalUser = true;"));

        session
            .remove_child("users.users", "users.users.bob")
            .unwrap();
        let saved = std::fs::read_to_string(&file).unwrap();
        assert!(!saved.contains("bob ="));
        assert!(saved.contains("alice ="));

        session
            .rename_child("users.users", "users.users.alice", "alicia")
            .unwrap();
        let saved = std::fs::read_to_string(&file).unwrap();
        assert!(saved.matches("alicia =").count() >= 2);
        assert!(!saved.contains("alice ="));

        session.add_child("keybinds", "terminal", None, &[]).unwrap();
        let saved = std::fs::read_to_string(&file).unwrap();
        assert!(saved.contains("terminal ="));
        assert!(saved.contains("modifier = [ ];"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn adds_a_generated_field_to_an_attribute_set_row() {
        let root = std::env::temp_dir().join(format!(
            "nix-web-gui-field-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(root.join("modules")).unwrap();
        let file = root.join("modules/system.nix");
        std::fs::write(
            &file,
            "/* @menu displayname = \"Test\"; */\n{ ... }: {\n  /* @option widget = \"users\"; label = \"Users\"; */\n  users.users = { alice = { isNormalUser = true; }; };\n}\n",
        )
        .unwrap();
        let mut session = SettingsSession::open(root.to_str().unwrap()).unwrap();
        session
            .describe_page_local(&file.display().to_string())
            .unwrap();
        session
            .add_field("users.users.alice", "shell", "pkgs.zsh")
            .unwrap();
        let saved = std::fs::read_to_string(&file).unwrap();
        assert!(saved.contains("shell = pkgs.zsh;"), "{saved}");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn adds_a_structured_list_element() {
        let root = std::env::temp_dir().join(format!(
            "nix-web-gui-list-add-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(root.join("modules")).unwrap();
        let file = root.join("modules/system.nix");
        std::fs::write(
            &file,
            "/* @menu displayname = \"Test\"; */\n{ ... }: {\n  /* @option widget = \"bar\"; label = \"Bar\"; */\n  bar = {\n    left = [ { id = \"clock\"; } ];\n  };\n}\n",
        )
        .unwrap();
        let mut session = SettingsSession::open(root.to_str().unwrap()).unwrap();
        session
            .describe_page_local(&file.display().to_string())
            .unwrap();
        session
            .add_child(
                "bar.left",
                "battery",
                None,
                &[("enabled".to_string(), "true".to_string())],
            )
            .unwrap();
        session
            .describe_page_local(&file.display().to_string())
            .unwrap();
        session
            .move_child("bar.left", "bar.left.[1]", -1)
            .unwrap();
        let saved = std::fs::read_to_string(&file).unwrap();
        assert!(saved.contains("enabled = true"), "{saved}");
        let battery = saved.find("id = \"battery\"").unwrap();
        let clock = saved.find("id = \"clock\"").unwrap();
        assert!(battery < clock, "{saved}");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn validates_the_users_invariant() {
        let root = std::env::temp_dir().join(format!(
            "nix-web-gui-users-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(root.join("modules")).unwrap();
        let file = root.join("modules/system.nix");
        std::fs::write(
            &file,
            "/* @menu displayname = \"Test\"; */\n{ ... }: {\n  /* @option widget = \"users\"; label = \"Users\"; */\n  users.users = {\n    alice = {\n      isNormalUser = true;\n      extraGroups = [ \"wheel\" \"networkmanager\" ];\n      hashedPassword = \"$y$abc\";\n    };\n  };\n  /* @option widget = \"text\"; label = \"Hostname\"; */\n  networking.hostName = \"vm\";\n}\n",
        )
        .unwrap();

        let mut session = SettingsSession::open(root.to_str().unwrap()).unwrap();
        session
            .describe_page_local(&file.display().to_string())
            .unwrap();
        assert!(session.record("users.users.alice").is_some());
        session
            .set("users.users.alice.hashedPassword", "null")
            .unwrap();
        let error = session.apply().unwrap_err();
        assert!(error.contains("sudo privileges"), "{error}");
        assert!(std::fs::read_to_string(&file).unwrap().contains("$y$abc"));

        session
            .set("users.users.alice.hashedPassword", "\"$y$ok\"")
            .unwrap();
        session.set("networking.hostName", "\"saved\"").unwrap();
        assert_eq!(session.apply().unwrap(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn describes_structured_list_elements() {
        let root = std::env::temp_dir().join(format!(
            "nix-web-gui-list-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(root.join("modules")).unwrap();
        let file = root.join("modules/system.nix");
        std::fs::write(
            &file,
            "/* @menu displayname = \"Test\"; */\n{ ... }: {\n  /* @option widget = \"bar\"; label = \"Bar\"; */\n  bar = {\n    left = [ { id = \"clock\"; } { id = \"battery\"; } ];\n    center = [ \"clock\" ];\n  };\n}\n",
        )
        .unwrap();

        let mut session = SettingsSession::open(root.to_str().unwrap()).unwrap();
        let descriptors = session.describe_page_local(&file.display().to_string()).unwrap();
        let bar = descriptors
            .iter()
            .find(|descriptor| descriptor["path"] == "bar")
            .expect("bar descriptor");
        assert!(bar["children"].is_array());
        assert!(session.record("bar.left.[0].id").is_some());
        assert!(session.record("bar.left.[1].id").is_some());
        let center = session.record("bar.center").expect("center record");
        assert!(!is_structured_list(center, WidgetKind::List));
        let _ = std::fs::remove_dir_all(root);
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
