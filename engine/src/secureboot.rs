use nix_settings_gui::secureboot;
use serde_json::{json, Value};

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
