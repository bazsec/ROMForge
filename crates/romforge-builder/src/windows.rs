use anyhow::{Context, Result, ensure};
use romforge_format::{BuildInfo, Metadata, append, detect, windows as pe};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Cursor, path::Path};

fn artwork(source: Option<&Path>) -> Result<image::DynamicImage> {
    let Some(source) = source else {
        return Ok(image::load_from_memory(include_bytes!(
            "../../../assets/branding/romforge-macos-icon.png"
        ))?);
    };
    ensure!(
        fs::metadata(source)?.len() <= 32 * 1024 * 1024,
        "Icon exceeds 32 MiB"
    );
    let bytes = fs::read(source)?;
    let image = if source
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("icns"))
    {
        ensure!(
            bytes.len() >= 8
                && &bytes[..4] == b"icns"
                && u32::from_be_bytes(bytes[4..8].try_into()?) as usize == bytes.len(),
            "Invalid ICNS icon"
        );
        let mut offset = 8;
        let mut best: Option<image::DynamicImage> = None;
        while offset + 8 <= bytes.len() {
            let length = u32::from_be_bytes(bytes[offset + 4..offset + 8].try_into()?) as usize;
            ensure!(
                length >= 8 && length <= bytes.len() - offset,
                "Invalid ICNS element"
            );
            let data = &bytes[offset + 8..offset + length];
            if data.starts_with(b"\x89PNG\r\n\x1a\n") {
                let image = image::load_from_memory(data)?;
                if best.as_ref().is_none_or(|old| image.width() > old.width()) {
                    best = Some(image);
                }
            }
            offset += length;
        }
        ensure!(offset == bytes.len(), "Truncated ICNS icon");
        best.context("ICNS has no PNG artwork; choose a PNG or JPEG icon")?
    } else {
        image::load_from_memory(&bytes)?
    };
    ensure!(
        (16..=8192).contains(&image.width()) && (16..=8192).contains(&image.height()),
        "Icon must be 16–8192 pixels per side"
    );
    Ok(image)
}

fn icon_resources(source: Option<&Path>) -> Result<Vec<(u16, u16, Vec<u8>)>> {
    let image = artwork(source)?;
    let sizes = [16, 32, 48, 64, 128, 256];
    let mut group = vec![0, 0, 1, 0, sizes.len() as u8, 0];
    let mut resources = Vec::new();
    for (index, size) in sizes.into_iter().enumerate() {
        let scaled = image
            .resize(size, size, image::imageops::FilterType::Lanczos3)
            .to_rgba8();
        let mut square = image::RgbaImage::new(size, size);
        image::imageops::overlay(
            &mut square,
            &scaled,
            ((size - scaled.width()) / 2).into(),
            ((size - scaled.height()) / 2).into(),
        );
        let mut png = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(square).write_to(&mut png, image::ImageFormat::Png)?;
        let bytes = png.into_inner();
        let id = (index + 1) as u16;
        group.extend([size as u8, size as u8, 0, 0]);
        group.extend(1u16.to_le_bytes());
        group.extend(32u16.to_le_bytes());
        group.extend((bytes.len() as u32).to_le_bytes());
        group.extend(id.to_le_bytes());
        resources.push((pe::ICON, id, bytes));
    }
    resources.push((pe::GROUP_ICON, 1, group));
    Ok(resources)
}

fn shader_notices(dir: &include_dir::Dir<'_>, files: &mut BTreeMap<String, String>) -> Result<()> {
    for file in dir.files() {
        files.insert(
            format!(
                "shaders/{}",
                file.path().to_string_lossy().replace('\\', "/")
            ),
            std::str::from_utf8(file.contents())?.to_owned(),
        );
    }
    for child in dir.dirs() {
        shader_notices(child, files)?;
    }
    Ok(())
}

/// All game assets and the Explorer icon are embedded in one native executable.
pub fn build(
    runtime: &Path,
    core: &Path,
    license: &Path,
    rom_path: &Path,
    output: &Path,
    icon: Option<&Path>,
) -> Result<()> {
    ensure!(
        output
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("exe")),
        "output must end in .exe"
    );
    ensure!(!output.exists(), "output exists; refusing to overwrite");
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
        matches!(
            console,
            romforge_format::Console::Nes | romforge_format::Console::Snes
        ),
        "runtime supports NES and SNES"
    );
    let core = fs::read(core)?;
    let core_hash = format!("{:x}", Sha256::digest(&core));
    let metadata = Metadata {
        title: rom_path
            .file_stem()
            .context("ROM title missing")?
            .to_string_lossy()
            .into_owned(),
        console,
        core_id: format!("{}-{core_hash}", console.core_name()),
        template_id: "wgpu-lottes-windows-single-v1".into(),
        extension,
    };
    let info = BuildInfo {
        version: romforge_format::VERSION.trim().into(),
        console,
        rom_sha256: format!("{:x}", Sha256::digest(&rom)),
        core_sha256: core_hash,
    };
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let staging = tempfile::Builder::new()
        .prefix(".romforge-")
        .tempdir_in(parent)?;
    let game = staging.path().join("Game.exe");
    fs::copy(runtime, &game)?;
    let mut payload = Vec::new();
    append(&mut payload, &metadata, &rom)?;
    let notices_dir = license.parent().context("License directory missing")?;
    let mut notices = BTreeMap::new();
    notices.insert(
        console.license_name().to_owned(),
        fs::read_to_string(license)?,
    );
    notices.insert(
        "SDL-LICENSE.txt".into(),
        fs::read_to_string(notices_dir.join("SDL-LICENSE.txt"))?,
    );
    if console == romforge_format::Console::Nes {
        notices.insert(
            "Nestopia-NOTICES.txt".into(),
            fs::read_to_string(notices_dir.join("Nestopia-NOTICES.txt"))?,
        );
    }
    notices.insert("BUILD.txt".into(), format!("ROMForge {}\nSource: https://github.com/bazsec/ROMForge\nRun this game with --export-resources DIRECTORY to export included licenses and shader sources.\n", info.version));
    static SHADERS: include_dir::Dir<'_> =
        include_dir::include_dir!("$CARGO_MANIFEST_DIR/../../assets/shaders");
    shader_notices(&SHADERS, &mut notices)?;
    let mut resources = icon_resources(icon)?;
    resources.extend([
        (pe::DATA, pe::PAYLOAD, payload),
        (pe::DATA, pe::CORE, core),
        (pe::DATA, pe::BUILD, serde_json::to_vec(&info)?),
        (pe::DATA, pe::NOTICES, serde_json::to_vec(&notices)?),
    ]);
    pe::embed(&game, &resources)?;
    let mut destination = tempfile::NamedTempFile::new_in(parent)?;
    std::io::copy(&mut fs::File::open(&game)?, destination.as_file_mut())?;
    destination.as_file().sync_all()?;
    destination.persist_noclobber(output)?;
    println!("Created {} (standalone game executable)", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn standalone_resources_icon_and_collision_protection() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let runtime = std::env::current_exe().unwrap();
        let core = root.join("nestopia.dll");
        let license = root.join("Nestopia-LICENSE.txt");
        for path in [
            &core,
            &license,
            &root.join("SDL-LICENSE.txt"),
            &root.join("Nestopia-NOTICES.txt"),
        ] {
            fs::write(path, b"fixture").unwrap();
        }
        let mut rom = vec![0; 16 + 16384 + 8192];
        rom[..6].copy_from_slice(b"NES\x1a\x01\x01");
        let source = root.join("Game.nes");
        fs::write(&source, &rom).unwrap();
        let output = root.join("Game.exe");
        build(&runtime, &core, &license, &source, &output, None).unwrap();
        assert!(output.is_file());
        let payload = romforge_format::read(Cursor::new(
            pe::read(&output, pe::DATA, pe::PAYLOAD).unwrap(),
        ))
        .unwrap();
        assert_eq!(payload.rom, rom);
        assert_eq!(pe::read(&output, pe::DATA, pe::CORE).unwrap(), b"fixture");
        let group = pe::read(&output, pe::GROUP_ICON, 1).unwrap();
        assert_eq!(u16::from_le_bytes(group[4..6].try_into().unwrap()), 6);
        assert!(
            pe::read(&output, pe::ICON, 6)
                .unwrap()
                .starts_with(b"\x89PNG")
        );
        assert!(build(&runtime, &core, &license, &source, &output, None).is_err());
        fs::remove_file(root.join("SDL-LICENSE.txt")).unwrap();
        let failed = root.join("Failed.exe");
        assert!(build(&runtime, &core, &license, &source, &failed, None).is_err());
        assert!(!failed.exists());
    }
}
