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
    migrate_project(project, path)
}

pub fn save_project(path: &Path, song: &Song) -> Result<()> {
    song.validate()
        .with_context(|| format!("project validation failed before saving {}", path.display()))?;
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

fn migrate_project(project: ProjectFile, path: &Path) -> Result<Song> {
    match project.format_version {
        CURRENT_FORMAT_VERSION => {
            let mut song = project.song;
            song.ensure_instruments_for_sample_assignments()
                .with_context(|| {
                    format!(
                        "project migration failed while loading {}",
                        path.display()
                    )
                })?;
            song.ensure_mixer_for_tracks();
            song.validate().with_context(|| {
                format!("project validation failed while loading {}", path.display())
            })?;
            Ok(song)
        }
        version => bail!(
            "unsupported project format version {version} in {}; current version is {CURRENT_FORMAT_VERSION}",
            path.display()
        ),
    }
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
    fn saves_and_loads_sample_assignments() {
        let path = test_project_path("sample-assignments");
        let mut song = Song::empty();
        let sample = song.upsert_sample_reference("samples/kick.wav", "kick.wav");
        let track = song.tracks[0].id;
        song.assign_sample_to_track(track, sample)
            .expect("assign sample");

        save_project(&path, &song).expect("save");
        let loaded = load_project(&path).expect("load");
        let _ = fs::remove_file(&path);

        assert_eq!(
            loaded.sample_for_track(track).expect("sample").path,
            "samples/kick.wav"
        );
        assert_eq!(
            loaded
                .instrument_for_track(track)
                .expect("instrument")
                .sample,
            Some(sample)
        );
    }

    #[test]
    fn rejects_unknown_format_versions() {
        let path = test_project_path("version");
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
    fn rejects_malformed_project_json() {
        let path = test_project_path("malformed");
        fs::write(&path, "{ not-json").expect("write");

        let error = load_project(&path).expect_err("parse error");
        let _ = fs::remove_file(&path);

        assert!(error.to_string().contains("failed to parse project"));
    }

    #[test]
    fn rejects_invalid_project_structure_on_load() {
        let path = test_project_path("invalid-structure");
        let mut song = Song::empty();
        song.sequence[0] = salieri_core::PatternId(99);
        let project = ProjectFile::new(song);
        fs::write(&path, serde_json::to_string(&project).expect("serialize")).expect("write");

        let error = load_project(&path).expect_err("validation error");
        let _ = fs::remove_file(&path);

        assert!(error
            .to_string()
            .contains("project validation failed while loading"));
        assert!(format!("{error:#}").contains("references missing pattern"));
    }

    #[test]
    fn rejects_invalid_project_structure_before_save() {
        let path = test_project_path("invalid-save");
        let mut song = Song::empty();
        song.tracks.clear();

        let error = save_project(&path, &song).expect_err("validation error");
        let _ = fs::remove_file(&path);

        assert!(error
            .to_string()
            .contains("project validation failed before saving"));
        assert!(format!("{error:#}").contains("at least one track"));
    }

    #[test]
    fn loads_default_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/default.salieri");
        let song = load_project(&path).expect("load fixture");

        assert_eq!(song.metadata.title, "256COLOR_rep");
        assert_eq!(song.tracks.len(), 15);
        assert_eq!(song.patterns.len(), 40);
        assert_eq!(song.sequence.len(), 40);
        assert_eq!(song.samples.len(), 20);
    }

    fn test_project_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "salieri-project-{label}-{}.salieri",
            std::process::id()
        ))
    }
}
