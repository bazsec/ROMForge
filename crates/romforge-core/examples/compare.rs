//! Developer regression probe: deterministic video/audio and state round trips.
use anyhow::{Result, ensure};
use romforge_core::Session;
use std::{path::Path, time::Instant};
fn frames(session: &mut Session, count: usize) -> Result<u64> {
    let mut hash = 0xcbf29ce484222325u64;
    for _ in 0..count {
        session.run_frames(1)?;
        session.output(|video, audio| {
            for byte in video
                .width
                .to_le_bytes()
                .into_iter()
                .chain(video.height.to_le_bytes())
                .chain(video.rgba.iter().copied())
                .chain(audio.iter().flat_map(|sample| sample.to_le_bytes()))
            {
                hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
            }
            Ok(())
        })?;
    }
    Ok(hash)
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    ensure!(
        args.len() >= 4,
        "compare CORE ROM FRAMES [STATE_OUT] [STATE_IN]"
    );
    let rom = Path::new(&args[2]);
    // SAFETY: this developer-only probe loads the explicitly supplied trusted core.
    let mut session = unsafe {
        Session::load(
            Path::new(&args[1]),
            std::fs::read(rom)?,
            rom.extension().unwrap().to_str().unwrap(),
        )
    }?;
    let start = Instant::now();
    let digest = frames(&mut session, args[3].parse()?)?;
    let state = session.save_state()?;
    if let Some(path) = args.get(4) {
        std::fs::write(path, &state)?;
    }
    let expected = frames(&mut session, 60)?;
    session.load_state(&state)?;
    let replay = frames(&mut session, 60)?;
    println!("CHECKPOINT {digest:016x} {expected:016x} {replay:016x}");
    ensure!(
        expected == replay,
        "state replay differs: {expected:x} != {replay:x}"
    );
    if let Some(path) = args.get(5) {
        session.load_state(&std::fs::read(path)?)?;
        let imported = frames(&mut session, 60)?;
        ensure!(expected == imported, "cross-build state differs");
    }
    println!(
        "RESULT {digest:016x} {expected:016x} {:.3}",
        start.elapsed().as_secs_f64()
    );
    Ok(())
}
