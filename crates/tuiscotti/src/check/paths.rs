use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

use super::CheckError;

pub(super) fn resolve_root(path: &Path, role: &str) -> Result<PathBuf, CheckError> {
    if path.as_os_str().is_empty() {
        return Err(CheckError::InvalidStorePath {
            detail: format!("{role} root path is empty"),
        });
    }
    let anchored: PathBuf;
    let path: &Path = if path.is_relative() {
        match std::env::current_dir() {
            Ok(cwd) => {
                anchored = cwd.join(path);
                &anchored
            }
            Err(_) => path,
        }
    } else {
        path
    };
    if let Ok(canonical) = path.canonicalize() {
        return Ok(canonical);
    }
    let mut remainder: Vec<std::ffi::OsString> = Vec::new();
    let mut cursor = path;
    loop {
        let parent = cursor.parent();
        let name = cursor.file_name();
        let (Some(parent), Some(name)) = (parent, name) else {
            return Err(CheckError::InvalidStorePath {
                detail: format!("{role} root {} has no resolvable ancestor", path.display()),
            });
        };
        if name == ".." {
            return Err(CheckError::InvalidStorePath {
                detail: format!(
                    "{role} root {} carries an unresolvable `..`",
                    path.display()
                ),
            });
        }
        remainder.push(name.to_os_string());
        if let Ok(canonical) = parent.canonicalize() {
            let mut resolved = canonical;
            for component in remainder.iter().rev() {
                resolved.push(component);
            }
            return Ok(resolved);
        }
        cursor = parent;
    }
}

pub(super) fn ensure_no_symlink(root: &Path, relative: &Path) -> Result<(), CheckError> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        let meta = match fs::symlink_metadata(&current) {
            Ok(meta) => meta,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(source) => return Err(CheckError::Io { source }),
        };
        if meta.file_type().is_symlink() {
            return Err(CheckError::InvalidStorePath {
                detail: format!(
                    "store-relative path {} crosses a symlink",
                    relative.display()
                ),
            });
        }
    }
    Ok(())
}

pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 15) as usize] as char);
    }
    out
}
