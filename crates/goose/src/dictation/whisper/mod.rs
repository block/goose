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
use candle_transformers::models::whisper::{self as m, audio as whisper_audio, Config, N_FRAMES};
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

mod audio;
mod models;
mod transcription;
pub use audio::*;
pub use models::{available_models, get_model, recommend_model, WhisperModel};
pub use transcription::*;

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
            80 => include_bytes!("../whisper_data/melfilters.bytes").as_slice(),
            128 => include_bytes!("../whisper_data/melfilters128.bytes").as_slice(),
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

        let raw_result = self.decode_tokens(&all_text_tokens)?;
        let result = deduplicate_text(&raw_result);
        if result != raw_result {
            tracing::debug!(
                before_len = raw_result.len(),
                after_len = result.len(),
                "text-level deduplication removed repeated phrases"
            );
        }
        tracing::debug!(result_len = result.len(), "transcription complete");
        Ok(result)
    }

    fn prepare_audio_input(&self, audio_data: &[u8]) -> Result<(Tensor, usize)> {
        tracing::debug!(audio_bytes = audio_data.len(), "decoding audio to PCM");
        let pcm_data = decode_audio_simple(audio_data)?;
        let pcm_samples = pcm_data.len();
        tracing::debug!(pcm_samples, "converting PCM to mel spectrogram");

        let actual_content_frames = pcm_samples / 160;

        let mel = whisper_audio::pcm_to_mel(&self.config, &pcm_data, &self.mel_filters);
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

        if tracing::enabled!(tracing::Level::DEBUG) {
            let mel_flat = mel_segment.flatten_all()?;
            let mel_mean: f32 = mel_flat.mean(0)?.to_scalar()?;
            let mel_max: f32 = mel_flat.max(0)?.to_scalar()?;
            let mel_min: f32 = mel_flat.min(0)?.to_scalar()?;
            tracing::debug!(mel_mean, mel_max, mel_min, "mel segment statistics");
        }

        self.model.decoder.reset_kv_cache();
        let audio_features = self.model.encoder.forward(&mel_segment, true)?;

        if tracing::enabled!(tracing::Level::DEBUG) {
            let af_flat = audio_features.flatten_all()?;
            let af_mean: f32 = af_flat.mean(0)?.to_scalar()?;
            let af_max: f32 = af_flat.max(0)?.to_scalar()?;
            let af_min: f32 = af_flat.min(0)?.to_scalar()?;
            tracing::debug!(af_mean, af_max, af_min, "audio features statistics");
        }
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

            if let Some(truncate_at) = self.detect_repetition(&tokens) {
                tracing::debug!(
                    truncate_at,
                    tokens_before = tokens.len(),
                    "repetition detected, truncating"
                );
                tokens.truncate(truncate_at);
                break;
            }
        }

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
            let timestamp_last = timestamp_tokens.last().unwrap() + 1;

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

    fn detect_repetition(&self, tokens: &[u32]) -> Option<usize> {
        detect_repetition_impl(tokens, SAMPLE_BEGIN, TIMESTAMP_BEGIN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use test_case::test_case;

    const TS: u32 = 50364; // A timestamp token for tests

    // detect_repetition_impl tests
    // sample_begin=3 means tokens[0..3] are SOT, language, transcribe
    // timestamp_begin=50364 means tokens >= 50364 are timestamps

    #[test_case(&[0, 1, 2, 10, 10, 10], Some(4) ; "single token repeated 3x")]
    #[test_case(&[0, 1, 2, 10, 10], None ; "single token repeated 2x not enough")]
    #[test_case(&[0, 1, 2, 10, 20, 30, 10, 20, 30], None ; "3-token pattern repeated 2x not enough")]
    #[test_case(&[0, 1, 2, 10, 20, 30, 40, 50, 10, 20, 30, 40, 50], Some(8) ; "5-token pattern repeated 2x")]
    #[test_case(&[0, 1, 2, 10, 20, 10, 20, 10, 20], Some(5) ; "2-token pattern repeated 3x")]
    #[test_case(&[0, 1, 2, 10, 20, 30, 40, 10, 20, 30, 40], None ; "4-token pattern repeated 2x not enough")]
    #[test_case(&[0, 1, 2, 10, 99, 20, 10, 99, 20], None ; "non-adjacent same tokens no trigger")]
    fn test_detect_repetition_no_timestamps(tokens: &[u32], expected: Option<usize>) {
        assert_eq!(detect_repetition_impl(tokens, 3, 50364), expected);
    }

    #[test_case(
        &[0, 1, 2, TS, 10, 20, 30, TS+1, TS+2, 10, 20, 30, TS+3],
        None ;
        "phrase 3 tokens with timestamps 2x not enough"
    )]
    #[test_case(
        &[0, 1, 2, TS, 10, 10, 10, TS+1],
        Some(5) ;
        "single token 3x with surrounding timestamps"
    )]
    #[test_case(
        &[0, 1, 2, TS, 10, 20, TS+1, TS+2, 10, 20, TS+3, TS+4, 10, 20, TS+5],
        Some(8) ;
        "2-token pattern 3x with timestamps interleaved"
    )]
    fn test_detect_repetition_with_timestamps(tokens: &[u32], expected: Option<usize>) {
        assert_eq!(detect_repetition_impl(tokens, 3, 50364), expected);
    }

    // Real example from logs: phrase repeated 3x with timestamps
    #[test]
    fn test_detect_repetition_real_example() {
        let tokens: Vec<u32> = vec![
            0, 1, 2, // SOT, lang, transcribe (indices 0-2)
            50364, 286, 500, 380, 458, 983, 309, 311, 18617, 2564, 13, // first phrase + ts
            50450, 50475, 286, 500, 380, 458, 983, 309, 311, 18617, 2564,
            13, // second phrase + ts
            50550, 50551, 286, 500, 380, 458, 983, 309, 311, 18617, 2564,
            13, // third phrase + ts
        ];
        // Text tokens are: 286, 500, 380, 458, 983, 309, 311, 18617, 2564, 13 (10 tokens)
        // Repeated 3 times, should trigger
        let result = detect_repetition_impl(&tokens, 3, 50364);
        assert!(result.is_some(), "Should detect repetition in real example");
    }

    #[test]
    fn test_no_false_positive_on_dog_sentences() {
        // "I saw a dog. I liked the dog. I gave the dog food."
        // dog=100, other tokens are different
        let tokens: Vec<u32> = vec![
            0, 1, 2, 10, 11, 12, 100, 13, // I saw a dog.
            20, 21, 22, 100, 23, // I liked the dog.
            30, 31, 32, 100, 33, // I gave the dog food.
        ];
        assert_eq!(detect_repetition_impl(&tokens, 3, 50364), None);
    }

    // deduplicate_text tests
    #[test_case("", "" ; "empty")]
    #[test_case("   ", "" ; "whitespace")]
    #[test_case("I went to the store. Then I came home.", "I went to the store. Then I came home." ; "no repetition")]
    #[test_case(
        "I could build a record mode. I could build a record mode. I could build a record mode.",
        "I could build a record mode." ;
        "single sentence 3x"
    )]
    #[test_case(
        "Yeah I was thinking about that. Yeah I was thinking about that.",
        "Yeah I was thinking about that." ;
        "single sentence 2x"
    )]
    #[test_case(
        "Who works for Flux? Who works for Flux? Who works for Flux?",
        "Who works for Flux?" ;
        "question marks"
    )]
    #[test_case("Stop! Stop! Stop!", "Stop!" ; "exclamation marks")]
    #[test_case("hello hello hello hello", "hello hello hello hello" ; "no sentence boundaries")]
    fn test_deduplicate_text(input: &str, expected: &str) {
        assert_eq!(deduplicate_text(input), expected);
    }

    #[test_case("Hello. World. Foo.", vec!["Hello. ", "World. ", "Foo."] ; "basic")]
    #[test_case("Hello. World", vec!["Hello. ", "World"] ; "trailing fragment")]
    #[test_case("Really? Yes! Ok.", vec!["Really? ", "Yes! ", "Ok."] ; "mixed punctuation")]
    fn test_split_into_sentences(input: &str, expected: Vec<&str>) {
        assert_eq!(split_into_sentences(input), expected);
    }
}
