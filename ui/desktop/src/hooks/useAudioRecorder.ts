import { useCallback, useEffect, useRef, useState } from 'react';
import type { DictationProvider } from '../api';
import { getDictationConfig, transcribeDictation } from '../api';
import { useConfig } from '../contexts/ConfigContext';
import { errorMessage } from '../utils/conversionUtils';

interface UseAudioRecorderOptions {
  onTranscription: (text: string) => void;
  onError: (message: string) => void;
}

const SAMPLE_RATE = 16000;
const SILENCE_MS = 800;
const MIN_SPEECH_MS = 200;
// RMS threshold for speech detection. Audio samples are Float32 in [-1, 1] range.
// 0.015 (~1.5% of full-scale) distinguishes normal speech from background noise
// without clipping early speech onsets. Determined empirically for 16kHz mono input.
const RMS_THRESHOLD = 0.015;

// Resolve worklet URL at runtime from window.location so it works under both
// the dev server (http://localhost) and packaged builds (file://).
const WORKLET_URL = new URL('audio-capture-worklet.js', window.location.href.split('#')[0]).href;

function encodeWav(samples: Float32Array, sampleRate: number): ArrayBuffer {
  const buf = new ArrayBuffer(44 + samples.length * 2);
  const v = new DataView(buf);
  const w = (o: number, s: string) => {
    for (let i = 0; i < s.length; i++) v.setUint8(o + i, s.charCodeAt(i));
  };
  w(0, 'RIFF');
  v.setUint32(4, 36 + samples.length * 2, true);
  w(8, 'WAVE');
  w(12, 'fmt ');
  v.setUint32(16, 16, true);
  v.setUint16(20, 1, true);
  v.setUint16(22, 1, true);
  v.setUint32(24, sampleRate, true);
  v.setUint32(28, sampleRate * 2, true);
  v.setUint16(32, 2, true);
  v.setUint16(34, 16, true);
  w(36, 'data');
  v.setUint32(40, samples.length * 2, true);
  let o = 44;
  for (let i = 0; i < samples.length; i++) {
    const s = Math.max(-1, Math.min(1, samples[i]));
    v.setInt16(o, s < 0 ? s * 0x8000 : s * 0x7fff, true);
    o += 2;
  }
  return buf;
}

function rms(samples: Float32Array): number {
  let sum = 0;
  for (let i = 0; i < samples.length; i++) sum += samples[i] * samples[i];
  return Math.sqrt(sum / samples.length);
}

function blobToBase64(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const r = new FileReader();
    r.onloadend = () => resolve((r.result as string).split(',')[1]);
    r.onerror = reject;
    r.readAsDataURL(blob);
  });
}

export const useAudioRecorder = ({ onTranscription, onError }: UseAudioRecorderOptions) => {
  const [isRecording, setIsRecording] = useState(false);
  const [isTranscribing, setIsTranscribing] = useState(false);
  const [isEnabled, setIsEnabled] = useState(false);
  const [provider, setProvider] = useState<DictationProvider | null>(null);

  const { read } = useConfig();

  const audioContextRef = useRef<AudioContext | null>(null);
  const streamRef = useRef<MediaStream | null>(null);

  // VAD state (all refs to avoid re-render/stale closure issues)
  const samplesRef = useRef<Float32Array[]>([]);
  const isSpeakingRef = useRef(false);
  const silenceStartRef = useRef(0);
  const speechStartRef = useRef(0);
  const pendingTranscriptions = useRef(0);
  const providerRef = useRef(provider);
  providerRef.current = provider;

  // Keep callback refs fresh
  const onTranscriptionRef = useRef(onTranscription);
  onTranscriptionRef.current = onTranscription;
  const onErrorRef = useRef(onError);
  onErrorRef.current = onError;

  useEffect(() => {
    const check = async () => {
      try {
        const val = await read('voice_dictation_provider', false);
        const pref = (val as DictationProvider) || null;
        if (!pref) {
          setIsEnabled(false);
          setProvider(null);
          return;
        }
        const resp = await getDictationConfig();
        setIsEnabled(!!resp.data?.[pref]?.configured);
        setProvider(pref);
      } catch (error) {
        console.error('Failed to check dictation config:', error);
        setIsEnabled(false);
        setProvider(null);
      }
    };
    check();
  }, [read]);

  const transcribeChunk = useCallback(async (samples: Float32Array) => {
    const prov = providerRef.current;
    if (!prov) return;

    pendingTranscriptions.current++;
    setIsTranscribing(true);

    try {
      const wav = new Blob([encodeWav(samples, SAMPLE_RATE)], { type: 'audio/wav' });
      const base64 = await blobToBase64(wav);
      const result = await transcribeDictation({
        body: { audio: base64, mime_type: 'audio/wav', provider: prov },
        throwOnError: true,
      });
      if (result.data?.text) {
        onTranscriptionRef.current(result.data.text);
      }
    } catch (error) {
      onErrorRef.current(errorMessage(error));
    } finally {
      pendingTranscriptions.current--;
      if (pendingTranscriptions.current === 0) setIsTranscribing(false);
    }
  }, []);

  const flush = useCallback(() => {
    const chunks = samplesRef.current;
    if (chunks.length === 0) return;

    const total = chunks.reduce((n, c) => n + c.length, 0);
    const merged = new Float32Array(total);
    let off = 0;
    for (const c of chunks) {
      merged.set(c, off);
      off += c.length;
    }
    samplesRef.current = [];
    transcribeChunk(merged);
  }, [transcribeChunk]);

  const flushRef = useRef(flush);
  flushRef.current = flush;

  const handleSamples = useCallback((samples: Float32Array) => {
    const now = Date.now();

    if (rms(samples) > RMS_THRESHOLD) {
      if (!isSpeakingRef.current) {
        isSpeakingRef.current = true;
        speechStartRef.current = now;
      }
      silenceStartRef.current = 0;
      samplesRef.current.push(new Float32Array(samples));
    } else if (isSpeakingRef.current) {
      samplesRef.current.push(new Float32Array(samples));

      if (silenceStartRef.current === 0) {
        silenceStartRef.current = now;
      } else if (now - silenceStartRef.current > SILENCE_MS) {
        if (now - speechStartRef.current > MIN_SPEECH_MS) {
          flushRef.current();
        } else {
          samplesRef.current = [];
        }
        isSpeakingRef.current = false;
        silenceStartRef.current = 0;
      }
    }
  }, []);

  const stopRecording = useCallback(() => {
    if (isSpeakingRef.current && samplesRef.current.length > 0) {
      flushRef.current();
    }
    isSpeakingRef.current = false;
    silenceStartRef.current = 0;

    audioContextRef.current?.close();
    audioContextRef.current = null;
    streamRef.current?.getTracks().forEach((t) => t.stop());
    streamRef.current = null;
    setIsRecording(false);
  }, []);

  const startRecording = useCallback(async () => {
    if (!isEnabled) {
      onError('Voice dictation is not enabled');
      return;
    }

    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        audio: { echoCancellation: true, noiseSuppression: true, autoGainControl: true },
      });
      streamRef.current = stream;

      const ctx = new AudioContext({ sampleRate: SAMPLE_RATE });
      audioContextRef.current = ctx;

      await ctx.audioWorklet.addModule(WORKLET_URL);

      const source = ctx.createMediaStreamSource(stream);
      // eslint-disable-next-line no-undef
      const worklet = new AudioWorkletNode(ctx, 'audio-capture');

      worklet.port.onmessage = (e: MessageEvent<Float32Array>) => handleSamples(e.data);

      // Connect through silent gain to keep worklet processing alive
      const silence = ctx.createGain();
      silence.gain.value = 0;
      source.connect(worklet);
      worklet.connect(silence);
      silence.connect(ctx.destination);

      setIsRecording(true);
    } catch (error) {
      stopRecording();
      onError(errorMessage(error));
    }
  }, [isEnabled, onError, handleSamples, stopRecording]);

  useEffect(() => {
    return () => {
      audioContextRef.current?.close();
      streamRef.current?.getTracks().forEach((t) => t.stop());
    };
  }, []);

  return {
    isEnabled,
    dictationProvider: provider,
    isRecording,
    isTranscribing,
    startRecording,
    stopRecording,
  };
};
