//! Local Whisper transcription using Candle
//!
//! This module provides local audio transcription using OpenAI's Whisper model
//! via the Candle ML framework. It supports loading GGUF quantized models for
//! efficient CPU inference.

use anyhow::{Context, Result};
use candle_core::{Device, IndexOp, Tensor};
use candle_nn::ops::log_softmax;
use candle_transformers::models::whisper::{self as m, audio, Config, N_FRAMES};
use std::io::Cursor;
use std::path::Path;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use tokenizers::Tokenizer;

/// Whisper transcription engine
pub struct WhisperTranscriber {
    model: m::quantized_model::Whisper,
    config: Config,
    device: Device,
    mel_filters: Vec<f32>,
    tokenizer: Option<Tokenizer>,
    // Token IDs for timestamp processing
    eot_token: u32,
    no_timestamps_token: u32,
    language_token: Option<u32>,
    max_initial_timestamp_index: Option<u32>,
}

impl WhisperTranscriber {
    /// Create a new transcriber by loading a model from a file path
    ///
    /// # Arguments
    /// * `model_path` - Path to the GGUF model file (e.g., "whisper-tiny-q80.gguf")
    ///
    /// # Example
    /// ```no_run
    /// use goose::whisper::WhisperTranscriber;
    ///
    /// let transcriber = WhisperTranscriber::new("~/.goose/whisper-models/whisper-tiny-q80.gguf")?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn new<P: AsRef<Path>>(model_path: P) -> Result<Self> {
        Self::new_with_tokenizer(model_path, None)
    }

    /// Create a new transcriber with an optional bundled tokenizer
    ///
    /// If the tokenizer doesn't exist at the expected location and bundled_tokenizer is provided,
    /// it will be written to disk for future use.
    ///
    /// # Arguments
    /// * `model_path` - Path to the GGUF model file (e.g., "whisper-tiny-q80.gguf")
    /// * `bundled_tokenizer` - Optional tokenizer JSON content to use if file doesn't exist
    pub fn new_with_tokenizer<P: AsRef<Path>>(
        model_path: P,
        bundled_tokenizer: Option<&str>,
    ) -> Result<Self> {
        // Try to use GPU acceleration: CUDA > Metal > CPU
        // Note: GPU features must be enabled at build time:
        //   cargo build --features cuda    (for NVIDIA GPUs)
        //   cargo build --features metal   (for Apple Silicon/AMD on macOS)
        let device = if let Ok(device) = Device::new_cuda(0) {
            tracing::info!("Using CUDA GPU acceleration");
            device
        } else if let Ok(device) = Device::new_metal(0) {
            tracing::info!("Using Metal GPU acceleration");
            device
        } else {
            tracing::info!("GPU not available, using CPU");
            Device::Cpu
        };

        let model_path_ref = model_path.as_ref();

        tracing::debug!("whisper.rs: Checking model path: {}", model_path_ref.display());
        tracing::debug!("whisper.rs: Path exists: {}", model_path_ref.exists());
        tracing::debug!("whisper.rs: Path is absolute: {}", model_path_ref.is_absolute());
        tracing::debug!("whisper.rs: Path as_os_str: {:?}", model_path_ref.as_os_str());

        if !model_path_ref.exists() {
            anyhow::bail!("Model file not found: {}", model_path_ref.display());
        }

        tracing::info!(
            "Loading Whisper model from: {}",
            model_path_ref.display()
        );

        // Detect model size from filename to use appropriate config
        let filename = model_path_ref
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        // Common suppress tokens for all models
        let suppress_tokens = vec![
            1, 2, 7, 8, 9, 10, 14, 25, 26, 27, 28, 29, 31, 58, 59, 60, 61, 62, 63, 90, 91,
            92, 93, 359, 503, 522, 542, 873, 893, 902, 918, 922, 931, 1350, 1853, 1982,
            2460, 2627, 3246, 3253, 3268, 3536, 3846, 3961, 4183, 4667, 6585, 6647, 7273,
            9061, 9383, 10428, 10929, 11938, 12033, 12331, 12562, 13793, 14157, 14635,
            15265, 15618, 16553, 16604, 18362, 18956, 20075, 21675, 22520, 26130, 26161,
            26435, 28279, 29464, 31650, 32302, 32470, 36865, 42863, 47425, 49870, 50254,
            50258, 50360, 50362,
        ];

        // Load config based on model size
        let config = if filename.contains("tiny") {
            Config {
                num_mel_bins: 80,
                max_source_positions: 1500,
                d_model: 384,
                encoder_attention_heads: 6,
                encoder_layers: 4,
                decoder_attention_heads: 6,
                decoder_layers: 4,
                vocab_size: 51865,
                suppress_tokens: suppress_tokens.clone(),
                max_target_positions: 448,
            }
        } else if filename.contains("base") {
            Config {
                num_mel_bins: 80,
                max_source_positions: 1500,
                d_model: 512,
                encoder_attention_heads: 8,
                encoder_layers: 6,
                decoder_attention_heads: 8,
                decoder_layers: 6,
                vocab_size: 51865,
                suppress_tokens: suppress_tokens.clone(),
                max_target_positions: 448,
            }
        } else if filename.contains("medium") {
            Config {
                num_mel_bins: 80,
                max_source_positions: 1500,
                d_model: 1024,
                encoder_attention_heads: 16,
                encoder_layers: 24,
                decoder_attention_heads: 16,
                decoder_layers: 24,
                vocab_size: 51865,
                suppress_tokens: suppress_tokens.clone(),
                max_target_positions: 448,
            }
        } else {
            // Small model (default)
            Config {
                num_mel_bins: 80,
                max_source_positions: 1500,
                d_model: 768,
                encoder_attention_heads: 12,
                encoder_layers: 12,
                decoder_attention_heads: 12,
                decoder_layers: 12,
                vocab_size: 51865,
                suppress_tokens,
                max_target_positions: 448,
            }
        };

        // Load mel filterbank
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

        // Load the quantized model from GGUF
        let vb = candle_transformers::quantized_var_builder::VarBuilder::from_gguf(
            model_path_ref,
            &device,
        )?;
        let model = m::quantized_model::Whisper::load(&vb, config.clone())?;

        tracing::info!("Whisper model loaded successfully");

        // Try to load tokenizer from the same directory or download it
        let tokenizer = Self::load_tokenizer(model_path_ref, bundled_tokenizer).ok();
        if tokenizer.is_some() {
            tracing::info!("Tokenizer loaded successfully");
        } else {
            tracing::warn!("Could not load tokenizer, token decoding will be limited");
        }

        Ok(Self {
            model,
            config,
            device,
            mel_filters,
            tokenizer,
            eot_token: 50257,
            no_timestamps_token: 50363,
            language_token: Some(50259), // English token
            max_initial_timestamp_index: Some(50), // Limit initial timestamp to first 1 second
        })
    }

    /// Load or download the Whisper tokenizer
    fn load_tokenizer(model_dir: &Path, bundled_tokenizer: Option<&str>) -> Result<Tokenizer> {
        // Try to find tokenizer in the same directory as the model
        let tokenizer_path = model_dir
            .parent()
            .unwrap_or(model_dir)
            .join("tokenizer.json");

        if tokenizer_path.exists() {
            tracing::info!("Loading tokenizer from {}", tokenizer_path.display());
            return Tokenizer::from_file(tokenizer_path)
                .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e));
        }

        // If we have a bundled tokenizer and the file doesn't exist, write it
        if let Some(tokenizer_json) = bundled_tokenizer {
            tracing::info!(
                "Writing bundled tokenizer to {}",
                tokenizer_path.display()
            );

            // Create parent directory if it doesn't exist
            if let Some(parent) = tokenizer_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // Write the bundled tokenizer
            std::fs::write(&tokenizer_path, tokenizer_json)?;

            tracing::info!("Bundled tokenizer written successfully");
            return Tokenizer::from_file(tokenizer_path)
                .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e));
        }

        // Try to download from Hugging Face as last resort
        tracing::info!("Downloading tokenizer from Hugging Face...");
        let api = hf_hub::api::sync::Api::new()?;
        let repo = api.model("openai/whisper-small".to_string());
        let tokenizer_file = repo.get("tokenizer.json")?;

        Tokenizer::from_file(tokenizer_file)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))
    }

    /// Transcribe audio data to text
    ///
    /// # Arguments
    /// * `audio_data` - Raw audio bytes in a supported format (WAV, MP3, etc.)
    ///
    /// # Returns
    /// The transcribed text
    ///
    /// # Example
    /// ```no_run
    /// # use goose::whisper::WhisperTranscriber;
    /// # let mut transcriber = WhisperTranscriber::new("~/.goose/whisper-models/ggml-small.bin").unwrap();
    /// let audio_bytes = std::fs::read("audio.wav")?;
    /// let text = transcriber.transcribe(&audio_bytes)?;
    /// println!("Transcription: {}", text);
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn transcribe(&mut self, audio_data: &[u8]) -> Result<String> {
        tracing::info!("Transcribing {} bytes of audio data", audio_data.len());

        // Debug: Save audio if requested via env var
        if std::env::var("GOOSE_SAVE_AUDIO").is_ok() {
            let save_path = "/tmp/whisper_audio.wav";
            if let Err(e) = std::fs::write(save_path, audio_data) {
                tracing::warn!("Failed to save audio to {}: {}", save_path, e);
            } else {
                tracing::info!("Saved audio to {} ({} bytes)", save_path, audio_data.len());
            }
        }

        // Decode audio to PCM samples at 16kHz
        let pcm_data = decode_audio_simple(audio_data)?;
        tracing::info!(
            "PCM decoded: {} samples = {:.2}s at 16kHz",
            pcm_data.len(),
            pcm_data.len() as f32 / 16000.0
        );

        // Debug: Save resampled PCM as WAV if requested
        if std::env::var("GOOSE_SAVE_AUDIO").is_ok() {
            if let Err(e) = save_wav_16khz("/tmp/whisper_audio_16k.wav", &pcm_data) {
                tracing::warn!("Failed to save 16kHz WAV: {}", e);
            } else {
                tracing::info!("Saved 16kHz WAV to /tmp/whisper_audio_16k.wav for comparison");
            }
        }

        // Convert to mel spectrogram
        tracing::info!("Converting PCM to mel spectrogram...");
        let mel = audio::pcm_to_mel(&self.config, &pcm_data, &self.mel_filters);
        let mel_len = mel.len();
        tracing::info!(
            "Mel data length: {} floats = {} frames x {} mel_bins",
            mel_len,
            mel_len / self.config.num_mel_bins,
            self.config.num_mel_bins
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

        let (_, _, content_frames) = mel_tensor.dims3()?;
        tracing::info!(
            "Mel tensor shape: {:?}, content_frames: {}",
            mel_tensor.dims(),
            content_frames
        );
        tracing::info!(
            "Expected frames from PCM: {} (pcm_len / hop_length)",
            pcm_data.len() / 160
        );

        // Special token IDs
        let sot_token_id = 50258_u32;
        let english_token_id = 50259_u32;
        let transcribe_token_id = 50359_u32;
        let no_timestamps_token_id = 50363_u32;
        let eot_token_id = 50257_u32;

        // Process mel in segments - encoder can handle up to N_FRAMES (3000) which downsamples to 1500
        let num_segments = (content_frames + N_FRAMES - 1) / N_FRAMES;
        tracing::info!(
            "Processing mel in {} segment(s) (N_FRAMES={}, content_frames={})",
            num_segments,
            N_FRAMES,
            content_frames
        );

        let mut all_text_tokens = Vec::new();
        let mut seek = 0;
        let mut segment_num = 0;

        while seek < content_frames {
            segment_num += 1;
            let segment_size = usize::min(content_frames - seek, N_FRAMES);
            let time_offset = (seek * 160) as f32 / 16000.0; // HOP_LENGTH = 160
            let segment_duration = (segment_size * 160) as f32 / 16000.0;

            tracing::info!(
                "=== SEGMENT {}/{} ===",
                segment_num,
                num_segments
            );
            tracing::info!(
                "  Seek: {}, Size: {}, Time: {:.1}s - {:.1}s",
                seek,
                segment_size,
                time_offset,
                time_offset + segment_duration
            );

            let mel_segment = mel_tensor.narrow(2, seek, segment_size)?;
            tracing::info!("  Mel segment shape: {:?}", mel_segment.dims());

            // Reset decoder KV cache before processing new segment
            self.model.decoder.reset_kv_cache();

            // Encode audio segment
            let audio_features = self.model.encoder.forward(&mel_segment, true)?;
            tracing::info!("  Encoder output shape: {:?}", audio_features.dims());

            // Create suppress tokens tensor
            // Per OpenAI Whisper: suppress the no_timestamps token when in timestamps mode
            // https://github.com/openai/whisper/blob/e8622f9afc4eba139bf796c210f5c01081000472/whisper/decoding.py#L452
            let suppress_tokens = {
                let mut suppress = vec![0f32; self.config.vocab_size];
                for &token_id in &self.config.suppress_tokens {
                    if (token_id as usize) < suppress.len() {
                        suppress[token_id as usize] = f32::NEG_INFINITY;
                    }
                }
                // In timestamps mode, also suppress the no_timestamps token
                suppress[no_timestamps_token_id as usize] = f32::NEG_INFINITY;

                Tensor::from_vec(suppress, self.config.vocab_size, &self.device)?
            };

            // Initialize token sequence for this segment
            // Match candle example: SOT + language + task tokens
            // NOTE: We do NOT add no_timestamps token, which enables timestamps mode
            let mut tokens = vec![
                sot_token_id,
                english_token_id,
                transcribe_token_id,
            ];

            let sample_len = self.config.max_target_positions / 2;

            // Decode loop for this segment
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

                // Apply timestamp rules in timestamps mode
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

                if next_token == eot_token_id || tokens.len() > self.config.max_target_positions {
                    break;
                }
            }

            // Extract text tokens (skip the 3 special tokens at start: SOT + English + Transcribe)
            // Also filter out EOT and timestamp tokens (>= 50364)
            let timestamp_begin = 50364_u32; // First timestamp token
            let segment_text_tokens: Vec<u32> = tokens[3..]
                .iter()
                .filter(|&&t| t != eot_token_id && t < timestamp_begin)
                .copied()
                .collect();

            tracing::info!(
                "  Segment produced {} text tokens (total tokens: {})",
                segment_text_tokens.len(),
                tokens.len()
            );

            // Decode this segment's tokens to see what we got
            if let Some(tokenizer) = &self.tokenizer {
                if let Ok(segment_text) = tokenizer.decode(&segment_text_tokens, true) {
                    tracing::info!("  Segment text: {:?}", segment_text);
                }
            }

            all_text_tokens.extend(segment_text_tokens);
            seek += segment_size;
        }

        tracing::info!(
            "=== COMPLETE: Processed {} segments, {} total text tokens ===",
            num_segments,
            all_text_tokens.len()
        );

        // Decode all tokens to text
        let text = self.decode_tokens(&all_text_tokens)?;
        tracing::info!("Transcription complete: {} tokens", all_text_tokens.len());

        Ok(text)
    }

    /// Apply timestamp rules to logits during decoding
    /// This implements the timestamp constraints from OpenAI's Whisper decoder
    fn apply_timestamp_rules(&self, input_logits: &Tensor, tokens: &[u32]) -> Result<Tensor> {
        let device = input_logits.device().clone();
        let timestamp_begin = self.no_timestamps_token + 1;
        let vocab_size = self.model.config.vocab_size as u32;

        // ========== SETUP: Extract sampled tokens for analysis ==========
        let sample_begin = if self.language_token.is_some() { 3 } else { 2 };
        let sampled_tokens = if tokens.len() > sample_begin {
            &tokens[sample_begin..]
        } else {
            &[]
        };

        let mut masks = Vec::new();
        // Pre-allocate reusable mask buffer to avoid repeated allocations
        let mut mask_buffer = vec![0.0f32; vocab_size as usize];

        // ========== RULE 1: Timestamp pairing constraints ==========
        // Timestamps must come in pairs, except directly before EOT
        if !sampled_tokens.is_empty() {
            let last_was_timestamp = sampled_tokens
                .last()
                .map(|&t| t >= timestamp_begin)
                .unwrap_or(false);

            let penultimate_was_timestamp = if sampled_tokens.len() >= 2 {
                sampled_tokens[sampled_tokens.len() - 2] >= timestamp_begin
            } else {
                false
            };

            if last_was_timestamp {
                if penultimate_was_timestamp {
                    // Has to be non-timestamp - suppress timestamp tokens
                    for i in 0..vocab_size {
                        mask_buffer[i as usize] = if i >= timestamp_begin {
                            f32::NEG_INFINITY
                        } else {
                            0.0
                        };
                    }
                    masks.push(Tensor::new(mask_buffer.as_slice(), &device)?);
                } else {
                    // Cannot be normal text tokens - suppress everything before EOT
                    for i in 0..vocab_size {
                        mask_buffer[i as usize] = if i < self.eot_token {
                            f32::NEG_INFINITY
                        } else {
                            0.0
                        };
                    }
                    masks.push(Tensor::new(mask_buffer.as_slice(), &device)?);
                }
            }

            // ========== RULE 2: Non-decreasing timestamp constraint ==========
            // Timestamps shouldn't decrease; forbid timestamp tokens smaller than the last
            let timestamp_tokens: Vec<u32> = sampled_tokens
                .iter()
                .filter(|&&t| t >= timestamp_begin)
                .cloned()
                .collect();

            if !timestamp_tokens.is_empty() {
                let timestamp_last = if last_was_timestamp && !penultimate_was_timestamp {
                    *timestamp_tokens.last().unwrap()
                } else {
                    timestamp_tokens.last().unwrap() + 1
                };

                for i in 0..vocab_size {
                    mask_buffer[i as usize] = if i >= timestamp_begin && i < timestamp_last {
                        f32::NEG_INFINITY
                    } else {
                        0.0
                    };
                }
                masks.push(Tensor::new(mask_buffer.as_slice(), &device)?);
            }
        }

        // ========== RULE 3: Force initial timestamp ==========
        // At the beginning, suppress generating non-timestamp tokens
        if tokens.len() == sample_begin {
            for i in 0..vocab_size {
                mask_buffer[i as usize] = if i < timestamp_begin {
                    f32::NEG_INFINITY
                } else {
                    0.0
                };
            }
            masks.push(Tensor::new(mask_buffer.as_slice(), &device)?);

            // Apply the max_initial_timestamp constraint
            if let Some(max_initial_timestamp_index) = self.max_initial_timestamp_index {
                let last_allowed = timestamp_begin + max_initial_timestamp_index;
                if last_allowed < vocab_size {
                    for i in 0..vocab_size {
                        mask_buffer[i as usize] = if i > last_allowed {
                            f32::NEG_INFINITY
                        } else {
                            0.0
                        };
                    }
                    masks.push(Tensor::new(mask_buffer.as_slice(), &device)?);
                }
            }
        }

        // ========== APPLY MASKS: Apply all constraint masks ==========
        let mut logits = input_logits.clone();
        for mask in masks {
            logits = logits.broadcast_add(&mask)?;
        }

        // ========== RULE 4: Probability-based timestamp preference ==========
        // If sum of probability over timestamps is above any other token, sample timestamp
        let log_probs = log_softmax(&logits, 0)?;

        // Extract timestamp and text log probabilities
        let timestamp_log_probs = log_probs.narrow(
            0,
            timestamp_begin as usize,
            vocab_size as usize - timestamp_begin as usize,
        )?;

        let text_log_probs = log_probs.narrow(0, 0, timestamp_begin as usize)?;

        // Implement logsumexp for timestamp tokens (numerically stable)
        let timestamp_logprob = {
            let max_val = timestamp_log_probs.max(0)?;
            let shifted = timestamp_log_probs.broadcast_sub(&max_val)?;
            let exp_shifted = shifted.exp()?;
            let sum_exp = exp_shifted.sum(0)?;
            let log_sum = sum_exp.log()?;
            max_val.broadcast_add(&log_sum)?.to_scalar::<f32>()?
        };

        // Get max text token log probability
        let max_text_token_logprob: f32 = text_log_probs.max(0)?.to_scalar::<f32>()?;

        // Compare in log space
        if timestamp_logprob > max_text_token_logprob {
            // Only consider timestamp tokens
            for i in 0..vocab_size {
                mask_buffer[i as usize] = if i < timestamp_begin {
                    f32::NEG_INFINITY
                } else {
                    0.0
                };
            }
            let mask_tensor = Tensor::new(mask_buffer.as_slice(), &device)?;
            logits = logits.broadcast_add(&mask_tensor)?;
        }

        Ok(logits)
    }

    /// Decode token IDs to text
    fn decode_tokens(&self, tokens: &[u32]) -> Result<String> {
        if let Some(tokenizer) = &self.tokenizer {
            // Use the tokenizer to decode
            let text = tokenizer
                .decode(tokens, true)
                .map_err(|e| anyhow::anyhow!("Failed to decode tokens: {}", e))?;
            Ok(text)
        } else {
            // Fallback without tokenizer
            let text = format!(
                "[Transcription: {} tokens - tokenizer not available]",
                tokens.len()
            );
            Ok(text)
        }
    }
}

/// Decode audio bytes to PCM samples using Symphonia
/// Handles WAV, MP3, M4A, WebM/Opus, and other formats
fn decode_audio_simple(audio_data: &[u8]) -> Result<Vec<f32>> {
    tracing::info!("Decoding {} bytes of audio data", audio_data.len());

    // Log first few bytes to help diagnose format
    if audio_data.len() >= 16 {
        tracing::info!(
            "Audio header bytes: {:02x?}",
            &audio_data[..16.min(audio_data.len())]
        );
    }

    // Create a media source from the audio bytes
    let audio_vec = audio_data.to_vec();
    let cursor = Cursor::new(audio_vec);
    let mss = MediaSourceStream::new(Box::new(cursor), Default::default());

    // Create a hint to help probe the format
    let hint = Hint::new();

    // Probe the media source to detect format
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .context("Failed to probe audio format - unsupported format")?;

    tracing::info!("Audio format probed successfully");

    let mut format = probed.format;

    // Get the default audio track
    let track = format
        .default_track()
        .context("No default audio track found")?;

    tracing::info!(
        "Audio track codec: {:?}, params: {:?}",
        track.codec_params.codec,
        track.codec_params
    );

    // Get sample rate and channels from track
    let sample_rate = track
        .codec_params
        .sample_rate
        .context("No sample rate in audio track")?;

    // Try to get channel count from channels field, or fall back to channel_layout
    let channels = if let Some(ch) = track.codec_params.channels {
        ch.count()
    } else if let Some(layout) = track.codec_params.channel_layout {
        // Determine channel count from layout
        // For now, just handle the common cases. Most web audio is mono or stereo anyway.
        use symphonia::core::audio::Layout;
        match layout {
            Layout::Mono => 1,
            Layout::Stereo => 2,
            _ => {
                // Default to mono for unknown layouts
                tracing::warn!("Unknown audio layout {:?}, assuming mono", layout);
                1
            }
        }
    } else {
        anyhow::bail!("No channel information in audio track (neither channels nor channel_layout)")
    };

    tracing::info!("Audio format: {}Hz, {} channel(s)", sample_rate, channels);

    // Create decoder using default codecs (WAV, MP3, FLAC, Vorbis, etc.)
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| {
            tracing::error!(
                "Failed to create decoder for codec {:?}: {}. Hint: Browser should send WAV format.",
                track.codec_params.codec,
                e
            );
            e
        })
        .context("Failed to create audio decoder - please ensure browser sends WAV format audio")?;

    // Decode all packets into PCM samples
    let mut pcm_data = Vec::new();

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

        // Decode the packet
        match decoder.decode(&packet) {
            Ok(decoded) => {
                // Convert to f32 samples (interleaved if multi-channel)
                let samples = audio_buffer_to_f32(&decoded);
                pcm_data.extend_from_slice(&samples);
            }
            Err(symphonia::core::errors::Error::DecodeError(_)) => {
                // Skip decode errors (corrupted packets)
                continue;
            }
            Err(e) => return Err(e).context("Failed to decode audio packet")?,
        }
    }

    tracing::info!("Decoded {} samples from audio", pcm_data.len());

    // Convert to mono if needed
    let mono_data = if channels > 1 {
        convert_to_mono(&pcm_data, channels)
    } else {
        pcm_data
    };

    // Resample to 16kHz if needed (Whisper requirement)
    let resampled = if sample_rate != 16000 {
        tracing::info!("Resampling from {}Hz to 16000Hz", sample_rate);
        resample_audio(&mono_data, sample_rate, 16000)?
    } else {
        mono_data
    };

    tracing::info!("Final PCM data: {} samples at 16kHz", resampled.len());

    Ok(resampled)
}

/// Save PCM samples as a 16kHz mono WAV file
fn save_wav_16khz(path: &str, samples: &[f32]) -> Result<()> {
    use std::io::Write;

    let sample_rate = 16000u32;
    let num_channels = 1u16;
    let bits_per_sample = 16u16;
    let byte_rate = sample_rate * num_channels as u32 * bits_per_sample as u32 / 8;
    let block_align = num_channels * bits_per_sample / 8;
    let data_size = (samples.len() * 2) as u32; // 2 bytes per sample (i16)

    let mut file = std::fs::File::create(path)?;

    // RIFF header
    file.write_all(b"RIFF")?;
    file.write_all(&(36 + data_size).to_le_bytes())?;
    file.write_all(b"WAVE")?;

    // fmt chunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?; // chunk size
    file.write_all(&1u16.to_le_bytes())?; // audio format (PCM)
    file.write_all(&num_channels.to_le_bytes())?;
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&block_align.to_le_bytes())?;
    file.write_all(&bits_per_sample.to_le_bytes())?;

    // data chunk
    file.write_all(b"data")?;
    file.write_all(&data_size.to_le_bytes())?;

    // Convert f32 samples to i16 and write
    for &sample in samples {
        let sample_i16 = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
        file.write_all(&sample_i16.to_le_bytes())?;
    }

    Ok(())
}

/// Convert a Symphonia AudioBufferRef to f32 samples normalized to [-1, 1]
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
            // For other formats, try to handle gracefully
            tracing::warn!("Unsupported audio buffer format, returning silence");
        }
    }

    samples
}

/// Convert multi-channel audio to mono by averaging all channels
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

/// Resample audio from source sample rate to target sample rate
fn resample_audio(data: &[f32], from_rate: u32, to_rate: u32) -> Result<Vec<f32>> {
    use rubato::{
        Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
    };

    if from_rate == to_rate {
        return Ok(data.to_vec());
    }

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
        1, // mono
    )?;

    let waves_in = vec![data.to_vec()];
    let waves_out = resampler.process(&waves_in, None)?;

    Ok(waves_out[0].clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires model file to be present
    fn test_load_model() {
        let result = WhisperTranscriber::new("~/.goose/whisper-models/ggml-small.bin");
        assert!(result.is_ok());
    }
}
