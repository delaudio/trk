#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackCursor {
    pub pattern_index: usize,
    pub sequence_index: Option<usize>,
    pub position: PlaybackPosition,
}

use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

use salieri_audio::AudioConfig;
use salieri_core::{PlaybackPosition, Song};
use salieri_midi::MidirMidiOutput;

use super::{
    audio_dispatch::PlaybackAudioOutput,
    logging::{send_all_notes_off_logged, MidiLogger},
    midi_dispatch::PlaybackOutput,
    scheduler::{run_pattern_chain, run_sequence, PlaybackRunContext},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackUpdate {
    Position(PlaybackCursor),
    Stopped,
    MidiConnected { port_index: usize },
    MidiDisconnected,
    MidiError(String),
    MidiLogError(String),
    AudioError(String),
}

#[derive(Debug)]
pub(super) enum PlaybackCommand {
    StartPattern {
        song: Song,
        sample_base_dir: Option<PathBuf>,
        pattern_index: usize,
        start_row: usize,
        loop_pattern: bool,
    },
    StartSequence {
        song: Song,
        sample_base_dir: Option<PathBuf>,
        start_sequence_index: usize,
    },
    ConnectMidi {
        port_index: usize,
    },
    DisconnectMidi,
    Panic,
    Stop,
    Shutdown,
}

pub struct PlaybackRuntime {
    command_tx: Sender<PlaybackCommand>,
    update_rx: Receiver<PlaybackUpdate>,
    handle: Option<JoinHandle<()>>,
}

impl PlaybackRuntime {
    pub fn spawn(midi_log_path: Option<PathBuf>) -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let (update_tx, update_rx) = mpsc::channel();
        let handle = thread::spawn(move || playback_thread(command_rx, update_tx, midi_log_path));

        Self {
            command_tx,
            update_rx,
            handle: Some(handle),
        }
    }

    pub fn start_pattern_from(
        &self,
        song: Song,
        sample_base_dir: Option<PathBuf>,
        pattern_index: usize,
        start_row: usize,
        loop_pattern: bool,
    ) {
        let _ = self.command_tx.send(PlaybackCommand::StartPattern {
            song,
            sample_base_dir,
            pattern_index,
            start_row,
            loop_pattern,
        });
    }

    pub fn start_sequence(
        &self,
        song: Song,
        sample_base_dir: Option<PathBuf>,
        start_sequence_index: usize,
    ) {
        let _ = self.command_tx.send(PlaybackCommand::StartSequence {
            song,
            sample_base_dir,
            start_sequence_index,
        });
    }

    pub fn stop(&self) {
        let _ = self.command_tx.send(PlaybackCommand::Stop);
    }

    pub fn connect_midi(&self, port_index: usize) {
        let _ = self
            .command_tx
            .send(PlaybackCommand::ConnectMidi { port_index });
    }

    pub fn disconnect_midi(&self) {
        let _ = self.command_tx.send(PlaybackCommand::DisconnectMidi);
    }

    pub fn panic_all_notes_off(&self) {
        let _ = self.command_tx.send(PlaybackCommand::Panic);
    }

    pub fn try_recv(&self) -> Option<PlaybackUpdate> {
        self.update_rx.try_recv().ok()
    }
}

impl std::fmt::Debug for PlaybackRuntime {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlaybackRuntime")
            .finish_non_exhaustive()
    }
}

impl Drop for PlaybackRuntime {
    fn drop(&mut self) {
        let _ = self.command_tx.send(PlaybackCommand::Shutdown);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn playback_thread(
    command_rx: Receiver<PlaybackCommand>,
    update_tx: Sender<PlaybackUpdate>,
    midi_log_path: Option<PathBuf>,
) {
    let mut output = PlaybackOutput::fake();
    let mut midi_logger = MidiLogger::new(midi_log_path, &update_tx);
    let mut next_command = None;

    loop {
        let command = match next_command.take() {
            Some(command) => command,
            None => match command_rx.recv() {
                Ok(command) => command,
                Err(_) => break,
            },
        };
        match command {
            PlaybackCommand::StartPattern {
                song,
                sample_base_dir,
                pattern_index,
                start_row,
                loop_pattern,
            } => {
                let mut audio_output = PlaybackAudioOutput::for_song(
                    &song,
                    AudioConfig::default(),
                    &update_tx,
                    sample_base_dir.as_deref(),
                );
                let audio_sample_rate = audio_output.sample_rate();
                let mut context = PlaybackRunContext {
                    command_rx: &command_rx,
                    update_tx: &update_tx,
                    output: &mut output,
                    midi_logger: &mut midi_logger,
                    audio_output: &mut audio_output,
                    audio_sample_rate,
                };
                next_command =
                    run_pattern_chain(&song, pattern_index, start_row, loop_pattern, &mut context);
                if matches!(next_command, Some(PlaybackCommand::Shutdown)) {
                    break;
                }
            }
            PlaybackCommand::StartSequence {
                song,
                sample_base_dir,
                start_sequence_index,
            } => {
                let mut audio_output = PlaybackAudioOutput::for_song(
                    &song,
                    AudioConfig::default(),
                    &update_tx,
                    sample_base_dir.as_deref(),
                );
                let audio_sample_rate = audio_output.sample_rate();
                let mut context = PlaybackRunContext {
                    command_rx: &command_rx,
                    update_tx: &update_tx,
                    output: &mut output,
                    midi_logger: &mut midi_logger,
                    audio_output: &mut audio_output,
                    audio_sample_rate,
                };
                next_command = run_sequence(song, start_sequence_index, &mut context);
                if matches!(next_command, Some(PlaybackCommand::Shutdown)) {
                    break;
                }
            }
            PlaybackCommand::ConnectMidi { port_index } => {
                midi_logger.log_line(format!("CONNECT_REQUEST port={port_index}"));
                let _ = send_all_notes_off_logged(&mut output, &mut midi_logger);
                match MidirMidiOutput::connect(port_index, "salieri-output") {
                    Ok(midir_output) => {
                        output = PlaybackOutput::Midir(midir_output);
                        midi_logger.log_line(format!("CONNECTED port={port_index}"));
                        let _ = update_tx.send(PlaybackUpdate::MidiConnected { port_index });
                    }
                    Err(error) => {
                        midi_logger.log_line(format!("CONNECT_ERROR port={port_index} {error}"));
                        output = PlaybackOutput::fake();
                        let _ = update_tx.send(PlaybackUpdate::MidiDisconnected);
                        let _ = update_tx.send(PlaybackUpdate::MidiError(format!(
                            "MIDI output connect failed: {error}"
                        )));
                    }
                }
            }
            PlaybackCommand::DisconnectMidi => {
                midi_logger.log_line("DISCONNECT");
                let _ = send_all_notes_off_logged(&mut output, &mut midi_logger);
                output = PlaybackOutput::fake();
                let _ = update_tx.send(PlaybackUpdate::MidiDisconnected);
            }
            PlaybackCommand::Panic => {
                midi_logger.log_line("PANIC");
                let _ = send_all_notes_off_logged(&mut output, &mut midi_logger);
                let _ = update_tx.send(PlaybackUpdate::Stopped);
            }
            PlaybackCommand::Stop => {
                midi_logger.log_line("STOP");
                let _ = send_all_notes_off_logged(&mut output, &mut midi_logger);
                let _ = update_tx.send(PlaybackUpdate::Stopped);
            }
            PlaybackCommand::Shutdown => break,
        }
    }

    midi_logger.log_line("SHUTDOWN");
    let _ = send_all_notes_off_logged(&mut output, &mut midi_logger);
    let _ = update_tx.send(PlaybackUpdate::Stopped);
}
