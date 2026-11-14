use std::path::{Component, Path, PathBuf};

use base64::Engine as _;
use nix_settings_gui::dsl::WidgetKind;
use serde_json::{json, Value};

use crate::paths;
use crate::session::SettingsSession;

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "svg", "gif"];
const FILE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "svg", "pdf", "txt", "md"];
const MAX_UPLOAD_BYTES: usize = 32 * 1024 * 1024;

pub fn list(session: &SettingsSession, option_path: &str) -> Result<Value, String> {
    let record = session
        .record(option_path)
        .ok_or_else(|| format!("unknown option path: {option_path}"))?;
    let (directory, extensions) = media_directory(session, record)?;
    let root = session_root(session)?;
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

pub fn read(
    session: &SettingsSession,
    option_path: Option<&str>,
    path: &str,
) -> Result<Value, String> {
    let root = session_root(session)?;
    let path = paths::confine(&root, Path::new(path)).map_err(|error| error.to_string())?;
    if let Some(option_path) = option_path.filter(|value| !value.is_empty()) {
        let record = session
            .record(option_path)
            .ok_or_else(|| format!("unknown option path: {option_path}"))?;
        let (directory, _) = media_directory(session, record)?;
        let directory = directory
            .canonicalize()
            .map_err(|error| format!("cannot resolve {}: {error}", directory.display()))?;
        if !path.starts_with(&directory) {
            return Err("preview path is outside the media directory".to_string());
        }
    }
    if !path.is_file() {
        return Err("preview path is not a file".to_string());
    }
    let data = std::fs::read(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mime = mime_for(&path);
    let encoded = base64::engine::general_purpose::STANDARD.encode(data);
    Ok(json!({ "mime": mime, "data": encoded }))
}

/// Decode and write one uploaded file into the widget's media directory.
/// The file name is reduced to its final component, so a caller cannot escape
/// the directory with separators or `..`.
pub fn upload(
    session: &SettingsSession,
    option_path: &str,
    name: &str,
    encoded: &str,
) -> Result<Value, String> {
    let record = session
        .record(option_path)
        .ok_or_else(|| format!("unknown option path: {option_path}"))?;
    let (directory, extensions) = media_directory(session, record)?;
    let name = safe_file_name(name)?;
    ensure_extension(&name, &extensions)?;

    let data = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|error| format!("invalid upload data: {error}"))?;
    if data.is_empty() {
        return Err("the uploaded file is empty".to_string());
    }
    if data.len() > MAX_UPLOAD_BYTES {
        return Err(format!(
            "the uploaded file is larger than {} MiB",
            MAX_UPLOAD_BYTES / 1024 / 1024
        ));
    }

    let root = session_root(session)?;
    let target = crate::paths::confine_new(&root, &directory.join(&name))
        .map_err(|error| error.to_string())?;
    std::fs::write(&target, data)
        .map_err(|error| format!("cannot write {}: {error}", target.display()))?;
    Ok(json!({
        "name": name,
        "value": name,
        "path": target.display().to_string(),
    }))
}

/// Remove one file from the widget's media directory.
pub fn delete(session: &SettingsSession, option_path: &str, name: &str) -> Result<Value, String> {
    let record = session
        .record(option_path)
        .ok_or_else(|| format!("unknown option path: {option_path}"))?;
    let (directory, extensions) = media_directory(session, record)?;
    let name = safe_file_name(name)?;
    ensure_extension(&name, &extensions)?;

    let root = session_root(session)?;
    let directory = directory
        .canonicalize()
        .map_err(|error| format!("cannot resolve {}: {error}", directory.display()))?;
    let target = paths::confine(&root, &directory.join(&name))
        .map_err(|error| error.to_string())?;
    if !target.starts_with(&directory) {
        return Err("file is outside the media directory".to_string());
    }
    if !target.is_file() {
        return Err("media file does not exist".to_string());
    }
    std::fs::remove_file(&target)
        .map_err(|error| format!("cannot remove {}: {error}", target.display()))?;
    Ok(json!({ "deleted": name }))
}

fn session_root(session: &SettingsSession) -> Result<PathBuf, String> {
    Path::new(&session.root)
        .canonicalize()
        .map_err(|error| format!("cannot resolve root: {error}"))
}

/// The directory declared by a wallpaper, file, or icon widget, plus the
/// extensions the widget accepts. The relative directory may not be absolute
/// or contain `..`, so a module comment cannot point outside the flake.
fn media_directory(
    session: &SettingsSession,
    record: &nix_settings_gui::model::Record,
) -> Result<(PathBuf, Vec<String>), String> {
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
        Some(WidgetKind::TableAttrset | WidgetKind::TableList)
            if record
                .spec
                .column_types
                .as_ref()
                .map(|types| types.iter().any(|kind| kind == "icon" || kind == "image"))
                .unwrap_or(false) =>
        {
            (
                record
                    .spec
                    .icon_dir
                    .clone()
                    .unwrap_or_else(|| "./icons".to_string()),
                IMAGE_EXTENSIONS.iter().map(|ext| ext.to_string()).collect(),
            )
        }
        _ => return Err("option is not a media picker".to_string()),
    };

    let relative = Path::new(&relative_dir);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
    {
        return Err("media directory must stay inside the module directory".to_string());
    }

    let module_dir = Path::new(&record.file)
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(&session.root));
    let root = session_root(session)?;
    let module_dir = module_dir
        .canonicalize()
        .map_err(|error| format!("cannot resolve {}: {error}", module_dir.display()))?;
    if !module_dir.starts_with(&root) {
        return Err("module directory is outside the configuration root".to_string());
    }
    Ok((module_dir.join(relative), extensions))
}

fn safe_file_name(name: &str) -> Result<String, String> {
    let path = Path::new(name);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "invalid file name".to_string())?;
    if file_name != name || name.starts_with('.') {
        return Err("invalid file name".to_string());
    }
    Ok(file_name.to_string())
}

fn ensure_extension(name: &str, extensions: &[String]) -> Result<(), String> {
    let extension = Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if extensions.iter().any(|candidate| candidate == &extension) {
        Ok(())
    } else {
        Err(format!(
            "{} is not one of the accepted extensions: {}",
            name,
            extensions.join(", ")
        ))
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_file_names() {
        assert!(safe_file_name("../escape.png").is_err());
        assert!(safe_file_name("nested/file.png").is_err());
        assert!(safe_file_name(".hidden.png").is_err());
        assert_eq!(safe_file_name("wall.png").unwrap(), "wall.png");
    }

    #[test]
    fn enforces_the_accepted_extensions() {
        let extensions = vec!["png".to_string()];
        assert!(ensure_extension("wall.png", &extensions).is_ok());
        assert!(ensure_extension("wall.exe", &extensions).is_err());
    }
}
