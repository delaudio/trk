use std::fmt;

use serde::{Deserialize, Serialize};

mod catalog;
mod drive_catalog;
mod dynamics_catalog;
mod modulation_catalog;
mod reverb_catalog;
#[cfg(test)]
mod tests;

pub use catalog::*;
pub use drive_catalog::*;
pub use dynamics_catalog::*;
pub use modulation_catalog::*;
pub use reverb_catalog::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ParameterId(pub String);

impl ParameterId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ParameterId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for ParameterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ParameterGroupId(pub String);

impl From<&str> for ParameterGroupId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ParameterValueType {
    NormalizedFloat,
    PlainFloat,
    BipolarFloat,
    Integer,
    Boolean,
    Enum,
    Note,
    Semitones,
    Seconds,
    FrequencyHertz,
    Decibels,
    Percentage,
    Ratio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ParameterUnit {
    None,
    Normalized,
    Gain,
    Decibels,
    Pan,
    Percent,
    Hertz,
    Seconds,
    Milliseconds,
    Semitones,
    Ratio,
    Note,
    Choice,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum ParameterValue {
    Normalized(f32),
    Float(f32),
    Bipolar(f32),
    Integer(i64),
    Bool(bool),
    Enum(String),
    Note(u8),
    Semitones(f32),
    Seconds(f32),
    FrequencyHertz(f32),
    Decibels(f32),
    Percentage(f32),
    Ratio(f32),
    Unknown { value_type: String, raw: String },
}

impl ParameterValue {
    #[must_use]
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Self::Normalized(value)
            | Self::Float(value)
            | Self::Bipolar(value)
            | Self::Semitones(value)
            | Self::Seconds(value)
            | Self::FrequencyHertz(value)
            | Self::Decibels(value)
            | Self::Percentage(value)
            | Self::Ratio(value) => Some(*value),
            Self::Integer(value) => Some(*value as f32),
            Self::Note(value) => Some(f32::from(*value)),
            Self::Bool(_) | Self::Enum(_) | Self::Unknown { .. } => None,
        }
    }

    #[must_use]
    pub fn with_numeric_value(&self, value: f32) -> Self {
        match self {
            Self::Normalized(_) => Self::Normalized(value),
            Self::Float(_) => Self::Float(value),
            Self::Bipolar(_) => Self::Bipolar(value),
            Self::Integer(_) => Self::Integer(value.round() as i64),
            Self::Note(_) => Self::Note(value.round().clamp(0.0, 127.0) as u8),
            Self::Semitones(_) => Self::Semitones(value),
            Self::Seconds(_) => Self::Seconds(value),
            Self::FrequencyHertz(_) => Self::FrequencyHertz(value),
            Self::Decibels(_) => Self::Decibels(value),
            Self::Percentage(_) => Self::Percentage(value),
            Self::Ratio(_) => Self::Ratio(value),
            Self::Bool(_) | Self::Enum(_) | Self::Unknown { .. } => self.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterChoice {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ParameterRange {
    Continuous {
        min: f32,
        max: f32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        step: Option<f32>,
    },
    Integer {
        min: i64,
        max: i64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        step: Option<u64>,
    },
    Boolean,
    Enum {
        choices: Vec<ParameterChoice>,
    },
    Unbounded,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterFlags {
    #[serde(default)]
    pub automatable: bool,
    #[serde(default)]
    pub modulatable: bool,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub advanced: bool,
    #[serde(default)]
    pub bipolar: bool,
    #[serde(default)]
    pub logarithmic: bool,
    #[serde(default)]
    pub stepped: bool,
    #[serde(default)]
    pub per_voice: bool,
}

impl ParameterFlags {
    #[must_use]
    pub const fn automatable() -> Self {
        Self {
            automatable: true,
            modulatable: false,
            read_only: false,
            hidden: false,
            advanced: false,
            bipolar: false,
            logarithmic: false,
            stepped: false,
            per_voice: false,
        }
    }

    #[must_use]
    pub const fn automatable_bipolar() -> Self {
        Self {
            bipolar: true,
            ..Self::automatable()
        }
    }

    #[must_use]
    pub const fn automatable_logarithmic() -> Self {
        Self {
            logarithmic: true,
            ..Self::automatable()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterDescriptor {
    pub id: ParameterId,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_name: Option<String>,
    pub value_type: ParameterValueType,
    pub default: ParameterValue,
    pub range: ParameterRange,
    pub unit: ParameterUnit,
    pub flags: ParameterFlags,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<ParameterGroupId>,
    pub order: u16,
}

impl ParameterDescriptor {
    pub fn validate(&self, value: &ParameterValue) -> Result<(), ParameterValidationError> {
        if !self.value_matches_type(value) {
            return Err(ParameterValidationError::WrongType {
                id: self.id.clone(),
                expected: self.value_type,
            });
        }
        match (&self.range, value) {
            (ParameterRange::Continuous { min, max, step }, _) => {
                let Some(value) = value.as_f32() else {
                    return Err(ParameterValidationError::WrongType {
                        id: self.id.clone(),
                        expected: self.value_type,
                    });
                };
                if !value.is_finite() || value < *min || value > *max {
                    return Err(ParameterValidationError::OutOfRange {
                        id: self.id.clone(),
                    });
                }
                if let Some(step) = step {
                    if *step > 0.0 && !is_aligned_to_step(value, *min, *step) {
                        return Err(ParameterValidationError::NotOnStep {
                            id: self.id.clone(),
                        });
                    }
                }
                Ok(())
            }
            (ParameterRange::Integer { min, max, step }, ParameterValue::Integer(value)) => {
                if value < min || value > max {
                    return Err(ParameterValidationError::OutOfRange {
                        id: self.id.clone(),
                    });
                }
                if let Some(step) = step {
                    let offset = value.saturating_sub(*min);
                    if *step > 0 && offset % (*step as i64) != 0 {
                        return Err(ParameterValidationError::NotOnStep {
                            id: self.id.clone(),
                        });
                    }
                }
                Ok(())
            }
            (ParameterRange::Integer { min, max, step }, ParameterValue::Note(value)) => {
                let value = i64::from(*value);
                if value < *min || value > *max {
                    return Err(ParameterValidationError::OutOfRange {
                        id: self.id.clone(),
                    });
                }
                if let Some(step) = step {
                    let offset = value.saturating_sub(*min);
                    if *step > 0 && offset % (*step as i64) != 0 {
                        return Err(ParameterValidationError::NotOnStep {
                            id: self.id.clone(),
                        });
                    }
                }
                Ok(())
            }
            (ParameterRange::Boolean, ParameterValue::Bool(_)) => Ok(()),
            (ParameterRange::Enum { choices }, ParameterValue::Enum(value)) => {
                if choices.iter().any(|choice| choice.id == *value) {
                    Ok(())
                } else {
                    Err(ParameterValidationError::InvalidChoice {
                        id: self.id.clone(),
                    })
                }
            }
            (ParameterRange::Unbounded, _) => {
                if value.as_f32().is_none_or(f32::is_finite) {
                    Ok(())
                } else {
                    Err(ParameterValidationError::OutOfRange {
                        id: self.id.clone(),
                    })
                }
            }
            _ => Err(ParameterValidationError::WrongType {
                id: self.id.clone(),
                expected: self.value_type,
            }),
        }
    }

    #[must_use]
    pub fn validate_f32(&self, value: f32) -> bool {
        self.validate(&self.value_from_f32(value)).is_ok()
    }

    #[must_use]
    pub fn value_from_f32(&self, value: f32) -> ParameterValue {
        match self.value_type {
            ParameterValueType::NormalizedFloat => ParameterValue::Normalized(value),
            ParameterValueType::PlainFloat => ParameterValue::Float(value),
            ParameterValueType::BipolarFloat => ParameterValue::Bipolar(value),
            ParameterValueType::Integer => ParameterValue::Integer(value.round() as i64),
            ParameterValueType::Note => ParameterValue::Note(value.round().clamp(0.0, 127.0) as u8),
            ParameterValueType::Semitones => ParameterValue::Semitones(value),
            ParameterValueType::Seconds => ParameterValue::Seconds(value),
            ParameterValueType::FrequencyHertz => ParameterValue::FrequencyHertz(value),
            ParameterValueType::Decibels => ParameterValue::Decibels(value),
            ParameterValueType::Percentage => ParameterValue::Percentage(value),
            ParameterValueType::Ratio => ParameterValue::Ratio(value),
            ParameterValueType::Boolean => ParameterValue::Bool(value >= 0.5),
            ParameterValueType::Enum => ParameterValue::Enum(value.round().to_string()),
        }
    }

    #[must_use]
    pub fn clamp(&self, value: &ParameterValue) -> ParameterValue {
        match &self.range {
            ParameterRange::Continuous { min, max, step } => {
                let mut value = value.as_f32().unwrap_or(*min);
                if !value.is_finite() {
                    value = *min;
                }
                value = value.clamp(*min, *max);
                if let Some(step) = step {
                    if *step > 0.0 {
                        value = ((value - *min) / *step).round().mul_add(*step, *min);
                        value = value.clamp(*min, *max);
                    }
                }
                self.value_from_f32(value)
            }
            ParameterRange::Integer { min, max, step } => {
                let value = value.as_f32().unwrap_or(*min as f32);
                let mut value = value.round().clamp(*min as f32, *max as f32) as i64;
                if let Some(step) = step {
                    if *step > 0 {
                        let offset = value.saturating_sub(*min) as f32;
                        value = ((offset / *step as f32).round() as i64)
                            .saturating_mul(*step as i64)
                            .saturating_add(*min);
                        value = value.clamp(*min, *max);
                    }
                }
                match self.value_type {
                    ParameterValueType::Note => ParameterValue::Note(value.clamp(0, 127) as u8),
                    _ => ParameterValue::Integer(value),
                }
            }
            ParameterRange::Boolean => {
                if let ParameterValue::Bool(value) = value {
                    ParameterValue::Bool(*value)
                } else {
                    self.default.clone()
                }
            }
            ParameterRange::Enum { choices } => {
                let ParameterValue::Enum(value) = value else {
                    return self.default.clone();
                };
                if choices.iter().any(|choice| choice.id == *value) {
                    ParameterValue::Enum(value.clone())
                } else {
                    self.default.clone()
                }
            }
            ParameterRange::Unbounded => value.clone(),
        }
    }

    pub fn normalized_to_plain(
        &self,
        normalized: f32,
    ) -> Result<ParameterValue, ParameterValidationError> {
        if !normalized.is_finite() {
            return Err(ParameterValidationError::OutOfRange {
                id: self.id.clone(),
            });
        }
        let normalized = normalized.clamp(0.0, 1.0);
        match self.range {
            ParameterRange::Continuous { min, max, step } => {
                let mut value = if self.flags.logarithmic {
                    if min <= 0.0 || max <= 0.0 {
                        return Err(ParameterValidationError::InvalidRange {
                            id: self.id.clone(),
                        });
                    }
                    let min = min.ln();
                    let max = max.ln();
                    normalized.mul_add(max - min, min).exp()
                } else {
                    normalized.mul_add(max - min, min)
                };
                if let Some(step) = step {
                    if step > 0.0 {
                        value = ((value - min) / step).round().mul_add(step, min);
                    }
                }
                Ok(self.clamp(&self.value_from_f32(value)))
            }
            ParameterRange::Integer { min, max, step } => {
                let mut value = normalized.mul_add((max - min) as f32, min as f32).round() as i64;
                if let Some(step) = step {
                    if step > 0 {
                        let offset = value.saturating_sub(min);
                        value = ((offset as f32 / step as f32).round() as i64)
                            .saturating_mul(step as i64)
                            .saturating_add(min);
                    }
                }
                Ok(self.clamp(&self.value_from_f32(value as f32)))
            }
            _ => Err(ParameterValidationError::NotNormalizable {
                id: self.id.clone(),
            }),
        }
    }

    pub fn plain_to_normalized(
        &self,
        value: &ParameterValue,
    ) -> Result<f32, ParameterValidationError> {
        self.validate(value)?;
        match self.range {
            ParameterRange::Continuous { min, max, .. } => {
                if min >= max {
                    return Err(ParameterValidationError::InvalidRange {
                        id: self.id.clone(),
                    });
                }
                let value = value.as_f32().expect("validated numeric value");
                if self.flags.logarithmic {
                    if min <= 0.0 || max <= 0.0 || value <= 0.0 {
                        return Err(ParameterValidationError::InvalidRange {
                            id: self.id.clone(),
                        });
                    }
                    Ok(((value.ln() - min.ln()) / (max.ln() - min.ln())).clamp(0.0, 1.0))
                } else {
                    Ok(((value - min) / (max - min)).clamp(0.0, 1.0))
                }
            }
            ParameterRange::Integer { min, max, .. } => {
                if min >= max {
                    return Err(ParameterValidationError::InvalidRange {
                        id: self.id.clone(),
                    });
                }
                let value = value.as_f32().expect("validated numeric value");
                Ok(((value - min as f32) / (max - min) as f32).clamp(0.0, 1.0))
            }
            _ => Err(ParameterValidationError::NotNormalizable {
                id: self.id.clone(),
            }),
        }
    }

    #[must_use]
    pub fn format_value(&self, value: &ParameterValue) -> String {
        match (&self.range, value, self.unit) {
            (ParameterRange::Enum { choices }, ParameterValue::Enum(value), _) => choices
                .iter()
                .find(|choice| choice.id == *value)
                .map_or_else(|| value.clone(), |choice| choice.label.clone()),
            (_, ParameterValue::Bool(value), _) => {
                if *value {
                    "on".to_string()
                } else {
                    "off".to_string()
                }
            }
            (_, ParameterValue::Note(value), _) => value.to_string(),
            (_, value, ParameterUnit::Gain) => format_gain(value.as_f32().unwrap_or_default()),
            (_, value, ParameterUnit::Decibels) => {
                format!("{:.1} dB", value.as_f32().unwrap_or_default())
            }
            (_, value, ParameterUnit::Pan) => format_pan(value.as_f32().unwrap_or_default()),
            (_, value, ParameterUnit::Percent) => {
                format!("{:.0}%", value.as_f32().unwrap_or_default() * 100.0)
            }
            (_, value, ParameterUnit::Hertz) => format_hertz(value.as_f32().unwrap_or_default()),
            (_, value, ParameterUnit::Milliseconds) => {
                format!("{:.1} ms", value.as_f32().unwrap_or_default() * 1000.0)
            }
            (_, value, ParameterUnit::Seconds) => {
                format!("{:.3} s", value.as_f32().unwrap_or_default())
            }
            (_, value, ParameterUnit::Semitones) => {
                format!("{:+.1} st", value.as_f32().unwrap_or_default())
            }
            (_, value, ParameterUnit::Ratio) => format_ratio(value.as_f32().unwrap_or_default()),
            (_, value, _) => format!("{:.3}", value.as_f32().unwrap_or_default()),
        }
    }

    pub fn parse_value(&self, input: &str) -> Result<ParameterValue, ParameterValidationError> {
        let normalized = input.trim();
        let value = match &self.range {
            ParameterRange::Boolean => match normalized.to_ascii_lowercase().as_str() {
                "on" | "true" | "yes" | "1" => ParameterValue::Bool(true),
                "off" | "false" | "no" | "0" => ParameterValue::Bool(false),
                _ => {
                    return Err(ParameterValidationError::ParseFailed {
                        id: self.id.clone(),
                    })
                }
            },
            ParameterRange::Enum { choices } => choices
                .iter()
                .find(|choice| {
                    choice.id.eq_ignore_ascii_case(normalized)
                        || choice.label.eq_ignore_ascii_case(normalized)
                })
                .map(|choice| ParameterValue::Enum(choice.id.clone()))
                .ok_or_else(|| ParameterValidationError::ParseFailed {
                    id: self.id.clone(),
                })?,
            _ => self.value_from_f32(parse_numeric(normalized, self.unit).ok_or_else(|| {
                ParameterValidationError::ParseFailed {
                    id: self.id.clone(),
                }
            })?),
        };
        self.validate(&value)?;
        Ok(value)
    }

    fn value_matches_type(&self, value: &ParameterValue) -> bool {
        matches!(
            (self.value_type, value),
            (
                ParameterValueType::NormalizedFloat,
                ParameterValue::Normalized(_)
            ) | (ParameterValueType::PlainFloat, ParameterValue::Float(_))
                | (ParameterValueType::BipolarFloat, ParameterValue::Bipolar(_))
                | (ParameterValueType::Integer, ParameterValue::Integer(_))
                | (ParameterValueType::Boolean, ParameterValue::Bool(_))
                | (ParameterValueType::Enum, ParameterValue::Enum(_))
                | (ParameterValueType::Note, ParameterValue::Note(_))
                | (ParameterValueType::Semitones, ParameterValue::Semitones(_))
                | (ParameterValueType::Seconds, ParameterValue::Seconds(_))
                | (
                    ParameterValueType::FrequencyHertz,
                    ParameterValue::FrequencyHertz(_)
                )
                | (ParameterValueType::Decibels, ParameterValue::Decibels(_))
                | (
                    ParameterValueType::Percentage,
                    ParameterValue::Percentage(_)
                )
                | (ParameterValueType::Ratio, ParameterValue::Ratio(_))
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParameterValidationError {
    #[error("parameter {id} has the wrong value type, expected {expected:?}")]
    WrongType {
        id: ParameterId,
        expected: ParameterValueType,
    },
    #[error("parameter {id} is out of range")]
    OutOfRange { id: ParameterId },
    #[error("parameter {id} is not aligned to its step size")]
    NotOnStep { id: ParameterId },
    #[error("parameter {id} has an invalid enum choice")]
    InvalidChoice { id: ParameterId },
    #[error("parameter {id} cannot be normalized")]
    NotNormalizable { id: ParameterId },
    #[error("parameter {id} has an invalid range")]
    InvalidRange { id: ParameterId },
    #[error("parameter {id} could not be parsed")]
    ParseFailed { id: ParameterId },
}

fn is_aligned_to_step(value: f32, min: f32, step: f32) -> bool {
    let steps = (value - min) / step;
    (steps - steps.round()).abs() <= 0.001
}

fn format_gain(gain: f32) -> String {
    if gain <= 0.0 || !gain.is_finite() {
        "-inf dB".to_string()
    } else {
        format!("{:+.1} dB", 20.0 * gain.log10())
    }
}

fn format_pan(pan: f32) -> String {
    let pan = pan.clamp(-1.0, 1.0);
    if pan.abs() < 0.0005 {
        "C".to_string()
    } else if pan < 0.0 {
        format!("L{:.0}", pan.abs() * 100.0)
    } else {
        format!("R{:.0}", pan * 100.0)
    }
}

fn format_hertz(value: f32) -> String {
    if value.abs() >= 1000.0 {
        format!("{:.1} kHz", value / 1000.0)
    } else {
        format!("{:.0} Hz", value)
    }
}

fn format_ratio(value: f32) -> String {
    if value > 0.0 && value < 1.0 {
        let denominator = (1.0 / value).round();
        if ((1.0 / denominator) - value).abs() < 0.0001 {
            return format!("1/{denominator:.0}");
        }
    }
    if value >= 1.0 {
        return format!("{value:.1}:1");
    }
    format!("{value:.3}")
}

fn parse_numeric(input: &str, unit: ParameterUnit) -> Option<f32> {
    let lower = input.trim().to_ascii_lowercase();
    match unit {
        ParameterUnit::Percent => lower
            .strip_suffix('%')
            .unwrap_or(&lower)
            .trim()
            .parse::<f32>()
            .ok()
            .map(|value| value / 100.0),
        ParameterUnit::Hertz => {
            if let Some(value) = lower.strip_suffix("khz") {
                value.trim().parse::<f32>().ok().map(|value| value * 1000.0)
            } else {
                lower
                    .strip_suffix("hz")
                    .unwrap_or(&lower)
                    .trim()
                    .parse::<f32>()
                    .ok()
            }
        }
        ParameterUnit::Decibels | ParameterUnit::Gain => lower
            .strip_suffix("db")
            .unwrap_or(&lower)
            .trim()
            .parse::<f32>()
            .ok()
            .map(|value| {
                if unit == ParameterUnit::Gain {
                    10.0_f32.powf(value / 20.0)
                } else {
                    value
                }
            }),
        ParameterUnit::Milliseconds => lower
            .strip_suffix("ms")
            .unwrap_or(&lower)
            .trim()
            .parse::<f32>()
            .ok()
            .map(|value| value / 1000.0),
        ParameterUnit::Seconds => lower
            .strip_suffix('s')
            .unwrap_or(&lower)
            .trim()
            .parse::<f32>()
            .ok(),
        ParameterUnit::Semitones => lower
            .strip_suffix("st")
            .unwrap_or(&lower)
            .trim()
            .parse::<f32>()
            .ok(),
        ParameterUnit::Ratio => {
            if let Some((numerator, denominator)) = lower.split_once('/') {
                let numerator = numerator.trim().parse::<f32>().ok()?;
                let denominator = denominator.trim().parse::<f32>().ok()?;
                if denominator == 0.0 {
                    None
                } else {
                    Some(numerator / denominator)
                }
            } else if let Some((numerator, denominator)) = lower.split_once(':') {
                let numerator = numerator.trim().parse::<f32>().ok()?;
                let denominator = denominator.trim().parse::<f32>().ok()?;
                if denominator == 0.0 {
                    None
                } else {
                    Some(numerator / denominator)
                }
            } else {
                lower.parse::<f32>().ok()
            }
        }
        _ => lower.parse::<f32>().ok(),
    }
}
