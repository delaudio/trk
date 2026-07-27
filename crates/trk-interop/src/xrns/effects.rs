use trk_core::{tracker_command_spec, TrackerCommand, TrackerCommandSupport};

const DEFAULT_RENOISE_TICKS_PER_LINE: u8 = 12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct XrnsEffectCommand {
    pub original_code: String,
    pub command: TrackerCommand,
}

pub(super) fn normalize_xrns_effect_code(value: &str) -> Option<String> {
    let value = value.trim().to_ascii_uppercase();
    if value.is_empty() || matches!(value.as_str(), ".." | "---") {
        return None;
    }
    Some(value)
}

pub(super) fn translate_xrns_effect_command(
    code: &str,
    value: u8,
    ticks_per_line: Option<u8>,
) -> XrnsEffectCommand {
    let original_code = code.to_string();
    let command = match code {
        "D" => TrackerCommand::delay(value),
        "R" => TrackerCommand::retrigger(value),
        "0Q" | "Q" => {
            TrackerCommand::delay(renoise_tick_delay_to_row_fraction(value, ticks_per_line))
        }
        "0R" => TrackerCommand::retrigger(renoise_retrigger_count(value, ticks_per_line)),
        "0U" => TrackerCommand::from_code_char('U', value),
        "0D" => TrackerCommand::from_code_char('N', value),
        "0G" => TrackerCommand::from_code_char('G', value),
        "0S" => TrackerCommand::from_code_char('O', value),
        "0C" => TrackerCommand::from_code_char('C', value),
        "0A" => TrackerCommand::from_code_char('A', value),
        "0M" | "0L" => TrackerCommand::from_code_char('V', value),
        "0P" => TrackerCommand::from_code_char('P', value),
        "ZT" | "ZL" | "ZB" | "ZD" => TrackerCommand::from_code_char('T', value),
        _ if code.len() == 1 => TrackerCommand::from_code_char(
            code.chars()
                .next()
                .expect("single-character XRNS effect code"),
            value,
        ),
        _ => TrackerCommand::from_code_char('?', value),
    };
    XrnsEffectCommand {
        original_code,
        command,
    }
}

pub(super) fn effect_command_needs_warning(command: TrackerCommand) -> bool {
    let Some(spec) = tracker_command_spec(command.code) else {
        return true;
    };
    spec.support != TrackerCommandSupport::Supported
        || !(spec.min..=spec.max).contains(&command.value)
}

pub(super) fn effect_command_warning_message(translated: &XrnsEffectCommand, value: u8) -> String {
    format!(
        "Renoise effect command {}{:02X} preserved as tracker command {}{:02X} without playback semantics",
        translated.original_code,
        value,
        translated.command.display_code(),
        translated.command.value
    )
}

fn renoise_tick_delay_to_row_fraction(value: u8, ticks_per_line: Option<u8>) -> u8 {
    let ticks_per_line = effective_ticks_per_line(ticks_per_line);
    let clamped = value.min(ticks_per_line);
    ((u16::from(clamped) * 0xff) / u16::from(ticks_per_line)) as u8
}

fn renoise_retrigger_count(value: u8, ticks_per_line: Option<u8>) -> u8 {
    if value == 0 {
        return 0;
    }
    let ticks_per_line = effective_ticks_per_line(ticks_per_line);
    let repeats = u16::from(ticks_per_line).div_ceil(u16::from(value));
    u8::try_from((repeats + 1).clamp(1, 16)).expect("clamped retrigger count fits in u8")
}

fn effective_ticks_per_line(ticks_per_line: Option<u8>) -> u8 {
    ticks_per_line
        .unwrap_or(DEFAULT_RENOISE_TICKS_PER_LINE)
        .max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_renoise_delay_and_retrigger_to_supported_trk_commands() {
        assert_eq!(
            translate_xrns_effect_command("0Q", 0x06, Some(12)).command,
            TrackerCommand::delay(0x7f)
        );
        assert_eq!(
            translate_xrns_effect_command("0R", 0x04, Some(12)).command,
            TrackerCommand::retrigger(4)
        );
    }

    #[test]
    fn maps_deferred_high_priority_renoise_effect_families() {
        assert_eq!(
            translate_xrns_effect_command("0D", 0x20, None).command,
            TrackerCommand::from_code_char('N', 0x20)
        );
        assert_eq!(
            translate_xrns_effect_command("0S", 0x40, None).command,
            TrackerCommand::from_code_char('O', 0x40)
        );
        assert!(effect_command_needs_warning(
            translate_xrns_effect_command("ZB", 0x01, None).command
        ));
    }
}
