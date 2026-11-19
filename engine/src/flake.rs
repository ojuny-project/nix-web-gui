use std::collections::BTreeMap;
use std::path::Path;

use nix_settings_gui::flake_inputs::{self, FlakeInput};
use nix_settings_gui::flake_lock::{self, LockedInput};
use nix_settings_gui::flake_update::{self, InputState, InputStatus};
use nix_settings_gui::{edit, nixpkgs, ref_suggest};
use serde_json::{json, Value};

pub fn inputs(root: &str, declared: &[flake_inputs::DeclaredInput]) -> Result<Value, String> {
    let path = Path::new(root).join("flake.nix");
    let document = flake_inputs::load(&path.display().to_string())?;
    let list: Vec<Value> = document.inputs.iter().map(input_json).collect();
    let nixpkgs_input = document.input("nixpkgs").map(input_json);
    let locked = flake_lock::read_input_in(Path::new(root), "nixpkgs").map(|locked| locked_json(&locked));
    let missing = flake_inputs::missing_declared_inputs(declared, &document.inputs);
    Ok(json!({
        "inputs": list,
        "nixpkgs": nixpkgs_input,
        "locked": locked,
        "missing": missing.iter().map(|input| json!({
            "name": input.name,
            "url": input.url,
            "file": input.file,
            "line": input.line,
        })).collect::<Vec<_>>(),
    }))
}

fn input_json(input: &FlakeInput) -> Value {
    json!({
        "name": input.name,
        "url": input.url,
        "flake": input.flake,
        "submodules": input.submodules,
        "lfs": input.lfs,
        "follows": input.follows,
        "description": input.description,
    })
}

fn locked_json(locked: &LockedInput) -> Value {
    json!({
        "type": locked.input_type,
        "owner": locked.owner,
        "repo": locked.repo,
        "ref": locked.ref_,
        "rev": locked.rev,
        "lastModified": locked.last_modified,
        "narHash": locked.nar_hash,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn upsert(
    root: &str,
    name: &str,
    url: &str,
    flake: bool,
    submodules: bool,
    lfs: bool,
    follows: BTreeMap<String, String>,
    description: Option<String>,
) -> Result<(), String> {
    let path = Path::new(root).join("flake.nix");
    let document = flake_inputs::load(&path.display().to_string())?;
    let input = FlakeInput {
        name: name.to_string(),
        url: Some(url.to_string()),
        flake,
        submodules,
        lfs,
        follows,
        description,
    };
    let edits = document.edits_for_upsert(&input)?;
    edit::apply(&edits, root).map(|_| ())
}

pub fn delete(root: &str, name: &str) -> Result<(), String> {
    let path = Path::new(root).join("flake.nix");
    let document = flake_inputs::load(&path.display().to_string())?;
    let edits = document.edits_for_delete(name)?;
    edit::apply(&edits, root).map(|_| ())
}

/// Declare every input that is referenced by an `@inputs` marker but missing
/// from `flake.nix`. Returns the added names.
pub fn add_all(
    root: &str,
    declared: &[flake_inputs::DeclaredInput],
) -> Result<Vec<String>, String> {
    let path = Path::new(root).join("flake.nix");
    let document = flake_inputs::load(&path.display().to_string())?;
    let missing = flake_inputs::missing_declared_inputs(declared, &document.inputs);
    if missing.is_empty() {
        return Ok(Vec::new());
    }
    let inputs: Vec<FlakeInput> = missing
        .iter()
        .map(|declared| FlakeInput {
            url: Some(declared.url.clone()),
            ..FlakeInput::new(&declared.name)
        })
        .collect();
    let edits = document.edits_for_add_all(&inputs)?;
    edit::apply(&edits, root)?;
    Ok(missing.into_iter().map(|input| input.name).collect())
}

pub fn channels() -> Vec<String> {
    nixpkgs::channels()
}

pub struct Suggest<'a> {
    pub kind: &'a str,
    pub forge: &'a str,
    pub host: Option<&'a str>,
    pub query: &'a str,
    pub owner: &'a str,
    pub repo: &'a str,
    pub branch: &'a str,
}

pub fn suggest(request: Suggest<'_>) -> Result<Vec<String>, String> {
    match request.kind {
        "channels" => nixpkgs::scan_channels(),
        "registry" => ref_suggest::scan_registry(),
        "users" => ref_suggest::scan_users(request.forge, request.query),
        "repos" => ref_suggest::scan_repos(request.forge, request.host, request.owner),
        "tags" => ref_suggest::scan_refs(
            request.forge,
            request.host,
            request.owner,
            request.repo,
            true,
        ),
        "refs" => ref_suggest::scan_refs(
            request.forge,
            request.host,
            request.owner,
            request.repo,
            false,
        ),
        "commits" => ref_suggest::scan_commits(
            request.forge,
            request.host,
            request.owner,
            request.repo,
            request.branch,
        ),
        other => Err(format!("unknown scan kind: {other}")),
    }
}

pub fn check_updates(root: &str, names: &[String]) -> Result<Value, String> {
    check_updates_with(root, names, |_, _, _| {})
}

pub fn check_updates_with<F>(
    root: &str,
    names: &[String],
    mut progress: F,
) -> Result<Value, String>
where
    F: FnMut(&str, Option<&InputStatus>, Option<&str>),
{
    let path = Path::new(root).join("flake.nix");
    let document = flake_inputs::load(&path.display().to_string())?;
    let mut out = Vec::new();
    for name in names {
        let Some(input) = document.input(name) else {
            progress(name, None, Some("input is not declared"));
            out.push(json!({ "name": name, "error": "input is not declared" }));
            continue;
        };
        match flake_update::check(root, input) {
            Ok(status) => {
                progress(name, Some(&status), None);
                out.push(status_json(name, &status));
            }
            Err(error) => {
                progress(name, None, Some(&error));
                out.push(json!({ "name": name, "error": error }));
            }
        }
    }
    Ok(Value::Array(out))
}

fn status_json(name: &str, status: &InputStatus) -> Value {
    let (state, reason) = match &status.state {
        InputState::UpToDate => ("up-to-date", None),
        InputState::UpdateAvailable => ("update-available", None),
        InputState::NotLocked => ("not-locked", None),
        InputState::Skipped(reason) => ("skipped", Some(reason.clone())),
    };
    json!({
        "name": name,
        "state": state,
        "reason": reason,
        "currentRev": status.current_rev,
        "currentDate": status.current_date,
        "remoteRev": status.remote_rev,
        "remoteDate": status.remote_date,
    })
}

pub fn update(root: &str, names: &[String]) -> Result<usize, String> {
    if names.len() == 1 {
        flake_update::update(root, &names[0])?;
        Ok(1)
    } else {
        flake_update::update_all(root, names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_inputs_from_the_fixture() {
        let root = format!(
            "{}/../tests/fixtures/plain-flake",
            env!("CARGO_MANIFEST_DIR")
        );
        let value = inputs(&root, &[]).unwrap();
        assert!(value["inputs"].is_array());
    }
}
