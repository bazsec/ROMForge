use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::Path};
const MAGIC: &[u8; 8] = b"RFSTATE1";
const HEADER: usize = 112;
const MAX: usize = 64 * 1024 * 1024;
pub fn encode(state: &[u8], game: &[u8; 32], core: &[u8; 32]) -> Result<Vec<u8>> {
    ensure!(
        !state.is_empty() && state.len() <= MAX,
        "Invalid state size"
    );
    let mut bytes = Vec::with_capacity(HEADER + state.len());
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(game);
    bytes.extend_from_slice(core);
    bytes.extend_from_slice(&(state.len() as u64).to_le_bytes());
    bytes.extend_from_slice(&Sha256::digest(state));
    bytes.extend_from_slice(state);
    Ok(bytes)
}
pub fn read(path: &Path, game: &[u8; 32], core: &[u8; 32]) -> Result<Vec<u8>> {
    let file = File::open(path).context("Slot is empty or unreadable")?;
    let mut bytes = Vec::new();
    file.take((HEADER + MAX + 1) as u64)
        .read_to_end(&mut bytes)?;
    decode(&bytes, game, core).map(Vec::from)
}
fn decode<'a>(bytes: &'a [u8], game: &[u8; 32], core: &[u8; 32]) -> Result<&'a [u8]> {
    ensure!(
        bytes.len() >= HEADER && &bytes[..8] == MAGIC,
        "Invalid state file"
    );
    ensure!(&bytes[8..40] == game, "State belongs to another game");
    ensure!(&bytes[40..72] == core, "State uses another core version");
    let len = u64::from_le_bytes(bytes[72..80].try_into().unwrap());
    ensure!(
        len > 0 && len <= MAX as u64 && bytes.len() as u64 == HEADER as u64 + len,
        "Invalid state size"
    );
    let state = &bytes[HEADER..];
    ensure!(
        Sha256::digest(state)[..] == bytes[80..112],
        "State checksum failed"
    );
    Ok(state)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_identity_size_and_corruption() {
        let mut bytes = encode(&[7; 512], &[1; 32], &[2; 32]).unwrap();
        assert_eq!(decode(&bytes, &[1; 32], &[2; 32]).unwrap(), &[7; 512]);
        assert!(decode(&bytes, &[3; 32], &[2; 32]).is_err());
        assert!(decode(&bytes, &[1; 32], &[3; 32]).is_err());
        assert!(decode(&bytes[..100], &[1; 32], &[2; 32]).is_err());
        bytes[112] ^= 1;
        assert!(decode(&bytes, &[1; 32], &[2; 32]).is_err());
    }
}
