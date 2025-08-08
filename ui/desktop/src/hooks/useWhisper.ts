import { useState, useRef, useCallback, useEffect } from 'react';
import { useConfig } from '../components/ConfigContext';
import { getApiUrl } from '../config';
import { useDictationSettings } from './useDictationSettings';
import { safeJsonParse } from '../utils/jsonUtils';

interface UseWhisperOptions {
  onTranscription?: (text: string) => void;
  onError?: (error: Error) => void;
  onSizeWarning?: (sizeInMB: number) => void;
}

// Constants
const MAX_AUDIO_SIZE_MB = 25;
const MAX_RECORDING_DURATION_SECONDS = 600; // 10 minutes
const WARNING_SIZE_MB = 20; // Warn at 20MB

export const useWhisper = ({ onTranscription, onError, onSizeWarning }: UseWhisperOptions = {}) => {
  const [isRecording, setIsRecording] = useState(false);
  const [isTranscribing, setIsTranscribing] = useState(false);
  const [hasOpenAIKey, setHasOpenAIKey] = useState(false);
  const [canUseDictation, setCanUseDictation] = useState(false);
  const [audioContext, setAudioContext] = useState<AudioContext | null>(null);
  const [analyser, setAnalyser] = useState<AnalyserNode | null>(null);
  const [recordingDuration, setRecordingDuration] = useState(0);
  const [estimatedSize, setEstimatedSize] = useState(0);

  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const audioChunksRef = useRef<Blob[]>([]);
  const streamRef = useRef<MediaStream | null>(null);
  const recordingStartTimeRef = useRef<number | null>(null);
  const durationIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const currentSizeRef = useRef<number>(0);

  const { getProviders } = useConfig();
  const { settings: dictationSettings, hasElevenLabsKey } = useDictationSettings();

  // Check if OpenAI API key is configured (regardless of current provider)
  useEffect(() => {
    const checkOpenAIKey = async () => {
      try {
        // Get all configured providers
        const providers = await getProviders(false);

        // Find OpenAI provider
        const openAIProvider = providers.find((p) => p.name === 'openai');

        // Check if OpenAI is configured
        if (openAIProvider && openAIProvider.is_configured) {
          setHasOpenAIKey(true);
        } else {
          setHasOpenAIKey(false);
        }
      } catch (error) {
        console.error('Error checking OpenAI configuration:', error);
        setHasOpenAIKey(false);
      }
    };

    checkOpenAIKey();
  }, [getProviders]); // Re-check when providers change

  // Check if dictation can be used based on settings
  useEffect(() => {
    if (!dictationSettings) {
      setCanUseDictation(false);
      return;
    }

    if (!dictationSettings.enabled) {
      setCanUseDictation(false);
      return;
    }

    // Check provider availability
    switch (dictationSettings.provider) {
      case 'openai':
        setCanUseDictation(hasOpenAIKey);
        break;
      case 'elevenlabs':
        setCanUseDictation(hasElevenLabsKey);
        break;
      default:
        setCanUseDictation(false);
    }
  }, [dictationSettings, hasOpenAIKey, hasElevenLabsKey]);

  const transcribeAudio = useCallback(
    async (audioBlob: Blob) => {
      if (!dictationSettings) {
        onError?.(new Error('Dictation settings not loaded'));
        return;
      }

      setIsTranscribing(true);

      try {
        // Check final size
        const sizeMB = audioBlob.size / (1024 * 1024);
        if (sizeMB > MAX_AUDIO_SIZE_MB) {
          throw new Error(
            `Audio file too large (${sizeMB.toFixed(1)}MB). Maximum size is ${MAX_AUDIO_SIZE_MB}MB.`
          );
        }

        // Convert blob to base64 for easier transport
        const reader = new FileReader();
        const base64Audio = await new Promise<string>((resolve, reject) => {
          reader.onloadend = () => {
            const base64 = reader.result as string;
            resolve(base64.split(',')[1]); // Remove data:audio/webm;base64, prefix
          };
          reader.onerror = reject;
          reader.readAsDataURL(audioBlob);
        });

        let endpoint = '';
        let headers: Record<string, string> = {
          'Content-Type': 'application/json',
          'X-Secret-Key': await window.electron.getSecretKey(),
        };
        let body: Record<string, string> = {
          audio: base64Audio,
          mime_type: 'audio/webm',
        };

        // Choose endpoint based on provider
        switch (dictationSettings.provider) {
          case 'openai':
            endpoint = '/audio/transcribe';
            break;
          case 'elevenlabs':
            endpoint = '/audio/transcribe/elevenlabs';
            break;
          default:
            throw new Error(`Unsupported provider: ${dictationSettings.provider}`);
        }

        const response = await fetch(getApiUrl(endpoint), {
          method: 'POST',
          headers,
          body: JSON.stringify(body),
        });

        if (!response.ok) {
          if (response.status === 404) {
            throw new Error(
              `Audio transcription endpoint not found. Please implement ${endpoint} endpoint in the Goose backend.`
            );
          } else if (response.status === 401) {
            throw new Error('Invalid API key. Please check your API key is correct.');
          } else if (response.status === 402) {
            throw new Error('API quota exceeded. Please check your account limits.');
          }
          const errorData = await safeJsonParse<{
            error: { message: string };
          }>(response, 'Failed to parse error response').catch(() => ({
            error: { message: 'Transcription failed' },
          }));
          throw new Error(errorData.error?.message || 'Transcription failed');
        }

        const data = await safeJsonParse<{ text: string }>(
          response,
          'Failed to parse transcription response'
        );
        if (data.text) {
          onTranscription?.(data.text);
        }
      } catch (error) {
        console.error('Error transcribing audio:', error);
        onError?.(error as Error);
      } finally {
        setIsTranscribing(false);
        setRecordingDuration(0);
        setEstimatedSize(0);
      }
    },
    [onTranscription, onError, dictationSettings]
  );

  // Define stopRecording before startRecording to avoid circular dependency
  const stopRecording = useCallback(() => {
    if (mediaRecorderRef.current && mediaRecorderRef.current.state !== 'inactive') {
      mediaRecorderRef.current.stop();
      setIsRecording(false);
    }

    // Clear interval
    if (durationIntervalRef.current) {
      clearInterval(durationIntervalRef.current);
      durationIntervalRef.current = null;
    }

    // Stop all tracks in the stream
    if (streamRef.current) {
      streamRef.current.getTracks().forEach((track) => track.stop());
      streamRef.current = null;
    }

    // Close audio context
    if (audioContext && audioContext.state !== 'closed') {
      audioContext.close().catch(console.error);
      setAudioContext(null);
      setAnalyser(null);
    }
  }, [audioContext]);

  // Cleanup effect to prevent memory leaks
  useEffect(() => {
    return () => {
      // Cleanup on unmount
      if (durationIntervalRef.current) {
        clearInterval(durationIntervalRef.current);
      }
      if (streamRef.current) {
        streamRef.current.getTracks().forEach((track) => track.stop());
      }
      if (audioContext && audioContext.state !== 'closed') {
        audioContext.close().catch(console.error);
      }
    };
  }, [audioContext]);

  const startRecording = useCallback(async () => {
    if (!dictationSettings) {
      onError?.(new Error('Dictation settings not loaded'));
      return;
    }

    try {
      // Request microphone permission
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      streamRef.current = stream;

      // Create audio context and analyser for visualization
      const context = new AudioContext();
      const source = context.createMediaStreamSource(stream);
      const analyserNode = context.createAnalyser();
      analyserNode.fftSize = 2048;
      source.connect(analyserNode);

      setAudioContext(context);
      setAnalyser(analyserNode);

      // Create MediaRecorder
      const mediaRecorder = new MediaRecorder(stream, {
        mimeType: 'audio/webm',
      });

      mediaRecorderRef.current = mediaRecorder;
      audioChunksRef.current = [];
      currentSizeRef.current = 0;
      recordingStartTimeRef.current = Date.now();

      // Start duration and size tracking
      durationIntervalRef.current = setInterval(() => {
        const elapsed = (Date.now() - (recordingStartTimeRef.current || 0)) / 1000;
        setRecordingDuration(elapsed);

        // Estimate size based on typical webm bitrate (~128kbps)
        const estimatedSizeMB = (elapsed * 128 * 1024) / (8 * 1024 * 1024);
        setEstimatedSize(estimatedSizeMB);

        // Check if we're approaching the limit
        if (estimatedSizeMB > WARNING_SIZE_MB) {
          onSizeWarning?.(estimatedSizeMB);
        }

        // Auto-stop if we hit the duration limit
        if (elapsed >= MAX_RECORDING_DURATION_SECONDS) {
          stopRecording();
          onError?.(
            new Error(
              `Maximum recording duration (${MAX_RECORDING_DURATION_SECONDS / 60} minutes) reached.`
            )
          );
        }
      }, 100);

      mediaRecorder.ondataavailable = (event) => {
        if (event.data.size > 0) {
          audioChunksRef.current.push(event.data);
          currentSizeRef.current += event.data.size;

          // Check actual size
          const actualSizeMB = currentSizeRef.current / (1024 * 1024);
          if (actualSizeMB > MAX_AUDIO_SIZE_MB) {
            stopRecording();
            onError?.(new Error(`Maximum file size (${MAX_AUDIO_SIZE_MB}MB) reached.`));
          }
        }
      };

      mediaRecorder.onstop = async () => {
        const audioBlob = new Blob(audioChunksRef.current, { type: 'audio/webm' });
        await transcribeAudio(audioBlob);
      };

      mediaRecorder.start(1000); // Collect data every second for size monitoring
      setIsRecording(true);
    } catch (error) {
      console.error('Error starting recording:', error);
      onError?.(error as Error);
    }
  }, [onError, onSizeWarning, transcribeAudio, stopRecording, dictationSettings]);

  return {
    isRecording,
    isTranscribing,
    hasOpenAIKey,
    canUseDictation,
    audioContext,
    analyser,
    startRecording,
    stopRecording,
    recordingDuration,
    estimatedSize,
  };
};
