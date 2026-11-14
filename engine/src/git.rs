use std::process::Command;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    pub branch: String,
    pub upstream: Option<String>,
    pub dirty: bool,
    pub ahead: u64,
    pub behind: u64,
    pub state: String,
}

#[derive(Debug, PartialEq, Eq)]
enum RemoteState {
    Current,
    Behind,
    Diverged,
}

pub fn status(dir: &str) -> Result<GitStatus, String> {
    run_git(dir, &["rev-parse", "--is-inside-work-tree"])
        .map_err(|_| "the flake directory is not a git repository".to_string())?;
    let branch = run_git(dir, &["rev-parse", "--abbrev-ref", "HEAD"])?
        .trim()
        .to_string();
    let dirty = !run_git(dir, &["status", "--porcelain"])?.trim().is_empty();
    let upstream = run_git(
        dir,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    )
    .ok()
    .map(|value| value.trim().to_string())
    .filter(|value| !value.is_empty());

    let (ahead, behind) = match &upstream {
        Some(_) => run_git(dir, &["rev-list", "--left-right", "--count", "HEAD...@{u}"])
            .ok()
            .and_then(|value| parse_counts(&value).ok())
            .unwrap_or((0, 0)),
        None => (0, 0),
    };

    let state = match &upstream {
        None => "no-upstream",
        Some(_) if behind == 0 => "current",
        Some(_) if ahead == 0 => "behind",
        Some(_) => "diverged",
    };

    Ok(GitStatus {
        branch,
        upstream,
        dirty,
        ahead,
        behind,
        state: state.to_string(),
    })
}

pub fn pull_ff_only(dir: &str) -> Result<(), String> {
    run_git(dir, &["pull", "--ff-only"]).map(|_| ())
}

/// Unstaged working-tree diff together with the short status list. Untracked
/// files appear in the status block because `git diff` cannot show them.
pub fn diff(dir: &str) -> Result<String, String> {
    run_git(dir, &["rev-parse", "--is-inside-work-tree"])
        .map_err(|_| "the flake directory is not a git repository".to_string())?;
    let status = run_git(dir, &["status", "--porcelain", "--", "."])?;
    let patch = run_git(dir, &["diff", "--no-color", "--", "."])?;
    let mut out = String::new();
    if !status.trim().is_empty() {
        out.push_str("Status:\n");
        out.push_str(status.trim_end());
        out.push_str("\n\n");
    }
    out.push_str(&patch);
    Ok(out)
}

/// Stage every change under the root and commit it without pushing.
pub fn commit(dir: &str, message: &str) -> Result<String, String> {
    if message.trim().is_empty() {
        return Err("commit message is required".to_string());
    }
    run_git(dir, &["rev-parse", "--is-inside-work-tree"])
        .map_err(|_| "the flake directory is not a git repository".to_string())?;
    run_git(dir, &["add", "-A", "."])?;
    run_git(dir, &["commit", "-m", message.trim()])?;
    Ok(run_git(dir, &["rev-parse", "--short", "HEAD"])?
        .trim()
        .to_string())
}

/// Push the current branch to its configured upstream.
pub fn push(dir: &str) -> Result<String, String> {
    run_git(dir, &["rev-parse", "--is-inside-work-tree"])
        .map_err(|_| "the flake directory is not a git repository".to_string())?;
    let upstream = run_git(
        dir,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    )
    .map_err(|_| "no upstream branch is configured".to_string())?;
    run_git(dir, &["push"])?;
    Ok(upstream.trim().to_string())
}

/// Stage the working tree, commit it, and push when the branch is current.
pub fn sync(dir: &str, message: &str) -> Result<(), String> {
    if message.trim().is_empty() {
        return Err("commit message is required".to_string());
    }
    run_git(dir, &["rev-parse", "--is-inside-work-tree"])
        .map_err(|_| "the flake directory is not a git repository".to_string())?;
    run_git(
        dir,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    )
    .map_err(|_| "no upstream branch is configured".to_string())?;
    run_git(dir, &["fetch"])?;
    let counts = run_git(dir, &["rev-list", "--left-right", "--count", "HEAD...@{u}"])?;
    match classify(&counts)? {
        RemoteState::Current => {}
        RemoteState::Behind => {
            return Err(
                "the upstream branch has newer commits; nothing was committed or pushed".to_string(),
            )
        }
        RemoteState::Diverged => {
            return Err(
                "the branch and its upstream have diverged; nothing was committed or pushed"
                    .to_string(),
            )
        }
    }

    commit(dir, message)?;
    push(dir)?;
    Ok(())
}

fn parse_counts(output: &str) -> Result<(u64, u64), String> {
    let mut fields = output.split_whitespace();
    let mut next = || -> Result<u64, String> {
        fields
            .next()
            .ok_or_else(|| "cannot read git ahead/behind count".to_string())?
            .parse()
            .map_err(|_| "cannot read git ahead/behind count".to_string())
    };
    Ok((next()?, next()?))
}

fn classify(output: &str) -> Result<RemoteState, String> {
    let (ahead, behind) = parse_counts(output)?;
    Ok(if behind == 0 {
        RemoteState::Current
    } else if ahead == 0 {
        RemoteState::Behind
    } else {
        RemoteState::Diverged
    })
}

fn run_git(dir: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|error| format!("cannot run git {}: {error}", args.join(" ")))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if stderr.is_empty() { stdout } else { stderr };
    Err(if detail.is_empty() {
        format!("git {} failed", args.join(" "))
    } else {
        format!("git {}: {detail}", args.join(" "))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_remote_state() {
        assert_eq!(classify("0\t0").unwrap(), RemoteState::Current);
        assert_eq!(classify("2\t0").unwrap(), RemoteState::Current);
        assert_eq!(classify("0\t2").unwrap(), RemoteState::Behind);
        assert_eq!(classify("1\t2").unwrap(), RemoteState::Diverged);
    }

    #[test]
    fn rejects_an_empty_commit_message() {
        assert!(commit("/tmp", "").is_err());
        assert!(sync("/tmp", "  ").is_err());
    }
}
