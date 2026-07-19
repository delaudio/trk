use super::{
    ParameterDescriptor, ParameterFlags, ParameterGroupId, ParameterId, ParameterRange,
    ParameterUnit, ParameterValue, ParameterValueType,
};

pub const SAMPLE_GAIN_PARAMETER_ID: &str = "sample.gain";
pub const MIXER_TRACK_GAIN_PARAMETER_ID: &str = "mixer.track.gain";
pub const MIXER_TRACK_PAN_PARAMETER_ID: &str = "mixer.track.pan";
pub const MIXER_MASTER_GAIN_PARAMETER_ID: &str = "mixer.master.gain";
pub const MIXER_SEND_GAIN_PARAMETER_ID: &str = "mixer.send.gain";
pub const NATIVE_GAIN_PARAMETER_ID: &str = "native.gain.gain";
pub const NATIVE_PAN_PARAMETER_ID: &str = "native.pan.pan";

#[must_use]
pub fn sample_gain_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: SAMPLE_GAIN_PARAMETER_ID,
        name: "Sample Gain",
        short_name: Some("Gain"),
        value_type: ParameterValueType::PlainFloat,
        default: ParameterValue::Float(1.0),
        min: 0.0,
        max: 2.0,
        step: Some(0.001),
        unit: ParameterUnit::Gain,
        flags: ParameterFlags::automatable(),
        group: Some("sampler"),
        order: 10,
    })
}

#[must_use]
pub fn mixer_track_gain_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: MIXER_TRACK_GAIN_PARAMETER_ID,
        name: "Track Gain",
        short_name: Some("Gain"),
        value_type: ParameterValueType::PlainFloat,
        default: ParameterValue::Float(1.0),
        min: 0.0,
        max: 2.0,
        step: Some(0.001),
        unit: ParameterUnit::Gain,
        flags: ParameterFlags::automatable(),
        group: Some("mixer"),
        order: 10,
    })
}

#[must_use]
pub fn mixer_track_pan_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: MIXER_TRACK_PAN_PARAMETER_ID,
        name: "Track Pan",
        short_name: Some("Pan"),
        value_type: ParameterValueType::BipolarFloat,
        default: ParameterValue::Bipolar(0.0),
        min: -1.0,
        max: 1.0,
        step: Some(0.001),
        unit: ParameterUnit::Pan,
        flags: ParameterFlags::automatable_bipolar(),
        group: Some("mixer"),
        order: 20,
    })
}

#[must_use]
pub fn mixer_master_gain_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: MIXER_MASTER_GAIN_PARAMETER_ID,
        name: "Master Gain",
        short_name: Some("Mst"),
        value_type: ParameterValueType::PlainFloat,
        default: ParameterValue::Float(1.0),
        min: 0.0,
        max: 2.0,
        step: Some(0.001),
        unit: ParameterUnit::Gain,
        flags: ParameterFlags::automatable(),
        group: Some("mixer"),
        order: 30,
    })
}

#[must_use]
pub fn mixer_send_gain_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: MIXER_SEND_GAIN_PARAMETER_ID,
        name: "Send Gain",
        short_name: Some("Send"),
        value_type: ParameterValueType::PlainFloat,
        default: ParameterValue::Float(0.0),
        min: 0.0,
        max: 2.0,
        step: Some(0.001),
        unit: ParameterUnit::Gain,
        flags: ParameterFlags::automatable(),
        group: Some("mixer"),
        order: 40,
    })
}

#[must_use]
pub fn native_gain_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: NATIVE_GAIN_PARAMETER_ID,
        name: "Gain",
        short_name: Some("Gain"),
        value_type: ParameterValueType::PlainFloat,
        default: ParameterValue::Float(1.0),
        min: 0.0,
        max: 2.0,
        step: Some(0.001),
        unit: ParameterUnit::Gain,
        flags: ParameterFlags::automatable(),
        group: Some("native.gain"),
        order: 10,
    })
}

#[must_use]
pub fn native_pan_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: NATIVE_PAN_PARAMETER_ID,
        name: "Pan",
        short_name: Some("Pan"),
        value_type: ParameterValueType::BipolarFloat,
        default: ParameterValue::Bipolar(0.0),
        min: -1.0,
        max: 1.0,
        step: Some(0.001),
        unit: ParameterUnit::Pan,
        flags: ParameterFlags::automatable_bipolar(),
        group: Some("native.pan"),
        order: 10,
    })
}

#[must_use]
pub fn sampler_parameter_descriptors() -> Vec<ParameterDescriptor> {
    vec![sample_gain_descriptor()]
}

#[must_use]
pub fn mixer_parameter_descriptors() -> Vec<ParameterDescriptor> {
    vec![
        mixer_track_gain_descriptor(),
        mixer_track_pan_descriptor(),
        mixer_master_gain_descriptor(),
        mixer_send_gain_descriptor(),
    ]
}

#[must_use]
pub fn native_effect_parameter_descriptors() -> Vec<ParameterDescriptor> {
    vec![native_gain_descriptor(), native_pan_descriptor()]
}

#[must_use]
pub fn builtin_parameter_descriptor(id: &ParameterId) -> Option<ParameterDescriptor> {
    match id.as_str() {
        SAMPLE_GAIN_PARAMETER_ID => Some(sample_gain_descriptor()),
        MIXER_TRACK_GAIN_PARAMETER_ID => Some(mixer_track_gain_descriptor()),
        MIXER_TRACK_PAN_PARAMETER_ID => Some(mixer_track_pan_descriptor()),
        MIXER_MASTER_GAIN_PARAMETER_ID => Some(mixer_master_gain_descriptor()),
        MIXER_SEND_GAIN_PARAMETER_ID => Some(mixer_send_gain_descriptor()),
        NATIVE_GAIN_PARAMETER_ID => Some(native_gain_descriptor()),
        NATIVE_PAN_PARAMETER_ID => Some(native_pan_descriptor()),
        _ => None,
    }
}

pub(crate) struct ContinuousDescriptorSpec<'a> {
    pub(crate) id: &'a str,
    pub(crate) name: &'a str,
    pub(crate) short_name: Option<&'a str>,
    pub(crate) value_type: ParameterValueType,
    pub(crate) default: ParameterValue,
    pub(crate) min: f32,
    pub(crate) max: f32,
    pub(crate) step: Option<f32>,
    pub(crate) unit: ParameterUnit,
    pub(crate) flags: ParameterFlags,
    pub(crate) group: Option<&'a str>,
    pub(crate) order: u16,
}

pub(crate) fn continuous_descriptor(spec: ContinuousDescriptorSpec<'_>) -> ParameterDescriptor {
    ParameterDescriptor {
        id: ParameterId::from(spec.id),
        name: spec.name.to_string(),
        short_name: spec.short_name.map(str::to_string),
        value_type: spec.value_type,
        default: spec.default,
        range: ParameterRange::Continuous {
            min: spec.min,
            max: spec.max,
            step: spec.step,
        },
        unit: spec.unit,
        flags: spec.flags,
        group: spec.group.map(ParameterGroupId::from),
        order: spec.order,
    }
}
