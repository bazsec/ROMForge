#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod batch;
mod builds;
mod icons;
mod library;
mod pages;
mod preferences;
mod rom_drop;
mod updater;
use eframe::egui::{self, Color32, RichText};
use std::{
    path::PathBuf,
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    time::Duration,
};

// Scope handle colors to the scroll area, preserving the content widget theme.
trait BrandedScrollArea {
    fn show_branded(self, ui: &mut egui::Ui, contents: impl FnOnce(&mut egui::Ui));
}
impl BrandedScrollArea for egui::ScrollArea {
    fn show_branded(self, ui: &mut egui::Ui, contents: impl FnOnce(&mut egui::Ui)) {
        let content_style = ui.style().clone();
        ui.scope(|ui| {
            let (now, scrolling) =
                ui.input(|input| (input.time, input.smooth_scroll_delta != egui::Vec2::ZERO));
            let activity_id = ui.id().with("scrollbar-last-scroll");
            // Context queries must happen before data_mut acquires its write lock.
            let scrolling_here = scrolling && ui.rect_contains_pointer(ui.max_rect());
            let last_scroll = ui.ctx().data_mut(|data| {
                if scrolling_here {
                    data.insert_temp(activity_id, now);
                }
                data.get_temp::<f64>(activity_id)
                    .unwrap_or(f64::NEG_INFINITY)
            });
            let recently_scrolled = now - last_scroll < 0.7;
            ui.style_mut().spacing.scroll.active_handle_opacity =
                if recently_scrolled { 0.35 } else { 0.0 };
            if recently_scrolled {
                ui.ctx().request_repaint_after(Duration::from_millis(100));
            }
            let widgets = &mut ui.visuals_mut().widgets;
            for state in [
                &mut widgets.noninteractive,
                &mut widgets.inactive,
                &mut widgets.hovered,
                &mut widgets.active,
                &mut widgets.open,
            ] {
                state.fg_stroke.color = Color32::from_rgb(255, 183, 66);
            }
            self.show(ui, |ui| {
                ui.set_style(content_style);
                contents(ui);
            });
        });
    }
}

enum UpdateEvent {
    Checked(Option<updater::Release>),
    Prepared(PathBuf),
}

enum JobEvent {
    Progress(usize, usize, String),
    Finished(batch::Report),
}

struct Forge {
    logo: egui::TextureHandle,
    wordmark: egui::TextureHandle,
    wordmark_uv: egui::Rect,
    wordmark_size: egui::Vec2,
    search: String,
    filter: library::Filter,
    entries: Vec<library::Entry>,
    library_dirty: bool,
    library_job: Option<Receiver<Vec<library::Entry>>>,
    rebuild_outdated: bool,
    failed_roms: Vec<PathBuf>,
    retry_replace: bool,
    progress: (usize, usize),
    cover_open: bool,
    settings_open: bool,
    batch_preview: bool,
    update_job: Option<Receiver<Result<UpdateEvent, String>>>,
    available_update: Option<updater::Release>,
    update_status: String,
    icon_index: icons::Index,
    indexed_folder: PathBuf,
    overrides: std::collections::BTreeMap<PathBuf, PathBuf>,
    saved_preferences: preferences::Preferences,
    preview_path: Option<PathBuf>,
    preview_texture: Option<egui::TextureHandle>,
    preview_error: Option<String>,
    cancel: Arc<AtomicBool>,
    batch_running: bool,
    root: PathBuf,
    input: PathBuf,
    output: PathBuf,
    icons: PathBuf,
    roms: Vec<PathBuf>,
    selected: Option<usize>,
    status: String,
    log: String,
    job: Option<Receiver<JobEvent>>,
}
fn project_root() -> PathBuf {
    if let Some(root) = std::env::var_os("ROMFORGE_PROJECT") {
        return root.into();
    }
    if cfg!(windows)
        && let Ok(exe) = std::env::current_exe()
        && let Some(directory) = exe.parent()
    {
        if let Ok(root) = std::fs::read_to_string(directory.join("Contents/Resources/project-root"))
        {
            return PathBuf::from(root.trim());
        }
        if directory.join("Contents/Frameworks").is_dir() {
            return directory.to_path_buf();
        }
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(macos) = exe.parent()
        && let Some(contents) = macos.parent()
        && let Ok(root) = std::fs::read_to_string(contents.join("Resources/project-root"))
    {
        return PathBuf::from(root.trim());
    }
    if let Ok(exe) = std::env::current_exe()
        && exe
            .ancestors()
            .any(|p| p.extension().is_some_and(|e| e == "app"))
    {
        return PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
            .join("Documents/ROMForge");
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
impl Forge {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        cc.egui_ctx.style_mut(|style| {
            // Hide idle handles, retaining a wider grab target on hover.
            style.spacing.scroll = egui::style::ScrollStyle {
                bar_width: 7.0,
                floating_width: 3.0,
                floating_allocated_width: 8.0,
                handle_min_length: 28.0,
                dormant_background_opacity: 0.0,
                active_background_opacity: 0.0,
                interact_background_opacity: 0.15,
                dormant_handle_opacity: 0.0,
                active_handle_opacity: 0.35,
                interact_handle_opacity: 0.65,
                ..egui::style::ScrollStyle::floating()
            };
            style.spacing.item_spacing = egui::vec2(12.0, 12.0);
            style.spacing.button_padding = egui::vec2(14.0, 9.0);
            style.visuals.panel_fill = Color32::from_rgb(20, 22, 27);
            style.visuals.window_fill = Color32::from_rgb(28, 31, 37);
            style.visuals.extreme_bg_color = Color32::from_rgb(16, 18, 22);
            style.visuals.selection.bg_fill = Color32::from_rgb(93, 66, 30);
            style.visuals.selection.stroke.color = Color32::from_rgb(255, 202, 116);
            for state in [
                &mut style.visuals.widgets.inactive,
                &mut style.visuals.widgets.hovered,
                &mut style.visuals.widgets.active,
            ] {
                state.corner_radius = egui::CornerRadius::same(8);
                state.bg_stroke = egui::Stroke::new(1.0_f32, Color32::from_rgb(69, 60, 46));
            }
            style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(34, 39, 46);
            style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(66, 49, 29);
            style.visuals.widgets.active.weak_bg_fill = Color32::from_rgb(91, 63, 29);
            style.visuals.override_text_color = Some(Color32::from_rgb(224, 227, 233));
            style
                .text_styles
                .insert(egui::TextStyle::Body, egui::FontId::proportional(15.0));
            style
                .text_styles
                .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
        });
        let image =
            image::load_from_memory(include_bytes!("../../../assets/branding/romforge-logo.png"))
                .expect("embedded logo")
                .to_rgba8();
        let logo = cc.egui_ctx.load_texture(
            "ROMForge logo",
            egui::ColorImage::from_rgba_unmultiplied(
                [image.width() as usize, image.height() as usize],
                image.as_raw(),
            ),
            egui::TextureOptions::LINEAR,
        );
        let image = image::load_from_memory(include_bytes!(
            "../../../assets/branding/romforge-wordmark.png"
        ))
        .expect("embedded wordmark")
        .thumbnail(1536, 512)
        .to_rgba8();
        // Fit the visible lettering, excluding the generated asset's transparent margins.
        let (mut left, mut top, mut right, mut bottom) = (image.width(), image.height(), 0, 0);
        for (x, y, pixel) in image.enumerate_pixels() {
            if pixel[3] > 16 {
                left = left.min(x);
                top = top.min(y);
                right = right.max(x + 1);
                bottom = bottom.max(y + 1);
            }
        }
        let wordmark_uv = egui::Rect::from_min_max(
            egui::pos2(
                left as f32 / image.width() as f32,
                top as f32 / image.height() as f32,
            ),
            egui::pos2(
                right as f32 / image.width() as f32,
                bottom as f32 / image.height() as f32,
            ),
        );
        let wordmark_size =
            egui::vec2(350.0, 350.0 * (bottom - top) as f32 / (right - left) as f32);
        let wordmark = cc.egui_ctx.load_texture(
            "ROMForge wordmark",
            egui::ColorImage::from_rgba_unmultiplied(
                [image.width() as usize, image.height() as usize],
                image.as_raw(),
            ),
            egui::TextureOptions::LINEAR,
        );
        let root = project_root();
        let mut app = Self {
            logo,
            wordmark,
            wordmark_uv,
            wordmark_size,
            search: String::new(),
            filter: Default::default(),
            entries: Vec::new(),
            library_dirty: true,
            library_job: None,
            rebuild_outdated: false,
            failed_roms: Vec::new(),
            retry_replace: false,
            progress: (0, 0),
            cover_open: false,
            settings_open: false,
            batch_preview: false,
            update_job: None,
            available_update: None,
            update_status: String::new(),
            icon_index: icons::Index::default(),
            indexed_folder: PathBuf::new(),
            overrides: Default::default(),
            saved_preferences: Default::default(),
            preview_path: None,
            preview_texture: None,
            preview_error: None,
            cancel: Arc::new(AtomicBool::new(false)),
            batch_running: false,
            input: root.join("ROMs"),
            icons: root.join("ROMs/Icons"),
            output: root.join("artifacts"),
            root,
            roms: Vec::new(),
            selected: None,
            status: "Ready to test your ROMs".into(),
            log: String::new(),
            job: None,
        };
        match preferences::load() {
            Ok(p) => {
                if !p.input.as_os_str().is_empty() {
                    app.input = p.input.clone();
                }
                if !p.output.as_os_str().is_empty() {
                    app.output = p.output.clone();
                }
                if !p.icons.as_os_str().is_empty() {
                    app.icons = p.icons.clone();
                }
                app.overrides = p.overrides.clone();
                app.saved_preferences = p;
            }
            Err(e) => app.log = format!("Could not restore preferences: {e}"),
        }
        app.refresh_icons();
        app.scan();
        app
    }
    fn scan(&mut self) {
        self.library_dirty = true;
        self.roms.clear();
        self.selected = None;
        let mut dirs = vec![self.input.clone()];
        let mut errors = 0;
        while let Some(dir) = dirs.pop() {
            match std::fs::read_dir(dir) {
                Ok(entries) => {
                    for entry in entries {
                        let Ok(entry) = entry else {
                            errors += 1;
                            continue;
                        };
                        let Ok(kind) = entry.file_type() else {
                            errors += 1;
                            continue;
                        };
                        // Do not follow directory symlinks or accept symlinked ROMs.
                        if kind.is_dir() {
                            dirs.push(entry.path());
                        } else if kind.is_file()
                            && entry
                                .path()
                                .extension()
                                .and_then(|s| s.to_str())
                                .is_some_and(|s| {
                                    matches!(s.to_ascii_lowercase().as_str(), "nes" | "sfc" | "smc")
                                })
                        {
                            self.roms.push(entry.path());
                        }
                    }
                }
                Err(_) => errors += 1,
            }
        }
        self.roms.sort();
        if !self.roms.is_empty() {
            self.selected = Some(0);
        }
        self.status = if errors == 0 {
            format!("{} ROMs found", self.roms.len())
        } else {
            format!(
                "{} ROMs found; {errors} folders or files could not be read",
                self.roms.len()
            )
        };
    }
}
impl eframe::App for Forge {
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        self.persist_preferences();
    }
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.persist_preferences();
        if self.indexed_folder != self.icons {
            self.refresh_icons();
        }
        if let Some(rx) = &self.job {
            match rx.try_recv() {
                Ok(JobEvent::Progress(done, total, status)) => {
                    self.progress = (done, total);
                    self.status = status;
                    ctx.request_repaint();
                }
                Ok(JobEvent::Finished(report)) => {
                    self.status = report.summary();
                    self.progress.0 = report.built + report.skipped + report.failed.len();
                    self.log = report.log;
                    self.failed_roms = report.failed;
                    self.library_dirty = true;
                    self.job = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.status = "Worker stopped unexpectedly".into();
                    self.job = None;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(Duration::from_millis(100))
                }
            }
        }
        self.poll_library(ctx);
        let dropped = ctx.input(|input| input.raw.dropped_files.clone());
        if !dropped.is_empty() {
            if self.job.is_some() {
                self.status = "Please wait for the current job, then drop your ROM again.".into();
            } else {
                match rom_drop::rom_path(&dropped) {
                    Ok(rom) => {
                        if let Some(index) = self.roms.iter().position(|path| path == &rom) {
                            self.selected = Some(index);
                        } else {
                            self.roms.push(rom);
                            self.selected = Some(self.roms.len() - 1);
                        }
                        self.search.clear();
                        self.library_dirty = true;
                        self.start(true);
                    }
                    Err(message) => self.status = message.into(),
                }
            }
        }
        let busy = self.job.is_some();
        let amber = Color32::from_rgb(255, 185, 76);
        let muted = Color32::from_rgb(143, 152, 169);
        egui::TopBottomPanel::top("brand")
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(25, 28, 34))
                    .inner_margin(20),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::Image::new(&self.logo).fit_to_exact_size(egui::vec2(96.0, 96.0)));
                    ui.add(
                        egui::Image::new(&self.wordmark)
                            .uv(self.wordmark_uv)
                            .maintain_aspect_ratio(false)
                            .fit_to_exact_size(
                                self.wordmark_size
                                    * (ui.available_width() / 650.0).clamp(0.55, 1.0),
                            )
                            .alt_text("ROMForge"),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(if self.settings_open || self.batch_preview {
                                "Back to library"
                            } else {
                                "Settings"
                            })
                            .clicked()
                        {
                            if self.settings_open || self.batch_preview {
                                self.settings_open = false;
                                self.batch_preview = false;
                            } else {
                                self.settings_open = true;
                            }
                        }
                        ui.label(
                            RichText::new(format!(
                                "{} · {}",
                                std::env::consts::OS,
                                updater::VERSION.trim()
                            ))
                            .size(12.0)
                            .color(amber),
                        );
                    });
                });
            });
        egui::TopBottomPanel::bottom("activity")
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(25, 28, 34))
                    .inner_margin(18),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let batch_roms = self.ready_roms();
                    let ready = !busy && if self.batch_preview { !batch_roms.is_empty() } else { self.selected.is_some() };
                    if ui
                        .add_enabled(
                            ready && romforge_format::native_package_flag().is_some(),
                            egui::Button::new(
                                RichText::new(if self.batch_preview { "Start batch" } else if cfg!(windows) { "Build Windows game" } else { "Build macOS app" })
                                    .strong()
                                    .color(Color32::from_rgb(30, 24, 16)),
                            )
                            .fill(amber)
                            .min_size(egui::vec2(200.0, 46.0)),
                        )
                        .clicked()
                    {
                        if self.batch_preview { self.start_roms(batch_roms, self.rebuild_outdated); self.batch_preview = false; }
                        else { self.start(true); }
                    }
                    if !self.batch_preview && ui.add_enabled(
                        !busy && !self.roms.is_empty() && romforge_format::native_package_flag().is_some(),
                        egui::Button::new(format!("Create all ({})", self.roms.len()))
                            .min_size(egui::vec2(160.0, 46.0)),
                    ).on_hover_text("Build the entire library, including games hidden by search. Existing apps are skipped.").clicked() {
                        self.batch_preview = true; self.settings_open = false; self.library_dirty = true;
                    }
                    if !busy && !self.failed_roms.is_empty() && ui.button(format!("Retry failed ({})", self.failed_roms.len())).clicked() {
                        self.start_roms(self.failed_roms.clone(), self.retry_replace);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new("Open output folder").min_size(egui::vec2(160.0,46.0))).clicked()
                            && let Err(error) = std::fs::create_dir_all(&self.output).and_then(|_| Command::new(if cfg!(windows) { "explorer.exe" } else { "open" }).arg(&self.output).spawn().map(|_| ())) { self.log = error.to_string(); }
                    });
                });
                ui.horizontal(|ui| {
                    if busy {
                        ui.spinner();
                    }
                    ui.label(RichText::new(&self.status).color(if busy { amber } else { muted }));
                    if busy && self.batch_running && ui.add_enabled(!self.cancel.load(Ordering::Relaxed), egui::Button::new("Cancel after current game")).clicked() {
                        self.cancel.store(true, Ordering::Relaxed);
                        self.status = "Stopping after the current game finishes…".into();
                    }
                });
                if busy && self.progress.1 > 0 {
                    ui.add(egui::ProgressBar::new(self.progress.0 as f32 / self.progress.1 as f32)
                        .text(format!("{} / {} complete",self.progress.0,self.progress.1)));
                }
                if !self.log.is_empty() {
                    egui::CollapsingHeader::new("Activity details").show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(140.0)
                            .show_branded(ui, |ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&self.log).monospace().size(12.0),
                                    )
                                    .wrap(),
                                );
                            });
                    });
                }
            });
        if self.settings_open {
            self.settings_page(ctx);
            return;
        }
        if self.batch_preview {
            self.batch_page(ctx);
            return;
        }
        egui::SidePanel::left("library")
            .resizable(true)
            .default_width(350.0)
            .width_range(260.0..=480.0)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(23, 26, 31))
                    .inner_margin(20),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("LIBRARY").size(12.0).strong().color(muted));
                    ui.label(
                        RichText::new(format!("{} games", self.roms.len()))
                            .size(12.0)
                            .color(muted),
                    );
                });
                ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text("Search your games…")
                        .desired_width(f32::INFINITY),
                );
                ui.add_enabled_ui(!busy, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Choose ROM folder").clicked()
                            && let Some(path) = rfd::FileDialog::new()
                                .set_directory(&self.input)
                                .pick_folder()
                        {
                            self.input = path;
                            self.icons = self.input.join("Icons");
                            self.scan();
                        }
                        if ui.button("Refresh").clicked() {
                            self.refresh_icons();
                            self.scan();
                        }
                    });
                });
                ui.add(
                    egui::Label::new(
                        RichText::new(self.input.display().to_string())
                            .size(11.0)
                            .color(muted),
                    )
                    .truncate(),
                )
                .on_hover_text(self.input.display().to_string());
                egui::ComboBox::from_id_salt("library-filter")
                    .selected_text(self.filter.label())
                    .show_ui(ui, |ui| {
                        for filter in [
                            library::Filter::All,
                            library::Filter::MissingCover,
                            library::Filter::NotBuilt,
                            library::Filter::NeedsUpdate,
                        ] {
                            ui.selectable_value(&mut self.filter, filter, filter.label());
                        }
                    });
                if self.library_job.is_some() {
                    ui.label(
                        RichText::new("Checking covers and builds…")
                            .small()
                            .color(muted),
                    );
                }
                ui.separator();
                let query = self.search.to_lowercase();
                egui::ScrollArea::vertical()
                    .id_salt("library-list")
                    .auto_shrink([false, false])
                    .show_branded(ui, |ui| {
                        let mut shown = 0;
                        for (i, rom) in self.roms.iter().enumerate() {
                            let title = rom.file_stem().unwrap_or_default().to_string_lossy();
                            if !title.to_lowercase().contains(&query)
                                || (self.filter != library::Filter::All
                                    && !self
                                        .entries
                                        .iter()
                                        .find(|e| &e.rom == rom)
                                        .is_some_and(|e| e.matches(self.filter)))
                            {
                                continue;
                            }
                            shown += 1;
                            let chosen = self.selected == Some(i);
                            let button =
                                egui::Button::new(RichText::new(title.as_ref()).color(if chosen {
                                    amber
                                } else {
                                    Color32::from_rgb(215, 220, 228)
                                }))
                                .selected(chosen)
                                .fill(if chosen {
                                    Color32::from_rgb(70, 49, 27)
                                } else {
                                    Color32::TRANSPARENT
                                })
                                .min_size(egui::vec2(ui.available_width(), 46.0));
                            if ui.add_enabled(!busy, button).clicked() {
                                self.selected = Some(i);
                            }
                        }
                        if shown == 0 {
                            ui.label(if self.roms.is_empty() {
                                "Choose a folder with .nes, .sfc or .smc games to get started."
                            } else {
                                "No games match your search and filter."
                            });
                        }
                    });
            });
        egui::CentralPanel::default().frame(egui::Frame::new().fill(Color32::from_rgb(20,22,27)).inner_margin(28)).show(ctx, |ui| {
            egui::ScrollArea::vertical().id_salt("workspace").show_branded(ui, |ui| {
                if let Some(rom)=self.selected.and_then(|i| self.roms.get(i)).cloned() {
                    self.load_cover_preview(ctx, &rom);
                    ui.horizontal(|ui| {
                        if let Some(texture) = &self.preview_texture {
                            if ui.add(egui::Image::new(texture).max_size(egui::vec2(120.0, 120.0)).sense(egui::Sense::click())).on_hover_text("View or change cover").clicked() {self.cover_open=true;}
                            ui.add_space(10.0);
                        }
                        ui.vertical(|ui| {
                            ui.label(RichText::new("BUILD YOUR GAME").size(12.0).strong().color(amber));
                            ui.add_space(8.0);
                            ui.add(egui::Label::new(RichText::new(rom.file_stem().unwrap_or_default().to_string_lossy()).size(27.0).strong()).wrap());
                            let size=self.entries.iter().find(|e|e.rom==rom).map_or(0,|e|e.size);
                            let console=library::console(&rom).map_or("Unknown console",|c|c.name());
                            ui.label(RichText::new(format!("{console}   ·   {:.2} MB   ·   {}",size as f64/1_000_000.0,rom.extension().unwrap_or_default().to_string_lossy().to_uppercase())).color(muted));
                        });
                    });
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(if self.preview_texture.is_some() {"Cover matched"} else {"No cover preview"}).small().color(muted));
                        if ui.button("View / change cover…").clicked() {self.cover_open=true;}
                    });
                    if let Some(entry)=self.entries.iter().find(|e|e.rom==rom) {ui.label(RichText::new(entry.status(false)).small().color(muted));}
                    if self.cover_open {
                        let mut open=true;
                        egui::Window::new("Game cover").open(&mut open).resizable(false).show(ctx, |ui| {
                            if let Some(texture)=&self.preview_texture {ui.add(egui::Image::new(texture).max_size(egui::vec2(360.0,360.0)));}
                            self.cover_ui(ui,&rom,busy);
                        });
                        self.cover_open=open;
                    }
                } else {
                    ui.label(RichText::new("A new home for your classics.").size(27.0).strong());
                    ui.label("Select a game from the library to build its own app.");
                }
                ui.add_space(20.0);
                ui.label(RichText::new("DESTINATIONS").size(12.0).strong().color(muted));
                ui.add_enabled_ui(!busy, |ui| {
                    for (label,path) in [("Game apps", &mut self.output),("Game icons", &mut self.icons)] {
                        egui::Frame::new().fill(Color32::from_rgb(29,33,40)).corner_radius(10).inner_margin(14).show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(label).strong());
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {
                                    if ui.button("Change…").clicked() && let Some(chosen)=rfd::FileDialog::new().set_directory(&*path).pick_folder() { *path=chosen; }
                                });
                            });
                            ui.add(egui::Label::new(RichText::new(path.display().to_string()).size(12.0).color(muted)).truncate()).on_hover_text(path.display().to_string());
                        });
                    }
                    if ui.small_button("Reset icons to ROM folder / Icons").clicked() {self.icons=self.input.join("Icons");}
                });
                ui.add_space(12.0);
                ui.label(RichText::new("Fullscreen play, controllers, CRT-Lottes and the F1 menu are included. Progress lives in Save Games beside your apps.").size(13.0).color(muted));
                ui.add_space(18.0);

            });
        });
        if ctx.input(|input| !input.raw.hovered_files.is_empty()) {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("rom-drop-overlay"),
            ));
            let rect = ctx.content_rect();
            painter.rect_filled(rect, 0.0, Color32::from_black_alpha(225));
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                if busy {
                    "A job is running — drop your ROM after it finishes"
                } else {
                    "Drop one NES or SNES ROM to create its app"
                },
                egui::FontId::proportional(26.0),
                amber,
            );
        }
    }
}
fn main() -> eframe::Result {
    let icon = image::load_from_memory(include_bytes!(
        "../../../assets/branding/romforge-macos-icon.png"
    ))
    .expect("embedded app icon")
    .resize(256, 256, image::imageops::FilterType::Lanczos3)
    .to_rgba8();
    let icon = egui::IconData {
        width: icon.width(),
        height: icon.height(),
        rgba: icon.into_raw(),
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_icon(icon)
            .with_inner_size([1060.0, 810.0])
            .with_min_inner_size([820.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ROMForge",
        options,
        Box::new(|cc| Ok(Box::new(Forge::new(cc)))),
    )
}
