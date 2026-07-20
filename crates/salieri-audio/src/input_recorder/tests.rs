use super::*;

fn test_input() -> AudioInputDeviceInfo {
    AudioInputDeviceInfo {
        id: "fake".to_string(),
        name: "Fake Input".to_string(),
        channels: 2,
        sample_rate: 48_000,
    }
}

#[test]
fn recorder_captures_bounds_peak_and_trimmed_audio() {
    let mut recorder = SampleRecorder::default();
    recorder.select_input(test_input()).expect("select input");
    recorder.set_gain(0.5).expect("set gain");
    recorder.start(3).expect("start");

    assert_eq!(recorder.status(), SampleRecorderStatus::Recording);
    assert_eq!(
        recorder
            .push_input_interleaved(&[0.0, 0.5, 2.0, -2.0, 0.25, -0.25, 0.1, 0.1])
            .expect("push"),
        3
    );
    assert_eq!(recorder.status(), SampleRecorderStatus::Recorded);
    assert_eq!(recorder.recorded_frames(), 3);
    assert_eq!(recorder.peak(), 1.0);

    recorder.trim(1, 3).expect("trim");
    let audio = recorder.rendered_audio().expect("render");
    assert_eq!(audio.sample_rate, 48_000);
    assert_eq!(audio.channels, 2);
    assert_eq!(audio.frames, 2);
    assert_eq!(audio.data, vec![1.0, -1.0, 0.125, -0.125]);
}

#[test]
fn recorder_rejects_invalid_state_transitions() {
    let mut recorder = SampleRecorder::default();
    assert!(matches!(
        recorder.start(1),
        Err(AudioInputError::InvalidState { .. })
    ));
    assert!(matches!(
        recorder.push_input_interleaved(&[0.0, 0.0]),
        Err(AudioInputError::InvalidState { .. })
    ));

    recorder.select_input(test_input()).expect("select input");
    recorder.start(2).expect("start");
    assert!(matches!(
        recorder.select_input(test_input()),
        Err(AudioInputError::InvalidState { .. })
    ));
    recorder.stop().expect("stop");
    assert!(matches!(
        recorder.trim(2, 1),
        Err(AudioInputError::InvalidTrimRange)
    ));
}

#[test]
fn recorder_loads_fake_capture_for_headless_tests() {
    let mut recorder = SampleRecorder::default();
    recorder
        .load_recorded_audio(
            RenderedAudio {
                sample_rate: 44_100,
                channels: 1,
                frames: 4,
                data: vec![0.0, 0.25, -0.5, 0.75],
            },
            None,
        )
        .expect("load capture");

    assert_eq!(recorder.status(), SampleRecorderStatus::Recorded);
    assert_eq!(recorder.recorded_frames(), 4);
    assert_eq!(recorder.trim_range(), (0, 4));
    assert_eq!(recorder.peak(), 0.75);
}
