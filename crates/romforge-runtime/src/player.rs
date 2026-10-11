use crate::{
    gpu::{Renderer, Shader},
    menu::{Action, LABELS, Menu, View},
    states,
};
use anyhow::{Context, Result, ensure};
use romforge_core::Session;
use sdl3::{
    audio::{AudioFormat, AudioSpec},
    event::{Event, WindowEvent},
    gamepad::{Axis, Button},
    keyboard::{Keycode, Scancode},
    rect::Rect,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::Path,
    time::{Duration, Instant},
};

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    fullscreen: bool,
    scanlines: bool,
    shader: Option<Shader>,
    keyboard: [String; 12],
    gamepad: [String; 12],
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            fullscreen: true,
            scanlines: true,
            shader: None,
            keyboard: KEYS.map(|k| k.name().to_string()),
            gamepad: BUTTONS.map(|b| b.string()),
        }
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file =
        tempfile::NamedTempFile::new_in(path.parent().context("save directory missing")?)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    Ok(())
}
pub(crate) fn destination(width: u32, height: u32, aspect: f32) -> Rect {
    let mut w = width as f32;
    let mut h = w / aspect;
    if h > height as f32 {
        h = height as f32;
        w = h * aspect;
    }
    Rect::new(
        ((width as f32 - w) / 2.0) as i32,
        ((height as f32 - h) / 2.0) as i32,
        (w as u32).max(1),
        (h as u32).max(1),
    )
}
const KEYS: [Scancode; 12] = [
    Scancode::Z,
    Scancode::A,
    Scancode::RShift,
    Scancode::Return,
    Scancode::Up,
    Scancode::Down,
    Scancode::Left,
    Scancode::Right,
    Scancode::X,
    Scancode::S,
    Scancode::Q,
    Scancode::W,
];
const BUTTONS: [Button; 12] = [
    Button::South,
    Button::West,
    Button::Back,
    Button::Start,
    Button::DPadUp,
    Button::DPadDown,
    Button::DPadLeft,
    Button::DPadRight,
    Button::East,
    Button::North,
    Button::LeftShoulder,
    Button::RightShoulder,
];

fn assign_binding(bindings: &mut [String; 12], index: usize, value: String) {
    if let Some(other) = bindings.iter().position(|b| *b == value) {
        bindings.swap(index, other);
    } else {
        bindings[index] = value;
    }
}
fn axis_direction(value: i16) -> i8 {
    if value < -16000 {
        -1
    } else if value > 16000 {
        1
    } else {
        0
    }
}
fn pad_pressed(pad: &sdl3::gamepad::Gamepad, binding: &str) -> bool {
    if let Some(rest) = binding.strip_prefix("axis:") {
        if let Some((axis, sign)) = rest.split_once(':') {
            return Axis::from_string(axis).is_some_and(|a| {
                if sign == "+" {
                    pad.axis(a) > 16000
                } else {
                    pad.axis(a) < -16000
                }
            });
        }
        false
    } else {
        Button::from_string(binding).is_some_and(|b| pad.button(b))
    }
}

pub fn play(
    core: &Path,
    rom: Vec<u8>,
    extension: &str,
    title: &str,
    windowed: bool,
    smoke_frames: Option<usize>,
    shader_override: Option<Shader>,
) -> Result<()> {
    let game_hash: [u8; 32] = Sha256::digest(&rom).into();
    let core_hash: [u8; 32] = Sha256::digest(fs::read(core)?).into();
    let game_id = format!("{:x}", Sha256::digest(&rom));
    let save_location = crate::save_location::open(title, &game_id)?;
    let saves = &save_location.path;
    let settings_path = saves.join("settings.json");
    let mut settings: Settings = match fs::read(&settings_path) {
        Ok(bytes) => serde_json::from_slice(&bytes).context("invalid saved settings")?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Settings::default(),
        Err(e) => return Err(e.into()),
    };
    let save_path = saves.join("battery.srm");
    // SAFETY: this is a trusted local Libretro core, owned by the session on the main thread.
    let mut session = unsafe { Session::load(core, rom, extension) }?;
    if save_path.exists() {
        ensure!(
            fs::metadata(&save_path)?.len() <= 16 * 1024 * 1024,
            "save file too large"
        );
        session.restore_ram(&fs::read(&save_path)?)?;
    }
    let mut saved_ram = session.save_ram()?;
    let av = session.av_info()?;
    let result = (|| -> Result<()> {
        let sdl = sdl3::init()?;
        let video = sdl.video()?;
        let mut window = video
            .window(title, 960, 720)
            .position_centered()
            .resizable()
            .metal_view()
            .build()?;
        window.set_minimum_size(480, 340)?;
        let mut renderer = Renderer::new(window)?;
        let mut fullscreen = settings.fullscreen && !windowed;
        renderer.window_mut().set_fullscreen(fullscreen)?;
        sdl.mouse().show_cursor(!fullscreen);
        let mut chosen_shader = shader_override.unwrap_or_else(|| {
            settings.shader.unwrap_or(if settings.scanlines {
                Shader::Lottes
            } else {
                Shader::Off
            })
        });
        renderer.select(chosen_shader);
        let mut shader_frames = 0usize;
        let smoke_start = Instant::now();
        let audio = sdl.audio()?;
        let spec = AudioSpec {
            freq: Some(av.timing.sample_rate.round() as i32),
            channels: Some(2),
            format: Some(AudioFormat::s16_sys()),
        };
        let device = audio.open_playback_device(&spec)?;
        let stream = device.open_device_stream(Some(&spec))?;
        stream.resume()?;
        let gamepads = sdl.gamepad()?;
        let mut controller = gamepads
            .gamepads()?
            .into_iter()
            .find_map(|id| gamepads.open(id).ok());
        let mut events = sdl.event_pump()?;
        let step = Duration::from_secs_f64(1.0 / av.timing.fps);
        let mut next_frame = Instant::now();
        let mut last_save = Instant::now();
        let mut last_controller_scan = Instant::now();
        let mut focused = true;
        let mut frames = 0usize;
        let mut queued_samples = 0usize;
        let mut menu = Menu::default();
        let mut paused = false;
        let mut chord_held = false;
        let mut suppress_input = false;
        let mut axis_nav = (0i8, 0i8);
        let mut pending = None;

        'game: loop {
            for event in events.poll_iter() {
                match event {
                    Event::Quit { .. } => break 'game,
                    Event::KeyDown {
                        keycode: Some(Keycode::F1),
                        repeat: false,
                        ..
                    } => {
                        menu.toggle();
                        suppress_input = true;
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        repeat: false,
                        ..
                    } => {
                        break 'game;
                    }
                    Event::KeyDown {
                        scancode: Some(key),
                        repeat: false,
                        ..
                    } if menu.open => {
                        if menu.capturing && !menu.controller {
                            if key != Scancode::F11 {
                                assign_binding(
                                    &mut settings.keyboard,
                                    menu.selected,
                                    key.name().to_string(),
                                );
                                menu.capturing = false;
                                menu.message = "Binding saved".into();
                                atomic_write(
                                    &settings_path,
                                    &serde_json::to_vec_pretty(&settings)?,
                                )?;
                            }
                        } else if !menu.capturing {
                            pending = match key {
                                Scancode::Up => Some(Action::Up),
                                Scancode::Down => Some(Action::Down),
                                Scancode::Left => Some(Action::Left),
                                Scancode::Right => Some(Action::Right),
                                Scancode::Return | Scancode::Space => Some(Action::Confirm),
                                Scancode::Backspace => Some(Action::Back),
                                _ => None,
                            };
                        }
                    }
                    Event::GamepadButtonDown { which, button, .. }
                        if menu.open
                            && controller
                                .as_ref()
                                .is_some_and(|c| c.id().ok() == Some(which)) =>
                    {
                        if menu.capturing && menu.controller {
                            if button != Button::Guide {
                                assign_binding(
                                    &mut settings.gamepad,
                                    menu.selected,
                                    button.string(),
                                );
                                menu.capturing = false;
                                menu.message = "Binding saved".into();
                                atomic_write(
                                    &settings_path,
                                    &serde_json::to_vec_pretty(&settings)?,
                                )?;
                            }
                        } else if !menu.capturing {
                            pending = match button {
                                Button::DPadUp => Some(Action::Up),
                                Button::DPadDown => Some(Action::Down),
                                Button::DPadLeft => Some(Action::Left),
                                Button::DPadRight => Some(Action::Right),
                                Button::East => Some(Action::Confirm),
                                Button::South => Some(Action::Back),
                                _ => None,
                            };
                        }
                    }
                    Event::GamepadAxisMotion {
                        which, axis, value, ..
                    } if menu.open
                        && menu.capturing
                        && menu.controller
                        && value.unsigned_abs() > 20000
                        && controller
                            .as_ref()
                            .is_some_and(|c| c.id().ok() == Some(which)) =>
                    {
                        assign_binding(
                            &mut settings.gamepad,
                            menu.selected,
                            format!(
                                "axis:{}:{}",
                                axis.string(),
                                if value > 0 { "+" } else { "-" }
                            ),
                        );
                        menu.capturing = false;
                        menu.message = "Binding saved".into();
                        atomic_write(&settings_path, &serde_json::to_vec_pretty(&settings)?)?;
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::F11),
                        repeat: false,
                        ..
                    } => {
                        fullscreen = !fullscreen;
                        renderer.window_mut().set_fullscreen(fullscreen)?;
                        sdl.mouse().show_cursor(!fullscreen);
                        settings.fullscreen = fullscreen;
                    }
                    Event::Window {
                        win_event: WindowEvent::FocusLost,
                        ..
                    } => {
                        focused = false;
                        stream.pause()?;
                        stream.clear()?;
                        session.set_buttons(0);
                    }
                    Event::Window {
                        win_event: WindowEvent::FocusGained,
                        ..
                    } => {
                        focused = true;
                        if !menu.open {
                            stream.resume()?;
                        }
                        next_frame = Instant::now();
                    }
                    Event::GamepadAdded { which, .. } => {
                        if controller.as_ref().is_none_or(|c| !c.connected()) {
                            controller = gamepads.open(which).ok();
                        }
                    }
                    Event::GamepadRemoved { .. }
                        if controller.as_ref().is_some_and(|c| !c.connected()) =>
                    {
                        controller = gamepads
                            .gamepads()?
                            .into_iter()
                            .find_map(|id| gamepads.open(id).ok());
                    }
                    _ => {}
                }
            }
            // Recover a late Bluetooth connection even if the device event was missed.
            if last_controller_scan.elapsed() >= Duration::from_secs(1) {
                if controller.as_ref().is_none_or(|c| !c.connected()) {
                    controller = gamepads
                        .gamepads()
                        .unwrap_or_default()
                        .into_iter()
                        .find_map(|id| gamepads.open(id).ok());
                }
                last_controller_scan = Instant::now();
            }
            if !focused && smoke_frames.is_none() {
                std::thread::sleep(Duration::from_millis(20));
                continue;
            }
            let chord = controller.as_ref().is_some_and(|c| {
                c.button(Button::Guide) || (c.button(Button::Back) && c.button(Button::Start))
            });
            if chord && !chord_held {
                menu.toggle();
                pending = None;
                suppress_input = true;
            }
            chord_held = chord;
            if menu.open {
                if let Some(pad) = &controller {
                    let axes = (
                        axis_direction(pad.axis(Axis::LeftX)),
                        axis_direction(pad.axis(Axis::LeftY)),
                    );
                    if !menu.capturing && pending.is_none() {
                        if axes.1 != 0 && axes.1 != axis_nav.1 {
                            pending = Some(if axes.1 < 0 { Action::Up } else { Action::Down });
                        } else if axes.0 != 0 && axes.0 != axis_nav.0 {
                            pending = Some(if axes.0 < 0 {
                                Action::Left
                            } else {
                                Action::Right
                            });
                        }
                    }
                    axis_nav = axes;
                }
                if let Some(action) = pending.take() {
                    if matches!(action, Action::Confirm) {
                        match menu.view {
                            View::Main => match menu.selected {
                                0 => {
                                    menu.view = View::Keys;
                                    menu.selected = 0;
                                }
                                1 => {
                                    menu.view = View::Save;
                                    menu.selected = 0;
                                }
                                2 => {
                                    menu.view = View::Load;
                                    menu.selected = 0;
                                }
                                3 => {
                                    menu.view = View::Shaders;
                                    menu.selected = Shader::ALL
                                        .iter()
                                        .position(|s| *s == chosen_shader)
                                        .unwrap_or(0);
                                }
                                4 => {
                                    fullscreen = !fullscreen;
                                    renderer.window_mut().set_fullscreen(fullscreen)?;
                                    sdl.mouse().show_cursor(!fullscreen);
                                    settings.fullscreen = fullscreen;
                                }
                                5 => break 'game,
                                _ => {}
                            },
                            View::Keys => {
                                if menu.controller && controller.is_none() {
                                    menu.message = "Connect a controller first".into();
                                } else {
                                    menu.capturing = true;
                                    menu.message = "Press a key, button, or move an axis".into();
                                }
                            }
                            View::Shaders => {
                                chosen_shader = Shader::ALL[menu.selected];
                                renderer.select(chosen_shader);
                                settings.shader = Some(chosen_shader);
                                settings.scanlines = chosen_shader != Shader::Off;
                                menu.message.clear();
                            }
                            View::Save | View::Load => {
                                let path = saves.join(format!("slot-{}.state", menu.selected + 1));
                                let result = if menu.view == View::Save {
                                    session
                                        .save_state()
                                        .and_then(|data| {
                                            states::encode(&data, &game_hash, &core_hash)
                                        })
                                        .and_then(|data| atomic_write(&path, &data))
                                } else {
                                    states::read(&path, &game_hash, &core_hash)
                                        .and_then(|data| session.load_state(&data))
                                        .map(|()| renderer.reset_history())
                                };
                                menu.message = match result {
                                    Ok(()) => {
                                        if menu.view == View::Save {
                                            "State saved".into()
                                        } else {
                                            "State loaded".into()
                                        }
                                    }
                                    Err(error) => format!("{error}"),
                                };
                            }
                        }
                        atomic_write(&settings_path, &serde_json::to_vec_pretty(&settings)?)?;
                    } else {
                        menu.navigate(action);
                    }
                }
            } else {
                pending = None;
            }
            if paused != menu.open {
                paused = menu.open;
                stream.clear()?;
                if paused {
                    stream.pause()?;
                } else {
                    stream.resume()?;
                }
                session.set_buttons(0);
                suppress_input = true;
                next_frame = Instant::now();
            }
            let keys = events.keyboard_state();
            let mut buttons = 0u16;
            for i in 0..12 {
                if Scancode::from_name(&settings.keyboard[i])
                    .is_some_and(|key| keys.is_scancode_pressed(key))
                    || controller
                        .as_ref()
                        .is_some_and(|c| pad_pressed(c, &settings.gamepad[i]))
                {
                    buttons |= 1 << i;
                }
            }
            if let Some(pad) = &controller {
                for (index, axis, positive) in [
                    (6, Axis::LeftX, false),
                    (7, Axis::LeftX, true),
                    (4, Axis::LeftY, false),
                    (5, Axis::LeftY, true),
                ] {
                    if settings.gamepad[index] == BUTTONS[index].string()
                        && (if positive {
                            pad.axis(axis) > 12000
                        } else {
                            pad.axis(axis) < -12000
                        })
                    {
                        buttons |= 1 << index;
                    }
                }
            }
            if !focused {
                buttons = 0;
            }
            if suppress_input {
                if buttons == 0 && !chord {
                    suppress_input = false;
                }
                buttons = 0;
            }
            if !paused {
                session.set_buttons(buttons);
                session.run_frames(1)?;
            }
            let info = session.av_info()?;
            let presented = session.output(|frame, samples| {
                // Bound the queue to 150ms if a device stalls; never accumulate seconds of latency.
                if stream.queued_bytes()? > (av.timing.sample_rate * 4.0 * 0.15) as i32 {
                    stream.clear()?;
                }
                if !paused {
                    stream.put_data_i16(samples)?;
                    queued_samples += samples.len();
                }
                let ratio = info.geometry.aspect_ratio;
                let aspect = if ratio.is_finite() && ratio > 0.0 {
                    ratio
                } else {
                    frame.width as f32 / frame.height.max(1) as f32
                };
                let rows: Vec<String> = match menu.view {
                    View::Main => vec![
                        "Keybinds".into(),
                        "Save State".into(),
                        "Load State".into(),
                        format!("Scanlines ({})", chosen_shader.name()),
                        "Fullscreen / Windowed".into(),
                        "Exit Game".into(),
                    ],
                    View::Keys => LABELS
                        .iter()
                        .enumerate()
                        .map(|(i, label)| {
                            format!(
                                "{label:<8} {}{}",
                                if menu.controller {
                                    &settings.gamepad[i]
                                } else {
                                    &settings.keyboard[i]
                                },
                                if menu.capturing && menu.selected == i {
                                    " <press input>"
                                } else {
                                    ""
                                }
                            )
                        })
                        .collect(),
                    View::Save | View::Load => (1..=5)
                        .map(|slot| {
                            format!(
                                "Slot {slot}  [{}]",
                                if saves.join(format!("slot-{slot}.state")).exists() {
                                    "Saved"
                                } else {
                                    "Empty"
                                }
                            )
                        })
                        .collect(),
                    View::Shaders => Shader::ALL
                        .iter()
                        .map(|shader| {
                            format!(
                                "{}{}",
                                shader.name(),
                                if *shader == renderer.active {
                                    "  [Active]"
                                } else {
                                    ""
                                }
                            )
                        })
                        .collect(),
                };
                let footer = if menu.view == View::Keys {
                    "Left/Right: device  Enter: remap  Backspace: back"
                } else if menu.view != View::Main {
                    "Circle: select  Cross / Backspace: back"
                } else {
                    "D-pad: move  Circle: select  Cross: back"
                };
                let status = if menu.view == View::Shaders {
                    renderer.notice.clone()
                } else {
                    controller
                        .as_ref()
                        .filter(|c| c.connected())
                        .map(|c| {
                            format!(
                                "Controller: {}",
                                c.name().unwrap_or_else(|| "Connected".into())
                            )
                        })
                        .unwrap_or_else(|| "Controller: not connected".into())
                };
                renderer.render(
                    &frame.rgba,
                    frame.width,
                    frame.height,
                    aspect,
                    !paused,
                    if menu.open {
                        Some((&menu, &rows, footer, &status))
                    } else {
                        None
                    },
                )
            })?;
            if !paused {
                frames += 1;
            }
            if presented && renderer.active == chosen_shader && !paused {
                shader_frames += 1;
            }
            if smoke_frames.is_some_and(|limit| shader_frames >= limit) {
                println!(
                    "GPU smoke test: rendered {frames} frames ({shader_frames} with {}), queued {queued_samples} audio samples",
                    renderer.active.name()
                );
                break;
            }
            if smoke_frames.is_some() && smoke_start.elapsed() > Duration::from_secs(90) {
                anyhow::bail!("Shader smoke test timed out: {}", renderer.notice);
            }
            if last_save.elapsed() >= Duration::from_secs(5) {
                let ram = session.save_ram()?;
                if !ram.is_empty() && ram != saved_ram {
                    atomic_write(&save_path, &ram)?;
                    saved_ram = ram;
                }
                last_save = Instant::now();
            }
            next_frame += if paused {
                Duration::from_secs_f64(1.0 / 60.0)
            } else {
                step
            };
            let now = Instant::now();
            if next_frame > now {
                std::thread::sleep(next_frame - now);
            } else if now.duration_since(next_frame) > Duration::from_millis(100) {
                next_frame = now;
            }
        }
        Ok(())
    })();
    // Persist on both normal exit and recoverable playback errors, before unloading the core.
    let ram = session.save_ram()?;
    if !ram.is_empty() {
        atomic_write(&save_path, &ram)?;
    }
    atomic_write(&settings_path, &serde_json::to_vec_pretty(&settings)?)?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_settings_gain_defaults_and_rebinding_swaps_conflicts() {
        let settings: Settings = serde_json::from_str(r#"{"fullscreen":false}"#).unwrap();
        assert!(!settings.fullscreen);
        assert!(settings.scanlines);
        for old in ["Guest", "Royale"] {
            let migrated: Settings =
                serde_json::from_value(serde_json::json!({"shader": old})).unwrap();
            assert_eq!(migrated.shader, Some(Shader::Lottes));
        }
        assert_eq!(settings.keyboard[0], "Z");
        let mut keys = settings.keyboard;
        let original = keys[8].clone();
        assign_binding(&mut keys, 0, original.clone());
        assert_eq!(keys[0], original);
        assert_eq!(keys[8], "Z");
    }
    #[test]
    fn letterboxes_without_stretching() {
        let wide = destination(1920, 1080, 4.0 / 3.0);
        assert_eq!(
            (wide.x(), wide.y(), wide.width(), wide.height()),
            (240, 0, 1440, 1080)
        );
        let tall = destination(800, 1000, 4.0 / 3.0);
        assert_eq!(
            (tall.x(), tall.y(), tall.width(), tall.height()),
            (0, 200, 800, 600)
        );
    }
    #[test]
    fn replacing_save_is_atomic_and_complete() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("battery.srm");
        atomic_write(&path, &[1; 8192]).unwrap();
        atomic_write(&path, &[2; 8192]).unwrap();
        assert_eq!(fs::read(path).unwrap(), vec![2; 8192]);
    }
}
