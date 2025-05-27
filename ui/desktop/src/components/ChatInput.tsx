import React, { useRef, useState, useEffect, useCallback } from 'react';
import { Button } from './ui/button';
import type { View } from '../App';
import Stop from './ui/Stop';
import { Attach, Send, Close } from './icons'; // Replaced XCircle with Close, removed AlertTriangle
import { debounce } from 'lodash';
import BottomMenu from './bottom_menu/BottomMenu';
import { LocalMessageStorage } from '../utils/localMessageStorage';
import { Message } from '../types/message';

// Define the electron API exposed in preload (ensure your preload script matches this)
declare global {
  interface Window {
    electron: {
      saveDataUrlToTemp: (
        dataUrl: string,
        uniqueId: string
      ) => Promise<{ id: string; filePath?: string; error?: string }>;
      deleteTempFile: (filePath: string) => void;
      selectFileOrDirectory: () => Promise<string | null>;
    };
  }
}

interface PastedImage {
  id: string;
  dataUrl: string; // For immediate preview
  filePath?: string; // Path on filesystem after saving
  isLoading: boolean;
  error?: string;
}

interface ChatInputProps {
  handleSubmit: (e: React.FormEvent) => void;
  isLoading?: boolean;
  onStop?: () => void;
  commandHistory?: string[];
  initialValue?: string;
  droppedFiles?: string[];
  setView: (view: View) => void;
  numTokens?: number;
  hasMessages?: boolean;
  messages?: Message[];
  setMessages: (messages: Message[]) => void;
}

export default function ChatInput({
  handleSubmit,
  isLoading = false,
  onStop,
  commandHistory = [],
  initialValue = '',
  setView,
  numTokens,
  droppedFiles = [],
  messages = [],
  setMessages,
}: ChatInputProps) {
  const [_value, setValue] = useState(initialValue);
  const [displayValue, setDisplayValue] = useState(initialValue);
  const [isFocused, setIsFocused] = useState(false);
  const [pastedImages, setPastedImages] = useState<PastedImage[]>([]);

  useEffect(() => {
    setValue(initialValue);
    setDisplayValue(initialValue);

    // Use a functional update to get the current pastedImages
    // and perform cleanup. This avoids needing pastedImages in the deps.
    setPastedImages((currentPastedImages) => {
      currentPastedImages.forEach((img) => {
        if (img.filePath) {
          window.electron.deleteTempFile(img.filePath);
        }
      });
      return []; // Return a new empty array
    });

    setHistoryIndex(-1);
    setIsInGlobalHistory(false);
  }, [initialValue]); // Keep only initialValue as a dependency

  const [isComposing, setIsComposing] = useState(false);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [savedInput, setSavedInput] = useState('');
  const [isInGlobalHistory, setIsInGlobalHistory] = useState(false);
  const textAreaRef = useRef<HTMLTextAreaElement>(null);
  const [processedFilePaths, setProcessedFilePaths] = useState<string[]>([]);

  const handleRemovePastedImage = (idToRemove: string) => {
    const imageToRemove = pastedImages.find((img) => img.id === idToRemove);
    if (imageToRemove?.filePath) {
      window.electron.deleteTempFile(imageToRemove.filePath);
    }
    setPastedImages((currentImages) => currentImages.filter((img) => img.id !== idToRemove));
  };

  useEffect(() => {
    if (textAreaRef.current) {
      textAreaRef.current.focus();
    }
  }, []);

  const minHeight = '1rem';
  const maxHeight = 10 * 24;

  useEffect(() => {
    if (processedFilePaths !== droppedFiles && droppedFiles.length > 0) {
      const currentText = displayValue || '';
      const joinedPaths = currentText.trim()
        ? `${currentText.trim()} ${droppedFiles.filter((path) => !currentText.includes(path)).join(' ')}`
        : droppedFiles.join(' ');
      setDisplayValue(joinedPaths);
      setValue(joinedPaths);
      textAreaRef.current?.focus();
      setProcessedFilePaths(droppedFiles);
    }
  }, [droppedFiles, processedFilePaths, displayValue]);

  const debouncedSetValue = useCallback((val: string) => {
    debounce((value: string) => {
      setValue(value);
    }, 150)(val);
  }, []);

  const debouncedAutosize = useCallback(
    (textArea: HTMLTextAreaElement) => {
      debounce((element: HTMLTextAreaElement) => {
        element.style.height = '0px';
        const scrollHeight = element.scrollHeight;
        element.style.height = Math.min(scrollHeight, maxHeight) + 'px';
      }, 150)(textArea);
    },
    [maxHeight]
  );

  useEffect(() => {
    if (textAreaRef.current) {
      debouncedAutosize(textAreaRef.current);
    }
  }, [debouncedAutosize, displayValue]);

  const handleChange = (evt: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = evt.target.value;
    setDisplayValue(val);
    debouncedSetValue(val);
  };

  const handlePaste = async (evt: React.ClipboardEvent<HTMLTextAreaElement>) => {
    const files = Array.from(evt.clipboardData.files || []);
    for (const file of files) {
      if (file.type.startsWith('image/')) {
        evt.preventDefault();
        const reader = new FileReader();
        reader.onload = async (e) => {
          const dataUrl = e.target?.result as string;
          if (dataUrl) {
            const imageId = `img-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
            setPastedImages((prev) => [...prev, { id: imageId, dataUrl, isLoading: true }]);

            try {
              const result = await window.electron.saveDataUrlToTemp(dataUrl, imageId);
              setPastedImages((prev) =>
                prev.map((img) =>
                  img.id === result.id
                    ? { ...img, filePath: result.filePath, error: result.error, isLoading: false }
                    : img
                )
              );
            } catch (err) {
              console.error('Error saving pasted image:', err);
              setPastedImages((prev) =>
                prev.map((img) =>
                  img.id === imageId
                    ? { ...img, error: 'Failed to save image via Electron.', isLoading: false }
                    : img
                )
              );
            }
          }
        };
        reader.readAsDataURL(file);
      }
    }
  };

  useEffect(() => {
    return () => {
      debouncedSetValue.cancel?.();
      debouncedAutosize.cancel?.();
      // Cleanup any remaining temp files if component unmounts unexpectedly
      // This is a fallback; primary cleanup is on remove/submit or app quit
      pastedImages.forEach((img) => {
        if (img.filePath) {
          // window.electron.deleteTempFile(img.filePath); // Be cautious with this on HMR
        }
      });
    };
  }, [debouncedSetValue, debouncedAutosize, pastedImages]);

  const handleCompositionStart = () => setIsComposing(true);
  const handleCompositionEnd = () => setIsComposing(false);

  const handleHistoryNavigation = (evt: React.KeyboardEvent<HTMLTextAreaElement>) => {
    // ... (history navigation logic remains the same)
    const isUp = evt.key === 'ArrowUp';
    const isDown = evt.key === 'ArrowDown';

    if ((!isUp && !isDown) || !(evt.metaKey || evt.ctrlKey) || evt.altKey || evt.shiftKey) {
      return;
    }
    evt.preventDefault();
    const globalHistory = LocalMessageStorage.getRecentMessages() || [];
    if (historyIndex === -1) {
      setSavedInput(displayValue || '');
      setIsInGlobalHistory(commandHistory.length === 0);
    }
    const currentHistory = isInGlobalHistory ? globalHistory : commandHistory;
    let newIndex = historyIndex;
    let newValue = '';
    if (isUp) {
      if (newIndex < currentHistory.length - 1) {
        newIndex = historyIndex + 1;
        newValue = currentHistory[newIndex];
      } else if (!isInGlobalHistory && globalHistory.length > 0) {
        setIsInGlobalHistory(true);
        newIndex = 0;
        newValue = globalHistory[newIndex];
      }
    } else {
      if (newIndex > 0) {
        newIndex = historyIndex - 1;
        newValue = currentHistory[newIndex];
      } else if (isInGlobalHistory && commandHistory.length > 0) {
        setIsInGlobalHistory(false);
        newIndex = commandHistory.length - 1;
        newValue = commandHistory[newIndex];
      } else {
        newIndex = -1;
        newValue = savedInput;
      }
    }
    if (newIndex !== historyIndex) {
      setHistoryIndex(newIndex);
      if (newIndex === -1) {
        setDisplayValue(savedInput || '');
        setValue(savedInput || '');
      } else {
        setDisplayValue(newValue || '');
        setValue(newValue || '');
      }
    }
  };

  const performSubmit = () => {
    const validPastedImageFilesPaths = pastedImages
      .filter((img) => img.filePath && !img.error && !img.isLoading)
      .map((img) => img.filePath as string);

    let textToSend = displayValue.trim();

    if (validPastedImageFilesPaths.length > 0) {
      const pathsString = validPastedImageFilesPaths.join(' ');
      textToSend = textToSend ? `${textToSend} ${pathsString}` : pathsString;
    }

    if (textToSend) {
      // Only submit if there's some content
      // Log original displayValue to history if it had text,
      // otherwise log the paths if only images were present.
      if (displayValue.trim()) {
        LocalMessageStorage.addMessage(displayValue);
      } else if (validPastedImageFilesPaths.length > 0) {
        LocalMessageStorage.addMessage(validPastedImageFilesPaths.join(' '));
      }

      // Send ONLY the combined text string in detail.value
      handleSubmit(new CustomEvent('submit', { detail: { value: textToSend } }));

      setDisplayValue('');
      setValue('');
      // Decide on temp file cleanup strategy. For now, rely on explicit removal or app quit.
      // pastedImages.filter(img => img.filePath).forEach(img => window.electron.deleteTempFile(img.filePath!));
      setPastedImages([]);
      setHistoryIndex(-1);
      setSavedInput('');
      setIsInGlobalHistory(false);
    }
  };

  const handleKeyDown = (evt: React.KeyboardEvent<HTMLTextAreaElement>) => {
    handleHistoryNavigation(evt);
    if (evt.key === 'Enter') {
      if (evt.shiftKey || isComposing) return;
      if (evt.altKey) {
        const newValue = displayValue + '\n';
        setDisplayValue(newValue);
        setValue(newValue);
        return;
      }
      evt.preventDefault();
      const canSubmit =
        !isLoading &&
        (displayValue.trim() ||
          pastedImages.some((img) => img.filePath && !img.error && !img.isLoading));
      if (canSubmit) {
        performSubmit();
      }
    }
  };

  const onFormSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const canSubmit =
      !isLoading &&
      (displayValue.trim() ||
        pastedImages.some((img) => img.filePath && !img.error && !img.isLoading));
    if (canSubmit) {
      performSubmit();
    }
  };

  const handleFileSelect = async () => {
    const path = await window.electron.selectFileOrDirectory();
    if (path) {
      const newValue = displayValue.trim() ? `${displayValue.trim()} ${path}` : path;
      setDisplayValue(newValue);
      setValue(newValue);
      textAreaRef.current?.focus();
    }
  };

  const hasSubmittableContent =
    displayValue.trim() || pastedImages.some((img) => img.filePath && !img.error && !img.isLoading);
  const isAnyImageLoading = pastedImages.some((img) => img.isLoading);

  return (
    <div
      className={`flex flex-col relative h-auto border rounded-lg transition-colors ${
        isFocused
          ? 'border-borderProminent hover:border-borderProminent'
          : 'border-borderSubtle hover:border-borderStandard'
      } bg-bgApp z-10`}
    >
      <form onSubmit={onFormSubmit}>
        <textarea
          data-testid="chat-input"
          autoFocus
          id="dynamic-textarea"
          placeholder="What can goose help with?   ⌘↑/⌘↓"
          value={displayValue}
          onChange={handleChange}
          onCompositionStart={handleCompositionStart}
          onCompositionEnd={handleCompositionEnd}
          onKeyDown={handleKeyDown}
          onPaste={handlePaste}
          onFocus={() => setIsFocused(true)}
          onBlur={() => setIsFocused(false)}
          ref={textAreaRef}
          rows={1}
          style={{
            minHeight: `${minHeight}px`,
            maxHeight: `${maxHeight}px`,
            overflowY: 'auto',
          }}
          className="w-full pl-4 pr-[68px] outline-none border-none focus:ring-0 bg-transparent pt-3 pb-1.5 text-sm resize-none text-textStandard placeholder:text-textPlaceholder"
        />

        {pastedImages.length > 0 && (
          <div className="flex flex-wrap gap-2 p-2 border-t border-borderSubtle">
            {pastedImages.map((img) => (
              <div key={img.id} className="relative group w-20 h-20">
                <img
                  src={img.dataUrl} // Use dataUrl for instant preview
                  alt={`Pasted image ${img.id}`}
                  className={`w-full h-full object-cover rounded border ${img.error ? 'border-red-500' : 'border-borderStandard'}`}
                />
                {img.isLoading && (
                  <div className="absolute inset-0 flex items-center justify-center bg-black bg-opacity-50 rounded">
                    <div className="animate-spin rounded-full h-6 w-6 border-t-2 border-b-2 border-white"></div>
                  </div>
                )}
                {img.error && !img.isLoading && (
                  <div className="absolute inset-0 flex flex-col items-center justify-center bg-black bg-opacity-75 rounded p-1 text-center">
                    {/* <AlertTriangle className="w-5 h-5 text-red-400 mb-0.5" /> */}
                    <p className="text-red-400 text-[10px] leading-tight break-all">
                      {img.error.substring(0, 30)}
                    </p>
                  </div>
                )}
                {!img.isLoading && (
                  <button
                    type="button"
                    onClick={() => handleRemovePastedImage(img.id)}
                    className="absolute -top-1 -right-1 bg-gray-700 hover:bg-red-600 text-white rounded-full w-5 h-5 flex items-center justify-center text-xs leading-none opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity z-10"
                    aria-label="Remove image"
                  >
                    <Close size={14} />
                  </button>
                )}
              </div>
            ))}
          </div>
        )}

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
            className="absolute right-3 top-2 text-textSubtle rounded-full border border-borderSubtle hover:border-borderStandard hover:text-textStandard w-7 h-7 [&_svg]:size-4"
          >
            <Stop size={24} />
          </Button>
        ) : (
          <Button
            type="submit"
            size="icon"
            variant="ghost"
            disabled={!hasSubmittableContent || isAnyImageLoading} // Disable if no content or if images are still loading/saving
            className={`absolute right-3 top-2 transition-colors rounded-full w-7 h-7 [&_svg]:size-4 ${
              !hasSubmittableContent || isAnyImageLoading
                ? 'text-textSubtle cursor-not-allowed'
                : 'bg-bgAppInverse text-textProminentInverse hover:cursor-pointer'
            }`}
            title={isAnyImageLoading ? 'Waiting for images to save...' : 'Send'}
          >
            <Send />
          </Button>
        )}
      </form>

      <div className="flex items-center transition-colors text-textSubtle relative text-xs p-2 pr-3 border-t border-borderSubtle gap-2">
        <div className="gap-1 flex items-center justify-between w-full">
          <Button
            type="button"
            size="icon"
            variant="ghost"
            onClick={handleFileSelect}
            className="text-textSubtle hover:text-textStandard w-7 h-7 [&_svg]:size-4"
          >
            <Attach />
          </Button>

          <BottomMenu
            setView={setView}
            numTokens={numTokens}
            messages={messages}
            isLoading={isLoading}
            setMessages={setMessages}
          />
        </div>
      </div>
    </div>
  );
}
