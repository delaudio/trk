use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use salieri_core::{pattern_events, row_duration_micros, PlaybackPosition, Song};
use salieri_midi::{
    playback_event_to_midi, FakeMidiOutput, MidiError, MidiMessage, MidiOutput, MidirMidiOutput,
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
    MidiLogError(String),
}

#[derive(Debug)]
enum PlaybackCommand {
    StartPattern {
        song: Song,
        pattern_index: usize,
        start_row: usize,
        loop_pattern: bool,
    },
    StartSequence {
        song: Song,
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
        pattern_index: usize,
        start_row: usize,
        loop_pattern: bool,
    ) {
        let _ = self.command_tx.send(PlaybackCommand::StartPattern {
            song,
            pattern_index,
            start_row,
            loop_pattern,
        });
    }

    pub fn start_sequence(&self, song: Song, start_sequence_index: usize) {
        let _ = self.command_tx.send(PlaybackCommand::StartSequence {
            song,
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
                pattern_index,
                start_row,
                loop_pattern,
            } => {
                let mut context = PlaybackRunContext {
                    command_rx: &command_rx,
                    update_tx: &update_tx,
                    output: &mut output,
                    midi_logger: &mut midi_logger,
                };
                let result = run_pattern(
                    &song,
                    pattern_index,
                    start_row,
                    None,
                    loop_pattern,
                    &mut context,
                );
                if matches!(result, PatternRunResult::Finished) {
                    let _ = send_all_notes_off_logged(&mut output, &mut midi_logger);
                    let _ = update_tx.send(PlaybackUpdate::Stopped);
                }
                next_command = result.into_command();
                if matches!(next_command, Some(PlaybackCommand::Shutdown)) {
                    break;
                }
            }
            PlaybackCommand::StartSequence {
                song,
                start_sequence_index,
            } => {
                let mut context = PlaybackRunContext {
                    command_rx: &command_rx,
                    update_tx: &update_tx,
                    output: &mut output,
                    midi_logger: &mut midi_logger,
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
                        let _ = update_tx.send(PlaybackUpdate::MidiError(error.to_string()));
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

struct PlaybackRunContext<'a> {
    command_rx: &'a Receiver<PlaybackCommand>,
    update_tx: &'a Sender<PlaybackUpdate>,
    output: &'a mut PlaybackOutput,
    midi_logger: &'a mut MidiLogger,
}

fn run_pattern(
    song: &Song,
    pattern_index: usize,
    start_row: usize,
    sequence_index: Option<usize>,
    loop_pattern: bool,
    context: &mut PlaybackRunContext<'_>,
) -> PatternRunResult {
    let Some(pattern) = song.pattern(pattern_index) else {
        let _ = context.update_tx.send(PlaybackUpdate::Stopped);
        return PatternRunResult::Stopped;
    };
    let pattern = pattern.clone();
    let row_count = pattern.row_count();
    let row_duration = Duration::from_micros(row_duration_micros(&song.transport).max(1));
    let events = pattern_events(song, &pattern);
    let mut pass_start_row = start_row.min(row_count);
    loop {
        let loop_start = Instant::now();
        let mut active_sent_notes = Vec::new();

        for row in pass_start_row..=row_count {
            let relative_row = row.saturating_sub(pass_start_row);
            let deadline = loop_start + row_duration.saturating_mul(relative_row as u32);
            if let Some(command) = wait_until(context.command_rx, deadline) {
                let _ = send_all_notes_off_logged(context.output, context.midi_logger);
                let _ = context.update_tx.send(PlaybackUpdate::Stopped);
                return PatternRunResult::Command(command);
            }

            for event in events.iter().filter(|event| event.position.row == row) {
                if !mark_event_for_started_playback(&mut active_sent_notes, event) {
                    continue;
                }
                if let Err(error) =
                    send_playback_event_logged(context.output, context.midi_logger, *event)
                {
                    let _ = context
                        .update_tx
                        .send(PlaybackUpdate::MidiError(error.to_string()));
                    let _ = send_all_notes_off_logged(context.output, context.midi_logger);
                    let _ = context.update_tx.send(PlaybackUpdate::Stopped);
                    return PatternRunResult::Stopped;
                }
            }

            if row < row_count {
                let _ = context
                    .update_tx
                    .send(PlaybackUpdate::Position(PlaybackCursor {
                        pattern_index,
                        sequence_index,
                        position: PlaybackPosition {
                            row,
                            offset_micros: row_duration.as_micros().saturating_mul(row as u128)
                                as u64,
                        },
                    }));
            }
        }

        if !loop_pattern {
            return PatternRunResult::Finished;
        }
        pass_start_row = 0;
    }
}

fn run_sequence(
    song: Song,
    start_sequence_index: usize,
    context: &mut PlaybackRunContext<'_>,
) -> Option<PlaybackCommand> {
    for (sequence_index, pattern_id) in song.sequence.iter().enumerate().skip(start_sequence_index)
    {
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
            0,
            Some(sequence_index),
            false,
            context,
        ) {
            PatternRunResult::Finished => {}
            PatternRunResult::Stopped => return None,
            PatternRunResult::Command(command) => return Some(command),
        }
    }

    let _ = send_all_notes_off_logged(context.output, context.midi_logger);
    let _ = context.update_tx.send(PlaybackUpdate::Stopped);
    None
}

fn mark_event_for_started_playback(
    active_notes: &mut Vec<(salieri_core::TrackId, u8)>,
    event: &salieri_core::PlaybackEvent,
) -> bool {
    match event.kind {
        salieri_core::PlaybackEventKind::NoteOn { pitch, .. } => {
            active_notes.retain(|(track, _)| *track != event.track);
            active_notes.push((event.track, pitch));
            true
        }
        salieri_core::PlaybackEventKind::NoteOff { pitch } => {
            let was_active = active_notes
                .iter()
                .any(|(track, active_pitch)| *track == event.track && *active_pitch == pitch);
            if was_active {
                active_notes.retain(|(track, active_pitch)| {
                    *track != event.track || *active_pitch != pitch
                });
            }
            was_active
        }
    }
}

struct MidiLogger {
    start: Instant,
    file: Option<File>,
}

impl MidiLogger {
    fn new(path: Option<PathBuf>, update_tx: &Sender<PlaybackUpdate>) -> Self {
        let file = path.and_then(|path| match open_log_file(&path) {
            Ok(mut file) => {
                let _ = writeln!(file, "# Salieri MIDI log");
                Some(file)
            }
            Err(error) => {
                let _ = update_tx.send(PlaybackUpdate::MidiLogError(format!(
                    "{}: {error}",
                    path.display()
                )));
                None
            }
        });

        Self {
            start: Instant::now(),
            file,
        }
    }

    fn log_line(&mut self, line: impl AsRef<str>) {
        let Some(file) = &mut self.file else {
            return;
        };
        let elapsed = self.start.elapsed().as_millis();
        let _ = writeln!(file, "{elapsed:>8}ms {}", line.as_ref());
        let _ = file.flush();
    }
}

fn open_log_file(path: &PathBuf) -> std::io::Result<File> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    OpenOptions::new().create(true).append(true).open(path)
}

fn send_playback_event_logged(
    output: &mut PlaybackOutput,
    midi_logger: &mut MidiLogger,
    event: salieri_core::PlaybackEvent,
) -> Result<(), MidiError> {
    let message = playback_event_to_midi(event);
    send_midi_message_logged(output, midi_logger, message)
}

fn send_all_notes_off_logged(
    output: &mut PlaybackOutput,
    midi_logger: &mut MidiLogger,
) -> Result<(), MidiError> {
    for channel in 1..=16 {
        send_midi_message_logged(output, midi_logger, MidiMessage::all_notes_off(channel))?;
    }
    Ok(())
}

fn send_midi_message_logged(
    output: &mut PlaybackOutput,
    midi_logger: &mut MidiLogger,
    message: MidiMessage,
) -> Result<(), MidiError> {
    midi_logger.log_line(format_midi_message(message));
    output.send(message)
}

fn format_midi_message(message: MidiMessage) -> String {
    let bytes = message.to_bytes();
    match message {
        MidiMessage::NoteOn {
            channel,
            note,
            velocity,
        } => format!(
            "NOTE_ON ch={channel} note={note} velocity={velocity} bytes={:02X} {:02X} {:02X}",
            bytes[0], bytes[1], bytes[2]
        ),
        MidiMessage::NoteOff {
            channel,
            note,
            velocity,
        } => format!(
            "NOTE_OFF ch={channel} note={note} velocity={velocity} bytes={:02X} {:02X} {:02X}",
            bytes[0], bytes[1], bytes[2]
        ),
        MidiMessage::ControlChange {
            channel,
            controller,
            value,
        } => format!(
            "CC ch={channel} controller={controller} value={value} bytes={:02X} {:02X} {:02X}",
            bytes[0], bytes[1], bytes[2]
        ),
    }
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
        let runtime = PlaybackRuntime::spawn(None);
        let mut song = Song::empty();
        song.transport.bpm = u16::MAX;
        song.transport.lines_per_beat = u8::MAX;
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
            .expect("set note");

        runtime.start_pattern_from(song, 0, 0, true);

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

    #[test]
    fn runtime_starts_pattern_from_requested_row() {
        let runtime = PlaybackRuntime::spawn(None);
        let mut song = Song::empty();
        song.transport.bpm = u16::MAX;
        song.transport.lines_per_beat = u8::MAX;

        runtime.start_pattern_from(song, 0, 4, true);

        let deadline = Instant::now() + Duration::from_millis(250);
        let mut first_position = None;
        while Instant::now() < deadline {
            if let Some(PlaybackUpdate::Position(position)) = runtime.try_recv() {
                first_position = Some(position.position.row);
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }

        runtime.stop();

        assert_eq!(first_position, Some(4));
    }

    #[test]
    fn runtime_stops_when_pattern_loop_is_disabled() {
        let runtime = PlaybackRuntime::spawn(None);
        let mut song = Song::empty();
        song.transport.bpm = u16::MAX;
        song.transport.lines_per_beat = u8::MAX;

        runtime.start_pattern_from(song, 0, 0, false);

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

        assert!(saw_stop);
    }

    #[test]
    fn runtime_starts_sequence_from_requested_position() {
        let runtime = PlaybackRuntime::spawn(None);
        let mut song = Song::empty();
        song.transport.bpm = u16::MAX;
        song.transport.lines_per_beat = u8::MAX;
        let second_pattern_id = song.create_pattern(64);
        song.push_sequence_pattern(second_pattern_id)
            .expect("add second pattern to sequence");

        runtime.start_sequence(song, 1);

        let deadline = Instant::now() + Duration::from_millis(250);
        let mut first_sequence_index = None;
        while Instant::now() < deadline {
            if let Some(PlaybackUpdate::Position(position)) = runtime.try_recv() {
                first_sequence_index = position.sequence_index;
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }

        runtime.stop();

        assert_eq!(first_sequence_index, Some(1));
    }

    #[test]
    fn runtime_writes_midi_log_when_enabled() {
        let path =
            std::env::temp_dir().join(format!("salieri-midi-log-{}.log", std::process::id()));
        let runtime = PlaybackRuntime::spawn(Some(path.clone()));
        let mut song = Song::empty();
        song.transport.bpm = u16::MAX;
        song.transport.lines_per_beat = u8::MAX;
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
            .expect("set note");

        runtime.start_pattern_from(song, 0, 0, true);

        let deadline = Instant::now() + Duration::from_millis(250);
        while Instant::now() < deadline {
            if std::fs::read_to_string(&path).is_ok_and(|contents| contents.contains("NOTE_ON")) {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        runtime.stop();
        drop(runtime);

        let contents = std::fs::read_to_string(&path).expect("midi log");
        let _ = std::fs::remove_file(&path);

        assert!(contents.contains("NOTE_ON ch=10 note=60 velocity=127"));
    }
}
