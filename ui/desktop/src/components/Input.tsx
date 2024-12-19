import React, { useRef, useState, useEffect } from 'react';
import { Button } from './ui/button';
import Send from './ui/Send';
import Stop from './ui/Stop';
import { Paperclip } from 'lucide-react';
import ImagePreview from './ImagePreview';
import { getImageData, compressImage } from '../utils/imageUtils';

interface CustomSubmitEvent extends CustomEvent {
  detail: {
    value: string;
    image?: {
      preview: string;
      compressed: string;
      path?: string;
    };
  };
}

interface InputProps {
  handleSubmit: (e: CustomSubmitEvent) => void;
  disabled?: boolean;
  isLoading?: boolean;
  onStop?: () => void;
}

interface PastedImage {
  preview: string;
  compressed: string;
  path?: string;
}

declare global {
  interface Window {
    electron: {
      selectFileOrDirectory: () => Promise<string | null>;
      saveTemporaryImage: (imageData: string) => Promise<string>;
    };
    appConfig: {
      get: (key: string) => any;
    };
  }
}

export default function Input({
  handleSubmit,
  disabled = false,
  isLoading = false,
  onStop,
}: InputProps) {
  const [value, setValue] = useState('');
  const [pastedImage, setPastedImage] = useState<PastedImage | null | undefined>(null);
  const textAreaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (textAreaRef.current && !disabled) {
      textAreaRef.current.focus();
    }
  }, [disabled, value]);

  const useAutosizeTextArea = (textAreaRef: HTMLTextAreaElement | null, value: string) => {
    useEffect(() => {
      if (textAreaRef) {
        textAreaRef.style.height = '0px'; // Reset height
        const scrollHeight = textAreaRef.scrollHeight;
        textAreaRef.style.height = Math.min(scrollHeight, maxHeight) + 'px';
      }
    }, [textAreaRef, value]);
  };

  const minHeight = '1rem';
  const maxHeight = 10 * 24;

  useAutosizeTextArea(textAreaRef.current, value);

  const handleChange = (evt: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = evt.target?.value;
    setValue(val);
  };

  const handlePaste = async (e: React.ClipboardEvent) => {
    const items = e.clipboardData?.items;
    if (!items) return;

    for (const item of items) {
      if (item.type.indexOf('image') === 0) {
        const blob = item.getAsFile();
        if (!blob) continue;

        const previewBase64 = await getImageData(blob);
        const compressedBase64 = await compressImage(previewBase64);
        
        // Save to temp file if available
        let filePath: string | undefined;
        try {
          if (window.electron?.saveTemporaryImage) {
            filePath = await window.electron.saveTemporaryImage(compressedBase64);
          }
        } catch (error) {
          console.error('Failed to save temp image:', error);
        }
        
        setPastedImage({
          preview: previewBase64,
          compressed: compressedBase64,
          path: filePath  // Include file path if available
        });
        break;
      }
    }
  };

  const handleSubmitForm = (e: React.FormEvent) => {
    e.preventDefault();
    if (!value.trim() && !pastedImage) return;

    const detail = {
      value: value.trim(),
      image: pastedImage || undefined
    };

    handleSubmit(new CustomEvent('submit', { detail }) as CustomSubmitEvent);
    setValue('');
    setPastedImage(null);
  };

  const handleKeyDown = (evt: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (evt.key === 'Enter' && !evt.shiftKey) {
      evt.preventDefault();
      if (value.trim() || pastedImage) {
        const detail = {
          value: value.trim(),
          image: pastedImage || undefined
        };

        handleSubmit(new CustomEvent('submit', { detail }) as CustomSubmitEvent);
        setValue('');
        setPastedImage(null);
      }
    }
  };

  const handleFileSelect = async () => {
    const filePath = await window.electron.selectFileOrDirectory();
    if (filePath) {
      try {
        // Create a preview for the selected file
        const file = await window.electron.readFile(filePath);
        const previewUrl = await new Promise<string>((resolve) => {
          const reader = new FileReader();
          reader.onload = (e) => {
            resolve(e.target?.result as string);
          };
          reader.readAsDataURL(new Blob([file]));
        });
        const compressedBase64 = await compressImage(previewUrl);
        
        // Save to temp file if available
        let tempFilePath: string | undefined;
        try {
          if (window.electron?.saveTemporaryImage) {
            tempFilePath = await window.electron.saveTemporaryImage(compressedBase64);
          }
        } catch (error) {
          console.error('Failed to save temp image:', error);
        }

        setPastedImage({
          preview: previewUrl,
          compressed: compressedBase64,
          path: tempFilePath  // Include file path if available
        });
      } catch (error) {
        console.error('Error creating preview:', error);
        setValue(filePath); // Fallback to just showing the path
      }
      textAreaRef.current?.focus();
    }
  };

  return (
    <div className="flex flex-col px-[16px] py-[1rem]">
      {pastedImage && (
        <div className="mb-4">
          <ImagePreview
            imageData={pastedImage.preview}
            onRemove={() => setPastedImage(null)}
          />
        </div>
      )}
      <form onSubmit={handleSubmitForm} className="flex relative h-auto pr-[68px]">
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
            overflowY: 'auto',
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