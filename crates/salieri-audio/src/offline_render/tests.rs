use salieri_sampler::PreviewBuffer;

use crate::{
    fixtures::{assert_approx_eq, mono_sample},
    *,
};

#[test]
fn renders_sampler_preview_deterministically() {
    let preview = PreviewBuffer {
        sample_rate: 48_000,
        channels: 2,
        frames: 2,
        data: vec![0.25, -0.25, 0.5, -0.5],
    };
    let spec = OfflineRenderSpec {
        sample_rate: 48_000,
        channels: 2,
        frames: 4,
    };

    let first = render_sampler_preview(&preview, spec).expect("render");
    let second = render_sampler_preview(&preview, spec).expect("render");

    assert_eq!(first, second);
    assert_eq!(first.frames, 4);
    assert_eq!(first.data, vec![0.25, -0.25, 0.5, -0.5, 0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn renders_sampler_events_with_timing_gain_and_velocity() {
    let samples = vec![OfflineSamplerSample {
        sample_id: 7,
        buffer: mono_sample(vec![1.0, 0.5]),
    }];
    let events = vec![OfflineSamplerEvent {
        track_id: 1,
        sample_id: 7,
        frame: 2,
        gain: 0.5,
        pan: 0.0,
        pitch_ratio: 1.0,
        velocity: 64,
    }];

    let rendered = render_sampler_events(
        &samples,
        &events,
        OfflineRenderSpec {
            sample_rate: 48_000,
            channels: 1,
            frames: 5,
        },
    )
    .expect("render");

    assert_eq!(rendered.frames, 5);
    assert_eq!(rendered.data[0], 0.0);
    assert_eq!(rendered.data[1], 0.0);
    assert_approx_eq(rendered.data[2], 0.5 * (64.0 / 127.0));
    assert_approx_eq(rendered.data[3], 0.25 * (64.0 / 127.0));
    assert_eq!(rendered.data[4], 0.0);
}

#[test]
fn renders_sampler_events_with_linear_stereo_pan() {
    let samples = vec![OfflineSamplerSample {
        sample_id: 7,
        buffer: PreviewBuffer {
            sample_rate: 48_000,
            channels: 2,
            frames: 1,
            data: vec![1.0, 1.0],
        },
    }];
    let events = vec![OfflineSamplerEvent {
        track_id: 1,
        sample_id: 7,
        frame: 0,
        gain: 1.0,
        pan: 0.75,
        pitch_ratio: 1.0,
        velocity: 127,
    }];

    let rendered = render_sampler_events(
        &samples,
        &events,
        OfflineRenderSpec {
            sample_rate: 48_000,
            channels: 2,
            frames: 1,
        },
    )
    .expect("render");

    assert_approx_eq(rendered.data[0], 0.25);
    assert_approx_eq(rendered.data[1], 1.0);
}

#[test]
fn renders_sampler_events_through_track_and_master_dsp() {
    let samples = vec![OfflineSamplerSample {
        sample_id: 7,
        buffer: mono_sample(vec![1.0]),
    }];
    let events = vec![OfflineSamplerEvent {
        track_id: 2,
        sample_id: 7,
        frame: 0,
        gain: 1.0,
        pan: 0.0,
        pitch_ratio: 1.0,
        velocity: 127,
    }];
    let graph = DspGraphSpec {
        track_chains: vec![TrackDspChainSpec {
            track_id: 2,
            devices: vec![DspDeviceSpec {
                bypassed: false,
                kind: DspDeviceKind::Gain { gain: 0.5 },
            }],
        }],
        master: vec![DspDeviceSpec {
            bypassed: false,
            kind: DspDeviceKind::Gain { gain: 0.25 },
        }],
    };

    let rendered = render_sampler_events_with_dsp(
        &samples,
        &events,
        OfflineRenderSpec {
            sample_rate: 48_000,
            channels: 1,
            frames: 1,
        },
        &graph,
    )
    .expect("render");

    assert_approx_eq(rendered.data[0], 0.125);
}

#[test]
fn renders_sampler_events_through_native_utility_master_devices() {
    let samples = vec![OfflineSamplerSample {
        sample_id: 7,
        buffer: PreviewBuffer {
            sample_rate: 48_000,
            channels: 2,
            frames: 1,
            data: vec![1.0, 0.0],
        },
    }];
    let events = vec![OfflineSamplerEvent {
        track_id: 2,
        sample_id: 7,
        frame: 0,
        gain: 1.0,
        pan: 0.0,
        pitch_ratio: 1.0,
        velocity: 127,
    }];
    let graph = DspGraphSpec {
        track_chains: Vec::new(),
        master: vec![
            DspDeviceSpec {
                bypassed: false,
                kind: DspDeviceKind::StereoWidth { width: 2.0 },
            },
            DspDeviceSpec {
                bypassed: false,
                kind: DspDeviceKind::PhaseInvert {
                    invert_left: true,
                    invert_right: false,
                },
            },
        ],
    };

    let rendered = render_sampler_events_with_dsp(
        &samples,
        &events,
        OfflineRenderSpec {
            sample_rate: 48_000,
            channels: 2,
            frames: 1,
        },
        &graph,
    )
    .expect("render");

    assert_approx_eq(rendered.data[0], -1.5);
    assert_approx_eq(rendered.data[1], -0.5);
}

#[test]
fn renders_sampler_events_through_native_utility_track_devices() {
    let samples = vec![OfflineSamplerSample {
        sample_id: 7,
        buffer: PreviewBuffer {
            sample_rate: 48_000,
            channels: 2,
            frames: 1,
            data: vec![1.0, 0.0],
        },
    }];
    let events = vec![OfflineSamplerEvent {
        track_id: 2,
        sample_id: 7,
        frame: 0,
        gain: 1.0,
        pan: 0.0,
        pitch_ratio: 1.0,
        velocity: 127,
    }];
    let graph = DspGraphSpec {
        track_chains: vec![TrackDspChainSpec {
            track_id: 2,
            devices: vec![
                DspDeviceSpec {
                    bypassed: false,
                    kind: DspDeviceKind::StereoWidth { width: 2.0 },
                },
                DspDeviceSpec {
                    bypassed: false,
                    kind: DspDeviceKind::PhaseInvert {
                        invert_left: true,
                        invert_right: false,
                    },
                },
            ],
        }],
        master: Vec::new(),
    };

    let rendered = render_sampler_events_with_dsp(
        &samples,
        &events,
        OfflineRenderSpec {
            sample_rate: 48_000,
            channels: 2,
            frames: 1,
        },
        &graph,
    )
    .expect("render");

    assert_approx_eq(rendered.data[0], -1.5);
    assert_approx_eq(rendered.data[1], -0.5);
}

#[test]
fn renders_sampler_events_through_native_filter_modes() {
    let samples = vec![OfflineSamplerSample {
        sample_id: 7,
        buffer: PreviewBuffer {
            sample_rate: 48_000,
            channels: 2,
            frames: 4,
            data: vec![1.0, -1.0, 0.5, -0.5, 0.25, -0.25, 0.0, 0.0],
        },
    }];
    let events = vec![OfflineSamplerEvent {
        track_id: 2,
        sample_id: 7,
        frame: 0,
        gain: 1.0,
        pan: 0.0,
        pitch_ratio: 1.0,
        velocity: 127,
    }];

    for mode in [
        DspFilterMode::LowPass,
        DspFilterMode::HighPass,
        DspFilterMode::BandPass,
        DspFilterMode::Notch,
    ] {
        let graph = DspGraphSpec {
            track_chains: Vec::new(),
            master: vec![DspDeviceSpec {
                bypassed: false,
                kind: DspDeviceKind::Filter {
                    mode,
                    cutoff_hz: 2_000.0,
                    resonance: 0.5,
                    drive_db: 3.0,
                    key_track: 0.0,
                    env_amount: 0.0,
                    mix: 1.0,
                },
            }],
        };

        let rendered = render_sampler_events_with_dsp(
            &samples,
            &events,
            OfflineRenderSpec {
                sample_rate: 48_000,
                channels: 2,
                frames: 4,
            },
            &graph,
        )
        .expect("render filter");

        assert!(rendered.data.iter().all(|sample| sample.is_finite()));
        assert_ne!(rendered.data, samples[0].buffer.data);
    }
}

#[test]
fn renders_sampler_events_through_native_delay_timing() {
    let samples = vec![OfflineSamplerSample {
        sample_id: 7,
        buffer: PreviewBuffer {
            sample_rate: 48_000,
            channels: 2,
            frames: 1,
            data: vec![1.0, 0.0],
        },
    }];
    let events = vec![OfflineSamplerEvent {
        track_id: 2,
        sample_id: 7,
        frame: 0,
        gain: 1.0,
        pan: 0.0,
        pitch_ratio: 1.0,
        velocity: 127,
    }];
    let graph = DspGraphSpec {
        track_chains: Vec::new(),
        master: vec![DspDeviceSpec {
            bypassed: false,
            kind: DspDeviceKind::Delay {
                sync: false,
                time_left_ms: 1.0,
                time_right_ms: 1.0,
                link_times: true,
                feedback: 0.0,
                ping_pong: false,
                filter_low_cut_hz: 20.0,
                filter_high_cut_hz: 20_000.0,
                mod_rate_hz: 0.0,
                mod_depth: 0.0,
                mix: 1.0,
                output_db: 0.0,
            },
        }],
    };

    let rendered = render_sampler_events_with_dsp(
        &samples,
        &events,
        OfflineRenderSpec {
            sample_rate: 48_000,
            channels: 2,
            frames: 64,
        },
        &graph,
    )
    .expect("render delay");

    assert_eq!(rendered.data[0], 0.0);
    assert_approx_eq(rendered.data[48 * 2], 1.0);
    assert_eq!(rendered.data[48 * 2 + 1], 0.0);
    assert!(rendered.data.iter().all(|sample| sample.is_finite()));
}

#[test]
fn bypassed_dsp_devices_do_not_process_audio() {
    let samples = vec![OfflineSamplerSample {
        sample_id: 7,
        buffer: mono_sample(vec![1.0]),
    }];
    let events = vec![OfflineSamplerEvent {
        track_id: 2,
        sample_id: 7,
        frame: 0,
        gain: 1.0,
        pan: 0.0,
        pitch_ratio: 1.0,
        velocity: 127,
    }];
    let graph = DspGraphSpec {
        track_chains: vec![TrackDspChainSpec {
            track_id: 2,
            devices: vec![DspDeviceSpec {
                bypassed: true,
                kind: DspDeviceKind::Gain { gain: 0.0 },
            }],
        }],
        master: vec![DspDeviceSpec {
            bypassed: true,
            kind: DspDeviceKind::Gain { gain: 0.0 },
        }],
    };

    let rendered = render_sampler_events_with_dsp(
        &samples,
        &events,
        OfflineRenderSpec {
            sample_rate: 48_000,
            channels: 1,
            frames: 1,
        },
        &graph,
    )
    .expect("render");

    assert_approx_eq(rendered.data[0], 1.0);
}

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
fn renders_sampler_events_as_silence_without_events() {
    let rendered = render_sampler_events(
        &[],
        &[],
        OfflineRenderSpec {
            sample_rate: 48_000,
            channels: 2,
            frames: 3,
        },
    )
    .expect("render");

    assert_eq!(rendered.frames, 3);
    assert_eq!(rendered.data, vec![0.0; 6]);
}

#[test]
fn sampler_event_render_failures_are_clear() {
    let samples = vec![OfflineSamplerSample {
        sample_id: 1,
        buffer: mono_sample(vec![0.25]),
    }];

    assert!(matches!(
        render_sampler_events(
            &samples,
            &[OfflineSamplerEvent {
                track_id: 1,
                sample_id: 99,
                frame: 0,
                gain: 1.0,
                pan: 0.0,
                pitch_ratio: 1.0,
                velocity: 127,
            }],
            OfflineRenderSpec {
                sample_rate: 48_000,
                channels: 1,
                frames: 1,
            },
        ),
        Err(AudioExportError::MissingSample { sample_id: 99 })
    ));

    assert!(matches!(
        render_sampler_events(
            &samples,
            &[OfflineSamplerEvent {
                track_id: 1,
                sample_id: 1,
                frame: 0,
                gain: 1.0,
                pan: 0.0,
                pitch_ratio: 0.0,
                velocity: 127,
            }],
            OfflineRenderSpec {
                sample_rate: 48_000,
                channels: 1,
                frames: 1,
            },
        ),
        Err(AudioExportError::InvalidPitchRatio { pitch_ratio: 0.0 })
    ));
}
