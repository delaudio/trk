use super::*;
use salieri_midi::FakeMidiInput;

mod ai_commands;
mod cli_workflows;
mod command_palette;
mod commands_playback;
mod history_transactions;
mod layout_commands;
mod navigation_editing;
mod pattern_commands;
mod pattern_operations;
mod patterns_sequence;
mod persistence;
mod preset_inventory;
mod sampler_browsers;
mod tracks_panels;
mod workspace_libraries;

fn type_command(app: &mut App, command: &str) {
    enter_command(app, command);
    assert_eq!(app.mode, AppMode::Normal);
}

fn enter_command(app: &mut App, command: &str) {
    app.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::Command);
    for value in command.chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(value), KeyModifiers::NONE));
    }
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
}

fn wav_pcm16_bytes(sample_rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
    let data_size = samples.len() * 2;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size as u32).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * u32::from(channels) * 2;
    bytes.extend_from_slice(&byte_rate.to_le_bytes());
    let block_align = channels * 2;
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(data_size as u32).to_le_bytes());
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}
