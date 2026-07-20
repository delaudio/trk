use super::*;

#[test]
fn renders_sampler_events_with_pitch_ratio() {
    let samples = vec![OfflineSamplerSample {
        sample_id: 1,
        buffer: mono_sample(vec![0.25, 0.5, 0.75, 1.0]),
    }];
    let events = vec![OfflineSamplerEvent {
        track_id: 1,
        sample_id: 1,
        frame: 0,
        gain: 1.0,
        pan: 0.0,
        pitch_ratio: 2.0,
        velocity: 127,
        playback: AudioSamplerPlaybackSettings::default(),
    }];

    let rendered = render_sampler_events(
        &samples,
        &events,
        OfflineRenderSpec {
            sample_rate: 48_000,
            channels: 1,
            frames: 0,
        },
    )
    .expect("render");

    assert_eq!(rendered.frames, 2);
    assert_eq!(rendered.data, vec![0.25, 0.75]);
}

#[test]
fn renders_sampler_events_with_reverse_playback() {
    let samples = vec![OfflineSamplerSample {
        sample_id: 7,
        buffer: mono_sample(vec![0.0, 1.0, 2.0, 3.0]),
    }];
    let events = vec![OfflineSamplerEvent {
        track_id: 1,
        sample_id: 7,
        frame: 0,
        gain: 1.0,
        pan: 0.0,
        pitch_ratio: 1.0,
        velocity: 127,
        playback: AudioSamplerPlaybackSettings {
            mode: AudioSamplerPlaybackMode::Reverse,
            ..AudioSamplerPlaybackSettings::default()
        },
    }];

    let rendered = render_sampler_events(
        &samples,
        &events,
        OfflineRenderSpec {
            sample_rate: 48_000,
            channels: 1,
            frames: 4,
        },
    )
    .expect("render");

    assert_eq!(rendered.data, vec![3.0, 2.0, 1.0, 0.0]);
}

#[test]
fn renders_sampler_events_with_forward_backward_and_pingpong_loops() {
    let samples = vec![OfflineSamplerSample {
        sample_id: 7,
        buffer: mono_sample(vec![0.0, 1.0, 2.0, 3.0]),
    }];
    let loop_window = AudioSamplerPlaybackSettings {
        mode: AudioSamplerPlaybackMode::ForwardLoop,
        loop_start_frame: Some(1),
        loop_end_frame: Some(4),
    };

    let render_mode = |mode| {
        render_sampler_events(
            &samples,
            &[OfflineSamplerEvent {
                track_id: 1,
                sample_id: 7,
                frame: 0,
                gain: 1.0,
                pan: 0.0,
                pitch_ratio: 1.0,
                velocity: 127,
                playback: AudioSamplerPlaybackSettings {
                    mode,
                    ..loop_window
                },
            }],
            OfflineRenderSpec {
                sample_rate: 48_000,
                channels: 1,
                frames: 8,
            },
        )
        .expect("render")
        .data
    };

    assert_eq!(
        render_mode(AudioSamplerPlaybackMode::ForwardLoop),
        vec![0.0, 1.0, 2.0, 3.0, 1.0, 2.0, 3.0, 1.0]
    );
    assert_eq!(
        render_mode(AudioSamplerPlaybackMode::BackwardLoop),
        vec![0.0, 1.0, 2.0, 3.0, 3.0, 2.0, 1.0, 3.0]
    );
    assert_eq!(
        render_mode(AudioSamplerPlaybackMode::PingPongLoop),
        vec![0.0, 1.0, 2.0, 3.0, 2.0, 1.0, 2.0, 3.0]
    );
}
