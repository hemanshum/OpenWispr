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
    pub noise_gate_enabled: Arc<std::sync::atomic::AtomicBool>,
    is_paused: Arc<std::sync::atomic::AtomicBool>,
    paused_samples: Arc<Mutex<Vec<f32>>>,
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            stream: None,
            samples: Arc::new(Mutex::new(Vec::new())),
            sample_rate: 0,
            channels: 0,
            noise_gate_enabled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            is_paused: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            paused_samples: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn start_recording(&mut self, app_handle: AppHandle, device_name: Option<&str>) -> Result<(), String> {
        let api_config = crate::config::AppConfig::load(&app_handle);
        self.noise_gate_enabled.store(api_config.noise_gate, std::sync::atomic::Ordering::SeqCst);

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
        let paused_samples = self.paused_samples.clone();
        let is_paused = self.is_paused.clone();
        if let Ok(mut s) = samples.lock() {
            s.clear();
        }
        if let Ok(mut ps) = paused_samples.lock() {
            ps.clear();
        }
        is_paused.store(false, std::sync::atomic::Ordering::SeqCst);

        let (tx, rx) = std::sync::mpsc::channel::<f32>();
        let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
        let tx_clone = tx.clone();

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let paused = is_paused.load(std::sync::atomic::Ordering::SeqCst);
                    if paused {
                        if let Ok(mut ps) = paused_samples.lock() {
                            ps.extend_from_slice(data);
                        }
                    } else {
                        if let Ok(mut s) = samples.lock() {
                            s.extend_from_slice(data);
                        }
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
                    let paused = is_paused.load(std::sync::atomic::Ordering::SeqCst);
                    if paused {
                        if let Ok(mut ps) = paused_samples.lock() {
                            ps.extend(data.iter().map(|&x| x as f32 / i16::MAX as f32));
                        }
                    } else {
                        if let Ok(mut s) = samples.lock() {
                            s.extend(data.iter().map(|&x| x as f32 / i16::MAX as f32));
                        }
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
                    let paused = is_paused.load(std::sync::atomic::Ordering::SeqCst);
                    if paused {
                        if let Ok(mut ps) = paused_samples.lock() {
                            ps.extend(data.iter().map(|&x| {
                                (x as f32 - u16::MAX as f32 / 2.0) / (u16::MAX as f32 / 2.0)
                            }));
                        }
                    } else {
                        if let Ok(mut s) = samples.lock() {
                            s.extend(data.iter().map(|&x| {
                                (x as f32 - u16::MAX as f32 / 2.0) / (u16::MAX as f32 / 2.0)
                            }));
                        }
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
        let noise_gate_enabled = self.noise_gate_enabled.clone();
        std::thread::spawn(move || {
            let mut hold_counter = 0;
            loop {
                match rx.recv_timeout(std::time::Duration::from_millis(100)) {
                    Ok(first_val) => {
                        let mut max_val = first_val;
                        while let Ok(val) = rx.try_recv() {
                            if val > max_val {
                                max_val = val;
                            }
                        }
                        
                        let use_noise_gate = noise_gate_enabled.load(std::sync::atomic::Ordering::SeqCst);
                        if use_noise_gate {
                            if max_val >= 0.08 {
                                hold_counter = 2; // 200ms hold time
                            } else if hold_counter > 0 {
                                hold_counter -= 1;
                            } else {
                                max_val = 0.0;
                            }
                        }

                        let level = (max_val * 100.0) as u32;
                        let level = level.min(100);
                        let _ = app_handle_clone.emit("recording-mic-level", level);
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        let use_noise_gate = noise_gate_enabled.load(std::sync::atomic::Ordering::SeqCst);
                        if use_noise_gate {
                            if hold_counter > 0 {
                                hold_counter -= 1;
                            }
                        }
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

    pub fn stop_recording(&mut self, path: &str, use_noise_gate: bool) -> Result<(), String> {
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
        let mut resampled = resample(&mono_samples, self.sample_rate, target_sample_rate);

        // Apply DSP Noise Gate if enabled
        if use_noise_gate {
            let frame_size = 320; // 20ms at 16kHz
            let threshold = 0.08; // ~-22dB
            let hold_frames = 10; // 200ms hold time
            let mut hold_counter = 0;

            for chunk in resampled.chunks_mut(frame_size) {
                let peak = chunk.iter().map(|&x| x.abs()).fold(0.0f32, |m, v| m.max(v));
                if peak >= threshold {
                    hold_counter = hold_frames;
                } else if hold_counter > 0 {
                    hold_counter -= 1;
                } else {
                    for val in chunk.iter_mut() {
                        *val = 0.0;
                    }
                }
            }

            // Trim leading and trailing silence (zeroed frames) to reduce payload
            let silence_threshold = 0.001;
            let margin = 160; // 10ms safety margin at 16kHz

            let first_nonsilent = resampled.iter()
                .position(|&s| s.abs() > silence_threshold)
                .unwrap_or(0);
            let last_nonsilent = resampled.iter()
                .rposition(|&s| s.abs() > silence_threshold)
                .unwrap_or(resampled.len().saturating_sub(1));

            let trim_start = first_nonsilent.saturating_sub(margin);
            let trim_end = (last_nonsilent + margin + 1).min(resampled.len());

            if trim_start < trim_end {
                resampled = resampled[trim_start..trim_end].to_vec();
            }
        }

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
        if let Ok(mut ps) = self.paused_samples.lock() {
            ps.clear();
        }
        self.is_paused.store(false, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    pub fn pause_recording(&mut self) -> Result<(), String> {
        if self.stream.is_none() {
            return Err("Not recording".to_string());
        }
        self.is_paused.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    pub fn resume_recording(&mut self) -> Result<(), String> {
        if self.stream.is_none() {
            return Err("Not recording".to_string());
        }
        // Move paused samples back to main samples
        if let Ok(mut ps) = self.paused_samples.lock() {
            if !ps.is_empty() {
                if let Ok(mut s) = self.samples.lock() {
                    s.extend(ps.drain(..));
                }
            }
        }
        self.is_paused.store(false, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn start_mic_test(&mut self, app_handle: AppHandle, device_name: Option<&str>) -> Result<(), String> {
        let api_config = crate::config::AppConfig::load(&app_handle);
        self.noise_gate_enabled.store(api_config.noise_gate, std::sync::atomic::Ordering::SeqCst);

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
        let noise_gate_enabled = self.noise_gate_enabled.clone();
        std::thread::spawn(move || {
            let mut hold_counter = 0;
            loop {
                match rx.recv_timeout(std::time::Duration::from_millis(100)) {
                    Ok(first_val) => {
                        let mut max_val = first_val;
                        while let Ok(val) = rx.try_recv() {
                            if val > max_val {
                                max_val = val;
                            }
                        }
                        
                        let use_noise_gate = noise_gate_enabled.load(std::sync::atomic::Ordering::SeqCst);
                        if use_noise_gate {
                            if max_val >= 0.08 {
                                hold_counter = 2; // 200ms hold time
                            } else if hold_counter > 0 {
                                hold_counter -= 1;
                            } else {
                                max_val = 0.0;
                            }
                        }

                        let level = (max_val * 100.0) as u32;
                        let level = level.min(100);
                        let _ = app_handle_clone.emit("mic-test-level", level);
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        let use_noise_gate = noise_gate_enabled.load(std::sync::atomic::Ordering::SeqCst);
                        if use_noise_gate {
                            if hold_counter > 0 {
                                hold_counter -= 1;
                            }
                        }
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
