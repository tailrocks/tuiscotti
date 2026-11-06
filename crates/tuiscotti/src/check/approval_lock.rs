use fs2::FileExt as _;

pub(crate) const APPROVAL_LOCK_FILE: &str = ".tuiscotti-approval.lock";

pub(crate) const APPROVAL_LOCK_TIMEOUT_SECS: u64 = 30;

const APPROVAL_LOCK_POLL: std::time::Duration = std::time::Duration::from_millis(5);

pub(crate) struct ApprovalFileLock {
    file: std::fs::File,
}

impl std::fmt::Debug for ApprovalFileLock {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ApprovalFileLock")
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub(crate) enum ApprovalLockError {
    Timeout,
    Io(std::io::Error),
}

impl std::fmt::Display for ApprovalLockError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => write!(formatter, "approval lock deadline passed"),
            Self::Io(source) => write!(formatter, "approval lock I/O failed: {source}"),
        }
    }
}

impl std::error::Error for ApprovalLockError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Timeout => None,
            Self::Io(source) => Some(source),
        }
    }
}

impl From<std::io::Error> for ApprovalLockError {
    fn from(source: std::io::Error) -> Self {
        Self::Io(source)
    }
}

impl ApprovalFileLock {
    pub(crate) fn acquire(
        baseline_root: &std::path::Path,
        timeout: std::time::Duration,
    ) -> Result<Self, ApprovalLockError> {
        let path = baseline_root.join(APPROVAL_LOCK_FILE);
        let file = open_lock_file(&path)?;
        let start = std::time::Instant::now();
        loop {
            if file.try_lock_exclusive().is_ok() {
                return Ok(Self { file });
            }
            if start.elapsed() >= timeout {
                return Err(ApprovalLockError::Timeout);
            }
            std::thread::sleep(APPROVAL_LOCK_POLL);
        }
    }
}

impl Drop for ApprovalFileLock {
    fn drop(&mut self) {
        drop(self.file.unlock());
    }
}

fn open_lock_file(path: &std::path::Path) -> Result<std::fs::File, ApprovalLockError> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    options.open(path).map_err(ApprovalLockError::Io)
}
