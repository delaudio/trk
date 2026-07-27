use super::*;
use crate::{
    BitcrusherSpec, CompressorSpec, DelaySpec, DriveMode, DriveSpec, FilterSpec, GateSpec,
    LimiterSpec, ReverbSpec,
};

#[test]
fn effect_devices_expose_and_validate_parameter_values() {
    let mut gain = EffectDevice::gain(1, 1.0);
    let descriptor = gain
        .parameter_descriptors()
        .into_iter()
        .next()
        .expect("gain descriptor");

    assert_eq!(descriptor.id, ParameterId::from(NATIVE_GAIN_PARAMETER_ID));
    assert_eq!(
        gain.parameter_value(&descriptor.id),
        Some(ParameterValue::Float(1.0))
    );

    gain.set_parameter_value(&descriptor.id, ParameterValue::Float(0.5))
        .expect("set gain parameter");
    assert_eq!(gain.kind, EffectDeviceKind::Gain { gain: 0.5 });
    assert_eq!(
        gain.set_parameter_value(&descriptor.id, ParameterValue::Float(3.0))
            .expect_err("gain outside descriptor range"),
        EditError::InvalidParameterValue
    );

    let mut width = EffectDevice::stereo_width(4, 1.0);
    assert_eq!(
        width.parameter_value(&ParameterId::from(NATIVE_WIDTH_PARAMETER_ID)),
        Some(ParameterValue::Percentage(1.0))
    );
    width
        .set_parameter_value(
            &ParameterId::from(NATIVE_WIDTH_PARAMETER_ID),
            ParameterValue::Percentage(2.0),
        )
        .expect("set width");
    assert_eq!(width.kind, EffectDeviceKind::StereoWidth { width: 2.0 });

    let mut phase = EffectDevice::phase_invert(5, false, false);
    phase
        .set_parameter_value(
            &ParameterId::from(NATIVE_PHASE_INVERT_LEFT_PARAMETER_ID),
            ParameterValue::Bool(true),
        )
        .expect("set phase invert left");
    assert_eq!(
        phase.kind,
        EffectDeviceKind::PhaseInvert {
            invert_left: true,
            invert_right: false
        }
    );

    let mut filter = EffectDevice::filter(6, FilterSpec::default());
    assert_eq!(
        filter.parameter_value(&ParameterId::from(NATIVE_FILTER_MODE_PARAMETER_ID)),
        Some(ParameterValue::Enum("lowPass".to_string()))
    );
    filter
        .set_parameter_value(
            &ParameterId::from(NATIVE_FILTER_MODE_PARAMETER_ID),
            ParameterValue::Enum("highPass".to_string()),
        )
        .expect("set filter mode");
    filter
        .set_parameter_value(
            &ParameterId::from(NATIVE_FILTER_CUTOFF_PARAMETER_ID),
            ParameterValue::FrequencyHertz(2_000.0),
        )
        .expect("set cutoff");
    assert_eq!(
        filter.kind,
        EffectDeviceKind::Filter {
            mode: FilterMode::HighPass,
            cutoff_hz: 2_000.0,
            resonance: 0.25,
            drive_db: 0.0,
            key_track: 0.0,
            env_amount: 0.0,
            mix: 1.0
        }
    );
    assert_eq!(
        filter
            .set_parameter_value(
                &ParameterId::from(NATIVE_FILTER_RESONANCE_PARAMETER_ID),
                ParameterValue::Normalized(1.5),
            )
            .expect_err("reject invalid resonance"),
        EditError::InvalidParameterValue
    );

    let mut delay = EffectDevice::delay(7, DelaySpec::default());
    assert_eq!(
        delay.parameter_value(&ParameterId::from(NATIVE_DELAY_TIME_LEFT_PARAMETER_ID)),
        Some(ParameterValue::Float(500.0))
    );
    delay
        .set_parameter_value(
            &ParameterId::from(NATIVE_DELAY_FEEDBACK_PARAMETER_ID),
            ParameterValue::Percentage(0.5),
        )
        .expect("set feedback");
    assert_eq!(
        delay
            .set_parameter_value(
                &ParameterId::from(NATIVE_DELAY_FEEDBACK_PARAMETER_ID),
                ParameterValue::Percentage(1.0),
            )
            .expect_err("reject unstable feedback"),
        EditError::InvalidParameterValue
    );

    let mut reverb = EffectDevice::reverb(8, ReverbSpec::default());
    assert_eq!(
        reverb.parameter_value(&ParameterId::from(NATIVE_REVERB_PREDELAY_PARAMETER_ID)),
        Some(ParameterValue::Float(20.0))
    );
    reverb
        .set_parameter_value(
            &ParameterId::from(NATIVE_REVERB_DECAY_PARAMETER_ID),
            ParameterValue::Seconds(5.0),
        )
        .expect("set decay");
    assert_eq!(
        reverb.parameter_value(&ParameterId::from(NATIVE_REVERB_DECAY_PARAMETER_ID)),
        Some(ParameterValue::Seconds(5.0))
    );
    assert_eq!(
        reverb
            .set_parameter_value(
                &ParameterId::from(NATIVE_REVERB_DECAY_PARAMETER_ID),
                ParameterValue::Seconds(60.0),
            )
            .expect_err("reject invalid decay"),
        EditError::InvalidParameterValue
    );

    let mut drive = EffectDevice::drive(9, DriveSpec::default());
    drive
        .set_parameter_value(
            &ParameterId::from(NATIVE_DRIVE_MODE_PARAMETER_ID),
            ParameterValue::Enum("hardClip".to_string()),
        )
        .expect("set drive mode");
    drive
        .set_parameter_value(
            &ParameterId::from(NATIVE_DRIVE_TONE_PARAMETER_ID),
            ParameterValue::Percentage(0.25),
        )
        .expect("set tone");
    assert_eq!(
        drive.kind,
        EffectDeviceKind::Drive {
            mode: DriveMode::HardClip,
            drive_db: 12.0,
            tone: 0.25,
            bias: 0.0,
            mix: 1.0,
            output_db: 0.0
        }
    );
    assert_eq!(
        drive
            .set_parameter_value(
                &ParameterId::from(NATIVE_DRIVE_DRIVE_PARAMETER_ID),
                ParameterValue::Decibels(60.0),
            )
            .expect_err("reject invalid drive"),
        EditError::InvalidParameterValue
    );

    let mut bitcrusher = EffectDevice::bitcrusher(10, BitcrusherSpec::default());
    bitcrusher
        .set_parameter_value(
            &ParameterId::from(NATIVE_BITCRUSHER_BIT_DEPTH_PARAMETER_ID),
            ParameterValue::Integer(8),
        )
        .expect("set bit depth");
    bitcrusher
        .set_parameter_value(
            &ParameterId::from(NATIVE_BITCRUSHER_REDUCTION_PARAMETER_ID),
            ParameterValue::Ratio(4.0),
        )
        .expect("set reduction");
    assert_eq!(
        bitcrusher.kind,
        EffectDeviceKind::Bitcrusher {
            bit_depth: 8,
            reduction_ratio: 4.0,
            dither: false,
            mix: 1.0,
            output_db: 0.0
        }
    );
    assert_eq!(
        bitcrusher
            .set_parameter_value(
                &ParameterId::from(NATIVE_BITCRUSHER_BIT_DEPTH_PARAMETER_ID),
                ParameterValue::Integer(25),
            )
            .expect_err("reject invalid bit depth"),
        EditError::InvalidParameterValue
    );
}

#[test]
fn dynamics_devices_export_valid_native_module_state() {
    for device in [
        EffectDevice::compressor(14, CompressorSpec::default()),
        EffectDevice::gate(15, GateSpec::default()),
        EffectDevice::limiter(16, LimiterSpec::default()),
    ] {
        device
            .native_module_state()
            .validate_against(&device.native_module_descriptor())
            .expect("default dynamics device exports valid native module state");
    }
}

#[test]
fn effect_devices_round_trip_native_module_state() {
    let mut gain = EffectDevice::gain(1, 1.0);
    let mut state = gain.native_module_state();

    state.bypassed = true;
    state
        .set_parameter(
            &gain.native_module_descriptor(),
            ParameterId::from(NATIVE_GAIN_PARAMETER_ID),
            ParameterValue::Float(0.25),
        )
        .expect("set native module parameter");

    gain.apply_native_module_state(&state)
        .expect("apply module state");

    assert!(gain.bypassed);
    assert_eq!(gain.kind, EffectDeviceKind::Gain { gain: 0.25 });
}

#[test]
fn effect_devices_round_trip_multi_parameter_native_module_state() {
    let mut phase = EffectDevice::phase_invert(5, false, false);
    let mut state = phase.native_module_state();

    assert_eq!(state.parameters.len(), 2);
    state
        .set_parameter(
            &phase.native_module_descriptor(),
            ParameterId::from(NATIVE_PHASE_INVERT_LEFT_PARAMETER_ID),
            ParameterValue::Bool(true),
        )
        .expect("set phase left");
    state
        .set_parameter(
            &phase.native_module_descriptor(),
            ParameterId::from(NATIVE_PHASE_INVERT_RIGHT_PARAMETER_ID),
            ParameterValue::Bool(true),
        )
        .expect("set phase right");

    phase
        .apply_native_module_state(&state)
        .expect("apply phase state");

    assert_eq!(
        phase.kind,
        EffectDeviceKind::PhaseInvert {
            invert_left: true,
            invert_right: true
        }
    );
}

#[test]
fn effect_devices_round_trip_filter_native_module_state() {
    let mut filter = EffectDevice::filter(6, FilterSpec::default());
    let mut state = filter.native_module_state();

    assert_eq!(state.parameters.len(), 7);
    state
        .set_parameter(
            &filter.native_module_descriptor(),
            ParameterId::from(NATIVE_FILTER_MODE_PARAMETER_ID),
            ParameterValue::Enum("notch".to_string()),
        )
        .expect("set mode");
    state
        .set_parameter(
            &filter.native_module_descriptor(),
            ParameterId::from(NATIVE_FILTER_MIX_PARAMETER_ID),
            ParameterValue::Percentage(0.5),
        )
        .expect("set mix");

    filter
        .apply_native_module_state(&state)
        .expect("apply filter state");

    assert_eq!(
        filter.kind,
        EffectDeviceKind::Filter {
            mode: FilterMode::Notch,
            cutoff_hz: 12_000.0,
            resonance: 0.25,
            drive_db: 0.0,
            key_track: 0.0,
            env_amount: 0.0,
            mix: 0.5
        }
    );
}

#[test]
fn effect_devices_round_trip_delay_native_module_state() {
    let mut delay = EffectDevice::delay(7, DelaySpec::default());
    let mut state = delay.native_module_state();

    assert_eq!(state.parameters.len(), 12);
    state
        .set_parameter(
            &delay.native_module_descriptor(),
            ParameterId::from(NATIVE_DELAY_TIME_LEFT_PARAMETER_ID),
            ParameterValue::Float(250.0),
        )
        .expect("set left time");
    state
        .set_parameter(
            &delay.native_module_descriptor(),
            ParameterId::from(NATIVE_DELAY_PING_PONG_PARAMETER_ID),
            ParameterValue::Bool(true),
        )
        .expect("set ping pong");
    state
        .set_parameter(
            &delay.native_module_descriptor(),
            ParameterId::from(NATIVE_DELAY_MIX_PARAMETER_ID),
            ParameterValue::Percentage(0.5),
        )
        .expect("set mix");

    delay
        .apply_native_module_state(&state)
        .expect("apply delay state");

    assert_eq!(
        delay.kind,
        EffectDeviceKind::Delay {
            sync: true,
            time_left_ms: 250.0,
            time_right_ms: 500.0,
            link_times: true,
            feedback: 0.35,
            ping_pong: true,
            filter_low_cut_hz: 20.0,
            filter_high_cut_hz: 20_000.0,
            mod_rate_hz: 0.0,
            mod_depth: 0.0,
            mix: 0.5,
            output_db: 0.0
        }
    );
}

#[test]
fn effect_devices_round_trip_reverb_native_module_state() {
    let mut reverb = EffectDevice::reverb(8, ReverbSpec::default());
    let mut state = reverb.native_module_state();

    assert_eq!(state.parameters.len(), 11);
    state
        .set_parameter(
            &reverb.native_module_descriptor(),
            ParameterId::from(NATIVE_REVERB_SIZE_PARAMETER_ID),
            ParameterValue::Percentage(0.75),
        )
        .expect("set size");
    state
        .set_parameter(
            &reverb.native_module_descriptor(),
            ParameterId::from(NATIVE_REVERB_MIX_PARAMETER_ID),
            ParameterValue::Percentage(1.0),
        )
        .expect("set mix");
    state
        .set_parameter(
            &reverb.native_module_descriptor(),
            ParameterId::from(NATIVE_REVERB_OUTPUT_PARAMETER_ID),
            ParameterValue::Decibels(-6.0),
        )
        .expect("set output");

    reverb
        .apply_native_module_state(&state)
        .expect("apply reverb state");

    assert_eq!(
        reverb.kind,
        EffectDeviceKind::Reverb {
            size: 0.75,
            predelay_ms: 20.0,
            decay_s: 2.5,
            damping: 0.5,
            low_cut_hz: 100.0,
            high_cut_hz: 16_000.0,
            diffusion: 0.75,
            width: 1.0,
            early_reflections: 0.5,
            mix: 1.0,
            output_db: -6.0
        }
    );
}

#[test]
fn effect_devices_round_trip_drive_native_module_state() {
    let mut drive = EffectDevice::drive(9, DriveSpec::default());
    let mut state = drive.native_module_state();

    assert_eq!(state.parameters.len(), 6);
    state
        .set_parameter(
            &drive.native_module_descriptor(),
            ParameterId::from(NATIVE_DRIVE_MODE_PARAMETER_ID),
            ParameterValue::Enum("saturation".to_string()),
        )
        .expect("set mode");
    state
        .set_parameter(
            &drive.native_module_descriptor(),
            ParameterId::from(NATIVE_DRIVE_DRIVE_PARAMETER_ID),
            ParameterValue::Decibels(18.0),
        )
        .expect("set drive");
    state
        .set_parameter(
            &drive.native_module_descriptor(),
            ParameterId::from(NATIVE_DRIVE_MIX_PARAMETER_ID),
            ParameterValue::Percentage(0.5),
        )
        .expect("set mix");

    drive
        .apply_native_module_state(&state)
        .expect("apply drive state");

    assert_eq!(
        drive.kind,
        EffectDeviceKind::Drive {
            mode: DriveMode::Saturation,
            drive_db: 18.0,
            tone: 0.5,
            bias: 0.0,
            mix: 0.5,
            output_db: 0.0
        }
    );
}

#[test]
fn effect_devices_round_trip_bitcrusher_native_module_state() {
    let mut bitcrusher = EffectDevice::bitcrusher(10, BitcrusherSpec::default());
    let mut state = bitcrusher.native_module_state();

    assert_eq!(state.parameters.len(), 5);
    state
        .set_parameter(
            &bitcrusher.native_module_descriptor(),
            ParameterId::from(NATIVE_BITCRUSHER_BIT_DEPTH_PARAMETER_ID),
            ParameterValue::Integer(6),
        )
        .expect("set bit depth");
    state
        .set_parameter(
            &bitcrusher.native_module_descriptor(),
            ParameterId::from(NATIVE_BITCRUSHER_DITHER_PARAMETER_ID),
            ParameterValue::Bool(true),
        )
        .expect("set dither");
    state
        .set_parameter(
            &bitcrusher.native_module_descriptor(),
            ParameterId::from(NATIVE_BITCRUSHER_OUTPUT_PARAMETER_ID),
            ParameterValue::Decibels(-3.0),
        )
        .expect("set output");

    bitcrusher
        .apply_native_module_state(&state)
        .expect("apply bitcrusher state");

    assert_eq!(
        bitcrusher.kind,
        EffectDeviceKind::Bitcrusher {
            bit_depth: 6,
            reduction_ratio: 1.0,
            dither: true,
            mix: 1.0,
            output_db: -3.0
        }
    );
}
