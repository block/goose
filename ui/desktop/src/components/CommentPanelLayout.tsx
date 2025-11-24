import React from 'react';
import { useCommentPanelOptional } from '../contexts/CommentPanelContext';
import { cn } from '../utils';

interface CommentPanelLayoutProps {
  children: React.ReactNode;
  className?: string;
}

/**
 * Layout wrapper that shifts content left when comment panel is open
 * Similar to how sidecars work
 */
export default function CommentPanelLayout({ children, className }: CommentPanelLayoutProps) {
  const panelContext = useCommentPanelOptional();
  const isPanelOpen = panelContext?.isPanelOpen ?? false;

  return (
    <div
      className={cn(
        'relative w-full h-full transition-all duration-300 ease-in-out',
        className
      )}
      style={{
        marginRight: isPanelOpen ? '384px' : '0', // 384px = 96 * 4 (w-96)
      }}
    >
      {children}
    </div>
  );
}
