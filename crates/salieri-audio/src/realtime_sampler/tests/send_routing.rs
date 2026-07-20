use crate::{
    fixtures::{assert_approx_eq, mono_sample},
    *,
};

#[test]
fn realtime_and_offline_match_for_send_bus_routing() {
    let sample = mono_sample(vec![1.0, 0.5]);
    let graph = DspGraphSpec {
        sends: vec![SendDspBusSpec {
            send_id: 1,
            pre_fader: false,
            devices: vec![DspDeviceSpec {
                bypassed: false,
                kind: DspDeviceKind::Gain { gain: 0.25 },
            }],
        }],
        track_sends: vec![TrackSendSpec {
            track_id: 1,
            send_id: 1,
            gain: 1.0,
        }],
        track_chains: Vec::new(),
        master: Vec::new(),
    };
    let mut realtime = RealtimeSampler::new(RealtimeSamplerConfig {
        sample_rate: 48_000,
        channels: 1,
        max_voices: 8,
    });
    realtime
        .register_sample(7, sample.clone())
        .expect("register sample");
    realtime.set_dsp_graph(graph.clone());
    realtime
        .handle_command(RealtimeAudioCommand::TriggerSample {
            track_id: 1,
            sample_id: 7,
            frame: 0,
            gain: 1.0,
            pan: 0.0,
            pitch_ratio: 1.0,
        })
        .expect("trigger");

    let realtime = realtime.render(2);
    let offline = render_sampler_events_with_dsp(
        &[OfflineSamplerSample {
            sample_id: 7,
            buffer: sample,
        }],
        &[OfflineSamplerEvent {
            track_id: 1,
            sample_id: 7,
            frame: 0,
            gain: 1.0,
            pan: 0.0,
            pitch_ratio: 1.0,
            velocity: 127,
        }],
        OfflineRenderSpec {
            sample_rate: 48_000,
            channels: 1,
            frames: 2,
        },
        &graph,
    )
    .expect("offline");

    assert_eq!(realtime.frames, offline.frames);
    for (actual, expected) in realtime.data.iter().zip(offline.data.iter()) {
        assert_approx_eq(*actual, *expected);
    }
}
