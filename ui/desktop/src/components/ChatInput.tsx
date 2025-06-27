import React, { useRef, useState, useEffect, useMemo } from 'react';
import { Button } from './ui/button';
import type { View } from '../App';
import Stop from './ui/Stop';
import { Attach, Send, Close, Microphone } from './icons';
import { debounce } from 'lodash';
import BottomMenu from './bottom_menu/BottomMenu';
import { LocalMessageStorage } from '../utils/localMessageStorage';
import { Message } from '../types/message';
import { useWhisper } from '../hooks/useWhisper';
import { WaveformVisualizer } from './WaveformVisualizer';
import { toastError } from '../toasts';

interface PastedImage {
  id: string;
  dataUrl: string; // For immediate preview
  filePath?: string; // Path on filesystem after saving
  isLoading: boolean;
  error?: string;
}

// Constants for image handling
const MAX_IMAGES_PER_MESSAGE = 5;
const MAX_IMAGE_SIZE_MB = 5;

interface ChatInputProps {
  handleSubmit: (e: React.FormEvent) => void;
  isLoading?: boolean;
  onStop?: () => void;
  commandHistory?: string[]; // Current chat's message history
  initialValue?: string;
  droppedFiles?: string[];
  setView: (view: View) => void;
  numTokens?: number;
  inputTokens?: number;
  outputTokens?: number;
  hasMessages?: boolean;
  messages?: Message[];
  setMessages: (messages: Message[]) => void;
  sessionCosts?: {
    [key: string]: {
      inputTokens: number;
      outputTokens: number;
      totalCost: number;
    };
  };
}

export default function ChatInput({
  handleSubmit,
  isLoading = false,
  onStop,
  commandHistory = [],
  initialValue = '',
  setView,
  numTokens,
  inputTokens,
  outputTokens,
  droppedFiles = [],
  messages = [],
  setMessages,
  sessionCosts,
}: ChatInputProps) {
  const [_value, setValue] = useState(initialValue);
  const [displayValue, setDisplayValue] = useState(initialValue); // For immediate visual feedback
  const [isFocused, setIsFocused] = useState(false);
  const [pastedImages, setPastedImages] = useState<PastedImage[]>([]);

  // Whisper hook for voice dictation
  const {
    isRecording,
    isTranscribing,
    canUseDictation,
    audioContext,
    analyser,
    startRecording,
    stopRecording,
    recordingDuration,
    estimatedSize,
  } = useWhisper({
    onTranscription: (text) => {
      // Append transcribed text to the current input
      const newValue = displayValue.trim() ? `${displayValue.trim()} ${text}` : text;
      setDisplayValue(newValue);
      setValue(newValue);
      textAreaRef.current?.focus();
    },
    onError: (error) => {
      toastError({
        title: 'Dictation Error',
        msg: error.message,
      });
    },
    onSizeWarning: (sizeMB) => {
      toastError({
        title: 'Recording Size Warning',
        msg: `Recording is ${sizeMB.toFixed(1)}MB. Maximum size is 25MB.`,
      });
    },
  });

  // Update internal value when initialValue changes
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

    // Reset history index when input is cleared
    setHistoryIndex(-1);
    setIsInGlobalHistory(false);
  }, [initialValue]); // Keep only initialValue as a dependency

  // State to track if the IME is composing (i.e., in the middle of Japanese IME input)
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

  const handleRetryImageSave = async (imageId: string) => {
    const imageToRetry = pastedImages.find((img) => img.id === imageId);
    if (!imageToRetry || !imageToRetry.dataUrl) return;

    // Set the image to loading state
    setPastedImages((prev) =>
      prev.map((img) => (img.id === imageId ? { ...img, isLoading: true, error: undefined } : img))
    );

    try {
      const result = await window.electron.saveDataUrlToTemp(imageToRetry.dataUrl, imageId);
      setPastedImages((prev) =>
        prev.map((img) =>
          img.id === result.id
            ? { ...img, filePath: result.filePath, error: result.error, isLoading: false }
            : img
        )
      );
    } catch (err) {
      console.error('Error retrying image save:', err);
      setPastedImages((prev) =>
        prev.map((img) =>
          img.id === imageId
            ? { ...img, error: 'Failed to save image via Electron.', isLoading: false }
            : img
        )
      );
    }
  };

  useEffect(() => {
    if (textAreaRef.current) {
      textAreaRef.current.focus();
    }
  }, []);

  const minHeight = '1rem';
  const maxHeight = 10 * 24;

  // If we have dropped files, add them to the input and update our state.
  useEffect(() => {
    if (processedFilePaths !== droppedFiles && droppedFiles.length > 0) {
      // Append file paths that aren't in displayValue.
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

  // Debounced function to update actual value
  const debouncedSetValue = useMemo(
    () =>
      debounce((value: string) => {
        setValue(value);
      }, 150),
    [setValue]
  );

  // Debounced autosize function
  const debouncedAutosize = useMemo(
    () =>
      debounce((element: HTMLTextAreaElement) => {
        element.style.height = '0px'; // Reset height
        const scrollHeight = element.scrollHeight;
        element.style.height = Math.min(scrollHeight, maxHeight) + 'px';
      }, 150),
    [maxHeight]
  );

  useEffect(() => {
    if (textAreaRef.current) {
      debouncedAutosize(textAreaRef.current);
    }
  }, [debouncedAutosize, displayValue]);

  const handleChange = (evt: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = evt.target.value;
    setDisplayValue(val); // Update display immediately
    debouncedSetValue(val); // Debounce the actual state update
  };

  const handlePaste = async (evt: React.ClipboardEvent<HTMLTextAreaElement>) => {
    const files = Array.from(evt.clipboardData.files || []);
    const imageFiles = files.filter((file) => file.type.startsWith('image/'));

    if (imageFiles.length === 0) return;

    // Check if adding these images would exceed the limit
    if (pastedImages.length + imageFiles.length > MAX_IMAGES_PER_MESSAGE) {
      // Show error message to user
      setPastedImages((prev) => [
        ...prev,
        {
          id: `error-${Date.now()}`,
          dataUrl: '',
          isLoading: false,
          error: `Cannot paste ${imageFiles.length} image(s). Maximum ${MAX_IMAGES_PER_MESSAGE} images per message allowed.`,
        },
      ]);

      // Remove the error message after 3 seconds
      setTimeout(() => {
        setPastedImages((prev) => prev.filter((img) => !img.id.startsWith('error-')));
      }, 3000);

      return;
    }

    evt.preventDefault();

    for (const file of imageFiles) {
      // Check individual file size before processing
      if (file.size > MAX_IMAGE_SIZE_MB * 1024 * 1024) {
        const errorId = `error-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
        setPastedImages((prev) => [
          ...prev,
          {
            id: errorId,
            dataUrl: '',
            isLoading: false,
            error: `Image too large (${Math.round(file.size / (1024 * 1024))}MB). Maximum ${MAX_IMAGE_SIZE_MB}MB allowed.`,
          },
        ]);

        // Remove the error message after 3 seconds
        setTimeout(() => {
          setPastedImages((prev) => prev.filter((img) => img.id !== errorId));
        }, 3000);

        continue;
      }

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
  };

  // Cleanup debounced functions on unmount
  useEffect(() => {
    return () => {
      debouncedSetValue.cancel?.();
      debouncedAutosize.cancel?.();
    };
  }, [debouncedSetValue, debouncedAutosize]);

  // Handlers for composition events, which are crucial for proper IME behavior
  const handleCompositionStart = () => {
    setIsComposing(true);
  };

  const handleCompositionEnd = () => {
    setIsComposing(false);
  };

  const handleHistoryNavigation = (evt: React.KeyboardEvent<HTMLTextAreaElement>) => {
    const isUp = evt.key === 'ArrowUp';
    const isDown = evt.key === 'ArrowDown';

    // Only handle up/down keys with Cmd/Ctrl modifier
    if ((!isUp && !isDown) || !(evt.metaKey || evt.ctrlKey) || evt.altKey || evt.shiftKey) {
      return;
    }

    evt.preventDefault();

    // Get global history once to avoid multiple calls
    const globalHistory = LocalMessageStorage.getRecentMessages() || [];

    // Save current input if we're just starting to navigate history
    if (historyIndex === -1) {
      setSavedInput(displayValue || '');
      setIsInGlobalHistory(commandHistory.length === 0);
    }

    // Determine which history we're using
    const currentHistory = isInGlobalHistory ? globalHistory : commandHistory;
    let newIndex = historyIndex;
    let newValue = '';

    // Handle navigation
    if (isUp) {
      // Moving up through history
      if (newIndex < currentHistory.length - 1) {
        // Still have items in current history
        newIndex = historyIndex + 1;
        newValue = currentHistory[newIndex];
      } else if (!isInGlobalHistory && globalHistory.length > 0) {
        // Switch to global history
        setIsInGlobalHistory(true);
        newIndex = 0;
        newValue = globalHistory[newIndex];
      }
    } else {
      // Moving down through history
      if (newIndex > 0) {
        // Still have items in current history
        newIndex = historyIndex - 1;
        newValue = currentHistory[newIndex];
      } else if (isInGlobalHistory && commandHistory.length > 0) {
        // Switch to chat history
        setIsInGlobalHistory(false);
        newIndex = commandHistory.length - 1;
        newValue = commandHistory[newIndex];
      } else {
        // Return to original input
        newIndex = -1;
        newValue = savedInput;
      }
    }

    // Update display if we have a new value
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
      if (displayValue.trim()) {
        LocalMessageStorage.addMessage(displayValue);
      } else if (validPastedImageFilesPaths.length > 0) {
        LocalMessageStorage.addMessage(validPastedImageFilesPaths.join(' '));
      }

      handleSubmit(
        new CustomEvent('submit', { detail: { value: textToSend } }) as unknown as React.FormEvent
      );

      setDisplayValue('');
      setValue('');
      setPastedImages([]);
      setHistoryIndex(-1);
      setSavedInput('');
      setIsInGlobalHistory(false);
    }
  };

  const handleKeyDown = (evt: React.KeyboardEvent<HTMLTextAreaElement>) => {
    // Handle history navigation first
    handleHistoryNavigation(evt);

    if (evt.key === 'Enter') {
      // should not trigger submit on Enter if it's composing (IME input in progress) or shift/alt(option) is pressed
      if (evt.shiftKey || isComposing) {
        // Allow line break for Shift+Enter, or during IME composition
        return;
      }

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
        <div className="relative">
          <textarea
            data-testid="chat-input"
            autoFocus
            id="dynamic-textarea"
            placeholder={isRecording ? '' : 'What can goose help with?   ⌘↑/⌘↓'}
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
              opacity: isRecording ? 0 : 1,
            }}
            className="w-full pl-4 pr-[108px] outline-none border-none focus:ring-0 bg-transparent pt-3 pb-1.5 text-sm resize-none text-textStandard placeholder:text-textPlaceholder"
          />
          {isRecording && (
            <div className="absolute inset-0 flex items-center pl-4 pr-[108px] pt-3 pb-1.5">
              <WaveformVisualizer
                audioContext={audioContext}
                analyser={analyser}
                isRecording={isRecording}
              />
            </div>
          )}
        </div>

        {pastedImages.length > 0 && (
          <div className="flex flex-wrap gap-2 p-2 border-t border-borderSubtle">
            {pastedImages.map((img) => (
              <div key={img.id} className="relative group w-20 h-20">
                {img.dataUrl && (
                  <img
                    src={img.dataUrl} // Use dataUrl for instant preview
                    alt={`Pasted image ${img.id}`}
                    className={`w-full h-full object-cover rounded border ${img.error ? 'border-red-500' : 'border-borderStandard'}`}
                  />
                )}
                {img.isLoading && (
                  <div className="absolute inset-0 flex items-center justify-center bg-black bg-opacity-50 rounded">
                    <div className="animate-spin rounded-full h-6 w-6 border-t-2 border-b-2 border-white"></div>
                  </div>
                )}
                {img.error && !img.isLoading && (
                  <div className="absolute inset-0 flex flex-col items-center justify-center bg-black bg-opacity-75 rounded p-1 text-center">
                    <p className="text-red-400 text-[10px] leading-tight break-all mb-1">
                      {img.error.substring(0, 50)}
                    </p>
                    {img.dataUrl && (
                      <button
                        type="button"
                        onClick={() => handleRetryImageSave(img.id)}
                        className="bg-blue-600 hover:bg-blue-700 text-white rounded px-1 py-0.5 text-[8px] leading-none"
                        title="Retry saving image"
                      >
                        Retry
                      </button>
                    )}
                  </div>
                )}
                {!img.isLoading && (
                  <button
                    type="button"
                    onClick={() => handleRemovePastedImage(img.id)}
                    className="absolute -top-1 -right-1 bg-gray-700 hover:bg-red-600 text-white rounded-full w-5 h-5 flex items-center justify-center text-xs leading-none opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity z-10"
                    aria-label="Remove image"
                  >
                    <Close className="w-3.5 h-3.5" />
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
          <>
            {/* Microphone button - only show if dictation is enabled and configured */}
            {canUseDictation && (
              <>
                <Button
                  type="button"
                  size="icon"
                  variant="ghost"
                  onClick={() => {
                    if (isRecording) {
                      stopRecording();
                    } else {
                      startRecording();
                    }
                  }}
                  disabled={isTranscribing}
                  className={`absolute right-12 top-2 transition-colors rounded-full w-7 h-7 [&_svg]:size-4 ${
                    isRecording
                      ? 'bg-red-500 text-white hover:bg-red-600'
                      : isTranscribing
                        ? 'text-textSubtle cursor-not-allowed animate-pulse'
                        : 'text-textSubtle hover:text-textStandard'
                  }`}
                  title={
                    isRecording
                      ? `Stop recording (${Math.floor(recordingDuration)}s, ~${estimatedSize.toFixed(1)}MB)`
                      : isTranscribing
                        ? 'Transcribing...'
                        : 'Start dictation'
                  }
                >
                  <Microphone />
                </Button>
                {/* Recording/transcribing status indicator - positioned above the input */}
                {(isRecording || isTranscribing) && (
                  <div className="absolute right-0 -top-8 bg-bgApp px-2 py-1 rounded text-xs whitespace-nowrap shadow-md border border-borderSubtle">
                    {isTranscribing ? (
                      <span className="text-blue-500 flex items-center gap-1">
                        <span className="inline-block w-2 h-2 bg-blue-500 rounded-full animate-pulse" />
                        Transcribing...
                      </span>
                    ) : (
                      <span
                        className={`flex items-center gap-2 ${estimatedSize > 20 ? 'text-orange-500' : 'text-textSubtle'}`}
                      >
                        <span className="inline-block w-2 h-2 bg-red-500 rounded-full animate-pulse" />
                        {Math.floor(recordingDuration)}s • ~{estimatedSize.toFixed(1)}MB
                        {estimatedSize > 20 && <span className="text-xs">(near 25MB limit)</span>}
                      </span>
                    )}
                  </div>
                )}
              </>
            )}
            <Button
              type="submit"
              size="icon"
              variant="ghost"
              disabled={
                !hasSubmittableContent || isAnyImageLoading || isRecording || isTranscribing
              }
              className={`absolute right-3 top-2 transition-colors rounded-full w-7 h-7 [&_svg]:size-4 ${
                !hasSubmittableContent || isAnyImageLoading || isRecording || isTranscribing
                  ? 'text-textSubtle cursor-not-allowed'
                  : 'bg-bgAppInverse text-textProminentInverse hover:cursor-pointer'
              }`}
              title={
                isAnyImageLoading
                  ? 'Waiting for images to save...'
                  : isRecording
                    ? 'Recording...'
                    : isTranscribing
                      ? 'Transcribing...'
                      : 'Send'
              }
            >
              <Send />
            </Button>
          </>
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
            inputTokens={inputTokens}
            outputTokens={outputTokens}
            messages={messages}
            isLoading={isLoading}
            setMessages={setMessages}
            sessionCosts={sessionCosts}
          />
        </div>
      </div>
    </div>
  );
}
