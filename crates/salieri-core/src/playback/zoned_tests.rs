use crate::{playback::sampler_events, InstrumentSampleZone, NoteEvent, Song};

#[test]
fn sampler_events_select_instrument_zone_by_pitch_and_velocity() {
    let mut song = Song::empty();
    let low = song.upsert_sample_reference("samples/low.wav", "low.wav");
    let high = song.upsert_sample_reference("samples/high.wav", "high.wav");
    let instrument = song.upsert_sample_instrument(low).expect("instrument");
    song.instruments
        .iter_mut()
        .find(|candidate| candidate.id == instrument)
        .expect("instrument")
        .zones = vec![
        InstrumentSampleZone {
            sample: low,
            key_start: 0,
            key_end: 59,
            velocity_start: 0,
            velocity_end: 127,
        },
        InstrumentSampleZone {
            sample: high,
            key_start: 60,
            key_end: 127,
            velocity_start: 64,
            velocity_end: 127,
        },
    ];
    let pattern = song.current_pattern_mut().expect("pattern");
    pattern
        .set_note(0, 0, NoteEvent::Note { pitch: 72 }, 100)
        .expect("set note");
    pattern.cell_mut(0, 0).expect("cell").instrument = Some(instrument);

    let events = sampler_events(&song, song.current_pattern().expect("pattern"));

    assert_eq!(events[0].sample, high);
    assert_eq!(events[0].sample_path, "samples/high.wav");
}
