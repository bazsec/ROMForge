use super::*;
impl Forge {
    pub fn refresh_icons(&mut self) {
        self.icon_index = icons::Index::scan(&self.icons);
        self.indexed_folder = self.icons.clone();
        self.preview_path = None;
        self.preview_texture = None;
    }
    pub fn persist_preferences(&mut self) {
        let prefs = preferences::Preferences {
            input: self.input.clone(),
            output: self.output.clone(),
            icons: self.icons.clone(),
            overrides: self.overrides.clone(),
        };
        if prefs != self.saved_preferences {
            if let Err(e) = preferences::save(&prefs) {
                self.log = format!("Could not save preferences: {e}");
            }
            self.saved_preferences = prefs;
        }
    }
    fn matched_icon(&self, rom: &std::path::Path) -> icons::Match {
        self.overrides
            .get(rom)
            .cloned()
            .map(icons::Match::Found)
            .unwrap_or_else(|| self.icon_index.find(rom))
    }
    pub fn cover_ui(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        rom: &std::path::Path,
        busy: bool,
    ) {
        let matched = self.matched_icon(rom);
        let path = if let icons::Match::Found(path) = &matched {
            Some(path.clone())
        } else {
            None
        };
        if self.preview_path != path {
            self.preview_path = path.clone();
            self.preview_texture = None;
            self.preview_error = None;
            if let Some(path) = &path {
                let load = || -> Result<image::DynamicImage, String> {
                    if path
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("icns"))
                    {
                        let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
                        let png = temp.path().join("preview.png");
                        let result = Command::new("sips")
                            .args(["-s", "format", "png"])
                            .arg(path)
                            .arg("--out")
                            .arg(&png)
                            .output()
                            .map_err(|e| e.to_string())?;
                        if !result.status.success() {
                            return Err("ICNS preview unavailable".into());
                        }
                        image::open(png).map_err(|e| e.to_string())
                    } else {
                        image::open(path).map_err(|e| e.to_string())
                    }
                };
                match load() {
                    Ok(image) => {
                        let image = image.thumbnail(256, 256).to_rgba8();
                        self.preview_texture = Some(ctx.load_texture(
                            "cover-preview",
                            egui::ColorImage::from_rgba_unmultiplied(
                                [image.width() as usize, image.height() as usize],
                                image.as_raw(),
                            ),
                            egui::TextureOptions::LINEAR,
                        ));
                    }
                    Err(e) => self.preview_error = Some(e),
                }
            }
        }
        if let Some(texture) = &self.preview_texture {
            ui.add(egui::Image::new(texture).max_size(egui::vec2(144.0, 144.0)));
        }
        match matched {
            icons::Match::Found(path) => {
                ui.label(if self.overrides.contains_key(rom) {
                    "Selected cover"
                } else {
                    "Matching cover"
                });
                ui.add(egui::Label::new(path.display().to_string()).wrap());
            }
            icons::Match::Missing => {
                ui.label("No matching cover — default app icon will be used.");
            }
            icons::Match::Ambiguous(paths) => {
                ui.colored_label(
                    Color32::from_rgb(255, 185, 76),
                    "Multiple covers match — choose one before building.",
                );
                for path in paths {
                    if ui
                        .add_enabled(
                            !busy,
                            egui::Button::new(
                                path.strip_prefix(&self.icons)
                                    .unwrap_or(&path)
                                    .display()
                                    .to_string(),
                            ),
                        )
                        .clicked()
                    {
                        self.overrides.insert(rom.to_path_buf(), path);
                    }
                }
            }
        }
        if let Some(error) = &self.preview_error {
            ui.colored_label(Color32::LIGHT_RED, format!("Preview unavailable: {error}"));
        }
        ui.add_enabled_ui(!busy, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("Choose cover…").clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .add_filter("Icons", &["png", "icns"])
                        .set_directory(&self.icons)
                        .pick_file()
                {
                    self.overrides.insert(rom.to_path_buf(), path);
                }
                if self.overrides.contains_key(rom) && ui.button("Use automatic match").clicked() {
                    self.overrides.remove(rom);
                }
            });
        });
    }
    fn folders_ui(&mut self, ui: &mut egui::Ui) {
        let mut rescan = false;
        ui.add_enabled_ui(self.job.is_none(), |ui| {
            for (label, path) in [
                ("ROM folder", &mut self.input),
                ("Output folder", &mut self.output),
                ("Icons folder", &mut self.icons),
            ] {
                ui.label(RichText::new(label).strong());
                ui.horizontal_wrapped(|ui| {
                    if ui.button(format!("Choose {label}…")).clicked()
                        && let Some(chosen) =
                            rfd::FileDialog::new().set_directory(&*path).pick_folder()
                    {
                        *path = chosen;
                        rescan = true;
                    }
                    ui.add(egui::Label::new(path.display().to_string()).wrap());
                });
            }
        });
        if rescan {
            self.refresh_icons();
            self.scan();
        }
    }
    fn updates_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.label(format!(
            "ROMForge {} · macOS Apple Silicon",
            updater::VERSION.trim()
        ));
        if let Some(rx) = &self.update_job {
            match rx.try_recv() {
                Ok(result) => {
                    self.update_job = None;
                    match result {
                        Ok(UpdateEvent::Checked(release)) => {
                            self.update_status = if release.is_some() {
                                "A new version is available.".into()
                            } else {
                                "You are up to date.".into()
                            };
                            self.available_update = release;
                        }
                        Ok(UpdateEvent::Prepared(script)) => {
                            match Command::new("sh").arg(script).spawn() {
                                Ok(_) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                                Err(e) => self.update_status = e.to_string(),
                            }
                        }
                        Err(e) => self.update_status = e,
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.update_job = None;
                    self.update_status = "Update worker stopped. Please retry.".into();
                }
                Err(mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(Duration::from_millis(100));
                }
            }
        }
        if ui
            .add_enabled(
                self.update_job.is_none(),
                egui::Button::new("Check for updates"),
            )
            .clicked()
        {
            let (tx, rx) = mpsc::channel();
            self.update_job = Some(rx);
            self.update_status = "Checking GitHub Releases…".into();
            std::thread::spawn(move || {
                let _ = tx.send(updater::check().map(UpdateEvent::Checked));
            });
        }
        if let Some(release) = self.available_update.clone() {
            ui.label(format!("Available: {}", release.tag_name));
            ui.hyperlink_to("Release notes", &release.html_url);
            if ui
                .add_enabled(
                    self.update_job.is_none() && self.job.is_none(),
                    egui::Button::new("Install update and restart"),
                )
                .clicked()
            {
                let (tx, rx) = mpsc::channel();
                self.update_job = Some(rx);
                self.update_status = "Downloading and verifying update…".into();
                std::thread::spawn(move || {
                    let _ = tx.send(updater::prepare(&release).map(UpdateEvent::Prepared));
                });
            }
        }
        ui.label(&self.update_status);
        ui.label("Updates preserve your folders and preferences. Builds are locally signed, not Apple-notarized.");
    }
    pub fn settings_page(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(egui::Frame::new().inner_margin(28).fill(Color32::from_rgb(20,22,27))).show(ctx, |ui| {
            egui::ScrollArea::vertical().id_salt("settings-page").show_branded(ui, |ui| {
                ui.heading("Settings");
                ui.label("Your folders and cover selections are remembered automatically.");
                ui.separator(); self.folders_ui(ui);
                ui.separator();
                ui.heading("Updates");
                self.updates_ui(ui,ctx);
                ui.separator();
                ui.heading("How to use ROMForge");
                ui.label("Choose a ROM folder to load .sfc and .smc games, including subfolders. Extract ZIP archives first. Select a game, check its cover, then Build macOS app. You can also drop one ROM into the library to build it immediately.");
                ui.label("Create all opens a review of the entire library, including games hidden by search. Existing apps are preserved. Cancel stops after the current game. Every new app receives an automatic headless launch check.");
                ui.label("Cover matching searches subfolders, ignores region and revision tags, and uses the reviewed title aliases. Ambiguous matches need a manual selection. Refresh rescans both games and covers.");
                if !self.icon_index.warnings.is_empty() { ui.colored_label(Color32::LIGHT_RED,format!("{} icon folders or entries could not be read.",self.icon_index.warnings.len())); for warning in &self.icon_index.warnings {ui.label(warning);} }
                ui.separator(); ui.heading("Playing and saving");
                ui.label("Games open fullscreen. F1 opens controls, save states and CRT settings; F11 toggles fullscreen. Focus the game window for controller input. Saves live in Save Games beside your game apps, with a separate folder per game. Keep this folder when moving your collection.");
                ui.separator(); ui.heading("Personal use only");
                ui.label("Use ROMForge only for personal use with ROMs you own and have permission to use. ROMForge does not provide ROMs. Do not distribute, upload, share or sell ROMs or generated apps containing ROMs. Owning a game does not grant redistribution rights.");
                ui.label("Snes9x is provided under its noncommercial license. Third-party license notices are included with the app.");
            });
        });
    }
    pub fn batch_page(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(egui::Frame::new().inner_margin(28).fill(Color32::from_rgb(20,22,27))).show(ctx, |ui| {
            ui.heading("Review batch build");
            let mut seen = std::collections::BTreeSet::new();
            let mut ready = Vec::new(); let mut rows = Vec::new(); let (mut skipped,mut missing,mut blocked) = (0,0,0);
            for rom in &self.roms {
                let title = rom.file_stem().unwrap_or_default().to_string_lossy();
                let destination = self.output.join(format!("{title}.app"));
                let status = if destination.exists() || !seen.insert(title.to_lowercase()) { skipped += 1; "Skip — existing app or duplicate output name" }
                else { match self.matched_icon(rom) {
                    icons::Match::Ambiguous(_) => { blocked += 1; "Needs cover selection" }
                    icons::Match::Found(path) if !path.is_file() => {blocked += 1; "Selected cover is missing"}
                    icons::Match::Missing => {missing += 1; ready.push(rom.clone()); "Build — default icon"}
                    _ => {ready.push(rom.clone()); "Build — cover matched"}
                }};
                rows.push((title.to_string(),status));
            }
            ui.label(format!("{} ready · {skipped} skipped · {missing} without covers · {blocked} need attention",ready.len()));
            ui.add(egui::Label::new(format!("Destination: {}",self.output.display())).wrap());
            ui.label("Existing apps are never replaced. Resolve cover conflicts in the library or build only the ready games.");
            if ui.add_enabled(self.job.is_none() && !ready.is_empty(),egui::Button::new(format!("Build {} ready games",ready.len())).fill(Color32::from_rgb(93,66,30))).clicked() { self.start_roms(ready,true,true); self.batch_preview=false; }
            ui.separator();
            egui::ScrollArea::vertical().id_salt("batch-review").show_branded(ui, |ui| {
                for (title,status) in rows { ui.label(RichText::new(title).strong()); ui.label(status); ui.separator(); }
            });
        });
    }
}
