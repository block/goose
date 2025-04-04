import React, { useRef, useState, useEffect } from 'react';
import { Button } from './ui/button';
import Stop from './ui/Stop';
import { Send } from './icons';
import BottomMenu from './BottomMenu';

interface InputProps {
  handleSubmit: (e: React.FormEvent) => void;
  isLoading?: boolean;
  onStop?: () => void;
  commandHistory?: string[];
  initialValue?: string;
  hasMessages?: boolean;
  setView: (view: string) => void;
  onFileSelect?: (files: File[]) => void;
}

export default function Input({
  handleSubmit,
  isLoading = false,
  onStop,
  commandHistory = [],
  initialValue = '',
  hasMessages,
  setView,
  onFileSelect
}: InputProps) {
  const [value, setValue] = useState(initialValue);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [isComposing, setIsComposing] = useState(false);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [savedInput, setSavedInput] = useState('');
  const textAreaRef = useRef<HTMLTextAreaElement>(null);

  const handleFileSelect = async () => {
    if (window.electron) {
      try {
        const result = await window.electron.fileChooser();
        if (result && onFileSelect) {
          // Convert the file paths to File objects
          const files = await Promise.all(
            result.map(async (filePath: string) => {
              const response = await fetch(`file://${filePath}`);
              const blob = await response.blob();
              return new File([blob], filePath.split('/').pop() || 'file', { type: blob.type });
            })
          );
          onFileSelect(files);
        }
      } catch (error) {
        console.error('Error selecting files:', error);
      }
    } else {
      // Fallback to standard file input for non-electron environments
      fileInputRef.current?.click();
    }
  };

  const handleFileInputChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(event.target.files || []);
    if (files.length > 0 && onFileSelect) {
      onFileSelect(files);
    }
    // Reset the input value to allow selecting the same file again
    event.target.value = '';
  };

  useEffect(() => {
    if (initialValue) {
      setValue(initialValue);
    }
    // Reset height when value is cleared
    if (!value && textAreaRef.current) {
      textAreaRef.current.style.height = '24px';
    }
  }, [initialValue, value]);

  useEffect(() => {
    if (textAreaRef.current) {
      textAreaRef.current.focus();
    }
  }, []);

  const handleChange = (evt: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = evt.target.value;
    setValue(val);
    
    // Adjust height after value change
    const textArea = evt.target;
    textArea.style.height = 'auto';
    const lineHeight = 24; // Base line height
    const maxHeight = lineHeight * 10; // 10 lines maximum
    const newHeight = Math.min(textArea.scrollHeight, maxHeight);
    textArea.style.height = `${newHeight}px`;
  };

  const handleCompositionStart = () => {
    setIsComposing(true);
  };

  const handleCompositionEnd = () => {
    setIsComposing(false);
  };

  const handleHistoryNavigation = (evt: React.KeyboardEvent<HTMLTextAreaElement>) => {
    evt.preventDefault();

    if (historyIndex === -1) {
      setSavedInput(value);
    }

    let newIndex = historyIndex;
    if (evt.key === 'ArrowUp') {
      if (historyIndex < commandHistory.length - 1) {
        newIndex = historyIndex + 1;
      }
    } else {
      if (historyIndex > -1) {
        newIndex = historyIndex - 1;
      }
    }

    if (newIndex == historyIndex) {
      return;
    }

    setHistoryIndex(newIndex);
    if (newIndex === -1) {
      setValue(savedInput);
    } else {
      setValue(commandHistory[newIndex] || '');
    }
  };

  const handleKeyDown = (evt: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if ((evt.metaKey || evt.ctrlKey) && (evt.key === 'ArrowUp' || evt.key === 'ArrowDown')) {
      handleHistoryNavigation(evt);
      return;
    }

    if (evt.key === 'Enter') {
      if (evt.shiftKey || isComposing) {
        return;
      }
      if (evt.altKey) {
        setValue(value + '\n');
        return;
      }

      evt.preventDefault();

      if (!isLoading && value.trim()) {
        handleSubmit(new CustomEvent('submit', { detail: { value } }));
        setValue('');
        setHistoryIndex(-1);
        setSavedInput('');
      }
    }
  };

  const onFormSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (value.trim() && !isLoading) {
      handleSubmit(new CustomEvent('submit', { detail: { value } }));
      setValue('');
      setHistoryIndex(-1);
      setSavedInput('');
    }
  };

  return (
    <div className="w-full">
      <div className="-space-y-[1px] bg-transparent">
        <form
          onSubmit={onFormSubmit}
          className="flex relative h-auto px-4 py-4 rounded-t-xl border border-borderSubtle bg-[#1E1E1E]"
        >
          <input
            type="file"
            ref={fileInputRef}
            onChange={handleFileInputChange}
            className="hidden"
            multiple
          />
          <textarea
            autoFocus
            id="dynamic-textarea"
            placeholder="What can goose help with?   ⌘↑/⌘↓"
            value={value}
            onChange={handleChange}
            onCompositionStart={handleCompositionStart}
            onCompositionEnd={handleCompositionEnd}
            onKeyDown={handleKeyDown}
            ref={textAreaRef}
            rows={1}
            className="w-full outline-none border-none focus:ring-0 bg-transparent p-0 text-base resize-none text-white placeholder-gray-400"
            style={{
              minHeight: '24px', // Single line height
              height: 'auto',
              overflowY: 'auto'
            }}
          />
          {isLoading ? (
            <Button
              type="button"
              size="icon"
              variant="ghost"
              onClick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                onStop?.();
              }}
              className="inline-flex items-center justify-center gap-2 whitespace-nowrap text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 hover:bg-accent h-6 w-6 absolute right-4 bottom-[16px] text-gray-400 hover:text-white"
            >
              <Stop size={16} />
            </Button>
          ) : (
            <Button
              type="submit"
              size="icon"
              variant="ghost"
              disabled={!value.trim()}
              className={`inline-flex items-center justify-center gap-2 whitespace-nowrap text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 hover:bg-accent h-6 w-6 absolute right-4 bottom-[16px] text-gray-400 hover:text-white ${
                !value.trim() ? 'opacity-50 cursor-not-allowed' : ''
              }`}
            >
              <Send />
            </Button>
          )}
        </form>
        <BottomMenu hasMessages={hasMessages} setView={setView} />
      </div>
    </div>
  );
}