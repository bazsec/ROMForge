use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
pub const VERSION: &str = include_str!("../../../VERSION");
const REPO: &str = "bazsec/ROMForge";
#[derive(Clone, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub html_url: String,
    pub assets: Vec<Asset>,
}
#[derive(Clone, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}
pub fn number(tag: &str) -> Option<u64> {
    let value = tag.strip_prefix('v')?;
    if value.len() < 3 || !value.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}
pub fn check() -> Result<Option<Release>, String> {
    let result = crate::process::background("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--proto",
            "=https",
            "--max-time",
            "30",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "User-Agent: ROMForge",
        ])
        .arg(format!(
            "https://api.github.com/repos/{REPO}/releases/latest"
        ))
        .output()
        .map_err(|e| e.to_string())?;
    if !result.status.success() {
        return Err("Could not check GitHub Releases. Check your connection and try again.".into());
    }
    let release: Release = serde_json::from_slice(&result.stdout).map_err(|e| e.to_string())?;
    let version = number(&release.tag_name).ok_or("Unrecognized release version")?;
    if !release
        .html_url
        .starts_with(&format!("https://github.com/{REPO}/releases/"))
    {
        return Err("Unexpected release URL".into());
    }
    Ok((version > number(VERSION.trim()).unwrap_or(0)).then_some(release))
}
fn download(url: &str, path: &std::path::Path) -> Result<(), String> {
    let status = crate::process::background("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--max-time",
            "300",
            "--output",
        ])
        .arg(path)
        .arg(url)
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("Update download failed".into())
    }
}
fn quote(path: &std::path::Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}
/// Stage beside the installed app so the final rename stays on one filesystem.
pub fn prepare(release: &Release) -> Result<PathBuf, String> {
    if !cfg!(target_os = "macos") {
        return Err("Automatic installation is currently available only on macOS. Download a Windows release manually.".into());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let app = exe
        .ancestors()
        .nth(3)
        .ok_or("Run the packaged ROMForge.app to update")?;
    if app.extension().is_none_or(|e| e != "app") {
        return Err("Run the packaged ROMForge.app to update".into());
    }
    let parent = app.parent().ok_or("App location unavailable")?;
    let temp = tempfile::Builder::new()
        .prefix(".romforge-update-")
        .tempdir_in(parent)
        .map_err(|_| "Cannot write beside this app. Move ROMForge to a writable folder first.")?;
    let name = format!("ROMForge-{}-macOS-arm64.zip", release.tag_name);
    let asset_url = |name: &str| -> Result<&str, String> {
        let url = release
            .assets
            .iter()
            .find(|a| a.name == name)
            .ok_or(format!("Release is missing {name}"))?
            .browser_download_url
            .as_str();
        if !url.starts_with(&format!(
            "https://github.com/{REPO}/releases/download/{}/",
            release.tag_name
        )) {
            return Err("Unexpected asset URL".into());
        }
        Ok(url)
    };
    let zip = temp.path().join(&name);
    let checksum = temp.path().join("SHA256SUMS.txt");
    download(asset_url(&name)?, &zip)?;
    download(asset_url("SHA256SUMS.txt")?, &checksum)?;
    let sums = std::fs::read_to_string(checksum).map_err(|e| e.to_string())?;
    let expected = sums
        .lines()
        .find_map(|line| {
            let mut parts = line.split_whitespace();
            let hash = parts.next()?;
            let file = parts.next()?;
            (file.trim_start_matches('*') == name).then_some(hash)
        })
        .ok_or("Checksum missing for update")?;
    let actual = format!(
        "{:x}",
        Sha256::digest(std::fs::read(&zip).map_err(|e| e.to_string())?)
    );
    if expected != actual {
        return Err("Update checksum did not match".into());
    }
    let list = crate::process::background("unzip")
        .arg("-Z1")
        .arg(&zip)
        .output()
        .map_err(|e| e.to_string())?;
    if !list.status.success()
        || String::from_utf8_lossy(&list.stdout)
            .lines()
            .any(|p| !p.starts_with("ROMForge.app/") || p.split('/').any(|c| c == ".."))
    {
        return Err("Unexpected update archive contents".into());
    }
    let status = crate::process::background("ditto")
        .args(["-x", "-k"])
        .arg(&zip)
        .arg(temp.path())
        .status()
        .map_err(|e| e.to_string())?;
    let staged = temp.path().join("ROMForge.app");
    if !status.success() || !staged.join("Contents/MacOS/ROMForge").is_file() {
        return Err("Update extraction failed".into());
    }
    if !crate::process::background("codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(&staged)
        .status()
        .map_err(|e| e.to_string())?
        .success()
    {
        return Err("Update signature is invalid".into());
    }
    let backup = temp.path().join("Previous.app");
    let script = temp.path().join("install.sh");
    let body = format!(
        "#!/bin/sh\nset -eu\nwhile kill -0 {} 2>/dev/null; do sleep 1; done\nmv {} {}\nif mv {} {}; then\n  open {}\nelse\n  mv {} {}\n  open {}\n  exit 1\nfi\n",
        std::process::id(),
        quote(app),
        quote(&backup),
        quote(&staged),
        quote(app),
        quote(app),
        quote(&backup),
        quote(app),
        quote(app)
    );
    std::fs::write(&script, body).map_err(|e| e.to_string())?;
    let _ = temp.keep(); // Retain the previous app for recovery; never remove user data.
    Ok(script)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numeric_versions() {
        assert_eq!(number("v001"), Some(1));
        assert_eq!(number("v010"), Some(10));
        assert_eq!(number("v1.0"), None);
        assert_eq!(number("main"), None);
    }
}
