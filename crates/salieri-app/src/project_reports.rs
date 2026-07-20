use super::*;

use std::fmt::Write as _;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProjectReport {
    title: String,
    bpm: u16,
    lines_per_beat: u8,
    tracks: usize,
    patterns: usize,
    sequence_positions: usize,
    rows: usize,
    note_cells: usize,
    active_tracks: usize,
    density: f32,
    samples: usize,
    instruments: usize,
    annotations: usize,
    track_summaries: Vec<TrackReport>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrackReport {
    number: usize,
    name: String,
    note_cells: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CritiqueReport {
    score: u8,
    strengths: Vec<String>,
    issues: Vec<String>,
    suggested_revisions: Vec<String>,
    follow_up_commands: Vec<String>,
}

pub(crate) fn analyze_project(song: &Song) -> ProjectReport {
    let mut track_summaries = song
        .tracks
        .iter()
        .enumerate()
        .map(|(index, track)| TrackReport {
            number: index + 1,
            name: track.name.clone(),
            note_cells: 0,
        })
        .collect::<Vec<_>>();
    let mut rows = 0;
    let mut note_cells = 0;
    for pattern in &song.patterns {
        rows += pattern.row_count();
        for row in &pattern.rows {
            for (track_index, cell) in row.cells.iter().enumerate() {
                if matches!(cell.note, Some(NoteEvent::Note { .. })) {
                    note_cells += 1;
                    if let Some(track) = track_summaries.get_mut(track_index) {
                        track.note_cells += 1;
                    }
                }
            }
        }
    }
    let possible_cells = rows.saturating_mul(song.tracks.len()).max(1);
    let active_tracks = track_summaries
        .iter()
        .filter(|track| track.note_cells > 0)
        .count();
    ProjectReport {
        title: song.metadata.title.clone(),
        bpm: song.transport.bpm,
        lines_per_beat: song.transport.lines_per_beat,
        tracks: song.tracks.len(),
        patterns: song.patterns.len(),
        sequence_positions: song.sequence.len(),
        rows,
        note_cells,
        active_tracks,
        density: note_cells as f32 / possible_cells as f32,
        samples: song.samples.len(),
        instruments: song.instruments.len(),
        annotations: song.annotations.len(),
        track_summaries,
    }
}

pub(crate) fn critique_project(song: &Song) -> CritiqueReport {
    let report = analyze_project(song);
    let mut strengths = Vec::new();
    let mut issues = Vec::new();
    let mut suggested_revisions = Vec::new();
    if report.note_cells > 0 {
        strengths.push(format!(
            "{} note cell(s) give the project concrete musical material.",
            report.note_cells
        ));
    } else {
        issues.push("No note cells are present yet.".to_string());
        suggested_revisions.push("Generate a first motif in the active pattern.".to_string());
    }
    if report.active_tracks >= 2 {
        strengths.push(format!(
            "{} active track(s) provide basic arrangement contrast.",
            report.active_tracks
        ));
    } else {
        issues.push("Fewer than two tracks contain notes.".to_string());
        suggested_revisions.push("Add a supporting bass, chord, or percussion lane.".to_string());
    }
    if report.patterns > 1 || report.sequence_positions > 1 {
        strengths.push("The project has multiple arrangement units.".to_string());
    } else {
        issues.push("Only one pattern/sequence position is present.".to_string());
        suggested_revisions
            .push("Duplicate or extend the pattern into a contrasting section.".to_string());
    }
    if report.annotations > 0 {
        strengths.push("Text annotations can guide later critique and revision.".to_string());
    } else {
        issues.push("No project notes, lyrics, or cues are attached.".to_string());
        suggested_revisions
            .push("Add a project note describing the intended direction.".to_string());
    }
    if report.density > 0.45 {
        issues.push("Note density is high; the arrangement may need space.".to_string());
        suggested_revisions.push("Clear or thin selected rows to improve contrast.".to_string());
    }
    let penalty = issues.len().saturating_mul(12) as u8;
    let score = 100_u8.saturating_sub(penalty).max(40);
    CritiqueReport {
        score,
        strengths,
        issues,
        suggested_revisions,
        follow_up_commands: vec![
            ":revise add a contrasting variation to the current pattern".to_string(),
            ":ai show".to_string(),
            ":ai accept".to_string(),
        ],
    }
}

pub(crate) fn format_project_report(song: &Song) -> String {
    let report = analyze_project(song);
    let mut output = String::new();
    writeln!(output, "# Project Report").expect("write string");
    writeln!(output).expect("write string");
    writeln!(output, "- Title: {}", report.title).expect("write string");
    writeln!(
        output,
        "- Tempo: {} BPM, {} LPB",
        report.bpm, report.lines_per_beat
    )
    .expect("write string");
    writeln!(output, "- Tracks: {}", report.tracks).expect("write string");
    writeln!(output, "- Patterns: {}", report.patterns).expect("write string");
    writeln!(
        output,
        "- Sequence positions: {}",
        report.sequence_positions
    )
    .expect("write string");
    writeln!(output, "- Rows: {}", report.rows).expect("write string");
    writeln!(output, "- Note cells: {}", report.note_cells).expect("write string");
    writeln!(output, "- Active tracks: {}", report.active_tracks).expect("write string");
    writeln!(output, "- Density: {:.1}%", report.density * 100.0).expect("write string");
    writeln!(output, "- Samples: {}", report.samples).expect("write string");
    writeln!(output, "- Instruments: {}", report.instruments).expect("write string");
    writeln!(output, "- Text annotations: {}", report.annotations).expect("write string");
    writeln!(output).expect("write string");
    writeln!(output, "## Tracks").expect("write string");
    for track in report.track_summaries {
        writeln!(
            output,
            "- {:02}. {}: {} note cell(s)",
            track.number, track.name, track.note_cells
        )
        .expect("write string");
    }
    output
}

pub(crate) fn format_critique_report(song: &Song) -> String {
    let critique = critique_project(song);
    let mut output = String::new();
    writeln!(output, "# Critique Report").expect("write string");
    writeln!(output).expect("write string");
    writeln!(output, "- Score: {}/100", critique.score).expect("write string");
    write_section(&mut output, "Strengths", &critique.strengths);
    write_section(&mut output, "Issues", &critique.issues);
    write_section(
        &mut output,
        "Suggested revisions",
        &critique.suggested_revisions,
    );
    write_section(
        &mut output,
        "Follow-up commands",
        &critique.follow_up_commands,
    );
    output
}

pub(crate) fn project_report_summary(song: &Song) -> String {
    let report = analyze_project(song);
    format!(
        "Project report: {} note cell(s), {} active track(s), {:.1}% density",
        report.note_cells,
        report.active_tracks,
        report.density * 100.0
    )
}

pub(crate) fn critique_report_summary(song: &Song) -> String {
    let critique = critique_project(song);
    format!(
        "Critique score {}/100: {} issue(s), {} suggested revision(s)",
        critique.score,
        critique.issues.len(),
        critique.suggested_revisions.len()
    )
}

fn write_section(output: &mut String, title: &str, values: &[String]) {
    writeln!(output).expect("write string");
    writeln!(output, "## {title}").expect("write string");
    if values.is_empty() {
        writeln!(output, "- None").expect("write string");
    } else {
        for value in values {
            writeln!(output, "- {value}").expect("write string");
        }
    }
}
