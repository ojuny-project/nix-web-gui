use std::path::{Path, PathBuf};

use nix_settings_gui::templates::{self, Template};
use serde_json::{json, Value};

use crate::paths;

pub fn load_templates(root: &str) -> Vec<Template> {
    templates::load_templates(root)
}

pub fn render_template(template: &Template, values: &[(String, String)]) -> String {
    templates::render(&template.body, values)
}

pub fn render_body(body: &str, values: &[(String, String)]) -> String {
    templates::render(body, values)
}

pub fn templates_json(root: &str) -> Value {
    let templates = load_templates(root);
    Value::Array(
        templates
            .iter()
            .enumerate()
            .map(|(index, template)| {
                json!({
                    "index": index,
                    "label": template.label,
                    "kind": template.kind,
                    "filename": template.filename,
                    "output": template.output,
                    "placeholders": template.placeholders.iter().map(|placeholder| json!({
                        "name": placeholder.name,
                        "label": placeholder.label,
                        "default": placeholder.default,
                        "hint": placeholder.hint,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect(),
    )
}

/// Build a new module file. Raw files are plain functions for non-flake-parts
/// use; NixOS and home-manager files wrap their body in the flake-parts
/// `flake.modules.<kind>.<name>` container.
pub fn template_for(displayname: &str, kind: &str, module_name: &str, path: &Path) -> String {
    let header = format!("/* @menu\n   displayname = \"{displayname}\";\n   order = 1000;\n*/\n\n");
    let name = if module_name.trim().is_empty() {
        path.file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| "module".into())
    } else {
        module_name.trim().to_string()
    };
    let body = match kind {
        "raw" => "{ ... }:\n\n{\n}\n".to_string(),
        "home" => format!(
            "{{ self, inputs, ... }}:\n\n{{\n\nflake.modules.homeManager.{name} = {{ pkgs, ... }}: {{\n\n}};\n}}\n"
        ),
        _ => format!(
            "{{ self, inputs, ... }}:\n\n{{\n\nflake.modules.nixos.{name} = {{ pkgs, ... }}: {{\n\n}};\n}}\n"
        ),
    };
    format!("{header}{body}")
}

pub fn write_new(root: &str, path: &str, body: &str) -> Result<PathBuf, String> {
    let path = paths::confine_new(Path::new(root), Path::new(path)).map_err(|error| error.to_string())?;
    if path.exists() {
        return Err(format!("{} already exists", path.display()));
    }
    std::fs::write(&path, body).map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    Ok(path)
}

pub fn read_source(root: &str, path: &str) -> Result<String, String> {
    let path = paths::confine(Path::new(root), Path::new(path))
        .map_err(|error| error.to_string())?;
    if path.extension().and_then(|extension| extension.to_str()) != Some("nix") {
        return Err("the editor only opens .nix files".to_string());
    }
    std::fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))
}

pub fn write_source(root: &str, path: &str, text: &str) -> Result<(), String> {
    let path = paths::confine(Path::new(root), Path::new(path))
        .map_err(|error| error.to_string())?;
    if path.extension().and_then(|extension| extension.to_str()) != Some("nix") {
        return Err("the editor only writes .nix files".to_string());
    }
    std::fs::write(&path, text)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_new_files_outside_the_root() {
        let root = std::env::temp_dir().join(format!(
            "nix-web-gui-authoring-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let error = write_new(root.to_str().unwrap(), "../escape.nix", "{}").unwrap_err();
        assert!(error.contains("outside"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn writes_kind_specific_templates() {
        let path = Path::new("/tmp/audio.nix");
        let raw = template_for("audio", "raw", "", path);
        assert!(raw.contains("{ ... }:"));
        assert!(!raw.contains("flake.modules"));

        let nixos = template_for("audio", "nixos", "audio", path);
        assert!(nixos.contains("flake.modules.nixos.audio"));

        let home = template_for("audio", "home", "shell", path);
        assert!(home.contains("flake.modules.homeManager.shell"));
    }

    #[test]
    fn reads_and_writes_sources_inside_the_root() {
        let root = std::env::temp_dir().join(format!(
            "nix-web-gui-editor-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("module.nix");
        std::fs::write(&file, "{ ... }: { }\n").unwrap();
        let path = file.to_str().unwrap();
        assert!(read_source(root.to_str().unwrap(), path)
            .unwrap()
            .contains("{ ... }"));
        write_source(root.to_str().unwrap(), path, "{ ... }: { edited = true; }\n").unwrap();
        assert!(read_source(root.to_str().unwrap(), path)
            .unwrap()
            .contains("edited"));
        assert!(read_source(root.to_str().unwrap(), "/etc/passwd").is_err());
        let _ = std::fs::remove_dir_all(root);
    }
}
