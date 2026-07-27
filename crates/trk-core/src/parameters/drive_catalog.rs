use super::{
    catalog::{
        bool_descriptor, continuous_descriptor, enum_descriptor, BoolDescriptorSpec,
        ContinuousDescriptorSpec, EnumDescriptorSpec,
    },
    ParameterDescriptor, ParameterFlags, ParameterGroupId, ParameterId, ParameterRange,
    ParameterUnit, ParameterValue, ParameterValueType,
};

pub const NATIVE_DRIVE_MODE_PARAMETER_ID: &str = "native.drive.mode";
pub const NATIVE_DRIVE_DRIVE_PARAMETER_ID: &str = "native.drive.driveDb";
pub const NATIVE_DRIVE_TONE_PARAMETER_ID: &str = "native.drive.tone";
pub const NATIVE_DRIVE_BIAS_PARAMETER_ID: &str = "native.drive.bias";
pub const NATIVE_DRIVE_MIX_PARAMETER_ID: &str = "native.drive.mix";
pub const NATIVE_DRIVE_OUTPUT_PARAMETER_ID: &str = "native.drive.outputDb";
pub const NATIVE_BITCRUSHER_BIT_DEPTH_PARAMETER_ID: &str = "native.bitcrusher.bitDepth";
pub const NATIVE_BITCRUSHER_REDUCTION_PARAMETER_ID: &str = "native.bitcrusher.reductionRatio";
pub const NATIVE_BITCRUSHER_DITHER_PARAMETER_ID: &str = "native.bitcrusher.dither";
pub const NATIVE_BITCRUSHER_MIX_PARAMETER_ID: &str = "native.bitcrusher.mix";
pub const NATIVE_BITCRUSHER_OUTPUT_PARAMETER_ID: &str = "native.bitcrusher.outputDb";

#[must_use]
pub fn native_drive_mode_descriptor() -> ParameterDescriptor {
    enum_descriptor(EnumDescriptorSpec {
        id: NATIVE_DRIVE_MODE_PARAMETER_ID,
        name: "Drive Mode",
        short_name: Some("Mode"),
        default: "overdrive",
        choices: &[
            ("overdrive", "Overdrive"),
            ("saturation", "Saturation"),
            ("hardClip", "Hard clip"),
            ("softClip", "Soft clip"),
        ],
        flags: ParameterFlags {
            stepped: true,
            ..ParameterFlags::automatable()
        },
        group: Some("native.drive"),
        order: 10,
    })
}

#[must_use]
pub fn native_drive_drive_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: NATIVE_DRIVE_DRIVE_PARAMETER_ID,
        name: "Drive",
        short_name: Some("Drive"),
        value_type: ParameterValueType::Decibels,
        default: ParameterValue::Decibels(12.0),
        min: 0.0,
        max: 48.0,
        step: Some(0.1),
        unit: ParameterUnit::Decibels,
        flags: ParameterFlags::automatable(),
        group: Some("native.drive"),
        order: 20,
    })
}

#[must_use]
pub fn native_drive_tone_descriptor() -> ParameterDescriptor {
    percent_descriptor(
        NATIVE_DRIVE_TONE_PARAMETER_ID,
        "Tone",
        "Tone",
        0.5,
        "native.drive",
        30,
    )
}

#[must_use]
pub fn native_drive_bias_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: NATIVE_DRIVE_BIAS_PARAMETER_ID,
        name: "Bias",
        short_name: Some("Bias"),
        value_type: ParameterValueType::Percentage,
        default: ParameterValue::Percentage(0.0),
        min: -1.0,
        max: 1.0,
        step: Some(0.001),
        unit: ParameterUnit::Percent,
        flags: ParameterFlags::automatable_bipolar(),
        group: Some("native.drive"),
        order: 40,
    })
}

#[must_use]
pub fn native_drive_mix_descriptor() -> ParameterDescriptor {
    percent_descriptor(
        NATIVE_DRIVE_MIX_PARAMETER_ID,
        "Mix",
        "Mix",
        1.0,
        "native.drive",
        50,
    )
}

#[must_use]
pub fn native_drive_output_descriptor() -> ParameterDescriptor {
    output_descriptor(NATIVE_DRIVE_OUTPUT_PARAMETER_ID, "native.drive", 60)
}

#[must_use]
pub fn native_bitcrusher_bit_depth_descriptor() -> ParameterDescriptor {
    ParameterDescriptor {
        id: ParameterId::from(NATIVE_BITCRUSHER_BIT_DEPTH_PARAMETER_ID),
        name: "Bit Depth".to_string(),
        short_name: Some("Bits".to_string()),
        value_type: ParameterValueType::Integer,
        default: ParameterValue::Integer(12),
        range: ParameterRange::Integer {
            min: 1,
            max: 24,
            step: Some(1),
        },
        unit: ParameterUnit::None,
        flags: ParameterFlags {
            stepped: true,
            ..ParameterFlags::automatable()
        },
        group: Some(ParameterGroupId::from("native.bitcrusher")),
        order: 10,
    }
}

#[must_use]
pub fn native_bitcrusher_reduction_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: NATIVE_BITCRUSHER_REDUCTION_PARAMETER_ID,
        name: "Reduction",
        short_name: Some("Rate"),
        value_type: ParameterValueType::Ratio,
        default: ParameterValue::Ratio(1.0),
        min: 1.0,
        max: 64.0,
        step: Some(1.0),
        unit: ParameterUnit::Ratio,
        flags: ParameterFlags {
            stepped: true,
            ..ParameterFlags::automatable()
        },
        group: Some("native.bitcrusher"),
        order: 20,
    })
}

#[must_use]
pub fn native_bitcrusher_dither_descriptor() -> ParameterDescriptor {
    bool_descriptor(BoolDescriptorSpec {
        id: NATIVE_BITCRUSHER_DITHER_PARAMETER_ID,
        name: "Dither",
        short_name: Some("Dith"),
        default: false,
        flags: ParameterFlags {
            stepped: true,
            ..ParameterFlags::automatable()
        },
        group: Some("native.bitcrusher"),
        order: 30,
    })
}

#[must_use]
pub fn native_bitcrusher_mix_descriptor() -> ParameterDescriptor {
    percent_descriptor(
        NATIVE_BITCRUSHER_MIX_PARAMETER_ID,
        "Mix",
        "Mix",
        1.0,
        "native.bitcrusher",
        40,
    )
}

#[must_use]
pub fn native_bitcrusher_output_descriptor() -> ParameterDescriptor {
    output_descriptor(
        NATIVE_BITCRUSHER_OUTPUT_PARAMETER_ID,
        "native.bitcrusher",
        50,
    )
}

#[must_use]
pub fn native_drive_parameter_descriptors() -> Vec<ParameterDescriptor> {
    vec![
        native_drive_mode_descriptor(),
        native_drive_drive_descriptor(),
        native_drive_tone_descriptor(),
        native_drive_bias_descriptor(),
        native_drive_mix_descriptor(),
        native_drive_output_descriptor(),
    ]
}

#[must_use]
pub fn native_bitcrusher_parameter_descriptors() -> Vec<ParameterDescriptor> {
    vec![
        native_bitcrusher_bit_depth_descriptor(),
        native_bitcrusher_reduction_descriptor(),
        native_bitcrusher_dither_descriptor(),
        native_bitcrusher_mix_descriptor(),
        native_bitcrusher_output_descriptor(),
    ]
}

fn percent_descriptor(
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

fn output_descriptor(id: &'static str, group: &'static str, order: u16) -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id,
        name: "Output",
        short_name: Some("Out"),
        value_type: ParameterValueType::Decibels,
        default: ParameterValue::Decibels(0.0),
        min: -60.0,
        max: 12.0,
        step: Some(0.1),
        unit: ParameterUnit::Decibels,
        flags: ParameterFlags::automatable(),
        group: Some(group),
        order,
    })
}
