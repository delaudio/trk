use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use trk_core::{PatternVariationHistory, Song};

pub const CURRENT_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFile {
    pub format_version: u32,
    pub song: Song,
    #[serde(default, skip_serializing_if = "PatternVariationHistory::is_empty")]
    pub variation_history: PatternVariationHistory,
}

impl ProjectFile {
    #[must_use]
    pub fn new(song: Song) -> Self {
        Self {
            format_version: CURRENT_FORMAT_VERSION,
            song,
            variation_history: PatternVariationHistory::default(),
        }
    }

    #[must_use]
    pub fn with_history(song: Song, variation_history: PatternVariationHistory) -> Self {
        Self {
            format_version: CURRENT_FORMAT_VERSION,
            song,
            variation_history,
        }
    }
}

pub fn load_project(path: &Path) -> Result<Song> {
    Ok(load_project_file(path)?.song)
}

pub fn load_project_file(path: &Path) -> Result<ProjectFile> {
    let file =
        File::open(path).with_context(|| format!("failed to read project {}", path.display()))?;
    load_project_reader(file, path)
}

pub(crate) fn load_project_reader(reader: impl Read, path: &Path) -> Result<ProjectFile> {
    let project: ProjectFile = serde_json::from_reader(reader)
        .with_context(|| format!("failed to parse project {}", path.display()))?;
    migrate_project(project, path)
}

/// Writes a new project from song data only; project-metadata round trips must
/// use [`save_project_file`] so variation history is preserved.
pub fn save_song_project(path: &Path, song: &Song) -> Result<()> {
    save_project_file(path, &ProjectFile::new(song.clone()))
}

pub fn save_project_file(path: &Path, project: &ProjectFile) -> Result<()> {
    project
        .song
        .validate()
        .with_context(|| format!("project validation failed before saving {}", path.display()))?;
    project
        .variation_history
        .validate_for_song(&project.song)
        .with_context(|| {
            format!(
                "variation history validation failed before saving {}",
                path.display()
            )
        })?;
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

fn migrate_project(project: ProjectFile, path: &Path) -> Result<ProjectFile> {
    match project.format_version {
        CURRENT_FORMAT_VERSION => {
            let mut project = project;
            let song = &mut project.song;
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
            project.variation_history.validate_for_song(song).with_context(|| {
                format!(
                    "variation history validation failed while loading {}",
                    path.display()
                )
            })?;
            Ok(project)
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
    use serde_json::Value;
    use std::collections::BTreeSet;
    use trk_core::{
        AutomationTarget, EffectDevice, NoteEvent, PatternVariationSource, SampleEnvelope,
        SamplePlaybackMode,
    };

    #[test]
    fn saves_and_loads_project_file() {
        let path =
            std::env::temp_dir().join(format!("trk-project-roundtrip-{}.trk", std::process::id()));
        let mut song = Song::empty();
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
            .expect("set note");

        save_song_project(&path, &song).expect("save");
        let loaded = load_project(&path).expect("load");
        let _ = fs::remove_file(&path);

        assert_eq!(loaded, song);
    }

    #[test]
    fn project_history_round_trips_and_legacy_projects_default_empty() {
        let path = test_project_path("variation-history");
        let legacy_path = test_project_path("variation-history-legacy");
        let song = Song::empty();
        let mut history = PatternVariationHistory::default();
        history
            .record_at(
                123,
                "AI bass variation",
                PatternVariationSource::AiProposal,
                0,
                Some(0),
                song.patterns[0].clone(),
            )
            .expect("record variation");
        let project = ProjectFile::with_history(song.clone(), history.clone());

        save_project_file(&path, &project).expect("save project with history");
        let persisted: Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("read project with history"))
                .expect("parse project with history");
        assert_eq!(
            persisted["variationHistory"]["entries"][0]["source"],
            "aiProposal"
        );
        let loaded = load_project_file(&path).expect("load project with history");
        assert_eq!(loaded, project);

        let legacy = serde_json::json!({"formatVersion": 1, "song": song});
        fs::write(
            &legacy_path,
            serde_json::to_vec_pretty(&legacy).expect("serialize legacy project"),
        )
        .expect("write legacy project");
        assert!(load_project_file(&legacy_path)
            .expect("load legacy project")
            .variation_history
            .is_empty());

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(legacy_path);
    }

    #[test]
    fn load_rejects_invalid_persisted_variation_history() {
        let path = test_project_path("invalid-variation-history");
        let project = serde_json::json!({
            "formatVersion": 1,
            "song": Song::empty(),
            "variationHistory": {
                "nextId": 2,
                "activeId": 99,
                "entries": []
            }
        });
        fs::write(
            &path,
            serde_json::to_vec_pretty(&project).expect("serialize invalid history"),
        )
        .expect("write invalid history");

        let error = load_project_file(&path).expect_err("invalid history");
        let _ = fs::remove_file(path);

        assert!(error
            .to_string()
            .contains("variation history validation failed while loading"));
        assert!(format!("{error:#}").contains("active variation v099 is not retained"));
    }

    #[test]
    fn saves_and_loads_sample_assignments() {
        let path = test_project_path("sample-assignments");
        let mut song = Song::empty();
        let sample = song.upsert_sample_reference("samples/kick.wav", "kick.wav");
        let track = song.tracks[0].id;
        song.assign_sample_to_track(track, sample)
            .expect("assign sample");

        save_song_project(&path, &song).expect("save");
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
            variation_history: PatternVariationHistory::default(),
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
        song.sequence[0] = trk_core::PatternId(99);
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

        let error = save_song_project(&path, &song).expect_err("validation error");
        let _ = fs::remove_file(&path);

        assert!(error
            .to_string()
            .contains("project validation failed before saving"));
        assert!(format!("{error:#}").contains("at least one track"));
    }

    #[test]
    fn loads_foundations_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/projects/foundations.trk");
        let song = load_project(&path).expect("load fixture");

        assert_eq!(song.metadata.title, "Foundations Fixture");
        assert_eq!(song.tracks.len(), 2);
        assert_eq!(song.patterns.len(), 1);
        assert_eq!(song.sequence.len(), 1);
    }

    #[test]
    fn foundations_fixture_preserves_project_contracts() {
        let song = foundations_fixture_song();
        let project = ProjectFile::new(song.clone());
        let actual: Value = serde_json::from_str(
            &serde_json::to_string(&project).expect("serialize fixture value"),
        )
        .expect("parse serialized fixture value");
        let path = fixture_path("projects/foundations.trk");

        if std::env::var_os("UPDATE_TRK_FIXTURES").is_some() {
            fs::create_dir_all(path.parent().expect("fixture parent")).expect("create fixtures");
            let mut contents = serde_json::to_string_pretty(&project).expect("serialize fixture");
            contents.push('\n');
            fs::write(&path, contents).expect("write fixture");
        }

        let expected: Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("read foundations fixture"))
                .expect("parse foundations fixture");
        let mut differences = Vec::new();
        collect_json_differences(&expected, &actual, "$", &mut differences);
        assert!(
            differences.is_empty(),
            "project fixture mismatch:\n{}",
            differences.join("\n")
        );

        let loaded = load_project(&path).expect("load foundations fixture");
        assert_eq!(loaded, song);
    }

    fn foundations_fixture_song() -> Song {
        let mut song = Song::empty();
        song.metadata.title = "Foundations Fixture".to_string();
        song.metadata.author = Some("trk Tests".to_string());
        song.transport.bpm = 132;
        song.transport.lines_per_beat = 8;
        song.delete_track(3).expect("delete track");
        song.delete_track(2).expect("delete track");
        song.resize_pattern(0, 8).expect("resize pattern");

        song.current_pattern_mut()
            .expect("pattern")
            .set_note(0, 0, NoteEvent::Note { pitch: 48 }, 100)
            .expect("set note");
        song.current_pattern_mut()
            .expect("pattern")
            .set_note_event(4, 0, NoteEvent::NoteOff, None)
            .expect("set note off");

        let sample = song.upsert_sample_reference("samples/bass.wav", "bass.wav");
        song.samples[0].root_pitch = 48;
        song.samples[0].gain = 0.8;
        song.assign_sample_to_track(song.tracks[0].id, sample)
            .expect("assign sample");
        song.set_sample_frame_window(sample, Some(10), Some(1_000))
            .expect("sample window");
        song.set_sample_loop(sample, SamplePlaybackMode::Loop, Some(100), Some(900))
            .expect("sample loop");
        song.set_sample_envelope(
            sample,
            SampleEnvelope {
                attack_seconds: 0.01,
                decay_seconds: 0.05,
                sustain: 0.75,
                release_seconds: 0.1,
            },
        )
        .expect("sample envelope");

        song.set_track_mixer_gain(0, 0.7).expect("track gain");
        song.set_track_mixer_pan(0, -0.2).expect("track pan");
        song.set_master_gain(0.9).expect("master gain");
        song.mixer.tracks[0]
            .effects
            .push(EffectDevice::gain(1, 0.8));
        song.mixer.master_effects.push(EffectDevice::pan(1, 0.1));
        song.current_pattern_mut()
            .expect("pattern")
            .set_automation_point(AutomationTarget::SampleGain { sample }, 2, 0.5)
            .expect("automation");
        song.validate().expect("valid fixture song");
        song
    }

    fn collect_json_differences(
        expected: &Value,
        actual: &Value,
        path: &str,
        differences: &mut Vec<String>,
    ) {
        match (expected, actual) {
            (Value::Object(expected), Value::Object(actual)) => {
                let keys = expected
                    .keys()
                    .chain(actual.keys())
                    .collect::<BTreeSet<_>>();
                for key in keys {
                    let child_path = format!("{path}.{key}");
                    match (expected.get(key), actual.get(key)) {
                        (Some(expected), Some(actual)) => {
                            collect_json_differences(expected, actual, &child_path, differences)
                        }
                        (Some(_), None) => {
                            differences.push(format!("- {child_path}: missing from actual"));
                        }
                        (None, Some(value)) => {
                            differences.push(format!("+ {child_path}: {value}"));
                        }
                        (None, None) => {}
                    }
                }
            }
            (Value::Array(expected), Value::Array(actual)) => {
                if expected.len() != actual.len() {
                    differences.push(format!(
                        "~ {path}.length: expected {}, actual {}",
                        expected.len(),
                        actual.len()
                    ));
                }
                for (index, (expected, actual)) in expected.iter().zip(actual.iter()).enumerate() {
                    collect_json_differences(
                        expected,
                        actual,
                        &format!("{path}[{index}]"),
                        differences,
                    );
                }
            }
            _ if expected != actual => {
                differences.push(format!("~ {path}: expected {expected}, actual {actual}"))
            }
            _ => {}
        }
    }

    fn fixture_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(relative)
    }

    fn test_project_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!("trk-project-{label}-{}.trk", std::process::id()))
    }
}
