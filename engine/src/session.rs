use std::collections::HashMap;

use nix_settings_gui::model::Record;
use nix_settings_gui::{edit, scan};

/// One open settings root and its staged edits.
pub struct SettingsSession {
    pub root: String,
    pub scanned: scan::ScanResult,
    staged: HashMap<String, String>,
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
        })
    }

    pub fn reload(&mut self) -> Result<(), String> {
        self.scanned = scan::scan_root(&self.root)?;
        Ok(())
    }

    pub fn discard(&mut self) -> Result<(), String> {
        self.staged.clear();
        self.reload()
    }

    pub fn menus(&self) -> &[nix_settings_gui::model::MenuModel] {
        &self.scanned.menus
    }

    pub fn records(&self) -> impl Iterator<Item = &Record> {
        self.scanned
            .menus
            .iter()
            .flat_map(|menu| menu.records.iter())
    }

    pub fn record(&self, path: &str) -> Option<&Record> {
        self.records().find(|record| record.path() == path)
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

    /// Gate: annotations inside lists, map, and mkMerge must stay invisible.
    /// `lib.mkIf` is supported. A local patch that adds list descent fails here.
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
        assert!(paths.iter().any(|path| path == "time.timeZone"));
    }
}
