import React, { useRef, useState, useEffect } from 'react';
import { Button } from './ui/button';
import Stop from './ui/Stop';
import { Attach, Send } from './icons';

interface InputProps {
  handleSubmit: (e: React.FormEvent) => void;
  disabled?: boolean;
  isLoading?: boolean;
  onStop?: () => void;
}

export default function Input({
  handleSubmit,
  disabled = false,
  isLoading = false,
  onStop,
}: InputProps) {
  const [value, setValue] = useState('');
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

  const handleKeyDown = (evt: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (evt.key === 'Enter' && !evt.shiftKey) {
      evt.preventDefault();
      if (value.trim()) {
        handleSubmit(new CustomEvent('submit', { detail: { value } }));
        setValue('');
      }
    }
  };

  const onFormSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (value.trim()) {
      handleSubmit(new CustomEvent('submit', { detail: { value } }));
      setValue('');
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
    <form
      onSubmit={onFormSubmit}
      className="flex relative h-auto px-[16px] pr-[68px] py-[1rem] border-t border-borderSubtle"
    >
      {/* loading */}
      {/* {isLoading && (
        <div className="absolute top-[-2px] left-0 w-full h-[2px]">
          <div className="absolute w-[300px] h-[2px] bg-gradient-to-r from-blockTeal to-blockOrange animate-gradient-loader"></div>
        </div>
      )} */}
      <textarea
        autoFocus
        id="dynamic-textarea"
        placeholder="What can goose help with?"
        value={value}
        onChange={handleChange}
        onKeyDown={handleKeyDown}
        disabled={disabled}
        ref={textAreaRef}
        rows={1}
        style={{
          minHeight: `${minHeight}px`,
          maxHeight: `${maxHeight}px`,
          overflowY: 'auto',
        }}
        className={`w-full outline-none border-none focus:ring-0 bg-transparent p-0 text-base resize-none text-textStandard ${
          disabled ? 'cursor-not-allowed opacity-50' : ''
        }`}
      />
      <Button
        type="button"
        size="icon"
        variant="ghost"
        onClick={handleFileSelect}
        disabled={disabled}
        className={`absolute right-[40px] top-1/2 -translate-y-1/2 text-textSubtle hover:text-textStandard ${
          disabled ? 'text-textSubtle cursor-not-allowed' : ''
        }`}
      >
        <Attach />
      </Button>
      {isLoading ? (
        <Button
          type="button"
          size="icon"
          variant="ghost"
          onClick={onStop}
          className="absolute right-2 top-1/2 -translate-y-1/2 [&_svg]:size-5 text-textSubtle hover:text-textStandard"
        >
          <Stop size={24} />
        </Button>
      ) : (
        <Button
          type="submit"
          size="icon"
          variant="ghost"
          disabled={disabled || !value.trim()}
          className={`absolute right-2 top-1/2 -translate-y-1/2 text-textSubtle hover:text-textStandard ${
            disabled || !value.trim() ? 'text-textSubtle cursor-not-allowed' : ''
          }`}
        >
          <Send />
        </Button>
      )}
    </form>
  );
}
