use std::path::PathBuf;

/// Validate the entire drop before changing the selection or starting a build.
pub fn rom_path(files: &[eframe::egui::DroppedFile]) -> Result<PathBuf, &'static str> {
    if files.len() != 1 {
        return Err("Drop one ROM at a time (.nes, .sfc or .smc).");
    }
    let path = files[0]
        .path
        .as_ref()
        .ok_or("Drop a ROM file from Finder (.nes, .sfc or .smc).")?;
    if !path.is_file() {
        return Err("That file is unavailable. Drop an individual ROM, not a folder.");
    }
    if !path
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| matches!(s.to_ascii_lowercase().as_str(), "nes" | "sfc" | "smc"))
    {
        return Err("Unsupported file. Drop an uncompressed NES or SNES ROM (.nes, .sfc or .smc).");
    }
    Ok(path.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::DroppedFile;

    #[test]
    fn accepts_rom_extensions_and_rejects_invalid_or_multiple_files() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["Game.sfc", "Game.SMC", "Game.zip"] {
            std::fs::write(dir.path().join(name), []).unwrap();
        }
        let drop = |name: &str| DroppedFile {
            path: Some(dir.path().join(name)),
            ..Default::default()
        };
        assert!(rom_path(&[drop("Game.sfc")]).is_ok());
        assert!(rom_path(&[drop("Game.SMC")]).is_ok());
        assert!(rom_path(&[drop("Game.zip")]).is_err());
        assert!(rom_path(&[drop("missing.sfc")]).is_err());
        assert!(rom_path(&[drop("")]).is_err());
        assert!(rom_path(&[DroppedFile::default()]).is_err());
        assert!(rom_path(&[drop("Game.sfc"), drop("Game.SMC")]).is_err());
    }
}
