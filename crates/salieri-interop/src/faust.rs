use std::collections::HashSet;

use salieri_core::{
    NativeModuleDescriptor, NativeModuleRole, NativeModuleState, ParameterDescriptor,
    ParameterFlags, ParameterGroupId, ParameterId, ParameterRange, ParameterUnit, ParameterValue,
    ParameterValueType,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaustTargetRecommendation {
    CppNativeFirst,
    WasmWebExportLater,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FaustUiParameter {
    pub address: String,
    pub label: String,
    pub init: f32,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub unit: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FaustNativeModulePlan {
    pub descriptor: NativeModuleDescriptor,
    pub default_state: NativeModuleState,
    pub recommended_target: FaustTargetRecommendation,
    pub source_distribution_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FaustInteropError {
    #[error("Faust module id is empty")]
    EmptyModuleId,
    #[error("Faust module name is empty")]
    EmptyModuleName,
    #[error("Faust UI parameter list is empty")]
    EmptyParameterList,
    #[error("duplicate Faust UI address {0}")]
    DuplicateAddress(String),
    #[error("invalid Faust UI range for {0}")]
    InvalidRange(String),
    #[error("invalid Faust UI default for {0}")]
    InvalidDefault(String),
}

pub fn faust_ui_to_native_module_descriptor(
    module_id: &str,
    module_name: &str,
    role: NativeModuleRole,
    parameters: &[FaustUiParameter],
) -> Result<FaustNativeModulePlan, FaustInteropError> {
    if module_id.trim().is_empty() {
        return Err(FaustInteropError::EmptyModuleId);
    }
    if module_name.trim().is_empty() {
        return Err(FaustInteropError::EmptyModuleName);
    }
    if parameters.is_empty() {
        return Err(FaustInteropError::EmptyParameterList);
    }

    let mut seen = HashSet::new();
    let mut descriptors = Vec::with_capacity(parameters.len());
    for (order, parameter) in parameters.iter().enumerate() {
        if !seen.insert(parameter.address.clone()) {
            return Err(FaustInteropError::DuplicateAddress(
                parameter.address.clone(),
            ));
        }
        descriptors.push(faust_parameter_to_descriptor(parameter, order as u16)?);
    }

    let descriptor = NativeModuleDescriptor {
        id: module_id.into(),
        name: module_name.to_string(),
        role,
        parameters: descriptors,
        latency_frames: 0,
        realtime_safe: true,
    };
    let default_state = NativeModuleState::defaults_for(&descriptor);
    default_state
        .validate_against(&descriptor)
        .map_err(|_| FaustInteropError::InvalidDefault(module_id.to_string()))?;

    Ok(FaustNativeModulePlan {
        descriptor,
        default_state,
        recommended_target: FaustTargetRecommendation::CppNativeFirst,
        source_distribution_required: true,
    })
}

fn faust_parameter_to_descriptor(
    parameter: &FaustUiParameter,
    order: u16,
) -> Result<ParameterDescriptor, FaustInteropError> {
    if !parameter.min.is_finite()
        || !parameter.max.is_finite()
        || !parameter.step.is_finite()
        || parameter.min > parameter.max
        || parameter.step < 0.0
    {
        return Err(FaustInteropError::InvalidRange(parameter.address.clone()));
    }
    if !parameter.init.is_finite()
        || parameter.init < parameter.min
        || parameter.init > parameter.max
    {
        return Err(FaustInteropError::InvalidDefault(parameter.address.clone()));
    }
    let (value_type, default, unit, flags) = classify_parameter(parameter);
    Ok(ParameterDescriptor {
        id: ParameterId::new(format!("faust.{}", slugify_address(&parameter.address))),
        name: parameter.label.clone(),
        short_name: None,
        value_type,
        default,
        range: ParameterRange::Continuous {
            min: parameter.min,
            max: parameter.max,
            step: (parameter.step > 0.0).then_some(parameter.step),
        },
        unit,
        flags,
        group: Some(ParameterGroupId::from("faust")),
        order,
    })
}

fn classify_parameter(
    parameter: &FaustUiParameter,
) -> (
    ParameterValueType,
    ParameterValue,
    ParameterUnit,
    ParameterFlags,
) {
    match parameter.unit.as_deref() {
        Some("Hz" | "hz" | "hertz") => (
            ParameterValueType::FrequencyHertz,
            ParameterValue::FrequencyHertz(parameter.init),
            ParameterUnit::Hertz,
            ParameterFlags::automatable_logarithmic(),
        ),
        Some("dB" | "db" | "decibels") => (
            ParameterValueType::Decibels,
            ParameterValue::Decibels(parameter.init),
            ParameterUnit::Decibels,
            ParameterFlags::automatable(),
        ),
        Some("%" | "percent") => (
            ParameterValueType::Percentage,
            ParameterValue::Percentage(parameter.init),
            ParameterUnit::Percent,
            ParameterFlags::automatable(),
        ),
        _ if parameter.min >= 0.0 && parameter.max <= 1.0 => (
            ParameterValueType::NormalizedFloat,
            ParameterValue::Normalized(parameter.init),
            ParameterUnit::Normalized,
            ParameterFlags::automatable(),
        ),
        _ if parameter.min < 0.0 && parameter.max > 0.0 => (
            ParameterValueType::BipolarFloat,
            ParameterValue::Bipolar(parameter.init),
            ParameterUnit::Normalized,
            ParameterFlags::automatable_bipolar(),
        ),
        _ => (
            ParameterValueType::PlainFloat,
            ParameterValue::Float(parameter.init),
            ParameterUnit::None,
            ParameterFlags::automatable(),
        ),
    }
}

fn slugify_address(address: &str) -> String {
    address
        .trim_matches('/')
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '.'
            }
        })
        .collect::<String>()
        .split('.')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn representative_filter_parameters() -> Vec<FaustUiParameter> {
        vec![
            FaustUiParameter {
                address: "/filter/cutoff".to_string(),
                label: "Cutoff".to_string(),
                init: 1_200.0,
                min: 20.0,
                max: 20_000.0,
                step: 0.1,
                unit: Some("Hz".to_string()),
            },
            FaustUiParameter {
                address: "/filter/resonance".to_string(),
                label: "Resonance".to_string(),
                init: 0.25,
                min: 0.0,
                max: 1.0,
                step: 0.001,
                unit: None,
            },
            FaustUiParameter {
                address: "/filter/gain".to_string(),
                label: "Gain".to_string(),
                init: -3.0,
                min: -24.0,
                max: 24.0,
                step: 0.1,
                unit: Some("dB".to_string()),
            },
        ]
    }

    #[test]
    fn faust_ui_metadata_maps_to_native_descriptor_and_state() {
        let plan = faust_ui_to_native_module_descriptor(
            "native.effect.faust.filterPoc",
            "Faust Filter PoC",
            NativeModuleRole::Effect,
            &representative_filter_parameters(),
        )
        .expect("map faust ui");

        assert_eq!(
            plan.recommended_target,
            FaustTargetRecommendation::CppNativeFirst
        );
        assert!(plan.source_distribution_required);
        assert_eq!(plan.descriptor.parameters.len(), 3);
        assert_eq!(
            plan.descriptor.parameters[0].id.as_str(),
            "faust.filter.cutoff"
        );
        assert_eq!(plan.descriptor.parameters[0].unit, ParameterUnit::Hertz);
        assert_eq!(
            plan.descriptor.parameters[1].value_type,
            ParameterValueType::NormalizedFloat
        );
        assert_eq!(plan.default_state.parameters.len(), 3);
        plan.default_state
            .validate_against(&plan.descriptor)
            .expect("valid defaults");
    }

    #[test]
    fn faust_ui_metadata_rejects_duplicate_and_invalid_parameters() {
        let mut duplicate = representative_filter_parameters();
        duplicate[1].address = duplicate[0].address.clone();
        assert!(matches!(
            faust_ui_to_native_module_descriptor(
                "native.effect.faust.invalid",
                "Invalid",
                NativeModuleRole::Effect,
                &duplicate
            ),
            Err(FaustInteropError::DuplicateAddress(_))
        ));

        let invalid_range = [FaustUiParameter {
            address: "/bad".to_string(),
            label: "Bad".to_string(),
            init: 1.0,
            min: 2.0,
            max: 1.0,
            step: 0.1,
            unit: None,
        }];
        assert!(matches!(
            faust_ui_to_native_module_descriptor(
                "native.effect.faust.invalid",
                "Invalid",
                NativeModuleRole::Effect,
                &invalid_range
            ),
            Err(FaustInteropError::InvalidRange(_))
        ));
    }

    #[test]
    fn faust_ui_metadata_rejects_non_finite_defaults() {
        let invalid = [FaustUiParameter {
            address: "/bad".to_string(),
            label: "Bad".to_string(),
            init: f32::NAN,
            min: 0.0,
            max: 1.0,
            step: 0.1,
            unit: None,
        }];
        assert!(matches!(
            faust_ui_to_native_module_descriptor(
                "native.effect.faust.invalid",
                "Invalid",
                NativeModuleRole::Effect,
                &invalid
            ),
            Err(FaustInteropError::InvalidDefault(_))
        ));
    }
}
