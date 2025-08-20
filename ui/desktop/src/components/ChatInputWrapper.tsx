import React, { useState, useEffect, useRef } from 'react';

interface ChatInputWrapperProps {
  children: React.ReactNode;
  onTypingStateChange?: (isTyping: boolean) => void;
}

const ChatInputWrapper: React.FC<ChatInputWrapperProps> = ({ children, onTypingStateChange }) => {
  const [isTyping, setIsTyping] = useState(false);
  const timeoutRef = useRef<NodeJS.Timeout | null>(null);
  
  // Listen for keyboard input events in the chat input
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Only trigger for textarea elements
      if ((e.target as HTMLElement).tagName.toLowerCase() === 'textarea') {
        // Set typing state to true
        setIsTyping(true);
        if (onTypingStateChange) onTypingStateChange(true);
        
        // Clear any existing timeout
        if (timeoutRef.current) {
          clearTimeout(timeoutRef.current);
        }
        
        // Set a timeout to turn off the typing state after 1.5 seconds of inactivity
        timeoutRef.current = setTimeout(() => {
          setIsTyping(false);
          if (onTypingStateChange) onTypingStateChange(false);
        }, 1500);
      }
    };
    
    // Add event listeners
    document.addEventListener('keydown', handleKeyDown);
    
    // Cleanup
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    };
  }, [onTypingStateChange]);
  
  return (
    <div className="relative">
      {/* Render the chat input */}
      {children}
    </div>
  );
};

export default ChatInputWrapper;
