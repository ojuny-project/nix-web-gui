use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::config::AppConfig;

pub mod session;

#[cfg(feature = "pam")]
mod pam;

const MAX_FAILURES: u32 = 5;
const FAILURE_WINDOW: Duration = Duration::from_secs(300);
const LOCKOUT: Duration = Duration::from_secs(300);

/// Per-username failed-login accounting. A successful login clears the entry.
#[derive(Default)]
pub struct LoginThrottle {
    entries: Mutex<HashMap<String, Attempt>>,
}

#[derive(Debug, Clone)]
struct Attempt {
    failures: u32,
    first: Instant,
    locked_until: Option<Instant>,
}

impl LoginThrottle {
    pub fn new() -> Self {
        Self::default()
    }

    /// Remaining lockout, or `None` when the account may attempt a login.
    pub fn check(&self, username: &str) -> Option<Duration> {
        let now = Instant::now();
        let mut entries = self.entries.lock().expect("login throttle poisoned");
        let attempt = entries.get(username).cloned()?;
        if let Some(until) = attempt.locked_until {
            if until > now {
                return Some(until - now);
            }
            entries.remove(username);
            return None;
        }
        if now.duration_since(attempt.first) > FAILURE_WINDOW {
            entries.remove(username);
        }
        None
    }

    pub fn failure(&self, username: &str) {
        let now = Instant::now();
        let mut entries = self.entries.lock().expect("login throttle poisoned");
        entries.retain(|_, attempt| {
            attempt.locked_until.map_or(true, |until| until > now)
                && now.duration_since(attempt.first) <= FAILURE_WINDOW
        });
        let attempt = entries.entry(username.to_string()).or_insert(Attempt {
            failures: 0,
            first: now,
            locked_until: None,
        });
        attempt.failures += 1;
        if attempt.failures >= MAX_FAILURES {
            attempt.locked_until = Some(now + LOCKOUT);
        }
    }

    pub fn success(&self, username: &str) {
        self.entries
            .lock()
            .expect("login throttle poisoned")
            .remove(username);
    }
}

#[derive(Debug)]
pub enum AuthError {
    InvalidCredentials,
    Denied(String),
    #[allow(dead_code)]
    Unavailable,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCredentials => formatter.write_str("invalid username or password"),
            Self::Denied(reason) => formatter.write_str(reason),
            Self::Unavailable => formatter.write_str("authentication is unavailable"),
        }
    }
}

pub fn authenticate(config: &AppConfig, username: &str, password: &str) -> Result<(), AuthError> {
    if config.dev_auth {
        return match config.dev_password.as_deref() {
            Some(expected) if password == expected => Ok(()),
            _ => Err(AuthError::InvalidCredentials),
        };
    }

    #[cfg(feature = "pam")]
    {
        pam::authenticate(&config.pam_service, username, password)?;
        if !config.allowed_groups.is_empty() {
            require_allowed_group(username, &config.allowed_groups)?;
        }
        return Ok(());
    }

    #[cfg(not(feature = "pam"))]
    {
        let _ = (username, password);
        Err(AuthError::Unavailable)
    }
}

pub fn require_allowed_group(username: &str, allowed: &[String]) -> Result<(), AuthError> {
    let user = uzers::get_user_by_name(username)
        .ok_or_else(|| AuthError::Denied("unknown device account".to_string()))?;
    let groups = user.groups().unwrap_or_default();
    let matched = groups.iter().any(|group| {
        allowed
            .iter()
            .any(|name| group.name().to_string_lossy() == name.as_str())
    });
    if matched {
        Ok(())
    } else {
        Err(AuthError::Denied(
            "account is not in an allowed group".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_auth_requires_the_configured_password() {
        let config = AppConfig {
            listen: "127.0.0.1:0".parse().unwrap(),
            static_dir: ".".into(),
            theme_file: "/tmp/theme".into(),
            flake: None,
            startup_page: None,
            pam_service: "nix-web-gui".into(),
            allowed_groups: Vec::new(),
            dev_auth: true,
            dev_password: Some("secret".into()),
            session_ttl: std::time::Duration::from_secs(60),
            max_sessions: 8,
            secure_cookie: false,
        };
        assert!(authenticate(&config, "den", "secret").is_ok());
        assert!(authenticate(&config, "den", "wrong").is_err());
    }

    #[test]
    fn locks_out_after_repeated_failures() {
        let throttle = LoginThrottle::new();
        for _ in 0..MAX_FAILURES {
            assert!(throttle.check("alice").is_none());
            throttle.failure("alice");
        }
        assert!(throttle.check("alice").is_some());
        assert!(throttle.check("bob").is_none());
        throttle.success("alice");
        assert!(throttle.check("alice").is_none());
    }
}
