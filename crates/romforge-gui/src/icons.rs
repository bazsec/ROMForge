use std::{
    collections::{BTreeMap, BTreeSet},
    io,
    path::{Path, PathBuf},
};

/// Modern ICNS artwork embeds PNGs; preview those without macOS's `sips`.
pub fn icns_preview(path: &Path) -> Result<image::DynamicImage, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    if bytes.len() < 8
        || bytes.len() > 32 * 1024 * 1024
        || &bytes[..4] != b"icns"
        || u32::from_be_bytes(bytes[4..8].try_into().unwrap()) as usize != bytes.len()
    {
        return Err("Invalid ICNS cover".into());
    }
    let mut offset = 8;
    let mut best = None;
    while offset + 8 <= bytes.len() {
        let length = u32::from_be_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        if length < 8 || length > bytes.len() - offset {
            return Err("Invalid ICNS element".into());
        }
        let data = &bytes[offset + 8..offset + length];
        if data.starts_with(b"\x89PNG\r\n\x1a\n") {
            let image = image::load_from_memory(data).map_err(|e| e.to_string())?;
            if best
                .as_ref()
                .is_none_or(|old: &image::DynamicImage| image.width() > old.width())
            {
                best = Some(image);
            }
        }
        offset += length;
    }
    if offset != bytes.len() {
        return Err("Truncated ICNS cover".into());
    }
    best.ok_or("This ICNS has no PNG image. Choose a PNG or JPEG cover.".into())
}

/// Normalize naming conventions, preserving meaningful parenthetical title text.
pub fn key(title: &str) -> String {
    let mut title = title.to_lowercase();
    for (open, close) in [('(', ')'), ('[', ']')] {
        let mut from = 0;
        while let Some(start) = title[from..].find(open).map(|i| i + from) {
            let Some(end) = title[start..].find(close).map(|i| i + start) else {
                break;
            };
            let tag = &title[start + 1..end];
            let metadata = tag.split(',').all(|part| {
                matches!(
                    part.trim(),
                    "usa"
                        | "us"
                        | "europe"
                        | "japan"
                        | "world"
                        | "canada"
                        | "en"
                        | "fr"
                        | "de"
                        | "es"
                        | "it"
                        | "ja"
                )
            }) || tag.starts_with("rev ")
                || tag.starts_with("version ")
                || matches!(tag, "!" | "beta" | "proto" | "prototype");
            if metadata {
                title.replace_range(start..=end, "");
                from = start;
            } else {
                from = end + 1;
            }
        }
    }
    title = title.replace(", the", "");
    let words: Vec<_> = title
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    let words = if words.first() == Some(&"the") {
        &words[1..]
    } else {
        &words[..]
    };
    let words = if words.last() == Some(&"the") {
        &words[..words.len() - 1]
    } else {
        words
    };
    words.concat()
}

#[derive(Clone, Default)]
pub struct Index {
    entries: BTreeMap<String, Vec<PathBuf>>,
    aliases: BTreeMap<String, BTreeSet<String>>,
    pub warnings: Vec<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Match {
    Found(PathBuf),
    Missing,
    Ambiguous(Vec<PathBuf>),
}
impl Index {
    pub fn scan(folder: &Path) -> Self {
        let mut index = Self::default();
        let mut dirs = vec![folder.to_path_buf()];
        while let Some(dir) = dirs.pop() {
            let entries = match std::fs::read_dir(&dir) {
                Ok(entries) => entries,
                Err(e) if e.kind() == io::ErrorKind::NotFound && dir == folder => continue,
                Err(e) => {
                    index.warnings.push(format!("{}: {e}", dir.display()));
                    continue;
                }
            };
            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        index.warnings.push(e.to_string());
                        continue;
                    }
                };
                let kind = match entry.file_type() {
                    Ok(k) => k,
                    Err(e) => {
                        index.warnings.push(e.to_string());
                        continue;
                    }
                };
                let path = entry.path();
                if kind.is_dir() {
                    dirs.push(path);
                } else if kind.is_file()
                    && path.extension().and_then(|s| s.to_str()).is_some_and(|s| {
                        ["png", "jpg", "jpeg", "icns"]
                            .iter()
                            .any(|ext| s.eq_ignore_ascii_case(ext))
                    })
                {
                    index
                        .entries
                        .entry(key(&path.file_stem().unwrap_or_default().to_string_lossy()))
                        .or_default()
                        .push(path);
                }
            }
        }
        // Embed the reviewed catalog mapping; matching needs no network access.
        let catalog: serde_json::Value =
            serde_json::from_str(include_str!("../../../catalog/snes-cover-matches.json"))
                .expect("embedded cover catalog");
        for row in catalog.as_array().into_iter().flatten() {
            let Some(cover) = row["cover"].as_str() else {
                continue;
            };
            let cover_key = key(&Path::new(cover)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy());
            for name in row["aliases"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str())
                .chain(row["title"].as_str())
            {
                index
                    .aliases
                    .entry(key(name))
                    .or_default()
                    .insert(cover_key.clone());
            }
        }
        index
    }
    pub fn find(&self, rom: &Path) -> Match {
        let name = key(&rom.file_stem().unwrap_or_default().to_string_lossy());
        let mut paths = self.entries.get(&name).cloned().unwrap_or_default();
        if paths.is_empty()
            && let Some(aliases) = self.aliases.get(&name)
        {
            for alias in aliases {
                paths.extend(self.entries.get(alias).into_iter().flatten().cloned());
            }
        }
        paths.sort();
        paths.dedup();
        // Prefer an ICNS only over its sibling image, never silently choose between covers.
        let icns: BTreeSet<_> = paths
            .iter()
            .filter(|p| {
                p.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("icns"))
            })
            .map(|p| p.with_extension(""))
            .collect();
        paths.retain(|p| {
            !p.extension().is_some_and(|e| {
                ["png", "jpg", "jpeg"]
                    .iter()
                    .any(|ext| e.eq_ignore_ascii_case(ext))
            }) || !icns.contains(&p.with_extension(""))
        });
        match paths.len() {
            0 => Match::Missing,
            1 => Match::Found(paths.remove(0)),
            _ => Match::Ambiguous(paths),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn previews_embedded_png_and_rejects_truncated_icns() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cover.icns");
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(16, 16)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let png = png.into_inner();
        let mut bytes = b"icns".to_vec();
        bytes.extend(((16 + png.len()) as u32).to_be_bytes());
        bytes.extend(b"icp4");
        bytes.extend(((8 + png.len()) as u32).to_be_bytes());
        bytes.extend(png);
        std::fs::write(&path, &bytes).unwrap();
        assert_eq!(icns_preview(&path).unwrap().width(), 16);
        bytes.pop();
        std::fs::write(&path, bytes).unwrap();
        assert!(icns_preview(&path).is_err());
    }
    #[test]
    fn normalizes_tags_articles_and_punctuation_without_losing_subtitles() {
        assert_eq!(
            key("Legend of Zelda, The - A Link to the Past (USA) (Rev 1)"),
            key("The Legend of Zelda: A Link to the Past")
        );
        assert_ne!(key("Game (Special Edition)"), key("Game"));
        assert_ne!(key("Game 2"), key("Game"));
    }
    #[test]
    fn matches_jpeg_and_prefers_prepared_sibling_icns() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Game.jpg"), []).unwrap();
        assert_eq!(
            Index::scan(dir.path()).find(Path::new("Game (USA).sfc")),
            Match::Found(dir.path().join("Game.jpg"))
        );
        std::fs::write(dir.path().join("Game.icns"), []).unwrap();
        assert_eq!(
            Index::scan(dir.path()).find(Path::new("Game.sfc")),
            Match::Found(dir.path().join("Game.icns"))
        );
    }
    #[test]
    fn finds_nested_aliases_and_reports_conflicts() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("covers");
        std::fs::create_dir(&nested).unwrap();
        std::fs::write(nested.join("7th Saga, The.png"), []).unwrap();
        assert_eq!(
            Index::scan(dir.path()).find(Path::new("The 7th Saga (USA).sfc")),
            Match::Found(nested.join("7th Saga, The.png"))
        );
        std::fs::write(dir.path().join("The 7th Saga.png"), []).unwrap();
        assert!(matches!(
            Index::scan(dir.path()).find(Path::new("The 7th Saga.sfc")),
            Match::Ambiguous(_)
        ));
        std::fs::write(nested.join("Radical Psycho Machine Racing.png"), []).unwrap();
        assert!(matches!(
            Index::scan(dir.path()).find(Path::new("RPM Racing (USA).sfc")),
            Match::Found(_)
        ));
    }
}
