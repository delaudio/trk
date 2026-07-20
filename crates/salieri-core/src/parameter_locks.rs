use crate::{
    model::{
        EditError, EffectDevice, EffectDeviceKind, ParameterLock, ParameterLockAction,
        ParameterLockDiagnostic, ParameterLockTarget, Pattern, Song, ValidationError,
    },
    parameters::{
        mixer_master_gain_descriptor, mixer_send_gain_descriptor, mixer_track_gain_descriptor,
        mixer_track_pan_descriptor, sample_gain_descriptor, ParameterDescriptor, ParameterId,
        ParameterValue,
    },
    playback::{row_duration_micros, PlaybackPosition},
    TrackId,
};

#[derive(Debug, Clone, PartialEq)]
pub struct ParameterLockEvent {
    pub position: PlaybackPosition,
    pub cell_track: TrackId,
    pub order: usize,
    pub lock: ParameterLock,
}

impl Song {
    #[must_use]
    pub fn parameter_lock_descriptor(&self, lock: &ParameterLock) -> Option<ParameterDescriptor> {
        descriptor_for_target(self, &lock.target, &lock.parameter)
    }

    pub fn validate_parameter_lock(&self, lock: &ParameterLock) -> Result<(), ValidationError> {
        let Some(descriptor) = self.parameter_lock_descriptor(lock) else {
            return Ok(());
        };
        if descriptor.flags.read_only || !descriptor.flags.automatable {
            return Err(ValidationError::InvalidParameterLockValue {
                pattern_index: 0,
                row_index: 0,
                track_index: 0,
            });
        }
        if let ParameterLockAction::Set { value } = &lock.action {
            descriptor
                .validate(value)
                .map_err(|_| ValidationError::InvalidParameterLockValue {
                    pattern_index: 0,
                    row_index: 0,
                    track_index: 0,
                })?;
        }
        Ok(())
    }

    #[must_use]
    pub fn parameter_lock_diagnostics(&self) -> Vec<ParameterLockDiagnostic> {
        self.patterns
            .iter()
            .enumerate()
            .flat_map(|(pattern_index, pattern)| {
                pattern
                    .rows
                    .iter()
                    .enumerate()
                    .flat_map(move |(row_index, row)| {
                        row.cells
                            .iter()
                            .enumerate()
                            .flat_map(move |(track_index, cell)| {
                                cell.parameter_locks.iter().filter_map(move |lock| {
                                    parameter_lock_diagnostic(
                                        self,
                                        pattern_index,
                                        row_index,
                                        track_index,
                                        lock,
                                    )
                                })
                            })
                    })
            })
            .collect()
    }
}

impl Pattern {
    pub fn set_parameter_lock(
        &mut self,
        row: usize,
        track: usize,
        lock: ParameterLock,
    ) -> Result<(), EditError> {
        let cell = self
            .cell_mut(row, track)
            .ok_or(EditError::CellOutOfBounds { row, track })?;
        if let Some(existing) = cell
            .parameter_locks
            .iter_mut()
            .find(|existing| existing.target == lock.target && existing.parameter == lock.parameter)
        {
            *existing = lock;
        } else {
            cell.parameter_locks.push(lock);
        }
        Ok(())
    }

    pub fn clear_parameter_lock(
        &mut self,
        row: usize,
        track: usize,
        target: &ParameterLockTarget,
        parameter: &ParameterId,
    ) -> Result<(), EditError> {
        let cell = self
            .cell_mut(row, track)
            .ok_or(EditError::CellOutOfBounds { row, track })?;
        cell.parameter_locks
            .retain(|lock| &lock.target != target || &lock.parameter != parameter);
        Ok(())
    }

    #[must_use]
    pub fn parameter_lock_action_at(
        &self,
        row: usize,
        target: &ParameterLockTarget,
        parameter: &ParameterId,
    ) -> Option<&ParameterLockAction> {
        self.rows.get(row).and_then(|row| {
            row.cells
                .iter()
                .flat_map(|cell| cell.parameter_locks.iter())
                .rfind(|lock| &lock.target == target && &lock.parameter == parameter)
                .map(|lock| &lock.action)
        })
    }

    #[must_use]
    pub fn parameter_lock_value_at(
        &self,
        row: usize,
        target: &ParameterLockTarget,
        parameter: &ParameterId,
        default_value: ParameterValue,
    ) -> ParameterValue {
        match self.parameter_lock_action_at(row, target, parameter) {
            Some(ParameterLockAction::Set { value }) => value.clone(),
            Some(ParameterLockAction::Reset) | None => default_value,
        }
    }
}

#[must_use]
pub fn parameter_lock_events(song: &Song, pattern: &Pattern) -> Vec<ParameterLockEvent> {
    let row_duration = row_duration_micros(&song.transport);
    let mut events = Vec::new();
    let mut order = 0;
    for (row_index, row) in pattern.rows.iter().enumerate() {
        let position = PlaybackPosition {
            row: row_index,
            offset_micros: row_duration.saturating_mul(row_index as u64),
        };
        for (track_index, cell) in row.cells.iter().enumerate() {
            let Some(track) = song.tracks.get(track_index) else {
                continue;
            };
            for lock in &cell.parameter_locks {
                events.push(ParameterLockEvent {
                    position,
                    cell_track: track.id,
                    order,
                    lock: lock.clone(),
                });
                order = order.saturating_add(1);
            }
        }
    }
    events.sort_by_key(|event| (event.position.offset_micros, event.order));
    events
}

#[must_use]
pub(crate) fn parameter_lock_f32_at(
    pattern: &Pattern,
    row: usize,
    target: ParameterLockTarget,
    parameter: &str,
    default_value: f32,
) -> f32 {
    pattern
        .parameter_lock_value_at(
            row,
            &target,
            &ParameterId::from(parameter),
            ParameterValue::Float(default_value),
        )
        .as_f32()
        .unwrap_or(default_value)
}

fn descriptor_for_target(
    song: &Song,
    target: &ParameterLockTarget,
    parameter: &ParameterId,
) -> Option<ParameterDescriptor> {
    match target {
        ParameterLockTarget::Sample { sample } => song
            .sample_for_id(*sample)
            .and_then(|_| descriptor_by_id(parameter, &[sample_gain_descriptor()])),
        ParameterLockTarget::Instrument { instrument } => song
            .instrument_for_id(*instrument)
            .and_then(|_| descriptor_by_id(parameter, &[sample_gain_descriptor()])),
        ParameterLockTarget::TrackMixer { track } => song
            .tracks
            .iter()
            .find(|candidate| candidate.id == *track)
            .and_then(|_| {
                descriptor_by_id(
                    parameter,
                    &[mixer_track_gain_descriptor(), mixer_track_pan_descriptor()],
                )
            }),
        ParameterLockTarget::MasterMixer => {
            descriptor_by_id(parameter, &[mixer_master_gain_descriptor()])
        }
        ParameterLockTarget::TrackSend { track, send } => song
            .track_mixer_for_track(*track)
            .sends
            .iter()
            .find(|level| level.send == *send)
            .and_then(|_| descriptor_by_id(parameter, &[mixer_send_gain_descriptor()])),
        ParameterLockTarget::SendBus { send } => song
            .mixer
            .sends
            .iter()
            .find(|candidate| candidate.id == *send)
            .and_then(|_| descriptor_by_id(parameter, &[mixer_send_gain_descriptor()])),
        ParameterLockTarget::TrackEffect { track, device } => song
            .track_mixer_for_track(*track)
            .effects
            .iter()
            .find(|effect| effect.id == *device)
            .and_then(|effect| effect_descriptor(effect, parameter)),
        ParameterLockTarget::MasterEffect { device } => song
            .mixer
            .master_effects
            .iter()
            .find(|effect| effect.id == *device)
            .and_then(|effect| effect_descriptor(effect, parameter)),
    }
}

fn descriptor_by_id(
    parameter: &ParameterId,
    descriptors: &[ParameterDescriptor],
) -> Option<ParameterDescriptor> {
    descriptors
        .iter()
        .find(|descriptor| descriptor.id == *parameter)
        .cloned()
}

fn effect_descriptor(
    effect: &EffectDevice,
    parameter: &ParameterId,
) -> Option<ParameterDescriptor> {
    match effect.kind {
        EffectDeviceKind::Gain { .. }
        | EffectDeviceKind::Pan { .. }
        | EffectDeviceKind::Balance { .. }
        | EffectDeviceKind::StereoWidth { .. }
        | EffectDeviceKind::PhaseInvert { .. }
        | EffectDeviceKind::Filter { .. }
        | EffectDeviceKind::Delay { .. }
        | EffectDeviceKind::Reverb { .. }
        | EffectDeviceKind::Drive { .. }
        | EffectDeviceKind::Bitcrusher { .. }
        | EffectDeviceKind::Chorus { .. }
        | EffectDeviceKind::Flanger { .. }
        | EffectDeviceKind::Phaser { .. } => effect
            .native_module_descriptor()
            .parameter(parameter)
            .cloned(),
    }
}

fn parameter_lock_diagnostic(
    song: &Song,
    pattern_index: usize,
    row_index: usize,
    track_index: usize,
    lock: &ParameterLock,
) -> Option<ParameterLockDiagnostic> {
    let descriptor = descriptor_for_target(song, &lock.target, &lock.parameter);
    let message = match descriptor {
        None => "unknown parameter lock target or parameter".to_string(),
        Some(descriptor) if descriptor.flags.read_only || !descriptor.flags.automatable => {
            "parameter is not automatable".to_string()
        }
        Some(descriptor) => match &lock.action {
            ParameterLockAction::Set { value } => descriptor
                .validate(value)
                .err()
                .map(|error| format!("invalid parameter value: {error}"))?,
            ParameterLockAction::Reset => return None,
        },
    };
    Some(ParameterLockDiagnostic {
        pattern_index,
        row_index,
        track_index,
        target: lock.target.clone(),
        parameter: lock.parameter.clone(),
        message,
    })
}

impl From<crate::parameters::ParameterValidationError> for EditError {
    fn from(_: crate::parameters::ParameterValidationError) -> Self {
        Self::InvalidParameterValue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        EffectDevice, MixerSend, ParameterValue, Song, TrackId, TrackSendLevel,
        MIXER_SEND_GAIN_PARAMETER_ID, NATIVE_GAIN_PARAMETER_ID, NATIVE_PAN_PARAMETER_ID,
    };

    #[test]
    fn parameter_lock_events_are_ordered_and_cover_supported_targets() {
        let mut song = Song::empty();
        let sample_id = song.upsert_sample_reference("samples/kick.wav", "kick.wav");
        let track_id = song.tracks[0].id;
        song.assign_sample_to_track(track_id, sample_id)
            .expect("assign sample");
        song.mixer.sends.push(MixerSend {
            id: 1,
            name: "Send A".to_string(),
        });
        song.mixer.tracks[0]
            .sends
            .push(TrackSendLevel { send: 1, gain: 0.0 });
        song.mixer.tracks[0]
            .effects
            .push(EffectDevice::gain(1, 1.0));
        song.mixer.master_effects.push(EffectDevice::pan(2, 0.0));
        let pattern = song.current_pattern_mut().expect("pattern");
        pattern
            .set_parameter_lock(
                2,
                1,
                ParameterLock {
                    target: ParameterLockTarget::TrackSend {
                        track: track_id,
                        send: 1,
                    },
                    parameter: ParameterId::from(MIXER_SEND_GAIN_PARAMETER_ID),
                    action: ParameterLockAction::Set {
                        value: ParameterValue::Float(0.25),
                    },
                },
            )
            .expect("send lock");
        pattern
            .set_parameter_lock(
                2,
                0,
                ParameterLock {
                    target: ParameterLockTarget::TrackEffect {
                        track: track_id,
                        device: 1,
                    },
                    parameter: ParameterId::from(NATIVE_GAIN_PARAMETER_ID),
                    action: ParameterLockAction::Set {
                        value: ParameterValue::Float(0.75),
                    },
                },
            )
            .expect("track effect lock");
        pattern
            .set_parameter_lock(
                2,
                2,
                ParameterLock {
                    target: ParameterLockTarget::MasterEffect { device: 2 },
                    parameter: ParameterId::from(NATIVE_PAN_PARAMETER_ID),
                    action: ParameterLockAction::Reset,
                },
            )
            .expect("master effect lock");

        let events = parameter_lock_events(&song, song.current_pattern().expect("pattern"));

        assert_eq!(events.len(), 3);
        assert_eq!(events[0].cell_track, TrackId(1));
        assert_eq!(events[1].cell_track, TrackId(2));
        assert_eq!(events[2].cell_track, TrackId(3));
        assert_eq!(events[0].position.row, 2);
        assert_eq!(events[0].order, 0);
        assert!(song.parameter_lock_diagnostics().is_empty());
    }
}
