//! Development packager: appends a ROM to a same-host headless template.
mod icon;
mod macos;
#[cfg(windows)]
mod windows;
use anyhow::{Context, Result, ensure};
use romforge_format::{Metadata, append, detect};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
};
fn main() -> Result<()> {
    let mut args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|s| s == "--native") {
        args[0] = romforge_format::native_package_flag()
            .context("Native packaging is currently supported on Windows and macOS")?
            .into();
    }
    if args.first().is_some_and(|s| s == "--windows") {
        #[cfg(not(windows))]
        anyhow::bail!("Windows packaging requires a Windows host");
        #[cfg(windows)]
        {
            ensure!(
                args.len() == 6 || (args.len() == 8 && args[6] == "--icon"),
                "usage: romforge-builder --windows RUNTIME.exe CORE.dll CORE_LICENSE ROM OUTPUT.exe [--icon IMAGE]"
            );
            return windows::build(
                std::path::Path::new(&args[1]),
                std::path::Path::new(&args[2]),
                std::path::Path::new(&args[3]),
                std::path::Path::new(&args[4]),
                std::path::Path::new(&args[5]),
                args.get(7).map(std::path::Path::new),
            );
        }
    }
    if args.first().is_some_and(|s| s == "--macos") {
        ensure!(
            args.len() == 6 || (args.len() == 8 && args[6] == "--icon"),
            "usage: romforge-builder --macos RUNTIME CORE CORE_LICENSE ROM OUTPUT.app [--icon ICON.png|ICON.icns]"
        );
        return macos::build(
            std::path::Path::new(&args[1]),
            std::path::Path::new(&args[2]),
            std::path::Path::new(&args[3]),
            std::path::Path::new(&args[4]),
            std::path::Path::new(&args[5]),
            args.get(7).map(std::path::Path::new),
        );
    }
    ensure!(
        args.len() == 3,
        "usage: romforge-builder TEMPLATE ROM OUTPUT (development, same-host only)"
    );
    let template = PathBuf::from(&args[0]);
    let rom_path = PathBuf::from(&args[1]);
    let output = PathBuf::from(&args[2]);
    ensure!(!output.exists(), "output exists; refusing to overwrite");
    ensure!(
        fs::metadata(&rom_path)?.len() <= romforge_format::MAX_ROM as u64,
        "ROM too large"
    );
    let mut rom = Vec::new();
    File::open(&rom_path)?.read_to_end(&mut rom)?;
    let extension = rom_path
        .extension()
        .and_then(|s| s.to_str())
        .context("ROM extension missing")?
        .to_owned();
    let console = detect(&extension, &rom)?;
    let metadata = Metadata {
        title: rom_path
            .file_stem()
            .context("title missing")?
            .to_string_lossy()
            .into_owned(),
        console,
        core_id: "external-libretro-development".into(),
        template_id: "headless-v1".into(),
        extension,
    };
    // create_new protects the output against collisions, including concurrent builds.
    let mut destination = File::options().write(true).create_new(true).open(&output)?;
    let result = (|| -> Result<()> {
        std::io::copy(&mut File::open(&template)?, &mut destination)?;
        append(&mut destination, &metadata, &rom)?;
        destination.flush()?;
        destination.sync_all()?;
        fs::set_permissions(&output, fs::metadata(&template)?.permissions())?;
        Ok(())
    })();
    if result.is_err() {
        drop(destination);
        let _ = fs::remove_file(&output);
    }
    result?;
    println!(
        "Created {} (headless development artifact; requires an external core)",
        output.display()
    );
    Ok(())
}
