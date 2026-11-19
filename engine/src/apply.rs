use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc;

use nix_settings_gui::apply::{plain_value, substitute};
use nix_settings_gui::model::Record;

use crate::config::WebConfig;

/// Run a shell command in the flake root. The returned string is stdout on
/// success. On failure the first non-empty stderr line is returned.
pub fn run_shell(root: &str, command: &str) -> Result<String, String> {
    let output = Command::new("sh")
        .arg("-lc")
        .arg(command)
        .current_dir(root)
        .output()
        .map_err(|error| format!("cannot run command: {error}"))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let first = stderr
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("");
    if first.trim().is_empty() {
        Err(format!("command failed: {command}"))
    } else {
        Err(first.trim().to_string())
    }
}

/// Run a shell command in the flake root, forwarding every stdout and stderr
/// line to the callback. The returned string is the complete combined output.
/// On failure the first non-empty stderr line is returned.
pub fn run_shell_streaming(
    root: &str,
    command: &str,
    mut on_line: impl FnMut(&str),
) -> Result<String, String> {
    let mut child = Command::new("sh")
        .arg("-lc")
        .arg(command)
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot run command: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "cannot capture command output".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "cannot capture command output".to_string())?;
    let (sender, receiver) = mpsc::channel::<(bool, String)>();
    let stdout_reader = {
        let sender = sender.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let _ = sender.send((false, line));
            }
        })
    };
    let stderr_reader = std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            let _ = sender.send((true, line));
        }
    });
    let mut output = String::new();
    let mut first_error: Option<String> = None;
    for (is_stderr, line) in receiver {
        if is_stderr && first_error.is_none() && !line.trim().is_empty() {
            first_error = Some(line.clone());
        }
        on_line(&line);
        output.push_str(&line);
        output.push('\n');
    }
    let _ = stdout_reader.join();
    let _ = stderr_reader.join();
    let status = child
        .wait()
        .map_err(|error| format!("cannot wait for command: {error}"))?;
    if status.success() {
        Ok(output)
    } else if let Some(first) = first_error {
        Err(first.trim().to_string())
    } else {
        Err(format!("command failed: {command}"))
    }
}

/// Apply one option's `applyCommand` with the new Nix literal, forwarding each
/// output line to the callback.
pub fn run_apply(
    root: &str,
    record: &Record,
    literal: &str,
    on_line: impl FnMut(&str),
) -> Result<(), String> {
    let command = record
        .spec
        .apply_command
        .as_deref()
        .ok_or_else(|| "this option has no apply command".to_string())?;
    let value = plain_value(literal);
    run_shell_streaming(root, &substitute(command, &value), on_line).map(|_| ())
}

/// Append the optional compositor reloads to the configured rebuild command.
pub fn with_reloads(command: String, reload_mango: bool, reload_dms: bool) -> String {
    let mut command = command;
    if reload_mango {
        command.push_str(" && mmsg dispatch reload_config");
    }
    if reload_dms {
        command.push_str(" && dms kill && mmsg dispatch spawn_shell,dms run");
    }
    command
}

/// Load the shared desktop config and run its rebuild command through a shell,
/// forwarding each output line. The web frontend cannot open a terminal, so
/// terminal_command is ignored.
pub fn rebuild_streaming(
    root: &str,
    password: Option<&str>,
    mut on_line: impl FnMut(&str),
) -> Result<String, String> {
    let config = WebConfig::load();
    let command = config
        .rebuild_command
        .clone()
        .ok_or_else(|| "no rebuild command configured".to_string())?;
    let command = with_reloads(
        command,
        config.reload_mango_on_rebuild,
        config.reload_dms_on_rebuild,
    );
    let (command, password) = match password.filter(|password| !password.is_empty()) {
        Some(password) if command.contains("sudo ") => (
            command.replacen("sudo ", "sudo -S -p '' ", 1),
            Some(password),
        ),
        _ => (command, None),
    };
    let mut child = Command::new("sh")
        .arg("-lc")
        .arg(format!("{command} 2>&1"))
        .current_dir(root)
        .stdin(if password.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot run rebuild command: {error}"))?;
    if let (Some(password), Some(mut stdin)) = (password, child.stdin.take()) {
        use std::io::Write;
        stdin
            .write_all(password.as_bytes())
            .and_then(|_| stdin.write_all(b"\n"))
            .map_err(|error| format!("cannot send the sudo password: {error}"))?;
    }
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "cannot capture rebuild output".to_string())?;
    let mut output = String::new();
    let mut last_line = String::new();
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        on_line(&line);
        last_line = line.clone();
        output.push_str(&line);
        output.push('\n');
    }
    let status = child
        .wait()
        .map_err(|error| format!("cannot wait for rebuild: {error}"))?;
    if status.success() {
        Ok(output)
    } else if last_line.trim().is_empty() {
        Err(format!("rebuild command failed: {command}"))
    } else {
        Err(format!("rebuild command failed: {}", last_line.trim()))
    }
}

pub fn rebuild(root: &str) -> Result<String, String> {
    rebuild_streaming(root, None, |_| {})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_a_shell_command_in_the_root() {
        let root = std::env::temp_dir();
        let output = run_shell(root.to_str().unwrap(), "printf hello").unwrap();
        assert_eq!(output, "hello");
    }

    #[test]
    fn reports_the_first_error_line() {
        let root = std::env::temp_dir();
        let error = run_shell(root.to_str().unwrap(), "echo first >&2; echo second >&2; exit 1")
            .unwrap_err();
        assert_eq!(error, "first");
    }

    #[test]
    fn appends_optional_reloads() {
        let command = with_reloads("true".to_string(), true, true);
        assert!(command.contains("mmsg dispatch reload_config"));
        assert!(command.contains("dms kill"));
    }

    #[test]
    fn streams_stdout_and_stderr_lines() {
        let root = std::env::temp_dir();
        let mut lines = Vec::new();
        let output = run_shell_streaming(
            root.to_str().unwrap(),
            "printf 'one\\ntwo\\n'; printf 'err\\n' >&2",
            |line| lines.push(line.to_string()),
        )
        .unwrap();
        assert!(lines.contains(&"one".to_string()));
        assert!(lines.contains(&"two".to_string()));
        assert!(lines.contains(&"err".to_string()));
        assert!(output.contains("one"));
        assert!(output.contains("two"));
        assert!(output.contains("err"));
    }

    #[test]
    fn streaming_reports_the_first_error_line() {
        let root = std::env::temp_dir();
        let error = run_shell_streaming(
            root.to_str().unwrap(),
            "echo first >&2; echo second >&2; exit 1",
            |_| {},
        )
        .unwrap_err();
        assert_eq!(error, "first");
    }
}
