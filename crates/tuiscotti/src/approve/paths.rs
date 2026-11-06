use std::fs;
use std::io::Write as _;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::sample::{Sample, SampleError, read_bounded, read_bounded_optional};

use super::changes::ChangeSet;
use super::errors::ApproveError;
use super::outcome::AcceptOutcome;

pub(super) fn baseline_relative(candidate: &Sample) -> Result<PathBuf, ApproveError> {
    let mut relative = PathBuf::new();
    for component in Path::new(candidate.test_id()).components() {
        match component {
            Component::Normal(part) => relative.push(part),
            other => {
                return Err(ApproveError::InvalidStorePath {
                    detail: format!(
                        "test ID {:?} carries a non-normal path component ({other:?})",
                        candidate.test_id()
                    ),
                    observed_baseline: None,
                });
            }
        }
    }
    relative.set_extension("tui.pb");
    if relative.parent().is_none() {
        return Err(ApproveError::InvalidStorePath {
            detail: "baseline path has no parent directory".to_string(),
            observed_baseline: None,
        });
    }
    if relative
        .file_name()
        .and_then(|name| name.to_str())
        .is_none()
    {
        return Err(ApproveError::InvalidStorePath {
            detail: "baseline path has no file name".to_string(),
            observed_baseline: None,
        });
    }
    Ok(relative)
}

pub(super) fn read_baseline_bytes(
    baselines: &Path,
    relative: &Path,
) -> Result<Option<Vec<u8>>, ApproveError> {
    read_bounded_optional(&baselines.join(relative)).map_err(|error| match error {
        SampleError::Io { source } => ApproveError::Io {
            source,
            observed_baseline: None,
        },
        other => ApproveError::from(other),
    })
}

pub(super) fn write_baseline(
    baselines: &Path,
    relative: &Path,
    candidate: &Sample,
    candidate_bytes: &[u8],
    candidate_digest: &str,
    previous_digest: &str,
    changes: ChangeSet,
) -> Result<AcceptOutcome, ApproveError> {
    let parent = relative
        .parent()
        .ok_or_else(|| ApproveError::InvalidStorePath {
            detail: "baseline path has no parent directory".to_string(),
            observed_baseline: None,
        })?;
    let file_name = relative
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| ApproveError::InvalidStorePath {
            detail: "baseline path has no file name".to_string(),
            observed_baseline: None,
        })?;
    let parent_dir = baselines.join(parent);
    fs::create_dir_all(&parent_dir).map_err(|source| ApproveError::Io {
        source,
        observed_baseline: None,
    })?;
    let (mut staged, temp) =
        TempFile::create(&parent_dir, file_name).map_err(|source| ApproveError::Io {
            source,
            observed_baseline: None,
        })?;
    staged
        .write_all(candidate_bytes)
        .map_err(|source| ApproveError::Io {
            source,
            observed_baseline: None,
        })?;
    staged.flush().map_err(|source| ApproveError::Io {
        source,
        observed_baseline: None,
    })?;
    drop(staged);
    let outcome = AcceptOutcome {
        test_id: candidate.test_id().to_string(),
        gate: candidate.gate(),
        candidate_sha256: candidate_digest.to_string(),
        previous_baseline_sha256: previous_digest.to_string(),
        baseline_sha256: candidate_digest.to_string(),
        baseline_path: parent_dir.join(file_name),
        reason: changes.reason,
        changed_fields: changes.fields,
    };
    temp.commit(outcome.baseline_path())
        .map_err(|source| ApproveError::Io {
            source,
            observed_baseline: None,
        })?;
    Ok(outcome)
}
pub(super) fn read_candidate_bytes(path: &Path) -> Result<Vec<u8>, ApproveError> {
    read_bounded(path).map_err(|error| match error {
        SampleError::Io { source } => ApproveError::Io {
            source: std::io::Error::new(
                source.kind(),
                format!("cannot read candidate {}: {source}", path.display()),
            ),
            observed_baseline: None,
        },
        other => ApproveError::from(other),
    })
}

pub(super) fn resolve_baseline_root(path: &Path) -> Result<PathBuf, ApproveError> {
    if path.as_os_str().is_empty() {
        return Err(ApproveError::InvalidStorePath {
            detail: "baseline root path is empty".to_string(),
            observed_baseline: None,
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
            return Err(ApproveError::InvalidStorePath {
                detail: format!(
                    "baseline root {} has no resolvable ancestor",
                    path.display()
                ),
                observed_baseline: None,
            });
        };
        if name == ".." {
            return Err(ApproveError::InvalidStorePath {
                detail: format!(
                    "baseline root {} carries an unresolvable `..`",
                    path.display()
                ),
                observed_baseline: None,
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

pub(super) fn ensure_no_symlink(root: &Path, relative: &Path) -> Result<(), ApproveError> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        let meta = match fs::symlink_metadata(&current) {
            Ok(meta) => meta,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(source) => {
                return Err(ApproveError::Io {
                    source,
                    observed_baseline: None,
                });
            }
        };
        if meta.file_type().is_symlink() {
            return Err(ApproveError::InvalidStorePath {
                detail: format!(
                    "store-relative path {} crosses a symlink",
                    relative.display()
                ),
                observed_baseline: None,
            });
        }
    }
    Ok(())
}

struct TempFile {
    path: Option<PathBuf>,
}

impl TempFile {
    fn create(dir: &Path, name: &str) -> Result<(fs::File, Self), std::io::Error> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = dir.join(format!(".tmp-accept-{}-{id}-{name}", std::process::id()));
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        let file = options.open(&path)?;
        Ok((file, Self { path: Some(path) }))
    }

    fn commit(mut self, target: &Path) -> Result<(), std::io::Error> {
        let Some(path) = self.path.take() else {
            return Err(std::io::Error::other(
                "temporary approval file is already committed",
            ));
        };
        match fs::rename(&path, target) {
            Ok(()) => Ok(()),
            Err(error) => {
                drop(fs::remove_file(&path));
                Err(error)
            }
        }
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            drop(fs::remove_file(path));
        }
    }
}
