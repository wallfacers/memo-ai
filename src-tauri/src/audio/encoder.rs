use hound::{WavSpec, WavWriter, SampleFormat};
use std::path::Path;
use crate::error::AppResult;

/// Write PCM i16 samples to a WAV file.
pub fn write_wav(path: &Path, samples: &[i16], sample_rate: u32, channels: u16) -> AppResult<()> {
    let spec = WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec)
        .map_err(|e| crate::error::AppError::Audio(e.to_string()))?;
    for &s in samples {
        writer.write_sample(s)
            .map_err(|e| crate::error::AppError::Audio(e.to_string()))?;
    }
    writer.finalize()
        .map_err(|e| crate::error::AppError::Audio(e.to_string()))?;
    Ok(())
}
