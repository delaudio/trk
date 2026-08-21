//! Stable performance-page and encoder identities shared by the application and TUI.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParameterPage {
    Source,
    Filter,
    Amp,
    Effects,
    Lfo,
    Algorithm,
}

impl ParameterPage {
    pub const ALL: [Self; 6] = [
        Self::Source,
        Self::Filter,
        Self::Amp,
        Self::Effects,
        Self::Lfo,
        Self::Algorithm,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Source => "SRC",
            Self::Filter => "FLTR",
            Self::Amp => "AMP",
            Self::Effects => "FX",
            Self::Lfo => "LFO",
            Self::Algorithm => "ALG",
        }
    }

    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Source => 0,
            Self::Filter => 1,
            Self::Amp => 2,
            Self::Effects => 3,
            Self::Lfo => 4,
            Self::Algorithm => 5,
        }
    }

    #[must_use]
    pub const fn from_function_key(number: u8) -> Option<Self> {
        match number {
            1 => Some(Self::Source),
            2 => Some(Self::Filter),
            3 => Some(Self::Amp),
            4 => Some(Self::Effects),
            5 => Some(Self::Lfo),
            6 => Some(Self::Algorithm),
            _ => None,
        }
    }
}

pub const PARAMETER_ENCODER_KEYS: [char; 8] = ['Q', 'W', 'E', 'R', 'A', 'S', 'D', 'F'];

#[must_use]
pub fn parameter_encoder_index(key: char) -> Option<usize> {
    let key = key.to_ascii_uppercase();
    PARAMETER_ENCODER_KEYS
        .iter()
        .position(|candidate| *candidate == key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_and_encoder_keys_have_stable_hardware_order() {
        assert_eq!(
            ParameterPage::ALL.map(ParameterPage::label),
            ["SRC", "FLTR", "AMP", "FX", "LFO", "ALG"]
        );
        assert_eq!(
            PARAMETER_ENCODER_KEYS,
            ['Q', 'W', 'E', 'R', 'A', 'S', 'D', 'F']
        );
        assert_eq!(parameter_encoder_index('d'), Some(6));
        assert_eq!(
            ParameterPage::from_function_key(6),
            Some(ParameterPage::Algorithm)
        );
        assert_eq!(ParameterPage::from_function_key(7), None);
    }
}
