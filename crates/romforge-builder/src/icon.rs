use anyhow::{Context, Result, ensure};
use std::{fs, path::Path, process::Command};

fn run(command: &mut Command) -> Result<String> {
    let output = command
        .output()
        .context("could not run macOS icon conversion tool")?;
    ensure!(
        output.status.success(),
        "icon conversion failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
/// Convert an existing PNG into all standard Retina/non-Retina ICNS sizes.
/// The source image is never changed. Existing ICNS files can be copied directly.
pub fn install(source: &Path, resources: &Path) -> Result<()> {
    ensure!(
        fs::metadata(source)?.len() <= 32 * 1024 * 1024,
        "icon exceeds 32 MiB"
    );
    let extension = source
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let destination = resources.join("Game.icns");
    if extension == "icns" {
        let bytes = fs::read(source)?;
        ensure!(
            bytes.len() >= 8
                && &bytes[..4] == b"icns"
                && u32::from_be_bytes(bytes[4..8].try_into()?) as usize == bytes.len(),
            "invalid ICNS file"
        );
        fs::write(destination, bytes)?;
        return Ok(());
    }
    ensure!(extension == "png", "icons must be PNG or ICNS files");
    let details = run(Command::new("/usr/bin/sips")
        .args(["-g", "pixelWidth", "-g", "pixelHeight"])
        .arg(source))?;
    let property = |key: &str| {
        details
            .lines()
            .find_map(|line| line.trim().strip_prefix(key))
            .and_then(|s| s.trim().parse::<u32>().ok())
    };
    let width = property("pixelWidth:").context("could not read icon width")?;
    let height = property("pixelHeight:").context("could not read icon height")?;
    ensure!(
        (16..=8192).contains(&width) && (16..=8192).contains(&height),
        "PNG icon must be between 16 and 8192 pixels per side"
    );
    let temp = tempfile::tempdir()?;
    let padded = temp.path().join("padded.png");
    let source = if width != height {
        let image = image::open(source)?.thumbnail(1024, 1024).to_rgba8();
        let side = image.width().max(image.height());
        let mut square = image::RgbaImage::new(side, side);
        image::imageops::overlay(
            &mut square,
            &image,
            ((side - image.width()) / 2).into(),
            ((side - image.height()) / 2).into(),
        );
        square.save(&padded)?;
        padded.as_path()
    } else {
        source
    };
    let iconset = temp.path().join("Game.iconset");
    fs::create_dir(&iconset)?;
    for size in [16, 32, 128, 256, 512] {
        for scale in [1, 2] {
            let pixels = (size * scale).to_string();
            let suffix = if scale == 2 { "@2x" } else { "" };
            run(Command::new("/usr/bin/sips")
                .args(["-z", &pixels, &pixels])
                .arg(source)
                .arg("--out")
                .arg(iconset.join(format!("icon_{size}x{size}{suffix}.png"))))?;
        }
    }
    run(Command::new("/usr/bin/iconutil")
        .args(["-c", "icns"])
        .arg(&iconset)
        .arg("-o")
        .arg(destination))?;
    Ok(())
}
