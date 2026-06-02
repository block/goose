use super::*;

pub fn decode_audio_simple(audio_data: &[u8]) -> Result<Vec<f32>> {
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

    if tracing::enabled!(tracing::Level::DEBUG) {
        if !resampled.is_empty() {
            let max_abs = resampled.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
            let mean_abs = resampled.iter().map(|s| s.abs()).sum::<f32>() / resampled.len() as f32;
            let rms =
                (resampled.iter().map(|s| s * s).sum::<f32>() / resampled.len() as f32).sqrt();
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
    }

    Ok(resampled)
}

pub fn audio_buffer_to_f32(buffer: &AudioBufferRef) -> Vec<f32> {
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

pub fn convert_to_mono(data: &[f32], channels: usize) -> Vec<f32> {
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

pub fn resample_audio(data: &[f32], from_rate: u32, to_rate: u32) -> Result<Vec<f32>> {
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
