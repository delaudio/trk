use super::*;

#[test]
fn realtime_and_offline_match_for_native_chorus_fixture() {
    let preview = PreviewBuffer {
        sample_rate: 48_000,
        channels: 2,
        frames: 64,
        data: (0..64)
            .flat_map(|frame| {
                let value = (frame as f32 / 64.0).sin();
                [value, -value]
            })
            .collect(),
    };
    let graph = DspGraphSpec {
        sends: Vec::new(),
        track_sends: Vec::new(),
        track_chains: Vec::new(),
        master: vec![DspDeviceSpec {
            bypassed: false,
            kind: DspDeviceKind::Chorus {
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
            frames: 64,
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
    let rendered = realtime.render(64);

    assert_eq!(rendered.data.len(), offline.data.len());
    for (actual, expected) in rendered.data.iter().zip(offline.data.iter()) {
        assert_approx_eq(*actual, *expected);
    }
}
