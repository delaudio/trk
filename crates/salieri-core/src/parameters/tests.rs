use super::catalog::{continuous_descriptor, ContinuousDescriptorSpec};
use super::*;

#[test]
fn continuous_descriptors_validate_clamp_and_convert_values() {
    let descriptor = native_pan_descriptor();

    descriptor
        .validate(&ParameterValue::Bipolar(-1.0))
        .expect("minimum pan is valid");
    descriptor
        .validate(&ParameterValue::Bipolar(1.0))
        .expect("maximum pan is valid");
    assert!(descriptor.validate(&ParameterValue::Bipolar(1.5)).is_err());
    assert_eq!(
        descriptor.clamp(&ParameterValue::Bipolar(1.5)),
        ParameterValue::Bipolar(1.0)
    );
    let ParameterValue::Bipolar(value) = descriptor
        .normalized_to_plain(0.25)
        .expect("normalized to plain")
    else {
        panic!("expected bipolar value");
    };
    assert!((value + 0.5).abs() < 0.0001);
    assert!(
        (descriptor
            .plain_to_normalized(&ParameterValue::Bipolar(0.5))
            .expect("plain to normalized")
            - 0.75)
            .abs()
            < f32::EPSILON
    );
}

#[test]
fn formatting_and_parsing_cover_tui_values() {
    let cutoff = continuous_descriptor(ContinuousDescriptorSpec {
        id: "native.filter.cutoff",
        name: "Cutoff",
        short_name: Some("Cut"),
        value_type: ParameterValueType::FrequencyHertz,
        default: ParameterValue::FrequencyHertz(12_000.0),
        min: 20.0,
        max: 20_000.0,
        step: Some(1.0),
        unit: ParameterUnit::Hertz,
        flags: ParameterFlags::automatable_logarithmic(),
        group: Some("native.filter"),
        order: 10,
    });
    let percent = continuous_descriptor(ContinuousDescriptorSpec {
        id: "native.delay.feedback",
        name: "Feedback",
        short_name: Some("Fbk"),
        value_type: ParameterValueType::Percentage,
        default: ParameterValue::Percentage(0.35),
        min: 0.0,
        max: 1.0,
        step: Some(0.01),
        unit: ParameterUnit::Percent,
        flags: ParameterFlags::automatable(),
        group: Some("native.delay"),
        order: 10,
    });
    let ratio = continuous_descriptor(ContinuousDescriptorSpec {
        id: "native.delay.division",
        name: "Division",
        short_name: Some("Div"),
        value_type: ParameterValueType::Ratio,
        default: ParameterValue::Ratio(0.125),
        min: 0.03125,
        max: 4.0,
        step: None,
        unit: ParameterUnit::Ratio,
        flags: ParameterFlags::automatable(),
        group: Some("native.delay"),
        order: 20,
    });

    assert_eq!(
        cutoff.format_value(&ParameterValue::FrequencyHertz(12_000.0)),
        "12.0 kHz"
    );
    assert_eq!(
        native_gain_descriptor().format_value(&ParameterValue::Float(0.501_187_2)),
        "-6.0 dB"
    );
    assert_eq!(
        percent.format_value(&ParameterValue::Percentage(0.35)),
        "35%"
    );
    assert_eq!(ratio.format_value(&ParameterValue::Ratio(0.125)), "1/8");

    assert_eq!(
        cutoff.parse_value("12.0 kHz").expect("parse cutoff"),
        ParameterValue::FrequencyHertz(12_000.0)
    );
    assert_eq!(
        percent.parse_value("35%").expect("parse percent"),
        ParameterValue::Percentage(0.35)
    );
    assert_eq!(
        ratio.parse_value("1/8").expect("parse ratio"),
        ParameterValue::Ratio(0.125)
    );
}

#[test]
fn enum_and_unknown_values_are_stable_serializable_shapes() {
    let descriptor = ParameterDescriptor {
        id: ParameterId::from("native.filter.mode"),
        name: "Mode".to_string(),
        short_name: Some("Mode".to_string()),
        value_type: ParameterValueType::Enum,
        default: ParameterValue::Enum("lowPass".to_string()),
        range: ParameterRange::Enum {
            choices: vec![
                ParameterChoice {
                    id: "lowPass".to_string(),
                    label: "Low-pass".to_string(),
                },
                ParameterChoice {
                    id: "highPass".to_string(),
                    label: "High-pass".to_string(),
                },
            ],
        },
        unit: ParameterUnit::Choice,
        flags: ParameterFlags {
            stepped: true,
            ..ParameterFlags::automatable()
        },
        group: Some(ParameterGroupId::from("native.filter")),
        order: 1,
    };

    assert_eq!(
        descriptor.parse_value("High-pass").expect("parse enum"),
        ParameterValue::Enum("highPass".to_string())
    );
    assert_eq!(
        descriptor.format_value(&ParameterValue::Enum("lowPass".to_string())),
        "Low-pass"
    );

    let unknown = ParameterValue::Unknown {
        value_type: "futureShape".to_string(),
        raw: "opaque".to_string(),
    };
    let serialized = serde_json::to_string(&unknown).expect("serialize unknown");
    assert!(serialized.contains("futureShape"));
    assert!(serialized.contains("opaque"));
}

#[test]
fn builtin_catalogs_have_unique_stable_ids() {
    let descriptors = sampler_parameter_descriptors()
        .into_iter()
        .chain(mixer_parameter_descriptors())
        .chain(native_effect_parameter_descriptors())
        .collect::<Vec<_>>();
    let mut ids = std::collections::HashSet::new();

    for descriptor in descriptors {
        assert!(
            ids.insert(descriptor.id.clone()),
            "duplicate {}",
            descriptor.id
        );
        assert!(builtin_parameter_descriptor(&descriptor.id).is_some());
    }
}
