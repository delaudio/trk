use crate::{PatternCell, SampleReference, Song, TrackId};

pub(super) fn sample_for_cell<'a>(
    song: &'a Song,
    cell: &PatternCell,
    track: TrackId,
    pitch: u8,
    velocity: u8,
) -> Option<&'a SampleReference> {
    if let Some(instrument_id) = cell.instrument {
        return song
            .instrument_for_id(instrument_id)
            .and_then(|instrument| instrument.sample_for_note(pitch, velocity))
            .and_then(|sample| song.sample_for_id(sample));
    }
    song.sample_for_track(track)
}
