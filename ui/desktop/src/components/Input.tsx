import React, { useState, useRef, useEffect } from 'react';

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
  const [value, setValue] = useState(initialValue);
  const textAreaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (textAreaRef.current) {
      textAreaRef.current.style.height = '0px';
      const scrollHeight = textAreaRef.current.scrollHeight;
      textAreaRef.current.style.height = Math.min(scrollHeight, 240) + 'px';
    }
  }, [value]);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      if (value.trim() && !isLoading) {
        handleSubmit(new CustomEvent('submit', { detail: { value } }));
        setValue('');
      }
    }
  };

  const onFormSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (value.trim() && !isLoading) {
      handleSubmit(new CustomEvent('submit', { detail: { value } }));
      setValue('');
    }
  };

  return (
    <form className="flex relative h-auto px-[16px] pr-[68px] py-[1rem] border-t border-borderSubtle" onSubmit={onFormSubmit}>
      <textarea
        id="dynamic-textarea"
        placeholder="What can goose help with?   ⌘↑/⌘↓"
        rows={1}
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={handleKeyDown}
        ref={textAreaRef}
        className="w-full outline-none border-none focus:ring-0 bg-transparent p-0 text-base resize-none text-textStandard"
        style={{
          maxHeight: '240px',
          overflowY: 'auto',
          height: '24px'
        }}
      />
      {isLoading ? (
        <button
          type="button"
          onClick={(e) => {
            e.preventDefault();
            e.stopPropagation();
            onStop?.();
          }}
          className="inline-flex items-center justify-center gap-2 whitespace-nowrap text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 hover:bg-accent h-9 w-9 absolute right-2 top-1/2 -translate-y-1/2 [&_svg]:size-5 text-textSubtle hover:text-textStandard"
        >
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
            <rect x="6" y="6" width="12" height="12" fill="currentColor" rx="1" />
          </svg>
        </button>
      ) : (
        <button
          className="inline-flex items-center justify-center gap-2 whitespace-nowrap text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:size-6 [&_svg]:shrink-0 hover:bg-accent h-9 w-9 absolute right-2 top-1/2 -translate-y-1/2 hover:text-textStandard text-textSubtle cursor-not-allowed"
          type="submit"
          disabled={!value.trim() || isLoading}
        >
          <svg fill="none" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" aria-hidden="true">
            <path fillRule="evenodd" clipRule="evenodd" d="M20.707 3.293a1 1 0 0 1 .25.994l-5.4 18a1 1 0 0 1-1.886.084l-3.44-8.602-8.602-3.44a1 1 0 0 1 .084-1.887l18-5.4a1 1 0 0 1 .994.25Zm-8.534 9.948 2.292 5.729L18.509 5.49 5.03 9.535l5.73 2.292 1.333-1.334a1 1 0 0 1 1.414 1.414l-1.334 1.334Z" fill="currentColor"/>
          </svg>
        </button>
      )}
    </form>
  );
}