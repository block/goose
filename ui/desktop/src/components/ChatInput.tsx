import React, { useRef, useState, useEffect, useCallback } from 'react';
import { Button } from './ui/button';
import Stop from './ui/Stop';
import { Attach, Send } from './icons';
import { debounce } from 'lodash';
import { LocalMessageStorage } from '../utils/localMessageStorage';

interface ChatInputProps {
  handleSubmit: (e: React.FormEvent) => void;
  isLoading?: boolean;
  onStop?: () => void;
  commandHistory?: string[]; // Current chat's message history
  initialValue?: string;
  droppedFiles?: string[];
}

export default function ChatInput({
  handleSubmit,
  isLoading = false,
  onStop,
  commandHistory = [],
  initialValue = '',
  droppedFiles = [],
}: ChatInputProps) {
  const [_value, setValue] = useState(initialValue);
  const [displayValue, setDisplayValue] = useState(initialValue); // For immediate visual feedback

  // Update internal value when initialValue changes
  useEffect(() => {
    setValue(initialValue);
    setDisplayValue(initialValue);
  }, [initialValue]);

  // State to track if the IME is composing (i.e., in the middle of Japanese IME input)
  const [isComposing, setIsComposing] = useState(false);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [savedInput, setSavedInput] = useState('');
  const [isInGlobalHistory, setIsInGlobalHistory] = useState(false);
  const textAreaRef = useRef<HTMLTextAreaElement>(null);
  const [processedFilePaths, setProcessedFilePaths] = useState<string[]>([]);

  useEffect(() => {
    if (textAreaRef.current) {
      textAreaRef.current.focus();
    }
  }, []);

  const minHeight = '1rem';
  const maxHeight = 10 * 24;

  // If we have dropped files, add them to the input and update our state.
  if (processedFilePaths !== droppedFiles) {
    // Append file paths that aren't in displayValue.
    let joinedPaths =
      displayValue.trim() +
      ' ' +
      droppedFiles.filter((path) => !displayValue.includes(path)).join(' ');
    setDisplayValue(joinedPaths);
    setValue(joinedPaths);
    textAreaRef.current?.focus();
    setProcessedFilePaths(droppedFiles);
  }

  // Debounced function to update actual value
  const debouncedSetValue = useCallback((val: string) => {
    debounce((value: string) => {
      setValue(value);
    }, 150)(val);
  }, []);

  // Debounced autosize function
  const debouncedAutosize = useCallback(
    (textArea: HTMLTextAreaElement) => {
      debounce((element: HTMLTextAreaElement) => {
        element.style.height = '0px'; // Reset height
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
    setDisplayValue(val); // Update display immediately
    debouncedSetValue(val); // Debounce the actual state update
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
    const globalHistory = LocalMessageStorage.getRecentMessages();
    console.log('Global history:', globalHistory); // Debug log
    console.log('Chat history:', commandHistory); // Debug log
    console.log('Current index:', historyIndex); // Debug log
    console.log('Is in global:', isInGlobalHistory); // Debug log

    // Save current input if we're just starting to navigate history
    if (historyIndex === -1) {
      setSavedInput(displayValue);
      setIsInGlobalHistory(commandHistory.length === 0);
    }

    // Calculate new history index and determine which history to use
    let newIndex = historyIndex;
    let newValue = '';
    let useGlobalHistory = isInGlobalHistory;

    // If we're in a new chat, always use global history
    if (commandHistory.length === 0) {
      useGlobalHistory = true;
      if (isUp && newIndex < globalHistory.length - 1) {
        newIndex = historyIndex + 1;
        newValue = globalHistory[newIndex];
      } else if (isDown && newIndex > -1) {
        newIndex = historyIndex - 1;
        newValue = newIndex === -1 ? savedInput : globalHistory[newIndex];
      }
    } else {
      // In an existing chat with messages
      if (isUp) {
        if (!useGlobalHistory && newIndex < commandHistory.length - 1) {
          // Still in chat history
          newIndex = historyIndex + 1;
          newValue = commandHistory[newIndex];
        } else if (!useGlobalHistory) {
          // Transition to global history
          useGlobalHistory = true;
          newIndex = 0;
          newValue = globalHistory[newIndex];
        } else if (newIndex < globalHistory.length - 1) {
          // In global history
          newIndex = historyIndex + 1;
          newValue = globalHistory[newIndex];
        }
      } else {
        // Moving down
        if (useGlobalHistory && newIndex > 0) {
          // Still in global history
          newIndex = historyIndex - 1;
          newValue = globalHistory[newIndex];
        } else if (useGlobalHistory) {
          // Transition back to chat history
          useGlobalHistory = false;
          newIndex = commandHistory.length - 1;
          newValue = commandHistory[newIndex];
        } else if (newIndex > 0) {
          // In chat history
          newIndex = historyIndex - 1;
          newValue = commandHistory[newIndex];
        } else {
          // Return to original input
          newIndex = -1;
          newValue = savedInput;
        }
      }
    }

    console.log('New index:', newIndex); // Debug log
    console.log('New value:', newValue); // Debug log
    console.log('Using global:', useGlobalHistory); // Debug log

    // Update state if we found a new value or changed history type
    if (newIndex !== historyIndex || useGlobalHistory !== isInGlobalHistory) {
      setHistoryIndex(newIndex);
      setIsInGlobalHistory(useGlobalHistory);
      if (newIndex === -1) {
        setDisplayValue(savedInput);
        setValue(savedInput);
      } else {
        setDisplayValue(newValue);
        setValue(newValue);
      }
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

      // Prevent default Enter behavior when loading or when not loading but has content
      // So it won't trigger a new line
      evt.preventDefault();

      // Only submit if not loading and has content
      if (!isLoading && displayValue.trim()) {
        // Always add to global chat storage before submitting
        LocalMessageStorage.addMessage(displayValue);

        handleSubmit(new CustomEvent('submit', { detail: { value: displayValue } }));
        setDisplayValue('');
        setValue('');
        setHistoryIndex(-1);
        setSavedInput('');
        setIsInGlobalHistory(false);
      }
    }
  };

  const onFormSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (displayValue.trim() && !isLoading) {
      // Always add to global chat storage before submitting
      LocalMessageStorage.addMessage(displayValue);

      handleSubmit(new CustomEvent('submit', { detail: { value: displayValue } }));
      setDisplayValue('');
      setValue('');
      setHistoryIndex(-1);
      setSavedInput('');
      setIsInGlobalHistory(false);
    }
  };

  const handleFileSelect = async () => {
    const path = await window.electron.selectFileOrDirectory();
    if (path) {
      // Append the path to existing text, with a space if there's existing text
      const newValue = displayValue.trim() ? `${displayValue.trim()} ${path}` : path;
      setDisplayValue(newValue);
      setValue(newValue);
      textAreaRef.current?.focus();
    }
  };

  return (
    <form
      onSubmit={onFormSubmit}
      className="flex relative h-auto px-[16px] pr-[68px] py-[1rem] border-t border-borderSubtle"
    >
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
        ref={textAreaRef}
        rows={1}
        style={{
          minHeight: `${minHeight}px`,
          maxHeight: `${maxHeight}px`,
          overflowY: 'auto',
        }}
        className="w-full outline-none border-none focus:ring-0 bg-transparent p-0 text-base resize-none text-textStandard placeholder:text-textPlaceholder placeholder:opacity-50"
      />
      <Button
        type="button"
        size="icon"
        variant="ghost"
        onClick={handleFileSelect}
        className="absolute right-[40px] top-1/2 -translate-y-1/2 text-textSubtle hover:text-textStandard"
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
            onStop?.();
          }}
          className="absolute right-2 top-1/2 -translate-y-1/2 [&_svg]:size-5 text-textSubtle hover:text-textStandard"
        >
          <Stop size={24} />
        </Button>
      ) : (
        <Button
          type="submit"
          size="icon"
          variant="ghost"
          disabled={!displayValue.trim()}
          className={`absolute right-2 top-1/2 -translate-y-1/2 text-textSubtle hover:text-textStandard ${
            !displayValue.trim() ? 'text-textSubtle cursor-not-allowed' : ''
          }`}
        >
          <Send />
        </Button>
      )}
    </form>
  );
}
