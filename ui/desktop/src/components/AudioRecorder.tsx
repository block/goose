import React, { useState, useRef, useEffect, useCallback } from 'react';
import { Button } from './ui/button';
import { Mic, Square } from 'lucide-react';
import { getApiUrl } from "../config";
import WaveSurfer from 'wavesurfer.js';
import RecordPlugin from 'wavesurfer.js/dist/plugins/record.esm.js';
declare class Blob{}
declare class FormData{}

// Separate button component
export const AudioButton = ({
  isRecording,
  onClick,
}: {
  isRecording: boolean;
  onClick: () => void;
}) => (
  <Button
    type="button"
    size="icon"
    variant="ghost"
    onClick={onClick}
    className={`text-indigo-600 dark:text-indigo-300 hover:text-indigo-700 dark:hover:text-indigo-200 hover:bg-indigo-100 dark:hover:bg-indigo-800 flex-shrink-0`}
  >
    {isRecording ? <Square size={20} /> : <Mic size={20} />}
  </Button>
);

// Separate waveform component with its own state management
export const AudioWaveform = React.forwardRef<
  HTMLDivElement,
  {
    isRecording: boolean;
    onRecordEnd?: (blob: Blob) => void;
    className?: string;
  }
>(({ isRecording, onRecordEnd, className = '' }, ref) => {
  const wavesurferRef = useRef<WaveSurfer | null>(null);
  const recordPluginRef = useRef<any>(null);
  const [progress, setProgress] = useState('00:00');

  const handleRecordProgress = useCallback((time: number) => {
    const minutes = Math.floor((time % 3600000) / 60000);
    const seconds = Math.floor((time % 60000) / 1000);
    const formattedTime = [minutes, seconds]
      .map(v => v < 10 ? '0' + v : v)
      .join(':');
    setProgress(formattedTime);
  }, []);

  useEffect(() => {
    const container = ref as React.RefObject<HTMLDivElement>;
    if (!container.current) return;

    const wavesurfer = WaveSurfer.create({
      container: container.current,
      waveColor: 'rgb(99, 102, 241)', // Indigo-600
      progressColor: 'rgb(79, 70, 229)', // Indigo-700
      height: 26,
      barWidth: 2,
      barGap: 1,
      barRadius: 1,
      normalize: true,
      minPxPerSec: 50, // Increase this value to make the waveform wider
    });

    const recordPlugin = wavesurfer.registerPlugin(
      RecordPlugin.create({
        renderRecordedAudio: false,
        scrollingWaveform: false,
        continuousWaveform: true,
        continuousWaveformDuration: 30,
      })
    );

    if (onRecordEnd) {
      recordPlugin.on('record-end', onRecordEnd);
    }
    recordPlugin.on('record-progress', handleRecordProgress);

    wavesurferRef.current = wavesurfer;
    recordPluginRef.current = recordPlugin;

    return () => {
      wavesurfer.destroy();
      wavesurferRef.current = null;
      recordPluginRef.current = null;
    };
  }, [ref, onRecordEnd, handleRecordProgress]);

  useEffect(() => {
    const recordPlugin = recordPluginRef.current;
    if (!recordPlugin) return;

    const handleRecording = async () => {
      if (isRecording) {
        try {
          await recordPlugin.startRecording();
        } catch (err) {
          console.error('Failed to start recording:', err);
        }
      } else {
        try {
          if (recordPlugin.isRecording()) {
            await recordPlugin.stopRecording();
            setProgress('00:00');
          }
        } catch (err) {
          console.error('Failed to stop recording:', err);
        }
      }
    };

    handleRecording();
  }, [isRecording]);

  return (
    <div
      className={`flex-grow transition-all duration-200 ${
        isRecording ? 'opacity-100 h-[26px]' : 'opacity-0 h-0'
      } ${className}`}
    >
      <div ref={ref} className="w-full h-full" />
    </div>
  );
});

AudioWaveform.displayName = 'AudioWaveform';

// Main AudioRecorder component that combines both
export function AudioRecorder({ onTranscription, containerClassName }: {
  onTranscription: (text: string) => void;
  containerClassName?: string;
}) {
  const [isRecording, setIsRecording] = useState(false);
  const micContainerRef = useRef<HTMLDivElement>(null);

  const handleRecordEnd = useCallback(async (blob: Blob) => {
    try {
      console.log('Recording completed, size:', blob.size, 'type:', blob.type);
      const formData = new FormData();
      formData.append('audio', blob, 'audio.webm');

      const response = await fetch(getApiUrl('/transcribe'), {
        method: 'POST',
        body: formData,
      });

      if (!response.ok) {
        throw new Error('Transcription failed');
      }

      const result = await response.json();
      console.log('Received response:', result);
      if (result.success) {
        onTranscription(result.text);
      } else {
        console.error('Transcription error:', result.error);
      }
    } catch (err) {
      console.error('Transcription error:', err);
    }
  }, [onTranscription]);

  const handleToggleRecording = useCallback(() => {
    setIsRecording(prev => !prev);
  }, []);

  return (
    <div className={`flex items-center gap-2 w-full ${containerClassName || ''}`}>
      <AudioWaveform
        ref={micContainerRef}
        isRecording={isRecording}
        onRecordEnd={handleRecordEnd}
        className="flex-grow"
      />
      <AudioButton isRecording={isRecording} onClick={handleToggleRecording} />
    </div>
  );
}
