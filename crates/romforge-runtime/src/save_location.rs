//! Portable per-game saves beside the collection, with non-destructive legacy migration.
use anyhow::{Context, Result, ensure};
use fs2::FileExt;
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

pub struct SaveLocation {
    pub path: PathBuf,
    // Keep both locks alive: an older app must not modify the migration source.
    _locks: Vec<File>,
}
fn lock(path: &Path) -> Result<File> {
    let file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path.join("session.lock"))?;
    file.try_lock_exclusive()
        .context("This game is already running. Close it before opening another copy.")?;
    Ok(file)
}
fn collection(exe: &Path) -> Option<&Path> {
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    let bundle = contents.parent()?;
    if macos.file_name()? == "MacOS"
        && contents.file_name()? == "Contents"
        && bundle.extension()? == "app"
    {
        bundle.parent()
    } else {
        None
    }
}
pub fn open(title: &str, id: &str) -> Result<SaveLocation> {
    let project = directories::ProjectDirs::from("", "", "ROMForge")
        .context("user data directory unavailable")?;
    let legacy = project.data_local_dir().join("games").join(id);
    let exe = std::env::current_exe()?;
    match collection(&exe) {
        Some(parent) => portable(&parent.join("Save Games"), title, id, &legacy)
            .context("Unable to use Save Games beside this app. Move the app collection to a writable folder and try again."),
        None => { fs::create_dir_all(&legacy)?; Ok(SaveLocation { _locks: vec![lock(&legacy)?], path: legacy }) }
    }
}
fn portable(root: &Path, title: &str, id: &str, legacy: &Path) -> Result<SaveLocation> {
    ensure!(
        id.len() == 64 && id.bytes().all(|c| c.is_ascii_hexdigit()),
        "Invalid ROM identity"
    );
    fs::create_dir_all(root)?;
    // A collection lock serializes first-run folder selection, including renamed ROMs.
    let collection_lock = lock(root)?;
    let suffix = format!(" [{}]", &id[..12]);
    let mut existing = None;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() && entry.file_name().to_string_lossy().ends_with(&suffix) {
            let identity = fs::read_to_string(entry.path().join("rom.sha256"));
            if identity.as_deref().is_ok_and(|text| text.trim() == id) {
                existing = Some(entry.path());
                break;
            }
        }
    }
    let safe: String = title
        .chars()
        .map(|c| {
            if c.is_control() || "/\\:*?\"<>|".contains(c) {
                '_'
            } else {
                c
            }
        })
        .take(80)
        .collect();
    let name = safe.trim_matches([' ', '.']);
    let path = existing.unwrap_or_else(|| {
        root.join(format!(
            "{}{suffix}",
            if name.is_empty() { "Game" } else { name }
        ))
    });
    fs::create_dir_all(&path)?;
    let mut locks = vec![lock(&path)?];
    let identity = path.join("rom.sha256");
    if identity.exists() {
        ensure!(
            fs::read_to_string(&identity)?.trim() == id,
            "Save folder belongs to another ROM"
        );
    } else {
        write_new(&identity, id.as_bytes())?;
    }
    drop(collection_lock);
    if legacy.is_dir() {
        locks.push(lock(legacy)?);
    }
    let migrated = path.join("migration-complete");
    if !migrated.exists() {
        let mut names = vec!["battery.srm".to_owned(), "settings.json".to_owned()];
        names.extend((1..=5).map(|n| format!("slot-{n}.state")));
        for name in names {
            let source = legacy.join(&name);
            let destination = path.join(&name);
            if source.is_file() && !destination.exists() {
                write_new(&destination, &fs::read(source)?)?;
            }
        }
        write_new(
            &migrated,
            b"Existing Library saves copied; originals retained.\n",
        )?;
    }
    // Detect a read-only destination before running a game whose progress cannot be saved.
    let probe = tempfile::NamedTempFile::new_in(&path)?;
    drop(probe);
    Ok(SaveLocation {
        path,
        _locks: locks,
    })
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = tempfile::NamedTempFile::new_in(path.parent().context("save parent missing")?)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist_noclobber(path)?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn collection_path_is_outside_bundle() {
        assert_eq!(
            collection(Path::new("/Games/Zelda.app/Contents/MacOS/Game")),
            Some(Path::new("/Games"))
        );
        assert_eq!(
            collection(Path::new("/target/release/romforge-runtime")),
            None
        );
    }
    #[test]
    fn migration_preserves_new_saves_and_survives_renaming() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("Save Games");
        let old = temp.path().join("legacy");
        fs::create_dir(&old).unwrap();
        fs::write(old.join("battery.srm"), b"old battery").unwrap();
        fs::write(old.join("slot-5.state"), b"old state").unwrap();
        let id = "a".repeat(64);
        let location = portable(&root, "Zelda", &id, &old).unwrap();
        let path = location.path.clone();
        assert_eq!(fs::read(path.join("slot-5.state")).unwrap(), b"old state");
        assert!(portable(&root, "Zelda", &id, &old).is_err());
        fs::write(path.join("battery.srm"), b"new battery").unwrap();
        fs::remove_file(path.join("slot-5.state")).unwrap();
        drop(location);
        let renamed = portable(&root, "New Title", &id, &old).unwrap();
        assert_eq!(renamed.path, path);
        assert_eq!(fs::read(path.join("battery.srm")).unwrap(), b"new battery");
        assert!(!path.join("slot-5.state").exists());
        assert!(old.join("slot-5.state").exists());
    }
    #[test]
    fn different_roms_and_hostile_titles_are_isolated() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("Save Games");
        let old = temp.path().join("absent");
        let a = portable(&root, "../../Game/", &"a".repeat(64), &old).unwrap();
        let b = portable(&root, "../../Game/", &"b".repeat(64), &old).unwrap();
        assert_eq!(a.path.parent(), Some(root.as_path()));
        assert_ne!(a.path, b.path);
    }
}
