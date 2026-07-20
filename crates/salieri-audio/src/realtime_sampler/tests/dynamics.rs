use super::*;

#[test]
fn realtime_and_offline_match_for_native_compressor_fixture() {
    let preview = PreviewBuffer {
        sample_rate: 48_000,
        channels: 2,
        frames: 4,
        data: vec![0.1, -0.1, 0.9, -0.9, 0.4, -0.4, 0.2, -0.2],
    };
    let graph = DspGraphSpec {
        sends: Vec::new(),
        track_sends: Vec::new(),
        track_chains: Vec::new(),
        master: vec![DspDeviceSpec {
            bypassed: false,
            kind: DspDeviceKind::Compressor {
                threshold_db: -18.0,
                ratio: 4.0,
                attack_ms: 0.01,
                release_ms: 100.0,
                knee_db: 6.0,
                makeup_db: 0.0,
                auto_makeup: false,
                detector: DspDynamicsDetector::Peak,
                stereo_link: 1.0,
                mix: 1.0,
            },
        }],
    };

    let offline = render_sampler_events_with_dsp(
        &[OfflineSamplerSample {
            sample_id: 1,
            buffer: preview.clone(),
        }],
        &[OfflineSamplerEvent {
            track_id: 1,
            sample_id: 1,
            frame: 0,
            gain: 1.0,
            pan: 0.0,
            pitch_ratio: 1.0,
            velocity: 127,
        }],
        OfflineRenderSpec {
            sample_rate: 48_000,
            channels: 2,
            frames: 4,
        },
        &graph,
    )
    .expect("offline render");

    let mut realtime = RealtimeSampler::new(RealtimeSamplerConfig {
        sample_rate: 48_000,
        channels: 2,
        max_voices: 4,
    });
    realtime
        .register_sample(1, preview)
        .expect("register sample");
    realtime.set_dsp_graph(graph);
    realtime
        .handle_command(RealtimeAudioCommand::TriggerSample {
            track_id: 1,
            sample_id: 1,
            frame: 0,
            gain: 1.0,
            pan: 0.0,
            pitch_ratio: 1.0,
        })
        .expect("trigger");
    let rendered = realtime.render(4);

    assert_eq!(rendered.data.len(), offline.data.len());
    for (actual, expected) in rendered.data.iter().zip(offline.data.iter()) {
        assert_approx_eq(*actual, *expected);
    }
}
