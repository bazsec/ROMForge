use std::path::{Path, PathBuf};
#[derive(Default)]
pub struct Report {
    pub built: usize,
    pub skipped: usize,
    pub failed: Vec<PathBuf>,
    pub cancelled: bool,
    pub log: String,
}
impl Report {
    pub fn summary(&self) -> String {
        format!(
            "{}: {} created, {} skipped, {} failed",
            if self.cancelled {
                "Cancelled"
            } else {
                "Finished"
            },
            self.built,
            self.skipped,
            self.failed.len()
        )
    }
}
/// Serial processing; cancellation never interrupts an in-flight build.
pub fn run(
    roms: &[PathBuf],
    output: &Path,
    replace: bool,
    cancel: &std::sync::atomic::AtomicBool,
    mut build: impl FnMut(&PathBuf) -> Result<String, String>,
    mut progress: impl FnMut(usize, usize, String),
) -> Report {
    let mut report = Report::default();
    for (index, rom) in roms.iter().enumerate() {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            report.cancelled = true;
            break;
        }
        let title = rom.file_stem().unwrap_or_default().to_string_lossy();
        progress(
            index,
            roms.len(),
            format!("Creating {} of {}: {title}", index + 1, roms.len()),
        );
        if !replace && output.join(format!("{title}.app")).exists() {
            report.skipped += 1;
            report
                .log
                .push_str(&format!("SKIPPED {title}: app already exists\n"));
            continue;
        }
        match build(rom) {
            Ok(details) => {
                report.built += 1;
                report
                    .log
                    .push_str(&format!("BUILT {title}\n{details}\n\n"));
            }
            Err(error) => {
                report.failed.push(rom.clone());
                report.log.push_str(&format!("FAILED {title}\n{error}\n\n"));
            }
        }
    }
    report
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    #[test]
    fn cancel_and_retry_paths() {
        let dir = tempfile::tempdir().unwrap();
        let cancel = AtomicBool::new(false);
        let roms = ["bad.sfc", "good.sfc", "later.sfc"].map(PathBuf::from);
        let report = run(
            &roms,
            dir.path(),
            false,
            &cancel,
            |rom| {
                if rom == Path::new("bad.sfc") {
                    Err("invalid".into())
                } else {
                    cancel.store(true, Ordering::Relaxed);
                    Ok("checked".into())
                }
            },
            |_, _, _| {},
        );
        assert!(report.cancelled);
        assert_eq!(report.built, 1);
        assert_eq!(report.failed, vec![roms[0].clone()]);
    }
    #[test]
    fn skips_existing_and_continues_failures() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("old.app"), b"preserve").unwrap();
        let report = run(
            &["old.sfc", "bad.sfc", "new.sfc"].map(PathBuf::from),
            dir.path(),
            false,
            &AtomicBool::new(false),
            |p| {
                if p == Path::new("bad.sfc") {
                    Err("bad".into())
                } else {
                    Ok(String::new())
                }
            },
            |_, _, _| {},
        );
        assert_eq!(report.summary(), "Finished: 1 created, 1 skipped, 1 failed");
        assert_eq!(
            std::fs::read(dir.path().join("old.app")).unwrap(),
            b"preserve"
        );
    }
}
