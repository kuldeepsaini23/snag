//! One RDM per user: a second copy would run the same downloads into the same files.

use std::fs::File;
use std::path::Path;

/// Takes the instance lock in `dir`. `None` means another RDM is already running.
/// Keep the returned file alive for as long as the app runs.
pub fn lock(dir: &Path) -> Option<File> {
    std::fs::create_dir_all(dir).ok()?;
    let file = std::fs::OpenOptions::new().create(true).write(true).truncate(false).open(dir.join("rdm.lock")).ok()?;
    file.try_lock().ok()?;
    Some(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_one_instance_holds_the_lock() {
        let dir = tempfile::tempdir().unwrap();
        let first = lock(dir.path()).expect("first instance gets the lock");
        assert!(lock(dir.path()).is_none(), "second instance must be refused");
        drop(first);
        // On Linux the lock is a flock, held by every copy of the file handle: a child process
        // another test starts at this moment holds one between fork and exec, so allow a moment.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let mut again = lock(dir.path());
        while again.is_none() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(20));
            again = lock(dir.path());
        }
        assert!(again.is_some(), "lock is free again after exit");
    }
}
