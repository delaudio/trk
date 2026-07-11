use std::collections::HashSet;

use salieri_core::{
    Clip, ClipId, ClipLaunchQuantization, ClipSlot, ClipSource, NoteEvent, Pattern, PatternCell,
    PatternId, Scene, SceneId, Session, Song,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbletonSessionDocument {
    pub schema_version: u32,
    pub tempo_bpm: u16,
    pub lines_per_beat: u8,
    pub tracks: Vec<AbletonTrack>,
    pub scenes: Vec<AbletonScene>,
    pub clips: Vec<AbletonClip>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbletonTrack {
    pub index: usize,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub midi_channel: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbletonScene {
    pub index: usize,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbletonClip {
    pub name: String,
    pub track_index: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene_index: Option<usize>,
    pub length_beats: f32,
    pub source_pattern: String,
    pub notes: Vec<AbletonNote>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbletonNote {
    pub pitch: u8,
    pub velocity: u8,
    pub start_beat: f32,
    pub duration_beats: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbletonBridgeSummary {
    pub track_count: usize,
    pub scene_count: usize,
    pub clip_count: usize,
    pub note_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AbletonPushPreview {
    pub document: AbletonSessionDocument,
    pub summary: AbletonBridgeSummary,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AbletonPullPreview {
    pub song: Song,
    pub summary: AbletonBridgeSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AbletonBridgeError {
    #[error("Ableton bridge schema version {0} is not supported")]
    UnsupportedSchema(u32),
    #[error("Ableton bridge document must contain at least one track")]
    EmptyTracks,
    #[error("Ableton bridge clip {clip_index} references missing track {track_index}")]
    MissingTrack {
        clip_index: usize,
        track_index: usize,
    },
    #[error("Ableton bridge clip {clip_index} references missing scene {scene_index}")]
    MissingScene {
        clip_index: usize,
        scene_index: usize,
    },
    #[error("failed to build pulled Salieri song: {0}")]
    InvalidSong(String),
}

pub fn preview_push_to_ableton(song: &Song) -> AbletonPushPreview {
    let mut warnings = Vec::new();
    let tracks = song
        .tracks
        .iter()
        .enumerate()
        .map(|(index, track)| AbletonTrack {
            index,
            name: prompt_safe(&track.name),
            midi_channel: Some(track.midi_channel),
        })
        .collect::<Vec<_>>();
    let scenes = song
        .session
        .scenes
        .iter()
        .enumerate()
        .map(|(index, scene)| AbletonScene {
            index,
            name: prompt_safe(&scene.name),
        })
        .collect::<Vec<_>>();
    let mut clips = Vec::new();
    let mut exported_clip_ids = HashSet::new();

    for (scene_index, scene) in song.session.scenes.iter().enumerate() {
        for slot in &scene.slots {
            let Some(clip_id) = slot.clip else {
                continue;
            };
            let Some(track_index) = song.tracks.iter().position(|track| track.id == slot.track)
            else {
                warnings.push(format!(
                    "scene {} has a slot for a missing track",
                    scene.name
                ));
                continue;
            };
            let Some(clip) = song.session.clips.iter().find(|clip| clip.id == clip_id) else {
                warnings.push(format!(
                    "scene {} references missing clip {}",
                    scene.name, clip_id.0
                ));
                continue;
            };
            if let Some(exported) = clip_to_ableton(song, clip, track_index, Some(scene_index)) {
                exported_clip_ids.insert(clip.id);
                clips.push(exported);
            }
        }
    }

    for clip in &song.session.clips {
        if exported_clip_ids.contains(&clip.id) {
            continue;
        }
        if let Some(exported) = clip_to_ableton(song, clip, 0, None) {
            clips.push(exported);
        }
    }

    let note_count = clips.iter().map(|clip| clip.notes.len()).sum();
    let document = AbletonSessionDocument {
        schema_version: 1,
        tempo_bpm: song.transport.bpm,
        lines_per_beat: song.transport.lines_per_beat,
        tracks,
        scenes,
        clips,
    };
    let summary = AbletonBridgeSummary {
        track_count: document.tracks.len(),
        scene_count: document.scenes.len(),
        clip_count: document.clips.len(),
        note_count,
        warnings,
    };
    AbletonPushPreview { document, summary }
}

pub fn preview_pull_from_ableton(
    document: &AbletonSessionDocument,
) -> Result<AbletonPullPreview, AbletonBridgeError> {
    if document.schema_version != 1 {
        return Err(AbletonBridgeError::UnsupportedSchema(
            document.schema_version,
        ));
    }
    if document.tracks.is_empty() {
        return Err(AbletonBridgeError::EmptyTracks);
    }
    for (clip_index, clip) in document.clips.iter().enumerate() {
        if !document
            .tracks
            .iter()
            .any(|track| track.index == clip.track_index)
        {
            return Err(AbletonBridgeError::MissingTrack {
                clip_index,
                track_index: clip.track_index,
            });
        }
        if let Some(scene_index) = clip.scene_index {
            if !document
                .scenes
                .iter()
                .any(|scene| scene.index == scene_index)
            {
                return Err(AbletonBridgeError::MissingScene {
                    clip_index,
                    scene_index,
                });
            }
        }
    }

    let mut song = Song::empty();
    song.metadata.title = "Ableton Session Pull".to_string();
    song.transport.bpm = document.tempo_bpm.max(1);
    song.transport.lines_per_beat = document.lines_per_beat.max(1);
    resize_tracks(&mut song, document.tracks.len());
    for track in &document.tracks {
        if let Some(target) = song.tracks.get_mut(track.index) {
            target.name = prompt_safe(&track.name);
            if let Some(channel) = track.midi_channel {
                target.midi_channel = channel.clamp(1, 16);
            }
        }
    }

    let mut patterns = Vec::new();
    let mut clips = Vec::new();
    for (clip_index, clip) in document.clips.iter().enumerate() {
        let pattern_id = PatternId(clip_index as u32 + 1);
        let row_count = clip_row_count(clip, song.transport.lines_per_beat);
        let mut pattern = Pattern::empty(
            pattern_id,
            format!("{} Pattern", prompt_safe(&clip.name)),
            row_count,
            song.tracks.len(),
        );
        write_clip_notes(&mut pattern, clip, song.transport.lines_per_beat);
        patterns.push(pattern);
        clips.push(Clip {
            id: ClipId(clip_index as u32 + 1),
            name: prompt_safe(&clip.name),
            source: ClipSource::Pattern {
                pattern_id,
                row_start: 0,
                row_count,
            },
            loop_enabled: true,
            launch_quantization: ClipLaunchQuantization::Pattern,
        });
    }
    if patterns.is_empty() {
        patterns.push(Pattern::empty(
            PatternId(1),
            "Ableton Empty Pattern",
            1,
            song.tracks.len(),
        ));
    }
    song.patterns = patterns;
    song.sequence = vec![song.patterns[0].id];

    let mut scenes = document
        .scenes
        .iter()
        .map(|scene| Scene {
            id: SceneId(scene.index as u32 + 1),
            name: prompt_safe(&scene.name),
            slots: Vec::new(),
        })
        .collect::<Vec<_>>();
    for (clip_index, clip) in document.clips.iter().enumerate() {
        let Some(scene_index) = clip.scene_index else {
            continue;
        };
        let Some(scene) = scenes
            .iter_mut()
            .find(|scene| scene.id.0 == scene_index as u32 + 1)
        else {
            continue;
        };
        let track_id = song.tracks[clip.track_index].id;
        if scene.slots.iter().any(|slot| slot.track == track_id) {
            continue;
        }
        scene.slots.push(ClipSlot {
            track: track_id,
            clip: Some(ClipId(clip_index as u32 + 1)),
        });
    }
    song.session = Session { clips, scenes };
    song.validate()
        .map_err(|error| AbletonBridgeError::InvalidSong(error.to_string()))?;

    let summary = AbletonBridgeSummary {
        track_count: song.tracks.len(),
        scene_count: song.session.scenes.len(),
        clip_count: song.session.clips.len(),
        note_count: document.clips.iter().map(|clip| clip.notes.len()).sum(),
        warnings: Vec::new(),
    };
    Ok(AbletonPullPreview { song, summary })
}

fn clip_to_ableton(
    song: &Song,
    clip: &Clip,
    track_index: usize,
    scene_index: Option<usize>,
) -> Option<AbletonClip> {
    let ClipSource::Pattern {
        pattern_id,
        row_start,
        row_count,
    } = clip.source;
    let pattern = song
        .patterns
        .iter()
        .find(|pattern| pattern.id == pattern_id)?;
    let lines_per_beat = song.transport.lines_per_beat.max(1);
    let notes = (row_start..row_start.saturating_add(row_count))
        .filter_map(|row| {
            let cell = pattern.cell(row, track_index)?;
            let NoteEvent::Note { pitch } = cell.note? else {
                return None;
            };
            Some(AbletonNote {
                pitch,
                velocity: cell.velocity.unwrap_or(100).min(127),
                start_beat: rows_to_beats(row.saturating_sub(row_start), lines_per_beat),
                duration_beats: cell_duration_beats(cell, lines_per_beat),
            })
        })
        .collect::<Vec<_>>();
    Some(AbletonClip {
        name: prompt_safe(&clip.name),
        track_index,
        scene_index,
        length_beats: rows_to_beats(row_count, lines_per_beat),
        source_pattern: prompt_safe(&pattern.name),
        notes,
    })
}

fn resize_tracks(song: &mut Song, target_len: usize) {
    while song.tracks.len() > target_len.max(1) {
        let index = song.tracks.len() - 1;
        let _ = song.delete_track(index);
    }
    while song.tracks.len() < target_len {
        song.create_track();
    }
}

fn write_clip_notes(pattern: &mut Pattern, clip: &AbletonClip, lines_per_beat: u8) {
    for note in &clip.notes {
        let row = beats_to_rows(note.start_beat, lines_per_beat);
        if let Some(cell) = pattern.cell_mut(row, clip.track_index) {
            cell.note = Some(NoteEvent::Note {
                pitch: note.pitch.min(127),
            });
            cell.velocity = Some(note.velocity.min(127));
            cell.gate =
                Some(beats_to_rows(note.duration_beats, lines_per_beat).clamp(1, 127) as u8);
        }
    }
}

fn clip_row_count(clip: &AbletonClip, lines_per_beat: u8) -> usize {
    let length_rows = beats_to_rows(clip.length_beats, lines_per_beat).max(1);
    let note_end = clip
        .notes
        .iter()
        .map(|note| beats_to_rows(note.start_beat + note.duration_beats, lines_per_beat))
        .max()
        .unwrap_or(0);
    length_rows.max(note_end).max(1)
}

fn rows_to_beats(rows: usize, lines_per_beat: u8) -> f32 {
    rows as f32 / f32::from(lines_per_beat.max(1))
}

fn beats_to_rows(beats: f32, lines_per_beat: u8) -> usize {
    (beats.max(0.0) * f32::from(lines_per_beat.max(1))).round() as usize
}

fn cell_duration_beats(cell: &PatternCell, lines_per_beat: u8) -> f32 {
    rows_to_beats(
        cell.gate.unwrap_or(1).clamp(1, 127) as usize,
        lines_per_beat,
    )
}

fn prompt_safe(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_control())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_exports_session_slots_as_ableton_clips() {
        let mut song = Song::empty();
        song.transport.bpm = 128;
        song.rename_track(0, "Bass").expect("track");
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(0, 0, NoteEvent::Note { pitch: 48 }, 100)
            .expect("note");
        let clip = song
            .create_clip(song.patterns[0].id, "Bass Clip", 0, 16)
            .expect("clip");
        let scene = song.create_scene("Intro").expect("scene");
        song.set_scene_clip(scene, song.tracks[0].id, Some(clip))
            .expect("slot");

        let preview = preview_push_to_ableton(&song);

        assert_eq!(preview.document.tempo_bpm, 128);
        assert_eq!(preview.summary.clip_count, 1);
        assert_eq!(preview.summary.note_count, 1);
        assert_eq!(preview.document.tracks[0].name, "Bass");
        assert_eq!(preview.document.clips[0].scene_index, Some(0));
        assert_eq!(preview.document.clips[0].notes[0].pitch, 48);
        assert_eq!(preview.document.clips[0].notes[0].velocity, 100);
    }

    #[test]
    fn pull_maps_ableton_snapshot_into_salieri_session() {
        let document = AbletonSessionDocument {
            schema_version: 1,
            tempo_bpm: 124,
            lines_per_beat: 4,
            tracks: vec![AbletonTrack {
                index: 0,
                name: "Lead".to_string(),
                midi_channel: Some(2),
            }],
            scenes: vec![AbletonScene {
                index: 0,
                name: "Drop".to_string(),
            }],
            clips: vec![AbletonClip {
                name: "Lead Clip".to_string(),
                track_index: 0,
                scene_index: Some(0),
                length_beats: 4.0,
                source_pattern: "Live".to_string(),
                notes: vec![AbletonNote {
                    pitch: 64,
                    velocity: 96,
                    start_beat: 1.0,
                    duration_beats: 0.5,
                }],
            }],
        };

        let preview = preview_pull_from_ableton(&document).expect("pull");
        let song = preview.song;

        assert_eq!(song.transport.bpm, 124);
        assert_eq!(song.tracks.len(), 1);
        assert_eq!(song.tracks[0].name, "Lead");
        assert_eq!(song.tracks[0].midi_channel, 2);
        assert_eq!(song.session.clips.len(), 1);
        assert_eq!(song.session.scenes.len(), 1);
        assert_eq!(song.session.scenes[0].slots[0].clip, Some(ClipId(1)));
        let cell = song.patterns[0].cell(4, 0).expect("cell");
        assert_eq!(cell.note, Some(NoteEvent::Note { pitch: 64 }));
        assert_eq!(cell.velocity, Some(96));
        assert_eq!(cell.gate, Some(2));
    }

    #[test]
    fn pull_rejects_missing_references() {
        let document = AbletonSessionDocument {
            schema_version: 1,
            tempo_bpm: 120,
            lines_per_beat: 4,
            tracks: vec![AbletonTrack {
                index: 0,
                name: "Track".to_string(),
                midi_channel: None,
            }],
            scenes: Vec::new(),
            clips: vec![AbletonClip {
                name: "Broken".to_string(),
                track_index: 2,
                scene_index: None,
                length_beats: 1.0,
                source_pattern: "Broken".to_string(),
                notes: Vec::new(),
            }],
        };

        assert!(matches!(
            preview_pull_from_ableton(&document),
            Err(AbletonBridgeError::MissingTrack {
                clip_index: 0,
                track_index: 2
            })
        ));
    }
}
