import React, { useState, useEffect, useRef } from 'react';
import { Button } from './ui/button';
import { Input } from './ui/input';
import { Play, Pause, Reset } from './icons';

interface RSVPDisplayProps {
  text: string;
  onClose: () => void;
}

// Types of structured content we want to handle
type StructuredContentType = 'table' | 'code' | 'list' | 'none';

interface ContentSection {
  type: StructuredContentType;
  content: string;
  startIndex: number;
  endIndex: number;
}

// Detect structured content in the text
function detectStructuredContent(text: string): ContentSection[] {
  const sections: ContentSection[] = [];

  // Regular expressions for different types of structured content
  const patterns = {
    // More lenient table detection that matches any line starting with |
    table: /(?:\|[^\n]*\|\n?)+/g,
    code: /```[\s\S]*?```/g,
    list: /(?:^|\n)(?:\s*[-*+]|\d+\.)\s+.*(?:\n(?:\s*[-*+]|\d+\.)\s+.*)*/g,
  };

  // Find all structured content
  for (const [type, pattern] of Object.entries(patterns)) {
    let match;
    while ((match = pattern.exec(text)) !== null) {
      console.log(`Found ${type} at index ${match.index}:`, match[0]);
      sections.push({
        type: type as StructuredContentType,
        content: match[0],
        startIndex: match.index,
        endIndex: match.index + match[0].length,
      });
    }
  }

  // Sort sections by start index
  sections.sort((a, b) => a.startIndex - b.startIndex);

  // Add non-structured content sections
  const result: ContentSection[] = [];
  let lastEnd = 0;

  for (const section of sections) {
    if (section.startIndex > lastEnd) {
      result.push({
        type: 'none',
        content: text.slice(lastEnd, section.startIndex),
        startIndex: lastEnd,
        endIndex: section.startIndex,
      });
    }
    result.push(section);
    lastEnd = section.endIndex;
  }

  if (lastEnd < text.length) {
    result.push({
      type: 'none',
      content: text.slice(lastEnd),
      startIndex: lastEnd,
      endIndex: text.length,
    });
  }

  console.log('Final sections:', result);
  return result;
}

export default function RSVPDisplay({ text, onClose }: RSVPDisplayProps) {
  const [isPlaying, setIsPlaying] = useState(false);
  const [currentWordIndex, setCurrentWordIndex] = useState(0);
  const [wordsPerMinute, setWordsPerMinute] = useState(420);
  const [words, setWords] = useState<string[]>([]);
  const [wordsPerChunk, setWordsPerChunk] = useState(1);
  const [customWPM, setCustomWPM] = useState('');
  const [contentSections, setContentSections] = useState<ContentSection[]>([]);
  const [currentSectionIndex, setCurrentSectionIndex] = useState(0);
  const [showStructuredPreview, setShowStructuredPreview] = useState(false);
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

  // Split text into words and detect structured content
  useEffect(() => {
    const sections = detectStructuredContent(text);
    setContentSections(sections);
    setCurrentSectionIndex(0);
    setCurrentWordIndex(0);
  }, [text]);

  // RSVP playback logic
  useEffect(() => {
    if (isPlaying) {
      const interval = setInterval(
        () => {
          setCurrentWordIndex((prevIndex) => {
            const currentSection = contentSections[currentSectionIndex];
            console.log('Current section:', currentSection);

            // If we're at a structured content section
            if (currentSection?.type !== 'none') {
              console.log('Stopping at structured content:', currentSection.type);
              setIsPlaying(false);
              setShowStructuredPreview(true);
              return prevIndex;
            }

            const words = currentSection.content.split(/\s+/);
            if (prevIndex + wordsPerChunk >= words.length) {
              // Move to next section if available
              if (currentSectionIndex + 1 < contentSections.length) {
                console.log('Moving to next section');
                setCurrentSectionIndex(currentSectionIndex + 1);
                return 0;
              } else {
                setIsPlaying(false);
                return prevIndex;
              }
            }
            return prevIndex + wordsPerChunk;
          });
        },
        (60 * 1000) / wordsPerMinute
      );

      return () => clearInterval(interval);
    }
  }, [isPlaying, wordsPerMinute, wordsPerChunk, contentSections, currentSectionIndex]);

  const currentSection = contentSections[currentSectionIndex];
  const currentWords = currentSection?.content.split(/\s+/) || [];
  const displayWords = currentWords.slice(currentWordIndex, currentWordIndex + wordsPerChunk);

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

        {showStructuredPreview && currentSection?.type !== 'none' ? (
          <div className="flex-1 overflow-auto mb-4">
            <div className="bg-bgSubtle p-4 rounded-lg text-center">
              <div className="text-lg mb-2">
                {currentSection.type === 'table' && '📊 Table detected'}
                {currentSection.type === 'code' && '💻 Code block detected'}
                {currentSection.type === 'list' && '📝 List detected'}
              </div>
              <div className="text-sm text-textSubtle mb-4">
                This section contains formatted content that may be difficult to read through RSVP
              </div>
              <div className="flex justify-center gap-4">
                <Button
                  variant="outline"
                  onClick={() => {
                    setShowStructuredPreview(false);
                    setCurrentSectionIndex(currentSectionIndex + 1);
                    setCurrentWordIndex(0);
                    setIsPlaying(true);
                  }}
                >
                  Skip and continue
                </Button>
                <Button
                  variant="default"
                  onClick={() => {
                    setShowStructuredPreview(false);
                    setCurrentSectionIndex(currentSectionIndex + 1);
                    setCurrentWordIndex(0);
                    setIsPlaying(true);
                  }}
                >
                  View in chat
                </Button>
              </div>
            </div>
          </div>
        ) : (
          <div className="h-[200px] flex items-center justify-center text-4xl font-bold mb-6">
            {displayWords.join(' ')}
          </div>
        )}

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
