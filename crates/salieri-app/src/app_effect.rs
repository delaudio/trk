use std::path::PathBuf;

use salieri_ai::AiPatternRequest;
use salieri_core::Song;

use crate::{
    app_event::{AppEvent, RequestId, RuntimeEvent},
    config::AiConfig,
    persistence::{load_project, save_project},
    App,
};
use salieri_interop::import_smf;

#[derive(Debug, Clone, PartialEq)]
pub enum AppEffect {
    Playback(PlaybackEffect),
    LoadProject {
        request_id: RequestId,
        path: PathBuf,
    },
    ImportMidiProject {
        path: PathBuf,
    },
    SaveProject {
        path: PathBuf,
        song: Song,
        quit_after: bool,
    },
    SubmitAiProposal {
        song: Song,
        request: AiPatternRequest,
        provider: AiConfig,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlaybackEffect {
    StartPattern {
        song: Song,
        sample_base_dir: Option<PathBuf>,
        pattern_index: usize,
        row: usize,
        loop_pattern: bool,
    },
    StartSequence {
        song: Song,
        sample_base_dir: Option<PathBuf>,
        position: usize,
    },
    Stop,
    ConnectMidi(usize),
    DisconnectMidi,
    PanicMidi,
}

pub trait AppEffectExecutor {
    fn execute(&mut self, app: &mut App, effect: AppEffect);
}

pub struct RuntimeEffectExecutor;

impl AppEffectExecutor for RuntimeEffectExecutor {
    fn execute(&mut self, app: &mut App, effect: AppEffect) {
        match effect {
            AppEffect::Playback(effect) => execute_playback_effect(app, effect),
            AppEffect::LoadProject { request_id, path } => {
                let result = load_project(&path).map_err(|error| error.to_string());
                app.dispatch_event(AppEvent::Runtime(RuntimeEvent::ProjectLoaded {
                    request_id,
                    path,
                    result: Box::new(result),
                }));
            }
            AppEffect::ImportMidiProject { path } => {
                let result = std::fs::read(&path)
                    .map_err(|error| {
                        format!("failed to read MIDI import {}: {error}", path.display())
                    })
                    .and_then(|bytes| {
                        import_smf(&bytes).map_err(|error| {
                            format!("MIDI import failed for {}: {error}", path.display())
                        })
                    });
                app.dispatch_event(AppEvent::Runtime(RuntimeEvent::MidiImported {
                    path,
                    result: Box::new(result),
                }));
            }
            AppEffect::SaveProject {
                path,
                song,
                quit_after,
            } => {
                let result = save_project(&path, &song).map_err(|error| error.to_string());
                app.dispatch_event(AppEvent::Runtime(RuntimeEvent::ProjectSaved {
                    path,
                    song: Box::new(song),
                    quit_after,
                    result,
                }));
            }
            AppEffect::SubmitAiProposal {
                song,
                request,
                provider,
            } => {
                app.submit_ai_proposal(song, request, provider);
            }
        }
    }
}

fn execute_playback_effect(app: &mut App, effect: PlaybackEffect) {
    match effect {
        PlaybackEffect::StartPattern {
            song,
            sample_base_dir,
            pattern_index,
            row,
            loop_pattern,
        } => {
            app.playback
                .start_pattern_from(song, sample_base_dir, pattern_index, row, loop_pattern)
        }
        PlaybackEffect::StartSequence {
            song,
            sample_base_dir,
            position,
        } => app.playback.start_sequence(song, sample_base_dir, position),
        PlaybackEffect::Stop => app.playback.stop(),
        PlaybackEffect::ConnectMidi(port_index) => app.playback.connect_midi(port_index),
        PlaybackEffect::DisconnectMidi => app.playback.disconnect_midi(),
        PlaybackEffect::PanicMidi => app.playback.panic_all_notes_off(),
    }
}
