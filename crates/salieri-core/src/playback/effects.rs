use crate::{PatternCell, TrackerCommand};

pub(super) fn delay_command(cell: &PatternCell) -> Option<TrackerCommand> {
    command_with_code(cell, TrackerCommand::DELAY_CODE)
}

pub(super) fn retrigger_command(cell: &PatternCell) -> Option<TrackerCommand> {
    command_with_code(cell, TrackerCommand::RETRIGGER_CODE)
}

fn command_with_code(cell: &PatternCell, code: u8) -> Option<TrackerCommand> {
    cell.commands()
        .find(|command| command.code.to_ascii_uppercase() == code)
}
