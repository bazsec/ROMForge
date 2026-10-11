//! Native-core state test; runs in memory and never opens user save files.
use anyhow::{Context, Result, ensure};
use romforge_core::Session;
use std::{fs, path::Path};
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    ensure!(args.len() == 2, "usage: check_states CORE ROM");
    let extension = Path::new(&args[1])
        .extension()
        .and_then(|s| s.to_str())
        .context("extension missing")?;
    // SAFETY: explicitly selected trusted development core.
    let mut session =
        unsafe { Session::load(Path::new(&args[0]), fs::read(&args[1])?, extension) }?;
    session.run_frames(120)?;
    let state = session.save_state()?;
    let ram = session.save_ram()?;
    session.set_buttons(1 << 3);
    session.run_frames(30)?;
    session.load_state(&state)?;
    ensure!(session.save_ram()? == ram, "State failed to restore SRAM");
    session.set_buttons(0);
    session.run_frames(1)?;
    let first = session.output(|frame, _| Ok(frame.rgba.clone()))?;
    session.run_frames(30)?;
    session.load_state(&state)?;
    session.run_frames(1)?;
    let second = session.output(|frame, _| Ok(frame.rgba.clone()))?;
    ensure!(
        first == second,
        "State restore produced different next video frame"
    );
    ensure!(
        session.load_state(&[0; 32]).is_err(),
        "Invalid state accepted"
    );
    session.run_frames(1)?;
    println!(
        "State restore, deterministic video, SRAM, invalid-state recovery passed ({} bytes)",
        state.len()
    );
    Ok(())
}
