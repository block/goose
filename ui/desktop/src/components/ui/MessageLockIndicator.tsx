import React, { useEffect, useState } from 'react';
import { Lock, Unlock, Eye, EyeOff } from 'lucide-react';

interface MessageLockIndicatorProps {
  messageId: string;
  onUnlock: () => void;
  isStreamingMessage?: boolean; // NEW: Whether a message is currently streaming
  isFollowingStream?: boolean; // NEW: Whether user is following the stream
}

export const MessageLockIndicator = React.forwardRef<HTMLDivElement, MessageLockIndicatorProps>(
  ({ messageId, onUnlock, isStreamingMessage = false, isFollowingStream = false }, ref) => {
    const [position, setPosition] = useState<{ top: number; left: number } | null>(null);
    const [isVisible, setIsVisible] = useState(false);

    // Position the indicator under the locked message
    useEffect(() => {
      const messageElement = document.querySelector(`[data-message-id="${messageId}"]`) as HTMLElement;
      if (!messageElement) return;

      const updatePosition = () => {
        const rect = messageElement.getBoundingClientRect();
        const scrollContainer = messageElement.closest('[data-radix-scroll-area-viewport]');
        const containerRect = scrollContainer?.getBoundingClientRect();
        
        if (containerRect) {
          setPosition({
            top: rect.bottom - containerRect.top + 8, // 8px below the message
            left: rect.left - containerRect.left + 16, // 16px from left edge
          });
          setIsVisible(true);
        }
      };

      updatePosition();
      
      // Update position on scroll or resize
      const handleUpdate = () => updatePosition();
      window.addEventListener('scroll', handleUpdate, true);
      window.addEventListener('resize', handleUpdate);
      
      return () => {
        window.removeEventListener('scroll', handleUpdate, true);
        window.removeEventListener('resize', handleUpdate);
      };
    }, [messageId]);

    if (!position || !isVisible) return null;

    // Determine the appropriate icon and message based on stream state
    const getIndicatorContent = () => {
      if (isStreamingMessage && isFollowingStream) {
        return {
          icon: <Eye className="w-3 h-3" />,
          text: "Following stream (locked to this message)",
          bgColor: "bg-blue-500/90",
          textColor: "text-white"
        };
      } else if (isStreamingMessage) {
        return {
          icon: <EyeOff className="w-3 h-3" />,
          text: "Stream blocked (locked to this message)",
          bgColor: "bg-orange-500/90",
          textColor: "text-white"
        };
      } else {
        return {
          icon: <Lock className="w-3 h-3" />,
          text: "Auto-scroll locked to this message",
          bgColor: "bg-gray-800/90",
          textColor: "text-white"
        };
      }
    };

    const { icon, text, bgColor, textColor } = getIndicatorContent();

    return (
      <div
        ref={ref}
        className="absolute z-50 pointer-events-auto"
        style={{
          top: position.top,
          left: position.left,
        }}
      >
        <div className={`flex items-center gap-2 px-3 py-2 rounded-lg shadow-lg border border-white/20 ${bgColor} ${textColor} text-sm`}>
          {icon}
          <span className="font-medium">{text}</span>
          <button
            onClick={onUnlock}
            className="ml-2 p-1 hover:bg-white/20 rounded transition-colors"
            title="Unlock and resume auto-scroll"
          >
            <Unlock className="w-3 h-3" />
          </button>
        </div>
      </div>
    );
  }
);

MessageLockIndicator.displayName = 'MessageLockIndicator';
