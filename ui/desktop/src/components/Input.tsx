import React, { useRef, useState, useEffect, useCallback } from 'react';
import { Button } from './ui/button';
import Stop from './ui/Stop';
import { Attach, Send, Close } from './icons';

interface InputProps {
  handleSubmit: (e: React.FormEvent | CustomEvent<{ value?: string; images?: string[] }>) => void;
  isLoading?: boolean;
  onStop?: () => void;
  commandHistory?: string[];
  initialValue?: string;
}

export default function Input({
  handleSubmit,
  isLoading = false,
  onStop,
  commandHistory = [],
  initialValue = '',
}: InputProps) {
  const [value, setValue] = useState(initialValue);
  const [attachedImages, setAttachedImages] = useState<string[]>([]);

  useEffect(() => {
    setValue(initialValue);
    if (!initialValue) {
      setAttachedImages([]);
    }
  }, [initialValue]);

  const [isComposing, setIsComposing] = useState(false);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [savedInput, setSavedInput] = useState('');
  const textAreaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (textAreaRef.current) {
      textAreaRef.current.focus();
    }
  }, []);

  const autosizeTextArea = (textArea: HTMLTextAreaElement | null, value: string) => {
    if (textArea) {
      textArea.style.height = '0px';
      const scrollHeight = textArea.scrollHeight;
      textArea.style.height = Math.min(scrollHeight, maxHeight) + 'px';
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

  const handleCompositionStart = (evt: React.CompositionEvent<HTMLTextAreaElement>) => {
    setIsComposing(true);
  };

  const handleCompositionEnd = (evt: React.CompositionEvent<HTMLTextAreaElement>) => {
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
      let detail: { value?: string; images?: string[] } = {};
      if (value.trim()) {
        detail.value = value;
      }
      if (attachedImages.length > 0) {
        detail.images = attachedImages;
      }
      handleSubmit(new CustomEvent('submit', { detail }));
      setValue('');
      setAttachedImages([]);
      setHistoryIndex(-1);
      setSavedInput('');
    }
  };

  const onFormSubmit = (e?: React.FormEvent) => {
    e?.preventDefault();
    submitData();
  };

  const handleFileSelect = async () => {
    const path = await window.electron.selectFileOrDirectory();
    if (path) {
      setValue((prev) => {
        const currentText = prev.trim();
        return currentText ? `${currentText} ${path}` : path;
      });
      setAttachedImages([]);
      textAreaRef.current?.focus();
    }
  };

  const handlePaste = (evt: React.ClipboardEvent<HTMLTextAreaElement>) => {
    const items = evt.clipboardData.items;
    let imageFound = false;

    for (let i = 0; i < items.length; i++) {
      if (items[i].type.indexOf('image') !== -1) {
        imageFound = true;
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

    if (imageFound) {
      evt.preventDefault();
    }
  };

  const removeAttachedImage = (indexToRemove: number) => {
    setAttachedImages((prevImages) => prevImages.filter((_, index) => index !== indexToRemove));
    textAreaRef.current?.focus();
  };

  return (
    <div className="flex flex-col border-t border-borderSubtle">
      <form onSubmit={onFormSubmit} className="flex relative h-auto px-[16px] pr-[68px] py-[1rem]">
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
          className="w-full outline-none border-none focus:ring-0 bg-transparent p-0 text-base resize-none text-textStandard"
        />
        <Button
          type="button"
          size="icon"
          variant="ghost"
          onClick={handleFileSelect}
          className="absolute right-[40px] top-1/2 -translate-y-1/2 text-textSubtle hover:text-textStandard"
          title="Attach File"
        >
          <Attach />
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
            className="absolute right-2 top-1/2 -translate-y-1/2 [&_svg]:size-5 text-textSubtle hover:text-textStandard"
            title="Stop Generating"
          >
            <Stop size={24} />
          </Button>
        ) : (
          <Button
            type="submit"
            size="icon"
            variant="ghost"
            disabled={isLoading || (!value.trim() && attachedImages.length === 0)}
            onClick={submitData}
            className={`absolute right-2 top-1/2 -translate-y-1/2 text-textSubtle hover:text-textStandard ${
              isLoading || (!value.trim() && attachedImages.length === 0)
                ? 'text-textSubtle cursor-not-allowed'
                : ''
            }`}
            title="Send Message"
          >
            <Send />
          </Button>
        )}
      </form>
      {attachedImages.length > 0 && (
        <div className="px-[16px] pb-2 flex flex-wrap gap-2">
          {attachedImages.map((imageBase64, index) => (
            <div key={index} className="relative max-w-xs">
              <img
                src={imageBase64}
                alt={`Pasted image preview ${index + 1}`}
                className="max-h-24 w-auto rounded border border-borderSubtle object-contain"
              />
              <Button
                type="button"
                size="icon"
                variant="ghost"
                onClick={() => removeAttachedImage(index)}
                className="absolute top-0 left-0 m-1 p-0.5 bg-black/50 text-white rounded-full hover:bg-black/75 w-5 h-5 flex items-center justify-center"
                title={`Remove Image ${index + 1}`}
              >
                <Close className="w-3 h-3" />
              </Button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
