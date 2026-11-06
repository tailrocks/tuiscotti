use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

struct TempFile {
    path: Option<PathBuf>,
}

const MAX_TEMP_CREATE_ATTEMPTS: u32 = 100;

impl TempFile {
    fn create(dir: &Path, name: &str) -> Result<(fs::File, Self), std::io::Error> {
        Self::create_retry(dir, name)
    }

    fn create_retry(dir: &Path, name: &str) -> Result<(fs::File, Self), std::io::Error> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        for _ in 0..MAX_TEMP_CREATE_ATTEMPTS {
            match Self::create_once(dir, name, COUNTER.fetch_add(1, Ordering::SeqCst)) {
                Ok(staged) => return Ok(staged),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("cannot stage a fresh temporary file for {name}"),
        ))
    }

    fn create_once(dir: &Path, name: &str, id: u64) -> Result<(fs::File, Self), std::io::Error> {
        let path = dir.join(format!(".tmp-{}-{id}-{name}", std::process::id()));
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
                "temporary evidence file is already committed",
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

pub(crate) fn atomic_write(
    dir: &Path,
    name: &str,
    bytes: &[u8],
) -> Result<PathBuf, std::io::Error> {
    let target = dir.join(name);
    let (mut file, temp) = TempFile::create(dir, name)?;
    file.write_all(bytes)?;
    file.flush()?;
    drop(file);
    temp.commit(&target)?;
    Ok(target)
}
