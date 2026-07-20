use super::{
    drive_catalog::{
        native_bitcrusher_bit_depth_descriptor, native_bitcrusher_dither_descriptor,
        native_bitcrusher_mix_descriptor, native_bitcrusher_output_descriptor,
        native_bitcrusher_parameter_descriptors, native_bitcrusher_reduction_descriptor,
        native_drive_bias_descriptor, native_drive_drive_descriptor, native_drive_mix_descriptor,
        native_drive_mode_descriptor, native_drive_output_descriptor,
        native_drive_parameter_descriptors, native_drive_tone_descriptor,
        NATIVE_BITCRUSHER_BIT_DEPTH_PARAMETER_ID, NATIVE_BITCRUSHER_DITHER_PARAMETER_ID,
        NATIVE_BITCRUSHER_MIX_PARAMETER_ID, NATIVE_BITCRUSHER_OUTPUT_PARAMETER_ID,
        NATIVE_BITCRUSHER_REDUCTION_PARAMETER_ID, NATIVE_DRIVE_BIAS_PARAMETER_ID,
        NATIVE_DRIVE_DRIVE_PARAMETER_ID, NATIVE_DRIVE_MIX_PARAMETER_ID,
        NATIVE_DRIVE_MODE_PARAMETER_ID, NATIVE_DRIVE_OUTPUT_PARAMETER_ID,
        NATIVE_DRIVE_TONE_PARAMETER_ID,
    },
    reverb_catalog::{
        native_reverb_damping_descriptor, native_reverb_decay_descriptor,
        native_reverb_diffusion_descriptor, native_reverb_early_reflections_descriptor,
        native_reverb_high_cut_descriptor, native_reverb_low_cut_descriptor,
        native_reverb_mix_descriptor, native_reverb_output_descriptor,
        native_reverb_parameter_descriptors, native_reverb_predelay_descriptor,
        native_reverb_size_descriptor, native_reverb_width_descriptor,
        NATIVE_REVERB_DAMPING_PARAMETER_ID, NATIVE_REVERB_DECAY_PARAMETER_ID,
        NATIVE_REVERB_DIFFUSION_PARAMETER_ID, NATIVE_REVERB_EARLY_REFLECTIONS_PARAMETER_ID,
        NATIVE_REVERB_HIGH_CUT_PARAMETER_ID, NATIVE_REVERB_LOW_CUT_PARAMETER_ID,
        NATIVE_REVERB_MIX_PARAMETER_ID, NATIVE_REVERB_OUTPUT_PARAMETER_ID,
        NATIVE_REVERB_PREDELAY_PARAMETER_ID, NATIVE_REVERB_SIZE_PARAMETER_ID,
        NATIVE_REVERB_WIDTH_PARAMETER_ID,
    },
    ParameterChoice, ParameterDescriptor, ParameterFlags, ParameterGroupId, ParameterId,
    ParameterRange, ParameterUnit, ParameterValue, ParameterValueType,
};

pub const SAMPLE_GAIN_PARAMETER_ID: &str = "sample.gain";
pub const MIXER_TRACK_GAIN_PARAMETER_ID: &str = "mixer.track.gain";
pub const MIXER_TRACK_PAN_PARAMETER_ID: &str = "mixer.track.pan";
pub const MIXER_MASTER_GAIN_PARAMETER_ID: &str = "mixer.master.gain";
pub const MIXER_SEND_GAIN_PARAMETER_ID: &str = "mixer.send.gain";
pub const NATIVE_GAIN_PARAMETER_ID: &str = "native.gain.gain";
pub const NATIVE_PAN_PARAMETER_ID: &str = "native.pan.pan";
pub const NATIVE_BALANCE_PARAMETER_ID: &str = "native.balance.balance";
pub const NATIVE_WIDTH_PARAMETER_ID: &str = "native.width.width";
pub const NATIVE_PHASE_INVERT_LEFT_PARAMETER_ID: &str = "native.phase.invertLeft";
pub const NATIVE_PHASE_INVERT_RIGHT_PARAMETER_ID: &str = "native.phase.invertRight";
pub const NATIVE_FILTER_MODE_PARAMETER_ID: &str = "native.filter.mode";
pub const NATIVE_FILTER_CUTOFF_PARAMETER_ID: &str = "native.filter.cutoffHz";
pub const NATIVE_FILTER_RESONANCE_PARAMETER_ID: &str = "native.filter.resonance";
pub const NATIVE_FILTER_DRIVE_PARAMETER_ID: &str = "native.filter.driveDb";
pub const NATIVE_FILTER_KEY_TRACK_PARAMETER_ID: &str = "native.filter.keyTrack";
pub const NATIVE_FILTER_ENV_AMOUNT_PARAMETER_ID: &str = "native.filter.envAmount";
pub const NATIVE_FILTER_MIX_PARAMETER_ID: &str = "native.filter.mix";
pub const NATIVE_DELAY_SYNC_PARAMETER_ID: &str = "native.delay.sync";
pub const NATIVE_DELAY_TIME_LEFT_PARAMETER_ID: &str = "native.delay.timeLeftMs";
pub const NATIVE_DELAY_TIME_RIGHT_PARAMETER_ID: &str = "native.delay.timeRightMs";
pub const NATIVE_DELAY_LINK_TIMES_PARAMETER_ID: &str = "native.delay.linkTimes";
pub const NATIVE_DELAY_FEEDBACK_PARAMETER_ID: &str = "native.delay.feedback";
pub const NATIVE_DELAY_PING_PONG_PARAMETER_ID: &str = "native.delay.pingPong";
pub const NATIVE_DELAY_FILTER_LOW_CUT_PARAMETER_ID: &str = "native.delay.filterLowCutHz";
pub const NATIVE_DELAY_FILTER_HIGH_CUT_PARAMETER_ID: &str = "native.delay.filterHighCutHz";
pub const NATIVE_DELAY_MOD_RATE_PARAMETER_ID: &str = "native.delay.modRateHz";
pub const NATIVE_DELAY_MOD_DEPTH_PARAMETER_ID: &str = "native.delay.modDepth";
pub const NATIVE_DELAY_MIX_PARAMETER_ID: &str = "native.delay.mix";
pub const NATIVE_DELAY_OUTPUT_PARAMETER_ID: &str = "native.delay.outputDb";

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
pub fn native_balance_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: NATIVE_BALANCE_PARAMETER_ID,
        name: "Balance",
        short_name: Some("Bal"),
        value_type: ParameterValueType::BipolarFloat,
        default: ParameterValue::Bipolar(0.0),
        min: -1.0,
        max: 1.0,
        step: Some(0.001),
        unit: ParameterUnit::Pan,
        flags: ParameterFlags::automatable_bipolar(),
        group: Some("native.balance"),
        order: 10,
    })
}

#[must_use]
pub fn native_width_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: NATIVE_WIDTH_PARAMETER_ID,
        name: "Stereo Width",
        short_name: Some("Width"),
        value_type: ParameterValueType::Percentage,
        default: ParameterValue::Percentage(1.0),
        min: 0.0,
        max: 2.0,
        step: Some(0.001),
        unit: ParameterUnit::Percent,
        flags: ParameterFlags::automatable(),
        group: Some("native.width"),
        order: 10,
    })
}

#[must_use]
pub fn native_phase_invert_left_descriptor() -> ParameterDescriptor {
    bool_descriptor(BoolDescriptorSpec {
        id: NATIVE_PHASE_INVERT_LEFT_PARAMETER_ID,
        name: "Invert Left",
        short_name: Some("Inv L"),
        default: false,
        flags: ParameterFlags::automatable(),
        group: Some("native.phase"),
        order: 10,
    })
}

#[must_use]
pub fn native_phase_invert_right_descriptor() -> ParameterDescriptor {
    bool_descriptor(BoolDescriptorSpec {
        id: NATIVE_PHASE_INVERT_RIGHT_PARAMETER_ID,
        name: "Invert Right",
        short_name: Some("Inv R"),
        default: false,
        flags: ParameterFlags::automatable(),
        group: Some("native.phase"),
        order: 20,
    })
}

#[must_use]
pub fn native_filter_mode_descriptor() -> ParameterDescriptor {
    enum_descriptor(EnumDescriptorSpec {
        id: NATIVE_FILTER_MODE_PARAMETER_ID,
        name: "Filter Mode",
        short_name: Some("Mode"),
        default: "lowPass",
        choices: &[
            ("lowPass", "Low-pass"),
            ("highPass", "High-pass"),
            ("bandPass", "Band-pass"),
            ("notch", "Notch"),
        ],
        flags: ParameterFlags {
            stepped: true,
            ..ParameterFlags::automatable()
        },
        group: Some("native.filter"),
        order: 10,
    })
}

#[must_use]
pub fn native_filter_cutoff_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: NATIVE_FILTER_CUTOFF_PARAMETER_ID,
        name: "Cutoff",
        short_name: Some("Cut"),
        value_type: ParameterValueType::FrequencyHertz,
        default: ParameterValue::FrequencyHertz(12_000.0),
        min: 20.0,
        max: 24_000.0,
        step: Some(0.1),
        unit: ParameterUnit::Hertz,
        flags: ParameterFlags::automatable_logarithmic(),
        group: Some("native.filter"),
        order: 20,
    })
}

#[must_use]
pub fn native_filter_resonance_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: NATIVE_FILTER_RESONANCE_PARAMETER_ID,
        name: "Resonance",
        short_name: Some("Res"),
        value_type: ParameterValueType::NormalizedFloat,
        default: ParameterValue::Normalized(0.25),
        min: 0.0,
        max: 1.0,
        step: Some(0.001),
        unit: ParameterUnit::Normalized,
        flags: ParameterFlags::automatable(),
        group: Some("native.filter"),
        order: 30,
    })
}

#[must_use]
pub fn native_filter_drive_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: NATIVE_FILTER_DRIVE_PARAMETER_ID,
        name: "Drive",
        short_name: Some("Drv"),
        value_type: ParameterValueType::Decibels,
        default: ParameterValue::Decibels(0.0),
        min: 0.0,
        max: 24.0,
        step: Some(0.1),
        unit: ParameterUnit::Decibels,
        flags: ParameterFlags::automatable(),
        group: Some("native.filter"),
        order: 40,
    })
}

#[must_use]
pub fn native_filter_key_track_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: NATIVE_FILTER_KEY_TRACK_PARAMETER_ID,
        name: "Key Track",
        short_name: Some("Key"),
        value_type: ParameterValueType::Percentage,
        default: ParameterValue::Percentage(0.0),
        min: -1.0,
        max: 1.0,
        step: Some(0.001),
        unit: ParameterUnit::Percent,
        flags: ParameterFlags::automatable_bipolar(),
        group: Some("native.filter"),
        order: 50,
    })
}

#[must_use]
pub fn native_filter_env_amount_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: NATIVE_FILTER_ENV_AMOUNT_PARAMETER_ID,
        name: "Envelope Amount",
        short_name: Some("Env"),
        value_type: ParameterValueType::Percentage,
        default: ParameterValue::Percentage(0.0),
        min: -1.0,
        max: 1.0,
        step: Some(0.001),
        unit: ParameterUnit::Percent,
        flags: ParameterFlags::automatable_bipolar(),
        group: Some("native.filter"),
        order: 60,
    })
}

#[must_use]
pub fn native_filter_mix_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: NATIVE_FILTER_MIX_PARAMETER_ID,
        name: "Mix",
        short_name: Some("Mix"),
        value_type: ParameterValueType::Percentage,
        default: ParameterValue::Percentage(1.0),
        min: 0.0,
        max: 1.0,
        step: Some(0.001),
        unit: ParameterUnit::Percent,
        flags: ParameterFlags::automatable(),
        group: Some("native.filter"),
        order: 70,
    })
}

#[must_use]
pub fn native_delay_sync_descriptor() -> ParameterDescriptor {
    bool_descriptor(BoolDescriptorSpec {
        id: NATIVE_DELAY_SYNC_PARAMETER_ID,
        name: "Sync",
        short_name: Some("Sync"),
        default: true,
        flags: ParameterFlags {
            stepped: true,
            ..ParameterFlags::automatable()
        },
        group: Some("native.delay"),
        order: 10,
    })
}

#[must_use]
pub fn native_delay_time_left_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: NATIVE_DELAY_TIME_LEFT_PARAMETER_ID,
        name: "Left Time",
        short_name: Some("L Time"),
        value_type: ParameterValueType::PlainFloat,
        default: ParameterValue::Float(500.0),
        min: 1.0,
        max: 4_000.0,
        step: Some(0.1),
        unit: ParameterUnit::Milliseconds,
        flags: ParameterFlags::automatable_logarithmic(),
        group: Some("native.delay"),
        order: 20,
    })
}

#[must_use]
pub fn native_delay_time_right_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: NATIVE_DELAY_TIME_RIGHT_PARAMETER_ID,
        name: "Right Time",
        short_name: Some("R Time"),
        value_type: ParameterValueType::PlainFloat,
        default: ParameterValue::Float(500.0),
        min: 1.0,
        max: 4_000.0,
        step: Some(0.1),
        unit: ParameterUnit::Milliseconds,
        flags: ParameterFlags::automatable_logarithmic(),
        group: Some("native.delay"),
        order: 30,
    })
}

#[must_use]
pub fn native_delay_link_times_descriptor() -> ParameterDescriptor {
    bool_descriptor(BoolDescriptorSpec {
        id: NATIVE_DELAY_LINK_TIMES_PARAMETER_ID,
        name: "Link Times",
        short_name: Some("Link"),
        default: true,
        flags: ParameterFlags {
            stepped: true,
            ..ParameterFlags::automatable()
        },
        group: Some("native.delay"),
        order: 40,
    })
}

#[must_use]
pub fn native_delay_feedback_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: NATIVE_DELAY_FEEDBACK_PARAMETER_ID,
        name: "Feedback",
        short_name: Some("Fbk"),
        value_type: ParameterValueType::Percentage,
        default: ParameterValue::Percentage(0.35),
        min: 0.0,
        max: 0.95,
        step: Some(0.001),
        unit: ParameterUnit::Percent,
        flags: ParameterFlags::automatable(),
        group: Some("native.delay"),
        order: 50,
    })
}

#[must_use]
pub fn native_delay_ping_pong_descriptor() -> ParameterDescriptor {
    bool_descriptor(BoolDescriptorSpec {
        id: NATIVE_DELAY_PING_PONG_PARAMETER_ID,
        name: "Ping Pong",
        short_name: Some("Ping"),
        default: false,
        flags: ParameterFlags {
            stepped: true,
            ..ParameterFlags::automatable()
        },
        group: Some("native.delay"),
        order: 60,
    })
}

#[must_use]
pub fn native_delay_filter_low_cut_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: NATIVE_DELAY_FILTER_LOW_CUT_PARAMETER_ID,
        name: "Low Cut",
        short_name: Some("LoCut"),
        value_type: ParameterValueType::FrequencyHertz,
        default: ParameterValue::FrequencyHertz(20.0),
        min: 20.0,
        max: 20_000.0,
        step: Some(0.1),
        unit: ParameterUnit::Hertz,
        flags: ParameterFlags::automatable_logarithmic(),
        group: Some("native.delay"),
        order: 70,
    })
}

#[must_use]
pub fn native_delay_filter_high_cut_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: NATIVE_DELAY_FILTER_HIGH_CUT_PARAMETER_ID,
        name: "High Cut",
        short_name: Some("HiCut"),
        value_type: ParameterValueType::FrequencyHertz,
        default: ParameterValue::FrequencyHertz(20_000.0),
        min: 20.0,
        max: 20_000.0,
        step: Some(0.1),
        unit: ParameterUnit::Hertz,
        flags: ParameterFlags::automatable_logarithmic(),
        group: Some("native.delay"),
        order: 80,
    })
}

#[must_use]
pub fn native_delay_mod_rate_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: NATIVE_DELAY_MOD_RATE_PARAMETER_ID,
        name: "Mod Rate",
        short_name: Some("Rate"),
        value_type: ParameterValueType::FrequencyHertz,
        default: ParameterValue::FrequencyHertz(0.0),
        min: 0.0,
        max: 20.0,
        step: Some(0.01),
        unit: ParameterUnit::Hertz,
        flags: ParameterFlags::automatable(),
        group: Some("native.delay"),
        order: 90,
    })
}

#[must_use]
pub fn native_delay_mod_depth_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: NATIVE_DELAY_MOD_DEPTH_PARAMETER_ID,
        name: "Mod Depth",
        short_name: Some("Depth"),
        value_type: ParameterValueType::Percentage,
        default: ParameterValue::Percentage(0.0),
        min: 0.0,
        max: 1.0,
        step: Some(0.001),
        unit: ParameterUnit::Percent,
        flags: ParameterFlags::automatable(),
        group: Some("native.delay"),
        order: 100,
    })
}

#[must_use]
pub fn native_delay_mix_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: NATIVE_DELAY_MIX_PARAMETER_ID,
        name: "Mix",
        short_name: Some("Mix"),
        value_type: ParameterValueType::Percentage,
        default: ParameterValue::Percentage(0.25),
        min: 0.0,
        max: 1.0,
        step: Some(0.001),
        unit: ParameterUnit::Percent,
        flags: ParameterFlags::automatable(),
        group: Some("native.delay"),
        order: 110,
    })
}

#[must_use]
pub fn native_delay_output_descriptor() -> ParameterDescriptor {
    continuous_descriptor(ContinuousDescriptorSpec {
        id: NATIVE_DELAY_OUTPUT_PARAMETER_ID,
        name: "Output",
        short_name: Some("Out"),
        value_type: ParameterValueType::Decibels,
        default: ParameterValue::Decibels(0.0),
        min: -60.0,
        max: 12.0,
        step: Some(0.1),
        unit: ParameterUnit::Decibels,
        flags: ParameterFlags::automatable(),
        group: Some("native.delay"),
        order: 120,
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
    let mut descriptors = vec![
        native_gain_descriptor(),
        native_pan_descriptor(),
        native_balance_descriptor(),
        native_width_descriptor(),
        native_phase_invert_left_descriptor(),
        native_phase_invert_right_descriptor(),
        native_filter_mode_descriptor(),
        native_filter_cutoff_descriptor(),
        native_filter_resonance_descriptor(),
        native_filter_drive_descriptor(),
        native_filter_key_track_descriptor(),
        native_filter_env_amount_descriptor(),
        native_filter_mix_descriptor(),
        native_delay_sync_descriptor(),
        native_delay_time_left_descriptor(),
        native_delay_time_right_descriptor(),
        native_delay_link_times_descriptor(),
        native_delay_feedback_descriptor(),
        native_delay_ping_pong_descriptor(),
        native_delay_filter_low_cut_descriptor(),
        native_delay_filter_high_cut_descriptor(),
        native_delay_mod_rate_descriptor(),
        native_delay_mod_depth_descriptor(),
        native_delay_mix_descriptor(),
        native_delay_output_descriptor(),
    ];
    descriptors.extend(native_reverb_parameter_descriptors());
    descriptors.extend(native_drive_parameter_descriptors());
    descriptors.extend(native_bitcrusher_parameter_descriptors());
    descriptors
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
        NATIVE_BALANCE_PARAMETER_ID => Some(native_balance_descriptor()),
        NATIVE_WIDTH_PARAMETER_ID => Some(native_width_descriptor()),
        NATIVE_PHASE_INVERT_LEFT_PARAMETER_ID => Some(native_phase_invert_left_descriptor()),
        NATIVE_PHASE_INVERT_RIGHT_PARAMETER_ID => Some(native_phase_invert_right_descriptor()),
        NATIVE_FILTER_MODE_PARAMETER_ID => Some(native_filter_mode_descriptor()),
        NATIVE_FILTER_CUTOFF_PARAMETER_ID => Some(native_filter_cutoff_descriptor()),
        NATIVE_FILTER_RESONANCE_PARAMETER_ID => Some(native_filter_resonance_descriptor()),
        NATIVE_FILTER_DRIVE_PARAMETER_ID => Some(native_filter_drive_descriptor()),
        NATIVE_FILTER_KEY_TRACK_PARAMETER_ID => Some(native_filter_key_track_descriptor()),
        NATIVE_FILTER_ENV_AMOUNT_PARAMETER_ID => Some(native_filter_env_amount_descriptor()),
        NATIVE_FILTER_MIX_PARAMETER_ID => Some(native_filter_mix_descriptor()),
        NATIVE_DELAY_SYNC_PARAMETER_ID => Some(native_delay_sync_descriptor()),
        NATIVE_DELAY_TIME_LEFT_PARAMETER_ID => Some(native_delay_time_left_descriptor()),
        NATIVE_DELAY_TIME_RIGHT_PARAMETER_ID => Some(native_delay_time_right_descriptor()),
        NATIVE_DELAY_LINK_TIMES_PARAMETER_ID => Some(native_delay_link_times_descriptor()),
        NATIVE_DELAY_FEEDBACK_PARAMETER_ID => Some(native_delay_feedback_descriptor()),
        NATIVE_DELAY_PING_PONG_PARAMETER_ID => Some(native_delay_ping_pong_descriptor()),
        NATIVE_DELAY_FILTER_LOW_CUT_PARAMETER_ID => Some(native_delay_filter_low_cut_descriptor()),
        NATIVE_DELAY_FILTER_HIGH_CUT_PARAMETER_ID => {
            Some(native_delay_filter_high_cut_descriptor())
        }
        NATIVE_DELAY_MOD_RATE_PARAMETER_ID => Some(native_delay_mod_rate_descriptor()),
        NATIVE_DELAY_MOD_DEPTH_PARAMETER_ID => Some(native_delay_mod_depth_descriptor()),
        NATIVE_DELAY_MIX_PARAMETER_ID => Some(native_delay_mix_descriptor()),
        NATIVE_DELAY_OUTPUT_PARAMETER_ID => Some(native_delay_output_descriptor()),
        NATIVE_REVERB_SIZE_PARAMETER_ID => Some(native_reverb_size_descriptor()),
        NATIVE_REVERB_PREDELAY_PARAMETER_ID => Some(native_reverb_predelay_descriptor()),
        NATIVE_REVERB_DECAY_PARAMETER_ID => Some(native_reverb_decay_descriptor()),
        NATIVE_REVERB_DAMPING_PARAMETER_ID => Some(native_reverb_damping_descriptor()),
        NATIVE_REVERB_LOW_CUT_PARAMETER_ID => Some(native_reverb_low_cut_descriptor()),
        NATIVE_REVERB_HIGH_CUT_PARAMETER_ID => Some(native_reverb_high_cut_descriptor()),
        NATIVE_REVERB_DIFFUSION_PARAMETER_ID => Some(native_reverb_diffusion_descriptor()),
        NATIVE_REVERB_WIDTH_PARAMETER_ID => Some(native_reverb_width_descriptor()),
        NATIVE_REVERB_EARLY_REFLECTIONS_PARAMETER_ID => {
            Some(native_reverb_early_reflections_descriptor())
        }
        NATIVE_REVERB_MIX_PARAMETER_ID => Some(native_reverb_mix_descriptor()),
        NATIVE_REVERB_OUTPUT_PARAMETER_ID => Some(native_reverb_output_descriptor()),
        NATIVE_DRIVE_MODE_PARAMETER_ID => Some(native_drive_mode_descriptor()),
        NATIVE_DRIVE_DRIVE_PARAMETER_ID => Some(native_drive_drive_descriptor()),
        NATIVE_DRIVE_TONE_PARAMETER_ID => Some(native_drive_tone_descriptor()),
        NATIVE_DRIVE_BIAS_PARAMETER_ID => Some(native_drive_bias_descriptor()),
        NATIVE_DRIVE_MIX_PARAMETER_ID => Some(native_drive_mix_descriptor()),
        NATIVE_DRIVE_OUTPUT_PARAMETER_ID => Some(native_drive_output_descriptor()),
        NATIVE_BITCRUSHER_BIT_DEPTH_PARAMETER_ID => Some(native_bitcrusher_bit_depth_descriptor()),
        NATIVE_BITCRUSHER_REDUCTION_PARAMETER_ID => Some(native_bitcrusher_reduction_descriptor()),
        NATIVE_BITCRUSHER_DITHER_PARAMETER_ID => Some(native_bitcrusher_dither_descriptor()),
        NATIVE_BITCRUSHER_MIX_PARAMETER_ID => Some(native_bitcrusher_mix_descriptor()),
        NATIVE_BITCRUSHER_OUTPUT_PARAMETER_ID => Some(native_bitcrusher_output_descriptor()),
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

pub(crate) struct BoolDescriptorSpec<'a> {
    pub(crate) id: &'a str,
    pub(crate) name: &'a str,
    pub(crate) short_name: Option<&'a str>,
    pub(crate) default: bool,
    pub(crate) flags: ParameterFlags,
    pub(crate) group: Option<&'a str>,
    pub(crate) order: u16,
}

pub(crate) fn bool_descriptor(spec: BoolDescriptorSpec<'_>) -> ParameterDescriptor {
    ParameterDescriptor {
        id: ParameterId::from(spec.id),
        name: spec.name.to_string(),
        short_name: spec.short_name.map(str::to_string),
        value_type: ParameterValueType::Boolean,
        default: ParameterValue::Bool(spec.default),
        range: ParameterRange::Boolean,
        unit: ParameterUnit::Choice,
        flags: spec.flags,
        group: spec.group.map(ParameterGroupId::from),
        order: spec.order,
    }
}

pub(crate) struct EnumDescriptorSpec<'a> {
    pub(crate) id: &'a str,
    pub(crate) name: &'a str,
    pub(crate) short_name: Option<&'a str>,
    pub(crate) default: &'a str,
    pub(crate) choices: &'a [(&'a str, &'a str)],
    pub(crate) flags: ParameterFlags,
    pub(crate) group: Option<&'a str>,
    pub(crate) order: u16,
}

pub(crate) fn enum_descriptor(spec: EnumDescriptorSpec<'_>) -> ParameterDescriptor {
    ParameterDescriptor {
        id: ParameterId::from(spec.id),
        name: spec.name.to_string(),
        short_name: spec.short_name.map(str::to_string),
        value_type: ParameterValueType::Enum,
        default: ParameterValue::Enum(spec.default.to_string()),
        range: ParameterRange::Enum {
            choices: spec
                .choices
                .iter()
                .map(|(id, label)| ParameterChoice {
                    id: (*id).to_string(),
                    label: (*label).to_string(),
                })
                .collect(),
        },
        unit: ParameterUnit::Choice,
        flags: spec.flags,
        group: spec.group.map(ParameterGroupId::from),
        order: spec.order,
    }
}
