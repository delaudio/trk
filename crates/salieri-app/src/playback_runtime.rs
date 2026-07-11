use std::{
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use salieri_core::{pattern_events, row_duration_micros, PlaybackPosition, Song};
use salieri_midi::{send_all_notes_off, send_playback_event, FakeMidiOutput};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackUpdate {
    Position(PlaybackPosition),
    Stopped,
}

#[derive(Debug)]
enum PlaybackCommand {
    StartPattern { song: Song, pattern_index: usize },
    Stop,
    Shutdown,
}

pub struct PlaybackRuntime {
    command_tx: Sender<PlaybackCommand>,
    update_rx: Receiver<PlaybackUpdate>,
    handle: Option<JoinHandle<()>>,
}

impl PlaybackRuntime {
    pub fn spawn() -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let (update_tx, update_rx) = mpsc::channel();
        let handle = thread::spawn(move || playback_thread(command_rx, update_tx));

        Self {
            command_tx,
            update_rx,
            handle: Some(handle),
        }
    }

    pub fn start_pattern(&self, song: Song, pattern_index: usize) {
        let _ = self.command_tx.send(PlaybackCommand::StartPattern {
            song,
            pattern_index,
        });
    }

    pub fn stop(&self) {
        let _ = self.command_tx.send(PlaybackCommand::Stop);
    }

    pub fn try_recv(&self) -> Option<PlaybackUpdate> {
        self.update_rx.try_recv().ok()
    }
}

impl std::fmt::Debug for PlaybackRuntime {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlaybackRuntime")
            .finish_non_exhaustive()
    }
}

impl Drop for PlaybackRuntime {
    fn drop(&mut self) {
        let _ = self.command_tx.send(PlaybackCommand::Shutdown);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn playback_thread(command_rx: Receiver<PlaybackCommand>, update_tx: Sender<PlaybackUpdate>) {
    let mut output = FakeMidiOutput::new();
    let mut next_command = None;

    loop {
        let command = match next_command.take() {
            Some(command) => command,
            None => match command_rx.recv() {
                Ok(command) => command,
                Err(_) => break,
            },
        };
        match command {
            PlaybackCommand::StartPattern {
                song,
                pattern_index,
            } => {
                next_command =
                    run_pattern(song, pattern_index, &command_rx, &update_tx, &mut output);
                if matches!(next_command, Some(PlaybackCommand::Shutdown)) {
                    break;
                }
            }
            PlaybackCommand::Stop => {
                let _ = send_all_notes_off(&mut output);
                let _ = update_tx.send(PlaybackUpdate::Stopped);
            }
            PlaybackCommand::Shutdown => break,
        }
    }

    let _ = send_all_notes_off(&mut output);
    let _ = update_tx.send(PlaybackUpdate::Stopped);
}

fn run_pattern(
    song: Song,
    pattern_index: usize,
    command_rx: &Receiver<PlaybackCommand>,
    update_tx: &Sender<PlaybackUpdate>,
    output: &mut FakeMidiOutput,
) -> Option<PlaybackCommand> {
    let Some(pattern) = song.pattern(pattern_index) else {
        let _ = update_tx.send(PlaybackUpdate::Stopped);
        return None;
    };
    let pattern = pattern.clone();
    let row_count = pattern.row_count();
    let row_duration = Duration::from_micros(row_duration_micros(&song.transport).max(1));
    let events = pattern_events(&song, &pattern);
    loop {
        let loop_start = Instant::now();

        for row in 0..=row_count {
            let deadline = loop_start + row_duration.saturating_mul(row as u32);
            if let Some(command) = wait_until(command_rx, deadline) {
                let _ = send_all_notes_off(output);
                let _ = update_tx.send(PlaybackUpdate::Stopped);
                return Some(command);
            }

            for event in events.iter().filter(|event| event.position.row == row) {
                let _ = send_playback_event(output, *event);
            }

            if row < row_count {
                let _ = update_tx.send(PlaybackUpdate::Position(PlaybackPosition {
                    row,
                    offset_micros: row_duration.as_micros().saturating_mul(row as u128) as u64,
                }));
            }
        }
    }
}

fn wait_until(
    command_rx: &Receiver<PlaybackCommand>,
    deadline: Instant,
) -> Option<PlaybackCommand> {
    match command_rx.try_recv() {
        Ok(command) => return Some(command),
        Err(TryRecvError::Empty) => {}
        Err(TryRecvError::Disconnected) => return Some(PlaybackCommand::Shutdown),
    }

    let now = Instant::now();
    if now >= deadline {
        return None;
    }

    match command_rx.recv_timeout(deadline.saturating_duration_since(now)) {
        Ok(command) => Some(command),
        Err(RecvTimeoutError::Timeout) => None,
        Err(RecvTimeoutError::Disconnected) => Some(PlaybackCommand::Shutdown),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use salieri_core::NoteEvent;

    #[test]
    fn runtime_emits_positions_and_stops() {
        let runtime = PlaybackRuntime::spawn();
        let mut song = Song::empty();
        song.transport.bpm = u16::MAX;
        song.transport.lines_per_beat = u8::MAX;
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
            .expect("set note");

        runtime.start_pattern(song, 0);

        let deadline = Instant::now() + Duration::from_millis(250);
        let mut saw_position = false;
        while Instant::now() < deadline {
            if matches!(runtime.try_recv(), Some(PlaybackUpdate::Position(_))) {
                saw_position = true;
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }

        runtime.stop();

        let deadline = Instant::now() + Duration::from_millis(250);
        let mut saw_stop = false;
        while Instant::now() < deadline {
            while let Some(update) = runtime.try_recv() {
                if matches!(update, PlaybackUpdate::Stopped) {
                    saw_stop = true;
                    break;
                }
            }
            if saw_stop {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }

        assert!(saw_position);
        assert!(saw_stop);
    }
}
