import React, { useState, useRef, useEffect, useCallback } from 'react';
import { Button } from './ui/button';
import { Mic, Square } from 'lucide-react';
import { getApiUrl } from "../config";
import WaveSurfer from 'wavesurfer.js';
import RecordPlugin from 'wavesurfer.js/dist/plugins/record.esm.js';

interface AudioRecorderProps {
  onTranscription: (text: string) => void;
}

export function AudioRecorder({ onTranscription }: AudioRecorderProps) {
  const [isRecording, setIsRecording] = useState(false);
  const [progress, setProgress] = useState('00:00');
  const wavesurferRef = useRef<WaveSurfer | null>(null);
  const recordPluginRef = useRef<any>(null);
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

  const handleRecordProgress = useCallback((time: number) => {
    const minutes = Math.floor((time % 3600000) / 60000);
    const seconds = Math.floor((time % 60000) / 1000);
    const formattedTime = [minutes, seconds]
      .map(v => v < 10 ? '0' + v : v)
      .join(':');
    setProgress(formattedTime);
  }, []);

  useEffect(() => {
    let wavesurfer: WaveSurfer | null = null;
    let recordPlugin: any = null;

    const initializeWaveSurfer = () => {
      if (!micContainerRef.current) return;

      // Create new WaveSurfer instance
      wavesurfer = WaveSurfer.create({
        container: micContainerRef.current,
        waveColor: 'rgb(99, 102, 241)', // Indigo-600
        progressColor: 'rgb(79, 70, 229)', // Indigo-700
        height: 40,
      });

      // Initialize Record plugin
      recordPlugin = wavesurfer.registerPlugin(
        RecordPlugin.create({
          renderRecordedAudio: false,
          scrollingWaveform: false,
          continuousWaveform: true,
          continuousWaveformDuration: 30,
        })
      );

      // Set up event handlers
      recordPlugin.on('record-end', handleRecordEnd);
      recordPlugin.on('record-progress', handleRecordProgress);

      // Store references
      wavesurferRef.current = wavesurfer;
      recordPluginRef.current = recordPlugin;
    };

    initializeWaveSurfer();

    // Cleanup
    return () => {
      if (wavesurfer) {
        wavesurfer.destroy();
      }
      wavesurferRef.current = null;
      recordPluginRef.current = null;
    };
  }, [handleRecordEnd, handleRecordProgress]);

  const startRecording = async () => {
    console.log('Attempting to start recording...');
    try {
      if (!recordPluginRef.current) {
        console.error('Record plugin not initialized');
        return;
      }

      await recordPluginRef.current.startRecording();
      console.log('Recording started!');
      setIsRecording(true);
    } catch (err) {
      console.error('Failed to start recording:', err);
    }
  };

  const stopRecording = async () => {
    if (!recordPluginRef.current || !isRecording) return;

    console.log('Stopping recording...');
    try {
      await recordPluginRef.current.stopRecording();
      setIsRecording(false);
      setProgress('00:00');
    } catch (err) {
      console.error('Failed to stop recording:', err);
    }
  };

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2">
        <div
            ref={micContainerRef}
            className={`waveform transition-opacity duration-200 ${isRecording ? 'opacity-100'
                : 'opacity-0'}`}
        />
        <Button
            type="button"
            size="icon"
            variant="ghost"
            onClick={isRecording ? stopRecording : startRecording}
            className={`text-indigo-600 dark:text-indigo-300 hover:text-indigo-700 dark:hover:text-indigo-200 hover:bg-indigo-100 dark:hover:bg-indigo-800`}
        >
          {isRecording ? <Square size={20}/> : <Mic size={20}/>}
        </Button>
      </div>
    </div>
  );
}
