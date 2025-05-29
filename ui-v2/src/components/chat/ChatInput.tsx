import React, { useState, useRef, useEffect } from 'react';

import { motion } from 'framer-motion';

interface ChatInputProps {
  handleSubmit: (event: React.FormEvent<HTMLFormElement>) => void;
  isLoading?: boolean;
  onStop?: () => void;
  initialValue?: string;
}

export const ChatInput: React.FC<ChatInputProps> = ({
  handleSubmit,
  isLoading = false,
  onStop: _onStop,
  initialValue = '',
}) => {
  const [input, setInput] = useState(initialValue);
  const [key, setKey] = useState(0); // Add a key to force re-render
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const adjustTextareaHeight = () => {
    const textarea = textareaRef.current;
    if (textarea) {
      textarea.style.height = 'auto';
      textarea.style.height = Math.min(textarea.scrollHeight, 200) + 'px';
    }
  };

  useEffect(() => {
    adjustTextareaHeight();
  }, [input]);

  // Watch for class changes on html element (theme changes)
  useEffect(() => {
    const observer = new MutationObserver((mutations) => {
      mutations.forEach((mutation) => {
        if (mutation.attributeName === 'class') {
          setKey((prev) => prev + 1); // Force textarea to re-render
        }
      });
    });

    const htmlElement = document.documentElement;
    observer.observe(htmlElement, { attributes: true });

    return () => observer.disconnect();
  }, []);

  const handleFormSubmit = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (!input.trim() || isLoading) return;

    handleSubmit(e);
    setInput('');
    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto';
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      const form = (e.target as HTMLTextAreaElement).form;
      if (form) form.requestSubmit();
    }
  };

  return (
    <motion.div
      ref={containerRef}
      className="w-full bg-black dark:bg-white rounded-xl shadow-lg"
      initial={{ opacity: 0, y: 20, scale: 0.95 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      transition={{
        type: 'spring',
        stiffness: 300,
        damping: 30,
      }}
    >
      <form onSubmit={handleFormSubmit} className="relative px-4 py-3">
        <div className="flex items-center gap-3">
          <textarea
            key={key} // Force re-render when theme changes
            ref={textareaRef}
            name="message"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="What can goose help with? ⌘↑/⌘↓"
            className="flex-1 resize-none bg-transparent text-white dark:text-black
                     focus:outline-none focus:ring-0 rounded-lg
                     min-h-[40px] max-h-[200px] transition-all duration-200
                     placeholder:text-zinc-500"
            style={{ overflow: input.split('\n').length > 1 ? 'auto' : 'hidden' }}
          />

          <motion.button
            type="submit"
            disabled={!input.trim()}
            whileHover={{ scale: 1.05 }}
            whileTap={{ scale: 0.95 }}
            className={`
              p-2 rounded-lg w-10 h-10 flex items-center justify-center
              transition-colors duration-200
              ${
                input.trim()
                  ? 'hover:bg-zinc-800 active:bg-zinc-700 dark:hover:bg-zinc-100 dark:active:bg-zinc-200'
                  : 'cursor-not-allowed'
              }
            `}
          >
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke={input.trim() ? 'currentColor' : '#666'}
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              className={`
                transition-colors duration-200
                ${input.trim() ? 'text-white dark:text-black' : 'text-zinc-600 dark:text-zinc-400'}
              `}
            >
              <path d="M22 2L11 13M22 2L15 22L11 13L2 9L22 2Z" />
            </svg>
          </motion.button>
        </div>
      </form>
    </motion.div>
  );
};
