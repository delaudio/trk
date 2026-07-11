use std::collections::BTreeMap;

use salieri_core::{ClipSource, NoteEvent, Pattern, PatternCell, PatternId, Song, TrackId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisProfile {
    pub schema_version: u32,
    pub project_title: String,
    pub transport: TransportProfile,
    pub totals: ProjectTotals,
    pub tracks: Vec<TrackProfile>,
    pub patterns: Vec<SectionProfile>,
    pub sequence: Vec<SectionProfile>,
    pub scenes: Vec<SectionProfile>,
    pub pitch_class_profile: [u32; 12],
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportProfile {
    pub bpm: u16,
    pub lines_per_beat: u8,
    pub row_duration_micros: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTotals {
    pub track_count: usize,
    pub pattern_count: usize,
    pub sequence_length: usize,
    pub clip_count: usize,
    pub scene_count: usize,
    pub note_count: u32,
    pub active_row_count: u32,
    pub density: f32,
    pub average_velocity: f32,
    pub pitch_min: Option<u8>,
    pub pitch_max: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackProfile {
    pub id: u32,
    pub name: String,
    pub role: TrackRole,
    pub midi_channel: u8,
    pub note_count: u32,
    pub active_row_count: u32,
    pub density: f32,
    pub average_velocity: f32,
    pub pitch_min: Option<u8>,
    pub pitch_max: Option<u8>,
    pub pitch_class_profile: [u32; 12],
    pub beat_phase_profile: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionProfile {
    pub index: usize,
    pub id: Option<u32>,
    pub name: String,
    pub source: SectionSource,
    pub row_count: usize,
    pub note_count: u32,
    pub active_row_count: u32,
    pub density: f32,
    pub average_velocity: f32,
    pub energy: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SectionSource {
    Pattern,
    Sequence,
    Scene,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrackRole {
    Drums,
    Bass,
    Lead,
    Harmony,
    Fx,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileComparison {
    pub left_title: String,
    pub right_title: String,
    pub deltas: ComparisonDeltas,
    pub role_counts: RoleComparison,
    pub pitch_class_distance: u32,
    pub section_energy_delta: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonDeltas {
    pub track_count: isize,
    pub pattern_count: isize,
    pub sequence_length: isize,
    pub clip_count: isize,
    pub scene_count: isize,
    pub note_count: i64,
    pub density: f32,
    pub average_velocity: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleComparison {
    pub left: BTreeMap<TrackRole, usize>,
    pub right: BTreeMap<TrackRole, usize>,
}

pub fn analyze_song(song: &Song) -> AnalysisProfile {
    let transport = TransportProfile {
        bpm: song.transport.bpm,
        lines_per_beat: song.transport.lines_per_beat,
        row_duration_micros: salieri_core::row_duration_micros(&song.transport),
    };
    let mut totals_accumulator = Accumulator::default();
    let mut track_accumulators = vec![Accumulator::default(); song.tracks.len()];

    for pattern in &song.patterns {
        collect_pattern(pattern, 0, pattern.row_count(), &mut totals_accumulator);
        collect_pattern_tracks(pattern, 0, pattern.row_count(), &mut track_accumulators);
    }

    let tracks = song
        .tracks
        .iter()
        .enumerate()
        .map(|(track_index, track)| {
            let accumulator = &track_accumulators[track_index];
            TrackProfile {
                id: track.id.0,
                name: track.name.clone(),
                role: classify_track_role(&track.name),
                midi_channel: track.midi_channel,
                note_count: accumulator.note_count,
                active_row_count: accumulator.active_row_count,
                density: density(accumulator.note_count, song_total_rows(song)),
                average_velocity: average_velocity(accumulator),
                pitch_min: accumulator.pitch_min,
                pitch_max: accumulator.pitch_max,
                pitch_class_profile: accumulator.pitch_class_profile,
                beat_phase_profile: accumulator.beat_phase_profile.clone(),
            }
        })
        .collect::<Vec<_>>();

    let patterns = song
        .patterns
        .iter()
        .enumerate()
        .map(|(index, pattern)| section_from_pattern(index, pattern, SectionSource::Pattern))
        .collect::<Vec<_>>();
    let sequence = song
        .sequence
        .iter()
        .enumerate()
        .filter_map(|(index, pattern_id)| {
            song.patterns
                .iter()
                .find(|pattern| pattern.id == *pattern_id)
                .map(|pattern| section_from_pattern(index, pattern, SectionSource::Sequence))
        })
        .collect::<Vec<_>>();
    let scenes = song
        .session
        .scenes
        .iter()
        .enumerate()
        .map(|(index, scene)| {
            let mut accumulator = Accumulator::default();
            let mut row_count = 0;
            for slot in &scene.slots {
                let Some(clip_id) = slot.clip else {
                    continue;
                };
                let Some(clip) = song.session.clips.iter().find(|clip| clip.id == clip_id) else {
                    continue;
                };
                let ClipSource::Pattern {
                    pattern_id,
                    row_start,
                    row_count: clip_rows,
                } = clip.source;
                let Some(pattern) = pattern_by_id(song, pattern_id) else {
                    continue;
                };
                let Some(track_index) = track_index_by_id(song, slot.track) else {
                    continue;
                };
                collect_track_range(pattern, track_index, row_start, clip_rows, &mut accumulator);
                row_count = row_count.max(clip_rows);
            }
            section_from_accumulator(
                index,
                Some(scene.id.0),
                scene.name.clone(),
                SectionSource::Scene,
                row_count,
                accumulator,
            )
        })
        .collect::<Vec<_>>();

    AnalysisProfile {
        schema_version: 1,
        project_title: song.metadata.title.clone(),
        transport,
        totals: ProjectTotals {
            track_count: song.tracks.len(),
            pattern_count: song.patterns.len(),
            sequence_length: song.sequence.len(),
            clip_count: song.session.clips.len(),
            scene_count: song.session.scenes.len(),
            note_count: totals_accumulator.note_count,
            active_row_count: totals_accumulator.active_row_count,
            density: density(totals_accumulator.note_count, song_total_cells(song)),
            average_velocity: average_velocity(&totals_accumulator),
            pitch_min: totals_accumulator.pitch_min,
            pitch_max: totals_accumulator.pitch_max,
        },
        tracks,
        patterns,
        sequence,
        scenes,
        pitch_class_profile: totals_accumulator.pitch_class_profile,
    }
}

pub fn compare_profiles(left: &AnalysisProfile, right: &AnalysisProfile) -> ProfileComparison {
    ProfileComparison {
        left_title: left.project_title.clone(),
        right_title: right.project_title.clone(),
        deltas: ComparisonDeltas {
            track_count: delta_usize(left.totals.track_count, right.totals.track_count),
            pattern_count: delta_usize(left.totals.pattern_count, right.totals.pattern_count),
            sequence_length: delta_usize(left.totals.sequence_length, right.totals.sequence_length),
            clip_count: delta_usize(left.totals.clip_count, right.totals.clip_count),
            scene_count: delta_usize(left.totals.scene_count, right.totals.scene_count),
            note_count: i64::from(right.totals.note_count) - i64::from(left.totals.note_count),
            density: rounded(right.totals.density - left.totals.density),
            average_velocity: rounded(right.totals.average_velocity - left.totals.average_velocity),
        },
        role_counts: RoleComparison {
            left: role_counts(left),
            right: role_counts(right),
        },
        pitch_class_distance: left
            .pitch_class_profile
            .iter()
            .zip(right.pitch_class_profile)
            .map(|(left, right)| left.abs_diff(right))
            .sum(),
        section_energy_delta: rounded(
            average_section_energy(&right.sequence) - average_section_energy(&left.sequence),
        ),
    }
}

pub fn render_profile_markdown(profile: &AnalysisProfile) -> String {
    let mut output = String::new();
    output.push_str(&format!("# Analysis: {}\n\n", profile.project_title));
    output.push_str("## Summary\n\n");
    output.push_str(&format!(
        "- Tempo: {} BPM, {} LPB\n",
        profile.transport.bpm, profile.transport.lines_per_beat
    ));
    output.push_str(&format!(
        "- Tracks: {} | Patterns: {} | Sequence positions: {}\n",
        profile.totals.track_count, profile.totals.pattern_count, profile.totals.sequence_length
    ));
    output.push_str(&format!(
        "- Clips: {} | Scenes: {}\n",
        profile.totals.clip_count, profile.totals.scene_count
    ));
    output.push_str(&format!(
        "- Notes: {} | Density: {:.3} | Average velocity: {:.1}\n",
        profile.totals.note_count, profile.totals.density, profile.totals.average_velocity
    ));
    if let (Some(min), Some(max)) = (profile.totals.pitch_min, profile.totals.pitch_max) {
        output.push_str(&format!("- Pitch range: {min}..{max}\n"));
    }
    output.push_str("\n## Tracks\n\n");
    output.push_str("| Track | Role | Notes | Density | Pitch Range |\n");
    output.push_str("| --- | --- | ---: | ---: | --- |\n");
    for track in &profile.tracks {
        let range = match (track.pitch_min, track.pitch_max) {
            (Some(min), Some(max)) => format!("{min}..{max}"),
            _ => "-".to_string(),
        };
        output.push_str(&format!(
            "| {} | {:?} | {} | {:.3} | {} |\n",
            track.name, track.role, track.note_count, track.density, range
        ));
    }
    output.push_str("\n## Arrangement\n\n");
    output.push_str("| Section | Source | Notes | Density | Energy |\n");
    output.push_str("| --- | --- | ---: | ---: | ---: |\n");
    for section in &profile.sequence {
        output.push_str(&format!(
            "| {} | {:?} | {} | {:.3} | {:.3} |\n",
            section.name, section.source, section.note_count, section.density, section.energy
        ));
    }
    if !profile.scenes.is_empty() {
        output.push_str("\n## Scenes\n\n");
        output.push_str("| Scene | Notes | Density | Energy |\n");
        output.push_str("| --- | ---: | ---: | ---: |\n");
        for scene in &profile.scenes {
            output.push_str(&format!(
                "| {} | {} | {:.3} | {:.3} |\n",
                scene.name, scene.note_count, scene.density, scene.energy
            ));
        }
    }
    output.push_str("\n## Generation Guidance\n\n");
    output.push_str("- Use track roles as constraints for future clip or pattern generation.\n");
    output.push_str(
        "- Use density, pitch range, and section energy to match arrangement intensity.\n",
    );
    output.push_str("- Use pitch-class and beat-phase profiles as local style references.\n");
    output
}

pub fn render_comparison_markdown(comparison: &ProfileComparison) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "# Comparison: {} -> {}\n\n",
        comparison.left_title, comparison.right_title
    ));
    output.push_str("## Deltas\n\n");
    output.push_str(&format!(
        "- Tracks: {:+}\n- Patterns: {:+}\n- Sequence positions: {:+}\n- Clips: {:+}\n- Scenes: {:+}\n- Notes: {:+}\n- Density: {:+.3}\n- Average velocity: {:+.1}\n",
        comparison.deltas.track_count,
        comparison.deltas.pattern_count,
        comparison.deltas.sequence_length,
        comparison.deltas.clip_count,
        comparison.deltas.scene_count,
        comparison.deltas.note_count,
        comparison.deltas.density,
        comparison.deltas.average_velocity
    ));
    output.push_str(&format!(
        "\nPitch-class distance: {}\n\nSection energy delta: {:+.3}\n",
        comparison.pitch_class_distance, comparison.section_energy_delta
    ));
    output.push_str("\n## Role Counts\n\n");
    output.push_str("| Role | Left | Right |\n| --- | ---: | ---: |\n");
    for role in [
        TrackRole::Drums,
        TrackRole::Bass,
        TrackRole::Lead,
        TrackRole::Harmony,
        TrackRole::Fx,
        TrackRole::Other,
    ] {
        output.push_str(&format!(
            "| {:?} | {} | {} |\n",
            role,
            comparison.role_counts.left.get(&role).copied().unwrap_or(0),
            comparison
                .role_counts
                .right
                .get(&role)
                .copied()
                .unwrap_or(0)
        ));
    }
    output
}

pub fn classify_track_role(name: &str) -> TrackRole {
    let normalized = name.to_ascii_lowercase();
    let tokens = normalized
        .split(|value: char| !value.is_ascii_alphanumeric())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if contains_any(
        &tokens,
        &["drum", "drums", "kick", "snare", "hat", "perc", "tom"],
    ) {
        TrackRole::Drums
    } else if contains_any(&tokens, &["bass", "sub", "808"]) {
        TrackRole::Bass
    } else if contains_any(&tokens, &["lead", "melody", "arp", "riff", "hook"]) {
        TrackRole::Lead
    } else if contains_any(
        &tokens,
        &["pad", "chord", "chords", "keys", "string", "strings"],
    ) {
        TrackRole::Harmony
    } else if contains_any(&tokens, &["fx", "sfx", "noise", "impact", "riser"]) {
        TrackRole::Fx
    } else {
        TrackRole::Other
    }
}

fn contains_any(tokens: &[&str], needles: &[&str]) -> bool {
    tokens
        .iter()
        .any(|token| needles.iter().any(|needle| token == needle))
}

#[derive(Debug, Clone, Default)]
struct Accumulator {
    note_count: u32,
    active_row_count: u32,
    velocity_sum: u32,
    pitch_min: Option<u8>,
    pitch_max: Option<u8>,
    pitch_class_profile: [u32; 12],
    beat_phase_profile: Vec<u32>,
}

fn collect_pattern(
    pattern: &Pattern,
    row_start: usize,
    row_count: usize,
    accumulator: &mut Accumulator,
) {
    for row_index in row_start..row_start.saturating_add(row_count).min(pattern.row_count()) {
        let Some(row) = pattern.rows.get(row_index) else {
            continue;
        };
        let before = accumulator.note_count;
        for cell in &row.cells {
            collect_cell(cell, row_index, accumulator);
        }
        if accumulator.note_count > before {
            accumulator.active_row_count = accumulator.active_row_count.saturating_add(1);
        }
    }
}

fn collect_pattern_tracks(
    pattern: &Pattern,
    row_start: usize,
    row_count: usize,
    accumulators: &mut [Accumulator],
) {
    for row_index in row_start..row_start.saturating_add(row_count).min(pattern.row_count()) {
        let Some(row) = pattern.rows.get(row_index) else {
            continue;
        };
        for (track_index, cell) in row.cells.iter().enumerate() {
            if let Some(accumulator) = accumulators.get_mut(track_index) {
                let before = accumulator.note_count;
                collect_cell(cell, row_index, accumulator);
                if accumulator.note_count > before {
                    accumulator.active_row_count = accumulator.active_row_count.saturating_add(1);
                }
            }
        }
    }
}

fn collect_track_range(
    pattern: &Pattern,
    track_index: usize,
    row_start: usize,
    row_count: usize,
    accumulator: &mut Accumulator,
) {
    for row_index in row_start..row_start.saturating_add(row_count).min(pattern.row_count()) {
        let Some(cell) = pattern.cell(row_index, track_index) else {
            continue;
        };
        let before = accumulator.note_count;
        collect_cell(cell, row_index, accumulator);
        if accumulator.note_count > before {
            accumulator.active_row_count = accumulator.active_row_count.saturating_add(1);
        }
    }
}

fn collect_cell(cell: &PatternCell, row_index: usize, accumulator: &mut Accumulator) {
    let Some(NoteEvent::Note { pitch }) = cell.note else {
        return;
    };
    accumulator.note_count = accumulator.note_count.saturating_add(1);
    accumulator.velocity_sum = accumulator
        .velocity_sum
        .saturating_add(u32::from(cell.velocity.unwrap_or(0x7f).min(0x7f)));
    accumulator.pitch_min = Some(
        accumulator
            .pitch_min
            .map_or(pitch, |value| value.min(pitch)),
    );
    accumulator.pitch_max = Some(
        accumulator
            .pitch_max
            .map_or(pitch, |value| value.max(pitch)),
    );
    accumulator.pitch_class_profile[(pitch % 12) as usize] += 1;
    let phase = row_index % 16;
    if accumulator.beat_phase_profile.len() <= phase {
        accumulator.beat_phase_profile.resize(phase + 1, 0);
    }
    accumulator.beat_phase_profile[phase] += 1;
}

fn section_from_pattern(index: usize, pattern: &Pattern, source: SectionSource) -> SectionProfile {
    let mut accumulator = Accumulator::default();
    collect_pattern(pattern, 0, pattern.row_count(), &mut accumulator);
    section_from_accumulator(
        index,
        Some(pattern.id.0),
        pattern.name.clone(),
        source,
        pattern.row_count(),
        accumulator,
    )
}

fn section_from_accumulator(
    index: usize,
    id: Option<u32>,
    name: String,
    source: SectionSource,
    row_count: usize,
    accumulator: Accumulator,
) -> SectionProfile {
    let section_density = density(accumulator.note_count, row_count.max(1));
    SectionProfile {
        index,
        id,
        name,
        source,
        row_count,
        note_count: accumulator.note_count,
        active_row_count: accumulator.active_row_count,
        density: section_density,
        average_velocity: average_velocity(&accumulator),
        energy: rounded(section_density * (average_velocity(&accumulator) / 127.0)),
    }
}

fn pattern_by_id(song: &Song, pattern_id: PatternId) -> Option<&Pattern> {
    song.patterns
        .iter()
        .find(|pattern| pattern.id == pattern_id)
}

fn track_index_by_id(song: &Song, track_id: TrackId) -> Option<usize> {
    song.tracks.iter().position(|track| track.id == track_id)
}

fn song_total_rows(song: &Song) -> usize {
    song.patterns
        .iter()
        .map(Pattern::row_count)
        .sum::<usize>()
        .max(1)
}

fn song_total_cells(song: &Song) -> usize {
    song.patterns
        .iter()
        .map(|pattern| pattern.row_count().saturating_mul(song.tracks.len()))
        .sum::<usize>()
        .max(1)
}

fn density(note_count: u32, denominator: usize) -> f32 {
    rounded(note_count as f32 / denominator.max(1) as f32)
}

fn average_velocity(accumulator: &Accumulator) -> f32 {
    if accumulator.note_count == 0 {
        0.0
    } else {
        rounded(accumulator.velocity_sum as f32 / accumulator.note_count as f32)
    }
}

fn rounded(value: f32) -> f32 {
    (value * 1000.0).round() / 1000.0
}

fn delta_usize(left: usize, right: usize) -> isize {
    right as isize - left as isize
}

fn role_counts(profile: &AnalysisProfile) -> BTreeMap<TrackRole, usize> {
    let mut counts = BTreeMap::new();
    for track in &profile.tracks {
        *counts.entry(track.role).or_insert(0) += 1;
    }
    counts
}

fn average_section_energy(sections: &[SectionProfile]) -> f32 {
    if sections.is_empty() {
        0.0
    } else {
        rounded(sections.iter().map(|section| section.energy).sum::<f32>() / sections.len() as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use salieri_core::NoteEvent;

    #[test]
    fn classifies_track_roles_deterministically() {
        assert_eq!(classify_track_role("Kick Drum"), TrackRole::Drums);
        assert_eq!(classify_track_role("Sub Bass"), TrackRole::Bass);
        assert_eq!(classify_track_role("Lead Arp"), TrackRole::Lead);
        assert_eq!(classify_track_role("Warm Chords"), TrackRole::Harmony);
        assert_eq!(classify_track_role("Noise FX"), TrackRole::Fx);
        assert_eq!(classify_track_role("Track 06"), TrackRole::Other);
    }

    #[test]
    fn analyzes_tracker_patterns_and_session_scenes() {
        let mut song = Song::empty();
        song.rename_track(0, "Kick Drum").expect("rename");
        song.rename_track(1, "Sub Bass").expect("rename");
        song.resize_pattern(0, 16).expect("resize");
        let pattern = song.current_pattern_mut().expect("pattern");
        pattern
            .set_note(0, 0, NoteEvent::Note { pitch: 36 }, 100)
            .expect("kick");
        pattern
            .set_note(4, 0, NoteEvent::Note { pitch: 36 }, 100)
            .expect("kick");
        pattern
            .set_note(8, 1, NoteEvent::Note { pitch: 40 }, 90)
            .expect("bass");
        let clip = song
            .create_clip(song.patterns[0].id, "Clip", 0, 8)
            .expect("clip");
        let scene = song.create_scene("Scene").expect("scene");
        song.set_scene_clip(scene, song.tracks[0].id, Some(clip))
            .expect("slot");

        let profile = analyze_song(&song);

        assert_eq!(profile.totals.note_count, 3);
        assert_eq!(profile.tracks[0].role, TrackRole::Drums);
        assert_eq!(profile.tracks[1].role, TrackRole::Bass);
        assert_eq!(profile.tracks[0].note_count, 2);
        assert_eq!(profile.totals.pitch_min, Some(36));
        assert_eq!(profile.totals.pitch_max, Some(40));
        assert_eq!(profile.sequence.len(), 1);
        assert_eq!(profile.scenes.len(), 1);
        assert_eq!(profile.scenes[0].note_count, 2);
    }

    #[test]
    fn renders_markdown_and_compares_profiles() {
        let mut left = Song::empty();
        left.resize_pattern(0, 8).expect("resize");
        left.current_pattern_mut()
            .expect("pattern")
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 64)
            .expect("note");
        let mut right = left.clone();
        right
            .current_pattern_mut()
            .expect("pattern")
            .set_note(4, 1, NoteEvent::Note { pitch: 67 }, 127)
            .expect("note");

        let left = analyze_song(&left);
        let right = analyze_song(&right);
        let comparison = compare_profiles(&left, &right);

        assert_eq!(comparison.deltas.note_count, 1);
        assert!(comparison.pitch_class_distance > 0);
        assert!(render_profile_markdown(&right).contains("Generation Guidance"));
        assert!(render_comparison_markdown(&comparison).contains("Density"));
    }
}
