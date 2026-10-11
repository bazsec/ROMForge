use super::*;
use romforge_format::{BuildInfo, Console, version_number};
use std::{collections::BTreeSet, path::Path};

#[derive(Clone, Copy, PartialEq, Default)]
pub enum Filter {
    #[default]
    All,
    MissingCover,
    NotBuilt,
    NeedsUpdate,
}
impl Filter {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All games",
            Self::MissingCover => "Missing cover",
            Self::NotBuilt => "Not built",
            Self::NeedsUpdate => "Needs update",
        }
    }
}
#[derive(Clone, PartialEq)]
pub enum Built {
    Missing,
    Current(String),
    Outdated(String),
    Legacy,
    Unknown,
}
#[derive(Clone)]
pub struct Entry {
    pub rom: PathBuf,
    pub size: u64,
    pub built: Built,
    pub cover_missing: bool,
    pub blocked: Option<String>,
}
impl Entry {
    pub fn needs_update(&self) -> bool {
        matches!(self.built, Built::Outdated(_) | Built::Legacy)
    }
    pub fn ready(&self, rebuild: bool) -> bool {
        self.blocked.is_none() && (self.built == Built::Missing || rebuild && self.needs_update())
    }
    pub fn status(&self, rebuild: bool) -> String {
        if let Some(reason) = &self.blocked {
            return format!("Needs attention — {reason}");
        }
        if self.ready(rebuild) {
            return format!(
                "{} — {}",
                if self.built == Built::Missing {
                    "Build"
                } else {
                    "Rebuild"
                },
                if self.cover_missing {
                    "default icon"
                } else {
                    "cover matched"
                }
            );
        }
        match &self.built {
            Built::Current(v) => format!("Already built · {v}"),
            Built::Outdated(v) => format!("Older build · {v}"),
            Built::Legacy => "Older ROMForge app · version not recorded".into(),
            Built::Unknown => "Existing app · unrecognized, preserved".into(),
            Built::Missing => "Not built".into(),
        }
    }
    pub fn matches(&self, filter: Filter) -> bool {
        match filter {
            Filter::All => true,
            Filter::MissingCover => self.cover_missing,
            Filter::NotBuilt => self.built == Built::Missing,
            Filter::NeedsUpdate => self.needs_update(),
        }
    }
}
pub fn built_state(app: &Path) -> Built {
    if !app.exists() {
        return Built::Missing;
    }
    if let Ok(bytes) = std::fs::read(app.join(romforge_format::BUILD_INFO))
        && let Ok(info) = serde_json::from_slice::<BuildInfo>(&bytes)
    {
        return if version_number(&info.version)
            .zip(version_number(updater::VERSION))
            .is_some_and(|(old, new)| old < new)
        {
            Built::Outdated(info.version)
        } else {
            Built::Current(info.version)
        };
    }
    if app.join("Contents/Resources/BUILD.txt").is_file()
        && app.join("Contents/Resources/game.romforge").is_file()
    {
        Built::Legacy
    } else {
        Built::Unknown
    }
}
pub fn inspect(
    roms: Vec<PathBuf>,
    output: PathBuf,
    icons: icons::Index,
    overrides: std::collections::BTreeMap<PathBuf, PathBuf>,
) -> Vec<Entry> {
    let mut seen = BTreeSet::new();
    roms.into_iter()
        .map(|rom| {
            let title = rom.file_stem().unwrap_or_default().to_string_lossy();
            let mut blocked = None;
            if !seen.insert(title.to_lowercase()) {
                blocked = Some("duplicate output name".into());
            }
            let cover = overrides
                .get(&rom)
                .cloned()
                .map(icons::Match::Found)
                .unwrap_or_else(|| icons.find(&rom));
            let cover_missing = match cover {
                icons::Match::Missing => true,
                icons::Match::Ambiguous(_) => {
                    blocked = Some("choose between matching covers".into());
                    true
                }
                icons::Match::Found(path) => {
                    if !path.is_file() {
                        blocked = Some("selected cover is missing".into());
                        true
                    } else {
                        false
                    }
                }
            };
            let metadata = std::fs::metadata(&rom);
            let size = metadata.as_ref().map_or(0, |m| m.len());
            if metadata.is_err() {
                blocked = Some("ROM is unavailable".into());
            }
            if console(&rom).is_none() {
                blocked = Some("unsupported console".into());
            }
            let built = built_state(&output.join(format!("{title}.app")));
            Entry {
                rom,
                size,
                built,
                cover_missing,
                blocked,
            }
        })
        .collect()
}
pub fn console(rom: &Path) -> Option<Console> {
    Console::playable(rom.extension()?.to_str()?)
}
impl Forge {
    pub fn poll_library(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.library_job {
            match rx.try_recv() {
                Ok(entries) => {
                    if !self.library_dirty {
                        self.entries = entries;
                    }
                    self.library_job = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.library_job = None;
                    self.status = "Library check stopped; press Refresh to retry.".into();
                }
                Err(mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(Duration::from_millis(100));
                }
            }
        }
        if self.library_dirty && self.library_job.is_none() {
            self.library_dirty = false;
            let (tx, rx) = mpsc::channel();
            self.library_job = Some(rx);
            let (roms, output, icons, overrides) = (
                self.roms.clone(),
                self.output.clone(),
                self.icon_index.clone(),
                self.overrides.clone(),
            );
            std::thread::spawn(move || {
                let _ = tx.send(inspect(roms, output, icons, overrides));
            });
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
    pub fn ready_roms(&self) -> Vec<PathBuf> {
        if self.library_dirty || self.library_job.is_some() {
            return Vec::new();
        }
        self.entries
            .iter()
            .filter(|e| e.ready(self.rebuild_outdated))
            .map(|e| e.rom.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_versions_without_downgrading_and_blocks_collisions() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("apps");
        let app = output.join("Game.app");
        std::fs::create_dir_all(app.join("Contents/Resources")).unwrap();
        let mut info = BuildInfo {
            version: "v001".into(),
            console: Console::Snes,
            rom_sha256: String::new(),
            core_sha256: String::new(),
        };
        std::fs::write(
            app.join(romforge_format::BUILD_INFO),
            serde_json::to_vec(&info).unwrap(),
        )
        .unwrap();
        assert!(matches!(built_state(&app), Built::Outdated(_)));
        info.version = "v999".into();
        std::fs::write(
            app.join(romforge_format::BUILD_INFO),
            serde_json::to_vec(&info).unwrap(),
        )
        .unwrap();
        assert!(matches!(built_state(&app), Built::Current(_)));
        let a = dir.path().join("Same.nes");
        let b = dir.path().join("Same.sfc");
        std::fs::write(&a, b"test").unwrap();
        std::fs::write(&b, b"test").unwrap();
        let entries = inspect(
            vec![a, b],
            output,
            icons::Index::default(),
            Default::default(),
        );
        assert!(entries[0].ready(false));
        assert!(!entries[1].ready(false));
        assert!(entries[0].matches(Filter::MissingCover));
    }
}
