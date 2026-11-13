use std::os::unix::process::CommandExt;
use std::process::Stdio;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use uzers::os::unix::UserExt;

pub struct Worker {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Worker {
    pub fn spawn(username: &str) -> Result<Self, String> {
        let user = uzers::get_user_by_name(username)
            .ok_or_else(|| format!("unknown device account: {username}"))?;
        let executable = std::env::current_exe()
            .map_err(|error| format!("cannot locate the server binary: {error}"))?;

        let mut command = std::process::Command::new(executable);
        command
            .arg("worker")
            .current_dir(user.home_dir())
            .env("HOME", user.home_dir())
            .env("USER", username)
            .env("LOGNAME", username)
            .env("XDG_CONFIG_HOME", user.home_dir().join(".config"))
            .env("XDG_STATE_HOME", user.home_dir().join(".local/state"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        let uid = user.uid();
        let gid = user.primary_group_id();
        let mut groups: Vec<u32> = user
            .groups()
            .unwrap_or_default()
            .iter()
            .map(|group| group.gid())
            .collect();
        if !groups.contains(&gid) {
            groups.push(gid);
        }
        unsafe {
            command.pre_exec(move || {
                if libc::setgroups(groups.len(), groups.as_ptr()) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::setgid(gid) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::setuid(uid) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }

        let mut child = Command::from(command)
            .spawn()
            .map_err(|error| format!("cannot start the settings worker: {error}"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "worker stdin is unavailable".to_string())?;
        let stdout = BufReader::new(
            child
                .stdout
                .take()
                .ok_or_else(|| "worker stdout is unavailable".to_string())?,
        );

        Ok(Self {
            child,
            stdin,
            stdout,
        })
    }

    pub async fn request(&mut self, request: &Value) -> Result<Value, String> {
        let mut line = serde_json::to_string(request)
            .map_err(|error| format!("cannot encode worker request: {error}"))?;
        line.push('\n');
        self.stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|error| format!("cannot write to the worker: {error}"))?;
        self.stdin
            .flush()
            .await
            .map_err(|error| format!("cannot flush the worker: {error}"))?;

        let mut response = String::new();
        let read = self
            .stdout
            .read_line(&mut response)
            .await
            .map_err(|error| format!("cannot read from the worker: {error}"))?;
        if read == 0 {
            return Err("the settings worker exited".to_string());
        }
        serde_json::from_str(&response)
            .map_err(|error| format!("invalid worker response: {error}"))
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}
