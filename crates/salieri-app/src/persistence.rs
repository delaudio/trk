use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use salieri_core::Song;
use serde::{Deserialize, Serialize};

pub const CURRENT_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFile {
    pub format_version: u32,
    pub song: Song,
}

impl ProjectFile {
    #[must_use]
    pub const fn new(song: Song) -> Self {
        Self {
            format_version: CURRENT_FORMAT_VERSION,
            song,
        }
    }
}

pub fn load_project(path: &Path) -> Result<Song> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read project {}", path.display()))?;
    let project: ProjectFile = serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse project {}", path.display()))?;

    if project.format_version != CURRENT_FORMAT_VERSION {
        bail!(
            "unsupported project format version {} in {}",
            project.format_version,
            path.display()
        );
    }

    Ok(project.song)
}

pub fn save_project(path: &Path, song: &Song) -> Result<()> {
    let project = ProjectFile::new(song.clone());
    let contents = serde_json::to_vec_pretty(&project).context("failed to serialize project")?;
    let temp_path = temp_path_for(path);

    {
        let mut file = File::create(&temp_path)
            .with_context(|| format!("failed to create temp file {}", temp_path.display()))?;
        file.write_all(&contents)
            .with_context(|| format!("failed to write temp file {}", temp_path.display()))?;
        file.write_all(b"\n")
            .with_context(|| format!("failed to finish temp file {}", temp_path.display()))?;
        file.sync_all()
            .with_context(|| format!("failed to sync temp file {}", temp_path.display()))?;
    }

    fs::rename(&temp_path, path).with_context(|| {
        format!(
            "failed to replace project {} with {}",
            path.display(),
            temp_path.display()
        )
    })?;

    Ok(())
}

fn temp_path_for(path: &Path) -> PathBuf {
    let mut temp_path = path.to_path_buf();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map_or_else(|| "tmp".to_string(), |value| format!("{value}.tmp"));
    temp_path.set_extension(extension);
    temp_path
}

#[cfg(test)]
mod tests {
    use super::*;
    use salieri_core::NoteEvent;

    #[test]
    fn saves_and_loads_project_file() {
        let path = std::env::temp_dir().join(format!(
            "salieri-project-roundtrip-{}.salieri",
            std::process::id()
        ));
        let mut song = Song::empty();
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
            .expect("set note");

        save_project(&path, &song).expect("save");
        let loaded = load_project(&path).expect("load");
        let _ = fs::remove_file(&path);

        assert_eq!(loaded, song);
    }

    #[test]
    fn rejects_unknown_format_versions() {
        let path = std::env::temp_dir().join(format!(
            "salieri-project-version-{}.salieri",
            std::process::id()
        ));
        let project = ProjectFile {
            format_version: 999,
            song: Song::empty(),
        };
        fs::write(&path, serde_json::to_string(&project).expect("serialize")).expect("write");

        let error = load_project(&path).expect_err("version error");
        let _ = fs::remove_file(&path);

        assert!(error.to_string().contains("unsupported project format"));
    }

    #[test]
    fn loads_default_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/default.salieri");
        let song = load_project(&path).expect("load fixture");

        assert_eq!(song.metadata.title, "Default Fixture");
        assert_eq!(song.tracks.len(), 4);
        assert_eq!(song.patterns.len(), 1);
        assert_eq!(song.sequence, vec![salieri_core::PatternId(1)]);
    }
}
