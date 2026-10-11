//! Versioned ROM payload shared by the packager and runtime.
use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Seek, SeekFrom, Write};

const MAGIC: &[u8; 8] = b"ROMFRG01";
const FOOTER: u64 = 56;
pub const MAX_ROM: usize = 64 * 1024 * 1024;
const MAX_METADATA: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Console {
    Nes,
    Snes,
    Gb,
    Gbc,
    Gba,
    Genesis,
}

/// Extension detection is a candidate classification, never proof of validity.
pub fn detect(extension: &str, rom: &[u8]) -> Result<Console> {
    ensure!(!rom.is_empty() && rom.len() <= MAX_ROM, "invalid ROM size");
    Ok(match extension.to_ascii_lowercase().as_str() {
        "nes" if rom.len() >= 16 && rom.starts_with(b"NES\x1a") => Console::Nes,
        "sfc" | "smc" if rom.len() >= 32768 && matches!(rom.len() % 1024, 0 | 512) => Console::Snes,
        "gb" if rom.len() >= 0x150 => Console::Gb,
        "gbc" if rom.len() >= 0x150 && rom[0x143] & 0x80 != 0 => Console::Gbc,
        "gba" if rom.len() >= 0xc0 && rom[0xb2] == 0x96 => Console::Gba,
        "md" | "gen" if rom.len() >= 0x200 && rom[0x100..0x104] == *b"SEGA" => Console::Genesis,
        _ => bail!(
            "unsupported format or invalid header; archives and ambiguous .bin files are not accepted"
        ),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub title: String,
    pub console: Console,
    pub core_id: String,
    pub template_id: String,
    pub extension: String,
}

pub struct Payload {
    pub metadata: Metadata,
    pub rom: Vec<u8>,
}

pub fn append(mut output: impl Write, metadata: &Metadata, rom: &[u8]) -> Result<()> {
    ensure!(
        detect(&metadata.extension, rom)? == metadata.console,
        "console mismatch"
    );
    let json = serde_json::to_vec(metadata)?;
    ensure!(json.len() <= MAX_METADATA, "metadata too large");
    let mut digest = Sha256::new();
    digest.update(&json);
    digest.update(rom);
    output.write_all(&json)?;
    output.write_all(rom)?;
    output.write_all(&(json.len() as u64).to_le_bytes())?;
    output.write_all(&(rom.len() as u64).to_le_bytes())?;
    output.write_all(&digest.finalize())?;
    output.write_all(MAGIC)?;
    Ok(())
}

pub fn read(mut input: impl Read + Seek) -> Result<Payload> {
    let size = input.seek(SeekFrom::End(0))?;
    ensure!(size >= FOOTER, "no ROMForge payload");
    input.seek(SeekFrom::End(-(FOOTER as i64)))?;
    let mut footer = [0; FOOTER as usize];
    input.read_exact(&mut footer)?;
    ensure!(
        &footer[48..] == MAGIC,
        "no ROMForge payload or unsupported version"
    );
    let metadata_len = u64::from_le_bytes(footer[..8].try_into()?);
    let rom_len = u64::from_le_bytes(footer[8..16].try_into()?);
    ensure!(
        metadata_len <= MAX_METADATA as u64 && rom_len <= MAX_ROM as u64,
        "payload exceeds limits"
    );
    let start = size
        .checked_sub(FOOTER + metadata_len + rom_len)
        .ok_or_else(|| anyhow::anyhow!("truncated payload"))?;
    input.seek(SeekFrom::Start(start))?;
    let mut json = vec![0; metadata_len as usize];
    input.read_exact(&mut json)?;
    let mut rom = vec![0; rom_len as usize];
    input.read_exact(&mut rom)?;
    let mut digest = Sha256::new();
    digest.update(&json);
    digest.update(&rom);
    ensure!(
        digest.finalize().as_slice() == &footer[16..48],
        "payload checksum mismatch"
    );
    let metadata: Metadata = serde_json::from_slice(&json)?;
    ensure!(
        detect(&metadata.extension, &rom)? == metadata.console,
        "console mismatch"
    );
    Ok(Payload { metadata, rom })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    #[test]
    fn roundtrip_and_corruption() {
        let metadata = Metadata {
            title: "Test".into(),
            console: Console::Snes,
            core_id: "test".into(),
            template_id: "test".into(),
            extension: "sfc".into(),
        };
        let mut bytes = b"native executable prefix".to_vec();
        append(&mut bytes, &metadata, &vec![0; 32768]).unwrap();
        assert_eq!(read(Cursor::new(&bytes)).unwrap().metadata, metadata);
        bytes[200] ^= 1;
        assert!(read(Cursor::new(bytes)).is_err());
    }
    #[test]
    fn rejects_truncation_and_hostile_lengths() {
        assert!(read(Cursor::new(vec![0; 20])).is_err());
        let mut footer = vec![255; 48];
        footer.extend(MAGIC);
        assert!(read(Cursor::new(footer)).is_err());
        assert!(detect("nes", &[0; 32]).is_err());
        assert!(detect("bin", &[0; 32768]).is_err());
    }
}
