import React, { useRef, useState, useEffect, useMemo, useCallback } from 'react';
import { Button } from './ui/button';
import type { View } from '../App';
import Stop from './ui/Stop';
import { Attach, Send, Close } from './icons';
import { debounce } from 'lodash';
import BottomMenu from './bottom_menu/BottomMenu';
import { LocalMessageStorage } from '../utils/localMessageStorage';
import { Message } from '../types/message';

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
  const [displayValue, setDisplayValue] = useState(initialValue); // For immediate visual feedback
  const [isFocused, setIsFocused] = useState(false);
  const [pastedImages, setPastedImages] = useState<PastedImage[]>([]);

  const [isHistorySearchOpen, setIsHistorySearchOpen] = useState(false);
  const [historySearchQuery, setHistorySearchQuery] = useState('');
  const [historySearchResults, setHistorySearchResults] = useState<string[]>([]);
  const [historySearchIndex, setHistorySearchIndex] = useState(0);
  const historySearchInputRef = useRef<HTMLInputElement>(null);

  // State to track if the IME is composing (i.e., in the middle of Japanese IME input)
  const [isComposing, setIsComposing] = useState(false);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [savedInput, setSavedInput] = useState('');
  const [isInGlobalHistory, setIsInGlobalHistory] = useState(false);
  const textAreaRef = useRef<HTMLTextAreaElement>(null);
  const [processedFilePaths, setProcessedFilePaths] = useState<string[]>([]);

  /**
   * Detect operating system so we can customise keyboard shortcuts where
   * necessary.  Ctrl+R triggers a hard refresh on Windows in Electron, which
   * we cannot reliably intercept.  Instead, we fall back to Ctrl+H (H for
   * "History") on Windows while keeping Ctrl+R on macOS / Linux.
   */
  const isWindows = useMemo(() => typeof navigator !== 'undefined' && navigator.platform.toUpperCase().includes('WIN'), []);
  const isMac = useMemo(() => typeof navigator !== 'undefined' && navigator.platform.toUpperCase().includes('MAC'), []);

  // Human-friendly hints displayed in the textarea placeholder.
  const arrowHint = isMac ? '⌘↑/⌘↓' : 'Ctrl↑/Ctrl↓';
  const searchKeyHint = isWindows ? 'Ctrl+H' : '⌃R';
  const placeholderText = `What can goose help with?   ${arrowHint} ${searchKeyHint} search`;

  const updateInitialValueRef = useRef(initialValue);
  // We add this effect later (after closeHistorySearch) to satisfy dependency order.

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

  const getCombinedHistory = useCallback(() => {
    // Combine current chat history and global history, removing duplicates while preserving order
    const globalHistory = LocalMessageStorage.getRecentMessages();
    const combined = [...commandHistory];
    for (const item of globalHistory) {
      if (!combined.includes(item)) {
        combined.push(item);
      }
    }
    return combined;
  }, [commandHistory]);

  const openHistorySearch = useCallback(() => {
    if (isHistorySearchOpen) return;
    setSavedInput(displayValue);
    setIsHistorySearchOpen(true);
    setHistorySearchQuery('');
    const combined = getCombinedHistory();
    setHistorySearchResults(combined);
    setHistorySearchIndex(0);
    // Focus search input on next tick
    setTimeout(() => {
      historySearchInputRef.current?.focus();
    }, 0);
  }, [displayValue, getCombinedHistory, isHistorySearchOpen]);

  const closeHistorySearch = useCallback(() => {
    setIsHistorySearchOpen(false);
    setHistorySearchQuery('');
    setHistorySearchResults([]);
    setHistorySearchIndex(0);
    setDisplayValue(savedInput);
    setValue(savedInput);
    textAreaRef.current?.focus();
  }, [savedInput]);

  // Sync component when initialValue prop changes (must appear after closeHistorySearch to avoid lint errors)
  useEffect(() => {
    if (updateInitialValueRef.current === initialValue) return;
    updateInitialValueRef.current = initialValue;

    setValue(initialValue);
    setDisplayValue(initialValue);

    // Clean up pasted images
    setPastedImages((currentPastedImages) => {
      currentPastedImages.forEach((img) => {
        if (img.filePath) {
          window.electron.deleteTempFile(img.filePath);
        }
      });
      return [];
    });

    // Reset history and search state
    setHistoryIndex(-1);
    setIsInGlobalHistory(false);
    closeHistorySearch();
  }, [initialValue, closeHistorySearch]);

  const updateHistorySearchResults = useCallback(
    (query: string) => {
      const all = getCombinedHistory();
      let filtered: string[];
      if (!query) {
        filtered = all;
      } else {
        const lower = query.toLowerCase();
        filtered = all.filter((m) => m.toLowerCase().includes(lower));
      }
      setHistorySearchResults(filtered);
      const idx = 0;
      setHistorySearchIndex(idx);
      if (filtered.length > 0) {
        setDisplayValue(filtered[idx]);
        setValue(filtered[idx]);
      } else {
        setDisplayValue(savedInput);
        setValue(savedInput);
      }
    },
    [getCombinedHistory, savedInput]
  );

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
    // Open history search. Use Ctrl+R everywhere except Windows where we
    // switch to Ctrl+H to avoid triggering a full refresh.
    const searchShortcutKey = isWindows ? 'h' : 'r';

    if (!evt.shiftKey && !evt.metaKey && evt.ctrlKey && evt.key.toLowerCase() === searchShortcutKey) {
      evt.preventDefault();
      openHistorySearch();
      return;
    }

    // If search is open, ignore other navigation key handling
    if (isHistorySearchOpen) {
      if (evt.key === 'Escape') {
        evt.preventDefault();
        closeHistorySearch();
      }
      return;
    }

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

  const handleHistorySearchInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const query = e.target.value;
    setHistorySearchQuery(query);
    updateHistorySearchResults(query);
  };

  const handleHistorySearchInputKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      closeHistorySearch();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      closeHistorySearch();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (historySearchResults.length > 0) {
        const newIndex =
          (historySearchIndex - 1 + historySearchResults.length) % historySearchResults.length;
        setHistorySearchIndex(newIndex);
        setDisplayValue(historySearchResults[newIndex]);
        setValue(historySearchResults[newIndex]);
      }
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (historySearchResults.length > 0) {
        const newIndex = (historySearchIndex + 1) % historySearchResults.length;
        setHistorySearchIndex(newIndex);
        setDisplayValue(historySearchResults[newIndex]);
        setValue(historySearchResults[newIndex]);
      }
    }
  };

  // Keep displayValue in sync when search index changes via other means
  useEffect(() => {
    if (isHistorySearchOpen && historySearchResults.length > 0) {
      const current = historySearchResults[historySearchIndex] ?? '';
      setDisplayValue(current);
      setValue(current);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [historySearchIndex]);

  return (
    <div
      className={`flex flex-col relative h-auto border rounded-lg transition-colors ${
        isFocused
          ? 'border-borderProminent hover:border-borderProminent'
          : 'border-borderSubtle hover:border-borderStandard'
      } bg-bgApp z-10`}
    >
      {isHistorySearchOpen && (
        <div className="absolute -top-8 left-0 right-0 flex justify-center pointer-events-none">
          <div className="flex items-center gap-2 bg-bgSubtle text-textStandard rounded px-3 py-1 shadow pointer-events-auto">
            <span className="text-xs text-textSubtle">history:</span>
            <input
              ref={historySearchInputRef}
              value={historySearchQuery}
              onChange={handleHistorySearchInputChange}
              onKeyDown={handleHistorySearchInputKeyDown}
              className="bg-transparent outline-none text-xs w-40 placeholder:text-textPlaceholder"
              placeholder="type to search..."
            />
            {historySearchResults.length > 0 && (
              <span className="text-xs text-textSubtle">
                {historySearchIndex + 1}/{historySearchResults.length}
              </span>
            )}
            <button type="button" onClick={closeHistorySearch} className="ml-1 p-0.5">
              <Close className="h-3 w-3 text-textSubtle" />
            </button>
          </div>
        </div>
      )}
      <form onSubmit={onFormSubmit}>
        <textarea
          data-testid="chat-input"
          autoFocus
          id="dynamic-textarea"
          placeholder={placeholderText}
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
