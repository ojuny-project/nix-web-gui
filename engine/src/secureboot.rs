use nix_settings_gui::secureboot;
use serde_json::{json, Value};

use crate::session::SettingsSession;

/// Collect the live Secure Boot state and derive the next setup stage.
pub fn status(configured: bool) -> Value {
    let state = secureboot::collect_state();
    let stage = secureboot::stage(&state, configured);
    json!({
        "state": state,
        "stage": {
            "title": stage.title(),
            "detail": stage.detail(),
        },
    })
}

pub fn config_block() -> String {
    secureboot::config_block()
}

/// Write the missing parts of the Lanzaboote configuration into the module
/// that owns the Secure Boot toggle. Assignments that already exist are left
/// untouched so a user's overrides survive. Returns the inserted lines.
pub fn apply_config(
    session: &mut SettingsSession,
    file: &str,
    origin: &str,
) -> Result<Vec<String>, String> {
    let lines = secureboot::config_lines();
    let missing: Vec<String> = lines
        .into_iter()
        .filter(|line| {
            let path = line.split(" = ").next().unwrap_or_default().trim();
            session.record(path).is_none()
        })
        .collect();
    if missing.is_empty() {
        return Ok(Vec::new());
    }
    if !crate::paths::is_editable(std::path::Path::new(&session.root), std::path::Path::new(file)) {
        return Err(format!("cannot edit {file}: outside the configuration root"));
    }
    let block = missing.join("\n");
    nix_settings_gui::edit::insert_assignment(file, origin, &block)?;
    session.reload()?;
    Ok(missing)
}

fn effective_uid() -> u32 {
    // SAFETY: geteuid has no preconditions and does not touch memory.
    unsafe { libc::geteuid() }
}

/// Create, migrate, or enroll the Secure Boot key bundle. Only the fixed
/// commands built by the engine are executed, never browser-supplied text.
pub fn setup(
    key_action: &str,
    include_microsoft: bool,
    include_firmware_builtin: bool,
) -> Result<Value, String> {
    if effective_uid() != 0 {
        return Err("Secure Boot setup requires the root service account".to_string());
    }
    let options = secureboot::SetupOptions {
        key_action: match key_action {
            "create" => secureboot::KeyAction::Create,
            "migrate" => secureboot::KeyAction::Migrate,
            _ => secureboot::KeyAction::Keep,
        },
        include_microsoft,
        include_firmware_builtin,
    };
    let state = secureboot::collect_state();
    if let Some(error) = options.validation_error(&state) {
        return Err(error);
    }
    let stage = secureboot::stage(&state, true);
    let commands = secureboot::setup_commands(&options, &state);
    if commands.is_empty() {
        return Err(format!(
            "nothing to do for {} at this stage; follow the firmware step instead",
            stage.title()
        ));
    }
    let mut output = Vec::new();
    for command in &commands {
        output.push(format!("$ {command}"));
        output.push(crate::apply::run_shell("/", command)?);
    }
    if stage == secureboot::SetupStage::Enroll {
        let marker = secureboot::enrollment_marker_command()?;
        crate::apply::run_shell("/", &marker)?;
        output.push("Recorded the enrollment marker.".to_string());
    }
    let state = secureboot::collect_state();
    let next = secureboot::stage(&state, true);
    Ok(json!({
        "output": output,
        "stage": { "title": next.title(), "detail": next.detail() },
    }))
}

/// Run the configured rebuild command as the root service account, forwarding
/// every output line so the panel can show progress while the command runs.
pub fn rebuild_streaming(root: &str, on_line: impl FnMut(&str)) -> Result<String, String> {
    if effective_uid() != 0 {
        return Err("rebuilding from the Secure Boot flow requires the root service account".to_string());
    }
    crate::apply::rebuild_streaming(root, None, on_line)
}

pub fn rebuild(root: &str) -> Result<String, String> {
    rebuild_streaming(root, |_| {})
}

/// Reboot the machine after an explicit confirmation from the browser.
pub fn reboot() -> Result<(), String> {
    if effective_uid() != 0 {
        return Err("reboot requires the root service account".to_string());
    }
    crate::apply::run_shell("/", "systemctl reboot").map(|_| ())
}
