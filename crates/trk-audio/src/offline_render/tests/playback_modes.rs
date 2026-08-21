use super::*;
use crate::offline_render::finite_playback_frames;

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
        ..AudioSamplerPlaybackSettings::default()
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

#[test]
fn loop_modes_apply_nonzero_playback_start_exactly_once() {
    let samples = vec![OfflineSamplerSample {
        sample_id: 7,
        buffer: mono_sample(vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0]),
    }];
    let playback = AudioSamplerPlaybackSettings {
        start_frame: Some(2),
        end_frame: Some(6),
        loop_start_frame: Some(3),
        loop_end_frame: Some(6),
        ..AudioSamplerPlaybackSettings::default()
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
                playback: AudioSamplerPlaybackSettings { mode, ..playback },
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
        vec![2.0, 3.0, 4.0, 5.0, 3.0, 4.0, 5.0, 3.0]
    );
    assert_eq!(
        render_mode(AudioSamplerPlaybackMode::BackwardLoop),
        vec![2.0, 3.0, 4.0, 5.0, 5.0, 4.0, 3.0, 5.0]
    );
    assert_eq!(
        render_mode(AudioSamplerPlaybackMode::PingPongLoop),
        vec![2.0, 3.0, 4.0, 5.0, 4.0, 3.0, 4.0, 5.0]
    );
}

#[test]
fn malformed_loop_window_falls_back_to_finite_playback() {
    let samples = vec![OfflineSamplerSample {
        sample_id: 7,
        buffer: mono_sample(vec![1.0, 2.0, 3.0, 4.0]),
    }];
    let rendered = render_sampler_events(
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
                mode: AudioSamplerPlaybackMode::ForwardLoop,
                loop_start_frame: Some(3),
                loop_end_frame: Some(2),
                ..AudioSamplerPlaybackSettings::default()
            },
        }],
        OfflineRenderSpec {
            sample_rate: 48_000,
            channels: 1,
            frames: 8,
        },
    )
    .expect("render");

    assert_eq!(rendered.data, vec![1.0, 2.0, 3.0, 4.0, 0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn normalizes_playback_windows_against_the_registered_buffer() {
    let samples = vec![OfflineSamplerSample {
        sample_id: 7,
        buffer: mono_sample(vec![0.0, 1.0, 2.0, 3.0]),
    }];
    let render_window = |start_frame, end_frame| {
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
                    start_frame,
                    end_frame,
                    ..AudioSamplerPlaybackSettings::default()
                },
            }],
            OfflineRenderSpec {
                sample_rate: 48_000,
                channels: 1,
                frames: 4,
            },
        )
        .expect("render")
        .data
    };

    assert_eq!(render_window(Some(1), Some(3)), vec![1.0, 2.0, 0.0, 0.0]);
    assert_eq!(render_window(Some(99), Some(1)), vec![3.0, 0.0, 0.0, 0.0]);
}

#[test]
fn applies_the_playback_envelope_at_render_time() {
    let rendered = render_sampler_events(
        &[OfflineSamplerSample {
            sample_id: 7,
            buffer: mono_sample(vec![1.0; 4]),
        }],
        &[OfflineSamplerEvent {
            track_id: 1,
            sample_id: 7,
            frame: 0,
            gain: 1.0,
            pan: 0.0,
            pitch_ratio: 1.0,
            velocity: 127,
            playback: AudioSamplerPlaybackSettings {
                attack_seconds: 2.0 / 48_000.0,
                release_seconds: 2.0 / 48_000.0,
                ..AudioSamplerPlaybackSettings::default()
            },
        }],
        OfflineRenderSpec {
            sample_rate: 48_000,
            channels: 1,
            frames: 4,
        },
    )
    .expect("render");

    assert_eq!(rendered.data, vec![0.0, 0.5, 1.0, 0.5]);
}

#[test]
fn finite_playback_rejects_invalid_pitch_ratios_defensively() {
    let sample = mono_sample(vec![1.0; 4]);
    for pitch_ratio in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert_eq!(
            finite_playback_frames(
                &sample,
                AudioSamplerPlaybackSettings::default(),
                pitch_ratio,
            ),
            Some(0)
        );
    }
}
