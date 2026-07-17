use std::{
    collections::HashSet,
    fs::{File, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use salieri_audio::{
    apply_preview_envelope, prepare_realtime_sample, slice_preview_buffer, AudioConfig,
    CpalAudioBackend, RealtimeAudioCommand,
};
use salieri_core::{
    pattern_events, row_duration_micros, sampler_events, PlaybackPosition, SamplePlaybackSettings,
    Song,
};
use salieri_midi::{
    playback_event_to_midi, FakeMidiOutput, MidiError, MidiMessage, MidiOutput, MidirMidiOutput,
};
use salieri_sampler::{PreviewBuffer, PreviewSettings, Sample};

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
    AudioError(String),
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
                let mut audio_output =
                    PlaybackAudioOutput::for_song(&song, AudioConfig::default(), &update_tx);
                let audio_sample_rate = audio_output.sample_rate();
                let mut context = PlaybackRunContext {
                    command_rx: &command_rx,
                    update_tx: &update_tx,
                    output: &mut output,
                    midi_logger: &mut midi_logger,
                    audio_output: &mut audio_output,
                    audio_sample_rate,
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
                    send_all_audio_notes_off(&mut audio_output);
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
                let mut audio_output =
                    PlaybackAudioOutput::for_song(&song, AudioConfig::default(), &update_tx);
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

#[derive(Debug)]
enum PatternRunResult {
    Finished,
    Stopped,
    Command(Box<PlaybackCommand>),
}

impl PatternRunResult {
    fn into_command(self) -> Option<PlaybackCommand> {
        match self {
            Self::Command(command) => Some(*command),
            Self::Finished | Self::Stopped => None,
        }
    }
}

enum PlaybackOutput {
    Fake(FakeMidiOutput),
    Midir(MidirMidiOutput),
    #[cfg(test)]
    Recording(RecordingMidiOutput),
    #[cfg(test)]
    Failing(FailingMidiOutput),
}

impl PlaybackOutput {
    fn fake() -> Self {
        Self::Fake(FakeMidiOutput::new())
    }

    #[cfg(test)]
    fn failing() -> Self {
        Self::Failing(FailingMidiOutput)
    }

    #[cfg(test)]
    fn recording(messages: std::sync::Arc<std::sync::Mutex<Vec<MidiMessage>>>) -> Self {
        Self::Recording(RecordingMidiOutput { messages })
    }
}

impl MidiOutput for PlaybackOutput {
    fn send(&mut self, message: MidiMessage) -> Result<(), MidiError> {
        match self {
            Self::Fake(output) => output.send(message),
            Self::Midir(output) => output.send(message),
            #[cfg(test)]
            Self::Recording(output) => output.send(message),
            #[cfg(test)]
            Self::Failing(output) => output.send(message),
        }
    }
}

enum PlaybackAudioOutput {
    Disabled {
        sample_rate: u32,
    },
    Cpal {
        backend: CpalAudioBackend,
        sample_rate: u32,
    },
    #[cfg(test)]
    Recording {
        command_tx: Sender<RealtimeAudioCommand>,
        sample_rate: u32,
    },
}

impl PlaybackAudioOutput {
    fn disabled(sample_rate: u32) -> Self {
        Self::Disabled { sample_rate }
    }

    fn for_song(song: &Song, config: AudioConfig, update_tx: &Sender<PlaybackUpdate>) -> Self {
        let samples = load_realtime_samples(song, config, update_tx);
        if samples.is_empty() {
            return Self::disabled(config.sample_rate);
        }

        let mut backend = CpalAudioBackend::new();
        if let Err(error) = salieri_audio::AudioBackend::start(&mut backend, config) {
            let _ = update_tx.send(PlaybackUpdate::AudioError(error.to_string()));
            return Self::disabled(config.sample_rate);
        }

        for (sample_id, buffer) in samples {
            if let Err(error) = backend.register_sample(sample_id, buffer) {
                let _ = update_tx.send(PlaybackUpdate::AudioError(error.to_string()));
            }
        }

        Self::Cpal {
            backend,
            sample_rate: config.sample_rate,
        }
    }

    #[cfg(test)]
    fn recording(command_tx: Sender<RealtimeAudioCommand>, sample_rate: u32) -> Self {
        Self::Recording {
            command_tx,
            sample_rate,
        }
    }

    fn sample_rate(&self) -> u32 {
        match self {
            Self::Disabled { sample_rate } | Self::Cpal { sample_rate, .. } => *sample_rate,
            #[cfg(test)]
            Self::Recording { sample_rate, .. } => *sample_rate,
        }
    }

    fn send(&mut self, command: RealtimeAudioCommand) {
        match self {
            Self::Disabled { .. } => {}
            Self::Cpal { backend, .. } => {
                let _ = backend.send_realtime_command(command);
            }
            #[cfg(test)]
            Self::Recording { command_tx, .. } => {
                let _ = command_tx.send(command);
            }
        }
    }
}

fn load_realtime_samples(
    song: &Song,
    config: AudioConfig,
    update_tx: &Sender<PlaybackUpdate>,
) -> Vec<(u32, salieri_sampler::PreviewBuffer)> {
    let assigned_samples = song
        .sample_assignments
        .iter()
        .map(|assignment| assignment.sample)
        .chain(
            song.track_instrument_assignments
                .iter()
                .filter_map(|assignment| {
                    song.instrument_for_id(assignment.instrument)
                        .and_then(|instrument| instrument.sample)
                }),
        )
        .collect::<HashSet<_>>();
    if assigned_samples.is_empty() {
        return Vec::new();
    }

    song.samples
        .iter()
        .filter(|sample| assigned_samples.contains(&sample.id))
        .filter_map(|reference| match Sample::load_wav(&reference.path) {
            Ok(sample) => {
                let preview = apply_sample_playback_settings(
                    &sample.preview(PreviewSettings::default()),
                    reference.playback,
                );
                Some((
                    reference.id.0,
                    prepare_realtime_sample(&preview, config.sample_rate, config.channels),
                ))
            }
            Err(error) => {
                let _ = update_tx.send(PlaybackUpdate::AudioError(format!(
                    "Sample audio load failed for {}: {error}",
                    reference.path
                )));
                None
            }
        })
        .collect()
}

pub(crate) fn apply_sample_playback_settings(
    preview: &PreviewBuffer,
    settings: SamplePlaybackSettings,
) -> PreviewBuffer {
    let sliced = slice_preview_buffer(preview, settings.start_frame, settings.end_frame);
    let sample_rate = sliced.sample_rate as f32;
    let envelope = settings.envelope;
    apply_preview_envelope(
        &sliced,
        seconds_to_frames(envelope.attack_seconds, sample_rate),
        seconds_to_frames(envelope.decay_seconds, sample_rate),
        envelope.sustain,
        seconds_to_frames(envelope.release_seconds, sample_rate),
    )
}

fn seconds_to_frames(seconds: f32, sample_rate: f32) -> usize {
    if !seconds.is_finite() || seconds <= 0.0 || !sample_rate.is_finite() || sample_rate <= 0.0 {
        0
    } else {
        (seconds * sample_rate).round() as usize
    }
}

#[cfg(test)]
struct RecordingMidiOutput {
    messages: std::sync::Arc<std::sync::Mutex<Vec<MidiMessage>>>,
}

#[cfg(test)]
impl MidiOutput for RecordingMidiOutput {
    fn send(&mut self, message: MidiMessage) -> Result<(), MidiError> {
        self.messages
            .lock()
            .expect("recorded MIDI messages")
            .push(message);
        Ok(())
    }
}

#[cfg(test)]
struct FailingMidiOutput;

#[cfg(test)]
impl MidiOutput for FailingMidiOutput {
    fn send(&mut self, _message: MidiMessage) -> Result<(), MidiError> {
        Err(MidiError::Send("simulated disconnected MIDI port".into()))
    }
}

struct PlaybackRunContext<'a> {
    command_rx: &'a Receiver<PlaybackCommand>,
    update_tx: &'a Sender<PlaybackUpdate>,
    output: &'a mut PlaybackOutput,
    midi_logger: &'a mut MidiLogger,
    audio_output: &'a mut PlaybackAudioOutput,
    audio_sample_rate: u32,
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
    let row_duration_micros = row_duration_micros(&song.transport).max(1);
    let row_duration = Duration::from_micros(row_duration_micros);
    let events = pattern_events(song, &pattern);
    let sample_events = sampler_events(song, &pattern);
    let mut pass_start_row = start_row.min(row_count);
    loop {
        let loop_start = Instant::now();
        let pass_start_offset = row_duration_micros.saturating_mul(pass_start_row as u64);
        let mut active_sent_notes = Vec::new();

        for row in pass_start_row..=row_count {
            let relative_row = row.saturating_sub(pass_start_row);
            let deadline = loop_start + row_duration.saturating_mul(relative_row as u32);
            if let Some(command) = wait_until(context.command_rx, deadline) {
                let _ = send_all_notes_off_logged(context.output, context.midi_logger);
                send_all_audio_notes_off(context.audio_output);
                let _ = context.update_tx.send(PlaybackUpdate::Stopped);
                return PatternRunResult::Command(Box::new(command));
            }

            for event in events.iter().filter(|event| event.position.row == row) {
                let event_deadline = loop_start
                    + Duration::from_micros(
                        event
                            .position
                            .offset_micros
                            .saturating_sub(pass_start_offset),
                    );
                if let Some(command) = wait_until(context.command_rx, event_deadline) {
                    let _ = send_all_notes_off_logged(context.output, context.midi_logger);
                    send_all_audio_notes_off(context.audio_output);
                    let _ = context.update_tx.send(PlaybackUpdate::Stopped);
                    return PatternRunResult::Command(Box::new(command));
                }
                if !mark_event_for_started_playback(&mut active_sent_notes, event) {
                    continue;
                }
                if let Err(error) =
                    send_playback_event_logged(context.output, context.midi_logger, *event)
                {
                    handle_midi_send_failure(context, error);
                    return PatternRunResult::Stopped;
                }
            }

            for event in sample_events
                .iter()
                .filter(|event| event.position.row == row)
            {
                let relative_offset = event
                    .position
                    .offset_micros
                    .saturating_sub(pass_start_offset);
                let event_deadline = loop_start + Duration::from_micros(relative_offset);
                if let Some(command) = wait_until(context.command_rx, event_deadline) {
                    let _ = send_all_notes_off_logged(context.output, context.midi_logger);
                    send_all_audio_notes_off(context.audio_output);
                    let _ = context.update_tx.send(PlaybackUpdate::Stopped);
                    return PatternRunResult::Command(Box::new(command));
                }
                let command = RealtimeAudioCommand::TriggerSample {
                    sample_id: event.sample.0,
                    frame: micros_to_frames(relative_offset, context.audio_sample_rate),
                    gain: event.gain * (f32::from(event.velocity.min(0x7f)) / 127.0),
                    pitch_ratio: event.pitch_ratio,
                };
                send_audio_command(context.audio_output, command);
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
            PatternRunResult::Command(command) => return Some(*command),
        }
    }

    let _ = send_all_notes_off_logged(context.output, context.midi_logger);
    send_all_audio_notes_off(context.audio_output);
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

fn handle_midi_send_failure(context: &mut PlaybackRunContext<'_>, error: MidiError) {
    context
        .midi_logger
        .log_line(format!("SEND_ERROR stopping playback: {error}"));
    if let Err(recovery_error) = send_all_notes_off_logged(context.output, context.midi_logger) {
        context.midi_logger.log_line(format!(
            "ALL_NOTES_OFF_ERROR during MIDI recovery: {recovery_error}"
        ));
    }
    *context.output = PlaybackOutput::fake();
    let _ = context.update_tx.send(PlaybackUpdate::MidiDisconnected);
    let _ = context.update_tx.send(PlaybackUpdate::Stopped);
    let _ = context.update_tx.send(PlaybackUpdate::MidiError(format!(
        "MIDI output disconnected during playback: {error}"
    )));
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

fn send_all_audio_notes_off(audio_output: &mut PlaybackAudioOutput) {
    send_audio_command(audio_output, RealtimeAudioCommand::AllNotesOff { frame: 0 })
}

fn send_audio_command(audio_output: &mut PlaybackAudioOutput, command: RealtimeAudioCommand) {
    audio_output.send(command);
}

fn micros_to_frames(offset_micros: u64, sample_rate: u32) -> u64 {
    u64::from(sample_rate).saturating_mul(offset_micros) / 1_000_000
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

    fn collect_position_times(
        runtime: &PlaybackRuntime,
        count: usize,
        timeout: Duration,
    ) -> Vec<(usize, Instant)> {
        let deadline = Instant::now() + timeout;
        let mut positions = Vec::new();
        while Instant::now() < deadline && positions.len() < count {
            while let Some(update) = runtime.try_recv() {
                if let PlaybackUpdate::Position(position) = update {
                    positions.push((position.position.row, Instant::now()));
                }
            }
            thread::sleep(Duration::from_millis(1));
        }
        positions
    }

    fn speed_up_transport(song: &mut Song) {
        song.transport.bpm = u16::MAX;
        song.transport.lines_per_beat = u8::MAX;
    }

    fn write_test_wav(path: &std::path::Path, sample_rate: u32, channels: u16, samples: &[i16]) {
        let data_bytes = samples.len() * 2;
        let mut bytes = Vec::with_capacity(44 + data_bytes);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_bytes as u32).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&(sample_rate * u32::from(channels) * 2).to_le_bytes());
        bytes.extend_from_slice(&(channels * 2).to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&(data_bytes as u32).to_le_bytes());
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        std::fs::write(path, bytes).expect("write wav");
    }

    fn run_pattern_with_recording(
        song: &Song,
        pattern_index: usize,
        start_row: usize,
        loop_pattern: bool,
        command_rx: &Receiver<PlaybackCommand>,
    ) -> (PatternRunResult, Vec<MidiMessage>, Vec<PlaybackUpdate>) {
        let messages = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let (update_tx, update_rx) = mpsc::channel();
        let mut output = PlaybackOutput::recording(messages.clone());
        let mut midi_logger = MidiLogger::new(None, &update_tx);
        let mut audio_output = PlaybackAudioOutput::disabled(AudioConfig::default().sample_rate);
        let audio_sample_rate = audio_output.sample_rate();
        let mut context = PlaybackRunContext {
            command_rx,
            update_tx: &update_tx,
            output: &mut output,
            midi_logger: &mut midi_logger,
            audio_output: &mut audio_output,
            audio_sample_rate,
        };

        let result = run_pattern(
            song,
            pattern_index,
            start_row,
            None,
            loop_pattern,
            &mut context,
        );
        let sent = messages.lock().expect("recorded MIDI messages").clone();
        let updates = update_rx.try_iter().collect();
        (result, sent, updates)
    }

    fn run_pattern_with_audio_recording(
        song: &Song,
        pattern_index: usize,
        audio_sample_rate: u32,
    ) -> (PatternRunResult, Vec<RealtimeAudioCommand>) {
        let messages = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let (_command_tx, command_rx) = mpsc::channel();
        let (update_tx, _update_rx) = mpsc::channel();
        let (audio_tx, audio_rx) = mpsc::channel();
        let mut output = PlaybackOutput::recording(messages);
        let mut midi_logger = MidiLogger::new(None, &update_tx);
        let mut audio_output = PlaybackAudioOutput::recording(audio_tx, audio_sample_rate);
        let mut context = PlaybackRunContext {
            command_rx: &command_rx,
            update_tx: &update_tx,
            output: &mut output,
            midi_logger: &mut midi_logger,
            audio_output: &mut audio_output,
            audio_sample_rate,
        };

        let result = run_pattern(song, pattern_index, 0, None, false, &mut context);
        (result, audio_rx.try_iter().collect())
    }

    fn run_sequence_with_recording(
        song: Song,
        start_sequence_index: usize,
    ) -> (
        Option<PlaybackCommand>,
        Vec<MidiMessage>,
        Vec<PlaybackUpdate>,
    ) {
        let messages = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let (_command_tx, command_rx) = mpsc::channel();
        let (update_tx, update_rx) = mpsc::channel();
        let mut output = PlaybackOutput::recording(messages.clone());
        let mut midi_logger = MidiLogger::new(None, &update_tx);
        let mut audio_output = PlaybackAudioOutput::disabled(AudioConfig::default().sample_rate);
        let audio_sample_rate = audio_output.sample_rate();
        let mut context = PlaybackRunContext {
            command_rx: &command_rx,
            update_tx: &update_tx,
            output: &mut output,
            midi_logger: &mut midi_logger,
            audio_output: &mut audio_output,
            audio_sample_rate,
        };

        let next_command = run_sequence(song, start_sequence_index, &mut context);
        let sent = messages.lock().expect("recorded MIDI messages").clone();
        let updates = update_rx.try_iter().collect();
        (next_command, sent, updates)
    }

    #[test]
    fn runtime_emits_positions_and_stops() {
        let runtime = PlaybackRuntime::spawn(None);
        let mut song = Song::empty();
        speed_up_transport(&mut song);
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
        speed_up_transport(&mut song);

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
        speed_up_transport(&mut song);

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
    fn pattern_playback_routes_assigned_samples_to_audio_commands() {
        let mut song = Song::empty();
        speed_up_transport(&mut song);
        let track = song.tracks[0].id;
        let sample = song.upsert_sample_reference("samples/kick.wav", "kick.wav");
        song.samples[0].gain = 0.5;
        song.assign_sample_to_track(track, sample)
            .expect("assign sample");
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(1, 0, NoteEvent::Note { pitch: 72 }, 64)
            .expect("set note");

        let (_result, commands) = run_pattern_with_audio_recording(&song, 0, 1_000_000);
        let trigger = commands
            .iter()
            .find_map(|command| match command {
                RealtimeAudioCommand::TriggerSample {
                    sample_id,
                    frame,
                    gain,
                    pitch_ratio,
                } => Some((*sample_id, *frame, *gain, *pitch_ratio)),
                RealtimeAudioCommand::StopVoice { .. }
                | RealtimeAudioCommand::AllNotesOff { .. } => None,
            })
            .expect("trigger sample command");

        assert_eq!(trigger.0, sample.0);
        assert_eq!(
            trigger.1,
            micros_to_frames(row_duration_micros(&song.transport), 1_000_000)
        );
        assert_approx_eq(trigger.2, 0.5 * (64.0 / 127.0));
        assert_approx_eq(trigger.3, 2.0);
    }

    #[test]
    fn realtime_sample_loader_prepares_assigned_wavs() {
        let path = std::env::temp_dir().join(format!(
            "salieri-realtime-sample-{}.wav",
            std::process::id()
        ));
        write_test_wav(&path, 44_100, 1, &[0, i16::MAX, i16::MIN, 16_384]);
        let mut song = Song::empty();
        let track = song.tracks[0].id;
        let sample = song.upsert_sample_reference(path.to_string_lossy(), "kick.wav");
        song.assign_sample_to_track(track, sample)
            .expect("assign sample");
        let (update_tx, update_rx) = mpsc::channel();

        let samples = load_realtime_samples(
            &song,
            AudioConfig {
                sample_rate: 48_000,
                channels: 2,
                buffer_frames: 256,
            },
            &update_tx,
        );
        let _ = std::fs::remove_file(&path);

        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].0, sample.0);
        assert_eq!(samples[0].1.sample_rate, 48_000);
        assert_eq!(samples[0].1.channels, 2);
        assert!(samples[0].1.frames >= 4);
        assert!(update_rx.try_iter().collect::<Vec<_>>().is_empty());
    }

    #[test]
    fn interrupted_pattern_playback_sends_audio_all_notes_off() {
        let mut song = Song::empty();
        speed_up_transport(&mut song);
        let (command_tx, command_rx) = mpsc::channel();
        let (update_tx, _update_rx) = mpsc::channel();
        let (audio_tx, audio_rx) = mpsc::channel();
        let messages = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut output = PlaybackOutput::recording(messages);
        let mut midi_logger = MidiLogger::new(None, &update_tx);
        let mut audio_output =
            PlaybackAudioOutput::recording(audio_tx, AudioConfig::default().sample_rate);
        let audio_sample_rate = audio_output.sample_rate();
        let mut context = PlaybackRunContext {
            command_rx: &command_rx,
            update_tx: &update_tx,
            output: &mut output,
            midi_logger: &mut midi_logger,
            audio_output: &mut audio_output,
            audio_sample_rate,
        };
        command_tx.send(PlaybackCommand::Stop).expect("send stop");

        let result = run_pattern(&song, 0, 0, None, true, &mut context);
        let commands = audio_rx.try_iter().collect::<Vec<_>>();

        assert!(matches!(result, PatternRunResult::Command(_)));
        assert!(commands
            .iter()
            .any(|command| matches!(command, RealtimeAudioCommand::AllNotesOff { frame: 0 })));
    }

    #[test]
    fn runtime_starts_sequence_from_requested_position() {
        let runtime = PlaybackRuntime::spawn(None);
        let mut song = Song::empty();
        speed_up_transport(&mut song);
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
    fn runtime_position_intervals_track_row_duration_with_tolerance() {
        let runtime = PlaybackRuntime::spawn(None);
        let mut song = Song::empty();
        song.transport.bpm = 300;
        song.transport.lines_per_beat = 4;
        let expected = Duration::from_micros(row_duration_micros(&song.transport));

        runtime.start_pattern_from(song, 0, 0, true);

        let positions = collect_position_times(&runtime, 6, Duration::from_millis(500));
        runtime.stop();

        let intervals: Vec<_> = positions
            .windows(2)
            .filter_map(|pair| {
                let (previous_row, previous_time) = pair[0];
                let (next_row, next_time) = pair[1];
                (next_row == previous_row + 1).then_some(next_time.duration_since(previous_time))
            })
            .take(4)
            .collect();

        assert!(
            intervals.len() >= 4,
            "expected at least four sequential row intervals, got {positions:?}"
        );

        let tolerance = Duration::from_millis(35);
        for interval in intervals {
            let drift = interval.abs_diff(expected);
            assert!(
                drift <= tolerance,
                "row interval {interval:?} drifted more than {tolerance:?} from {expected:?}"
            );
        }
    }

    #[test]
    fn playback_thread_advances_without_tui_polling() {
        let runtime = PlaybackRuntime::spawn(None);
        let mut song = Song::empty();
        song.transport.bpm = 300;
        song.transport.lines_per_beat = 4;
        let row_duration = Duration::from_micros(row_duration_micros(&song.transport));

        runtime.start_pattern_from(song, 0, 0, true);
        thread::sleep(row_duration.saturating_mul(5) + Duration::from_millis(30));

        let positions = collect_position_times(&runtime, 16, Duration::from_millis(100));
        runtime.stop();

        assert!(
            positions.iter().any(|(row, _)| *row >= 4),
            "playback did not advance while the test withheld TUI polling: {positions:?}"
        );
    }

    #[test]
    fn runtime_writes_midi_log_when_enabled() {
        let path =
            std::env::temp_dir().join(format!("salieri-midi-log-{}.log", std::process::id()));
        let runtime = PlaybackRuntime::spawn(Some(path.clone()));
        let mut song = Song::empty();
        speed_up_transport(&mut song);
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

    #[test]
    fn runtime_disconnects_and_stops_when_midi_send_fails() {
        let path = std::env::temp_dir().join(format!(
            "salieri-midi-failure-log-{}.log",
            std::process::id()
        ));
        let mut song = Song::empty();
        speed_up_transport(&mut song);
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
            .expect("set note");

        let (_command_tx, command_rx) = mpsc::channel();
        let (update_tx, update_rx) = mpsc::channel();
        let mut output = PlaybackOutput::failing();
        let mut midi_logger = MidiLogger::new(Some(path.clone()), &update_tx);
        let mut audio_output = PlaybackAudioOutput::disabled(AudioConfig::default().sample_rate);
        let audio_sample_rate = audio_output.sample_rate();
        let mut context = PlaybackRunContext {
            command_rx: &command_rx,
            update_tx: &update_tx,
            output: &mut output,
            midi_logger: &mut midi_logger,
            audio_output: &mut audio_output,
            audio_sample_rate,
        };

        let result = run_pattern(&song, 0, 0, None, true, &mut context);

        assert!(matches!(result, PatternRunResult::Stopped));
        assert!(matches!(output, PlaybackOutput::Fake(_)));

        let updates: Vec<_> = update_rx.try_iter().collect();
        assert!(updates
            .iter()
            .any(|update| matches!(update, PlaybackUpdate::MidiDisconnected)));
        assert!(updates
            .iter()
            .any(|update| matches!(update, PlaybackUpdate::Stopped)));
        assert!(updates.iter().any(|update| matches!(
            update,
            PlaybackUpdate::MidiError(message)
                if message.contains("MIDI output disconnected during playback")
        )));

        let contents = std::fs::read_to_string(&path).expect("midi log");
        let _ = std::fs::remove_file(&path);

        assert!(contents.contains("NOTE_ON"));
        assert!(contents.contains("SEND_ERROR stopping playback"));
        assert!(contents.contains("CC ch=1 controller=123 value=0"));
        assert!(contents.contains("ALL_NOTES_OFF_ERROR during MIDI recovery"));
    }

    #[test]
    fn fake_midi_pattern_playback_emits_note_on_and_note_off() {
        let mut song = Song::empty();
        speed_up_transport(&mut song);
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x64)
            .expect("set note");
        let (_command_tx, command_rx) = mpsc::channel();

        let (result, sent, _updates) = run_pattern_with_recording(&song, 0, 0, false, &command_rx);

        assert!(matches!(result, PatternRunResult::Finished));
        assert!(sent.contains(&MidiMessage::note_on(10, 60, 0x64)));
        assert!(sent.contains(&MidiMessage::note_off(10, 60, 0)));
    }

    #[test]
    fn fake_midi_sequence_playback_emits_each_pattern_and_panic_cleanup() {
        let mut song = Song::empty();
        speed_up_transport(&mut song);
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
            .expect("set first note");
        let second_pattern_id = song.create_pattern(4);
        let second_pattern_index = song
            .patterns
            .iter()
            .position(|pattern| pattern.id == second_pattern_id)
            .expect("second pattern");
        song.pattern_mut(second_pattern_index)
            .expect("second pattern")
            .set_note(0, 1, NoteEvent::Note { pitch: 48 }, 0x50)
            .expect("set second note");
        song.push_sequence_pattern(second_pattern_id)
            .expect("push sequence");

        let (next_command, sent, updates) = run_sequence_with_recording(song, 0);

        assert!(next_command.is_none());
        assert!(sent.contains(&MidiMessage::note_on(10, 60, 0x7f)));
        assert!(sent.contains(&MidiMessage::note_on(1, 48, 0x50)));
        assert_eq!(
            sent.iter()
                .filter(|message| matches!(
                    message,
                    MidiMessage::ControlChange {
                        controller: 123,
                        ..
                    }
                ))
                .count(),
            16
        );
        assert!(updates
            .iter()
            .any(|update| matches!(update, PlaybackUpdate::Stopped)));
    }

    #[test]
    fn fake_midi_stop_command_sends_all_notes_off() {
        let mut song = Song::empty();
        speed_up_transport(&mut song);
        let (command_tx, command_rx) = mpsc::channel();
        command_tx.send(PlaybackCommand::Stop).expect("queue stop");

        let (result, sent, updates) = run_pattern_with_recording(&song, 0, 0, true, &command_rx);

        assert!(matches!(
            result,
            PatternRunResult::Command(command) if matches!(*command, PlaybackCommand::Stop)
        ));
        assert_eq!(sent.len(), 16);
        assert_eq!(sent[0], MidiMessage::all_notes_off(1));
        assert_eq!(sent[15], MidiMessage::all_notes_off(16));
        assert!(updates
            .iter()
            .any(|update| matches!(update, PlaybackUpdate::Stopped)));
    }

    #[test]
    fn fake_midi_playback_honors_mute_and_solo() {
        let mut muted_song = Song::empty();
        speed_up_transport(&mut muted_song);
        {
            let pattern = muted_song.current_pattern_mut().expect("pattern");
            pattern
                .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
                .expect("set drums note");
            pattern
                .set_note(0, 1, NoteEvent::Note { pitch: 48 }, 0x70)
                .expect("set bass note");
        }
        muted_song.toggle_mute(0).expect("mute drums");
        let (_command_tx, command_rx) = mpsc::channel();

        let (_result, muted_sent, _updates) =
            run_pattern_with_recording(&muted_song, 0, 0, false, &command_rx);

        assert!(!muted_sent.contains(&MidiMessage::note_on(10, 60, 0x7f)));
        assert!(muted_sent.contains(&MidiMessage::note_on(1, 48, 0x70)));

        let mut solo_song = muted_song;
        solo_song.toggle_mute(0).expect("unmute drums");
        solo_song.toggle_solo(0).expect("solo drums");
        let (_command_tx, command_rx) = mpsc::channel();

        let (_result, solo_sent, _updates) =
            run_pattern_with_recording(&solo_song, 0, 0, false, &command_rx);

        assert!(solo_sent.contains(&MidiMessage::note_on(10, 60, 0x7f)));
        assert!(!solo_sent.contains(&MidiMessage::note_on(1, 48, 0x70)));
    }

    fn assert_approx_eq(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.000_001,
            "expected {expected}, got {actual}"
        );
    }
}
