use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::mpsc::Sender,
    time::Instant,
};

use salieri_midi::{playback_event_to_midi, MidiError, MidiMessage, MidiOutput};

use super::{midi_dispatch::PlaybackOutput, transport::PlaybackUpdate};

pub(super) struct MidiLogger {
    start: Instant,
    file: Option<File>,
}

impl MidiLogger {
    pub(super) fn new(path: Option<PathBuf>, update_tx: &Sender<PlaybackUpdate>) -> Self {
        let file = path.and_then(|path| match open_log_file(&path) {
            Ok(mut file) => {
                let _ = writeln!(file, "# Salieri MIDI log");
                Some(file)
            }
            Err(error) => {
                let _ = update_tx.send(PlaybackUpdate::MidiLogError(format!(
                    "{}: {error}",
                    path.display()
                )));
                None
            }
        });

        Self {
            start: Instant::now(),
            file,
        }
    }

    pub(super) fn log_line(&mut self, line: impl AsRef<str>) {
        let Some(file) = &mut self.file else {
            return;
        };
        let elapsed = self.start.elapsed().as_millis();
        let _ = writeln!(file, "{elapsed:>8}ms {}", line.as_ref());
        let _ = file.flush();
    }
}

fn open_log_file(path: &PathBuf) -> std::io::Result<File> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    OpenOptions::new().create(true).append(true).open(path)
}

pub(super) fn send_playback_event_logged(
    output: &mut PlaybackOutput,
    midi_logger: &mut MidiLogger,
    event: salieri_core::PlaybackEvent,
) -> Result<(), MidiError> {
    let message = playback_event_to_midi(event);
    send_midi_message_logged(output, midi_logger, message)
}

pub(super) fn send_all_notes_off_logged(
    output: &mut PlaybackOutput,
    midi_logger: &mut MidiLogger,
) -> Result<(), MidiError> {
    for channel in 1..=16 {
        send_midi_message_logged(output, midi_logger, MidiMessage::all_notes_off(channel))?;
    }
    Ok(())
}

fn send_midi_message_logged(
    output: &mut PlaybackOutput,
    midi_logger: &mut MidiLogger,
    message: MidiMessage,
) -> Result<(), MidiError> {
    midi_logger.log_line(format_midi_message(message));
    output.send(message)
}

fn format_midi_message(message: MidiMessage) -> String {
    let bytes = message.to_bytes();
    match message {
        MidiMessage::NoteOn {
            channel,
            note,
            velocity,
        } => format!(
            "NOTE_ON ch={channel} note={note} velocity={velocity} bytes={:02X} {:02X} {:02X}",
            bytes[0], bytes[1], bytes[2]
        ),
        MidiMessage::NoteOff {
            channel,
            note,
            velocity,
        } => format!(
            "NOTE_OFF ch={channel} note={note} velocity={velocity} bytes={:02X} {:02X} {:02X}",
            bytes[0], bytes[1], bytes[2]
        ),
        MidiMessage::ControlChange {
            channel,
            controller,
            value,
        } => format!(
            "CC ch={channel} controller={controller} value={value} bytes={:02X} {:02X} {:02X}",
            bytes[0], bytes[1], bytes[2]
        ),
    }
}
