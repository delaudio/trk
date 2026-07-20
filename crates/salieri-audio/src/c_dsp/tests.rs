use super::*;

fn prepare_stereo(module: &mut CNativeGainModule) {
    module
        .prepare(NativeModulePrepareSpec {
            sample_rate: 48_000,
            channels: 2,
            max_block_frames: 4,
        })
        .expect("prepare");
}

#[test]
fn c_gain_poc_processes_fixed_buffers_through_native_boundary() {
    let mut module = CNativeGainModule::new(CNativeGainSpec {
        bypassed: false,
        state: CNativeGainState { gain: 0.5 },
    });
    prepare_stereo(&mut module);

    let mut data = [1.0, -1.0, 0.5, -0.5];
    module.process_in_place(&mut data).expect("process");

    assert_eq!(data, [0.5, -0.5, 0.25, -0.25]);
    assert_eq!(module.state(), CNativeGainState { gain: 0.5 });
}

#[test]
fn c_gain_poc_exposes_stable_descriptor_and_plain_state() {
    let descriptor = CNativeGainModule::descriptor();

    assert_eq!(descriptor.id, C_NATIVE_GAIN_MODULE_ID);
    assert_eq!(descriptor.parameters.len(), 1);
    assert_eq!(descriptor.parameters[0].id, C_NATIVE_GAIN_GAIN_PARAMETER_ID);
    assert_eq!(descriptor.parameters[0].default, 1.0);
    assert_eq!(
        CNativeGainSpec::default().state,
        CNativeGainState { gain: 1.0 }
    );
}

#[test]
fn c_gain_poc_rejects_parameter_and_buffer_contract_violations() {
    let mut module = CNativeGainModule::new(CNativeGainSpec::default());
    prepare_stereo(&mut module);

    assert!(matches!(
        module.set_parameter(CNativeGainParameterValue::Gain(f32::NAN)),
        Err(AudioExportError::InvalidDspParameter)
    ));
    assert!(matches!(
        module.set_parameter(CNativeGainParameterValue::Gain(2.1)),
        Err(AudioExportError::InvalidDspParameter)
    ));
    assert!(matches!(
        module.process_in_place(&mut [1.0, 2.0, 3.0]),
        Err(AudioExportError::InvalidBufferLength { .. })
    ));
    assert!(matches!(
        module.process_in_place(&mut [0.0; 10]),
        Err(AudioExportError::InvalidBufferLength { .. })
    ));
}

#[test]
fn c_gain_poc_rejects_non_finite_audio_and_resets_to_defaults() {
    let mut module = CNativeGainModule::new(CNativeGainSpec {
        bypassed: false,
        state: CNativeGainState { gain: 0.25 },
    });
    prepare_stereo(&mut module);

    assert!(matches!(
        module.process_in_place(&mut [1.0, f32::INFINITY]),
        Err(AudioExportError::InvalidDspParameter)
    ));

    module
        .set_parameter(CNativeGainParameterValue::Gain(0.75))
        .expect("set gain");
    module.reset();
    assert!(!module.is_prepared());
    assert_eq!(module.state(), CNativeGainState { gain: 0.25 });

    prepare_stereo(&mut module);
    let mut data = [1.0, 1.0];
    module
        .process_in_place(&mut data)
        .expect("process after reset");
    assert_eq!(data, [0.25, 0.25]);
}
