use std::os::unix::io::{AsRawFd, FromRawFd};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};

use axum::extract::ws::{Message, WebSocket};
use serde::Deserialize;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uzers::os::unix::UserExt;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum ClientMessage {
    Command { command: String, root: Option<String> },
    Input { data: String },
    Resize { cols: u16, rows: u16 },
}

pub struct Terminal {
    master: File,
    child: Child,
}

impl Terminal {
    pub fn spawn(
        command: &str,
        root: Option<&str>,
        username: &str,
        drop_privileges: bool,
    ) -> Result<Self, String> {
        if command.trim().is_empty() {
            return Err("a command is required".to_string());
        }
        let mut master: libc::c_int = -1;
        let mut slave: libc::c_int = -1;
        let mut size = libc::winsize {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        // SAFETY: openpty writes both descriptors and does not retain the
        // pointer to the initial window size.
        let opened = unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null(),
                &mut size,
            )
        };
        if opened != 0 {
            return Err(format!(
                "cannot allocate a terminal: {}",
                std::io::Error::last_os_error()
            ));
        }

        let user = uzers::get_user_by_name(username)
            .ok_or_else(|| format!("unknown device account: {username}"))?;
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
        let home = user.home_dir().to_path_buf();
        let workdir = root
            .filter(|path| std::path::Path::new(path).is_dir())
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| home.clone());

        // SAFETY: the three descriptors are independent duplicates of the
        // slave side. They are owned by the child command after spawn.
        let stdin = unsafe { Stdio::from_raw_fd(slave) };
        let stdout = unsafe { Stdio::from_raw_fd(libc::dup(slave)) };
        let stderr = unsafe { Stdio::from_raw_fd(libc::dup(slave)) };
        let mut command_builder = Command::new("sh");
        command_builder
            .arg("-lc")
            .arg(command)
            .current_dir(&workdir)
            .stdin(stdin)
            .stdout(stdout)
            .stderr(stderr)
            .env("TERM", "xterm-256color")
            .env("COLORTERM", "truecolor")
            .env("HOME", &home)
            .env("USER", username)
            .env("LOGNAME", username)
            .env("XDG_CONFIG_HOME", home.join(".config"))
            .env("XDG_STATE_HOME", home.join(".local/state"));
        if drop_privileges {
            unsafe {
                command_builder.pre_exec(move || {
                    // SAFETY: called once in the forked child before exec.
                    if libc::setsid() < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    if libc::ioctl(0, libc::TIOCSCTTY, 0) < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
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
        } else {
            // SAFETY: called once in the forked child before exec.
            unsafe {
                command_builder.pre_exec(move || {
                    if libc::setsid() < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    if libc::ioctl(0, libc::TIOCSCTTY, 0) < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
        }
        let child = command_builder
            .spawn()
            .map_err(|error| format!("cannot start the terminal command: {error}"))?;
        // SAFETY: the master side is not used by the child and is owned here.
        let master_file = unsafe { std::fs::File::from_raw_fd(master) };
        Ok(Self {
            master: File::from_std(master_file),
            child,
        })
    }

    pub fn resize_fd(fd: i32, cols: u16, rows: u16) -> Result<(), String> {
        let size = libc::winsize {
            ws_row: rows.max(1),
            ws_col: cols.max(1),
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        // SAFETY: the master descriptor is live and the pointer is valid for
        // the duration of the call.
        let result = unsafe { libc::ioctl(fd, libc::TIOCSWINSZ, &size) };
        if result == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error().to_string())
        }
    }

    pub async fn run(self, mut socket: WebSocket) {
        let Terminal {
            master,
            mut child,
        } = self;
        let master_fd = master.as_raw_fd();
        let (mut reader, mut writer) = tokio::io::split(master);
        let mut buffer = [0u8; 8192];
        loop {
            tokio::select! {
                incoming = socket.recv() => {
                    match incoming {
                        Some(Ok(Message::Text(text))) => {
                            match serde_json::from_str::<ClientMessage>(text.as_str()) {
                                Ok(ClientMessage::Input { data }) => {
                                    if writer.write_all(data.as_bytes()).await.is_err() {
                                        break;
                                    }
                                }
                                Ok(ClientMessage::Resize { cols, rows }) => {
                                    let _ = Self::resize_fd(master_fd, cols, rows);
                                }
                                Ok(ClientMessage::Command { .. }) => {
                                    let _ = socket.send(Message::Text(
                                        serde_json::json!({
                                            "type": "error",
                                            "message": "a terminal command is already running"
                                        })
                                        .to_string()
                                        .into(),
                                    )).await;
                                }
                                Err(error) => {
                                    let _ = socket.send(Message::Text(
                                        serde_json::json!({
                                            "type": "error",
                                            "message": format!("invalid terminal message: {error}")
                                        })
                                        .to_string()
                                        .into(),
                                    )).await;
                                }
                            }
                        }
                        Some(Ok(Message::Close(_))) | None => break,
                        Some(Ok(_)) => {}
                        Some(Err(_)) => break,
                    }
                }
                read = reader.read(&mut buffer) => {
                    match read {
                        Ok(0) => break,
                        Ok(count) => {
                            if socket
                                .send(Message::Binary(buffer[..count].to_vec().into()))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
        }
        let _ = child.kill();
        let _ = child.wait();
    }
}

pub async fn handle_socket(
    mut socket: WebSocket,
    username: String,
    dev_auth: bool,
) -> Result<(), String> {
    let first = tokio::time::timeout(std::time::Duration::from_secs(30), socket.recv())
        .await
        .map_err(|_| "the terminal command was not sent".to_string())?
        .ok_or_else(|| "the terminal closed before a command arrived".to_string())?
        .map_err(|error| format!("cannot read the terminal command: {error}"))?;
    let Message::Text(text) = first else {
        return Err("send the terminal command as JSON text".to_string());
    };
    let ClientMessage::Command { command, root } = serde_json::from_str::<ClientMessage>(&text)
        .map_err(|error| format!("invalid terminal command: {error}"))?
    else {
        return Err("the first terminal message must be a command".to_string());
    };
    let terminal = Terminal::spawn(&command, root.as_deref(), &username, !dev_auth)?;
    terminal.run(socket).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;

    #[tokio::test]
    async fn streams_command_output_through_a_pty() {
        let username = uzers::get_current_username()
            .and_then(|name| name.into_string().ok())
            .unwrap_or_else(|| "root".to_string());
        let mut terminal =
            Terminal::spawn("printf terminal-ok", Some("."), &username, false)
                .expect("terminal spawn");
        let mut buffer = vec![0u8; 4096];
        let read = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            terminal.master.read(&mut buffer),
        )
        .await
        .expect("terminal output timed out")
        .expect("terminal read");
        let output = String::from_utf8_lossy(&buffer[..read]);
        assert!(output.contains("terminal-ok"), "{output}");
        let _ = terminal.child.kill();
        let _ = terminal.child.wait();
    }
}
