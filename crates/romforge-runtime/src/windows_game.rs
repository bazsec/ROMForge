use anyhow::{Context, Result, ensure};
use fs2::FileExt;
use romforge_format::{BuildInfo, Payload, windows as pe};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Cursor,
    path::{Component, Path, PathBuf},
};

pub struct Game {
    pub payload: Option<Payload>,
    pub core: PathBuf,
    _lease: fs::File,
    // The DLL stays on disk until Session and the native library have unloaded.
    _directory: tempfile::TempDir,
}
fn runtime_directory(root: &Path) -> Result<(fs::File, tempfile::TempDir)> {
    fs::create_dir_all(root)?;
    let root = root.canonicalize()?;
    let startup = fs::File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(root.join("startup.lock"))?;
    startup.lock_exclusive()?;
    // Only remove our own direct child directories, with an unlocked owner file.
    // The startup lock prevents a new game from being mistaken for a crashed one
    // between creating its directory and acquiring its per-game lease.
    for entry in fs::read_dir(&root)? {
        let Ok(entry) = entry else { continue };
        if !entry.file_name().to_string_lossy().starts_with("game-")
            || !entry.file_type().is_ok_and(|kind| kind.is_dir())
        {
            continue;
        }
        let Ok(path) = entry.path().canonicalize() else {
            continue;
        };
        if path.parent() != Some(root.as_path()) {
            continue;
        }
        let Ok(lease) = fs::File::options()
            .read(true)
            .write(true)
            .open(path.join("owner.lock"))
        else {
            continue;
        };
        if lease.try_lock_exclusive().is_ok() {
            drop(lease);
            // Failed cleanup (e.g. an antivirus scan) can be retried next launch.
            let _ = fs::remove_dir_all(&path);
        }
    }
    let directory = tempfile::Builder::new().prefix("game-").tempdir_in(root)?;
    let lease = fs::File::options()
        .read(true)
        .write(true)
        .create_new(true)
        .open(directory.path().join("owner.lock"))?;
    lease.lock_exclusive()?;
    drop(startup);
    Ok((lease, directory))
}
pub fn embedded() -> bool {
    pe::current(pe::BUILD).is_ok()
}
pub fn load() -> Result<Game> {
    let info: BuildInfo = serde_json::from_slice(&pe::current(pe::BUILD)?)?;
    let payload = romforge_format::read(Cursor::new(pe::current(pe::PAYLOAD)?))?;
    let core = pe::current(pe::CORE)?;
    ensure!(
        info.console == payload.metadata.console
            && info.rom_sha256 == format!("{:x}", Sha256::digest(&payload.rom)),
        "Embedded ROM identity mismatch"
    );
    ensure!(
        info.core_sha256 == format!("{:x}", Sha256::digest(&core)),
        "Embedded core checksum mismatch"
    );
    let (lease, directory) = runtime_directory(&std::env::temp_dir().join("ROMForge/runtime"))?;
    let path = directory
        .path()
        .join(format!("{}_libretro.dll", info.console.core_name()));
    fs::write(&path, core)?;
    Ok(Game {
        payload: Some(payload),
        core: path,
        _lease: lease,
        _directory: directory,
    })
}

pub fn export(output: &Path) -> Result<()> {
    let files: BTreeMap<String, String> = serde_json::from_slice(&pe::current(pe::NOTICES)?)?;
    ensure!(
        !output.exists(),
        "Export destination exists; choose an empty new directory"
    );
    for name in files.keys() {
        ensure!(
            Path::new(name)
                .components()
                .all(|part| matches!(part, Component::Normal(_))),
            "Invalid embedded resource path"
        );
    }
    fs::create_dir_all(output)?;
    for (name, text) in files {
        let path = output.join(name);
        fs::create_dir_all(path.parent().context("Missing resource parent")?)?;
        fs::write(path, text)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crash_worker() {
        let Some(root) = std::env::var_os("ROMFORGE_CRASH_TEST_ROOT") else {
            return;
        };
        let (_lease, directory) = runtime_directory(Path::new(&root)).unwrap();
        fs::write(directory.path().join("core.dll"), b"crash fixture").unwrap();
        // Deliberately bypass Rust destructors, as happens after a forced stop.
        std::process::exit(0);
    }
    #[test]
    fn abandoned_directory_after_process_exit_is_cleaned_on_next_launch() {
        let root = tempfile::tempdir().unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "windows_game::tests::crash_worker"])
            .env("ROMFORGE_CRASH_TEST_ROOT", root.path())
            .status()
            .unwrap();
        assert!(status.success());
        let abandoned: Vec<_> = fs::read_dir(root.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().starts_with("game-"))
            .map(|e| e.path())
            .collect();
        assert_eq!(abandoned.len(), 1);
        let (lease, directory) = runtime_directory(root.path()).unwrap();
        assert!(!abandoned[0].exists());
        drop(lease);
        drop(directory);
    }
    #[test]
    fn next_launch_removes_crashed_runtime_and_preserves_running_games() {
        let root = tempfile::tempdir().unwrap();
        let (active_lease, active) = runtime_directory(root.path()).unwrap();
        fs::write(active.path().join("core.dll"), b"running").unwrap();
        let (crashed_lease, crashed) = runtime_directory(root.path()).unwrap();
        fs::write(crashed.path().join("core.dll"), b"abandoned").unwrap();
        let abandoned = crashed.keep();
        drop(crashed_lease); // OS releases this lock after a real crash, too.
        let unrelated = root.path().join("keep-this-folder");
        fs::create_dir(&unrelated).unwrap();
        let (next_lease, next) = runtime_directory(root.path()).unwrap();
        assert!(!abandoned.exists());
        assert!(active.path().join("core.dll").is_file());
        assert!(unrelated.is_dir());
        let next_path = next.path().to_path_buf();
        drop(next_lease);
        drop(next);
        assert!(!next_path.exists());
        drop(active_lease);
    }
}
