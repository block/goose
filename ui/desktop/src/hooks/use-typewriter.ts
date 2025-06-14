import { useState, useEffect } from 'react';

interface UseTypewriterOptions {
  speed?: number; // Speed in milliseconds between each character
  delay?: number; // Initial delay before typing starts
}

export function useTypewriter(text: string, options: UseTypewriterOptions = {}) {
  const { speed = 50, delay = 0 } = options;
  const [displayText, setDisplayText] = useState('');
  const [isTyping, setIsTyping] = useState(true);

  useEffect(() => {
    let timeout: NodeJS.Timeout;
    let currentIndex = 0;

    // Reset state when text changes
    setDisplayText('');
    setIsTyping(true);

    // Only proceed if we have valid text
    if (!text || typeof text !== 'string') {
      setIsTyping(false);
      return;
    }

    // Initial delay
    const startTimeout = setTimeout(() => {
      const typeNextChar = () => {
        if (currentIndex < text.length) {
          const nextChar = text[currentIndex];
          if (nextChar !== undefined) {
            setDisplayText((prev) => prev + nextChar);
            currentIndex++;
            timeout = setTimeout(typeNextChar, speed);
          } else {
            setIsTyping(false);
          }
        } else {
          setIsTyping(false);
        }
      };

      typeNextChar();
    }, delay);

    return () => {
      clearTimeout(startTimeout);
      clearTimeout(timeout);
    };
  }, [text, speed, delay]);

  return { displayText, isTyping };
}
