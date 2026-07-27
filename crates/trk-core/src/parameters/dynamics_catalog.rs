use super::{
    catalog::{
        bool_descriptor, continuous_descriptor, enum_descriptor, BoolDescriptorSpec,
        ContinuousDescriptorSpec, EnumDescriptorSpec,
    },
    ParameterDescriptor, ParameterFlags, ParameterUnit, ParameterValue, ParameterValueType,
};

pub const NATIVE_COMPRESSOR_THRESHOLD_PARAMETER_ID: &str = "native.compressor.thresholdDb";
pub const NATIVE_COMPRESSOR_RATIO_PARAMETER_ID: &str = "native.compressor.ratio";
pub const NATIVE_COMPRESSOR_ATTACK_PARAMETER_ID: &str = "native.compressor.attackMs";
pub const NATIVE_COMPRESSOR_RELEASE_PARAMETER_ID: &str = "native.compressor.releaseMs";
pub const NATIVE_COMPRESSOR_KNEE_PARAMETER_ID: &str = "native.compressor.kneeDb";
pub const NATIVE_COMPRESSOR_MAKEUP_PARAMETER_ID: &str = "native.compressor.makeupDb";
pub const NATIVE_COMPRESSOR_AUTO_MAKEUP_PARAMETER_ID: &str = "native.compressor.autoMakeup";
pub const NATIVE_COMPRESSOR_DETECTOR_PARAMETER_ID: &str = "native.compressor.detector";
pub const NATIVE_COMPRESSOR_STEREO_LINK_PARAMETER_ID: &str = "native.compressor.stereoLink";
pub const NATIVE_COMPRESSOR_MIX_PARAMETER_ID: &str = "native.compressor.mix";
pub const NATIVE_COMPRESSOR_GAIN_REDUCTION_PARAMETER_ID: &str = "native.compressor.gainReductionDb";
pub const NATIVE_GATE_THRESHOLD_PARAMETER_ID: &str = "native.gate.thresholdDb";
pub const NATIVE_GATE_HYSTERESIS_PARAMETER_ID: &str = "native.gate.hysteresisDb";
pub const NATIVE_GATE_ATTACK_PARAMETER_ID: &str = "native.gate.attackMs";
pub const NATIVE_GATE_HOLD_PARAMETER_ID: &str = "native.gate.holdMs";
pub const NATIVE_GATE_RELEASE_PARAMETER_ID: &str = "native.gate.releaseMs";
pub const NATIVE_GATE_RANGE_PARAMETER_ID: &str = "native.gate.rangeDb";
pub const NATIVE_GATE_DETECTOR_PARAMETER_ID: &str = "native.gate.detector";
pub const NATIVE_GATE_STEREO_LINK_PARAMETER_ID: &str = "native.gate.stereoLink";
pub const NATIVE_GATE_STATE_PARAMETER_ID: &str = "native.gate.open";
pub const NATIVE_LIMITER_CEILING_PARAMETER_ID: &str = "native.limiter.ceilingDb";
pub const NATIVE_LIMITER_INPUT_GAIN_PARAMETER_ID: &str = "native.limiter.inputGainDb";
pub const NATIVE_LIMITER_RELEASE_PARAMETER_ID: &str = "native.limiter.releaseMs";
pub const NATIVE_LIMITER_LOOKAHEAD_PARAMETER_ID: &str = "native.limiter.lookaheadMs";
pub const NATIVE_LIMITER_STEREO_LINK_PARAMETER_ID: &str = "native.limiter.stereoLink";
pub const NATIVE_LIMITER_TRUE_PEAK_PARAMETER_ID: &str = "native.limiter.truePeak";
pub const NATIVE_LIMITER_GAIN_REDUCTION_PARAMETER_ID: &str = "native.limiter.gainReductionDb";

macro_rules! descriptor_fn {
    ($name:ident, $expr:expr) => {
        #[must_use]
        pub fn $name() -> ParameterDescriptor {
            $expr
        }
    };
}

descriptor_fn!(
    native_compressor_threshold_descriptor,
    db(
        NATIVE_COMPRESSOR_THRESHOLD_PARAMETER_ID,
        "Threshold",
        "Thr",
        -18.0,
        -80.0,
        0.0,
        "native.compressor",
        10
    )
);
descriptor_fn!(
    native_compressor_ratio_descriptor,
    ratio(
        NATIVE_COMPRESSOR_RATIO_PARAMETER_ID,
        4.0,
        "native.compressor",
        20
    )
);
descriptor_fn!(
    native_compressor_attack_descriptor,
    ms(
        NATIVE_COMPRESSOR_ATTACK_PARAMETER_ID,
        "Attack",
        "Atk",
        10.0,
        0.01,
        500.0,
        "native.compressor",
        30
    )
);
descriptor_fn!(
    native_compressor_release_descriptor,
    ms(
        NATIVE_COMPRESSOR_RELEASE_PARAMETER_ID,
        "Release",
        "Rel",
        100.0,
        1.0,
        5_000.0,
        "native.compressor",
        40
    )
);
descriptor_fn!(
    native_compressor_knee_descriptor,
    db(
        NATIVE_COMPRESSOR_KNEE_PARAMETER_ID,
        "Knee",
        "Knee",
        6.0,
        0.0,
        24.0,
        "native.compressor",
        50
    )
);
descriptor_fn!(
    native_compressor_makeup_descriptor,
    db(
        NATIVE_COMPRESSOR_MAKEUP_PARAMETER_ID,
        "Makeup",
        "MkUp",
        0.0,
        -24.0,
        24.0,
        "native.compressor",
        60
    )
);
descriptor_fn!(
    native_compressor_auto_makeup_descriptor,
    flag(
        NATIVE_COMPRESSOR_AUTO_MAKEUP_PARAMETER_ID,
        "Auto Makeup",
        "Auto",
        false,
        "native.compressor",
        70
    )
);
descriptor_fn!(
    native_compressor_detector_descriptor,
    detector(
        NATIVE_COMPRESSOR_DETECTOR_PARAMETER_ID,
        "native.compressor",
        80
    )
);
descriptor_fn!(
    native_compressor_stereo_link_descriptor,
    percent(
        NATIVE_COMPRESSOR_STEREO_LINK_PARAMETER_ID,
        "Stereo Link",
        "Link",
        1.0,
        "native.compressor",
        90
    )
);
descriptor_fn!(
    native_compressor_mix_descriptor,
    percent(
        NATIVE_COMPRESSOR_MIX_PARAMETER_ID,
        "Mix",
        "Mix",
        1.0,
        "native.compressor",
        100
    )
);
descriptor_fn!(
    native_compressor_gain_reduction_descriptor,
    meter(
        NATIVE_COMPRESSOR_GAIN_REDUCTION_PARAMETER_ID,
        "Gain Reduction",
        "GR",
        "native.compressor",
        900
    )
);
descriptor_fn!(
    native_gate_threshold_descriptor,
    db(
        NATIVE_GATE_THRESHOLD_PARAMETER_ID,
        "Threshold",
        "Thr",
        -48.0,
        -80.0,
        0.0,
        "native.gate",
        10
    )
);
descriptor_fn!(
    native_gate_hysteresis_descriptor,
    db(
        NATIVE_GATE_HYSTERESIS_PARAMETER_ID,
        "Hysteresis",
        "Hyst",
        3.0,
        0.0,
        24.0,
        "native.gate",
        20
    )
);
descriptor_fn!(
    native_gate_attack_descriptor,
    ms(
        NATIVE_GATE_ATTACK_PARAMETER_ID,
        "Attack",
        "Atk",
        5.0,
        0.01,
        500.0,
        "native.gate",
        30
    )
);
descriptor_fn!(
    native_gate_hold_descriptor,
    ms(
        NATIVE_GATE_HOLD_PARAMETER_ID,
        "Hold",
        "Hold",
        25.0,
        0.0,
        1_000.0,
        "native.gate",
        40
    )
);
descriptor_fn!(
    native_gate_release_descriptor,
    ms(
        NATIVE_GATE_RELEASE_PARAMETER_ID,
        "Release",
        "Rel",
        100.0,
        1.0,
        5_000.0,
        "native.gate",
        50
    )
);
descriptor_fn!(
    native_gate_range_descriptor,
    db(
        NATIVE_GATE_RANGE_PARAMETER_ID,
        "Range",
        "Rng",
        80.0,
        0.0,
        80.0,
        "native.gate",
        60
    )
);
descriptor_fn!(
    native_gate_detector_descriptor,
    detector(NATIVE_GATE_DETECTOR_PARAMETER_ID, "native.gate", 70)
);
descriptor_fn!(
    native_gate_stereo_link_descriptor,
    percent(
        NATIVE_GATE_STEREO_LINK_PARAMETER_ID,
        "Stereo Link",
        "Link",
        1.0,
        "native.gate",
        80
    )
);
descriptor_fn!(
    native_gate_state_descriptor,
    read_only_bool(
        NATIVE_GATE_STATE_PARAMETER_ID,
        "Gate Open",
        "Open",
        "native.gate",
        900
    )
);
descriptor_fn!(
    native_limiter_ceiling_descriptor,
    db(
        NATIVE_LIMITER_CEILING_PARAMETER_ID,
        "Ceiling",
        "Ceil",
        -0.1,
        -24.0,
        0.0,
        "native.limiter",
        10
    )
);
descriptor_fn!(
    native_limiter_input_gain_descriptor,
    db(
        NATIVE_LIMITER_INPUT_GAIN_PARAMETER_ID,
        "Input Gain",
        "In",
        0.0,
        -24.0,
        24.0,
        "native.limiter",
        20
    )
);
descriptor_fn!(
    native_limiter_release_descriptor,
    ms(
        NATIVE_LIMITER_RELEASE_PARAMETER_ID,
        "Release",
        "Rel",
        50.0,
        1.0,
        1_000.0,
        "native.limiter",
        30
    )
);
descriptor_fn!(
    native_limiter_lookahead_descriptor,
    ms(
        NATIVE_LIMITER_LOOKAHEAD_PARAMETER_ID,
        "Lookahead",
        "Look",
        1.0,
        0.0,
        20.0,
        "native.limiter",
        40
    )
);
descriptor_fn!(
    native_limiter_stereo_link_descriptor,
    percent(
        NATIVE_LIMITER_STEREO_LINK_PARAMETER_ID,
        "Stereo Link",
        "Link",
        1.0,
        "native.limiter",
        50
    )
);
descriptor_fn!(
    native_limiter_true_peak_descriptor,
    flag(
        NATIVE_LIMITER_TRUE_PEAK_PARAMETER_ID,
        "True Peak",
        "TP",
        false,
        "native.limiter",
        60
    )
);
descriptor_fn!(
    native_limiter_gain_reduction_descriptor,
    meter(
        NATIVE_LIMITER_GAIN_REDUCTION_PARAMETER_ID,
        "Gain Reduction",
        "GR",
        "native.limiter",
        900
    )
);

#[must_use]
pub fn native_compressor_parameter_descriptors() -> Vec<ParameterDescriptor> {
    vec![
        native_compressor_threshold_descriptor(),
        native_compressor_ratio_descriptor(),
        native_compressor_attack_descriptor(),
        native_compressor_release_descriptor(),
        native_compressor_knee_descriptor(),
        native_compressor_makeup_descriptor(),
        native_compressor_auto_makeup_descriptor(),
        native_compressor_detector_descriptor(),
        native_compressor_stereo_link_descriptor(),
        native_compressor_mix_descriptor(),
        native_compressor_gain_reduction_descriptor(),
    ]
}

#[must_use]
pub fn native_gate_parameter_descriptors() -> Vec<ParameterDescriptor> {
    vec![
        native_gate_threshold_descriptor(),
        native_gate_hysteresis_descriptor(),
        native_gate_attack_descriptor(),
        native_gate_hold_descriptor(),
        native_gate_release_descriptor(),
        native_gate_range_descriptor(),
        native_gate_detector_descriptor(),
        native_gate_stereo_link_descriptor(),
        native_gate_state_descriptor(),
    ]
}

#[must_use]
pub fn native_limiter_parameter_descriptors() -> Vec<ParameterDescriptor> {
    vec![
        native_limiter_ceiling_descriptor(),
        native_limiter_input_gain_descriptor(),
        native_limiter_release_descriptor(),
        native_limiter_lookahead_descriptor(),
        native_limiter_stereo_link_descriptor(),
        native_limiter_true_peak_descriptor(),
        native_limiter_gain_reduction_descriptor(),
    ]
}

#[must_use]
pub fn native_dynamics_parameter_descriptors() -> Vec<ParameterDescriptor> {
    let mut descriptors = native_compressor_parameter_descriptors();
    descriptors.extend(native_gate_parameter_descriptors());
    descriptors.extend(native_limiter_parameter_descriptors());
    descriptors
}

#[must_use]
pub fn native_dynamics_parameter_descriptor(id: &str) -> Option<ParameterDescriptor> {
    match id {
        NATIVE_COMPRESSOR_THRESHOLD_PARAMETER_ID => Some(native_compressor_threshold_descriptor()),
        NATIVE_COMPRESSOR_RATIO_PARAMETER_ID => Some(native_compressor_ratio_descriptor()),
        NATIVE_COMPRESSOR_ATTACK_PARAMETER_ID => Some(native_compressor_attack_descriptor()),
        NATIVE_COMPRESSOR_RELEASE_PARAMETER_ID => Some(native_compressor_release_descriptor()),
        NATIVE_COMPRESSOR_KNEE_PARAMETER_ID => Some(native_compressor_knee_descriptor()),
        NATIVE_COMPRESSOR_MAKEUP_PARAMETER_ID => Some(native_compressor_makeup_descriptor()),
        NATIVE_COMPRESSOR_AUTO_MAKEUP_PARAMETER_ID => {
            Some(native_compressor_auto_makeup_descriptor())
        }
        NATIVE_COMPRESSOR_DETECTOR_PARAMETER_ID => Some(native_compressor_detector_descriptor()),
        NATIVE_COMPRESSOR_STEREO_LINK_PARAMETER_ID => {
            Some(native_compressor_stereo_link_descriptor())
        }
        NATIVE_COMPRESSOR_MIX_PARAMETER_ID => Some(native_compressor_mix_descriptor()),
        NATIVE_COMPRESSOR_GAIN_REDUCTION_PARAMETER_ID => {
            Some(native_compressor_gain_reduction_descriptor())
        }
        NATIVE_GATE_THRESHOLD_PARAMETER_ID => Some(native_gate_threshold_descriptor()),
        NATIVE_GATE_HYSTERESIS_PARAMETER_ID => Some(native_gate_hysteresis_descriptor()),
        NATIVE_GATE_ATTACK_PARAMETER_ID => Some(native_gate_attack_descriptor()),
        NATIVE_GATE_HOLD_PARAMETER_ID => Some(native_gate_hold_descriptor()),
        NATIVE_GATE_RELEASE_PARAMETER_ID => Some(native_gate_release_descriptor()),
        NATIVE_GATE_RANGE_PARAMETER_ID => Some(native_gate_range_descriptor()),
        NATIVE_GATE_DETECTOR_PARAMETER_ID => Some(native_gate_detector_descriptor()),
        NATIVE_GATE_STEREO_LINK_PARAMETER_ID => Some(native_gate_stereo_link_descriptor()),
        NATIVE_GATE_STATE_PARAMETER_ID => Some(native_gate_state_descriptor()),
        NATIVE_LIMITER_CEILING_PARAMETER_ID => Some(native_limiter_ceiling_descriptor()),
        NATIVE_LIMITER_INPUT_GAIN_PARAMETER_ID => Some(native_limiter_input_gain_descriptor()),
        NATIVE_LIMITER_RELEASE_PARAMETER_ID => Some(native_limiter_release_descriptor()),
        NATIVE_LIMITER_LOOKAHEAD_PARAMETER_ID => Some(native_limiter_lookahead_descriptor()),
        NATIVE_LIMITER_STEREO_LINK_PARAMETER_ID => Some(native_limiter_stereo_link_descriptor()),
        NATIVE_LIMITER_TRUE_PEAK_PARAMETER_ID => Some(native_limiter_true_peak_descriptor()),
        NATIVE_LIMITER_GAIN_REDUCTION_PARAMETER_ID => {
            Some(native_limiter_gain_reduction_descriptor())
        }
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn db(
    id: &'static str,
    name: &'static str,
    short_name: &'static str,
    default: f32,
    min: f32,
    max: f32,
    group: &'static str,
    order: u16,
) -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id,
        name,
        short_name: Some(short_name),
        value_type: ParameterValueType::Decibels,
        default: ParameterValue::Decibels(default),
        min,
        max,
        step: Some(0.1),
        unit: ParameterUnit::Decibels,
        flags: ParameterFlags::automatable(),
        group: Some(group),
        order,
    })
}

fn ratio(id: &'static str, default: f32, group: &'static str, order: u16) -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id,
        name: "Ratio",
        short_name: Some("Ratio"),
        value_type: ParameterValueType::Ratio,
        default: ParameterValue::Ratio(default),
        min: 1.0,
        max: 20.0,
        step: Some(0.01),
        unit: ParameterUnit::Ratio,
        flags: ParameterFlags::automatable_logarithmic(),
        group: Some(group),
        order,
    })
}

#[allow(clippy::too_many_arguments)]
fn ms(
    id: &'static str,
    name: &'static str,
    short_name: &'static str,
    default: f32,
    min: f32,
    max: f32,
    group: &'static str,
    order: u16,
) -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id,
        name,
        short_name: Some(short_name),
        value_type: ParameterValueType::PlainFloat,
        default: ParameterValue::Float(default),
        min,
        max,
        step: Some(0.01),
        unit: ParameterUnit::Milliseconds,
        flags: ParameterFlags::automatable_logarithmic(),
        group: Some(group),
        order,
    })
}

fn percent(
    id: &'static str,
    name: &'static str,
    short_name: &'static str,
    default: f32,
    group: &'static str,
    order: u16,
) -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id,
        name,
        short_name: Some(short_name),
        value_type: ParameterValueType::Percentage,
        default: ParameterValue::Percentage(default),
        min: 0.0,
        max: 1.0,
        step: Some(0.001),
        unit: ParameterUnit::Percent,
        flags: ParameterFlags::automatable(),
        group: Some(group),
        order,
    })
}

fn flag(
    id: &'static str,
    name: &'static str,
    short_name: &'static str,
    default: bool,
    group: &'static str,
    order: u16,
) -> ParameterDescriptor {
    bool_descriptor(BoolDescriptorSpec {
        id,
        name,
        short_name: Some(short_name),
        default,
        flags: ParameterFlags {
            stepped: true,
            ..ParameterFlags::automatable()
        },
        group: Some(group),
        order,
    })
}

fn detector(id: &'static str, group: &'static str, order: u16) -> ParameterDescriptor {
    enum_descriptor(EnumDescriptorSpec {
        id,
        name: "Detector",
        short_name: Some("Det"),
        default: "peak",
        choices: &[("peak", "Peak"), ("rms", "RMS")],
        flags: ParameterFlags {
            stepped: true,
            ..ParameterFlags::automatable()
        },
        group: Some(group),
        order,
    })
}

fn meter(
    id: &'static str,
    name: &'static str,
    short_name: &'static str,
    group: &'static str,
    order: u16,
) -> ParameterDescriptor {
    let mut descriptor = db(id, name, short_name, 0.0, -80.0, 0.0, group, order);
    descriptor.flags = ParameterFlags {
        read_only: true,
        ..ParameterFlags::default()
    };
    descriptor
}

fn read_only_bool(
    id: &'static str,
    name: &'static str,
    short_name: &'static str,
    group: &'static str,
    order: u16,
) -> ParameterDescriptor {
    let mut descriptor = bool_descriptor(BoolDescriptorSpec {
        id,
        name,
        short_name: Some(short_name),
        default: false,
        flags: ParameterFlags::default(),
        group: Some(group),
        order,
    });
    descriptor.flags.read_only = true;
    descriptor
}
