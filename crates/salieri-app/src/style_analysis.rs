use super::*;

use std::fmt::Write as _;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StyleAnalysis {
    title: String,
    bpm: u16,
    tracks: usize,
    patterns: usize,
    sequence_positions: usize,
    note_cells: usize,
    active_tracks: usize,
    density: f32,
    average_velocity: f32,
    pitch_min: Option<u8>,
    pitch_max: Option<u8>,
    energy: String,
    roles: Vec<TrackRoleAnalysis>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TrackRoleAnalysis {
    track: usize,
    name: String,
    role: String,
    note_cells: usize,
    average_pitch: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StyleComparison {
    left: StyleAnalysis,
    right: StyleAnalysis,
    tempo_delta: i16,
    note_delta: isize,
    active_track_delta: isize,
    density_delta: f32,
    summary: Vec<String>,
}

pub(crate) fn analyze_style(song: &Song) -> StyleAnalysis {
    let mut roles = song
        .tracks
        .iter()
        .enumerate()
        .map(|(index, track)| TrackRoleAccumulator {
            track: index + 1,
            name: track.name.clone(),
            pitches: Vec::new(),
            velocity_sum: 0,
        })
        .collect::<Vec<_>>();
    let mut note_cells = 0;
    let mut velocity_sum = 0_u64;
    let mut pitch_min = None::<u8>;
    let mut pitch_max = None::<u8>;
    let mut rows = 0;
    for pattern in &song.patterns {
        rows += pattern.row_count();
        for row in &pattern.rows {
            for (track_index, cell) in row.cells.iter().enumerate() {
                if let Some(NoteEvent::Note { pitch }) = cell.note {
                    note_cells += 1;
                    let velocity = u64::from(cell.velocity.unwrap_or(0x7f));
                    velocity_sum += velocity;
                    pitch_min = Some(pitch_min.map_or(pitch, |value| value.min(pitch)));
                    pitch_max = Some(pitch_max.map_or(pitch, |value| value.max(pitch)));
                    if let Some(role) = roles.get_mut(track_index) {
                        role.pitches.push(pitch);
                        role.velocity_sum += velocity;
                    }
                }
            }
        }
    }
    let possible_cells = rows.saturating_mul(song.tracks.len()).max(1);
    let density = note_cells as f32 / possible_cells as f32;
    let average_velocity = if note_cells == 0 {
        0.0
    } else {
        velocity_sum as f32 / note_cells as f32
    };
    let energy_score = density * (average_velocity / 127.0);
    StyleAnalysis {
        title: song.metadata.title.clone(),
        bpm: song.transport.bpm,
        tracks: song.tracks.len(),
        patterns: song.patterns.len(),
        sequence_positions: song.sequence.len(),
        note_cells,
        active_tracks: roles.iter().filter(|role| !role.pitches.is_empty()).count(),
        density,
        average_velocity,
        pitch_min,
        pitch_max,
        energy: classify_energy(energy_score),
        roles: roles
            .into_iter()
            .map(TrackRoleAccumulator::finish)
            .collect(),
    }
}

pub(crate) fn compare_styles(left: &Song, right: &Song) -> StyleComparison {
    let left = analyze_style(left);
    let right = analyze_style(right);
    let mut summary = Vec::new();
    summary.push(format!(
        "Tempo delta: {} BPM",
        right.bpm as i16 - left.bpm as i16
    ));
    summary.push(format!(
        "Note delta: {}",
        right.note_cells as isize - left.note_cells as isize
    ));
    summary.push(format!(
        "Density delta: {:.1} percentage point(s)",
        (right.density - left.density) * 100.0
    ));
    summary.push(format!(
        "Active track delta: {}",
        right.active_tracks as isize - left.active_tracks as isize
    ));
    StyleComparison {
        tempo_delta: right.bpm as i16 - left.bpm as i16,
        note_delta: right.note_cells as isize - left.note_cells as isize,
        active_track_delta: right.active_tracks as isize - left.active_tracks as isize,
        density_delta: right.density - left.density,
        left,
        right,
        summary,
    }
}

pub(crate) fn format_analysis_output(
    analysis: &StyleAnalysis,
    format: AnalysisOutputFormat,
) -> Result<String> {
    match format {
        AnalysisOutputFormat::Text => Ok(format_style_analysis_text(analysis)),
        AnalysisOutputFormat::Json => {
            let json = serde_json::to_string_pretty(analysis)
                .context("failed to encode style analysis JSON")?;
            Ok(format!("{json}\n"))
        }
    }
}

pub(crate) fn format_comparison_output(
    comparison: &StyleComparison,
    format: AnalysisOutputFormat,
) -> Result<String> {
    match format {
        AnalysisOutputFormat::Text => Ok(format_style_comparison_text(comparison)),
        AnalysisOutputFormat::Json => {
            let json = serde_json::to_string_pretty(comparison)
                .context("failed to encode style comparison JSON")?;
            Ok(format!("{json}\n"))
        }
    }
}

pub(crate) fn format_style_analysis_text(analysis: &StyleAnalysis) -> String {
    let mut output = String::new();
    writeln!(output, "# Style Analysis").expect("write string");
    writeln!(output).expect("write string");
    writeln!(output, "- Title: {}", analysis.title).expect("write string");
    writeln!(output, "- Tempo: {} BPM", analysis.bpm).expect("write string");
    writeln!(output, "- Tracks: {}", analysis.tracks).expect("write string");
    writeln!(output, "- Patterns: {}", analysis.patterns).expect("write string");
    writeln!(
        output,
        "- Sequence positions: {}",
        analysis.sequence_positions
    )
    .expect("write string");
    writeln!(output, "- Note cells: {}", analysis.note_cells).expect("write string");
    writeln!(output, "- Active tracks: {}", analysis.active_tracks).expect("write string");
    writeln!(output, "- Density: {:.1}%", analysis.density * 100.0).expect("write string");
    writeln!(
        output,
        "- Average velocity: {:.1}",
        analysis.average_velocity
    )
    .expect("write string");
    writeln!(output, "- Pitch range: {}", format_pitch_range(analysis)).expect("write string");
    writeln!(output, "- Energy: {}", analysis.energy).expect("write string");
    writeln!(output).expect("write string");
    writeln!(output, "## Track roles").expect("write string");
    for role in &analysis.roles {
        writeln!(
            output,
            "- {:02}. {}: {} ({} note cell(s))",
            role.track, role.name, role.role, role.note_cells
        )
        .expect("write string");
    }
    output
}

pub(crate) fn format_style_comparison_text(comparison: &StyleComparison) -> String {
    let mut output = String::new();
    writeln!(output, "# Style Comparison").expect("write string");
    writeln!(output).expect("write string");
    writeln!(output, "- Left: {}", comparison.left.title).expect("write string");
    writeln!(output, "- Right: {}", comparison.right.title).expect("write string");
    for line in &comparison.summary {
        writeln!(output, "- {line}").expect("write string");
    }
    output
}

pub(crate) fn style_analysis_summary(analysis: &StyleAnalysis) -> String {
    format!(
        "Style analysis: {} notes, {} active track(s), {} energy",
        analysis.note_cells, analysis.active_tracks, analysis.energy
    )
}

pub(crate) fn style_comparison_summary(comparison: &StyleComparison) -> String {
    format!(
        "Style comparison: note delta {}, density delta {:.1} pp",
        comparison.note_delta,
        comparison.density_delta * 100.0
    )
}

struct TrackRoleAccumulator {
    track: usize,
    name: String,
    pitches: Vec<u8>,
    velocity_sum: u64,
}

impl TrackRoleAccumulator {
    fn finish(self) -> TrackRoleAnalysis {
        let average_pitch = if self.pitches.is_empty() {
            None
        } else {
            Some(
                self.pitches
                    .iter()
                    .map(|pitch| f32::from(*pitch))
                    .sum::<f32>()
                    / self.pitches.len() as f32,
            )
        };
        TrackRoleAnalysis {
            track: self.track,
            role: infer_track_role(&self.name, average_pitch, self.pitches.len()),
            name: self.name,
            note_cells: self.pitches.len(),
            average_pitch,
        }
    }
}

fn infer_track_role(name: &str, average_pitch: Option<f32>, notes: usize) -> String {
    let name = name.to_ascii_lowercase();
    if notes == 0 {
        "empty".to_string()
    } else if name.contains("drum") || name.contains("kick") || name.contains("perc") {
        "percussion".to_string()
    } else if average_pitch.is_some_and(|pitch| pitch < 48.0) || name.contains("bass") {
        "bass".to_string()
    } else if average_pitch.is_some_and(|pitch| pitch >= 72.0) || name.contains("lead") {
        "lead".to_string()
    } else {
        "harmony".to_string()
    }
}

fn classify_energy(score: f32) -> String {
    if score < 0.05 {
        "low".to_string()
    } else if score < 0.18 {
        "medium".to_string()
    } else {
        "high".to_string()
    }
}

fn format_pitch_range(analysis: &StyleAnalysis) -> String {
    match (analysis.pitch_min, analysis.pitch_max) {
        (Some(min), Some(max)) => format!("{min}-{max}"),
        _ => "none".to_string(),
    }
}
