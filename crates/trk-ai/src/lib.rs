mod engine;
mod external;

use serde::{Deserialize, Serialize};
use trk_core::{NoteEvent, Song};

pub use engine::{
    discover_engines, discover_engines_with, environment_value, EngineDescriptor,
    EngineDiscoveryInput, EngineId, EngineSelectionError, EngineSelectionState,
    ExternalResponseFormat,
};
pub use external::{parse_external_proposal, ExternalEngineProvider};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AiPatternRequest {
    pub prompt: String,
    pub pattern: usize,
    pub track: usize,
    pub rows: usize,
    pub root_pitch: u8,
    pub velocity: u8,
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
    Mock { model: String },
    External { provider: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockProvider {
    model: String,
}

impl MockProvider {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("pattern {0} does not exist")]
    MissingPattern(usize),
    #[error("row {row} does not exist in pattern {pattern}")]
    MissingRow { pattern: usize, row: usize },
    #[error("track {0} does not exist")]
    MissingTrack(usize),
    #[error("note pitch {0} exceeds MIDI range")]
    InvalidPitch(u8),
    #[error("velocity {0} exceeds MIDI range")]
    InvalidVelocity(u8),
    #[error("prompt cannot be empty")]
    EmptyPrompt,
    #[error("proposal contains no edits")]
    EmptyProposal,
    #[error("AI provider unavailable: {0}")]
    ProviderUnavailable(String),
    #[error("AI provider launch failed: {0}")]
    ProviderLaunch(String),
    #[error("AI provider I/O failed: {0}")]
    ProviderIo(String),
    #[error("AI provider timed out after {0} ms")]
    ProviderTimeout(u64),
    #[error("AI provider task cancelled")]
    ProviderCancelled,
    #[error("AI provider exited unsuccessfully: {0}")]
    ProviderExit(String),
    #[error("AI provider returned an invalid response: {0}")]
    ProviderResponse(String),
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

impl AiProposalProvider for MockProvider {
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
        let row = request.rows.min(pattern.row_count()).saturating_sub(1);
        Ok(AiProposal {
            source: AiSource::Mock {
                model: self.model.clone(),
            },
            prompt: request.prompt.clone(),
            summary: format!("Mock AI provider {} preview", self.model),
            edits: vec![AiEdit::SetNote {
                pattern: request.pattern,
                row,
                track: request.track,
                pitch: request.root_pitch.min(127),
                velocity: request.velocity.min(127),
            }],
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
        if let AiEdit::SetNote {
            pitch, velocity, ..
        } = *edit
        {
            if pitch > 127 {
                return Err(AiError::InvalidPitch(pitch));
            }
            if velocity > 127 {
                return Err(AiError::InvalidVelocity(velocity));
            }
        }
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
    use std::{fmt::Write as _, fs, path::PathBuf};

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
    fn mock_provider_returns_reviewable_local_proposals() {
        let song = Song::empty();
        let request = AiPatternRequest {
            prompt: "mock idea".to_string(),
            pattern: 0,
            track: 0,
            rows: 4,
            root_pitch: 60,
            velocity: 80,
        };

        let proposal = MockProvider::new("fixture-mock")
            .propose(&song, &request)
            .expect("proposal");
        let preview = preview_proposal(&song, &proposal).expect("preview");

        assert_eq!(
            proposal.source,
            AiSource::Mock {
                model: "fixture-mock".to_string()
            }
        );
        assert_eq!(preview.touched_cells.len(), 1);
        assert_eq!(preview.touched_cells[0].row, 3);
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
    fn local_proposal_matches_golden_fixture() {
        let song = Song::empty();
        let request = AiPatternRequest {
            prompt: "fixture bass variation".to_string(),
            pattern: 0,
            track: 1,
            rows: 16,
            root_pitch: 48,
            velocity: 96,
        };
        let proposal = LocalDeterministicProvider
            .propose(&song, &request)
            .expect("proposal");
        let actual = render_proposal(&proposal);
        let path = fixture_path("ai/local-proposal.txt");

        if std::env::var_os("UPDATE_TRK_FIXTURES").is_some() {
            fs::create_dir_all(path.parent().expect("fixture parent")).expect("create fixtures");
            fs::write(&path, &actual).expect("write fixture");
        }

        let expected = fs::read_to_string(&path).expect("read AI fixture");
        assert_eq!(actual, expected, "AI proposal fixture mismatch");
    }

    fn render_proposal(proposal: &AiProposal) -> String {
        let mut output = format!("summary={}\n", proposal.summary);
        for edit in &proposal.edits {
            match edit {
                AiEdit::SetNote {
                    pattern,
                    row,
                    track,
                    pitch,
                    velocity,
                } => writeln!(
                    output,
                    "set-note pattern={pattern} row={row} track={track} pitch={pitch} velocity={velocity}"
                )
                .expect("write proposal fixture"),
                AiEdit::ClearCell {
                    pattern,
                    row,
                    track,
                } => writeln!(
                    output,
                    "clear-cell pattern={pattern} row={row} track={track}"
                )
                .expect("write proposal fixture"),
            }
        }
        output
    }

    fn fixture_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(relative)
    }
}
