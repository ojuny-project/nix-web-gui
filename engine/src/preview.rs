use std::path::{Path, PathBuf};

use base64::Engine as _;
use nix_settings_gui::dsl::WidgetKind;
use serde_json::{json, Value};

use crate::paths;
use crate::session::SettingsSession;

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "svg", "gif"];
const FILE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "svg", "pdf", "txt", "md"];

pub fn list(session: &SettingsSession, option_path: &str) -> Result<Value, String> {
    let record = session
        .record(option_path)
        .ok_or_else(|| format!("unknown option path: {option_path}"))?;
    let (relative_dir, extensions): (String, Vec<String>) = match record.spec.widget {
        Some(WidgetKind::Wallpaper) => (
            record
                .spec
                .wallpaper_dir
                .clone()
                .unwrap_or_else(|| "./wallpapers".to_string()),
            IMAGE_EXTENSIONS.iter().map(|ext| ext.to_string()).collect(),
        ),
        Some(WidgetKind::File) => (
            record
                .spec
                .file_dir
                .clone()
                .unwrap_or_else(|| "./files".to_string()),
            record
                .spec
                .file_extensions
                .clone()
                .unwrap_or_else(|| FILE_EXTENSIONS.iter().map(|ext| ext.to_string()).collect()),
        ),
        _ => return Err("option is not a media picker".to_string()),
    };

    let module_dir = Path::new(&record.file)
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(&session.root));
    let directory = module_dir.join(relative_dir);
    let root = Path::new(&session.root)
        .canonicalize()
        .map_err(|error| format!("cannot resolve root: {error}"))?;
    let directory = directory
        .canonicalize()
        .map_err(|error| format!("cannot resolve {}: {error}", directory.display()))?;
    if !directory.starts_with(&root) {
        return Err("media directory is outside the configuration root".to_string());
    }

    let mut files = Vec::new();
    let entries = std::fs::read_dir(&directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !extensions.iter().any(|candidate| candidate == &extension) {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        let value = path
            .strip_prefix(&directory)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        files.push(json!({
            "name": name,
            "value": value,
            "path": path.display().to_string(),
        }));
    }
    files.sort_by(|left, right| {
        left["name"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["name"].as_str().unwrap_or_default())
    });

    Ok(json!({ "files": files }))
}

pub fn read(session: &SettingsSession, path: &str) -> Result<Value, String> {
    let path = paths::confine(Path::new(&session.root), Path::new(path))
        .map_err(|error| error.to_string())?;
    if !path.is_file() {
        return Err("preview path is not a file".to_string());
    }
    let data = std::fs::read(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mime = mime_for(&path);
    let encoded = base64::engine::general_purpose::STANDARD.encode(data);
    Ok(json!({ "mime": mime, "data": encoded }))
}

fn mime_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "txt" | "md" => "text/plain",
        _ => "application/octet-stream",
    }
}
