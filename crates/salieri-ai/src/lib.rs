use salieri_core::{NoteEvent, Song};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiPatternRequest {
    pub prompt: String,
    pub pattern: usize,
    pub track: usize,
    pub rows: usize,
    pub root_pitch: u8,
    pub velocity: u8,
    pub guidance: Option<AiGuidanceReference>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiGuidanceReference {
    pub style_path: Option<String>,
    pub profile_path: Option<String>,
    pub dossier_path: Option<String>,
    pub palette_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchDossier {
    pub schema_version: u32,
    pub title: String,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub observations: Vec<String>,
    #[serde(default)]
    pub guardrails: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationalPalette {
    pub schema_version: u32,
    pub title: String,
    #[serde(default)]
    pub track_roles: Vec<PaletteTrackRole>,
    #[serde(default)]
    pub sound_sources: Vec<String>,
    #[serde(default)]
    pub arrangement_functions: Vec<String>,
    #[serde(default)]
    pub guardrails: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaletteTrackRole {
    pub role: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuidanceSummary {
    pub title: String,
    pub kind: GuidanceKind,
    pub bullet_count: usize,
    pub prompt_safe_summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuidanceKind {
    Dossier,
    Palette,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiProposal {
    pub source: AiSource,
    pub prompt: String,
    pub summary: String,
    pub edits: Vec<AiEdit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiSource {
    LocalDeterministic,
    External { provider: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiEdit {
    SetNote {
        pattern: usize,
        row: usize,
        track: usize,
        pitch: u8,
        velocity: u8,
    },
    ClearCell {
        pattern: usize,
        row: usize,
        track: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellAddress {
    pub pattern: usize,
    pub row: usize,
    pub track: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiProposalPreview {
    pub touched_cells: Vec<CellAddress>,
}

pub trait AiProposalProvider {
    fn propose(&self, song: &Song, request: &AiPatternRequest) -> Result<AiProposal, AiError>;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LocalDeterministicProvider;

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("pattern {0} does not exist")]
    MissingPattern(usize),
    #[error("row {row} does not exist in pattern {pattern}")]
    MissingRow { pattern: usize, row: usize },
    #[error("track {0} does not exist")]
    MissingTrack(usize),
    #[error("prompt cannot be empty")]
    EmptyPrompt,
    #[error("proposal contains no edits")]
    EmptyProposal,
    #[error("schema version {0} is not supported")]
    UnsupportedGuidanceSchema(u32),
    #[error("guidance title cannot be empty")]
    EmptyGuidanceTitle,
    #[error("guidance must contain at least one prompt-safe item")]
    EmptyGuidance,
}

pub fn validate_dossier(dossier: &ResearchDossier) -> Result<GuidanceSummary, AiError> {
    if dossier.schema_version != 1 {
        return Err(AiError::UnsupportedGuidanceSchema(dossier.schema_version));
    }
    let title = clean_title(&dossier.title)?;
    let items = dossier
        .keywords
        .iter()
        .chain(dossier.observations.iter())
        .chain(dossier.guardrails.iter())
        .map(|value| prompt_safe(value))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    guidance_summary(title, GuidanceKind::Dossier, items)
}

pub fn validate_palette(palette: &OperationalPalette) -> Result<GuidanceSummary, AiError> {
    if palette.schema_version != 1 {
        return Err(AiError::UnsupportedGuidanceSchema(palette.schema_version));
    }
    let title = clean_title(&palette.title)?;
    let role_items = palette.track_roles.iter().filter_map(|track_role| {
        let role = prompt_safe(&track_role.role);
        let description = prompt_safe(&track_role.description);
        match (role.is_empty(), description.is_empty()) {
            (true, true) => None,
            (false, true) => Some(role),
            (true, false) => Some(description),
            (false, false) => Some(format!("{role}: {description}")),
        }
    });
    let items = role_items
        .chain(palette.sound_sources.iter().map(|value| prompt_safe(value)))
        .chain(
            palette
                .arrangement_functions
                .iter()
                .map(|value| prompt_safe(value)),
        )
        .chain(palette.guardrails.iter().map(|value| prompt_safe(value)))
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>();
    guidance_summary(title, GuidanceKind::Palette, items)
}

impl AiProposalProvider for LocalDeterministicProvider {
    fn propose(&self, song: &Song, request: &AiPatternRequest) -> Result<AiProposal, AiError> {
        if request.prompt.trim().is_empty() {
            return Err(AiError::EmptyPrompt);
        }
        let pattern = song
            .pattern(request.pattern)
            .ok_or(AiError::MissingPattern(request.pattern))?;
        if request.track >= song.tracks.len() {
            return Err(AiError::MissingTrack(request.track));
        }

        let row_limit = request.rows.min(pattern.row_count()).max(1);
        let seed = stable_prompt_seed(&request.prompt);
        let scale = [0_u8, 2, 3, 5, 7, 10];
        let mut edits = Vec::new();

        for row in 0..row_limit {
            let step = row + seed as usize;
            if step.is_multiple_of(4) || step.is_multiple_of(7) {
                let interval = scale[(step / 2) % scale.len()];
                edits.push(AiEdit::SetNote {
                    pattern: request.pattern,
                    row,
                    track: request.track,
                    pitch: request.root_pitch.saturating_add(interval).min(127),
                    velocity: request.velocity.min(127),
                });
            }
        }

        if edits.is_empty() {
            edits.push(AiEdit::SetNote {
                pattern: request.pattern,
                row: 0,
                track: request.track,
                pitch: request.root_pitch.min(127),
                velocity: request.velocity.min(127),
            });
        }

        Ok(AiProposal {
            source: AiSource::LocalDeterministic,
            prompt: request.prompt.clone(),
            summary: format!(
                "Local deterministic pattern sketch with {} edits",
                edits.len()
            ),
            edits,
        })
    }
}

pub fn preview_proposal(song: &Song, proposal: &AiProposal) -> Result<AiProposalPreview, AiError> {
    validate_proposal(song, proposal)?;
    Ok(AiProposalPreview {
        touched_cells: proposal.edits.iter().map(AiEdit::address).collect(),
    })
}

pub fn apply_proposal(
    song: &mut Song,
    proposal: &AiProposal,
) -> Result<AiProposalPreview, AiError> {
    let preview = preview_proposal(song, proposal)?;
    for edit in &proposal.edits {
        match *edit {
            AiEdit::SetNote {
                pattern,
                row,
                track,
                pitch,
                velocity,
            } => {
                let cell = song
                    .pattern_mut(pattern)
                    .and_then(|pattern| pattern.cell_mut(row, track))
                    .expect("proposal was validated before apply");
                cell.note = Some(NoteEvent::Note { pitch });
                cell.velocity = Some(velocity);
            }
            AiEdit::ClearCell {
                pattern,
                row,
                track,
            } => {
                let cell = song
                    .pattern_mut(pattern)
                    .and_then(|pattern| pattern.cell_mut(row, track))
                    .expect("proposal was validated before apply");
                cell.note = None;
                cell.velocity = None;
                cell.gate = None;
                cell.command = None;
            }
        }
    }
    Ok(preview)
}

impl AiEdit {
    fn address(&self) -> CellAddress {
        match *self {
            Self::SetNote {
                pattern,
                row,
                track,
                ..
            }
            | Self::ClearCell {
                pattern,
                row,
                track,
            } => CellAddress {
                pattern,
                row,
                track,
            },
        }
    }
}

fn validate_proposal(song: &Song, proposal: &AiProposal) -> Result<(), AiError> {
    if proposal.edits.is_empty() {
        return Err(AiError::EmptyProposal);
    }

    for edit in &proposal.edits {
        let address = edit.address();
        let pattern = song
            .pattern(address.pattern)
            .ok_or(AiError::MissingPattern(address.pattern))?;
        if address.row >= pattern.row_count() {
            return Err(AiError::MissingRow {
                pattern: address.pattern,
                row: address.row,
            });
        }
        if address.track >= song.tracks.len() {
            return Err(AiError::MissingTrack(address.track));
        }
    }

    Ok(())
}

fn clean_title(title: &str) -> Result<String, AiError> {
    let title = prompt_safe(title);
    if title.is_empty() {
        Err(AiError::EmptyGuidanceTitle)
    } else {
        Ok(title)
    }
}

fn guidance_summary(
    title: String,
    kind: GuidanceKind,
    items: Vec<String>,
) -> Result<GuidanceSummary, AiError> {
    if items.is_empty() {
        return Err(AiError::EmptyGuidance);
    }
    let bullet_count = items.len();
    let prompt_safe_summary = items
        .into_iter()
        .take(8)
        .map(|item| format!("- {item}"))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(GuidanceSummary {
        title,
        kind,
        bullet_count,
        prompt_safe_summary,
    })
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

fn stable_prompt_seed(prompt: &str) -> u64 {
    prompt
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            hash.wrapping_mul(0x100_0000_01b3) ^ u64::from(byte)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_provider_generates_deterministic_reviewable_proposals() {
        let song = Song::empty();
        let request = AiPatternRequest {
            prompt: "acid bass variation".to_string(),
            pattern: 0,
            track: 1,
            rows: 16,
            root_pitch: 48,
            velocity: 96,
            guidance: Some(AiGuidanceReference {
                style_path: Some("style.json".to_string()),
                profile_path: Some("profile.json".to_string()),
                dossier_path: None,
                palette_path: Some("palette.json".to_string()),
            }),
        };
        let provider = LocalDeterministicProvider;

        let first = provider.propose(&song, &request).expect("proposal");
        let second = provider.propose(&song, &request).expect("proposal");
        let preview = preview_proposal(&song, &first).expect("preview");

        assert_eq!(first, second);
        assert_eq!(first.source, AiSource::LocalDeterministic);
        assert!(!preview.touched_cells.is_empty());
        assert!(preview
            .touched_cells
            .iter()
            .all(|address| address.pattern == 0 && address.track == 1));
    }

    #[test]
    fn proposing_does_not_mutate_song_until_explicit_apply() {
        let mut song = Song::empty();
        let before = song.clone();
        let request = AiPatternRequest {
            prompt: "minimal lead".to_string(),
            pattern: 0,
            track: 0,
            rows: 8,
            root_pitch: 60,
            velocity: 80,
            guidance: None,
        };
        let proposal = LocalDeterministicProvider
            .propose(&song, &request)
            .expect("proposal");

        assert_eq!(song, before);

        let preview = apply_proposal(&mut song, &proposal).expect("apply");
        assert_ne!(song, before);

        let active_cells = preview
            .touched_cells
            .iter()
            .filter(|address| {
                song.pattern(address.pattern)
                    .and_then(|pattern| pattern.cell(address.row, address.track))
                    .and_then(|cell| cell.note)
                    .is_some()
            })
            .count();
        assert_eq!(active_cells, preview.touched_cells.len());

        song = before.clone();
        assert_eq!(song, before);
    }

    #[test]
    fn proposal_validation_rejects_invalid_or_empty_edits() {
        let song = Song::empty();
        let empty = AiProposal {
            source: AiSource::External {
                provider: "example".to_string(),
            },
            prompt: "noop".to_string(),
            summary: "No changes".to_string(),
            edits: Vec::new(),
        };
        assert!(matches!(
            preview_proposal(&song, &empty),
            Err(AiError::EmptyProposal)
        ));

        let invalid = AiProposal {
            edits: vec![AiEdit::SetNote {
                pattern: 0,
                row: 999,
                track: 0,
                pitch: 60,
                velocity: 100,
            }],
            ..empty
        };
        assert!(matches!(
            preview_proposal(&song, &invalid),
            Err(AiError::MissingRow {
                pattern: 0,
                row: 999
            })
        ));
    }

    #[test]
    fn validates_dossier_and_sanitizes_summary() {
        let dossier = ResearchDossier {
            schema_version: 1,
            title: " Detroit lineage ".to_string(),
            sources: vec!["private notes".to_string()],
            keywords: vec!["electro\u{0000}".to_string(), "machine funk".to_string()],
            observations: vec!["syncopated bass with clipped envelopes".to_string()],
            guardrails: vec!["avoid cloud-only references\n".to_string()],
        };

        let summary = validate_dossier(&dossier).expect("dossier summary");

        assert_eq!(summary.kind, GuidanceKind::Dossier);
        assert_eq!(summary.title, "Detroit lineage");
        assert_eq!(summary.bullet_count, 4);
        assert!(summary.prompt_safe_summary.contains("- electro"));
        assert!(!summary.prompt_safe_summary.contains('\u{0000}'));
        assert!(summary
            .prompt_safe_summary
            .contains("avoid cloud-only references"));
    }

    #[test]
    fn validates_palette_roles_and_guardrails() {
        let palette = OperationalPalette {
            schema_version: 1,
            title: "Live clip palette".to_string(),
            track_roles: vec![PaletteTrackRole {
                role: "bass".to_string(),
                description: "short mono phrases that answer drums".to_string(),
            }],
            sound_sources: vec!["external MIDI synth".to_string()],
            arrangement_functions: vec!["scene intro muting kick".to_string()],
            guardrails: vec!["keep edits reversible".to_string()],
        };

        let summary = validate_palette(&palette).expect("palette summary");

        assert_eq!(summary.kind, GuidanceKind::Palette);
        assert_eq!(summary.title, "Live clip palette");
        assert_eq!(summary.bullet_count, 4);
        assert!(summary
            .prompt_safe_summary
            .contains("bass: short mono phrases that answer drums"));
    }

    #[test]
    fn rejects_invalid_guidance_schema_or_empty_content() {
        let mut dossier = ResearchDossier {
            schema_version: 2,
            title: "Broken".to_string(),
            sources: Vec::new(),
            keywords: vec!["item".to_string()],
            observations: Vec::new(),
            guardrails: Vec::new(),
        };
        assert!(matches!(
            validate_dossier(&dossier),
            Err(AiError::UnsupportedGuidanceSchema(2))
        ));

        dossier.schema_version = 1;
        dossier.title = " \n ".to_string();
        assert!(matches!(
            validate_dossier(&dossier),
            Err(AiError::EmptyGuidanceTitle)
        ));

        dossier.title = "Empty".to_string();
        dossier.keywords.clear();
        assert!(matches!(
            validate_dossier(&dossier),
            Err(AiError::EmptyGuidance)
        ));
    }
}
