use std::io::{BufWriter, Write};
use std::path::Path;
use crate::error::{AppError, AppResult};

/// Write PCM i16 samples to a WAV file.
/// Uses bulk byte writes instead of per-sample loops to avoid blocking the UI thread.
pub fn write_wav(path: &Path, samples: &[i16], sample_rate: u32, channels: u16) -> AppResult<()> {
    let err = |e: std::io::Error| AppError::Audio(e.to_string());

    let file = std::fs::File::create(path).map_err(err)?;
    let mut w = BufWriter::with_capacity(512 * 1024, file);

    let data_len = (samples.len() * 2) as u32;
    let byte_rate = sample_rate * channels as u32 * 2;
    let block_align = channels * 2;

    // RIFF header
    w.write_all(b"RIFF").map_err(err)?;
    w.write_all(&(36 + data_len).to_le_bytes()).map_err(err)?;
    w.write_all(b"WAVE").map_err(err)?;

    // fmt chunk
    w.write_all(b"fmt ").map_err(err)?;
    w.write_all(&16u32.to_le_bytes()).map_err(err)?;  // chunk size
    w.write_all(&1u16.to_le_bytes()).map_err(err)?;   // PCM
    w.write_all(&channels.to_le_bytes()).map_err(err)?;
    w.write_all(&sample_rate.to_le_bytes()).map_err(err)?;
    w.write_all(&byte_rate.to_le_bytes()).map_err(err)?;
    w.write_all(&block_align.to_le_bytes()).map_err(err)?;
    w.write_all(&16u16.to_le_bytes()).map_err(err)?;  // bits per sample

    // data chunk — convert all samples to LE bytes and write in one call
    w.write_all(b"data").map_err(err)?;
    w.write_all(&data_len.to_le_bytes()).map_err(err)?;
    let raw: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    w.write_all(&raw).map_err(err)?;

    w.flush().map_err(err)?;
    Ok(())
}
