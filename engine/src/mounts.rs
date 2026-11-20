//! Mount candidates and terminal command helpers for the web mounts widget.

use serde_json::{json, Value};

use crate::session::SettingsSession;

/// The `mkfs` command for a type, or `None` when the type has no formatter.
pub fn mkfs_command(fs_type: &str, device: &str) -> Option<String> {
    nix_settings_gui::mounts::fs::mkfs_command(fs_type, device)
}

/// Add one mount row through the session's structure insert.
pub fn add(
    session: &mut SettingsSession,
    path: &str,
    mountpoint: &str,
    device: &str,
    fs_type: &str,
    options: &[String],
) -> Result<(), String> {
    nix_settings_gui::mounts::paths::check_target(mountpoint)?;
    let mut options = options.to_vec();
    let requirements = nix_settings_gui::mounts::fs::requirements(fs_type);
    if requirements.contains(&nix_settings_gui::mounts::fs::Requirement::Owner) {
        if !options.iter().any(|option| option.starts_with("uid=")) {
            // SAFETY: geteuid and getegid have no preconditions.
            options.push(format!("uid={}", unsafe { libc::geteuid() }));
        }
        if !options.iter().any(|option| option.starts_with("gid=")) {
            options.push(format!("gid={}", unsafe { libc::getegid() }));
        }
        if !options.iter().any(|option| option.starts_with("umask=")) {
            options.push("umask=0022".to_string());
        }
    }
    let options: Vec<String> = options
        .iter()
        .map(|option| crate::render::quote(option))
        .collect();
    let mut defaults = vec![
        ("device".to_string(), crate::render::quote(device)),
        ("fsType".to_string(), crate::render::quote(fs_type)),
        ("options".to_string(), format!("[ {} ]", options.join(" "))),
    ];
    if !nix_settings_gui::mounts::fs::fsck(fs_type) {
        defaults.push(("fsck".to_string(), "false".to_string()));
    }
    session.add_child(path, mountpoint, None, &defaults)
}

/// Detected internal disks, mountable partitions and unformatted partitions.
pub fn candidates() -> Value {
    let Some(devices) = nix_settings_gui::mounts::detect::probe() else {
        return json!({ "partitions": [], "disks": [], "unformatted": [] });
    };

    let partitions: Vec<Value> = nix_settings_gui::mounts::detect::internal_candidates(&devices)
        .into_iter()
        .map(|candidate| {
            let requirements = nix_settings_gui::mounts::fs::requirements(&candidate.fstype);
            let device = candidate
                .uuid
                .as_ref()
                .map(|uuid| format!("/dev/disk/by-uuid/{uuid}"))
                .unwrap_or_else(|| candidate.path.clone());
            json!({
                "name": candidate.name,
                "path": candidate.path,
                "uuid": candidate.uuid,
                "fstype": candidate.fstype,
                "size": candidate.size,
                "model": candidate.model,
                "mountpoint": format!("/mnt/{}", candidate.name),
                "device": device,
                "defaultOptions": nix_settings_gui::mounts::fs::default_options(&candidate.fstype),
                "needsOwner": requirements.contains(&nix_settings_gui::mounts::fs::Requirement::Owner),
                "needsSubvolume": requirements.contains(&nix_settings_gui::mounts::fs::Requirement::Subvolume),
            })
        })
        .collect();

    let disks: Vec<Value> = nix_settings_gui::mounts::detect::internal_disks(&devices)
        .into_iter()
        .map(|disk| {
            json!({
                "name": disk.name,
                "path": disk.path,
                "size": disk.size,
                "model": disk.model,
            })
        })
        .collect();

    let unformatted: Vec<Value> = nix_settings_gui::mounts::detect::unformatted_partitions(&devices)
        .into_iter()
        .map(|part| {
            json!({
                "name": part.name,
                "path": part.path,
                "size": part.size,
                "model": part.model,
            })
        })
        .collect();

    json!({
        "partitions": partitions,
        "disks": disks,
        "unformatted": unformatted,
    })
}
