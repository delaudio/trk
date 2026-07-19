use std::collections::HashMap;

use salieri_core::{pattern_events, NoteEvent, PlaybackEventKind, Song};

use crate::diagnostics::InteropError;

#[cfg(test)]
mod tests;

const MTHD: &[u8; 4] = b"MThd";
const MTRK: &[u8; 4] = b"MTrk";
const DEFAULT_TICKS_PER_QUARTER: u16 = 480;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MidiExportOptions {
    pub pattern: usize,
    pub ticks_per_quarter: u16,
}

impl Default for MidiExportOptions {
    fn default() -> Self {
        Self {
            pattern: 0,
            ticks_per_quarter: DEFAULT_TICKS_PER_QUARTER,
        }
    }
}

pub fn export_pattern_smf(
    song: &Song,
    options: MidiExportOptions,
) -> Result<Vec<u8>, InteropError> {
    let pattern = song
        .pattern(options.pattern)
        .ok_or(InteropError::MissingPattern(options.pattern))?;
    let ticks_per_quarter = options.ticks_per_quarter.max(1);
    let micros_per_quarter = 60_000_000_u64 / u64::from(song.transport.bpm.max(1));
    let mut track = Vec::new();

    write_var_len(0, &mut track);
    track.extend_from_slice(&[0xff, 0x51, 0x03]);
    let tempo = micros_per_quarter.min(0x00ff_ffff) as u32;
    track.extend_from_slice(&tempo.to_be_bytes()[1..4]);

    let mut last_tick = 0_u64;
    for event in pattern_events(song, pattern) {
        let tick = event
            .position
            .offset_micros
            .saturating_mul(u64::from(ticks_per_quarter))
            / micros_per_quarter;
        write_var_len(tick.saturating_sub(last_tick), &mut track);
        last_tick = tick;

        let channel = event.midi_channel.saturating_sub(1).min(15);
        match event.kind {
            PlaybackEventKind::NoteOn { pitch, velocity } => {
                track.extend_from_slice(&[0x90 | channel, pitch.min(127), velocity.min(127)]);
            }
            PlaybackEventKind::NoteOff { pitch } => {
                track.extend_from_slice(&[0x80 | channel, pitch.min(127), 0]);
            }
        }
    }

    write_var_len(0, &mut track);
    track.extend_from_slice(&[0xff, 0x2f, 0x00]);

    let mut bytes = Vec::new();
    bytes.extend_from_slice(MTHD);
    bytes.extend_from_slice(&6_u32.to_be_bytes());
    bytes.extend_from_slice(&0_u16.to_be_bytes());
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&ticks_per_quarter.to_be_bytes());
    bytes.extend_from_slice(MTRK);
    bytes.extend_from_slice(&(track.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&track);
    Ok(bytes)
}

pub fn import_smf(bytes: &[u8]) -> Result<Song, InteropError> {
    let mut cursor = Cursor::new(bytes);
    if cursor.read_exact(4)? != MTHD {
        return Err(InteropError::InvalidMidiHeader);
    }
    let header_len = cursor.read_u32()? as usize;
    if header_len < 6 {
        return Err(InteropError::InvalidMidiHeader);
    }
    let format = cursor.read_u16()?;
    let track_count = cursor.read_u16()?;
    let division = cursor.read_u16()?;
    cursor.skip(header_len - 6)?;

    if format != 0 {
        return Err(InteropError::UnsupportedMidiFormat(format));
    }
    if track_count != 1 {
        return Err(InteropError::UnsupportedMidiFormat(format));
    }
    if division & 0x8000 != 0 {
        return Err(InteropError::UnsupportedSmpteDivision(division));
    }

    if cursor.read_exact(4)? != MTRK {
        return Err(InteropError::InvalidMidiTrack);
    }
    let track_len = cursor.read_u32()? as usize;
    let track_end = cursor.position().saturating_add(track_len);
    if track_end > bytes.len() {
        return Err(InteropError::TruncatedMidiData);
    }

    let mut song = Song::empty();
    let ticks_per_quarter = u64::from(division.max(1));
    let mut absolute_tick = 0_u64;
    let mut running_status = None;
    let mut channel_tracks = HashMap::new();

    while cursor.position() < track_end {
        absolute_tick = absolute_tick.saturating_add(read_var_len(&mut cursor)?);
        let mut status = cursor.read_u8()?;
        if status < 0x80 {
            status = running_status.ok_or(InteropError::UnsupportedMidiEvent(status))?;
            cursor.rewind(1);
        } else if status < 0xf0 {
            running_status = Some(status);
        }

        match status {
            0x80..=0x9f => {
                let pitch = cursor.read_u8()?.min(127);
                let velocity = cursor.read_u8()?.min(127);
                if status & 0xf0 == 0x90 && velocity > 0 {
                    let channel = (status & 0x0f) + 1;
                    let track = track_for_channel(&mut song, &mut channel_tracks, channel);
                    let row = ticks_to_row(
                        absolute_tick,
                        ticks_per_quarter,
                        song.transport.lines_per_beat,
                    );
                    ensure_pattern_row(&mut song, row)?;
                    song.current_pattern_mut()
                        .expect("default song has a pattern")
                        .set_note(row, track, NoteEvent::Note { pitch }, velocity)
                        .expect("row and track were ensured");
                }
            }
            0xff => {
                let meta_type = cursor.read_u8()?;
                let len = read_var_len(&mut cursor)? as usize;
                if meta_type == 0x51 && len == 3 {
                    let tempo = read_tempo(&mut cursor)?;
                    song.transport.bpm = tempo_to_bpm(tempo);
                } else {
                    cursor.skip(len)?;
                }
                if meta_type == 0x2f {
                    break;
                }
            }
            0xf0 | 0xf7 => {
                return Err(InteropError::UnsupportedMidiEvent(status));
            }
            _ => return Err(InteropError::UnsupportedMidiEvent(status)),
        }
    }

    Ok(song)
}

fn track_for_channel(
    song: &mut Song,
    channel_tracks: &mut HashMap<u8, usize>,
    channel: u8,
) -> usize {
    if let Some(track) = channel_tracks.get(&channel) {
        return *track;
    }
    let track = song
        .tracks
        .iter()
        .position(|track| track.midi_channel == channel)
        .unwrap_or_else(|| {
            song.create_track();
            let index = song.tracks.len() - 1;
            song.tracks[index].midi_channel = channel;
            index
        });
    channel_tracks.insert(channel, track);
    track
}

fn ensure_pattern_row(song: &mut Song, row: usize) -> Result<(), InteropError> {
    let row_count = song
        .current_pattern()
        .expect("default song has a pattern")
        .row_count();
    if row >= row_count {
        song.resize_pattern(0, row + 1)
            .map_err(|_| InteropError::InvalidMidiTrack)?;
    }
    Ok(())
}

fn ticks_to_row(tick: u64, ticks_per_quarter: u64, lines_per_beat: u8) -> usize {
    let lines_per_beat = u64::from(lines_per_beat.max(1));
    ((tick.saturating_mul(lines_per_beat) + ticks_per_quarter / 2) / ticks_per_quarter) as usize
}

fn tempo_to_bpm(tempo_micros_per_quarter: u32) -> u16 {
    if tempo_micros_per_quarter == 0 {
        return 120;
    }
    (60_000_000 / tempo_micros_per_quarter).clamp(1, u32::from(u16::MAX)) as u16
}

fn write_var_len(mut value: u64, bytes: &mut Vec<u8>) {
    let mut buffer = [0_u8; 10];
    let mut index = buffer.len() - 1;
    buffer[index] = (value & 0x7f) as u8;
    value >>= 7;
    while value > 0 {
        index -= 1;
        buffer[index] = ((value & 0x7f) as u8) | 0x80;
        value >>= 7;
    }
    bytes.extend_from_slice(&buffer[index..]);
}

fn read_var_len(cursor: &mut Cursor<'_>) -> Result<u64, InteropError> {
    let mut value = 0_u64;
    for _ in 0..4 {
        let byte = cursor.read_u8()?;
        value = (value << 7) | u64::from(byte & 0x7f);
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(InteropError::InvalidMidiTrack)
}

fn read_tempo(cursor: &mut Cursor<'_>) -> Result<u32, InteropError> {
    let bytes = cursor.read_exact(3)?;
    Ok((u32::from(bytes[0]) << 16) | (u32::from(bytes[1]) << 8) | u32::from(bytes[2]))
}

struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    const fn position(&self) -> usize {
        self.position
    }

    fn rewind(&mut self, count: usize) {
        self.position = self.position.saturating_sub(count);
    }

    fn skip(&mut self, count: usize) -> Result<(), InteropError> {
        self.read_exact(count).map(|_| ())
    }

    fn read_u8(&mut self) -> Result<u8, InteropError> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_u16(&mut self) -> Result<u16, InteropError> {
        let bytes = self.read_exact(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, InteropError> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_exact(&mut self, count: usize) -> Result<&'a [u8], InteropError> {
        let end = self
            .position
            .checked_add(count)
            .ok_or(InteropError::TruncatedMidiData)?;
        if end > self.bytes.len() {
            return Err(InteropError::TruncatedMidiData);
        }
        let bytes = &self.bytes[self.position..end];
        self.position = end;
        Ok(bytes)
    }
}
