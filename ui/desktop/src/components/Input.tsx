import React, { useRef, useState, useEffect, useCallback } from 'react';
import { Button } from './ui/button';
import Stop from './ui/Stop';
import { Attach, Send } from './icons';
import { debounce } from 'lodash';

interface InputProps {
  handleSubmit: (e: React.FormEvent) => void;
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
  const [_value, setValue] = useState(initialValue);
  const [displayValue, setDisplayValue] = useState(initialValue); // For immediate visual feedback

  // Update internal value when initialValue changes
  useEffect(() => {
    if (initialValue) {
      setValue(initialValue);
      setDisplayValue(initialValue);
    }
  }, [initialValue]);

  // State to track if the IME is composing (i.e., in the middle of Japanese IME input)
  const [isComposing, setIsComposing] = useState(false);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [savedInput, setSavedInput] = useState('');
  const textAreaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (textAreaRef.current) {
      textAreaRef.current.focus();
    }
  }, []);

  const minHeight = '1rem';
  const maxHeight = 10 * 24;

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
    evt.preventDefault();

    // Save current input if we're just starting to navigate history
    if (historyIndex === -1) {
      setSavedInput(displayValue);
    }

    // Calculate new history index
    let newIndex = historyIndex;
    if (evt.key === 'ArrowUp') {
      // Move backwards through history
      if (historyIndex < commandHistory.length - 1) {
        newIndex = historyIndex + 1;
      }
    } else {
      // Move forwards through history
      if (historyIndex > -1) {
        newIndex = historyIndex - 1;
      }
    }

    if (newIndex === historyIndex) {
      return;
    }

    // Update index and value
    setHistoryIndex(newIndex);
    if (newIndex === -1) {
      // Restore saved input when going past the end of history
      setDisplayValue(savedInput);
      setValue(savedInput);
    } else {
      setDisplayValue(commandHistory[newIndex] || '');
      setValue(commandHistory[newIndex] || '');
    }
  };

  const handleKeyDown = (evt: React.KeyboardEvent<HTMLTextAreaElement>) => {
    // Handle command history navigation
    if ((evt.metaKey || evt.ctrlKey) && (evt.key === 'ArrowUp' || evt.key === 'ArrowDown')) {
      handleHistoryNavigation(evt);
      return;
    }

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
        handleSubmit(new CustomEvent('submit', { detail: { value: displayValue } }));
        setDisplayValue('');
        setValue('');
        setHistoryIndex(-1);
        setSavedInput('');
      }
    }
  };

  const onFormSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (displayValue.trim() && !isLoading) {
      handleSubmit(new CustomEvent('submit', { detail: { value: displayValue } }));
      setDisplayValue('');
      setValue('');
      setHistoryIndex(-1);
      setSavedInput('');
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
        className="w-full outline-none border-none focus:ring-0 bg-transparent p-0 text-base resize-none text-textStandard"
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
