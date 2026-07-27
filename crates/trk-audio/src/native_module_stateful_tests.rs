use super::*;

#[test]
fn native_effect_module_processes_delay_with_sample_accurate_timing() {
    let mut module = NativeEffectModule::new(NativeEffectModuleSpec {
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
    });
    module
        .prepare(NativeModulePrepareSpec {
            sample_rate: 48_000,
            channels: 2,
            max_block_frames: 64,
        })
        .expect("prepare delay");

    let mut data = vec![0.0; 64 * 2];
    data[0] = 1.0;
    module.process_in_place(&mut data).expect("process delay");

    assert_eq!(data[0], 0.0);
    assert_eq!(data[48 * 2], 1.0);
    assert_eq!(data[48 * 2 + 1], 0.0);
}

#[test]
fn native_effect_module_delay_feedback_and_ping_pong_stay_stable() {
    let mut module = NativeEffectModule::new(NativeEffectModuleSpec {
        bypassed: false,
        kind: DspDeviceKind::Delay {
            sync: true,
            time_left_ms: 125.0,
            time_right_ms: 250.0,
            link_times: false,
            feedback: 0.95,
            ping_pong: true,
            filter_low_cut_hz: 40.0,
            filter_high_cut_hz: 12_000.0,
            mod_rate_hz: 0.5,
            mod_depth: 0.25,
            mix: 0.5,
            output_db: 0.0,
        },
    });
    module
        .prepare(NativeModulePrepareSpec {
            sample_rate: 48_000,
            channels: 2,
            max_block_frames: 512,
        })
        .expect("prepare delay");

    let mut data = vec![0.0; 512 * 2];
    data[0] = 1.0;
    module.process_in_place(&mut data).expect("process delay");

    assert!(data.iter().all(|sample| sample.is_finite()));
    assert!(matches!(
        module.set_parameter(NativeEffectParameterValue::DelayFeedback(1.0)),
        Err(AudioExportError::InvalidDspParameter)
    ));
}

#[test]
fn native_effect_module_processes_reverb_tail_and_reset() {
    let mut module = NativeEffectModule::new(NativeEffectModuleSpec {
        bypassed: false,
        kind: test_reverb_kind(),
    });
    module
        .prepare(NativeModulePrepareSpec {
            sample_rate: 48_000,
            channels: 2,
            max_block_frames: 2_048,
        })
        .expect("prepare reverb");

    let mut data = vec![0.0; 2_048 * 2];
    data[0] = 1.0;
    module.process_in_place(&mut data).expect("process reverb");

    assert_eq!(data[0], 0.0);
    assert!(data.iter().all(|sample| sample.is_finite()));
    assert!(
        data[2..].iter().any(|sample| sample.abs() > 0.000_1),
        "reverb should produce a deterministic tail after the input frame"
    );

    module.reset();
    let mut silence = vec![0.0; 256 * 2];
    module
        .process_in_place(&mut silence)
        .expect("process silence after reset");
    assert!(silence.iter().all(|sample| sample.abs() <= f32::EPSILON));
}

#[test]
fn native_effect_module_reverb_rejects_invalid_parameter_values() {
    let mut module = NativeEffectModule::new(NativeEffectModuleSpec {
        bypassed: false,
        kind: test_reverb_kind(),
    });

    assert!(matches!(
        module.set_parameter(NativeEffectParameterValue::ReverbDecayS(60.0)),
        Err(AudioExportError::InvalidDspParameter)
    ));
    assert!(matches!(
        module.set_parameter(NativeEffectParameterValue::ReverbWidth(2.1)),
        Err(AudioExportError::InvalidDspParameter)
    ));
    module
        .set_parameter(NativeEffectParameterValue::ReverbMix(0.5))
        .expect("set valid mix");
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
