//! Local Whisper transcription using Candle
//!
//! This module provides local audio transcription using OpenAI's Whisper model
//! via the Candle ML framework. It supports loading GGUF quantized models for
//! efficient CPU inference.
//! Heavily "inspired" by the Candle Whisper example:
//! https://github.com/huggingface/candle/tree/main/candle-examples/whisper

use crate::config::paths::Paths;

pub const LOCAL_WHISPER_MODEL_CONFIG_KEY: &str = "LOCAL_WHISPER_MODEL";
use anyhow::{Context, Result};
use candle_core::{Device, IndexOp, Tensor};
use candle_nn::ops::log_softmax;
use candle_transformers::models::whisper::{self as m, audio, Config, N_FRAMES};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use symphonia::core::audio::{AudioBufferRef, Layout, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use tokenizers::Tokenizer;
use utoipa::ToSchema;

// Common suppress tokens for all Whisper models
const SUPPRESS_TOKENS: &[u32] = &[
    1, 2, 7, 8, 9, 10, 14, 25, 26, 27, 28, 29, 31, 58, 59, 60, 61, 62, 63, 90, 91, 92, 93, 359,
    503, 522, 542, 873, 893, 902, 918, 922, 931, 1350, 1853, 1982, 2460, 2627, 3246, 3253, 3268,
    3536, 3846, 3961, 4183, 4667, 6585, 6647, 7273, 9061, 9383, 10428, 10929, 11938, 12033, 12331,
    12562, 13793, 14157, 14635, 15265, 15618, 16553, 16604, 18362, 18956, 20075, 21675, 22520,
    26130, 26161, 26435, 28279, 29464, 31650, 32302, 32470, 36865, 42863, 47425, 49870, 50254,
    50258, 50360, 50362,
];

// Special token IDs
const SOT_TOKEN: u32 = 50258;
const TRANSCRIBE_TOKEN: u32 = 50359;
const EOT_TOKEN: u32 = 50257;
const TIMESTAMP_BEGIN: u32 = 50364;
const SAMPLE_BEGIN: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WhisperModel {
    /// Model identifier (e.g., "tiny", "base", "small")
    pub id: &'static str,
    /// Model file size in MB
    pub size_mb: u32,
    /// Download URL from HuggingFace
    pub url: &'static str,
    /// Description
    pub description: &'static str,
}

const MODELS: &[WhisperModel] = &[
    WhisperModel {
        id: "tiny",
        size_mb: 40,
        url: "https://huggingface.co/oxide-lab/whisper-tiny-GGUF/resolve/main/model-tiny-q80.gguf",
        description: "Fastest, ~2-3x realtime on CPU (5-10x with GPU)",
    },
    WhisperModel {
        id: "base",
        size_mb: 78,
        url: "https://huggingface.co/oxide-lab/whisper-base-GGUF/resolve/main/whisper-base-q8_0.gguf",
        description: "Good balance, ~1.5-2x realtime on CPU (4-8x with GPU)",
    },
    WhisperModel {
        id: "small",
        size_mb: 247,
        url: "https://huggingface.co/oxide-lab/whisper-small-GGUF/resolve/main/whisper-small-q8_0.gguf",
        description: "High accuracy, ~0.8-1x realtime on CPU (3-5x with GPU)",
    },
    WhisperModel {
        id: "medium",
        size_mb: 777,
        url: "https://huggingface.co/oxide-lab/whisper-medium-GGUF/resolve/main/whisper-medium-q8_0.gguf",
        description: "Highest accuracy, ~0.5x realtime on CPU (2-4x with GPU)",
    },
];

impl WhisperModel {
    pub fn local_path(&self) -> PathBuf {
        let filename = self.url.rsplit('/').next().unwrap_or("");
        Paths::in_data_dir("models").join(filename)
    }

    pub fn is_downloaded(&self) -> bool {
        self.local_path().exists()
    }

    pub fn config(&self) -> Config {
        match self.id {
            "tiny" => Config {
                num_mel_bins: 80,
                max_source_positions: 1500,
                d_model: 384,
                encoder_attention_heads: 6,
                encoder_layers: 4,
                decoder_attention_heads: 6,
                decoder_layers: 4,
                vocab_size: 51865,
                suppress_tokens: SUPPRESS_TOKENS.to_vec(),
                max_target_positions: 448,
            },
            "base" => Config {
                num_mel_bins: 80,
                max_source_positions: 1500,
                d_model: 512,
                encoder_attention_heads: 8,
                encoder_layers: 6,
                decoder_attention_heads: 8,
                decoder_layers: 6,
                vocab_size: 51865,
                suppress_tokens: SUPPRESS_TOKENS.to_vec(),
                max_target_positions: 448,
            },
            "small" => Config {
                num_mel_bins: 80,
                max_source_positions: 1500,
                d_model: 768,
                encoder_attention_heads: 12,
                encoder_layers: 12,
                decoder_attention_heads: 12,
                decoder_layers: 12,
                vocab_size: 51865,
                suppress_tokens: SUPPRESS_TOKENS.to_vec(),
                max_target_positions: 448,
            },
            "medium" => Config {
                num_mel_bins: 80,
                max_source_positions: 1500,
                d_model: 1024,
                encoder_attention_heads: 16,
                encoder_layers: 24,
                decoder_attention_heads: 16,
                decoder_layers: 24,
                vocab_size: 51865,
                suppress_tokens: SUPPRESS_TOKENS.to_vec(),
                max_target_positions: 448,
            },
            _ => {
                tracing::warn!("Unknown model '{}', falling back to tiny config", self.id);
                Config {
                    num_mel_bins: 80,
                    max_source_positions: 1500,
                    d_model: 384,
                    encoder_attention_heads: 6,
                    encoder_layers: 4,
                    decoder_attention_heads: 6,
                    decoder_layers: 4,
                    vocab_size: 51865,
                    suppress_tokens: SUPPRESS_TOKENS.to_vec(),
                    max_target_positions: 448,
                }
            }
        }
    }
}

pub fn available_models() -> &'static [WhisperModel] {
    MODELS
}

pub fn get_model(id: &str) -> Option<&'static WhisperModel> {
    MODELS.iter().find(|m| m.id == id)
}

pub fn recommend_model() -> &'static str {
    let has_gpu_or_metal = Device::new_cuda(0).is_ok() || Device::new_metal(0).is_ok();

    if has_gpu_or_metal {
        "small"
    } else {
        let cpu_count = sys_info::cpu_num().unwrap_or(1) as u64;
        let cpu_speed_mhz = sys_info::cpu_speed().unwrap_or(0);

        if cpu_count * cpu_speed_mhz >= 16_000 {
            "base"
        } else {
            "tiny"
        }
    }
}

pub struct WhisperTranscriber {
    model: m::quantized_model::Whisper,
    config: Config,
    device: Device,
    mel_filters: Vec<f32>,
    tokenizer: Tokenizer,
    eot_token: u32,
    no_timestamps_token: u32,
    language_token: u32,
    max_initial_timestamp_index: u32,
}

impl WhisperTranscriber {
    pub fn new_with_tokenizer<P: AsRef<Path>>(
        model_id: &str,
        model_path: P,
        bundled_tokenizer: &str,
    ) -> Result<Self> {
        tracing::debug!(model_id, "initializing whisper transcriber");

        let device = if let Ok(device) = Device::new_cuda(0) {
            tracing::debug!("using CUDA device");
            device
        } else if let Ok(device) = Device::new_metal(0) {
            tracing::debug!("using Metal device");
            device
        } else {
            tracing::debug!("using CPU device");
            Device::Cpu
        };

        let model_path_ref = model_path.as_ref();
        tracing::debug!(path = %model_path_ref.display(), "loading model from path");

        if !model_path_ref.exists() {
            anyhow::bail!("Model file not found: {}", model_path_ref.display());
        }

        let model =
            get_model(model_id).ok_or_else(|| anyhow::anyhow!("Unknown model: {}", model_id))?;
        let config = model.config();
        tracing::debug!(
            num_mel_bins = config.num_mel_bins,
            d_model = config.d_model,
            "loaded model config"
        );

        let mel_bytes = match config.num_mel_bins {
            80 => include_bytes!("whisper_data/melfilters.bytes").as_slice(),
            128 => include_bytes!("whisper_data/melfilters128.bytes").as_slice(),
            nmel => anyhow::bail!("unexpected num_mel_bins {nmel}"),
        };
        let mut mel_filters = vec![0f32; mel_bytes.len() / 4];
        byteorder::ReadBytesExt::read_f32_into::<byteorder::LittleEndian>(
            &mut &mel_bytes[..],
            &mut mel_filters,
        )?;
        tracing::debug!(mel_filters_len = mel_filters.len(), "loaded mel filters");

        tracing::debug!("loading GGUF model weights");
        let vb = candle_transformers::quantized_var_builder::VarBuilder::from_gguf(
            model_path_ref,
            &device,
        )?;
        let model = m::quantized_model::Whisper::load(&vb, config.clone())?;
        tracing::debug!("model weights loaded successfully");

        tracing::debug!("loading tokenizer");
        let tokenizer = Self::load_tokenizer(model_path_ref, Some(bundled_tokenizer))?;
        tracing::debug!("tokenizer loaded successfully");

        Ok(Self {
            model,
            config,
            device,
            mel_filters,
            tokenizer,
            eot_token: 50257,
            no_timestamps_token: 50363,
            language_token: 50259,
            max_initial_timestamp_index: 50,
        })
    }

    fn load_tokenizer(model_dir: &Path, bundled_tokenizer: Option<&str>) -> Result<Tokenizer> {
        let tokenizer_path = model_dir
            .parent()
            .unwrap_or(model_dir)
            .join("tokenizer.json");

        if tokenizer_path.exists() {
            return Tokenizer::from_file(tokenizer_path)
                .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e));
        }

        if let Some(tokenizer_json) = bundled_tokenizer {
            if let Some(parent) = tokenizer_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&tokenizer_path, tokenizer_json)?;
            return Tokenizer::from_file(tokenizer_path)
                .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e));
        }

        anyhow::bail!(
            "Tokenizer not found at {} and no bundled tokenizer provided",
            tokenizer_path.display()
        )
    }

    pub fn transcribe(&mut self, audio_data: &[u8]) -> Result<String> {
        tracing::debug!(audio_bytes = audio_data.len(), "starting transcription");

        if audio_data.is_empty() {
            tracing::debug!("empty audio data received");
            return Ok(String::new());
        }

        let (mel_tensor, actual_content_frames) = self.prepare_audio_input(audio_data)?;
        let (_, _, padded_frames) = mel_tensor.dims3()?;
        // Use actual content frames (from PCM length), not padded mel frames
        let content_frames = actual_content_frames.min(padded_frames);
        let audio_duration_secs = (content_frames * 160) as f32 / 16000.0;
        tracing::debug!(
            actual_content_frames,
            padded_frames,
            content_frames,
            audio_duration_secs,
            "prepared mel spectrogram"
        );

        if content_frames == 0 {
            tracing::debug!("no content frames in mel spectrogram");
            return Ok(String::new());
        }

        let num_segments = content_frames.div_ceil(N_FRAMES);
        tracing::debug!(
            num_segments,
            n_frames = N_FRAMES,
            "processing audio segments"
        );

        let mut all_text_tokens = Vec::new();
        let mut seek = 0;
        let mut segment_num = 0;

        while seek < content_frames {
            segment_num += 1;
            let segment_size = usize::min(content_frames - seek, N_FRAMES);
            tracing::debug!(segment_num, segment_size, seek, "processing segment");

            let segment_text_tokens =
                self.process_segment(&mel_tensor, seek, segment_size, segment_num, num_segments)?;

            tracing::debug!(
                tokens_in_segment = segment_text_tokens.len(),
                "segment produced tokens"
            );
            all_text_tokens.extend(segment_text_tokens);
            seek += segment_size;
        }

        tracing::debug!(
            total_tokens = all_text_tokens.len(),
            "decoding tokens to text"
        );

        if all_text_tokens.is_empty() {
            tracing::warn!(
                audio_bytes = audio_data.len(),
                audio_duration_secs,
                num_segments,
                "no tokens produced from audio - possible silence or unrecognized speech"
            );
            return Ok(String::new());
        }

        let result = self.decode_tokens(&all_text_tokens)?;
        tracing::debug!(result_len = result.len(), "transcription complete");
        Ok(result)
    }

    fn prepare_audio_input(&self, audio_data: &[u8]) -> Result<(Tensor, usize)> {
        tracing::debug!(audio_bytes = audio_data.len(), "decoding audio to PCM");
        let pcm_data = decode_audio_simple(audio_data)?;
        let pcm_samples = pcm_data.len();
        tracing::debug!(pcm_samples, "converting PCM to mel spectrogram");

        // Calculate actual content frames from PCM length (HOP_LENGTH = 160)
        // pcm_to_mel pads to 30 seconds, but we only want to process actual audio
        let actual_content_frames = pcm_samples / 160;

        let mel = audio::pcm_to_mel(&self.config, &pcm_data, &self.mel_filters);
        let mel_len = mel.len();
        tracing::debug!(
            mel_len,
            num_mel_bins = self.config.num_mel_bins,
            actual_content_frames,
            "creating mel tensor"
        );
        let mel_tensor = Tensor::from_vec(
            mel,
            (
                1,
                self.config.num_mel_bins,
                mel_len / self.config.num_mel_bins,
            ),
            &self.device,
        )?;

        Ok((mel_tensor, actual_content_frames))
    }

    fn process_segment(
        &mut self,
        mel_tensor: &Tensor,
        seek: usize,
        segment_size: usize,
        _segment_num: usize,
        _num_segments: usize,
    ) -> Result<Vec<u32>> {
        let _time_offset = (seek * 160) as f32 / 16000.0; // HOP_LENGTH = 160
        let _segment_duration = (segment_size * 160) as f32 / 16000.0;
        let mel_segment = mel_tensor.narrow(2, seek, segment_size)?;

        // Debug: check mel segment statistics
        let mel_flat = mel_segment.flatten_all()?;
        let mel_mean: f32 = mel_flat.mean(0)?.to_scalar()?;
        let mel_max: f32 = mel_flat.max(0)?.to_scalar()?;
        let mel_min: f32 = mel_flat.min(0)?.to_scalar()?;
        tracing::debug!(mel_mean, mel_max, mel_min, "mel segment statistics");

        self.model.decoder.reset_kv_cache();
        let audio_features = self.model.encoder.forward(&mel_segment, true)?;

        // Debug: check encoder output statistics
        let af_flat = audio_features.flatten_all()?;
        let af_mean: f32 = af_flat.mean(0)?.to_scalar()?;
        let af_max: f32 = af_flat.max(0)?.to_scalar()?;
        let af_min: f32 = af_flat.min(0)?.to_scalar()?;
        tracing::debug!(af_mean, af_max, af_min, "audio features statistics");
        let suppress_tokens = {
            let mut suppress = vec![0f32; self.config.vocab_size];
            for &token_id in &self.config.suppress_tokens {
                if (token_id as usize) < suppress.len() {
                    suppress[token_id as usize] = f32::NEG_INFINITY;
                }
            }
            suppress[self.no_timestamps_token as usize] = f32::NEG_INFINITY;
            Tensor::from_vec(suppress, self.config.vocab_size, &self.device)?
        };
        let mut tokens = vec![SOT_TOKEN, self.language_token, TRANSCRIBE_TOKEN];
        let sample_len = self.config.max_target_positions / 2;

        for i in 0..sample_len {
            let tokens_tensor = Tensor::new(tokens.as_slice(), &self.device)?.unsqueeze(0)?;
            let ys = self
                .model
                .decoder
                .forward(&tokens_tensor, &audio_features, i == 0)?;

            let (_, seq_len, _) = ys.dims3()?;
            let mut logits = self
                .model
                .decoder
                .final_linear(&ys.i((..1, seq_len - 1..))?)?
                .i(0)?
                .i(0)?;

            logits = self.apply_timestamp_rules(&logits, &tokens)?;
            let logits = logits.broadcast_add(&suppress_tokens)?;

            let logits_v: Vec<f32> = logits.to_vec1()?;
            let next_token = logits_v
                .iter()
                .enumerate()
                .max_by(|(_, u), (_, v)| u.total_cmp(v))
                .map(|(i, _)| i as u32)
                .unwrap();

            tokens.push(next_token);

            if next_token == EOT_TOKEN {
                tracing::debug!(tokens_generated = tokens.len() - 3, "EOT token received");
                break;
            }
            if tokens.len() > self.config.max_target_positions {
                tracing::debug!("max target positions reached");
                break;
            }
        }

        // Log all generated tokens for debugging
        tracing::debug!(
            all_tokens = ?&tokens[3..],
            "all tokens generated in segment"
        );

        let segment_text_tokens: Vec<u32> = tokens[3..]
            .iter()
            .filter(|&&t| t != EOT_TOKEN && t < TIMESTAMP_BEGIN)
            .copied()
            .collect();

        if segment_text_tokens.is_empty() && tokens.len() > 3 {
            tracing::debug!(
                raw_tokens = ?&tokens[3..],
                "no text tokens found after filtering (all were EOT or timestamps)"
            );
        }

        Ok(segment_text_tokens)
    }

    fn apply_timestamp_rules(&self, input_logits: &Tensor, tokens: &[u32]) -> Result<Tensor> {
        let device = input_logits.device().clone();
        let vocab_size = self.model.config.vocab_size as u32;

        let sampled_tokens = if tokens.len() > SAMPLE_BEGIN {
            &tokens[SAMPLE_BEGIN..]
        } else {
            &[]
        };

        let mut masks = Vec::new();
        let mut mask_buffer = vec![0.0f32; vocab_size as usize];

        self.apply_timestamp_pairing_rule(
            sampled_tokens,
            vocab_size,
            &mut masks,
            &mut mask_buffer,
            &device,
        )?;
        self.apply_initial_timestamp_rule(
            tokens.len(),
            vocab_size,
            &mut masks,
            &mut mask_buffer,
            &device,
        )?;

        let mut logits = input_logits.clone();
        for mask in masks {
            logits = logits.broadcast_add(&mask)?;
        }

        logits =
            self.apply_timestamp_probability_rule(&logits, vocab_size, &mut mask_buffer, &device)?;

        Ok(logits)
    }

    fn apply_timestamp_pairing_rule(
        &self,
        sampled_tokens: &[u32],
        vocab_size: u32,
        masks: &mut Vec<Tensor>,
        mask_buffer: &mut [f32],
        device: &Device,
    ) -> Result<()> {
        if sampled_tokens.is_empty() {
            return Ok(());
        }

        let last_was_timestamp = sampled_tokens
            .last()
            .map(|&t| t >= TIMESTAMP_BEGIN)
            .unwrap_or(false);

        let penultimate_was_timestamp = if sampled_tokens.len() >= 2 {
            sampled_tokens[sampled_tokens.len() - 2] >= TIMESTAMP_BEGIN
        } else {
            true
        };

        if last_was_timestamp {
            if penultimate_was_timestamp {
                for i in 0..vocab_size {
                    mask_buffer[i as usize] = if i >= TIMESTAMP_BEGIN {
                        f32::NEG_INFINITY
                    } else {
                        0.0
                    };
                }
                masks.push(Tensor::new(mask_buffer as &[f32], device)?);
            } else {
                for i in 0..vocab_size {
                    mask_buffer[i as usize] = if i < self.eot_token {
                        f32::NEG_INFINITY
                    } else {
                        0.0
                    };
                }
                masks.push(Tensor::new(mask_buffer as &[f32], device)?);
            }
        }

        let timestamp_tokens: Vec<u32> = sampled_tokens
            .iter()
            .filter(|&&t| t >= TIMESTAMP_BEGIN)
            .cloned()
            .collect();

        if !timestamp_tokens.is_empty() {
            let timestamp_last = if last_was_timestamp && !penultimate_was_timestamp {
                *timestamp_tokens.last().unwrap()
            } else {
                timestamp_tokens.last().unwrap() + 1
            };

            for i in 0..vocab_size {
                mask_buffer[i as usize] = if i >= TIMESTAMP_BEGIN && i < timestamp_last {
                    f32::NEG_INFINITY
                } else {
                    0.0
                };
            }
            masks.push(Tensor::new(mask_buffer as &[f32], device)?);
        }

        Ok(())
    }

    fn apply_initial_timestamp_rule(
        &self,
        tokens_len: usize,
        vocab_size: u32,
        masks: &mut Vec<Tensor>,
        mask_buffer: &mut [f32],
        device: &Device,
    ) -> Result<()> {
        if tokens_len != SAMPLE_BEGIN {
            return Ok(());
        }

        for i in 0..vocab_size {
            mask_buffer[i as usize] = if i < TIMESTAMP_BEGIN {
                f32::NEG_INFINITY
            } else {
                0.0
            };
        }
        masks.push(Tensor::new(mask_buffer as &[f32], device)?);

        let last_allowed = TIMESTAMP_BEGIN + self.max_initial_timestamp_index;
        if last_allowed < vocab_size {
            for i in 0..vocab_size {
                mask_buffer[i as usize] = if i > last_allowed {
                    f32::NEG_INFINITY
                } else {
                    0.0
                };
            }
            masks.push(Tensor::new(mask_buffer as &[f32], device)?);
        }

        Ok(())
    }

    fn apply_timestamp_probability_rule(
        &self,
        logits: &Tensor,
        vocab_size: u32,
        mask_buffer: &mut [f32],
        device: &Device,
    ) -> Result<Tensor> {
        let log_probs = log_softmax(logits, 0)?;

        let timestamp_log_probs = log_probs.narrow(
            0,
            TIMESTAMP_BEGIN as usize,
            vocab_size as usize - TIMESTAMP_BEGIN as usize,
        )?;

        let text_log_probs = log_probs.narrow(0, 0, TIMESTAMP_BEGIN as usize)?;

        let timestamp_logprob = {
            let max_val = timestamp_log_probs.max(0)?;
            let shifted = timestamp_log_probs.broadcast_sub(&max_val)?;
            let exp_shifted = shifted.exp()?;
            let sum_exp = exp_shifted.sum(0)?;
            let log_sum = sum_exp.log()?;
            max_val.broadcast_add(&log_sum)?.to_scalar::<f32>()?
        };

        let max_text_token_logprob: f32 = text_log_probs.max(0)?.to_scalar::<f32>()?;

        tracing::debug!(
            timestamp_logprob,
            max_text_token_logprob,
            "timestamp vs text probability comparison"
        );

        if timestamp_logprob > max_text_token_logprob {
            for i in 0..vocab_size {
                mask_buffer[i as usize] = if i < TIMESTAMP_BEGIN {
                    f32::NEG_INFINITY
                } else {
                    0.0
                };
            }
            let mask_tensor = Tensor::new(mask_buffer as &[f32], device)?;
            return logits.broadcast_add(&mask_tensor).map_err(Into::into);
        }

        Ok(logits.clone())
    }

    fn decode_tokens(&self, tokens: &[u32]) -> Result<String> {
        self.tokenizer
            .decode(tokens, true)
            .map_err(|e| anyhow::anyhow!("Failed to decode tokens: {}", e))
    }
}

fn decode_audio_simple(audio_data: &[u8]) -> Result<Vec<f32>> {
    tracing::debug!(input_bytes = audio_data.len(), "decoding audio");
    let audio_vec = audio_data.to_vec();
    let cursor = Cursor::new(audio_vec);
    let mss = MediaSourceStream::new(Box::new(cursor), Default::default());

    let hint = Hint::new();

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .context("Failed to probe audio format - unsupported format")?;

    let mut format = probed.format;

    let track = format
        .default_track()
        .context("No default audio track found")?;

    let sample_rate = track
        .codec_params
        .sample_rate
        .context("No sample rate in audio track")?;

    let channels = if let Some(ch) = track.codec_params.channels {
        ch.count()
    } else if let Some(layout) = track.codec_params.channel_layout {
        match layout {
            Layout::Mono => 1,
            Layout::Stereo => 2,
            _ => 1,
        }
    } else {
        anyhow::bail!("No channel information in audio track (neither channels nor channel_layout)")
    };

    tracing::debug!(sample_rate, channels, "audio format detected");

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .context("Failed to create audio decoder - please ensure browser sends WAV format audio")?;

    let mut pcm_data = Vec::new();
    let mut packet_count = 0;

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(e) => return Err(e).context("Failed to read audio packet")?,
        };

        match decoder.decode(&packet) {
            Ok(decoded) => {
                pcm_data.extend(audio_buffer_to_f32(&decoded));
                packet_count += 1;
            }
            Err(symphonia::core::errors::Error::DecodeError(_)) => {
                continue;
            }
            Err(e) => return Err(e).context("Failed to decode audio packet")?,
        }
    }

    tracing::debug!(
        packet_count,
        pcm_samples = pcm_data.len(),
        "decoded audio packets"
    );

    let mono_data = if channels > 1 {
        tracing::debug!(channels, "converting to mono");
        convert_to_mono(&pcm_data, channels)
    } else {
        pcm_data
    };

    let resampled = if sample_rate != 16000 {
        tracing::debug!(from_rate = sample_rate, to_rate = 16000, "resampling audio");
        resample_audio(&mono_data, sample_rate, 16000)?
    } else {
        mono_data
    };

    // Log PCM statistics to diagnose quiet/corrupt audio
    if !resampled.is_empty() {
        let max_abs = resampled.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        let mean_abs = resampled.iter().map(|s| s.abs()).sum::<f32>() / resampled.len() as f32;
        let rms = (resampled.iter().map(|s| s * s).sum::<f32>() / resampled.len() as f32).sqrt();
        tracing::debug!(
            output_samples = resampled.len(),
            max_abs,
            mean_abs,
            rms,
            "audio decoding complete with PCM stats"
        );
    } else {
        tracing::debug!(output_samples = 0, "audio decoding complete (empty)");
    }

    Ok(resampled)
}

fn audio_buffer_to_f32(buffer: &AudioBufferRef) -> Vec<f32> {
    let num_channels = buffer.spec().channels.count();
    let num_frames = buffer.frames();
    let mut samples = Vec::with_capacity(num_frames * num_channels);

    match buffer {
        AudioBufferRef::F32(buf) => {
            for frame_idx in 0..num_frames {
                for ch_idx in 0..num_channels {
                    samples.push(buf.chan(ch_idx)[frame_idx]);
                }
            }
        }
        AudioBufferRef::S16(buf) => {
            for frame_idx in 0..num_frames {
                for ch_idx in 0..num_channels {
                    samples.push(buf.chan(ch_idx)[frame_idx] as f32 / 32768.0);
                }
            }
        }
        AudioBufferRef::S32(buf) => {
            for frame_idx in 0..num_frames {
                for ch_idx in 0..num_channels {
                    samples.push(buf.chan(ch_idx)[frame_idx] as f32 / 2147483648.0);
                }
            }
        }
        AudioBufferRef::F64(buf) => {
            for frame_idx in 0..num_frames {
                for ch_idx in 0..num_channels {
                    samples.push(buf.chan(ch_idx)[frame_idx] as f32);
                }
            }
        }
        _ => {
            tracing::warn!("Unsupported audio buffer format, returning silence");
        }
    }

    samples
}

fn convert_to_mono(data: &[f32], channels: usize) -> Vec<f32> {
    if channels == 1 {
        return data.to_vec();
    }

    let frames = data.len() / channels;
    let mut mono = Vec::with_capacity(frames);

    for frame_idx in 0..frames {
        let mut sum = 0.0;
        for ch in 0..channels {
            sum += data[frame_idx * channels + ch];
        }
        mono.push(sum / channels as f32);
    }

    mono
}

fn resample_audio(data: &[f32], from_rate: u32, to_rate: u32) -> Result<Vec<f32>> {
    use rubato::{
        Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
    };

    if from_rate == to_rate {
        return Ok(data.to_vec());
    }

    tracing::debug!(
        from_rate,
        to_rate,
        input_samples = data.len(),
        "resampling audio"
    );

    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };

    let mut resampler = SincFixedIn::<f32>::new(
        to_rate as f64 / from_rate as f64,
        2.0,
        params,
        data.len(),
        1,
    )?;

    let waves_in = vec![data.to_vec()];
    let waves_out = resampler.process(&waves_in, None)?;

    tracing::debug!(output_samples = waves_out[0].len(), "resampling complete");
    Ok(waves_out[0].clone())
}
