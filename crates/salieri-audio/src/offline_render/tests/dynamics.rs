use super::*;

#[test]
fn renders_sampler_events_through_native_dynamics_effects() {
    let samples = vec![OfflineSamplerSample {
        sample_id: 7,
        buffer: PreviewBuffer {
            sample_rate: 48_000,
            channels: 2,
            frames: 4,
            data: vec![0.001, 0.001, 0.8, -0.8, 1.2, -1.2, 0.02, 0.02],
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
                    kind: DspDeviceKind::Compressor {
                        threshold_db: -18.0,
                        ratio: 6.0,
                        attack_ms: 0.01,
                        release_ms: 50.0,
                        knee_db: 3.0,
                        makeup_db: 0.0,
                        auto_makeup: false,
                        detector: DspDynamicsDetector::Peak,
                        stereo_link: 1.0,
                        mix: 1.0,
                    },
                },
                DspDeviceSpec {
                    bypassed: false,
                    kind: DspDeviceKind::Gate {
                        threshold_db: -50.0,
                        hysteresis_db: 3.0,
                        attack_ms: 0.01,
                        hold_ms: 0.0,
                        release_ms: 20.0,
                        range_db: 40.0,
                        detector: DspDynamicsDetector::Peak,
                        stereo_link: 1.0,
                    },
                },
            ],
        }],
        master: vec![DspDeviceSpec {
            bypassed: false,
            kind: DspDeviceKind::Limiter {
                ceiling_db: -1.0,
                input_gain_db: 6.0,
                release_ms: 25.0,
                lookahead_ms: 0.0,
                stereo_link: 1.0,
                true_peak: false,
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
    .expect("render dynamics");

    let ceiling = 10.0_f32.powf(-1.0 / 20.0);
    assert!(rendered.data.iter().all(|sample| sample.is_finite()));
    assert!(rendered
        .data
        .iter()
        .all(|sample| sample.abs() <= ceiling + 0.000_1));
    assert_ne!(rendered.data, samples[0].buffer.data);
}
