use std::{fs, path::Path};

use salieri_core::TrackId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SampleId(pub u32);

#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: usize,
    pub data: Vec<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreviewSettings {
    pub pitch_semitones: f32,
    pub volume: f32,
    pub max_frames: Option<usize>,
}

impl Default for PreviewSettings {
    fn default() -> Self {
        Self {
            pitch_semitones: 0.0,
            volume: 1.0,
            max_frames: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreviewBuffer {
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: usize,
    pub data: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SampleAssignment {
    pub track: TrackId,
    pub sample: SampleId,
    pub root_pitch: u8,
    pub volume: f32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SamplerProgram {
    assignments: Vec<SampleAssignment>,
}

#[derive(Debug, thiserror::Error)]
pub enum SamplerError {
    #[error("failed to read sample: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid WAV file: {0}")]
    InvalidWav(&'static str),
    #[error("unsupported WAV format: {0}")]
    UnsupportedWav(&'static str),
}

impl Sample {
    pub fn load_wav(path: impl AsRef<Path>) -> Result<Self, SamplerError> {
        let path = path.as_ref();
        let bytes = fs::read(path)?;
        let wav = parse_wav(&bytes)?;
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .map_or_else(|| "sample".to_string(), ToString::to_string);

        Ok(Self {
            name,
            sample_rate: wav.sample_rate,
            channels: wav.channels,
            frames: wav.data.len() / usize::from(wav.channels),
            data: wav.data,
        })
    }

    #[must_use]
    pub fn preview(&self, settings: PreviewSettings) -> PreviewBuffer {
        let channels = usize::from(self.channels);
        let pitch_step = 2.0_f32.powf(settings.pitch_semitones / 12.0).max(0.01);
        let volume = settings.volume.max(0.0);
        let source_frames = self.frames;
        let natural_frames = ((source_frames as f32) / pitch_step).ceil() as usize;
        let output_frames = settings
            .max_frames
            .map_or(natural_frames, |max_frames| max_frames.min(natural_frames));
        let mut data = Vec::with_capacity(output_frames.saturating_mul(channels));

        for frame in 0..output_frames {
            let source_frame = ((frame as f32) * pitch_step).floor() as usize;
            let source_frame = source_frame.min(source_frames.saturating_sub(1));
            let source_offset = source_frame.saturating_mul(channels);
            for channel in 0..channels {
                data.push(self.data[source_offset + channel] * volume);
            }
        }

        PreviewBuffer {
            sample_rate: self.sample_rate,
            channels: self.channels,
            frames: output_frames,
            data,
        }
    }
}

impl SamplerProgram {
    #[must_use]
    pub fn assignments(&self) -> &[SampleAssignment] {
        &self.assignments
    }

    pub fn assign_to_track(&mut self, assignment: SampleAssignment) {
        if let Some(existing) = self
            .assignments
            .iter_mut()
            .find(|existing| existing.track == assignment.track)
        {
            *existing = assignment;
        } else {
            self.assignments.push(assignment);
        }
    }

    #[must_use]
    pub fn assignment_for_track(&self, track: TrackId) -> Option<&SampleAssignment> {
        self.assignments
            .iter()
            .find(|assignment| assignment.track == track)
    }

    pub fn clear_track(&mut self, track: TrackId) {
        self.assignments
            .retain(|assignment| assignment.track != track);
    }
}

#[derive(Debug)]
struct ParsedWav {
    sample_rate: u32,
    channels: u16,
    data: Vec<f32>,
}

#[derive(Debug, Clone, Copy)]
struct WavFormat {
    audio_format: u16,
    channels: u16,
    sample_rate: u32,
    bits_per_sample: u16,
}

fn parse_wav(bytes: &[u8]) -> Result<ParsedWav, SamplerError> {
    if bytes.len() < 12 {
        return Err(SamplerError::InvalidWav("file is too short"));
    }
    if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(SamplerError::InvalidWav("missing RIFF/WAVE header"));
    }

    let mut offset = 12;
    let mut format = None;
    let mut data_chunk = None;
    while offset + 8 <= bytes.len() {
        let chunk_id = &bytes[offset..offset + 4];
        let chunk_size = read_u32_le(bytes, offset + 4)? as usize;
        let chunk_start = offset + 8;
        let chunk_end = chunk_start
            .checked_add(chunk_size)
            .ok_or(SamplerError::InvalidWav("chunk size overflow"))?;
        if chunk_end > bytes.len() {
            return Err(SamplerError::InvalidWav("chunk extends past end of file"));
        }

        match chunk_id {
            b"fmt " => format = Some(parse_format_chunk(&bytes[chunk_start..chunk_end])?),
            b"data" => data_chunk = Some(&bytes[chunk_start..chunk_end]),
            _ => {}
        }

        offset = chunk_end + (chunk_size % 2);
    }

    let format = format.ok_or(SamplerError::InvalidWav("missing fmt chunk"))?;
    let data_chunk = data_chunk.ok_or(SamplerError::InvalidWav("missing data chunk"))?;
    if format.channels == 0 {
        return Err(SamplerError::InvalidWav("channel count is zero"));
    }

    Ok(ParsedWav {
        sample_rate: format.sample_rate,
        channels: format.channels,
        data: decode_samples(data_chunk, format)?,
    })
}

fn parse_format_chunk(bytes: &[u8]) -> Result<WavFormat, SamplerError> {
    if bytes.len() < 16 {
        return Err(SamplerError::InvalidWav("fmt chunk is too short"));
    }

    Ok(WavFormat {
        audio_format: read_u16_le(bytes, 0)?,
        channels: read_u16_le(bytes, 2)?,
        sample_rate: read_u32_le(bytes, 4)?,
        bits_per_sample: read_u16_le(bytes, 14)?,
    })
}

fn decode_samples(bytes: &[u8], format: WavFormat) -> Result<Vec<f32>, SamplerError> {
    match (format.audio_format, format.bits_per_sample) {
        (1, 16) => {
            if !bytes.len().is_multiple_of(2) {
                return Err(SamplerError::InvalidWav("16-bit PCM data is truncated"));
            }
            Ok(bytes
                .chunks_exact(2)
                .map(|sample| i16::from_le_bytes([sample[0], sample[1]]) as f32 / 32768.0)
                .collect())
        }
        (3, 32) => {
            if !bytes.len().is_multiple_of(4) {
                return Err(SamplerError::InvalidWav("32-bit float data is truncated"));
            }
            Ok(bytes
                .chunks_exact(4)
                .map(|sample| f32::from_le_bytes([sample[0], sample[1], sample[2], sample[3]]))
                .collect())
        }
        (1, _) => Err(SamplerError::UnsupportedWav(
            "only 16-bit PCM WAV is currently supported",
        )),
        (3, _) => Err(SamplerError::UnsupportedWav(
            "only 32-bit float WAV is currently supported",
        )),
        _ => Err(SamplerError::UnsupportedWav(
            "only PCM and IEEE float WAV files are currently supported",
        )),
    }
}

fn read_u16_le(bytes: &[u8], offset: usize) -> Result<u16, SamplerError> {
    let value = bytes
        .get(offset..offset + 2)
        .ok_or(SamplerError::InvalidWav("unexpected end of file"))?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32, SamplerError> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or(SamplerError::InvalidWav("unexpected end of file"))?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_16_bit_pcm_wav_samples() {
        let path =
            std::env::temp_dir().join(format!("salieri-sampler-load-{}.wav", std::process::id()));
        fs::write(
            &path,
            wav_pcm16_bytes(44_100, 2, &[0, i16::MAX, i16::MIN, 16_384]),
        )
        .expect("write wav");

        let sample = Sample::load_wav(&path).expect("load wav");
        let _ = fs::remove_file(&path);

        assert_eq!(sample.sample_rate, 44_100);
        assert_eq!(sample.channels, 2);
        assert_eq!(sample.frames, 2);
        assert_eq!(sample.data[0], 0.0);
        assert!(sample.data[1] > 0.999);
        assert_eq!(sample.data[2], -1.0);
        assert_eq!(sample.data[3], 0.5);
    }

    #[test]
    fn preview_applies_volume_and_pitch() {
        let sample = Sample {
            name: "tone".to_string(),
            sample_rate: 48_000,
            channels: 1,
            frames: 4,
            data: vec![0.25, 0.5, 0.75, 1.0],
        };

        let preview = sample.preview(PreviewSettings {
            pitch_semitones: 12.0,
            volume: 0.5,
            max_frames: None,
        });

        assert_eq!(preview.sample_rate, 48_000);
        assert_eq!(preview.channels, 1);
        assert_eq!(preview.frames, 2);
        assert_eq!(preview.data, vec![0.125, 0.375]);
    }

    #[test]
    fn sampler_program_assigns_replaces_and_clears_track_samples() {
        let mut program = SamplerProgram::default();

        program.assign_to_track(SampleAssignment {
            track: TrackId(1),
            sample: SampleId(10),
            root_pitch: 60,
            volume: 1.0,
        });
        program.assign_to_track(SampleAssignment {
            track: TrackId(1),
            sample: SampleId(11),
            root_pitch: 48,
            volume: 0.8,
        });

        assert_eq!(program.assignments().len(), 1);
        assert_eq!(
            program.assignment_for_track(TrackId(1)),
            Some(&SampleAssignment {
                track: TrackId(1),
                sample: SampleId(11),
                root_pitch: 48,
                volume: 0.8,
            })
        );

        program.clear_track(TrackId(1));
        assert!(program.assignments().is_empty());
    }

    #[test]
    fn rejects_non_wav_files() {
        let error = parse_wav(b"not a wave").expect_err("invalid wav");

        assert!(matches!(error, SamplerError::InvalidWav(_)));
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
}
