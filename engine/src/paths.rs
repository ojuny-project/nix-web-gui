use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum PathError {
    #[error("path is outside the configured root")]
    OutsideRoot,
    #[error("path has no file name")]
    InvalidPath,
    #[error("cannot resolve path: {0}")]
    Io(#[from] std::io::Error),
}

pub fn confine(root: &Path, candidate: &Path) -> Result<PathBuf, PathError> {
    let root = root.canonicalize()?;
    let path = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    };
    let path = path.canonicalize()?;
    if path.starts_with(&root) {
        Ok(path)
    } else {
        Err(PathError::OutsideRoot)
    }
}

/// Resolve a path that may not exist yet. The parent must already resolve
/// inside the root; the final component is appended after the check.
pub fn confine_new(root: &Path, candidate: &Path) -> Result<PathBuf, PathError> {
    let root = root.canonicalize()?;
    let path = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    };
    let parent = path.parent().ok_or(PathError::InvalidPath)?;
    let name = path.file_name().ok_or(PathError::InvalidPath)?;
    std::fs::create_dir_all(parent)?;
    let parent = parent.canonicalize()?;
    if parent.starts_with(&root) {
        Ok(parent.join(name))
    } else {
        Err(PathError::OutsideRoot)
    }
}

pub fn is_editable(root: &Path, candidate: &Path) -> bool {
    !candidate.starts_with("/nix/store") && confine(root, candidate).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_paths_outside_the_root() {
        let root = std::env::temp_dir().join(format!("nix-web-gui-root-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let inside = root.join("a.nix");
        std::fs::write(&inside, "{}").unwrap();
        assert!(confine(&root, &inside).is_ok());
        assert!(confine(&root, Path::new("/etc/passwd")).is_err());
        assert!(!is_editable(&root, Path::new("/nix/store/abc/a.nix")));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn allows_a_new_file_inside_the_root() {
        let root = std::env::temp_dir().join(format!(
            "nix-web-gui-new-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let new = confine_new(&root, Path::new("modules/new.nix")).unwrap();
        assert!(new.starts_with(root.canonicalize().unwrap()));
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_symlink_that_escapes_the_root() {
        let root = std::env::temp_dir().join(format!(
            "nix-web-gui-symlink-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let link = root.join("escape.nix");
        std::os::unix::fs::symlink("/etc/passwd", &link).unwrap();
        assert!(confine(&root, &link).is_err());
        let _ = std::fs::remove_dir_all(root);
    }
}
