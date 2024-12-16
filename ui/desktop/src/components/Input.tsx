import React, { useRef, useState, useEffect } from 'react';
import { Button } from './ui/button';
import Send from './ui/Send';
import Stop from './ui/Stop';
import { Paperclip } from 'lucide-react';
import ImagePreview from './ImagePreview';

interface CustomSubmitEvent extends CustomEvent {
  detail: {
    value: string;
    image?: string;
  };
}

interface InputProps {
  handleSubmit: (e: CustomSubmitEvent) => void;
  disabled?: boolean;
  isLoading?: boolean;
  onStop?: () => void;
}

const getImageData = (file: File): Promise<string> => {
  return new Promise((resolve) => {
    const reader = new FileReader();
    reader.onload = (e) => {
      resolve(e.target?.result as string);
    };
    reader.readAsDataURL(file);
  });
};

export default function Input({
  handleSubmit,
  disabled = false,
  isLoading = false,
  onStop
}: InputProps) {
  const [value, setValue] = useState('');
  const [pastedImage, setPastedImage] = useState<string | null>(null);
  const textAreaRef = useRef<HTMLTextAreaElement>(null);

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
      if (value.trim() || pastedImage) {
        handleSubmit(new CustomEvent('submit', { detail: { value, image: pastedImage } }) as CustomSubmitEvent);
        setValue('');
        setPastedImage(null);
      }
    }
  };

  const onFormSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (value.trim() || pastedImage) {
      handleSubmit(new CustomEvent('submit', { detail: { value, image: pastedImage } }) as CustomSubmitEvent);
      setValue('');
      setPastedImage(null);
    }
  };

  const handlePaste = async (e: React.ClipboardEvent) => {
    const items = e.clipboardData?.items;
    
    if (items) {
      for (let i = 0; i < items.length; i++) {
        if (items[i].type.indexOf('image') !== -1) {
          const blob = items[i].getAsFile();
          if (blob) {
            try {
              const imageData = await getImageData(blob);
              setPastedImage(imageData);
            } catch (error) {
              console.error('Error processing image:', error);
            }
          }
          break;
        }
      }
    }
  };

  const handleFileSelect = async () => {
    const path = await window.electron.selectFileOrDirectory();
    if (path) {
      setValue(path);
      textAreaRef.current?.focus();
    }
  };

  return (
    <div className="flex flex-col bg-white dark:bg-gray-800 px-[16px] py-[1rem]">
      {pastedImage && (
        <div className="mb-2">
          <ImagePreview
            imageData={pastedImage}
            onRemove={() => setPastedImage(null)}
          />
        </div>
      )}
      <form onSubmit={onFormSubmit} className="flex relative h-auto pr-[68px]">
        <textarea
          autoFocus
          id="dynamic-textarea"
          placeholder="What should goose do?"
          value={value}
          onChange={handleChange}
          onKeyDown={handleKeyDown}
          onPaste={handlePaste}
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
          }`}
        />
        <Button
          type="button"
          size="icon"
          variant="ghost"
          onClick={handleFileSelect}
          disabled={disabled}
          className={`absolute right-[40px] top-1/2 -translate-y-1/2 text-indigo-600 dark:text-indigo-300 hover:text-indigo-700 dark:hover:text-indigo-200 hover:bg-indigo-100 dark:hover:bg-indigo-800 ${
            disabled ? 'opacity-50 cursor-not-allowed' : ''
          }`}
        >
          <Paperclip size={20} />
        </Button>
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
            disabled={disabled || (!value.trim() && !pastedImage)}
            className={`absolute right-2 top-1/2 -translate-y-1/2 text-indigo-600 dark:text-indigo-300 hover:text-indigo-700 dark:hover:text-indigo-200 hover:bg-indigo-100 dark:hover:bg-indigo-800 ${
              disabled || (!value.trim() && !pastedImage) ? 'opacity-50 cursor-not-allowed' : ''
            }`}
          >
            <Send size={24} />
          </Button>
        )}
      </form>
    </div>
  );
}