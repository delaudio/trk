use std::{
    sync::mpsc::{Receiver, RecvTimeoutError, Sender, TryRecvError},
    time::{Duration, Instant},
};

use trk_audio::RealtimeAudioCommand;
use trk_core::{pattern_events, row_duration_micros, sampler_events, PlaybackPosition, Song};
use trk_midi::MidiError;

use super::{
    audio_dispatch::{send_all_audio_notes_off, send_audio_command, PlaybackAudioOutput},
    logging::{send_all_notes_off_logged, send_playback_event_logged, MidiLogger},
    midi_dispatch::PlaybackOutput,
    sample_preload::audio_sampler_playback_settings,
    transport::{PlaybackCommand, PlaybackCursor, PlaybackUpdate},
};

#[derive(Debug)]
pub(super) enum PatternRunResult {
    Finished,
    Stopped,
    Command(Box<PlaybackCommand>),
}

pub(super) struct PlaybackRunContext<'a> {
    pub(super) command_rx: &'a Receiver<PlaybackCommand>,
    pub(super) update_tx: &'a Sender<PlaybackUpdate>,
    pub(super) output: &'a mut PlaybackOutput,
    pub(super) midi_logger: &'a mut MidiLogger,
    pub(super) audio_output: &'a mut PlaybackAudioOutput,
    pub(super) audio_sample_rate: u32,
}

pub(super) fn run_pattern(
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
                if !midi_event_allowed(song, event) {
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
                    track_id: event.track.0,
                    sample_id: event.sample.0,
                    frame: micros_to_frames(relative_offset, context.audio_sample_rate),
                    gain: event.gain * (f32::from(event.velocity.min(0x7f)) / 127.0),
                    pan: event.pan,
                    pitch_ratio: event.pitch_ratio,
                    playback: audio_sampler_playback_settings(event.playback),
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

pub(super) fn run_pattern_chain(
    song: &Song,
    start_pattern_index: usize,
    start_row: usize,
    loop_patterns: bool,
    context: &mut PlaybackRunContext<'_>,
) -> Option<PlaybackCommand> {
    if song.patterns.is_empty() {
        let _ = context.update_tx.send(PlaybackUpdate::Stopped);
        return None;
    }

    let mut pattern_index = start_pattern_index.min(song.patterns.len().saturating_sub(1));
    let mut first_pass = true;

    loop {
        let pattern_start_row = if first_pass { start_row } else { 0 };
        first_pass = false;

        match run_pattern(song, pattern_index, pattern_start_row, None, false, context) {
            PatternRunResult::Finished => {}
            PatternRunResult::Stopped => return None,
            PatternRunResult::Command(command) => return Some(*command),
        }

        let next_pattern_index = pattern_index.saturating_add(1);
        if next_pattern_index < song.patterns.len() {
            pattern_index = next_pattern_index;
            continue;
        }

        if loop_patterns {
            pattern_index = 0;
        } else {
            let _ = send_all_notes_off_logged(context.output, context.midi_logger);
            send_all_audio_notes_off(context.audio_output);
            let _ = context.update_tx.send(PlaybackUpdate::Stopped);
            return None;
        }
    }
}

pub(super) fn run_sequence(
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

fn midi_event_allowed(song: &Song, event: &trk_core::PlaybackEvent) -> bool {
    let kind_allowed = match event.kind {
        trk_core::PlaybackEventKind::NoteOn { .. }
        | trk_core::PlaybackEventKind::NoteOff { .. } => song.midi.notes_out,
        trk_core::PlaybackEventKind::ControlChange { .. } => song.midi.cc_out,
    };
    kind_allowed
        && (song.midi.output_channels.is_empty()
            || song.midi.output_channels.contains(&event.midi_channel))
}

fn mark_event_for_started_playback(
    active_notes: &mut Vec<(trk_core::TrackId, u8)>,
    event: &trk_core::PlaybackEvent,
) -> bool {
    match event.kind {
        trk_core::PlaybackEventKind::NoteOn { pitch, .. } => {
            active_notes.retain(|(track, _)| *track != event.track);
            active_notes.push((event.track, pitch));
            true
        }
        trk_core::PlaybackEventKind::NoteOff { pitch } => {
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
        trk_core::PlaybackEventKind::ControlChange { .. } => true,
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

pub(super) fn micros_to_frames(offset_micros: u64, sample_rate: u32) -> u64 {
    u64::from(sample_rate).saturating_mul(offset_micros) / 1_000_000
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
