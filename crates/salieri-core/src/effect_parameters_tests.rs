use super::*;
use crate::{DelaySpec, FilterSpec};

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
