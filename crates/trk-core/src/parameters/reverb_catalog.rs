use super::{
    catalog::{continuous_descriptor, ContinuousDescriptorSpec},
    ParameterDescriptor, ParameterFlags, ParameterUnit, ParameterValue, ParameterValueType,
};

pub const NATIVE_REVERB_SIZE_PARAMETER_ID: &str = "native.reverb.size";
pub const NATIVE_REVERB_PREDELAY_PARAMETER_ID: &str = "native.reverb.predelayMs";
pub const NATIVE_REVERB_DECAY_PARAMETER_ID: &str = "native.reverb.decayS";
pub const NATIVE_REVERB_DAMPING_PARAMETER_ID: &str = "native.reverb.damping";
pub const NATIVE_REVERB_LOW_CUT_PARAMETER_ID: &str = "native.reverb.lowCutHz";
pub const NATIVE_REVERB_HIGH_CUT_PARAMETER_ID: &str = "native.reverb.highCutHz";
pub const NATIVE_REVERB_DIFFUSION_PARAMETER_ID: &str = "native.reverb.diffusion";
pub const NATIVE_REVERB_WIDTH_PARAMETER_ID: &str = "native.reverb.width";
pub const NATIVE_REVERB_EARLY_REFLECTIONS_PARAMETER_ID: &str = "native.reverb.earlyReflections";
pub const NATIVE_REVERB_MIX_PARAMETER_ID: &str = "native.reverb.mix";
pub const NATIVE_REVERB_OUTPUT_PARAMETER_ID: &str = "native.reverb.outputDb";

#[must_use]
pub fn native_reverb_size_descriptor() -> ParameterDescriptor {
    reverb_percent_descriptor(
        NATIVE_REVERB_SIZE_PARAMETER_ID,
        "Size",
        "Size",
        0.5,
        0.0,
        1.0,
        10,
    )
}

#[must_use]
pub fn native_reverb_predelay_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: NATIVE_REVERB_PREDELAY_PARAMETER_ID,
        name: "Predelay",
        short_name: Some("Pre"),
        value_type: ParameterValueType::PlainFloat,
        default: ParameterValue::Float(20.0),
        min: 0.0,
        max: 250.0,
        step: Some(0.1),
        unit: ParameterUnit::Milliseconds,
        flags: ParameterFlags::automatable(),
        group: Some("native.reverb"),
        order: 20,
    })
}

#[must_use]
pub fn native_reverb_decay_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: NATIVE_REVERB_DECAY_PARAMETER_ID,
        name: "Decay",
        short_name: Some("Decay"),
        value_type: ParameterValueType::Seconds,
        default: ParameterValue::Seconds(2.5),
        min: 0.1,
        max: 30.0,
        step: Some(0.01),
        unit: ParameterUnit::Seconds,
        flags: ParameterFlags::automatable_logarithmic(),
        group: Some("native.reverb"),
        order: 30,
    })
}

#[must_use]
pub fn native_reverb_damping_descriptor() -> ParameterDescriptor {
    reverb_percent_descriptor(
        NATIVE_REVERB_DAMPING_PARAMETER_ID,
        "Damping",
        "Damp",
        0.5,
        0.0,
        1.0,
        40,
    )
}

#[must_use]
pub fn native_reverb_low_cut_descriptor() -> ParameterDescriptor {
    reverb_frequency_descriptor(
        NATIVE_REVERB_LOW_CUT_PARAMETER_ID,
        "Low Cut",
        "LoCut",
        100.0,
        20.0,
        2_000.0,
        50,
    )
}

#[must_use]
pub fn native_reverb_high_cut_descriptor() -> ParameterDescriptor {
    reverb_frequency_descriptor(
        NATIVE_REVERB_HIGH_CUT_PARAMETER_ID,
        "High Cut",
        "HiCut",
        16_000.0,
        1_000.0,
        20_000.0,
        60,
    )
}

#[must_use]
pub fn native_reverb_diffusion_descriptor() -> ParameterDescriptor {
    reverb_percent_descriptor(
        NATIVE_REVERB_DIFFUSION_PARAMETER_ID,
        "Diffusion",
        "Diff",
        0.75,
        0.0,
        1.0,
        70,
    )
}

#[must_use]
pub fn native_reverb_width_descriptor() -> ParameterDescriptor {
    reverb_percent_descriptor(
        NATIVE_REVERB_WIDTH_PARAMETER_ID,
        "Width",
        "Width",
        1.0,
        0.0,
        2.0,
        80,
    )
}

#[must_use]
pub fn native_reverb_early_reflections_descriptor() -> ParameterDescriptor {
    reverb_percent_descriptor(
        NATIVE_REVERB_EARLY_REFLECTIONS_PARAMETER_ID,
        "Early Reflections",
        "Early",
        0.5,
        0.0,
        1.0,
        90,
    )
}

#[must_use]
pub fn native_reverb_mix_descriptor() -> ParameterDescriptor {
    reverb_percent_descriptor(
        NATIVE_REVERB_MIX_PARAMETER_ID,
        "Mix",
        "Mix",
        0.25,
        0.0,
        1.0,
        100,
    )
}

#[must_use]
pub fn native_reverb_output_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: NATIVE_REVERB_OUTPUT_PARAMETER_ID,
        name: "Output",
        short_name: Some("Out"),
        value_type: ParameterValueType::Decibels,
        default: ParameterValue::Decibels(0.0),
        min: -60.0,
        max: 12.0,
        step: Some(0.1),
        unit: ParameterUnit::Decibels,
        flags: ParameterFlags::automatable(),
        group: Some("native.reverb"),
        order: 110,
    })
}

#[must_use]
pub fn native_reverb_parameter_descriptors() -> Vec<ParameterDescriptor> {
    vec![
        native_reverb_size_descriptor(),
        native_reverb_predelay_descriptor(),
        native_reverb_decay_descriptor(),
        native_reverb_damping_descriptor(),
        native_reverb_low_cut_descriptor(),
        native_reverb_high_cut_descriptor(),
        native_reverb_diffusion_descriptor(),
        native_reverb_width_descriptor(),
        native_reverb_early_reflections_descriptor(),
        native_reverb_mix_descriptor(),
        native_reverb_output_descriptor(),
    ]
}

fn reverb_percent_descriptor(
    id: &'static str,
    name: &'static str,
    short_name: &'static str,
    default: f32,
    min: f32,
    max: f32,
    order: u16,
) -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id,
        name,
        short_name: Some(short_name),
        value_type: ParameterValueType::Percentage,
        default: ParameterValue::Percentage(default),
        min,
        max,
        step: Some(0.001),
        unit: ParameterUnit::Percent,
        flags: ParameterFlags::automatable(),
        group: Some("native.reverb"),
        order,
    })
}

fn reverb_frequency_descriptor(
    id: &'static str,
    name: &'static str,
    short_name: &'static str,
    default: f32,
    min: f32,
    max: f32,
    order: u16,
) -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id,
        name,
        short_name: Some(short_name),
        value_type: ParameterValueType::FrequencyHertz,
        default: ParameterValue::FrequencyHertz(default),
        min,
        max,
        step: Some(0.1),
        unit: ParameterUnit::Hertz,
        flags: ParameterFlags::automatable_logarithmic(),
        group: Some("native.reverb"),
        order,
    })
}
