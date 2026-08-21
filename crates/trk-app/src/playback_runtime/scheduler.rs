use std::{
    sync::mpsc::{Receiver, RecvTimeoutError, Sender, TryRecvError},
    time::{Duration, Instant},
};

use trk_audio::RealtimeAudioCommand;
use trk_core::{
    pattern_events, row_duration_micros, sampler_events, Pattern, PlaybackEvent, PlaybackEventKind,
    PlaybackPosition, Song,
};
use trk_midi::MidiError;

use super::{
    audio_dispatch::{send_all_audio_notes_off, send_audio_command, PlaybackAudioOutput},
    logging::{send_all_notes_off_logged, send_playback_event_logged, MidiLogger},
    midi_dispatch::PlaybackOutput,
    sample_preload::audio_sampler_playback_settings,
    transport::{PlaybackCommand, PlaybackCursor, PlaybackUpdate},
};

mod live_updates;
use live_updates::{apply_live_mute, remap_active_pattern};

#[derive(Debug)]
pub(super) enum PatternRunResult {
    Finished,
    Stopped,
    Command(Box<PlaybackCommand>),
}

enum PatternWaitResult {
    Deadline,
    ApplyLiveMute {
        track: trk_core::TrackId,
        muted: bool,
    },
    Command(Box<PlaybackCommand>),
}

enum SettledPatternWaitResult {
    Deadline,
    Command(Box<PlaybackCommand>),
}

pub(super) struct PlaybackRunContext<'a> {
    pub(super) command_rx: &'a Receiver<PlaybackCommand>,
    pub(super) update_tx: &'a Sender<PlaybackUpdate>,
    pub(super) output: &'a mut PlaybackOutput,
    pub(super) midi_logger: &'a mut MidiLogger,
    pub(super) audio_output: &'a mut PlaybackAudioOutput,
    pub(super) audio_sample_rate: u32,
    pub(super) pending_reload: Option<(Song, u64)>,
}

pub(super) fn run_pattern(
    song: &mut Song,
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
    let mut pattern = pattern.clone();
    let mut active_pattern_id = pattern.id;
    let mut pattern_index = pattern_index;
    let mut row_count = pattern.row_count();
    let mut row_micros = row_duration_micros(&song.transport).max(1);
    let mut row_duration = Duration::from_micros(row_micros);
    let mut events = pattern_events(song, &pattern);
    let mut sample_events = sampler_events(song, &pattern);
    let mut pending_pattern = None;
    let mut pass_start_row = start_row.min(row_count);
    let mut audio_offset_micros = 0_u64;
    let mut has_emitted_row = false;
    loop {
        let mut active_sent_notes = Vec::new();
        let mut row = pass_start_row.min(row_count);
        let mut row_deadline = Instant::now();

        while row <= row_count {
            if let Some(replacement) = pending_pattern.take() {
                pattern = replacement;
                row_count = pattern.row_count();
                events = pattern_events(song, &pattern);
                sample_events = sampler_events(song, &pattern);
            }
            let mut song_changed = false;
            match wait_until_pattern_with_live_mutes(
                row_deadline,
                pattern_index,
                song,
                &mut pending_pattern,
                row,
                &mut active_sent_notes,
                context,
            ) {
                Ok(SettledPatternWaitResult::Deadline) => {}
                Ok(SettledPatternWaitResult::Command(command)) => {
                    let _ = send_all_notes_off_logged(context.output, context.midi_logger);
                    send_all_audio_notes_off(context.audio_output);
                    let _ = context.update_tx.send(PlaybackUpdate::Stopped);
                    return PatternRunResult::Command(command);
                }
                Err(error) => {
                    handle_midi_send_failure(context, error);
                    return PatternRunResult::Stopped;
                }
            }
            if let Some(replacement) = pending_pattern.take() {
                pattern = replacement;
                row_count = pattern.row_count();
                events = pattern_events(song, &pattern);
                sample_events = sampler_events(song, &pattern);
            }

            let beat_boundary = row % usize::from(song.transport.lines_per_beat.max(1)) == 0;
            let mut reload_token = None;
            if beat_boundary && has_emitted_row {
                if let Some((replacement, token)) = context.pending_reload.take() {
                    if let Err(error) =
                        send_all_notes_off_logged(context.output, context.midi_logger)
                    {
                        send_all_audio_notes_off(context.audio_output);
                        handle_midi_send_failure(context, error);
                        return PatternRunResult::Stopped;
                    }
                    send_all_audio_notes_off(context.audio_output);
                    active_sent_notes.clear();
                    if !context
                        .audio_output
                        .sync_samples(&replacement, context.update_tx)
                    {
                        let _ = context.update_tx.send(PlaybackUpdate::Stopped);
                        return PatternRunResult::Stopped;
                    }
                    *song = replacement;
                    let Some(remapped_index) =
                        remap_active_pattern(song, active_pattern_id, sequence_index)
                    else {
                        let _ = context.update_tx.send(PlaybackUpdate::Stopped);
                        return PatternRunResult::Stopped;
                    };
                    pattern_index = remapped_index;
                    active_pattern_id = song.patterns[pattern_index].id;
                    song_changed = true;
                    reload_token = Some(token);
                }
            }
            if song_changed {
                let Some(replacement) = song.pattern(pattern_index).cloned() else {
                    let _ = context.update_tx.send(PlaybackUpdate::Stopped);
                    return PatternRunResult::Stopped;
                };
                pattern = replacement;
                row_count = pattern.row_count();
                row_micros = row_duration_micros(&song.transport).max(1);
                row_duration = Duration::from_micros(row_micros);
                events = pattern_events(song, &pattern);
                sample_events = sampler_events(song, &pattern);
                context.audio_output.sync_dsp_graph(song, context.update_tx);
            }
            if let Some(token) = reload_token {
                let _ = context
                    .update_tx
                    .send(PlaybackUpdate::PerformanceReloaded { token });
            }
            if row > row_count {
                break;
            }

            context
                .audio_output
                .sync_dsp_graph_at_row(song, &pattern, row, context.update_tx);
            for event in events.iter().filter(|event| event.position.row == row) {
                let row_offset = row_micros.saturating_mul(row as u64);
                let event_deadline = row_deadline
                    + Duration::from_micros(
                        event.position.offset_micros.saturating_sub(row_offset),
                    );
                match wait_until_pattern_with_live_mutes(
                    event_deadline,
                    pattern_index,
                    song,
                    &mut pending_pattern,
                    row,
                    &mut active_sent_notes,
                    context,
                ) {
                    Ok(SettledPatternWaitResult::Deadline) => {}
                    Ok(SettledPatternWaitResult::Command(command)) => {
                        let _ = send_all_notes_off_logged(context.output, context.midi_logger);
                        send_all_audio_notes_off(context.audio_output);
                        let _ = context.update_tx.send(PlaybackUpdate::Stopped);
                        return PatternRunResult::Command(command);
                    }
                    Err(error) => {
                        handle_midi_send_failure(context, error);
                        return PatternRunResult::Stopped;
                    }
                }
                if !midi_event_allowed(song, event) {
                    continue;
                }
                if !event_can_be_sent(&active_sent_notes, event) {
                    continue;
                }
                if let Err(error) =
                    send_playback_event_logged(context.output, context.midi_logger, *event)
                {
                    handle_midi_send_failure(context, error);
                    return PatternRunResult::Stopped;
                }
                mark_sent_event(&mut active_sent_notes, event);
            }

            for event in sample_events
                .iter()
                .filter(|event| event.position.row == row)
            {
                let row_offset = row_micros.saturating_mul(row as u64);
                let relative_offset = event.position.offset_micros.saturating_sub(row_offset);
                let event_deadline = row_deadline + Duration::from_micros(relative_offset);
                match wait_until_pattern_with_live_mutes(
                    event_deadline,
                    pattern_index,
                    song,
                    &mut pending_pattern,
                    row,
                    &mut active_sent_notes,
                    context,
                ) {
                    Ok(SettledPatternWaitResult::Deadline) => {}
                    Ok(SettledPatternWaitResult::Command(command)) => {
                        let _ = send_all_notes_off_logged(context.output, context.midi_logger);
                        send_all_audio_notes_off(context.audio_output);
                        let _ = context.update_tx.send(PlaybackUpdate::Stopped);
                        return PatternRunResult::Command(command);
                    }
                    Err(error) => {
                        handle_midi_send_failure(context, error);
                        return PatternRunResult::Stopped;
                    }
                }
                if !track_is_audible_now(song, event.track) {
                    continue;
                }
                let playback_offset = audio_offset_micros.saturating_add(relative_offset);
                let command = RealtimeAudioCommand::TriggerSample {
                    track_id: event.track.0,
                    sample_id: event.sample.0,
                    frame: micros_to_frames(playback_offset, context.audio_sample_rate),
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
            if row < row_count {
                audio_offset_micros = audio_offset_micros.saturating_add(row_micros);
                has_emitted_row = true;
            }
            row = row.saturating_add(1);
            row_deadline += row_duration;
        }

        if !loop_pattern {
            return PatternRunResult::Finished;
        }
        pass_start_row = 0;
    }
}

pub(super) fn run_pattern_chain(
    mut song: Song,
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

        match run_pattern(
            &mut song,
            pattern_index,
            pattern_start_row,
            None,
            false,
            context,
        ) {
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
    mut song: Song,
    start_sequence_index: usize,
    context: &mut PlaybackRunContext<'_>,
) -> Option<PlaybackCommand> {
    let mut sequence_index = start_sequence_index;
    while let Some(pattern_id) = song.sequence.get(sequence_index).copied() {
        let Some(pattern_index) = song
            .patterns
            .iter()
            .position(|pattern| pattern.id == pattern_id)
        else {
            sequence_index = sequence_index.saturating_add(1);
            continue;
        };

        match run_pattern(
            &mut song,
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
        sequence_index = sequence_index.saturating_add(1);
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
        && track_is_audible_now(song, event.track)
        && (song.midi.output_channels.is_empty()
            || song.midi.output_channels.contains(&event.midi_channel))
}

fn track_is_audible_now(song: &Song, track_id: trk_core::TrackId) -> bool {
    let solo_active = song.tracks.iter().any(|track| track.solo);
    song.tracks
        .iter()
        .find(|track| track.id == track_id)
        .is_some_and(|track| !track.muted && (!solo_active || track.solo))
}

fn event_can_be_sent(
    active_notes: &[(trk_core::TrackId, u8)],
    event: &trk_core::PlaybackEvent,
) -> bool {
    match event.kind {
        trk_core::PlaybackEventKind::NoteOn { .. }
        | trk_core::PlaybackEventKind::ControlChange { .. } => true,
        trk_core::PlaybackEventKind::NoteOff { pitch } => active_notes
            .iter()
            .any(|(track, active_pitch)| *track == event.track && *active_pitch == pitch),
    }
}

fn mark_sent_event(
    active_notes: &mut Vec<(trk_core::TrackId, u8)>,
    event: &trk_core::PlaybackEvent,
) {
    match event.kind {
        trk_core::PlaybackEventKind::NoteOn { pitch, .. } => {
            active_notes.retain(|(track, _)| *track != event.track);
            active_notes.push((event.track, pitch));
        }
        trk_core::PlaybackEventKind::NoteOff { pitch } => {
            active_notes
                .retain(|(track, active_pitch)| *track != event.track || *active_pitch != pitch);
        }
        trk_core::PlaybackEventKind::ControlChange { .. } => {}
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

fn wait_until_pattern(
    command_rx: &Receiver<PlaybackCommand>,
    deadline: Instant,
    pattern_index: usize,
    song: &mut Song,
    pending_pattern: &mut Option<Pattern>,
    pending_reload: &mut Option<(Song, u64)>,
) -> PatternWaitResult {
    loop {
        match wait_until(command_rx, deadline) {
            Some(PlaybackCommand::ReplacePattern {
                pattern_index: replacement_index,
                pattern,
            }) => {
                if let Some(target) = song.patterns.get_mut(replacement_index) {
                    *target = pattern.clone();
                } else {
                    tracing::debug!(
                        replacement_index,
                        "ignored live replacement for an unknown pattern"
                    );
                    continue;
                }
                if replacement_index == pattern_index {
                    *pending_pattern = Some(pattern);
                } else {
                    tracing::debug!(
                        replacement_index,
                        active_pattern_index = pattern_index,
                        "stored live replacement for a later pattern"
                    );
                }
            }
            Some(PlaybackCommand::ApplyLiveMute { track, muted }) => {
                return PatternWaitResult::ApplyLiveMute { track, muted };
            }
            Some(PlaybackCommand::ReloadSongAtNextBeat { song, token }) => {
                *pending_reload = Some((song, token));
            }
            Some(command) => return PatternWaitResult::Command(Box::new(command)),
            None => return PatternWaitResult::Deadline,
        }
    }
}

fn wait_until_pattern_with_live_mutes(
    deadline: Instant,
    pattern_index: usize,
    song: &mut Song,
    pending_pattern: &mut Option<Pattern>,
    row: usize,
    active_notes: &mut Vec<(trk_core::TrackId, u8)>,
    context: &mut PlaybackRunContext<'_>,
) -> Result<SettledPatternWaitResult, MidiError> {
    loop {
        match wait_until_pattern(
            context.command_rx,
            deadline,
            pattern_index,
            song,
            pending_pattern,
            &mut context.pending_reload,
        ) {
            PatternWaitResult::Deadline => return Ok(SettledPatternWaitResult::Deadline),
            PatternWaitResult::Command(command) => {
                return Ok(SettledPatternWaitResult::Command(command));
            }
            PatternWaitResult::ApplyLiveMute { track, muted } => {
                apply_live_mute(song, track, muted, row, active_notes, context)?;
            }
        }
    }
}
