use anyhow::{Context, Result, ensure};
use romforge_format::{Console, Metadata, append, detect};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    path::Path,
    process::Command,
};

fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn sign(path: &Path) -> Result<()> {
    let result = Command::new("/usr/bin/codesign")
        .args(["--force", "--sign", "-"])
        .arg(path)
        .output()?;
    ensure!(
        result.status.success(),
        "signing failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    Ok(())
}
/// Package existing native binaries; never compile per ROM or extract dependencies at launch.
pub fn build(
    runtime: &Path,
    core: &Path,
    license: &Path,
    rom_path: &Path,
    output: &Path,
    icon: Option<&Path>,
) -> Result<()> {
    ensure!(
        cfg!(target_os = "macos"),
        "macOS bundle signing currently requires macOS"
    );
    ensure!(
        output.extension().is_some_and(|s| s == "app"),
        "output must end in .app"
    );
    ensure!(
        !output.exists(),
        "output already exists: {}. Choose a new output folder or name.",
        output.display()
    );
    ensure!(
        fs::metadata(rom_path)?.len() <= romforge_format::MAX_ROM as u64,
        "ROM too large"
    );
    let rom = fs::read(rom_path)?;
    let extension = rom_path
        .extension()
        .and_then(|s| s.to_str())
        .context("ROM extension missing")?
        .to_owned();
    let console = detect(&extension, &rom)?;
    ensure!(
        console == Console::Snes,
        "playable template currently supports SNES only"
    );
    let title = rom_path
        .file_stem()
        .context("ROM title missing")?
        .to_string_lossy()
        .into_owned();
    let id = format!("{:x}", Sha256::digest(&rom));
    let metadata = Metadata {
        title: title.clone(),
        console,
        core_id: "snes9x-fae2fea08f74180759ef540ee94259213f503480".into(),
        template_id: "wgpu-lottes-macos-v3".into(),
        extension,
    };
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let staging = tempfile::Builder::new().prefix(".romforge-").tempdir()?;
    let bundle = staging.path().join("Game.app");
    let contents = bundle.join("Contents");
    let resources = contents.join("Resources");
    fs::create_dir_all(contents.join("MacOS"))?;
    fs::create_dir_all(contents.join("Frameworks"))?;
    fs::create_dir_all(&resources)?;
    // Extract at build time only; packaged games read these resources in place.
    static SHADERS: include_dir::Dir<'_> =
        include_dir::include_dir!("$CARGO_MANIFEST_DIR/../../assets/shaders");
    SHADERS.extract(resources.join("shaders"))?;
    let game = contents.join("MacOS/Game");
    fs::copy(runtime, &game)?;
    fs::set_permissions(&game, fs::metadata(runtime)?.permissions())?;
    let bundled_core = contents.join("Frameworks/snes9x_libretro.dylib");
    fs::copy(core, &bundled_core)?;
    fs::copy(license, resources.join("Snes9x-LICENSE.txt"))?;
    // The builder's resources carry the SDL license alongside the core license.
    let sdl_license = license
        .parent()
        .context("license directory missing")?
        .join("SDL-LICENSE.txt");
    ensure!(
        sdl_license.is_file(),
        "SDL-LICENSE.txt must be next to the core license"
    );
    fs::copy(sdl_license, resources.join("SDL-LICENSE.txt"))?;
    fs::write(
        resources.join("BUILD.txt"),
        "ROMForge playable preview\nCore: https://github.com/libretro/snes9x/tree/fae2fea08f74180759ef540ee94259213f503480\nSDL 3.4.18 statically linked. Core is bundled, not extracted.\nLocal ad-hoc signature; not Developer ID signed or notarized.\nNoncommercial development use; see included licenses.\n",
    )?;
    let mut payload = File::create(resources.join("game.romforge"))?;
    append(&mut payload, &metadata, &rom)?;
    payload.sync_all()?;
    let icon_plist = if let Some(icon) = icon {
        crate::icon::install(icon, &resources)?;
        "<key>CFBundleIconFile</key><string>Game.icns</string>"
    } else {
        ""
    };
    let title = xml(&title);
    fs::write(
        contents.join("Info.plist"),
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>Game</string>
<key>CFBundleIdentifier</key><string>dev.romforge.game.{id}</string>
<key>CFBundleName</key><string>{title}</string>
<key>CFBundleDisplayName</key><string>{title}</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>0.2.0</string>
<key>CFBundleVersion</key><string>5</string>
{icon_plist}
<key>NSHighResolutionCapable</key><true/>
</dict></plist>"#
        ),
    )?;
    // Sign on local storage; SMB can attach Finder metadata to newly copied files.
    let clean = Command::new("/usr/bin/xattr")
        .args(["-cr"])
        .arg(&bundle)
        .output()?;
    ensure!(
        clean.status.success(),
        "could not clear staging metadata: {}",
        String::from_utf8_lossy(&clean.stderr)
    );
    sign(&bundled_core)?;
    sign(&game)?;
    sign(&bundle)?;
    let check = Command::new("/usr/bin/codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(&bundle)
        .output()?;
    ensure!(
        check.status.success(),
        "bundle signature verification failed: {}",
        String::from_utf8_lossy(&check.stderr)
    );
    ensure!(
        !output.exists(),
        "output appeared during build; refusing to replace it"
    );
    // Copy the signed bundle without resource forks or extended attributes, then
    // verify it on the destination volume before publishing its final name.
    let delivery = tempfile::Builder::new()
        .prefix(".romforge-")
        .tempdir_in(parent)?;
    let delivered = delivery.path().join("Game.app");
    let copied = Command::new("/usr/bin/ditto")
        .args(["--norsrc", "--noextattr", "--noacl"])
        .arg(&bundle)
        .arg(&delivered)
        .output()?;
    ensure!(
        copied.status.success(),
        "could not copy app to destination: {}",
        String::from_utf8_lossy(&copied.stderr)
    );
    let cleaned = Command::new("/usr/bin/xattr")
        .args(["-cr"])
        .arg(&delivered)
        .output()?;
    ensure!(
        cleaned.status.success(),
        "could not clear destination metadata: {}",
        String::from_utf8_lossy(&cleaned.stderr)
    );
    let verified = Command::new("/usr/bin/codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(&delivered)
        .output()?;
    ensure!(
        verified.status.success(),
        "destination signature verification failed: {}",
        String::from_utf8_lossy(&verified.stderr)
    );
    ensure!(
        !output.exists(),
        "output appeared during copy; refusing to replace it"
    );
    fs::rename(&delivered, output)?;
    // SMB metadata can be refreshed when the bundle receives its final name.
    let cleaned = Command::new("/usr/bin/xattr")
        .args(["-cr"])
        .arg(output)
        .output()?;
    ensure!(
        cleaned.status.success(),
        "could not clear final app metadata: {}",
        String::from_utf8_lossy(&cleaned.stderr)
    );
    let verified = Command::new("/usr/bin/codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(output)
        .output()?;
    ensure!(
        verified.status.success(),
        "final app signature verification failed: {}",
        String::from_utf8_lossy(&verified.stderr)
    );
    println!(
        "Created {}\nDouble-click the app to play. All game dependencies are inside the bundle.",
        output.display()
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn titles_are_xml_escaped() {
        assert_eq!(xml("A & B <Game>"), "A &amp; B &lt;Game&gt;");
    }
}
