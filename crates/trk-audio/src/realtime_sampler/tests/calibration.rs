use crate::{
    fixtures::{assert_approx_eq, mono_sample},
    *,
};

#[test]
fn targets_only_the_captured_track_and_updates_meters() {
    let calibration = calibration_control();
    let mut sampler = calibration_sampler(&calibration);
    sampler
        .register_sample(1, mono_sample(vec![0.25]))
        .expect("register sample");
    for track_id in [1, 2] {
        trigger(&mut sampler, track_id);
    }

    let rendered = sampler.render(1);

    assert_approx_eq(rendered.data[0], 0.375);
    assert_approx_eq(calibration.meters().peak, 0.375);
}

#[test]
fn selected_track_trim_applies_to_dry_and_send_paths() {
    let calibration = calibration_control();
    let mut sampler = calibration_sampler(&calibration);
    sampler
        .register_sample(1, mono_sample(vec![0.25]))
        .expect("register sample");
    sampler.set_dsp_graph(DspGraphSpec {
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
    });
    trigger(&mut sampler, 1);

    let rendered = sampler.render(1);

    assert_approx_eq(rendered.data[0], 0.1875);
}

fn calibration_control() -> CalibrationControl {
    let calibration = CalibrationControl::new();
    calibration
        .store(CalibrationSettings {
            target_track_id: Some(1),
            track_gain: 0.5,
            ..CalibrationSettings::default()
        })
        .expect("calibration settings");
    calibration
}

fn calibration_sampler(calibration: &CalibrationControl) -> RealtimeSampler {
    RealtimeSampler::with_calibration(
        RealtimeSamplerConfig {
            sample_rate: 48_000,
            channels: 1,
            max_voices: 4,
        },
        calibration.clone(),
    )
}

fn trigger(sampler: &mut RealtimeSampler, track_id: u32) {
    sampler
        .handle_command(RealtimeAudioCommand::TriggerSample {
            track_id,
            sample_id: 1,
            frame: 0,
            gain: 1.0,
            pan: 0.0,
            pitch_ratio: 1.0,
            playback: AudioSamplerPlaybackSettings::default(),
        })
        .expect("trigger");
}
