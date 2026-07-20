use crate::{
    fixtures::{assert_approx_eq, mono_sample},
    *,
};

#[test]
fn renders_sampler_events_through_send_bus_before_master() {
    let samples = vec![OfflineSamplerSample {
        sample_id: 7,
        buffer: mono_sample(vec![1.0]),
    }];
    let events = vec![OfflineSamplerEvent {
        track_id: 1,
        sample_id: 7,
        frame: 0,
        gain: 1.0,
        pan: 0.0,
        pitch_ratio: 1.0,
        velocity: 127,
        playback: AudioSamplerPlaybackSettings::default(),
    }];
    let graph = DspGraphSpec {
        sends: vec![SendDspBusSpec {
            send_id: 1,
            pre_fader: false,
            devices: vec![DspDeviceSpec {
                bypassed: false,
                kind: DspDeviceKind::Gain { gain: 0.5 },
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

    assert_approx_eq(rendered.data[0], 1.5);
}
