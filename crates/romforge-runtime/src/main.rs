#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod gpu;
mod lottes;
mod menu;
mod player;
mod save_location;
mod states;
#[cfg(windows)]
mod windows_game;
use anyhow::{Context, Result, ensure};
use romforge_core::Session;
use romforge_format::{Console, Payload, detect};
use std::{
    fs::{self, File},
    path::PathBuf,
};

fn run() -> Result<()> {
    let mut args: Vec<_> = std::env::args_os().skip(1).collect();
    #[cfg(windows)]
    if args.first().is_some_and(|s| s == "--export-resources") {
        ensure!(
            args.len() == 2,
            "--export-resources requires a new output directory"
        );
        return windows_game::export(std::path::Path::new(&args[1]));
    }
    if args.first().is_some_and(|s| s == "--controllers") {
        let sdl = sdl3::init()?;
        let gamepads = sdl.gamepad()?;
        let mut pump = sdl.event_pump()?;
        pump.pump_events();
        let ids = gamepads.gamepads()?;
        println!("SDL detects {} gamepad(s)", ids.len());
        for id in ids {
            let pad = gamepads.open(id)?;
            println!(
                "{}: connected={}, mapping={}",
                pad.name().unwrap_or_default(),
                pad.connected(),
                pad.mapping().unwrap_or_default()
            );
        }
        return Ok(());
    }
    let headless = args.first().is_some_and(|s| s == "--headless");
    if headless {
        args.remove(0);
    }
    let mut smoke_frames = None;
    if args.first().is_some_and(|s| s == "--smoke-frames") {
        ensure!(args.len() >= 2, "--smoke-frames requires a count");
        smoke_frames = Some(args[1].to_string_lossy().parse::<usize>()?);
        args.drain(..2);
    }
    let windowed = args.first().is_some_and(|s| s == "--windowed");
    if windowed {
        args.remove(0);
    }
    let shader_override = if args.first().is_some_and(|s| s == "--shader") {
        ensure!(args.len() >= 2, "--shader needs off or lottes");
        let shader = match args[1].to_string_lossy().as_ref() {
            "off" => gpu::Shader::Off,
            "lottes" => gpu::Shader::Lottes,
            _ => anyhow::bail!("Unknown shader; use off or lottes"),
        };
        args.drain(..2);
        Some(shader)
    } else {
        None
    };
    ensure!(
        args.len() <= 2,
        "usage: romforge-runtime [--headless | --smoke-frames N] [--windowed] [--shader NAME] [CORE [ROM]]"
    );
    let exe = std::env::current_exe()?;
    #[cfg(windows)]
    let mut standalone = if args.is_empty() && windows_game::embedded() {
        Some(windows_game::load()?)
    } else {
        None
    };
    let contents = if cfg!(windows) {
        exe.parent()
            .context("application directory missing")?
            .join("Contents")
    } else {
        exe.parent()
            .and_then(|p| p.parent())
            .context("application directory missing")?
            .to_path_buf()
    };
    let (rom, extension, console, title) = if args.len() == 2 {
        let path = PathBuf::from(&args[1]);
        ensure!(
            fs::metadata(&path)?.len() <= romforge_format::MAX_ROM as u64,
            "ROM too large"
        );
        let rom = fs::read(&path)?;
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .context("ROM extension missing")?
            .to_owned();
        let console = detect(&extension, &rom)?;
        let title = path
            .file_stem()
            .context("title missing")?
            .to_string_lossy()
            .into_owned();
        (rom, extension, console, title)
    } else {
        let payload_path = contents.join("Resources/game.romforge");
        #[cfg(windows)]
        let embedded_payload = standalone.as_mut().and_then(|game| game.payload.take());
        #[cfg(not(windows))]
        let embedded_payload: Option<Payload> = None;
        let Payload { rom, metadata } = if let Some(payload) = embedded_payload {
            payload
        } else if args.is_empty() {
            romforge_format::read(File::open(payload_path)?)?
        } else {
            romforge_format::read(File::open(&exe)?)?
        };
        (rom, metadata.extension, metadata.console, metadata.title)
    };
    ensure!(
        matches!(console, Console::Snes | Console::Nes),
        "runtime supports NES and SNES"
    );
    let core = args.first().map(PathBuf::from).unwrap_or_else(|| {
        #[cfg(windows)]
        if let Some(game) = &standalone {
            return game.core.clone();
        }
        contents.join(format!(
            "Frameworks/{}_libretro.{}",
            console.core_name(),
            std::env::consts::DLL_EXTENSION
        ))
    });
    if headless {
        // SAFETY: the development command accepts a trusted native core explicitly.
        let mut session = unsafe { Session::load(&core, rom, &extension) }?;
        let stats = session.run_frames(120)?;
        ensure!(
            stats.video_frames > 0,
            "core produced no software video frames"
        );
        println!(
            "Ran 120 frames: {} video callbacks, {} stereo audio frames",
            stats.video_frames, stats.audio_frames
        );
        return Ok(());
    }
    player::play(
        &core,
        rom,
        &extension,
        &title,
        windowed,
        smoke_frames,
        shader_override,
    )
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        if !std::env::args_os()
            .any(|s| s == "--headless" || s == "--smoke-frames" || s == "--export-resources")
        {
            let _ = sdl3::messagebox::show_simple_message_box(
                sdl3::messagebox::MessageBoxFlag::ERROR,
                "Unable to start game",
                &format!("{error:#}"),
                None,
            );
        }
        std::process::exit(1);
    }
}
