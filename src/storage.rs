use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static TEMP_ID: AtomicU64 = AtomicU64::new(0);

pub(crate) fn best_path() -> Option<PathBuf> {
    data_path(env::var_os("XDG_DATA_HOME"), env::var_os("HOME"))
}

fn data_path(xdg: Option<std::ffi::OsString>, home: Option<std::ffi::OsString>) -> Option<PathBuf> {
    let root = xdg
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            home.map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .map(|path| path.join(".local/share"))
        })?;
    Some(root.join("flappy-jack/best-score"))
}

pub(crate) fn load(path: &Path) -> u32 {
    let mut text = String::new();
    let result = File::open(path).and_then(|file| file.take(33).read_to_string(&mut text));
    if result.is_err() || text.len() > 32 {
        return 0;
    }
    text.trim().parse().unwrap_or(0)
}

pub(crate) fn save(path: &Path, score: u32) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "best score needs a parent directory",
            )
        })?;
    fs::create_dir_all(parent)?;
    let temp = parent.join(format!(
        ".best-score-{}-{}.tmp",
        std::process::id(),
        TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    let result = (|| {
        writeln!(file, "{}", score.max(load(path)))?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) struct TestDir(pub PathBuf);

    impl TestDir {
        pub(crate) fn new() -> Self {
            let path = env::temp_dir().join(format!(
                "flappy-jack-test-{}-{}",
                std::process::id(),
                TEMP_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn xdg_path_and_home_fallback_do_not_modify_environment() {
        assert_eq!(
            data_path(Some("/data".into()), Some("/home/player".into())),
            Some(PathBuf::from("/data/flappy-jack/best-score"))
        );
        for xdg in [None, Some("".into()), Some("relative".into())] {
            assert_eq!(
                data_path(xdg, Some("/home/player".into())),
                Some(PathBuf::from(
                    "/home/player/.local/share/flappy-jack/best-score"
                ))
            );
        }
        assert!(data_path(None, None).is_none());
        assert!(data_path(None, Some("".into())).is_none());
    }

    #[test]
    fn roundtrip_creates_parents_and_preserves_a_higher_record() {
        let dir = TestDir::new();
        let path = dir.0.join("nested/best-score");
        assert_eq!(load(&path), 0);
        save(&path, 42).unwrap();
        assert_eq!(load(&path), 42);
        save(&path, 2).unwrap();
        assert_eq!(load(&path), 42);
        save(&path, u32::MAX).unwrap();
        assert_eq!(load(&path), u32::MAX);
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    }

    #[test]
    fn malformed_or_unreadable_records_are_harmless() {
        let dir = TestDir::new();
        let path = dir.0.join("best-score");
        for bytes in [
            b"not a score".as_slice(),
            b"-1",
            b"4294967296",
            b"",
            &[255, 254],
            &[b'9'; 100],
        ] {
            fs::write(&path, bytes).unwrap();
            assert_eq!(load(&path), 0);
        }
        assert_eq!(load(&dir.0), 0);
        fs::write(&path, format!("17{}junk", " ".repeat(30))).unwrap();
        assert_eq!(load(&path), 0);
        fs::write(&path, " 13\n").unwrap();
        assert_eq!(load(&path), 13);
    }

    #[test]
    fn failed_parent_or_replace_is_non_destructive_and_leaves_no_temp() {
        let dir = TestDir::new();
        let file = dir.0.join("file");
        fs::write(&file, "71").unwrap();
        assert!(save(&file.join("best-score"), 99).is_err());
        assert_eq!(fs::read_to_string(&file).unwrap(), "71");
        let target = dir.0.join("directory");
        fs::create_dir(&target).unwrap();
        assert!(save(&target, 99).is_err());
        assert!(target.is_dir());
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 2);
    }
}
