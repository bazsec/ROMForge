//! Checks the native SRAM interface entirely in memory; never touches player save files.
use anyhow::{Context, Result, ensure};
use romforge_core::Session;
use std::{fs, path::Path};
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    ensure!(args.len() == 2, "usage: check_sram CORE ROM");
    let rom = fs::read(&args[1])?;
    let extension = Path::new(&args[1])
        .extension()
        .and_then(|s| s.to_str())
        .context("extension missing")?;
    // SAFETY: this developer test loads the explicitly selected trusted core.
    let mut session = unsafe { Session::load(Path::new(&args[0]), rom.clone(), extension) }?;
    session.run_frames(120)?;
    let original = session.save_ram()?;
    ensure!(!original.is_empty(), "test ROM needs save RAM");
    let mut changed = original.clone();
    changed[0] ^= 0x5a;
    session.restore_ram(&changed)?;
    ensure!(session.save_ram()? == changed, "SRAM write/read mismatch");
    session.restore_ram(&original)?;
    drop(session);
    let mut restarted = unsafe { Session::load(Path::new(&args[0]), rom, extension) }?;
    restarted.restore_ram(&original)?;
    ensure!(
        restarted.save_ram()? == original,
        "SRAM restore after reload mismatch"
    );
    println!(
        "SRAM roundtrip and core reload passed ({} bytes)",
        original.len()
    );
    Ok(())
}
