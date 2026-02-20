use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

#[allow(dead_code)]
const MODEL_PATH: &str = "models/ggml-base.en.bin";

#[allow(dead_code)]
pub struct SttManager {
    ctx: WhisperContext,
}

#[allow(dead_code)]
impl SttManager {
    pub fn new(model_path: &str) -> Result<Self> {
        // Just check if file exists, if not warn but try loading or error out
        // The user might put it in a different place.
        // For MVP, if path provided, use it. If not, use default.

        let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
            .map_err(|e| {
                anyhow::anyhow!("Failed to load Whisper model from '{}': {}", model_path, e)
            })?;

        Ok(Self { ctx })
    }

    pub async fn listen_once(&self, duration_secs: u64) -> Result<String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No input device"))?;
        let config = device.default_input_config()?;

        let sample_rate = config.sample_rate().0;
        let channels = config.channels();

        let recorded_samples = Arc::new(Mutex::new(Vec::new()));
        let samples_clone = recorded_samples.clone();

        let err_fn = move |err| {
            eprintln!("an error occurred on stream: {}", err);
        };

        let stream = device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &_| {
                let mut guard = samples_clone.lock().unwrap();
                if channels == 2 {
                    // Simple stereo to mono mix
                    for chunk in data.chunks(2) {
                        if chunk.len() == 2 {
                            let mono = (chunk[0] + chunk[1]) / 2.0;
                            guard.push(mono);
                        }
                    }
                } else {
                    guard.extend_from_slice(data);
                }
            },
            err_fn,
            None,
        )?;

        stream.play()?;

        println!("Listening for {} seconds...", duration_secs);
        tokio::time::sleep(std::time::Duration::from_secs(duration_secs)).await;

        drop(stream);
        println!("Processing audio...");

        let raw_samples = {
            let guard = recorded_samples.lock().unwrap();
            guard.clone()
        };

        // Resample logic to 16000 Hz
        let samples = if sample_rate != 16000 {
            self.resample(&raw_samples, sample_rate, 16000)
        } else {
            raw_samples
        };

        // Whisper Inference
        let mut state = self.ctx.create_state().expect("failed into create state");

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(4);
        params.set_language(Some("en"));
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        state
            .full(params, &samples)
            .map_err(|e| anyhow::anyhow!("Whisper inference failed: {}", e))?;

        let num_segments = state.full_n_segments().unwrap_or(0);
        let mut text = String::new();
        for i in 0..num_segments {
            if let Ok(segment) = state.full_get_segment_text(i) {
                text.push_str(&segment);
                text.push(' ');
            }
        }

        Ok(text.trim().to_string())
    }

    fn resample(&self, input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
        if from_rate == to_rate {
            return input.to_vec();
        }

        let ratio = from_rate as f32 / to_rate as f32;
        let output_len = (input.len() as f32 / ratio) as usize;
        let mut output = Vec::with_capacity(output_len);

        for i in 0..output_len {
            let src_idx = i as f32 * ratio;
            let idx_floor = src_idx.floor() as usize;
            let idx_ceil = (idx_floor + 1).min(input.len() - 1);
            let t = src_idx - idx_floor as f32;

            // Linear interpolation
            let val = input[idx_floor] * (1.0 - t) + input[idx_ceil] * t;
            output.push(val);
        }
        output
    }
}
