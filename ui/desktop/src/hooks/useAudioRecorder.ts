import { useState, useRef, useCallback, useEffect } from 'react';
import { transcribeDictation, getDictationConfig, DictationProvider } from '../api';
import { useConfig } from '../components/ConfigContext';

interface UseAudioRecorderOptions {
  onTranscription?: (text: string) => void;
  onError?: (error: Error) => void;
}

const MAX_AUDIO_SIZE_MB = 25;
const MAX_RECORDING_DURATION_SECONDS = 600; // 10 minutes

export const useAudioRecorder = ({ onTranscription, onError }: UseAudioRecorderOptions = {}) => {
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

  // Check provider configuration on mount
  useEffect(() => {
    const checkProviderConfig = async () => {
      try {
        // Read provider preference from backend config
        const providerValue = await read('voice_dictation_provider', false);
        const preferredProvider = (providerValue as DictationProvider) || null;
        console.log('[useAudioRecorder] Read voice_dictation_provider:', preferredProvider);

        // If no provider selected, dictation is disabled
        if (!preferredProvider) {
          console.log('[useAudioRecorder] No provider selected, setting to null');
          setIsEnabled(false);
          setProvider(null);
          return;
        }

        // Check backend audio config to see if provider is actually configured (has API key)
        const audioConfigResponse = await getDictationConfig();
        const providerStatus = audioConfigResponse.data?.[preferredProvider];
        console.log(
          '[useAudioRecorder] Provider status for',
          preferredProvider,
          ':',
          providerStatus
        );

        if (providerStatus?.configured) {
          console.log('[useAudioRecorder] Provider is configured, enabling');
          setIsEnabled(true);
          setProvider(preferredProvider);
        } else {
          console.log('[useAudioRecorder] Provider not configured, disabling but keeping provider');
          setIsEnabled(false);
          setProvider(preferredProvider);
        }
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

  // Cleanup on unmount
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
        onError?.(new Error('No transcription provider configured'));
        return;
      }

      setIsTranscribing(true);

      try {
        // Check file size
        const sizeMB = audioBlob.size / (1024 * 1024);
        if (sizeMB > MAX_AUDIO_SIZE_MB) {
          throw new Error(
            `Audio file too large (${sizeMB.toFixed(1)}MB). Maximum size is ${MAX_AUDIO_SIZE_MB}MB.`
          );
        }

        // Convert to base64
        const reader = new FileReader();
        const base64Audio = await new Promise<string>((resolve, reject) => {
          reader.onloadend = () => {
            const base64 = reader.result as string;
            resolve(base64.split(',')[1]);
          };
          reader.onerror = reject;
          reader.readAsDataURL(audioBlob);
        });

        const mimeType = audioBlob.type;
        if (!mimeType) {
          throw new Error('Unable to determine audio format');
        }

        // Transcribe using generated API
        const result = await transcribeDictation({
          body: {
            audio: base64Audio,
            mime_type: mimeType,
            provider: provider,
          },
          throwOnError: true,
        });

        if (result.data?.text) {
          onTranscription?.(result.data.text);
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
    [provider, onTranscription, onError]
  );

  const startRecording = useCallback(async () => {
    if (!isEnabled) {
      onError?.(new Error('Voice dictation is not enabled'));
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

      // Determine best supported MIME type
      const supportedTypes = ['audio/webm;codecs=opus', 'audio/webm', 'audio/mp4', 'audio/wav'];
      const mimeType = supportedTypes.find((type) => MediaRecorder.isTypeSupported(type)) || '';

      const mediaRecorder = new MediaRecorder(stream, mimeType ? { mimeType } : {});
      mediaRecorderRef.current = mediaRecorder;
      audioChunksRef.current = [];

      // Track recording duration and size
      const startTime = Date.now();
      durationIntervalRef.current = setInterval(() => {
        const elapsed = (Date.now() - startTime) / 1000;
        setRecordingDuration(elapsed);

        // Estimate size based on typical webm bitrate (~128kbps)
        const estimatedSizeMB = (elapsed * 128 * 1024) / (8 * 1024 * 1024);
        setEstimatedSize(estimatedSizeMB);

        // Auto-stop at max duration
        if (elapsed >= MAX_RECORDING_DURATION_SECONDS) {
          stopRecording();
          onError?.(
            new Error(
              `Maximum recording duration (${MAX_RECORDING_DURATION_SECONDS / 60} minutes) reached`
            )
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
          onError?.(new Error('No audio data was recorded. Please check your microphone.'));
          return;
        }

        await transcribeAudio(audioBlob);
      };

      mediaRecorder.onerror = (event) => {
        console.error('MediaRecorder error:', event);
        onError?.(new Error('Recording failed'));
      };

      mediaRecorder.start(100);
      setIsRecording(true);
    } catch (error) {
      console.error('Error starting recording:', error);
      stopRecording();
      onError?.(error as Error);
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
