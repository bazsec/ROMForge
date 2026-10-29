use super::*;
use sha2::{Digest, Sha256};
use std::path::Path;

fn components(
    root: &Path,
    console: romforge_format::Console,
) -> Result<(PathBuf, PathBuf, PathBuf, PathBuf), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let directory = exe.parent().ok_or("Application directory unavailable")?;
    let runtime = directory.join(format!("romforge-runtime{}", std::env::consts::EXE_SUFFIX));
    let builder = directory.join(format!("romforge-builder{}", std::env::consts::EXE_SUFFIX));
    let core_name = format!(
        "{}_libretro.{}",
        console.core_name(),
        std::env::consts::DLL_EXTENSION
    );
    let contents = if cfg!(windows) {
        directory.join("Contents")
    } else {
        directory.parent().unwrap_or(directory).to_path_buf()
    };
    let bundled = contents.join("Frameworks").join(&core_name);
    let core = if bundled.is_file() {
        bundled
    } else {
        root.join("vendor")
            .join(console.core_name())
            .join("libretro")
            .join(core_name)
    };
    let bundled = contents.join("Resources").join(console.license_name());
    let license = if bundled.is_file() {
        bundled
    } else {
        root.join("third-party").join(console.license_name())
    };
    for path in [&runtime, &builder, &core, &license] {
        if !path.is_file() {
            return Err(format!("Missing component: {}", path.display()));
        }
    }
    Ok((runtime, builder, core, license))
}
/// Only replace an older ROMForge bundle whose embedded ROM matches exactly.
fn replacement_allowed(app: &Path, rom: &Path) -> Result<(), String> {
    if !matches!(
        library::built_state(app),
        library::Built::Legacy | library::Built::Outdated(_)
    ) {
        return Err("Existing app is current or unrecognized; it was preserved.".into());
    }
    let payload = romforge_format::package_payload(app)
        .map_err(|e| format!("Existing app could not be verified: {e}"))?;
    let bytes = std::fs::read(rom).map_err(|e| e.to_string())?;
    if Sha256::digest(&payload.rom) != Sha256::digest(&bytes) {
        return Err("Existing app contains a different ROM; it was preserved.".into());
    }
    Ok(())
}
fn publish(
    staged: &Path,
    destination: &Path,
    replace: bool,
    rom: &Path,
    verify: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<String, String> {
    let mut backup = None;
    if destination.exists() {
        if !replace {
            return Err("An app appeared during the build; it was preserved.".into());
        }
        replacement_allowed(destination, rom)?;
        let backups = destination
            .parent()
            .ok_or("Missing output directory")?
            .join(".romforge-backups");
        std::fs::create_dir_all(&backups).map_err(|e| e.to_string())?;
        let holder = tempfile::Builder::new()
            .prefix("previous-")
            .tempdir_in(backups)
            .map_err(|e| e.to_string())?;
        let old = holder
            .path()
            .join(destination.file_name().ok_or("Missing app name")?);
        std::fs::rename(destination, &old).map_err(|e| e.to_string())?;
        let holder = holder.keep();
        backup = Some((old, holder));
    }
    if let Err(error) = std::fs::rename(staged, destination) {
        if let Some((old, _)) = &backup
            && let Err(rollback) = std::fs::rename(old, destination)
        {
            return Err(format!(
                "Publish failed: {error}. Restore failed: {rollback}. Previous app is safe at {}",
                old.display()
            ));
        }
        return Err(error.to_string());
    }
    if let Err(error) = verify(destination) {
        // Move the failed replacement back under temporary staging before restoring.
        std::fs::rename(destination, staged).map_err(|e| format!("Final verification failed: {error}; could not move failed app aside: {e}. Previous app: {backup:?}"))?;
        if let Some((old, _)) = &backup {
            std::fs::rename(old, destination).map_err(|e| {
                format!(
                    "Final verification failed: {error}; restore failed: {e}. Previous app: {}",
                    old.display()
                )
            })?;
        }
        return Err(format!(
            "Final verification failed; previous app preserved: {error}"
        ));
    }
    Ok(backup.map_or(String::new(), |(old, _)| {
        format!("Previous app retained at {}\n", old.display())
    }))
}
impl Forge {
    pub fn start(&mut self, _package: bool) {
        let Some(rom) = self.selected.and_then(|i| self.roms.get(i)).cloned() else {
            return;
        };
        self.start_roms(vec![rom], false);
    }
    pub fn start_roms(&mut self, roms: Vec<PathBuf>, replace: bool) {
        if self.job.is_some() || roms.is_empty() {
            return;
        }
        let (root, output_dir, icons, overrides) = (
            self.root.clone(),
            self.output.clone(),
            self.icon_index.clone(),
            self.overrides.clone(),
        );
        self.cancel = Arc::new(AtomicBool::new(false));
        self.batch_running = roms.len() > 1;
        self.failed_roms.clear();
        self.retry_replace = replace;
        self.progress = (0, roms.len());
        let cancel = self.cancel.clone();
        let (tx, rx) = mpsc::channel();
        self.job = Some(rx);
        self.status = "Preparing build…".into();
        self.log.clear();
        std::thread::spawn(move || {
            let build = |rom: &PathBuf| -> Result<String, String> {
                let console = library::console(rom).ok_or("Unsupported console")?;
                let (runtime, builder, core, license) = components(&root, console)?;
                let title = rom.file_stem().unwrap_or_default().to_string_lossy();
                let destination = output_dir.join(romforge_format::package_name(&title));
                if destination.exists() {
                    if !replace {
                        return Err("Existing app preserved".into());
                    }
                    replacement_allowed(&destination, rom)?;
                }
                let icon = match overrides
                    .get(rom)
                    .cloned()
                    .map(icons::Match::Found)
                    .unwrap_or_else(|| icons.find(rom))
                {
                    icons::Match::Found(path) if path.is_file() => Some(path),
                    icons::Match::Found(_) => return Err("Selected cover is missing".into()),
                    icons::Match::Missing => None,
                    icons::Match::Ambiguous(_) => {
                        return Err("Multiple covers match; choose one in the library.".into());
                    }
                };
                std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;
                // Stage on the destination volume, so publication and rollback use renames.
                let staging = tempfile::Builder::new()
                    .prefix(".romforge-checked-")
                    .tempdir_in(&output_dir)
                    .map_err(|e| e.to_string())?;
                let app = staging.path().join(romforge_format::package_name(&title));
                let mut command = Command::new(builder);
                command
                    .arg(
                        romforge_format::native_package_flag()
                            .ok_or("Native packaging is not supported on this platform")?,
                    )
                    .arg(runtime)
                    .arg(core)
                    .arg(license)
                    .arg(rom)
                    .arg(&app);
                if let Some(icon) = &icon {
                    command.arg("--icon").arg(icon);
                }
                let output = command.output().map_err(|e| e.to_string())?;
                let mut log = format!(
                    "{}{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
                if !output.status.success() {
                    return Err(log);
                }
                let check = Command::new(romforge_format::game_executable(&app))
                    .arg("--headless")
                    .output()
                    .map_err(|e| e.to_string())?;
                if !check.status.success() {
                    return Err(format!(
                        "Launch check failed: {}",
                        String::from_utf8_lossy(&check.stderr)
                    ));
                }
                log.push_str(&publish(&app, &destination, replace, rom, |path| {
                    if cfg!(windows) {
                        let check = Command::new(romforge_format::game_executable(path))
                            .arg("--headless")
                            .output()
                            .map_err(|e| e.to_string())?;
                        return if check.status.success() {
                            Ok(())
                        } else {
                            Err(format!(
                                "Launch check failed: {}",
                                String::from_utf8_lossy(&check.stderr)
                            ))
                        };
                    }
                    for (command, args) in [
                        ("/usr/bin/xattr", vec!["-cr"]),
                        ("/usr/bin/codesign", vec!["--verify", "--deep", "--strict"]),
                    ] {
                        let result = Command::new(command)
                            .args(args)
                            .arg(path)
                            .output()
                            .map_err(|e| e.to_string())?;
                        if !result.status.success() {
                            return Err(String::from_utf8_lossy(&result.stderr).into_owned());
                        }
                    }
                    Ok(())
                })?);
                log.push_str(&format!("Verified {} · {}\nF1 or Start + Select opens the menu. Escape exits the game.\n",destination.display(),updater::VERSION.trim()));
                Ok(log)
            };
            let report = batch::run(
                &roms,
                &output_dir,
                replace,
                &cancel,
                build,
                |done, total, status| {
                    let _ = tx.send(JobEvent::Progress(done, total, status));
                },
            );
            let _ = tx.send(JobEvent::Finished(report));
        });
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refuses_unknown_existing_bundle() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("Game.app");
        std::fs::create_dir(&app).unwrap();
        assert!(replacement_allowed(&app, Path::new("missing.sfc")).is_err());
        assert!(app.is_dir());
    }
    #[test]
    fn rebuild_preserves_old_app_and_refuses_different_rom() {
        let dir = tempfile::tempdir().unwrap();
        let rom = dir.path().join("Game.sfc");
        let bytes = vec![0u8; 32768];
        std::fs::write(&rom, &bytes).unwrap();
        let app = dir.path().join("Game.app");
        std::fs::create_dir_all(app.join("Contents/Resources")).unwrap();
        let metadata = romforge_format::Metadata {
            title: "Game".into(),
            console: romforge_format::Console::Snes,
            core_id: "test".into(),
            template_id: "test".into(),
            extension: "sfc".into(),
        };
        let file = std::fs::File::create(app.join("Contents/Resources/game.romforge")).unwrap();
        romforge_format::append(file, &metadata, &bytes).unwrap();
        std::fs::write(app.join("Contents/Resources/BUILD.txt"), b"legacy").unwrap();
        assert!(replacement_allowed(&app, &rom).is_ok());
        std::fs::write(&rom, vec![1u8; 32768]).unwrap();
        assert!(replacement_allowed(&app, &rom).is_err());
        std::fs::write(&rom, &bytes).unwrap();
        let staged = dir.path().join("new.app");
        std::fs::create_dir(&staged).unwrap();
        std::fs::write(staged.join("new"), b"verified").unwrap();
        assert!(
            publish(&staged, &app, true, &rom, |_| Err(
                "signature test failure".into()
            ))
            .is_err()
        );
        assert!(app.join("Contents/Resources/game.romforge").is_file());
        assert!(staged.join("new").is_file());
        let log = publish(&staged, &app, true, &rom, |_| Ok(())).unwrap();
        assert!(app.join("new").is_file());
        assert!(log.contains("Previous app retained"));
        assert_eq!(
            std::fs::read_dir(dir.path().join(".romforge-backups"))
                .unwrap()
                .filter(|e| e.as_ref().unwrap().path().join("Game.app").is_dir())
                .count(),
            1
        );
    }
    #[test]
    fn failed_publication_restores_previous_app() {
        let dir = tempfile::tempdir().unwrap();
        let rom = dir.path().join("Game.sfc");
        let bytes = vec![0u8; 32768];
        std::fs::write(&rom, &bytes).unwrap();
        let app = dir.path().join("Game.app");
        std::fs::create_dir_all(app.join("Contents/Resources")).unwrap();
        let metadata = romforge_format::Metadata {
            title: "Game".into(),
            console: romforge_format::Console::Snes,
            core_id: "test".into(),
            template_id: "test".into(),
            extension: "sfc".into(),
        };
        romforge_format::append(
            std::fs::File::create(app.join("Contents/Resources/game.romforge")).unwrap(),
            &metadata,
            &bytes,
        )
        .unwrap();
        std::fs::write(app.join("Contents/Resources/BUILD.txt"), b"legacy").unwrap();
        assert!(
            publish(
                &dir.path().join("missing.app"),
                &app,
                true,
                &rom,
                |_| Ok(())
            )
            .is_err()
        );
        assert!(app.join("Contents/Resources/game.romforge").is_file());
    }
}
