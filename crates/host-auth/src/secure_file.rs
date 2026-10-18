use anyhow::{Context, Result};
use fs2::FileExt;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    marker::PhantomData,
    path::{Path, PathBuf},
};

/// Read one atomic snapshot without acquiring the writer lock. Never wait on a
/// suspended commissioning process in the Host authorization path. The writer
/// publishes by rename, so readers see either complete generation.
pub fn read_snapshot<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    anyhow::ensure!(
        metadata.is_file() && metadata.len() <= 1024 * 1024,
        "invalid security file"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let uid = unsafe { libc::geteuid() };
        anyhow::ensure!(
            metadata.mode() & 0o077 == 0 && (uid == 0 || metadata.uid() == uid),
            "security file must be private to the service account"
        );
    }
    let mut bytes = Vec::new();
    file.take(1024 * 1024 + 1).read_to_end(&mut bytes)?;
    anyhow::ensure!(bytes.len() <= 1024 * 1024, "security file too large");
    serde_json::from_slice(&bytes).context("invalid security state")
}

/// The lock is separate from the atomically replaced data file. All processes use it.
pub struct SecureFile<T> {
    path: PathBuf,
    marker: PhantomData<T>,
}
impl<T> Clone for SecureFile<T> {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            marker: PhantomData,
        }
    }
}
impl<T: Serialize + DeserializeOwned + Default> SecureFile<T> {
    pub fn new(path: PathBuf) -> Result<Self> {
        let parent = path.parent().context("security file has no parent")?;
        fs::create_dir_all(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
        }
        #[cfg(windows)]
        crate::windows_security::apply_local_user_dacl(parent).map_err(anyhow::Error::msg)?;
        Ok(Self {
            path,
            marker: PhantomData,
        })
    }
    fn lock(&self) -> Result<File> {
        let mut options = OpenOptions::new();
        options.create(true).read(true).write(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        }
        let file = options.open(self.path.with_extension("lock"))?;
        file.lock_exclusive()?;
        Ok(file)
    }
    fn load(&self) -> Result<T> {
        match fs::symlink_metadata(&self.path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(T::default()),
            Err(e) => return Err(e.into()),
            Ok(m) if !m.file_type().is_file() => {
                anyhow::bail!("security state must be a regular file")
            }
            Ok(_) => {}
        }
        read_snapshot(&self.path).context("invalid security state; refusing to replace identity")
    }
    /// Call from a blocking worker, never while holding the async executor thread.
    pub fn read(&self) -> Result<T> {
        let _lock = self.lock()?;
        self.load()
    }
    pub fn update<R>(&self, change: impl FnOnce(&mut T) -> Result<R>) -> Result<R> {
        let _lock = self.lock()?;
        let mut value = self.load()?;
        let result = change(&mut value)?;
        let mut temp = tempfile::NamedTempFile::new_in(self.path.parent().unwrap())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            temp.as_file()
                .set_permissions(fs::Permissions::from_mode(0o600))?;
        }
        #[cfg(windows)]
        crate::windows_security::apply_local_user_dacl(temp.path()).map_err(anyhow::Error::msg)?;
        temp.write_all(&serde_json::to_vec(&value)?)?;
        temp.as_file().sync_all()?;
        temp.persist(&self.path).map_err(|e| e.error)?;
        #[cfg(unix)]
        File::open(self.path.parent().unwrap())?.sync_all()?;
        Ok(result)
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Private, atomic exports; identity.json remains the sole key source.
pub fn write_private_export(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut temp =
        tempfile::NamedTempFile::new_in(path.parent().context("export parent missing")?)?;
    #[cfg(windows)]
    crate::windows_security::apply_local_user_dacl(temp.path()).map_err(anyhow::Error::msg)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(path).map_err(|e| e.error)?;
    #[cfg(unix)]
    File::open(path.parent().unwrap())?.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn concurrent_updates_do_not_lose_bindings_and_failure_does_not_commit() {
        let dir = tempfile::tempdir().unwrap();
        let store = SecureFile::<Vec<u64>>::new(dir.path().join("state.json")).unwrap();
        let threads: Vec<_> = (0..12)
            .map(|i| {
                let store = store.clone();
                std::thread::spawn(move || {
                    store
                        .update(|v| {
                            v.push(i);
                            Ok(())
                        })
                        .unwrap()
                })
            })
            .collect();
        for t in threads {
            t.join().unwrap();
        }
        assert_eq!(store.read().unwrap().len(), 12);
        let result: Result<()> = store.update(|v| {
            v.clear();
            anyhow::bail!("rejected")
        });
        assert!(result.is_err());
        assert_eq!(store.read().unwrap().len(), 12);
        fs::write(store.path(), b"broken").unwrap();
        assert!(store.read().is_err());
        assert!(store
            .update(|v| {
                v.clear();
                Ok(())
            })
            .is_err());
    }
}
