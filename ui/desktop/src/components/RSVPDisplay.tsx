import React, { useState, useEffect, useRef } from 'react';
import { Button } from './ui/button';
import { Input } from './ui/input';
import { Play, Pause, Reset } from './icons';

interface RSVPDisplayProps {
  text: string;
  onClose: () => void;
}

export default function RSVPDisplay({ text, onClose }: RSVPDisplayProps) {
  const [isPlaying, setIsPlaying] = useState(false);
  const [currentWordIndex, setCurrentWordIndex] = useState(0);
  const [wordsPerMinute, setWordsPerMinute] = useState(420);
  const [words, setWords] = useState<string[]>([]);
  const [wordsPerChunk, setWordsPerChunk] = useState(1);
  const [customWPM, setCustomWPM] = useState('');
  const containerRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    // Split text into words and clean them
    const cleanedWords = text
      .replace(/\n/g, ' ')
      .split(/\s+/)
      .filter((word) => word.length > 0);
    setWords(cleanedWords);
    setCurrentWordIndex(0);
  }, [text]);

  useEffect(() => {
    let intervalId: ReturnType<typeof setInterval>;

    if (isPlaying && currentWordIndex < words.length) {
      const interval = (60 * 1000) / wordsPerMinute;
      intervalId = setInterval(() => {
        setCurrentWordIndex((prev) => {
          if (prev >= words.length - wordsPerChunk) {
            setIsPlaying(false);
            return prev;
          }
          return prev + wordsPerChunk;
        });
      }, interval);
    }

    return () => {
      if (intervalId) {
        clearInterval(intervalId);
      }
    };
  }, [isPlaying, currentWordIndex, words.length, wordsPerMinute, wordsPerChunk]);

  // Handle keyboard events
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.code === 'Space') {
      e.preventDefault();
      e.stopPropagation();
      setIsPlaying((prev) => !prev);
    } else if (e.code === 'Escape') {
      onClose();
    }
  };

  useEffect(() => {
    const handleGlobalKeyDown = (e: KeyboardEvent) => {
      if (e.code === 'Space') {
        e.preventDefault();
        e.stopPropagation();
        setIsPlaying((prev) => !prev);
      } else if (e.code === 'Escape') {
        onClose();
      }
    };

    window.addEventListener('keydown', handleGlobalKeyDown, true);
    return () => {
      window.removeEventListener('keydown', handleGlobalKeyDown, true);
    };
  }, [onClose]);

  const handleWPMChange = (delta: number) => {
    const newWPM = Math.max(100, Math.min(1000, wordsPerMinute + delta));
    setWordsPerMinute(newWPM);
    setCustomWPM('');
  };

  const handleCustomWPMSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const wpm = parseInt(customWPM);
    if (!isNaN(wpm) && wpm >= 100 && wpm <= 1000) {
      setWordsPerMinute(wpm);
    }
    setCustomWPM('');
  };

  const handleCustomWPMChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    setCustomWPM(value);
    const wpm = parseInt(value);
    if (!isNaN(wpm) && wpm >= 100 && wpm <= 1000) {
      setWordsPerMinute(wpm);
    }
  };

  const handleReset = () => {
    setCurrentWordIndex(0);
    setIsPlaying(false);
  };

  const progress = (currentWordIndex / words.length) * 100;

  return (
    <div
      className="fixed inset-0 bg-black/80 flex items-center justify-center z-[100] pointer-events-none"
      tabIndex={0}
      ref={containerRef}
    >
      <div
        className="bg-white dark:bg-gray-800 rounded-lg p-6 w-[600px] max-w-[90vw] relative pointer-events-auto"
        onKeyDown={handleKeyDown}
      >
        <div className="flex justify-end mb-2">
          <Button variant="ghost" onClick={onClose}>
            Close
          </Button>
        </div>

        <div className="text-center text-sm text-gray-500 mb-4">
          Press space to {isPlaying ? 'pause' : 'play'} • Esc to close
        </div>

        <div className="h-[200px] flex items-center justify-center text-4xl font-bold mb-6">
          {words.slice(currentWordIndex, currentWordIndex + wordsPerChunk).join(' ')}
        </div>

        <div className="space-y-4">
          <div className="flex items-center gap-4 justify-center">
            <Button
              onClick={() => setIsPlaying(!isPlaying)}
              variant={isPlaying ? 'secondary' : 'default'}
              size="icon"
              className="h-12 w-12 rounded-full"
            >
              {isPlaying ? <Pause className="h-6 w-6" /> : <Play className="h-6 w-6" />}
            </Button>
            <Button
              onClick={handleReset}
              variant="outline"
              size="icon"
              className="h-12 w-12 rounded-full"
            >
              <Reset className="h-6 w-6" />
            </Button>
          </div>

          <div className="space-y-2">
            <div className="flex items-center gap-2 justify-center">
              <Button
                variant="outline"
                size="sm"
                onClick={() => setWordsPerChunk((prev) => Math.max(1, prev - 1))}
                disabled={wordsPerChunk <= 1}
              >
                -1
              </Button>
              <span className="w-8 text-center">{wordsPerChunk}</span>
              <Button
                variant="outline"
                size="sm"
                onClick={() => setWordsPerChunk((prev) => Math.min(5, prev + 1))}
                disabled={wordsPerChunk >= 5}
              >
                +1
              </Button>
              <span className="text-sm ml-2">words per chunk</span>
            </div>

            <div className="flex items-center gap-2 justify-center">
              <Button variant="outline" size="sm" onClick={() => handleWPMChange(-10)}>
                -10
              </Button>
              <form onSubmit={handleCustomWPMSubmit} className="w-20">
                <Input
                  ref={inputRef}
                  type="number"
                  value={customWPM || wordsPerMinute}
                  onChange={handleCustomWPMChange}
                  className="text-center [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
                  min={100}
                  max={1000}
                />
              </form>
              <Button variant="outline" size="sm" onClick={() => handleWPMChange(10)}>
                +10
              </Button>
              <span className="text-sm ml-2">chunks per minute</span>
            </div>
          </div>

          <div className="w-full bg-gray-200 rounded-full h-2.5 dark:bg-gray-700">
            <div
              className="bg-primary h-2.5 rounded-full transition-all duration-300"
              style={{ width: `${progress}%` }}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
