use super::{
    ParameterChoice, ParameterDescriptor, ParameterFlags, ParameterGroupId, ParameterId,
    ParameterRange, ParameterUnit, ParameterValue, ParameterValueType,
};

pub const SAMPLE_ROOT_NOTE_PARAMETER_ID: &str = "sample.rootNote";
pub const SAMPLE_PLAYBACK_MODE_PARAMETER_ID: &str = "sample.playback.mode";
pub const SAMPLE_START_FRAME_PARAMETER_ID: &str = "sample.playback.startFrame";
pub const SAMPLE_END_FRAME_PARAMETER_ID: &str = "sample.playback.endFrame";
pub const SAMPLE_LOOP_START_FRAME_PARAMETER_ID: &str = "sample.playback.loopStartFrame";
pub const SAMPLE_LOOP_END_FRAME_PARAMETER_ID: &str = "sample.playback.loopEndFrame";
pub const SAMPLE_ENVELOPE_ATTACK_PARAMETER_ID: &str = "sample.envelope.attackS";
pub const SAMPLE_ENVELOPE_DECAY_PARAMETER_ID: &str = "sample.envelope.decayS";
pub const SAMPLE_ENVELOPE_SUSTAIN_PARAMETER_ID: &str = "sample.envelope.sustain";
pub const SAMPLE_ENVELOPE_RELEASE_PARAMETER_ID: &str = "sample.envelope.releaseS";

const MAX_SAMPLE_FRAME: i64 = i32::MAX as i64;
pub const MAX_SAMPLE_ENVELOPE_SECONDS: f32 = 60.0;

#[must_use]
pub fn sample_root_note_descriptor() -> ParameterDescriptor {
    integer_descriptor(IntegerDescriptorSpec {
        id: SAMPLE_ROOT_NOTE_PARAMETER_ID,
        name: "Sample Root Note",
        short_name: Some("Root"),
        value_type: ParameterValueType::Note,
        default: ParameterValue::Note(60),
        min: 0,
        max: 127,
        step: Some(1),
        unit: ParameterUnit::Note,
        flags: ParameterFlags {
            stepped: true,
            ..ParameterFlags::default()
        },
        group: Some("sampler"),
        order: 20,
    })
}

#[must_use]
pub fn sample_playback_mode_descriptor() -> ParameterDescriptor {
    ParameterDescriptor {
        id: ParameterId::from(SAMPLE_PLAYBACK_MODE_PARAMETER_ID),
        name: "Sample Playback Mode".to_string(),
        short_name: Some("Mode".to_string()),
        value_type: ParameterValueType::Enum,
        default: ParameterValue::Enum("oneShot".to_string()),
        range: ParameterRange::Enum {
            choices: vec![
                ParameterChoice {
                    id: "oneShot".to_string(),
                    label: "One-shot".to_string(),
                },
                ParameterChoice {
                    id: "loop".to_string(),
                    label: "Loop".to_string(),
                },
            ],
        },
        unit: ParameterUnit::Choice,
        flags: ParameterFlags {
            stepped: true,
            ..ParameterFlags::default()
        },
        group: Some(ParameterGroupId::from("sampler.playback")),
        order: 30,
    }
}

#[must_use]
pub fn sample_start_frame_descriptor() -> ParameterDescriptor {
    sample_frame_descriptor(
        SAMPLE_START_FRAME_PARAMETER_ID,
        "Sample Start Frame",
        Some("Start"),
        40,
    )
}

#[must_use]
pub fn sample_end_frame_descriptor() -> ParameterDescriptor {
    sample_frame_descriptor(
        SAMPLE_END_FRAME_PARAMETER_ID,
        "Sample End Frame",
        Some("End"),
        50,
    )
}

#[must_use]
pub fn sample_loop_start_frame_descriptor() -> ParameterDescriptor {
    sample_frame_descriptor(
        SAMPLE_LOOP_START_FRAME_PARAMETER_ID,
        "Sample Loop Start Frame",
        Some("Loop Start"),
        60,
    )
}

#[must_use]
pub fn sample_loop_end_frame_descriptor() -> ParameterDescriptor {
    sample_frame_descriptor(
        SAMPLE_LOOP_END_FRAME_PARAMETER_ID,
        "Sample Loop End Frame",
        Some("Loop End"),
        70,
    )
}

#[must_use]
pub fn sample_envelope_attack_descriptor() -> ParameterDescriptor {
    sample_envelope_seconds_descriptor(
        SAMPLE_ENVELOPE_ATTACK_PARAMETER_ID,
        "Sample Envelope Attack",
        Some("Attack"),
        80,
    )
}

#[must_use]
pub fn sample_envelope_decay_descriptor() -> ParameterDescriptor {
    sample_envelope_seconds_descriptor(
        SAMPLE_ENVELOPE_DECAY_PARAMETER_ID,
        "Sample Envelope Decay",
        Some("Decay"),
        90,
    )
}

#[must_use]
pub fn sample_envelope_sustain_descriptor() -> ParameterDescriptor {
    ParameterDescriptor {
        id: ParameterId::from(SAMPLE_ENVELOPE_SUSTAIN_PARAMETER_ID),
        name: "Sample Envelope Sustain".to_string(),
        short_name: Some("Sustain".to_string()),
        value_type: ParameterValueType::Percentage,
        default: ParameterValue::Percentage(1.0),
        range: ParameterRange::Continuous {
            min: 0.0,
            max: 1.0,
            step: Some(0.001),
        },
        unit: ParameterUnit::Percent,
        flags: ParameterFlags::default(),
        group: Some(ParameterGroupId::from("sampler.envelope")),
        order: 100,
    }
}

#[must_use]
pub fn sample_envelope_release_descriptor() -> ParameterDescriptor {
    sample_envelope_seconds_descriptor(
        SAMPLE_ENVELOPE_RELEASE_PARAMETER_ID,
        "Sample Envelope Release",
        Some("Release"),
        110,
    )
}

#[must_use]
pub fn sampler_playback_parameter_descriptors() -> Vec<ParameterDescriptor> {
    vec![
        sample_root_note_descriptor(),
        sample_playback_mode_descriptor(),
        sample_start_frame_descriptor(),
        sample_end_frame_descriptor(),
        sample_loop_start_frame_descriptor(),
        sample_loop_end_frame_descriptor(),
        sample_envelope_attack_descriptor(),
        sample_envelope_decay_descriptor(),
        sample_envelope_sustain_descriptor(),
        sample_envelope_release_descriptor(),
    ]
}

#[must_use]
pub fn sampler_playback_parameter_descriptor(id: &str) -> Option<ParameterDescriptor> {
    sampler_playback_parameter_descriptors()
        .into_iter()
        .find(|descriptor| descriptor.id.as_str() == id)
}

struct IntegerDescriptorSpec<'a> {
    id: &'a str,
    name: &'a str,
    short_name: Option<&'a str>,
    value_type: ParameterValueType,
    default: ParameterValue,
    min: i64,
    max: i64,
    step: Option<u64>,
    unit: ParameterUnit,
    flags: ParameterFlags,
    group: Option<&'a str>,
    order: u16,
}

fn integer_descriptor(spec: IntegerDescriptorSpec<'_>) -> ParameterDescriptor {
    ParameterDescriptor {
        id: ParameterId::from(spec.id),
        name: spec.name.to_string(),
        short_name: spec.short_name.map(str::to_string),
        value_type: spec.value_type,
        default: spec.default,
        range: ParameterRange::Integer {
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

fn sample_frame_descriptor(
    id: &'static str,
    name: &'static str,
    short_name: Option<&'static str>,
    order: u16,
) -> ParameterDescriptor {
    integer_descriptor(IntegerDescriptorSpec {
        id,
        name,
        short_name,
        value_type: ParameterValueType::Integer,
        default: ParameterValue::Integer(0),
        min: 0,
        max: MAX_SAMPLE_FRAME,
        step: Some(1),
        unit: ParameterUnit::None,
        flags: ParameterFlags {
            stepped: true,
            advanced: true,
            ..ParameterFlags::default()
        },
        group: Some("sampler.playback"),
        order,
    })
}

fn sample_envelope_seconds_descriptor(
    id: &'static str,
    name: &'static str,
    short_name: Option<&'static str>,
    order: u16,
) -> ParameterDescriptor {
    ParameterDescriptor {
        id: ParameterId::from(id),
        name: name.to_string(),
        short_name: short_name.map(str::to_string),
        value_type: ParameterValueType::Seconds,
        default: ParameterValue::Seconds(0.0),
        range: ParameterRange::Continuous {
            min: 0.0,
            max: MAX_SAMPLE_ENVELOPE_SECONDS,
            step: None,
        },
        unit: ParameterUnit::Seconds,
        flags: ParameterFlags::default(),
        group: Some(ParameterGroupId::from("sampler.envelope")),
        order,
    }
}
