import React, { useRef, useState, useEffect, useCallback } from 'react';
import { Button } from './ui/button';
import type { View } from '../App';
import Stop from './ui/Stop';
import { Attach, Send } from './icons';
import { debounce } from 'lodash';
import BottomMenu from './bottom_menu/BottomMenu';
import { LocalMessageStorage } from '../utils/localMessageStorage';
import { Message } from '../types/message';

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

  // Update internal value when initialValue changes
  useEffect(() => {
    setValue(initialValue);
    setDisplayValue(initialValue);
    // Reset history index when input is cleared
    setHistoryIndex(-1);
    setIsInGlobalHistory(false);
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
          placeholder="What can goose help with? (⌘↑/↓ for history)"
          value={displayValue}
          onChange={handleChange}
          onCompositionStart={handleCompositionStart}
          onCompositionEnd={handleCompositionEnd}
          onKeyDown={handleKeyDown}
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
            disabled={!displayValue.trim()}
            className={`absolute right-3 top-2 transition-colors rounded-full w-7 h-7 [&_svg]:size-4 ${
              !displayValue.trim()
                ? 'text-textSubtle cursor-not-allowed'
                : 'bg-bgAppInverse text-textProminentInverse hover:cursor-pointer'
            }`}
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
