use super::{
    catalog::{
        bool_descriptor, continuous_descriptor, BoolDescriptorSpec, ContinuousDescriptorSpec,
    },
    ParameterDescriptor, ParameterFlags, ParameterGroupId, ParameterId, ParameterRange,
    ParameterUnit, ParameterValue, ParameterValueType,
};

pub const NATIVE_CHORUS_RATE_PARAMETER_ID: &str = "native.chorus.rateHz";
pub const NATIVE_CHORUS_SYNC_PARAMETER_ID: &str = "native.chorus.sync";
pub const NATIVE_CHORUS_DEPTH_PARAMETER_ID: &str = "native.chorus.depth";
pub const NATIVE_CHORUS_DELAY_PARAMETER_ID: &str = "native.chorus.delayMs";
pub const NATIVE_CHORUS_VOICES_PARAMETER_ID: &str = "native.chorus.voices";
pub const NATIVE_CHORUS_SPREAD_PARAMETER_ID: &str = "native.chorus.spread";
pub const NATIVE_CHORUS_FEEDBACK_PARAMETER_ID: &str = "native.chorus.feedback";
pub const NATIVE_CHORUS_MIX_PARAMETER_ID: &str = "native.chorus.mix";
pub const NATIVE_CHORUS_OUTPUT_PARAMETER_ID: &str = "native.chorus.outputDb";
pub const NATIVE_FLANGER_RATE_PARAMETER_ID: &str = "native.flanger.rateHz";
pub const NATIVE_FLANGER_SYNC_PARAMETER_ID: &str = "native.flanger.sync";
pub const NATIVE_FLANGER_DEPTH_PARAMETER_ID: &str = "native.flanger.depth";
pub const NATIVE_FLANGER_MANUAL_PARAMETER_ID: &str = "native.flanger.manual";
pub const NATIVE_FLANGER_DELAY_PARAMETER_ID: &str = "native.flanger.delayMs";
pub const NATIVE_FLANGER_FEEDBACK_PARAMETER_ID: &str = "native.flanger.feedback";
pub const NATIVE_FLANGER_STEREO_PHASE_PARAMETER_ID: &str = "native.flanger.stereoPhase";
pub const NATIVE_FLANGER_MIX_PARAMETER_ID: &str = "native.flanger.mix";
pub const NATIVE_FLANGER_OUTPUT_PARAMETER_ID: &str = "native.flanger.outputDb";
pub const NATIVE_PHASER_RATE_PARAMETER_ID: &str = "native.phaser.rateHz";
pub const NATIVE_PHASER_SYNC_PARAMETER_ID: &str = "native.phaser.sync";
pub const NATIVE_PHASER_DEPTH_PARAMETER_ID: &str = "native.phaser.depth";
pub const NATIVE_PHASER_CENTER_PARAMETER_ID: &str = "native.phaser.centerHz";
pub const NATIVE_PHASER_STAGES_PARAMETER_ID: &str = "native.phaser.stages";
pub const NATIVE_PHASER_FEEDBACK_PARAMETER_ID: &str = "native.phaser.feedback";
pub const NATIVE_PHASER_STEREO_PHASE_PARAMETER_ID: &str = "native.phaser.stereoPhase";
pub const NATIVE_PHASER_MIX_PARAMETER_ID: &str = "native.phaser.mix";
pub const NATIVE_PHASER_OUTPUT_PARAMETER_ID: &str = "native.phaser.outputDb";

macro_rules! descriptor_fn {
    ($name:ident, $expr:expr) => {
        #[must_use]
        pub fn $name() -> ParameterDescriptor {
            $expr
        }
    };
}

descriptor_fn!(
    native_chorus_rate_descriptor,
    rate(NATIVE_CHORUS_RATE_PARAMETER_ID, "native.chorus", 10)
);
descriptor_fn!(
    native_chorus_sync_descriptor,
    sync(NATIVE_CHORUS_SYNC_PARAMETER_ID, "native.chorus", 20)
);
descriptor_fn!(
    native_chorus_depth_descriptor,
    depth(NATIVE_CHORUS_DEPTH_PARAMETER_ID, "native.chorus", 30)
);
descriptor_fn!(
    native_chorus_delay_descriptor,
    delay(
        NATIVE_CHORUS_DELAY_PARAMETER_ID,
        12.0,
        1.0,
        40.0,
        "native.chorus",
        40
    )
);
descriptor_fn!(
    native_chorus_voices_descriptor,
    integer(
        NATIVE_CHORUS_VOICES_PARAMETER_ID,
        "Voices",
        2,
        1,
        4,
        "native.chorus",
        50
    )
);
descriptor_fn!(
    native_chorus_spread_descriptor,
    percent(
        NATIVE_CHORUS_SPREAD_PARAMETER_ID,
        "Spread",
        "Spread",
        0.5,
        "native.chorus",
        60
    )
);
descriptor_fn!(
    native_chorus_feedback_descriptor,
    feedback(NATIVE_CHORUS_FEEDBACK_PARAMETER_ID, "native.chorus", 70)
);
descriptor_fn!(
    native_chorus_mix_descriptor,
    percent(
        NATIVE_CHORUS_MIX_PARAMETER_ID,
        "Mix",
        "Mix",
        0.5,
        "native.chorus",
        80
    )
);
descriptor_fn!(
    native_chorus_output_descriptor,
    output(NATIVE_CHORUS_OUTPUT_PARAMETER_ID, "native.chorus", 90)
);
descriptor_fn!(
    native_flanger_rate_descriptor,
    rate(NATIVE_FLANGER_RATE_PARAMETER_ID, "native.flanger", 10)
);
descriptor_fn!(
    native_flanger_sync_descriptor,
    sync(NATIVE_FLANGER_SYNC_PARAMETER_ID, "native.flanger", 20)
);
descriptor_fn!(
    native_flanger_depth_descriptor,
    depth(NATIVE_FLANGER_DEPTH_PARAMETER_ID, "native.flanger", 30)
);
descriptor_fn!(
    native_flanger_manual_descriptor,
    percent(
        NATIVE_FLANGER_MANUAL_PARAMETER_ID,
        "Manual",
        "Manual",
        0.5,
        "native.flanger",
        40
    )
);
descriptor_fn!(
    native_flanger_delay_descriptor,
    delay(
        NATIVE_FLANGER_DELAY_PARAMETER_ID,
        3.0,
        0.1,
        20.0,
        "native.flanger",
        50
    )
);
descriptor_fn!(
    native_flanger_feedback_descriptor,
    bipolar_feedback(NATIVE_FLANGER_FEEDBACK_PARAMETER_ID, "native.flanger", 60)
);
descriptor_fn!(
    native_flanger_stereo_phase_descriptor,
    percent(
        NATIVE_FLANGER_STEREO_PHASE_PARAMETER_ID,
        "Stereo Phase",
        "Phase",
        0.5,
        "native.flanger",
        70
    )
);
descriptor_fn!(
    native_flanger_mix_descriptor,
    percent(
        NATIVE_FLANGER_MIX_PARAMETER_ID,
        "Mix",
        "Mix",
        0.5,
        "native.flanger",
        80
    )
);
descriptor_fn!(
    native_flanger_output_descriptor,
    output(NATIVE_FLANGER_OUTPUT_PARAMETER_ID, "native.flanger", 90)
);
descriptor_fn!(
    native_phaser_rate_descriptor,
    rate(NATIVE_PHASER_RATE_PARAMETER_ID, "native.phaser", 10)
);
descriptor_fn!(
    native_phaser_sync_descriptor,
    sync(NATIVE_PHASER_SYNC_PARAMETER_ID, "native.phaser", 20)
);
descriptor_fn!(
    native_phaser_depth_descriptor,
    depth(NATIVE_PHASER_DEPTH_PARAMETER_ID, "native.phaser", 30)
);
descriptor_fn!(
    native_phaser_center_descriptor,
    center(NATIVE_PHASER_CENTER_PARAMETER_ID, "native.phaser", 40)
);
descriptor_fn!(
    native_phaser_stages_descriptor,
    integer(
        NATIVE_PHASER_STAGES_PARAMETER_ID,
        "Stages",
        4,
        2,
        12,
        "native.phaser",
        50
    )
);
descriptor_fn!(
    native_phaser_feedback_descriptor,
    bipolar_feedback(NATIVE_PHASER_FEEDBACK_PARAMETER_ID, "native.phaser", 60)
);
descriptor_fn!(
    native_phaser_stereo_phase_descriptor,
    percent(
        NATIVE_PHASER_STEREO_PHASE_PARAMETER_ID,
        "Stereo Phase",
        "Phase",
        0.5,
        "native.phaser",
        70
    )
);
descriptor_fn!(
    native_phaser_mix_descriptor,
    percent(
        NATIVE_PHASER_MIX_PARAMETER_ID,
        "Mix",
        "Mix",
        0.5,
        "native.phaser",
        80
    )
);
descriptor_fn!(
    native_phaser_output_descriptor,
    output(NATIVE_PHASER_OUTPUT_PARAMETER_ID, "native.phaser", 90)
);

#[must_use]
pub fn native_chorus_parameter_descriptors() -> Vec<ParameterDescriptor> {
    vec![
        native_chorus_rate_descriptor(),
        native_chorus_sync_descriptor(),
        native_chorus_depth_descriptor(),
        native_chorus_delay_descriptor(),
        native_chorus_voices_descriptor(),
        native_chorus_spread_descriptor(),
        native_chorus_feedback_descriptor(),
        native_chorus_mix_descriptor(),
        native_chorus_output_descriptor(),
    ]
}

#[must_use]
pub fn native_flanger_parameter_descriptors() -> Vec<ParameterDescriptor> {
    vec![
        native_flanger_rate_descriptor(),
        native_flanger_sync_descriptor(),
        native_flanger_depth_descriptor(),
        native_flanger_manual_descriptor(),
        native_flanger_delay_descriptor(),
        native_flanger_feedback_descriptor(),
        native_flanger_stereo_phase_descriptor(),
        native_flanger_mix_descriptor(),
        native_flanger_output_descriptor(),
    ]
}

#[must_use]
pub fn native_phaser_parameter_descriptors() -> Vec<ParameterDescriptor> {
    vec![
        native_phaser_rate_descriptor(),
        native_phaser_sync_descriptor(),
        native_phaser_depth_descriptor(),
        native_phaser_center_descriptor(),
        native_phaser_stages_descriptor(),
        native_phaser_feedback_descriptor(),
        native_phaser_stereo_phase_descriptor(),
        native_phaser_mix_descriptor(),
        native_phaser_output_descriptor(),
    ]
}

#[must_use]
pub fn native_modulation_parameter_descriptors() -> Vec<ParameterDescriptor> {
    let mut descriptors = native_chorus_parameter_descriptors();
    descriptors.extend(native_flanger_parameter_descriptors());
    descriptors.extend(native_phaser_parameter_descriptors());
    descriptors
}

#[must_use]
pub fn native_modulation_parameter_descriptor(id: &str) -> Option<ParameterDescriptor> {
    match id {
        NATIVE_CHORUS_RATE_PARAMETER_ID => Some(native_chorus_rate_descriptor()),
        NATIVE_CHORUS_SYNC_PARAMETER_ID => Some(native_chorus_sync_descriptor()),
        NATIVE_CHORUS_DEPTH_PARAMETER_ID => Some(native_chorus_depth_descriptor()),
        NATIVE_CHORUS_DELAY_PARAMETER_ID => Some(native_chorus_delay_descriptor()),
        NATIVE_CHORUS_VOICES_PARAMETER_ID => Some(native_chorus_voices_descriptor()),
        NATIVE_CHORUS_SPREAD_PARAMETER_ID => Some(native_chorus_spread_descriptor()),
        NATIVE_CHORUS_FEEDBACK_PARAMETER_ID => Some(native_chorus_feedback_descriptor()),
        NATIVE_CHORUS_MIX_PARAMETER_ID => Some(native_chorus_mix_descriptor()),
        NATIVE_CHORUS_OUTPUT_PARAMETER_ID => Some(native_chorus_output_descriptor()),
        NATIVE_FLANGER_RATE_PARAMETER_ID => Some(native_flanger_rate_descriptor()),
        NATIVE_FLANGER_SYNC_PARAMETER_ID => Some(native_flanger_sync_descriptor()),
        NATIVE_FLANGER_DEPTH_PARAMETER_ID => Some(native_flanger_depth_descriptor()),
        NATIVE_FLANGER_MANUAL_PARAMETER_ID => Some(native_flanger_manual_descriptor()),
        NATIVE_FLANGER_DELAY_PARAMETER_ID => Some(native_flanger_delay_descriptor()),
        NATIVE_FLANGER_FEEDBACK_PARAMETER_ID => Some(native_flanger_feedback_descriptor()),
        NATIVE_FLANGER_STEREO_PHASE_PARAMETER_ID => Some(native_flanger_stereo_phase_descriptor()),
        NATIVE_FLANGER_MIX_PARAMETER_ID => Some(native_flanger_mix_descriptor()),
        NATIVE_FLANGER_OUTPUT_PARAMETER_ID => Some(native_flanger_output_descriptor()),
        NATIVE_PHASER_RATE_PARAMETER_ID => Some(native_phaser_rate_descriptor()),
        NATIVE_PHASER_SYNC_PARAMETER_ID => Some(native_phaser_sync_descriptor()),
        NATIVE_PHASER_DEPTH_PARAMETER_ID => Some(native_phaser_depth_descriptor()),
        NATIVE_PHASER_CENTER_PARAMETER_ID => Some(native_phaser_center_descriptor()),
        NATIVE_PHASER_STAGES_PARAMETER_ID => Some(native_phaser_stages_descriptor()),
        NATIVE_PHASER_FEEDBACK_PARAMETER_ID => Some(native_phaser_feedback_descriptor()),
        NATIVE_PHASER_STEREO_PHASE_PARAMETER_ID => Some(native_phaser_stereo_phase_descriptor()),
        NATIVE_PHASER_MIX_PARAMETER_ID => Some(native_phaser_mix_descriptor()),
        NATIVE_PHASER_OUTPUT_PARAMETER_ID => Some(native_phaser_output_descriptor()),
        _ => None,
    }
}

fn rate(id: &'static str, group: &'static str, order: u16) -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id,
        name: "Rate",
        short_name: Some("Rate"),
        value_type: ParameterValueType::FrequencyHertz,
        default: ParameterValue::FrequencyHertz(0.5),
        min: 0.01,
        max: 20.0,
        step: Some(0.01),
        unit: ParameterUnit::Hertz,
        flags: ParameterFlags::automatable(),
        group: Some(group),
        order,
    })
}

fn sync(id: &'static str, group: &'static str, order: u16) -> ParameterDescriptor {
    bool_descriptor(BoolDescriptorSpec {
        id,
        name: "Sync",
        short_name: Some("Sync"),
        default: false,
        flags: ParameterFlags {
            stepped: true,
            ..ParameterFlags::automatable()
        },
        group: Some(group),
        order,
    })
}

fn depth(id: &'static str, group: &'static str, order: u16) -> ParameterDescriptor {
    percent(id, "Depth", "Depth", 0.5, group, order)
}

fn delay(
    id: &'static str,
    default: f32,
    min: f32,
    max: f32,
    group: &'static str,
    order: u16,
) -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id,
        name: "Delay",
        short_name: Some("Delay"),
        value_type: ParameterValueType::PlainFloat,
        default: ParameterValue::Float(default),
        min,
        max,
        step: Some(0.1),
        unit: ParameterUnit::Milliseconds,
        flags: ParameterFlags::automatable(),
        group: Some(group),
        order,
    })
}

fn center(id: &'static str, group: &'static str, order: u16) -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id,
        name: "Center",
        short_name: Some("Center"),
        value_type: ParameterValueType::FrequencyHertz,
        default: ParameterValue::FrequencyHertz(1_000.0),
        min: 200.0,
        max: 8_000.0,
        step: Some(0.1),
        unit: ParameterUnit::Hertz,
        flags: ParameterFlags::automatable(),
        group: Some(group),
        order,
    })
}

fn feedback(id: &'static str, group: &'static str, order: u16) -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id,
        name: "Feedback",
        short_name: Some("Fb"),
        value_type: ParameterValueType::Percentage,
        default: ParameterValue::Percentage(0.1),
        min: 0.0,
        max: 0.95,
        step: Some(0.001),
        unit: ParameterUnit::Percent,
        flags: ParameterFlags::automatable(),
        group: Some(group),
        order,
    })
}

fn bipolar_feedback(id: &'static str, group: &'static str, order: u16) -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id,
        name: "Feedback",
        short_name: Some("Fb"),
        value_type: ParameterValueType::Percentage,
        default: ParameterValue::Percentage(0.0),
        min: -0.95,
        max: 0.95,
        step: Some(0.001),
        unit: ParameterUnit::Percent,
        flags: ParameterFlags::automatable_bipolar(),
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

fn output(id: &'static str, group: &'static str, order: u16) -> ParameterDescriptor {
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

fn integer(
    id: &'static str,
    name: &'static str,
    default: i64,
    min: i64,
    max: i64,
    group: &'static str,
    order: u16,
) -> ParameterDescriptor {
    ParameterDescriptor {
        id: ParameterId::from(id),
        name: name.to_string(),
        short_name: Some(name.to_string()),
        value_type: ParameterValueType::Integer,
        default: ParameterValue::Integer(default),
        range: ParameterRange::Integer {
            min,
            max,
            step: Some(1),
        },
        unit: ParameterUnit::None,
        flags: ParameterFlags {
            stepped: true,
            ..ParameterFlags::automatable()
        },
        group: Some(ParameterGroupId::from(group)),
        order,
    }
}
