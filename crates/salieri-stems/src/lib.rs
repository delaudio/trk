use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use salieri_core::Song;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StemManifest {
    pub schema_version: u32,
    pub root_path: String,
    pub entries: Vec<StemEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StemEntry {
    pub id: String,
    pub source_path: String,
    pub display_name: String,
    pub role: StemRole,
    pub group: Option<String>,
    pub order: usize,
    pub size_bytes: u64,
    pub modified_unix_seconds: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StemRole {
    Drums,
    Bass,
    Vocals,
    Melody,
    Harmony,
    Fx,
    Mix,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StemReferenceWarning {
    pub track_name: String,
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum StemManifestError {
    #[error("failed to read directory {path}: {source}")]
    ReadDir {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to read metadata {path}: {source}")]
    Metadata {
        path: PathBuf,
        source: std::io::Error,
    },
}

pub fn scan_stem_manifest(root: &Path) -> Result<StemManifest, StemManifestError> {
    let mut files = Vec::new();
    collect_audio_files(root, root, &mut files)?;
    files.sort();
    let entries = files
        .iter()
        .enumerate()
        .map(|(order, relative_path)| stem_entry(root, relative_path, order))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(StemManifest {
        schema_version: 1,
        root_path: root.display().to_string(),
        entries,
    })
}

pub fn classify_stem_role(path: &str) -> StemRole {
    let normalized = path.to_ascii_lowercase();
    let tokens = normalized
        .split(|value: char| !value.is_ascii_alphanumeric())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if contains_any(
        &tokens,
        &["drum", "drums", "kick", "snare", "hat", "hats", "perc"],
    ) {
        StemRole::Drums
    } else if contains_any(&tokens, &["bass", "sub", "808"]) {
        StemRole::Bass
    } else if contains_any(&tokens, &["vox", "vocal", "vocals", "voice", "acapella"]) {
        StemRole::Vocals
    } else if contains_any(&tokens, &["lead", "melody", "arp", "riff", "hook"]) {
        StemRole::Melody
    } else if contains_any(
        &tokens,
        &["pad", "chord", "chords", "keys", "string", "strings"],
    ) {
        StemRole::Harmony
    } else if contains_any(&tokens, &["fx", "sfx", "noise", "impact", "riser"]) {
        StemRole::Fx
    } else if contains_any(&tokens, &["mix", "master", "full"]) {
        StemRole::Mix
    } else {
        StemRole::Other
    }
}

pub fn stem_reference_warnings(song: &Song, project_dir: &Path) -> Vec<StemReferenceWarning> {
    let Some(manifest_ref) = &song.stem_manifest else {
        return Vec::new();
    };
    let manifest_path = project_dir.join(&manifest_ref.path);
    let mut warnings = Vec::new();
    if !manifest_path.exists() {
        warnings.push(StemReferenceWarning {
            track_name: "project".to_string(),
            message: format!("Stem manifest missing: {}", manifest_path.display()),
        });
    }
    for track in &song.tracks {
        if let Some(stem_ref) = &track.stem {
            if stem_ref.entry_id.trim().is_empty() {
                warnings.push(StemReferenceWarning {
                    track_name: track.name.clone(),
                    message: "Stem reference has an empty entry id".to_string(),
                });
            }
        }
    }
    warnings
}

fn collect_audio_files(
    root: &Path,
    dir: &Path,
    files: &mut Vec<String>,
) -> Result<(), StemManifestError> {
    let entries = fs::read_dir(dir).map_err(|source| StemManifestError::ReadDir {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| StemManifestError::ReadDir {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|source| StemManifestError::Metadata {
                path: path.clone(),
                source,
            })?;
        if metadata.is_dir() {
            collect_audio_files(root, &path, files)?;
        } else if metadata.is_file() && is_audio_path(&path) {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(path.as_path())
                .to_string_lossy()
                .replace('\\', "/");
            files.push(relative);
        }
    }
    Ok(())
}

fn stem_entry(
    root: &Path,
    relative_path: &str,
    order: usize,
) -> Result<StemEntry, StemManifestError> {
    let path = root.join(relative_path);
    let metadata = fs::metadata(&path).map_err(|source| StemManifestError::Metadata {
        path: path.clone(),
        source,
    })?;
    let display_name = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(relative_path)
        .to_string();
    let group = Path::new(relative_path)
        .parent()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let modified_unix_seconds = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_secs());
    Ok(StemEntry {
        id: format!("stem_{order:03}_{:08x}", stable_hash(relative_path)),
        source_path: relative_path.to_string(),
        display_name,
        role: classify_stem_role(relative_path),
        group,
        order,
        size_bytes: metadata.len(),
        modified_unix_seconds,
    })
}

fn is_audio_path(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "wav" | "aif" | "aiff" | "flac" | "mp3" | "ogg" | "m4a"
            )
        })
        .unwrap_or(false)
}

fn contains_any(tokens: &[&str], needles: &[&str]) -> bool {
    tokens
        .iter()
        .any(|token| needles.iter().any(|needle| token == needle))
}

fn stable_hash(value: &str) -> u32 {
    let mut hash = 2_166_136_261u32;
    for byte in value.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use salieri_core::{StemManifestReference, StemTrackReference};

    #[test]
    fn classifies_stems_from_path_tokens() {
        assert_eq!(classify_stem_role("drums/kick.wav"), StemRole::Drums);
        assert_eq!(classify_stem_role("Bass/sub_bass.wav"), StemRole::Bass);
        assert_eq!(classify_stem_role("vox/lead_vocal.wav"), StemRole::Vocals);
        assert_eq!(classify_stem_role("music/lead_arp.wav"), StemRole::Melody);
        assert_eq!(
            classify_stem_role("pads/warm_chords.wav"),
            StemRole::Harmony
        );
        assert_eq!(classify_stem_role("fx/riser.wav"), StemRole::Fx);
    }

    #[test]
    fn scans_audio_files_into_stable_manifest() {
        let root = std::env::temp_dir().join(format!("salieri-stems-{}", std::process::id()));
        let drums = root.join("drums");
        fs::create_dir_all(&drums).expect("mkdir");
        fs::write(drums.join("kick.wav"), b"RIFF").expect("write wav");
        fs::write(root.join("notes.txt"), b"ignore").expect("write txt");

        let manifest = scan_stem_manifest(&root).expect("manifest");

        let _ = fs::remove_dir_all(&root);

        assert_eq!(manifest.entries.len(), 1);
        assert_eq!(manifest.entries[0].source_path, "drums/kick.wav");
        assert_eq!(manifest.entries[0].role, StemRole::Drums);
        assert_eq!(manifest.entries[0].group.as_deref(), Some("drums"));
        assert!(manifest.entries[0].id.starts_with("stem_000_"));
    }

    #[test]
    fn missing_manifest_reference_is_a_warning() {
        let mut song = Song::empty();
        song.stem_manifest = Some(StemManifestReference {
            path: "missing/stems.json".to_string(),
        });
        song.tracks[0].stem = Some(StemTrackReference {
            entry_id: "stem_000".to_string(),
        });

        let warnings = stem_reference_warnings(&song, Path::new("/tmp"));

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("Stem manifest missing"));
        song.validate().expect("stem references do not load-fail");
    }
}
