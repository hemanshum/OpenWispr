use std::sync::{Arc, Mutex};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use tauri::{AppHandle, Emitter};

pub struct SendStream(pub cpal::Stream);
unsafe impl Send for SendStream {}
unsafe impl Sync for SendStream {}

pub struct AudioRecorder {
    stream: Option<SendStream>,
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            stream: None,
            samples: Arc::new(Mutex::new(Vec::new())),
            sample_rate: 0,
            channels: 0,
        }
    }

    pub fn start_recording(&mut self, app_handle: AppHandle, device_name: Option<&str>) -> Result<(), String> {
        if self.stream.is_some() {
            return Err("An audio stream is already active".to_string());
        }

        let host = cpal::default_host();
        let device = if let Some(name) = device_name {
            if name == "Default" || name.is_empty() {
                host.default_input_device()
                    .ok_or_else(|| "No default input device found".to_string())?
            } else {
                let devices = host.input_devices()
                    .map_err(|e| format!("Failed to list input devices: {}", e))?;
                let mut found_device = None;
                for d in devices {
                    if let Ok(d_name) = d.name() {
                        if d_name == name {
                            found_device = Some(d);
                            break;
                        }
                    }
                }
                found_device.ok_or_else(|| format!("Input device '{}' not found", name))?
            }
        } else {
            host.default_input_device()
                .ok_or_else(|| "No default input device found".to_string())?
        };

        let config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get default input config: {}", e))?;

        let sample_rate = config.sample_rate().0;
        let channels = config.channels();

        self.sample_rate = sample_rate;
        self.channels = channels;

        let samples = self.samples.clone();
        if let Ok(mut s) = samples.lock() {
            s.clear();
        }

        let (tx, rx) = std::sync::mpsc::channel::<f32>();
        let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
        let tx_clone = tx.clone();

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut s) = samples.lock() {
                        s.extend_from_slice(data);
                    }
                    if !data.is_empty() {
                        let peak = data.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
                        let _ = tx_clone.send(peak);
                    }
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut s) = samples.lock() {
                        s.extend(data.iter().map(|&x| x as f32 / i16::MAX as f32));
                    }
                    if !data.is_empty() {
                        let peak = data.iter().map(|&x| (x as f32 / i16::MAX as f32).abs()).fold(0.0f32, f32::max);
                        let _ = tx_clone.send(peak);
                    }
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::U16 => device.build_input_stream(
                &config.into(),
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut s) = samples.lock() {
                        s.extend(data.iter().map(|&x| {
                            (x as f32 - u16::MAX as f32 / 2.0) / (u16::MAX as f32 / 2.0)
                        }));
                    }
                    if !data.is_empty() {
                        let peak = data.iter().map(|&x| {
                            ((x as f32 - u16::MAX as f32 / 2.0) / (u16::MAX as f32 / 2.0)).abs()
                        }).fold(0.0f32, f32::max);
                        let _ = tx_clone.send(peak);
                    }
                },
                err_fn,
                None,
            ),
            _ => return Err("Unsupported sample format".to_string()),
        }
        .map_err(|e| format!("Failed to build input stream: {}", e))?;

        stream.play().map_err(|e| format!("Failed to start stream: {}", e))?;
        self.stream = Some(SendStream(stream));

        let app_handle_clone = app_handle.clone();
        std::thread::spawn(move || {
            loop {
                match rx.recv_timeout(std::time::Duration::from_millis(100)) {
                    Ok(first_val) => {
                        let mut max_val = first_val;
                        while let Ok(val) = rx.try_recv() {
                            if val > max_val {
                                max_val = val;
                            }
                        }
                        let level = (max_val * 100.0) as u32;
                        let level = level.min(100);
                        let _ = app_handle_clone.emit("recording-mic-level", level);
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        let _ = app_handle_clone.emit("recording-mic-level", 0);
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        break;
                    }
                }
            }
            let _ = app_handle_clone.emit("recording-mic-level", 0);
        });

        Ok(())
    }

    pub fn stop_recording(&mut self, path: &str) -> Result<(), String> {
        if let Some(send_stream) = self.stream.take() {
            drop(send_stream.0);
        } else {
            return Err("Not recording".to_string());
        }

        let raw_samples = {
            let mut s = self.samples.lock().map_err(|e| format!("Lock error: {}", e))?;
            std::mem::take(&mut *s)
        };

        if raw_samples.is_empty() {
            return Err("No audio captured".to_string());
        }

        // Convert multi-channel to mono if necessary
        let mono_samples = if self.channels > 1 {
            let mut mono = Vec::with_capacity(raw_samples.len() / self.channels as usize);
            for chunk in raw_samples.chunks(self.channels as usize) {
                let sum: f32 = chunk.iter().sum();
                mono.push(sum / self.channels as f32);
            }
            mono
        } else {
            raw_samples
        };

        // Resample to 16kHz
        let target_sample_rate = 16000;
        let resampled = resample(&mono_samples, self.sample_rate, target_sample_rate);

        // Write to WAV file
        let spec = WavSpec {
            channels: 1,
            sample_rate: target_sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = WavWriter::create(path, spec)
            .map_err(|e| format!("Failed to create WAV writer: {}", e))?;

        for &sample in &resampled {
            let clamped = sample.clamp(-1.0, 1.0);
            let int_sample = (clamped * i16::MAX as f32) as i16;
            writer
                .write_sample(int_sample)
                .map_err(|e| format!("Failed to write sample: {}", e))?;
        }

        writer.finalize().map_err(|e| format!("Failed to finalize WAV: {}", e))?;

        Ok(())
    }

    pub fn cancel_recording(&mut self) -> Result<(), String> {
        if let Some(send_stream) = self.stream.take() {
            drop(send_stream.0);
        } else {
            return Err("Not recording".to_string());
        }
        if let Ok(mut s) = self.samples.lock() {
            s.clear();
        }
        Ok(())
    }

    pub fn start_mic_test(&mut self, app_handle: AppHandle, device_name: Option<&str>) -> Result<(), String> {
        if self.stream.is_some() {
            return Err("An audio stream is already active".to_string());
        }

        let host = cpal::default_host();
        let device = if let Some(name) = device_name {
            if name == "Default" || name.is_empty() {
                host.default_input_device()
                    .ok_or_else(|| "No default input device found".to_string())?
            } else {
                let devices = host.input_devices()
                    .map_err(|e| format!("Failed to list input devices: {}", e))?;
                let mut found_device = None;
                for d in devices {
                    if let Ok(d_name) = d.name() {
                        if d_name == name {
                            found_device = Some(d);
                            break;
                        }
                    }
                }
                found_device.ok_or_else(|| format!("Input device '{}' not found", name))?
            }
        } else {
            host.default_input_device()
                .ok_or_else(|| "No default input device found".to_string())?
        };

        let config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get default input config: {}", e))?;

        let (tx, rx) = std::sync::mpsc::channel::<f32>();
        let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
        let tx_clone = tx.clone();

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if !data.is_empty() {
                        let peak = data.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
                        let _ = tx_clone.send(peak);
                    }
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    if !data.is_empty() {
                        let peak = data.iter().map(|&x| (x as f32 / i16::MAX as f32).abs()).fold(0.0f32, f32::max);
                        let _ = tx_clone.send(peak);
                    }
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::U16 => device.build_input_stream(
                &config.into(),
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    if !data.is_empty() {
                        let peak = data.iter().map(|&x| {
                            ((x as f32 - u16::MAX as f32 / 2.0) / (u16::MAX as f32 / 2.0)).abs()
                        }).fold(0.0f32, f32::max);
                        let _ = tx_clone.send(peak);
                    }
                },
                err_fn,
                None,
            ),
            _ => return Err("Unsupported sample format".to_string()),
        }
        .map_err(|e| format!("Failed to build input stream: {}", e))?;

        stream.play().map_err(|e| format!("Failed to start stream: {}", e))?;
        self.stream = Some(SendStream(stream));

        let app_handle_clone = app_handle.clone();
        std::thread::spawn(move || {
            loop {
                match rx.recv_timeout(std::time::Duration::from_millis(100)) {
                    Ok(first_val) => {
                        let mut max_val = first_val;
                        while let Ok(val) = rx.try_recv() {
                            if val > max_val {
                                max_val = val;
                            }
                        }
                        let level = (max_val * 100.0) as u32;
                        let level = level.min(100);
                        let _ = app_handle_clone.emit("mic-test-level", level);
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        let _ = app_handle_clone.emit("mic-test-level", 0);
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        break;
                    }
                }
            }
            let _ = app_handle_clone.emit("mic-test-level", 0);
        });

        Ok(())
    }

    pub fn stop_mic_test(&mut self) -> Result<(), String> {
        if let Some(send_stream) = self.stream.take() {
            drop(send_stream.0);
            Ok(())
        } else {
            Err("Mic test is not active".to_string())
        }
    }
}

fn resample(input: &[f32], from_sample_rate: u32, to_sample_rate: u32) -> Vec<f32> {
    if from_sample_rate == to_sample_rate {
        return input.to_vec();
    }
    let ratio = from_sample_rate as f64 / to_sample_rate as f64;
    let new_len = (input.len() as f64 / ratio).round() as usize;
    let mut output = Vec::with_capacity(new_len);
    for i in 0..new_len {
        let input_idx = i as f64 * ratio;
        let idx_floor = input_idx.floor() as usize;
        let idx_ceil = (idx_floor + 1).min(input.len() - 1);
        let t = input_idx - idx_floor as f64;
        if idx_floor < input.len() {
            let val = (1.0 - t) as f32 * input[idx_floor] + t as f32 * input[idx_ceil];
            output.push(val);
        }
    }
    output
}
