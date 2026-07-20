use salieri_sampler::PreviewBuffer;

use crate::{
    fixtures::{assert_approx_eq, mono_sample},
    *,
};

mod dynamics;
mod send_routing;

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
        sends: Vec::new(),
        track_sends: Vec::new(),
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
        sends: Vec::new(),
        track_sends: Vec::new(),
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
        sends: Vec::new(),
        track_sends: Vec::new(),
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
            sends: Vec::new(),
            track_sends: Vec::new(),
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
fn renders_sampler_events_through_native_drive_modes() {
    let samples = vec![OfflineSamplerSample {
        sample_id: 7,
        buffer: PreviewBuffer {
            sample_rate: 48_000,
            channels: 2,
            frames: 4,
            data: vec![0.20, -0.20, 0.50, -0.50, 0.80, -0.80, 1.0, -1.0],
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
        DspDriveMode::Overdrive,
        DspDriveMode::Saturation,
        DspDriveMode::HardClip,
        DspDriveMode::SoftClip,
    ] {
        let graph = DspGraphSpec {
            sends: Vec::new(),
            track_sends: Vec::new(),
            track_chains: Vec::new(),
            master: vec![DspDeviceSpec {
                bypassed: false,
                kind: DspDeviceKind::Drive {
                    mode,
                    drive_db: 18.0,
                    tone: 0.5,
                    bias: 0.0,
                    mix: 1.0,
                    output_db: -6.0,
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
        .expect("render drive");

        assert!(rendered.data.iter().all(|sample| sample.is_finite()));
        assert_ne!(rendered.data, samples[0].buffer.data);
    }
}

#[test]
fn renders_sampler_events_through_native_bitcrusher_reduction() {
    let samples = vec![OfflineSamplerSample {
        sample_id: 7,
        buffer: PreviewBuffer {
            sample_rate: 48_000,
            channels: 2,
            frames: 4,
            data: vec![0.10, -0.10, 0.40, -0.40, 0.70, -0.70, 1.0, -1.0],
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
        sends: Vec::new(),
        track_sends: Vec::new(),
        track_chains: Vec::new(),
        master: vec![DspDeviceSpec {
            bypassed: false,
            kind: DspDeviceKind::Bitcrusher {
                bit_depth: 4,
                reduction_ratio: 2.0,
                dither: false,
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
            frames: 4,
        },
        &graph,
    )
    .expect("render bitcrusher");

    assert!(rendered.data.iter().all(|sample| sample.is_finite()));
    assert_approx_eq(rendered.data[0], rendered.data[2]);
    assert_approx_eq(rendered.data[1], rendered.data[3]);
    assert_ne!(rendered.data, samples[0].buffer.data);
}

#[test]
fn renders_sampler_events_through_native_modulation_effects() {
    let samples = vec![OfflineSamplerSample {
        sample_id: 7,
        buffer: PreviewBuffer {
            sample_rate: 48_000,
            channels: 2,
            frames: 64,
            data: (0..64)
                .flat_map(|frame| {
                    let value = (frame as f32 / 64.0).sin();
                    [value, -value]
                })
                .collect(),
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

    for kind in [
        DspDeviceKind::Chorus {
            rate_hz: 0.5,
            sync: false,
            depth: 0.75,
            delay_ms: 12.0,
            voices: 2,
            spread: 1.0,
            feedback: 0.1,
            mix: 0.5,
            output_db: 0.0,
        },
        DspDeviceKind::Flanger {
            rate_hz: 0.5,
            sync: true,
            depth: 0.75,
            manual: 0.5,
            delay_ms: 3.0,
            feedback: 0.25,
            stereo_phase: 1.0,
            mix: 0.5,
            output_db: 0.0,
        },
        DspDeviceKind::Phaser {
            rate_hz: 0.5,
            sync: false,
            depth: 0.75,
            center_hz: 1_000.0,
            stages: 4,
            feedback: 0.25,
            stereo_phase: 1.0,
            mix: 0.5,
            output_db: 0.0,
        },
    ] {
        let rendered = render_sampler_events_with_dsp(
            &samples,
            &events,
            OfflineRenderSpec {
                sample_rate: 48_000,
                channels: 2,
                frames: 64,
            },
            &DspGraphSpec {
                sends: Vec::new(),
                track_sends: Vec::new(),
                track_chains: Vec::new(),
                master: vec![DspDeviceSpec {
                    bypassed: false,
                    kind,
                }],
            },
        )
        .expect("render modulation");

        assert!(rendered.data.iter().all(|sample| sample.is_finite()));
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
        sends: Vec::new(),
        track_sends: Vec::new(),
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
fn renders_sampler_events_through_native_reverb_tail() {
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
        sends: Vec::new(),
        track_sends: Vec::new(),
        track_chains: Vec::new(),
        master: vec![DspDeviceSpec {
            bypassed: false,
            kind: test_reverb_kind(),
        }],
    };

    let rendered = render_sampler_events_with_dsp(
        &samples,
        &events,
        OfflineRenderSpec {
            sample_rate: 48_000,
            channels: 2,
            frames: 2_048,
        },
        &graph,
    )
    .expect("render reverb");

    assert_eq!(rendered.data[0], 0.0);
    assert!(rendered.data.iter().all(|sample| sample.is_finite()));
    assert!(
        rendered.data[2..]
            .iter()
            .any(|sample| sample.abs() > 0.000_1),
        "reverb should produce tail after the input frame"
    );
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
        sends: Vec::new(),
        track_sends: Vec::new(),
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

fn test_reverb_kind() -> DspDeviceKind {
    DspDeviceKind::Reverb {
        size: 0.5,
        predelay_ms: 0.0,
        decay_s: 1.0,
        damping: 0.5,
        low_cut_hz: 100.0,
        high_cut_hz: 16_000.0,
        diffusion: 0.75,
        width: 1.0,
        early_reflections: 0.5,
        mix: 1.0,
        output_db: 0.0,
    }
}
