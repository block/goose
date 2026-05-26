import React from 'react';
import { cn } from '../utils';

/**
 * Shared visual wrapper for the ChatInput.
 *
 * Both the Hub (empty-chat landing) and the BaseChat (active session)
 * present ChatInput as a floating rounded outlined card on the canvas.
 * Centralizing it here keeps the look in sync and gives a single place
 * to tweak the recipe.
 */
export const ChatInputCard: React.FC<{
  className?: string;
  children: React.ReactNode;
}> = ({ className, children }) => (
  <div
    className={cn(
      'rounded-2xl border border-border-primary shadow-sm overflow-hidden bg-background-primary',
      className
    )}
  >
    {children}
  </div>
);
