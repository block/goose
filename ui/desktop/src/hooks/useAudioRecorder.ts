import { useState, useRef, useCallback, useEffect } from 'react';
import { transcribeDictation, getDictationConfig, DictationProvider } from '../api';
import { useConfig } from '../components/ConfigContext';
import { errorMessage } from '../utils/conversionUtils';

interface UseAudioRecorderOptions {
  onTranscription: (text: string) => void;
  onError: (message: string) => void;
}

const MAX_AUDIO_SIZE_MB = 50;
const MAX_RECORDING_DURATION_SECONDS = 10 * 60;

// Convert audio blob to WAV format using Web Audio API
// Resamples to 16kHz mono (Whisper's native format) to reduce payload size
async function convertToWav(audioBlob: Blob): Promise<Blob> {
  const arrayBuffer = await audioBlob.arrayBuffer();
  const audioContext = new AudioContext();
  const audioBuffer = await audioContext.decodeAudioData(arrayBuffer);

  // Resample to 16kHz (Whisper's native sample rate)
  const targetSampleRate = 16000;
  const offlineContext = new OfflineAudioContext(
    1, // mono
    Math.ceil(audioBuffer.duration * targetSampleRate),
    targetSampleRate
  );

  // Create a buffer source
  const source = offlineContext.createBufferSource();
  source.buffer = audioBuffer;
  source.connect(offlineContext.destination);
  source.start(0);

  // Render the resampled audio
  const resampledBuffer = await offlineContext.startRendering();

  // Extract mono audio data
  const audioData = new Float32Array(resampledBuffer.length);
  resampledBuffer.copyFromChannel(audioData, 0);

  // Create WAV file at 16kHz
  const wavBuffer = encodeWav(audioData, targetSampleRate, 1);
  return new Blob([wavBuffer], { type: 'audio/wav' });
}

// Encode PCM data as WAV file
function encodeWav(samples: Float32Array, sampleRate: number, numChannels: number): ArrayBuffer {
  const bytesPerSample = 2; // 16-bit
  const blockAlign = numChannels * bytesPerSample;

  const buffer = new ArrayBuffer(44 + samples.length * bytesPerSample);
  const view = new DataView(buffer);

  // WAV header
  const writeString = (offset: number, string: string) => {
    for (let i = 0; i < string.length; i++) {
      view.setUint8(offset + i, string.charCodeAt(i));
    }
  };

  writeString(0, 'RIFF');
  view.setUint32(4, 36 + samples.length * bytesPerSample, true);
  writeString(8, 'WAVE');
  writeString(12, 'fmt ');
  view.setUint32(16, 16, true); // fmt chunk size
  view.setUint16(20, 1, true); // PCM format
  view.setUint16(22, numChannels, true);
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, sampleRate * blockAlign, true); // byte rate
  view.setUint16(32, blockAlign, true);
  view.setUint16(34, bytesPerSample * 8, true); // bits per sample
  writeString(36, 'data');
  view.setUint32(40, samples.length * bytesPerSample, true);

  // Write PCM data
  let offset = 44;
  for (let i = 0; i < samples.length; i++) {
    const sample = Math.max(-1, Math.min(1, samples[i]));
    const intSample = sample < 0 ? sample * 0x8000 : sample * 0x7fff;
    view.setInt16(offset, intSample, true);
    offset += 2;
  }

  return buffer;
}

export const useAudioRecorder = ({ onTranscription, onError }: UseAudioRecorderOptions) => {
  const [isRecording, setIsRecording] = useState(false);
  const [isTranscribing, setIsTranscribing] = useState(false);
  const [recordingDuration, setRecordingDuration] = useState(0);
  const [estimatedSize, setEstimatedSize] = useState(0);
  const [isEnabled, setIsEnabled] = useState(false);
  const [provider, setProvider] = useState<DictationProvider | null>(null);

  const { read } = useConfig();

  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const audioChunksRef = useRef<Blob[]>([]);
  const streamRef = useRef<MediaStream | null>(null);
  const durationIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    const checkProviderConfig = async () => {
      try {
        const providerValue = await read('voice_dictation_provider', false);
        const preferredProvider = (providerValue as DictationProvider) || null;

        if (!preferredProvider) {
          setIsEnabled(false);
          setProvider(null);
          return;
        }

        const audioConfigResponse = await getDictationConfig();
        const providerStatus = audioConfigResponse.data?.[preferredProvider];

        setIsEnabled(!!providerStatus?.configured);
        setProvider(preferredProvider);
      } catch (error) {
        console.error('Error checking audio config:', error);
        setIsEnabled(false);
        setProvider(null);
      }
    };

    checkProviderConfig();
  }, [read]);

  const stopRecording = useCallback(() => {
    setIsRecording(false);

    if (mediaRecorderRef.current && mediaRecorderRef.current.state !== 'inactive') {
      mediaRecorderRef.current.stop();
    }

    if (durationIntervalRef.current) {
      clearInterval(durationIntervalRef.current);
      durationIntervalRef.current = null;
    }

    if (streamRef.current) {
      streamRef.current.getTracks().forEach((track) => track.stop());
      streamRef.current = null;
    }
  }, []);

  useEffect(() => {
    return () => {
      if (durationIntervalRef.current) {
        clearInterval(durationIntervalRef.current);
      }
      if (streamRef.current) {
        streamRef.current.getTracks().forEach((track) => track.stop());
      }
    };
  }, []);

  const transcribeAudio = useCallback(
    async (audioBlob: Blob) => {
      if (!provider) {
        onError('No transcription provider configured');
        return;
      }

      setIsTranscribing(true);

      try {
        // Convert to WAV format for local transcription (works with all providers)
        const wavBlob = await convertToWav(audioBlob);

        const sizeMB = wavBlob.size / (1024 * 1024);
        if (sizeMB > MAX_AUDIO_SIZE_MB) {
          onError(
            `Audio file too large (${sizeMB.toFixed(1)}MB). Maximum size is ${MAX_AUDIO_SIZE_MB}MB.`
          );
          return;
        }

        const reader = new FileReader();
        const base64Audio = await new Promise<string>((resolve, reject) => {
          reader.onloadend = () => {
            const base64 = reader.result as string;
            resolve(base64.split(',')[1]);
          };
          reader.onerror = reject;
          reader.readAsDataURL(wavBlob);
        });

        const result = await transcribeDictation({
          body: {
            audio: base64Audio,
            mime_type: 'audio/wav',
            provider: provider,
          },
          throwOnError: true,
        });

        if (result.data?.text) {
          onTranscription(result.data.text);
        }
      } catch (error) {
        onError(errorMessage(error));
      } finally {
        setIsTranscribing(false);
        setRecordingDuration(0);
        setEstimatedSize(0);
      }
    },
    [provider, onTranscription, onError]
  );

  const startRecording = useCallback(async () => {
    if (!isEnabled) {
      onError('Voice dictation is not enabled');
      return;
    }

    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        audio: {
          echoCancellation: true,
          noiseSuppression: true,
          autoGainControl: true,
        },
      });
      streamRef.current = stream;

      // Record in whatever format the browser supports - we'll convert to WAV before sending
      const supportedTypes = ['audio/webm;codecs=opus', 'audio/webm', 'audio/mp4', 'audio/ogg'];
      const mimeType = supportedTypes.find((type) => MediaRecorder.isTypeSupported(type)) || '';

      const mediaRecorder = new MediaRecorder(stream, mimeType ? { mimeType } : {});
      mediaRecorderRef.current = mediaRecorder;
      audioChunksRef.current = [];

      const startTime = Date.now();
      durationIntervalRef.current = setInterval(() => {
        const elapsed = (Date.now() - startTime) / 1000;
        setRecordingDuration(elapsed);

        // Estimate final WAV size: 48kHz * 16-bit * mono ≈ 96 KB/s ≈ 0.09375 MB/s
        // (Recorded in compressed format but converted to WAV before sending)
        const estimatedSizeMB = (elapsed * 96) / 1024;
        setEstimatedSize(estimatedSizeMB);

        if (elapsed >= MAX_RECORDING_DURATION_SECONDS) {
          stopRecording();
          onError(
            `Maximum recording duration (${MAX_RECORDING_DURATION_SECONDS / 60} minutes) reached`
          );
        }
      }, 100);

      mediaRecorder.ondataavailable = (event) => {
        if (event.data.size > 0) {
          audioChunksRef.current.push(event.data);
        }
      };

      mediaRecorder.onstop = async () => {
        const audioBlob = new Blob(audioChunksRef.current, { type: mimeType || 'audio/webm' });

        if (audioBlob.size === 0) {
          onError('No audio data was recorded. Please check your microphone.');
          return;
        }

        await transcribeAudio(audioBlob);
      };

      mediaRecorder.onerror = (_event) => {
        onError('Recording failed');
      };

      mediaRecorder.start(100);
      setIsRecording(true);
    } catch (error) {
      stopRecording();
      onError(errorMessage(error));
    }
  }, [isEnabled, onError, transcribeAudio, stopRecording]);

  return {
    isEnabled,
    dictationProvider: provider,
    isRecording,
    isTranscribing,
    recordingDuration,
    estimatedSize,
    startRecording,
    stopRecording,
  };
};
