use std::path::{Path, PathBuf};

/// Process serially so large collections don't spawn competing builders.
pub fn run(
    roms: &[PathBuf],
    output: &Path,
    cancel: &std::sync::atomic::AtomicBool,
    mut build: impl FnMut(&PathBuf) -> Result<String, String>,
    mut progress: impl FnMut(String),
) -> (String, String) {
    let (mut built, mut skipped, mut failed) = (0, 0, 0);
    let mut log = String::new();
    let mut cancelled = false;
    for (index, rom) in roms.iter().enumerate() {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            cancelled = true;
            break;
        }
        let title = rom.file_stem().unwrap_or_default().to_string_lossy();
        progress(format!("Creating {} of {}: {title}", index + 1, roms.len()));
        let destination = output.join(format!("{title}.app"));
        if destination.exists() {
            skipped += 1;
            log.push_str(&format!("SKIPPED {title}: app already exists\n"));
            continue;
        }
        match build(rom) {
            Ok(details) => {
                built += 1;
                log.push_str(&format!("BUILT {title}\n{details}\n\n"));
            }
            Err(error) => {
                failed += 1;
                log.push_str(&format!("FAILED {title}\n{error}\n\n"));
            }
        }
    }
    (
        format!(
            "{}: {built} created, {skipped} skipped, {failed} failed",
            if cancelled { "Cancelled" } else { "Finished" }
        ),
        log,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_preserves_completed_work_and_stops_before_next_game() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let dir = tempfile::tempdir().unwrap();
        let cancel = AtomicBool::new(false);
        let mut attempts = 0;
        let (status, _) = run(
            &[PathBuf::from("First.sfc"), PathBuf::from("Second.sfc")],
            dir.path(),
            &cancel,
            |_| {
                attempts += 1;
                cancel.store(true, Ordering::Relaxed);
                Ok(String::new())
            },
            |_| {},
        );
        assert_eq!(attempts, 1);
        assert_eq!(status, "Cancelled: 1 created, 0 skipped, 0 failed");
    }
    #[test]
    fn preserves_existing_apps_continues_after_failure_and_handles_name_collisions() {
        let output = tempfile::tempdir().unwrap();
        std::fs::write(output.path().join("Existing.app"), b"preserve").unwrap();
        let roms = [
            "Existing.sfc",
            "Bad.sfc",
            "Good.sfc",
            "nested/Good.smc",
            "Last.sfc",
        ]
        .map(PathBuf::from);
        let mut attempted = Vec::new();
        let mut updates = Vec::new();
        let (status, log) = run(
            &roms,
            output.path(),
            &std::sync::atomic::AtomicBool::new(false),
            |rom| {
                attempted.push(rom.clone());
                if rom == Path::new("Bad.sfc") {
                    return Err("invalid ROM".into());
                }
                std::fs::create_dir(output.path().join(format!(
                    "{}.app",
                    rom.file_stem().unwrap().to_string_lossy()
                )))
                .unwrap();
                Ok("Launch check passed".into())
            },
            |status| updates.push(status),
        );
        assert_eq!(status, "Finished: 2 created, 2 skipped, 1 failed");
        assert_eq!(
            attempted,
            ["Bad.sfc", "Good.sfc", "Last.sfc"].map(PathBuf::from)
        );
        assert_eq!(updates.len(), 5);
        assert!(log.contains("FAILED Bad\ninvalid ROM"));
        assert_eq!(
            std::fs::read(output.path().join("Existing.app")).unwrap(),
            b"preserve"
        );
    }
}
