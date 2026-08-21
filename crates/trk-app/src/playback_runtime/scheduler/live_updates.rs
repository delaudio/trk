use super::*;

pub(super) fn remap_active_pattern(
    song: &Song,
    active_pattern_id: trk_core::PatternId,
    sequence_index: Option<usize>,
) -> Option<usize> {
    let target_id = match sequence_index {
        Some(index) => *song.sequence.get(index)?,
        None => active_pattern_id,
    };
    song.patterns
        .iter()
        .position(|pattern| pattern.id == target_id)
}

pub(super) fn apply_live_mute(
    current: &mut Song,
    track_id: trk_core::TrackId,
    muted: bool,
    row: usize,
    active_notes: &mut Vec<(trk_core::TrackId, u8)>,
    context: &mut PlaybackRunContext<'_>,
) -> Result<(), MidiError> {
    let Some(track) = current.tracks.iter_mut().find(|track| track.id == track_id) else {
        return Ok(());
    };
    let newly_muted = muted && !track.muted;
    track.muted = muted;
    if !newly_muted {
        return Ok(());
    }
    let midi_channel = track.midi_channel;
    context.audio_output.send(RealtimeAudioCommand::StopTrack {
        track_id: track_id.0,
    });
    let mut retained = Vec::with_capacity(active_notes.len());
    for (active_track, pitch) in active_notes.drain(..) {
        if active_track != track_id {
            retained.push((active_track, pitch));
            continue;
        }
        send_playback_event_logged(
            context.output,
            context.midi_logger,
            PlaybackEvent {
                position: PlaybackPosition {
                    row,
                    offset_micros: 0,
                },
                track: track_id,
                midi_channel,
                kind: PlaybackEventKind::NoteOff { pitch },
            },
        )?;
    }
    *active_notes = retained;
    Ok(())
}
