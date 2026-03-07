use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use crate::error::{AppError, AppResult};

pub struct AudioCapture {
    samples: Arc<Mutex<Vec<i16>>>,
    stream: Option<cpal::Stream>,
    sample_rate: u32,
    channels: u16,
    chunk_tx: Option<std::sync::mpsc::SyncSender<Vec<i16>>>,
}

// SAFETY: cpal::Stream is !Send on Windows due to WASAPI COM apartment threading.
// We only drop the stream from the main thread (via Tauri command), never call
// methods on it from other threads, so this is safe in practice.
unsafe impl Send for AudioCapture {}

impl AudioCapture {
    pub fn new() -> AppResult<Self> {
        Ok(AudioCapture {
            samples: Arc::new(Mutex::new(Vec::new())),
            stream: None,
            sample_rate: 16000,
            channels: 1,
            chunk_tx: None,
        })
    }

    /// 注册音频块转发 sender，录音时每批 PCM 样本会同时发送给此 channel
    pub fn set_chunk_sender(&mut self, tx: std::sync::mpsc::SyncSender<Vec<i16>>) {
        self.chunk_tx = Some(tx);
    }

    pub fn start(&mut self) -> AppResult<()> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| AppError::Audio("No input device found".into()))?;

        // Try to get a config close to 16kHz mono for Whisper
        let config = device
            .default_input_config()
            .map_err(|e| AppError::Audio(e.to_string()))?;

        self.sample_rate = config.sample_rate().0;
        self.channels = config.channels();

        let samples = Arc::clone(&self.samples);
        let err_fn = |err| eprintln!("Audio stream error: {}", err);
        let chunk_tx = self.chunk_tx.clone();

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                let samples = Arc::clone(&samples);
                let chunk_tx = chunk_tx.clone();
                device.build_input_stream(
                    &config.into(),
                    move |data: &[f32], _| {
                        let converted: Vec<i16> =
                            data.iter().map(|&s| (s * i16::MAX as f32) as i16).collect();
                        let mut buf = samples.lock().unwrap();
                        buf.extend_from_slice(&converted);
                        // 转发给 FunASR 实时转写（如已注册）
                        if let Some(ref tx) = chunk_tx {
                            let _ = tx.try_send(converted);
                        }
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                let samples = Arc::clone(&samples);
                let chunk_tx = chunk_tx.clone();
                device.build_input_stream(
                    &config.into(),
                    move |data: &[i16], _| {
                        let mut buf = samples.lock().unwrap();
                        buf.extend_from_slice(data);
                        // 转发给 FunASR 实时转写（如已注册）
                        if let Some(ref tx) = chunk_tx {
                            let _ = tx.try_send(data.to_vec());
                        }
                    },
                    err_fn,
                    None,
                )
            }
            _ => {
                return Err(AppError::Audio("Unsupported sample format".into()));
            }
        }
        .map_err(|e| AppError::Audio(e.to_string()))?;

        stream.play().map_err(|e| AppError::Audio(e.to_string()))?;
        self.stream = Some(stream);
        Ok(())
    }

    /// Stop the stream and move out the sample buffer (zero-copy).
    /// Used by async stop_recording to offload WAV writing to a blocking thread.
    pub fn take_samples_and_stop(&mut self) -> (Vec<i16>, u32, u16) {
        // Drop the stream first to stop the audio callback immediately
        self.stream.take();
        // Move samples out without cloning; leaves an empty Vec in place
        let samples = std::mem::take(&mut *self.samples.lock().unwrap());
        (samples, self.sample_rate, self.channels)
    }
}
