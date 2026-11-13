use std::io::Write;
use std::process::{Command, Stdio};

/// Hash a plaintext password for `users.users.<name>.hashedPassword`.
///
/// Prefer `mkpasswd` with yescrypt, which matches the default NixOS hash
/// format. Fall back to `openssl passwd -6` when mkpasswd is unavailable. The
/// password travels over stdin so it never appears in the process list.
pub fn hash_password(password: &str) -> Result<String, String> {
    if password.is_empty() {
        return Err("password cannot be empty".to_string());
    }
    let mut errors = Vec::new();
    match run_hasher("mkpasswd", &["-m", "yescrypt", "-s"], password) {
        Ok(hash) => return Ok(hash),
        Err(error) => errors.push(error),
    }
    match run_hasher("openssl", &["passwd", "-6", "-stdin"], password) {
        Ok(hash) => Ok(hash),
        Err(error) => {
            errors.push(error);
            Err(errors.join("; "))
        }
    }
}

fn run_hasher(program: &str, args: &[&str], password: &str) -> Result<String, String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot run {program}: {error}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(password.as_bytes())
            .and_then(|_| stdin.write_all(b"\n"))
            .map_err(|error| format!("cannot write to {program}: {error}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|error| format!("cannot wait for {program}: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("{program} exited unsuccessfully")
        } else {
            format!("{program}: {stderr}")
        });
    }
    let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !hash.starts_with('$') || hash.len() <= 3 {
        return Err(format!("{program} did not return a password hash"));
    }
    Ok(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_an_empty_password() {
        assert!(hash_password("").is_err());
    }
}
