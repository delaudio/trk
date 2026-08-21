use crate::{
    parameter_locks::parameter_lock_f32_at, AutomationTarget, NoteEvent, ParameterLockTarget,
    Pattern, PatternCell, SampleId, SamplePlaybackSettings, Song, TrackId, TrackerCommand,
    TransportSettings, MIXER_MASTER_GAIN_PARAMETER_ID, MIXER_TRACK_GAIN_PARAMETER_ID,
    MIXER_TRACK_PAN_PARAMETER_ID, SAMPLE_GAIN_PARAMETER_ID, SAMPLE_ROOT_NOTE_PARAMETER_ID,
};

mod effects;
mod sampler_locks;
mod sampler_selection;
use effects::{delay_command, retrigger_command};
use sampler_locks::{locked_sample_float, locked_sample_note, locked_sample_playback};
use sampler_selection::sample_for_cell;

#[cfg(test)]
mod fx_tests;
#[cfg(test)]
mod zoned_tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackPosition {
    pub row: usize,
    pub offset_micros: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackEvent {
    pub position: PlaybackPosition,
    pub track: TrackId,
    pub midi_channel: u8,
    pub kind: PlaybackEventKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackEventKind {
    NoteOn { pitch: u8, velocity: u8 },
    NoteOff { pitch: u8 },
    ControlChange { controller: u8, value: u8 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SamplerPlaybackEvent {
    pub position: PlaybackPosition,
    pub track: TrackId,
    pub sample: SampleId,
    pub sample_path: String,
    pub pitch: u8,
    pub velocity: u8,
    pub gain: f32,
    pub pan: f32,
    pub pitch_ratio: f32,
    pub playback: SamplePlaybackSettings,
}

#[must_use]
pub fn row_duration_micros(transport: &TransportSettings) -> u64 {
    let bpm = u64::from(transport.bpm.max(1));
    let lines_per_beat = u64::from(transport.lines_per_beat.max(1));
    60_000_000 / bpm / lines_per_beat
}

#[must_use]
pub fn pattern_events(song: &Song, pattern: &Pattern) -> Vec<PlaybackEvent> {
    let row_duration = row_duration_micros(&song.transport);
    let solo_active = song.tracks.iter().any(|track| track.solo);
    let mut active_notes = vec![None; song.tracks.len()];
    let mut events = Vec::new();
    let end_offset = row_duration.saturating_mul(pattern.row_count() as u64);

    for (row_index, row) in pattern.rows.iter().enumerate() {
        let position = PlaybackPosition {
            row: row_index,
            offset_micros: row_duration.saturating_mul(row_index as u64),
        };

        for lane in &pattern.automation {
            let AutomationTarget::MidiCc { track, controller } = lane.target else {
                continue;
            };
            let Some(point) = lane.points.iter().find(|point| point.row == row_index) else {
                continue;
            };
            let Some((track_index, target_track)) = song
                .tracks
                .iter()
                .enumerate()
                .find(|(_, candidate)| candidate.id == track)
            else {
                continue;
            };
            if !track_is_audible(target_track.muted, target_track.solo, solo_active) {
                continue;
            }
            let value = (point.value.clamp(0.0, 1.0) * 127.0).round() as u8;
            events.push(PlaybackEvent {
                position,
                track: song.tracks[track_index].id,
                midi_channel: target_track.midi_channel,
                kind: PlaybackEventKind::ControlChange { controller, value },
            });
        }

        for (track_index, track) in song.tracks.iter().enumerate() {
            if !track_is_audible(track.muted, track.solo, solo_active) {
                continue;
            }

            let Some(cell) = row.cells.get(track_index) else {
                continue;
            };

            let position = apply_cell_delay(position, row_duration, cell);

            if let Some((pitch, Some(release_offset))) = active_notes[track_index] {
                if release_offset <= position.offset_micros {
                    events.push(note_off(
                        playback_position_for_offset(release_offset, row_duration, pattern),
                        track.id,
                        track.midi_channel,
                        pitch,
                    ));
                    active_notes[track_index] = None;
                }
            }

            match cell.note {
                Some(NoteEvent::Note { pitch }) => {
                    if let Some((active_pitch, _)) = active_notes[track_index] {
                        events.push(note_off(
                            position,
                            track.id,
                            track.midi_channel,
                            active_pitch,
                        ));
                    }
                    let velocity = cell.velocity.unwrap_or(0x7f).min(0x7f);
                    let note_on = PlaybackEvent {
                        position,
                        track: track.id,
                        midi_channel: track.midi_channel,
                        kind: PlaybackEventKind::NoteOn { pitch, velocity },
                    };
                    events.push(note_on);
                    let release_offset = cell.gate.map(|gate| {
                        position
                            .offset_micros
                            .saturating_add(row_duration.saturating_mul(u64::from(gate.max(1))))
                            .min(end_offset)
                    });
                    active_notes[track_index] = Some((pitch, release_offset));
                    emit_retrigger_events(
                        &mut events,
                        note_on,
                        row_duration,
                        retrigger_command(cell),
                    );
                }
                Some(NoteEvent::NoteOff | NoteEvent::NoteCut) => {
                    if let Some((active_pitch, _)) = active_notes[track_index].take() {
                        events.push(note_off(
                            position,
                            track.id,
                            track.midi_channel,
                            active_pitch,
                        ));
                    }
                }
                None => {}
            }
        }
    }

    let end_position = PlaybackPosition {
        row: pattern.row_count(),
        offset_micros: row_duration.saturating_mul(pattern.row_count() as u64),
    };
    for (track_index, active_note) in active_notes.into_iter().enumerate() {
        if let Some((pitch, release_offset)) = active_note {
            let track = &song.tracks[track_index];
            events.push(note_off(
                release_offset.map_or(end_position, |offset| {
                    playback_position_for_offset(offset, row_duration, pattern)
                }),
                track.id,
                track.midi_channel,
                pitch,
            ));
        }
    }

    events.sort_by_key(|event| event.position.offset_micros);
    events
}

#[must_use]
pub fn sampler_events(song: &Song, pattern: &Pattern) -> Vec<SamplerPlaybackEvent> {
    let row_duration = row_duration_micros(&song.transport);
    let solo_active = song.tracks.iter().any(|track| track.solo);
    let mixer_solo_active = song.mixer.tracks.iter().any(|track| track.solo);
    let mut events = Vec::new();

    for (row_index, row) in pattern.rows.iter().enumerate() {
        let position = PlaybackPosition {
            row: row_index,
            offset_micros: row_duration.saturating_mul(row_index as u64),
        };

        for (track_index, track) in song.tracks.iter().enumerate() {
            if !track_is_audible(track.muted, track.solo, solo_active) {
                continue;
            }
            let mixer = song.track_mixer_for_track(track.id);
            if mixer.muted || (mixer_solo_active && !mixer.solo) {
                continue;
            }

            let Some(cell) = row.cells.get(track_index) else {
                continue;
            };
            let Some(NoteEvent::Note { pitch }) = cell.note else {
                continue;
            };
            let velocity = cell.velocity.unwrap_or(0x7f).min(0x7f);
            let Some(sample) = sample_for_cell(song, cell, track.id, pitch, velocity) else {
                continue;
            };

            let position = apply_cell_delay(position, row_duration, cell);
            let cell_gain = cell.volume.map_or(1.0, |volume| f32::from(volume) / 127.0);
            let sample_gain = pattern.automation_value_at(
                AutomationTarget::SampleGain { sample: sample.id },
                row_index,
                sample.gain,
            );
            let sample_target = ParameterLockTarget::Sample { sample: sample.id };
            let sample_gain =
                locked_sample_float(cell, &sample_target, SAMPLE_GAIN_PARAMETER_ID, sample_gain);
            let mixer_gain = parameter_lock_f32_at(
                pattern,
                row_index,
                ParameterLockTarget::TrackMixer { track: track.id },
                MIXER_TRACK_GAIN_PARAMETER_ID,
                mixer.gain,
            );
            let mixer_pan = parameter_lock_f32_at(
                pattern,
                row_index,
                ParameterLockTarget::TrackMixer { track: track.id },
                MIXER_TRACK_PAN_PARAMETER_ID,
                mixer.pan,
            );
            let master_gain = parameter_lock_f32_at(
                pattern,
                row_index,
                ParameterLockTarget::MasterMixer,
                MIXER_MASTER_GAIN_PARAMETER_ID,
                song.mixer.master_gain,
            );
            let gain = sample_gain * cell_gain * mixer_gain * master_gain;
            let pan = combine_pan(sample.pan, cell.pan.map_or(mixer_pan, pan_u7_to_float));
            let root_pitch = locked_sample_note(
                cell,
                &sample_target,
                SAMPLE_ROOT_NOTE_PARAMETER_ID,
                sample.root_pitch,
            );
            let playback = locked_sample_playback(cell, &sample_target, sample.playback);
            let trigger = SamplerPlaybackEvent {
                position,
                track: track.id,
                sample: sample.id,
                sample_path: sample.path.clone(),
                pitch,
                velocity,
                gain,
                pan,
                pitch_ratio: pitch_ratio(
                    pitch,
                    root_pitch,
                    sample.transpose_semitones,
                    sample.fine_tune_cents,
                ),
                playback,
            };
            events.push(trigger.clone());
            emit_sampler_retrigger_events(
                &mut events,
                trigger,
                row_duration,
                retrigger_command(cell),
            );
        }
    }

    events.sort_by_key(|event| event.position.offset_micros);
    events
}

fn apply_cell_delay(
    position: PlaybackPosition,
    row_duration: u64,
    cell: &PatternCell,
) -> PlaybackPosition {
    if let Some(delay) = cell.delay {
        return apply_delay_value(position, row_duration, delay);
    }
    apply_delay_command(position, row_duration, delay_command(cell))
}

fn apply_delay_command(
    position: PlaybackPosition,
    row_duration: u64,
    command: Option<TrackerCommand>,
) -> PlaybackPosition {
    let Some(command) = command else {
        return position;
    };
    if command.code.to_ascii_uppercase() != TrackerCommand::DELAY_CODE {
        return position;
    }

    apply_delay_value(position, row_duration, command.value)
}

fn apply_delay_value(position: PlaybackPosition, row_duration: u64, value: u8) -> PlaybackPosition {
    PlaybackPosition {
        offset_micros: position
            .offset_micros
            .saturating_add(row_duration.saturating_mul(u64::from(value)) / 256),
        ..position
    }
}

fn pan_u7_to_float(value: u8) -> f32 {
    ((f32::from(value.min(0x7f)) - 64.0) / 63.0).clamp(-1.0, 1.0)
}

fn emit_sampler_retrigger_events(
    events: &mut Vec<SamplerPlaybackEvent>,
    trigger: SamplerPlaybackEvent,
    row_duration: u64,
    command: Option<TrackerCommand>,
) {
    let Some(command) = command else {
        return;
    };
    if command.code.to_ascii_uppercase() != TrackerCommand::RETRIGGER_CODE {
        return;
    }

    let count = command.value.clamp(1, 16);
    for step in 1..count {
        let offset = trigger
            .position
            .offset_micros
            .saturating_add(row_duration.saturating_mul(u64::from(step)) / u64::from(count));
        events.push(SamplerPlaybackEvent {
            position: PlaybackPosition {
                offset_micros: offset,
                ..trigger.position
            },
            ..trigger.clone()
        });
    }
}

fn pitch_ratio(pitch: u8, root_pitch: u8, transpose_semitones: i8, fine_tune_cents: i16) -> f32 {
    let semitones = f32::from(pitch) - f32::from(root_pitch)
        + f32::from(transpose_semitones)
        + f32::from(fine_tune_cents) / 100.0;
    2.0_f32.powf(semitones / 12.0)
}

fn combine_pan(sample_pan: f32, event_pan: f32) -> f32 {
    (sample_pan + event_pan).clamp(-1.0, 1.0)
}

fn emit_retrigger_events(
    events: &mut Vec<PlaybackEvent>,
    note_on: PlaybackEvent,
    row_duration: u64,
    command: Option<TrackerCommand>,
) {
    let Some(command) = command else {
        return;
    };
    if command.code.to_ascii_uppercase() != TrackerCommand::RETRIGGER_CODE {
        return;
    }

    let count = command.value.clamp(1, 16);
    let PlaybackEventKind::NoteOn { pitch, velocity } = note_on.kind else {
        return;
    };
    for step in 1..count {
        let offset = note_on
            .position
            .offset_micros
            .saturating_add(row_duration.saturating_mul(u64::from(step)) / u64::from(count));
        let position = PlaybackPosition {
            offset_micros: offset,
            ..note_on.position
        };
        events.push(note_off(
            position,
            note_on.track,
            note_on.midi_channel,
            pitch,
        ));
        events.push(PlaybackEvent {
            position,
            track: note_on.track,
            midi_channel: note_on.midi_channel,
            kind: PlaybackEventKind::NoteOn { pitch, velocity },
        });
    }
}

fn track_is_audible(muted: bool, solo: bool, solo_active: bool) -> bool {
    if solo_active {
        solo
    } else {
        !muted
    }
}

fn note_off(
    position: PlaybackPosition,
    track: TrackId,
    midi_channel: u8,
    pitch: u8,
) -> PlaybackEvent {
    PlaybackEvent {
        position,
        track,
        midi_channel,
        kind: PlaybackEventKind::NoteOff { pitch },
    }
}

fn playback_position_for_offset(
    offset_micros: u64,
    row_duration: u64,
    pattern: &Pattern,
) -> PlaybackPosition {
    PlaybackPosition {
        row: usize::try_from(offset_micros / row_duration.max(1))
            .unwrap_or(pattern.row_count())
            .min(pattern.row_count()),
        offset_micros,
    }
}

#[cfg(test)]
#[path = "playback/gate_tests.rs"]
mod gate_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ParameterId, ParameterLockAction, ParameterValue, SamplePlaybackMode,
        SAMPLE_END_FRAME_PARAMETER_ID, SAMPLE_ENVELOPE_ATTACK_PARAMETER_ID,
        SAMPLE_PLAYBACK_MODE_PARAMETER_ID, SAMPLE_START_FRAME_PARAMETER_ID,
    };
    use crate::{PatternId, TrackId};

    #[test]
    fn solo_tracks_exclude_non_solo_tracks() {
        let mut song = Song::empty();
        song.toggle_solo(1).expect("solo");
        let pattern = song.current_pattern_mut().expect("pattern");
        pattern
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
            .expect("set muted by solo note");
        pattern
            .set_note(0, 1, NoteEvent::Note { pitch: 48 }, 0x7f)
            .expect("set solo note");

        let events = pattern_events(&song, song.current_pattern().expect("pattern"));

        assert_eq!(events[0].track, TrackId(2));
        assert!(events
            .iter()
            .all(|event| event.track == TrackId(2) && event.track != TrackId(1)));
    }

    #[test]
    fn sampler_events_emit_audio_engine_contract_for_assigned_tracks() {
        let mut song = Song::empty();
        let sample_id = song.upsert_sample_reference("samples/kick.wav", "kick.wav");
        let track_id = song.tracks[0].id;
        song.samples[0].root_pitch = 48;
        song.samples[0].gain = 0.75;
        song.assign_sample_to_track(track_id, sample_id)
            .expect("assign sample");
        let pattern = song.current_pattern_mut().expect("pattern");
        pattern
            .set_note(2, 0, NoteEvent::Note { pitch: 60 }, 0x64)
            .expect("set note");
        pattern.cell_mut(2, 0).expect("cell").command = Some(TrackerCommand::delay(128));
        pattern
            .set_note(3, 1, NoteEvent::Note { pitch: 72 }, 0x7f)
            .expect("set unassigned note");

        let events = sampler_events(&song, song.current_pattern().expect("pattern"));

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].position.row, 2);
        assert_eq!(events[0].position.offset_micros, 312_500);
        assert_eq!(events[0].track, track_id);
        assert_eq!(events[0].sample, sample_id);
        assert_eq!(events[0].sample_path, "samples/kick.wav");
        assert_eq!(events[0].pitch, 60);
        assert_eq!(events[0].velocity, 0x64);
        assert_eq!(events[0].gain, 0.75);
        assert!((events[0].pitch_ratio - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn sampler_events_apply_sample_tuning_to_pitch_ratio() {
        let mut song = Song::empty();
        let sample_id = song.upsert_sample_reference("samples/lead.wav", "lead.wav");
        let track_id = song.tracks[0].id;
        song.samples[0].root_pitch = 60;
        song.samples[0].transpose_semitones = 12;
        song.samples[0].fine_tune_cents = -50;
        song.assign_sample_to_track(track_id, sample_id)
            .expect("assign sample");
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
            .expect("set note");
        let events = sampler_events(&song, song.current_pattern().expect("pattern"));
        let expected = 2.0_f32.powf(11.5 / 12.0);
        assert!((events[0].pitch_ratio - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn sampler_events_apply_same_row_source_and_envelope_locks() {
        let mut song = Song::empty();
        let sample_id = song.upsert_sample_reference("samples/lead.wav", "lead.wav");
        let track_id = song.tracks[0].id;
        song.assign_sample_to_track(track_id, sample_id)
            .expect("assign sample");
        let pattern = song.current_pattern_mut().expect("pattern");
        pattern
            .set_note(2, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
            .expect("set note");
        for (parameter, value) in [
            (SAMPLE_ROOT_NOTE_PARAMETER_ID, ParameterValue::Note(72)),
            (
                SAMPLE_PLAYBACK_MODE_PARAMETER_ID,
                ParameterValue::Enum("reverse".to_string()),
            ),
            (SAMPLE_START_FRAME_PARAMETER_ID, ParameterValue::Integer(12)),
            (SAMPLE_END_FRAME_PARAMETER_ID, ParameterValue::Integer(100)),
            (
                SAMPLE_ENVELOPE_ATTACK_PARAMETER_ID,
                ParameterValue::Seconds(0.2),
            ),
        ] {
            pattern
                .set_parameter_lock(
                    2,
                    0,
                    crate::ParameterLock {
                        target: ParameterLockTarget::Sample { sample: sample_id },
                        parameter: ParameterId::from(parameter),
                        action: ParameterLockAction::Set { value },
                    },
                )
                .expect("source lock");
        }

        let events = sampler_events(&song, song.current_pattern().expect("pattern"));

        assert_eq!(events.len(), 1);
        assert!((events[0].pitch_ratio - 0.5).abs() < f32::EPSILON);
        assert_eq!(events[0].playback.mode, SamplePlaybackMode::Reverse);
        assert_eq!(events[0].playback.start_frame, Some(12));
        assert_eq!(events[0].playback.end_frame, Some(100));
        assert_eq!(events[0].playback.envelope.attack_seconds, 0.2);

        song.current_pattern_mut()
            .expect("pattern")
            .set_parameter_lock(
                2,
                0,
                crate::ParameterLock {
                    target: ParameterLockTarget::Sample { sample: sample_id },
                    parameter: ParameterId::from(SAMPLE_END_FRAME_PARAMETER_ID),
                    action: ParameterLockAction::Set {
                        value: ParameterValue::Integer(i64::MAX),
                    },
                },
            )
            .expect("oversized frame lock");
        let events = sampler_events(&song, song.current_pattern().expect("pattern"));
        assert_eq!(events[0].playback.end_frame, Some(i32::MAX as usize));

        let attack_lock = song.current_pattern_mut().expect("pattern").rows[2].cells[0]
            .parameter_locks
            .iter_mut()
            .find(|lock| lock.parameter.as_str() == SAMPLE_ENVELOPE_ATTACK_PARAMETER_ID)
            .expect("attack lock");
        attack_lock.action = ParameterLockAction::Set {
            value: ParameterValue::Seconds(999.0),
        };
        let events = sampler_events(&song, song.current_pattern().expect("pattern"));
        assert_eq!(events[0].playback.envelope.attack_seconds, 60.0);
    }

    #[test]
    fn sample_playback_metadata_survives_json_round_trip() {
        let mut song = Song::empty();
        song.upsert_sample_reference("samples/lead.wav", "lead.wav");
        song.samples[0].root_pitch = 72;
        song.samples[0].transpose_semitones = -12;
        song.samples[0].fine_tune_cents = 25;
        song.samples[0].gain = 0.625;
        song.samples[0].pan = -0.5;
        let serialized = serde_json::to_string(&song).expect("serialize song");
        let restored: Song = serde_json::from_str(&serialized).expect("deserialize song");
        assert_eq!(restored.samples[0].root_pitch, 72);
        assert_eq!(restored.samples[0].transpose_semitones, -12);
        assert_eq!(restored.samples[0].fine_tune_cents, 25);
        assert_eq!(restored.samples[0].gain, 0.625);
        assert_eq!(restored.samples[0].pan, -0.5);
    }

    #[test]
    fn sampler_events_apply_stepped_sample_gain_automation() {
        let mut song = Song::empty();
        let sample_id = song.upsert_sample_reference("samples/kick.wav", "kick.wav");
        let track_id = song.tracks[0].id;
        song.samples[0].gain = 1.0;
        song.assign_sample_to_track(track_id, sample_id)
            .expect("assign sample");
        let pattern = song.current_pattern_mut().expect("pattern");
        pattern
            .set_note(0, 0, NoteEvent::Note { pitch: 48 }, 0x7f)
            .expect("set first note");
        pattern
            .set_note(4, 0, NoteEvent::Note { pitch: 48 }, 0x7f)
            .expect("set automated note");
        pattern
            .set_automation_point(AutomationTarget::SampleGain { sample: sample_id }, 4, 0.25)
            .expect("set automation");

        let events = sampler_events(&song, song.current_pattern().expect("pattern"));

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].position.row, 0);
        assert_eq!(events[0].gain, 1.0);
        assert_eq!(events[1].position.row, 4);
        assert_eq!(events[1].gain, 0.25);
    }

    #[test]
    fn sampler_events_apply_mixer_gain_pan_and_master() {
        let mut song = Song::empty();
        let sample_id = song.upsert_sample_reference("samples/kick.wav", "kick.wav");
        let track_id = song.tracks[0].id;
        song.samples[0].gain = 0.5;
        song.samples[0].pan = -0.25;
        song.set_track_mixer_gain(0, 0.5).expect("track gain");
        song.set_track_mixer_pan(0, 0.75).expect("track pan");
        song.set_master_gain(0.5).expect("master gain");
        song.assign_sample_to_track(track_id, sample_id)
            .expect("assign sample");
        let pattern = song.current_pattern_mut().expect("pattern");
        pattern
            .set_note(0, 0, NoteEvent::Note { pitch: 48 }, 0x7f)
            .expect("set note");

        let events = sampler_events(&song, song.current_pattern().expect("pattern"));

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].gain, 0.125);
        assert_eq!(events[0].pan, 0.5);
    }

    #[test]
    fn sampler_events_apply_same_row_sample_and_mixer_parameter_locks() {
        let mut song = Song::empty();
        let sample_id = song.upsert_sample_reference("samples/kick.wav", "kick.wav");
        let track_id = song.tracks[0].id;
        song.samples[0].gain = 1.0;
        song.set_track_mixer_gain(0, 1.0).expect("track gain");
        song.set_track_mixer_pan(0, 0.0).expect("track pan");
        song.set_master_gain(1.0).expect("master gain");
        song.assign_sample_to_track(track_id, sample_id)
            .expect("assign sample");
        let pattern = song.current_pattern_mut().expect("pattern");
        pattern
            .set_note(4, 0, NoteEvent::Note { pitch: 48 }, 0x7f)
            .expect("set note");
        pattern
            .set_parameter_lock(
                4,
                0,
                crate::ParameterLock {
                    target: ParameterLockTarget::Sample { sample: sample_id },
                    parameter: ParameterId::from(SAMPLE_GAIN_PARAMETER_ID),
                    action: ParameterLockAction::Set {
                        value: ParameterValue::Float(0.5),
                    },
                },
            )
            .expect("sample lock");
        pattern
            .set_parameter_lock(
                4,
                1,
                crate::ParameterLock {
                    target: ParameterLockTarget::TrackMixer { track: track_id },
                    parameter: ParameterId::from(MIXER_TRACK_PAN_PARAMETER_ID),
                    action: ParameterLockAction::Set {
                        value: ParameterValue::Bipolar(-0.25),
                    },
                },
            )
            .expect("mixer lock");

        let events = sampler_events(&song, song.current_pattern().expect("pattern"));

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].gain, 0.5);
        assert_eq!(events[0].pan, -0.25);
    }

    #[test]
    fn sampler_events_apply_tracker_cell_columns() {
        let mut song = Song::empty();
        let track_sample = song.upsert_sample_reference("samples/kick.wav", "kick.wav");
        let cell_sample = song.upsert_sample_reference("samples/snare.wav", "snare.wav");
        let instrument = song
            .upsert_sample_instrument(cell_sample)
            .expect("instrument");
        let track_id = song.tracks[0].id;
        song.samples[0].gain = 0.5;
        song.samples[1].gain = 1.0;
        song.assign_sample_to_track(track_id, track_sample)
            .expect("assign track sample");
        let pattern = song.current_pattern_mut().expect("pattern");
        pattern
            .set_note(1, 0, NoteEvent::Note { pitch: 48 }, 0x7f)
            .expect("set note");
        let cell = pattern.cell_mut(1, 0).expect("cell");
        cell.instrument = Some(instrument);
        cell.volume = Some(0x40);
        cell.pan = Some(0x7f);
        cell.delay = Some(0x40);

        let events = sampler_events(&song, song.current_pattern().expect("pattern"));

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].sample, cell_sample);
        assert_eq!(events[0].sample_path, "samples/snare.wav");
        assert_eq!(events[0].position.offset_micros, 156_250);
        assert!((events[0].gain - (64.0 / 127.0)).abs() < f32::EPSILON);
        assert_eq!(events[0].pan, 1.0);
    }

    #[test]
    fn can_schedule_non_current_pattern() {
        let song = Song::empty();
        let mut pattern = crate::Pattern::empty(PatternId(2), "Other", 16, song.tracks.len());
        pattern
            .set_note(1, 0, NoteEvent::Note { pitch: 36 }, 0x50)
            .expect("set note");

        let events = pattern_events(&song, &pattern);

        assert_eq!(events[0].position.row, 1);
        assert_eq!(events[0].position.offset_micros, 125_000);
    }
}
