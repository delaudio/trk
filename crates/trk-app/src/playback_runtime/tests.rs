use std::{
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

use trk_audio::{AudioConfig, RealtimeAudioCommand};
use trk_core::{
    row_duration_micros, AutomationTarget, InstrumentSampleZone, NoteEvent, PlaybackPosition, Song,
};
use trk_midi::MidiMessage;

use super::*;

mod runtime;
mod scheduling;

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
    let mut song = song.clone();
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
        &mut song,
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
    let mut song = song.clone();
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

    let result = run_pattern(&mut song, pattern_index, 0, None, false, &mut context);
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

fn run_pattern_chain_with_recording(
    song: &Song,
    start_pattern_index: usize,
    loop_patterns: bool,
    queued_command: Option<PlaybackCommand>,
) -> (
    Option<PlaybackCommand>,
    Vec<MidiMessage>,
    Vec<PlaybackUpdate>,
) {
    let messages = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let (command_tx, command_rx) = mpsc::channel();
    if let Some(command) = queued_command {
        command_tx.send(command).expect("queue playback command");
    }
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

    let next_command = run_pattern_chain(
        song.clone(),
        start_pattern_index,
        0,
        loop_patterns,
        &mut context,
    );
    let sent = messages.lock().expect("recorded MIDI messages").clone();
    let updates = update_rx.try_iter().collect();
    (next_command, sent, updates)
}

fn assert_approx_eq(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.000_001,
        "expected {expected}, got {actual}"
    );
}
