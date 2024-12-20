import React, { useRef, useState, useEffect } from 'react';
import { Button } from './ui/button';
import Send from './ui/Send';
import Stop from './ui/Stop';
import { Paperclip } from 'lucide-react';
import { getApiUrl } from "../config";
import { AudioButton, AudioWaveform } from './AudioRecorder';
declare class Blob{}
declare class FormData{}

interface InputProps {
  handleSubmit: (e: React.FormEvent) => void;
  disabled?: boolean;
  isLoading?: boolean;
  onStop?: () => void;
}

declare global {
  interface Window {
    electron: {
      selectFileOrDirectory: () => Promise<string | null>;
    };
  }
}

export default function Input({
  handleSubmit,
  disabled = false,
  isLoading = false,
  onStop
}: InputProps) {
  const [value, setValue] = useState('');
  const [isRecording, setIsRecording] = useState(false);
  const textAreaRef = useRef<HTMLTextAreaElement>(null);
  const waveformRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (textAreaRef.current && !disabled) {
      textAreaRef.current.focus();
    }
  }, [disabled, value]);

  const useAutosizeTextArea = (textAreaRef: HTMLTextAreaElement | null, value: string) => {
    useEffect(() => {
      if (textAreaRef) {
        textAreaRef.style.height = "0px"; // Reset height
        const scrollHeight = textAreaRef.scrollHeight;
        textAreaRef.style.height = Math.min(scrollHeight, maxHeight) + "px";
      }
    }, [textAreaRef, value]);
  };

  const minHeight = "1rem";
  const maxHeight = 10 * 24;

  useAutosizeTextArea(textAreaRef.current, value);

  const handleChange = (evt: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = evt.target?.value;
    setValue(val);
  };

  const handleKeyDown = (evt: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (evt.key === 'Enter' && !evt.shiftKey) {
      evt.preventDefault();
      if (value.trim()) {
        handleSubmit(new CustomEvent('submit', { detail: { value } }));
        setValue('');
      }
    }
  };

  const onFormSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (value.trim()) {
      handleSubmit(new CustomEvent('submit', { detail: { value } }));
      setValue('');
    }
  };

  const handleFileSelect = async () => {
    const path = await window.electron.selectFileOrDirectory();
    if (path) {
      setValue(path);
      textAreaRef.current?.focus();
    }
  };

  const handleRecordEnd = async (blob: Blob) => {
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
        setValue(result.text);
        textAreaRef.current?.focus();
      } else {
        console.error('Transcription error:', result.error);
      }
    } catch (err) {
      console.error('Transcription error:', err);
    }
  };

  return (
    <form onSubmit={onFormSubmit} className="flex relative bg-white dark:bg-gray-800 h-auto px-[16px] pr-[68px] py-[1rem]">
      <div className="relative flex-grow">
        <textarea
          autoFocus
          id="dynamic-textarea"
          placeholder="What should goose do?"
          value={value}
          onChange={handleChange}
          onKeyDown={handleKeyDown}
          disabled={disabled}
          ref={textAreaRef}
          rows={1}
          style={{
            minHeight: `${minHeight}px`,
            maxHeight: `${maxHeight}px`,
            overflowY: 'auto'
          }}
          className={`w-full outline-none border-none focus:ring-0 bg-transparent p-0 text-14 resize-none ${
            disabled ? 'cursor-not-allowed opacity-50' : ''
          }
          ${!isRecording ? 'opacity-100' : 'opacity-0'}`}
        />
        <AudioWaveform
          ref={waveformRef}
          isRecording={isRecording}
          onRecordEnd={handleRecordEnd}
          className="absolute left-0 right-0 bottom-0 z-5 overflow-hidden w-5/6"
        />
      </div>
      <div className="absolute right-[68px] top-1/2 -translate-y-1/2 flex items-center gap-2">
        <AudioButton
          isRecording={isRecording}
          onClick={() => setIsRecording(!isRecording)}
        />
        <Button
          type="button"
          size="icon"
          variant="ghost"
          onClick={handleFileSelect}
          disabled={disabled}
          className={`text-indigo-600 dark:text-indigo-300 hover:text-indigo-700 dark:hover:text-indigo-200 hover:bg-indigo-100 dark:hover:bg-indigo-800 ${
            disabled ? 'opacity-50 cursor-not-allowed' : ''
          }`}
        >
          <Paperclip size={20} />
        </Button>
      </div>
      {isLoading ? (
        <Button
          type="button"
          size="icon"
          variant="ghost"
          onClick={onStop}
          className="absolute right-2 top-1/2 -translate-y-1/2 text-indigo-600 hover:text-indigo-700 hover:bg-indigo-100"
        >
          <Stop size={24} />
        </Button>
      ) : (
        <Button
          type="submit"
          size="icon"
          variant="ghost"
          disabled={disabled || !value.trim()}
          className={`absolute right-2 top-1/2 -translate-y-1/2 text-indigo-600 dark:text-indigo-300 hover:text-indigo-700 dark:hover:text-indigo-200 hover:bg-indigo-100 dark:hover:bg-indigo-800 ${
            disabled || !value.trim() ? 'opacity-50 cursor-not-allowed' : ''
          }`}
        >
          <Send size={24} />
        </Button>
      )}
    </form>
  );
}
