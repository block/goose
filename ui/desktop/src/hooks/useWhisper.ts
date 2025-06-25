import { useState, useRef, useCallback, useEffect } from 'react';
import { useConfig } from '../components/ConfigContext';
import { getApiUrl, getSecretKey } from '../config';

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

  const transcribeAudio = useCallback(
    async (audioBlob: Blob) => {
      setIsTranscribing(true);

      try {
        // Check final size
        const sizeMB = audioBlob.size / (1024 * 1024);
        if (sizeMB > MAX_AUDIO_SIZE_MB) {
          throw new Error(`Audio file too large (${sizeMB.toFixed(1)}MB). Maximum size is ${MAX_AUDIO_SIZE_MB}MB.`);
        }

        // IMPORTANT: This is the proper way to implement audio transcription in Goose.
        // The API keys are securely stored on the backend and should never be exposed to the frontend.
        
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
        
        // The backend endpoint should be implemented to handle this request
        const response = await fetch(getApiUrl('/audio/transcribe'), {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'X-Secret-Key': getSecretKey(),
          },
          body: JSON.stringify({
            audio: base64Audio,
            mime_type: 'audio/webm',
          }),
        });
        
        if (!response.ok) {
          if (response.status === 404) {
            throw new Error(
              'Audio transcription endpoint not found. Please implement /audio/transcribe endpoint in the Goose backend.'
            );
          }
          const errorData = await response.json().catch(() => ({ error: { message: 'Transcription failed' } }));
          throw new Error(errorData.error?.message || 'Transcription failed');
        }
        
        const data = await response.json();
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
    [onTranscription, onError]
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
    if (audioContext) {
      audioContext.close();
      setAudioContext(null);
      setAnalyser(null);
    }
  }, [audioContext]);

  const startRecording = useCallback(async () => {
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
          onError?.(new Error(`Maximum recording duration (${MAX_RECORDING_DURATION_SECONDS / 60} minutes) reached.`));
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
  }, [onError, onSizeWarning, transcribeAudio, stopRecording]);

  return {
    isRecording,
    isTranscribing,
    hasOpenAIKey,
    audioContext,
    analyser,
    startRecording,
    stopRecording,
    recordingDuration,
    estimatedSize,
  };
};
