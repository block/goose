import React, { useRef, useState, useEffect } from 'react';
import { Button } from './ui/button';
import Stop from './ui/Stop';
import { Send, Close, Attach, Upload } from './icons';
import { DropzoneInputProps } from 'react-dropzone';

interface InputProps {
  handleSubmit: (e?: React.FormEvent) => void;
  isLoading?: boolean;
  onStop?: () => void;
  commandHistory?: string[];
  value: string;
  setValue: React.Dispatch<React.SetStateAction<string>>;
  isDragActive: boolean;
  getInputProps: (props?: React.HTMLProps<HTMLInputElement>) => DropzoneInputProps;
  attachedImages: string[];
  setAttachedImages: React.Dispatch<React.SetStateAction<string[]>>;
  removeAttachedImage: (indexToRemove: number) => void;
}

export default function Input({
  handleSubmit,
  isLoading = false,
  onStop,
  commandHistory = [],
  value,
  setValue,
  isDragActive,
  getInputProps,
  attachedImages,
  setAttachedImages,
  removeAttachedImage,
}: InputProps) {
  const [isComposing, setIsComposing] = useState(false);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [savedInput, setSavedInput] = useState('');
  const textAreaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (textAreaRef.current) {
      textAreaRef.current.focus();
    }
  }, []);

  const autosizeTextArea = (textArea: HTMLTextAreaElement | null, _value: string) => {
    if (textArea) {
      textArea.style.height = '0px';
      const scrollHeight = textArea.scrollHeight;
      textArea.style.height = Math.min(scrollHeight, 10 * 24) + 'px';
    }
  };

  const useAutosizeTextArea = (textAreaRef: HTMLTextAreaElement | null, value: string) => {
    useEffect(() => {
      autosizeTextArea(textAreaRef, value);
    }, [textAreaRef, value]);
  };

  const minHeight = '1rem';
  const maxHeight = 10 * 24;

  useAutosizeTextArea(textAreaRef.current, value);

  const handleChange = (evt: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = evt.target.value;
    setValue(val);
  };

  const handleCompositionStart = (_evt: React.CompositionEvent<HTMLTextAreaElement>) => {
    setIsComposing(true);
  };

  const handleCompositionEnd = (_evt: React.CompositionEvent<HTMLTextAreaElement>) => {
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

    if (newIndex === historyIndex) {
      return;
    }

    setHistoryIndex(newIndex);
    const newValue = newIndex === -1 ? savedInput : commandHistory[newIndex] || '';
    setValue(newValue);
  };

  const handleKeyDown = (evt: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if ((evt.metaKey || evt.ctrlKey) && (evt.key === 'ArrowUp' || evt.key === 'ArrowDown')) {
      handleHistoryNavigation(evt);
      return;
    }

    if (evt.key === 'Enter') {
      if (evt.shiftKey || isComposing || evt.altKey) {
        if (evt.altKey && !evt.shiftKey) {
          setValue(value + '\n');
        }
        return;
      }
      evt.preventDefault();
      submitData();
    }
  };

  const submitData = () => {
    if (!isLoading && (value.trim() || attachedImages.length > 0)) {
      handleSubmit();
      setHistoryIndex(-1);
      setSavedInput('');
    }
  };

  const handlePaste = (evt: React.ClipboardEvent<HTMLTextAreaElement>) => {
    const items = evt.clipboardData.items;
    for (let i = 0; i < items.length; i++) {
      if (items[i].type.indexOf('image') !== -1) {
        const blob = items[i].getAsFile();
        if (blob) {
          const reader = new FileReader();
          reader.onload = (event) => {
            const base64Image = event.target?.result as string;
            if (base64Image) {
              setAttachedImages((prevImages) => [...prevImages, base64Image]);
              textAreaRef.current?.focus();
            }
          };
          reader.onerror = (error) => {
            console.error('[Input.tsx] FileReader onerror:', error);
          };
          reader.readAsDataURL(blob);
        }
      }
    }
  };

  const openFileDialog = () => {
    const inputElement = document.querySelector('input[type="file"]');
    if (inputElement instanceof HTMLInputElement) {
      inputElement.click();
    }
  };

  const showDragOverlay = isDragActive;
  const hasAttachedImages = attachedImages.length > 0;
  const isOverlayMode = showDragOverlay && hasAttachedImages;
  const isCollapseReplaceMode = showDragOverlay && !hasAttachedImages;

  return (
    <div
      className={`border-t border-borderSubtle px-4 pt-4 pb-4 overflow-hidden ${isOverlayMode ? 'relative' : ''}`}
    >
      {/* Render hidden input unconditionally */}
      <input {...getInputProps()} />

      {/* Normal Input Content Wrapper - Conditionally Hidden */}
      <div
        className={`transition-opacity duration-150 ${
          isOverlayMode
            ? 'opacity-0 pointer-events-none' // Keep for height, hide visually
            : isCollapseReplaceMode
              ? 'opacity-0 h-0 overflow-hidden p-0 m-0 border-0 pointer-events-none hidden' // Collapse completely
              : 'opacity-100' // Normal visible state
        }`}
      >
        {attachedImages.length > 0 && (
          <div className="mb-4 flex flex-wrap gap-2 justify-start">
            {attachedImages.map((imageBase64, index) => (
              <div key={index} className="relative max-w-xs">
                <img
                  src={imageBase64}
                  alt={`Attached image preview ${index + 1}`}
                  className="max-h-20 w-auto rounded border border-borderSubtle object-contain"
                />
                <Button
                  type="button"
                  size="icon"
                  variant="ghost"
                  onClick={(e) => {
                    e.stopPropagation();
                    removeAttachedImage(index);
                  }}
                  className="absolute -top-1 -right-1 p-0.5 bg-black/50 text-white rounded-full hover:bg-black/75 w-5 h-5 flex items-center justify-center"
                  title={`Remove Image ${index + 1}`}
                >
                  <Close className="w-3 h-3" />
                </Button>
              </div>
            ))}
          </div>
        )}

        <div className="flex items-end gap-2">
          <textarea
            autoFocus
            id="dynamic-textarea"
            placeholder="What can goose help with?   ⌘↑/⌘↓"
            value={value}
            onChange={handleChange}
            onCompositionStart={handleCompositionStart}
            onCompositionEnd={handleCompositionEnd}
            onKeyDown={handleKeyDown}
            onPaste={handlePaste}
            ref={textAreaRef}
            rows={1}
            style={{
              minHeight: `${minHeight}px`,
              maxHeight: `${maxHeight}px`,
              overflowY: 'auto',
            }}
            className="flex-1 outline-none border-none focus:ring-0 bg-transparent p-0 text-base resize-none text-textStandard self-center"
          />
          <Button
            type="button"
            size="icon"
            variant="ghost"
            className="text-textSubtle hover:text-textStandard w-6 h-6 self-end mb-0.5"
            onClick={openFileDialog}
            title="Attach File"
          >
            <Attach className="w-4 h-4" />
          </Button>
          {isLoading ? (
            <Button
              type="button"
              size="icon"
              variant="ghost"
              onClick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                onStop && onStop();
              }}
              className="text-textSubtle hover:text-textStandard w-6 h-6 self-end mb-0.5"
              title="Stop Generating"
            >
              <Stop size={20} />
            </Button>
          ) : (
            <Button
              type="button"
              size="icon"
              variant="ghost"
              disabled={isLoading || (!value.trim() && attachedImages.length === 0)}
              onClick={submitData}
              className={`text-textSubtle hover:text-textStandard w-6 h-6 self-end mb-0.5 ${
                isLoading || (!value.trim() && attachedImages.length === 0)
                  ? 'text-textSubtle cursor-not-allowed opacity-50'
                  : ''
              }`}
              title="Send Message"
            >
              <Send className="w-4 h-4" />
            </Button>
          )}
        </div>
      </div>

      {/* Drop Zone Indicator - Style and positioning depend on mode */}
      <div
        className={`flex flex-col items-center justify-center border-2 border-dashed rounded-md text-center cursor-pointer transition-opacity duration-150 border-gray-300 ${
          isOverlayMode
            ? 'absolute top-4 right-4 bottom-4 left-4 z-10 opacity-100' // Overlay mode
            : isCollapseReplaceMode
              ? 'opacity-100 min-h-28' // Collapse/replace mode: visible, normal flow, with min-height
              : 'opacity-0 h-0 overflow-hidden p-0 m-0 border-0 pointer-events-none hidden' // Hidden and collapsed
        }`}
      >
        <div>
          <Upload className="w-8 h-8 text-textSubtle mx-auto" />
          <p className="text-textSubtle mt-1">Drop files here to upload into your goose chat</p>
        </div>
      </div>
    </div>
  );
}
