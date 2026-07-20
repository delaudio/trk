use super::*;

#[test]
fn performance_punch_in_applies_to_playback_clone_and_releases() {
    let mut app = App::default();
    let track_id = app.song.tracks[1].id;
    app.song
        .set_track_mixer_gain(1, 0.75)
        .expect("baseline gain");

    type_command(&mut app, "performance slot 1 track 2 gain 0.250");
    type_command(&mut app, "performance punch 1");

    let playback_song = app.performance_playback_song();
    assert_eq!(playback_song.track_mixer_for_track(track_id).gain, 0.25);
    assert_eq!(app.song.track_mixer_for_track(track_id).gain, 0.75);
    assert!(!app.dirty);

    type_command(&mut app, "performance release 1");
    let playback_song = app.performance_playback_song();
    assert_eq!(playback_song.track_mixer_for_track(track_id).gain, 0.75);
    assert_eq!(app.song.track_mixer_for_track(track_id).gain, 0.75);
}

#[test]
fn performance_sample_gain_punch_in_does_not_mutate_project_sample() {
    let mut app = App::default();
    let sample = app.song.upsert_sample_reference("samples/kick.wav", "Kick");
    app.song
        .assign_sample_to_track(app.song.tracks[0].id, sample)
        .expect("assign sample");

    type_command(&mut app, "performance slot 1 sample-gain 0.500");
    type_command(&mut app, "performance punch 1");

    let playback_song = app.performance_playback_song();
    assert_eq!(
        playback_song.sample_for_id(sample).expect("sample").gain,
        0.5
    );
    assert_eq!(app.song.sample_for_id(sample).expect("sample").gain, 1.0);
}
