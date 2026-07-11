use std::{
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use salieri_core::{pattern_events, row_duration_micros, PlaybackPosition, Song};
use salieri_midi::{
    send_all_notes_off, send_playback_event, FakeMidiOutput, MidiError, MidiMessage, MidiOutput,
    MidirMidiOutput,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackCursor {
    pub pattern_index: usize,
    pub sequence_index: Option<usize>,
    pub position: PlaybackPosition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackUpdate {
    Position(PlaybackCursor),
    Stopped,
    MidiConnected { port_index: usize },
    MidiDisconnected,
    MidiError(String),
}

#[derive(Debug)]
enum PlaybackCommand {
    StartPattern { song: Song, pattern_index: usize },
    StartSequence { song: Song },
    ConnectMidi { port_index: usize },
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
    pub fn spawn() -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let (update_tx, update_rx) = mpsc::channel();
        let handle = thread::spawn(move || playback_thread(command_rx, update_tx));

        Self {
            command_tx,
            update_rx,
            handle: Some(handle),
        }
    }

    pub fn start_pattern(&self, song: Song, pattern_index: usize) {
        let _ = self.command_tx.send(PlaybackCommand::StartPattern {
            song,
            pattern_index,
        });
    }

    pub fn start_sequence(&self, song: Song) {
        let _ = self
            .command_tx
            .send(PlaybackCommand::StartSequence { song });
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

fn playback_thread(command_rx: Receiver<PlaybackCommand>, update_tx: Sender<PlaybackUpdate>) {
    let mut output = PlaybackOutput::fake();
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
                pattern_index,
            } => {
                next_command = run_pattern(
                    &song,
                    pattern_index,
                    None,
                    true,
                    &command_rx,
                    &update_tx,
                    &mut output,
                )
                .into_command();
                if matches!(next_command, Some(PlaybackCommand::Shutdown)) {
                    break;
                }
            }
            PlaybackCommand::StartSequence { song } => {
                next_command = run_sequence(song, &command_rx, &update_tx, &mut output);
                if matches!(next_command, Some(PlaybackCommand::Shutdown)) {
                    break;
                }
            }
            PlaybackCommand::ConnectMidi { port_index } => {
                let _ = send_all_notes_off(&mut output);
                match MidirMidiOutput::connect(port_index, "salieri-output") {
                    Ok(midir_output) => {
                        output = PlaybackOutput::Midir(midir_output);
                        let _ = update_tx.send(PlaybackUpdate::MidiConnected { port_index });
                    }
                    Err(error) => {
                        let _ = update_tx.send(PlaybackUpdate::MidiError(error.to_string()));
                    }
                }
            }
            PlaybackCommand::DisconnectMidi => {
                let _ = send_all_notes_off(&mut output);
                output = PlaybackOutput::fake();
                let _ = update_tx.send(PlaybackUpdate::MidiDisconnected);
            }
            PlaybackCommand::Panic => {
                let _ = send_all_notes_off(&mut output);
                let _ = update_tx.send(PlaybackUpdate::Stopped);
            }
            PlaybackCommand::Stop => {
                let _ = send_all_notes_off(&mut output);
                let _ = update_tx.send(PlaybackUpdate::Stopped);
            }
            PlaybackCommand::Shutdown => break,
        }
    }

    let _ = send_all_notes_off(&mut output);
    let _ = update_tx.send(PlaybackUpdate::Stopped);
}

#[derive(Debug)]
enum PatternRunResult {
    Finished,
    Stopped,
    Command(PlaybackCommand),
}

impl PatternRunResult {
    fn into_command(self) -> Option<PlaybackCommand> {
        match self {
            Self::Command(command) => Some(command),
            Self::Finished | Self::Stopped => None,
        }
    }
}

enum PlaybackOutput {
    Fake(FakeMidiOutput),
    Midir(MidirMidiOutput),
}

impl PlaybackOutput {
    fn fake() -> Self {
        Self::Fake(FakeMidiOutput::new())
    }
}

impl MidiOutput for PlaybackOutput {
    fn send(&mut self, message: MidiMessage) -> Result<(), MidiError> {
        match self {
            Self::Fake(output) => output.send(message),
            Self::Midir(output) => output.send(message),
        }
    }
}

fn run_pattern(
    song: &Song,
    pattern_index: usize,
    sequence_index: Option<usize>,
    loop_pattern: bool,
    command_rx: &Receiver<PlaybackCommand>,
    update_tx: &Sender<PlaybackUpdate>,
    output: &mut PlaybackOutput,
) -> PatternRunResult {
    let Some(pattern) = song.pattern(pattern_index) else {
        let _ = update_tx.send(PlaybackUpdate::Stopped);
        return PatternRunResult::Stopped;
    };
    let pattern = pattern.clone();
    let row_count = pattern.row_count();
    let row_duration = Duration::from_micros(row_duration_micros(&song.transport).max(1));
    let events = pattern_events(song, &pattern);
    loop {
        let loop_start = Instant::now();

        for row in 0..=row_count {
            let deadline = loop_start + row_duration.saturating_mul(row as u32);
            if let Some(command) = wait_until(command_rx, deadline) {
                let _ = send_all_notes_off(output);
                let _ = update_tx.send(PlaybackUpdate::Stopped);
                return PatternRunResult::Command(command);
            }

            for event in events.iter().filter(|event| event.position.row == row) {
                if let Err(error) = send_playback_event(output, *event) {
                    let _ = update_tx.send(PlaybackUpdate::MidiError(error.to_string()));
                    let _ = send_all_notes_off(output);
                    let _ = update_tx.send(PlaybackUpdate::Stopped);
                    return PatternRunResult::Stopped;
                }
            }

            if row < row_count {
                let _ = update_tx.send(PlaybackUpdate::Position(PlaybackCursor {
                    pattern_index,
                    sequence_index,
                    position: PlaybackPosition {
                        row,
                        offset_micros: row_duration.as_micros().saturating_mul(row as u128) as u64,
                    },
                }));
            }
        }

        if !loop_pattern {
            return PatternRunResult::Finished;
        }
    }
}

fn run_sequence(
    song: Song,
    command_rx: &Receiver<PlaybackCommand>,
    update_tx: &Sender<PlaybackUpdate>,
    output: &mut PlaybackOutput,
) -> Option<PlaybackCommand> {
    for (sequence_index, pattern_id) in song.sequence.iter().enumerate() {
        let Some(pattern_index) = song
            .patterns
            .iter()
            .position(|pattern| pattern.id == *pattern_id)
        else {
            continue;
        };

        match run_pattern(
            &song,
            pattern_index,
            Some(sequence_index),
            false,
            command_rx,
            update_tx,
            output,
        ) {
            PatternRunResult::Finished => {}
            PatternRunResult::Stopped => return None,
            PatternRunResult::Command(command) => return Some(command),
        }
    }

    let _ = send_all_notes_off(output);
    let _ = update_tx.send(PlaybackUpdate::Stopped);
    None
}

fn wait_until(
    command_rx: &Receiver<PlaybackCommand>,
    deadline: Instant,
) -> Option<PlaybackCommand> {
    match command_rx.try_recv() {
        Ok(command) => return Some(command),
        Err(TryRecvError::Empty) => {}
        Err(TryRecvError::Disconnected) => return Some(PlaybackCommand::Shutdown),
    }

    let now = Instant::now();
    if now >= deadline {
        return None;
    }

    match command_rx.recv_timeout(deadline.saturating_duration_since(now)) {
        Ok(command) => Some(command),
        Err(RecvTimeoutError::Timeout) => None,
        Err(RecvTimeoutError::Disconnected) => Some(PlaybackCommand::Shutdown),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use salieri_core::NoteEvent;

    #[test]
    fn runtime_emits_positions_and_stops() {
        let runtime = PlaybackRuntime::spawn();
        let mut song = Song::empty();
        song.transport.bpm = u16::MAX;
        song.transport.lines_per_beat = u8::MAX;
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
            .expect("set note");

        runtime.start_pattern(song, 0);

        let deadline = Instant::now() + Duration::from_millis(250);
        let mut saw_position = false;
        while Instant::now() < deadline {
            if matches!(runtime.try_recv(), Some(PlaybackUpdate::Position(_))) {
                saw_position = true;
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }

        runtime.stop();

        let deadline = Instant::now() + Duration::from_millis(250);
        let mut saw_stop = false;
        while Instant::now() < deadline {
            while let Some(update) = runtime.try_recv() {
                if matches!(update, PlaybackUpdate::Stopped) {
                    saw_stop = true;
                    break;
                }
            }
            if saw_stop {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }

        assert!(saw_position);
        assert!(saw_stop);
    }
}
