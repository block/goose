import React, { useEffect, useRef } from 'react';
import { Card } from './ui/card';

interface ModalProps {
  children: React.ReactNode;
  footer?: React.ReactNode; // Optional footer
  onClose: () => void; // Function to call when modal should close
  preventBackdropClose?: boolean; // Optional prop to prevent closing on backdrop click
}

/**
 * A reusable modal component that renders content with a semi-transparent backdrop and blur effect.
 * Closes when clicking outside the modal or pressing Esc key.
 */
export default function Modal({
  children,
  footer,
  onClose,
  preventBackdropClose = false,
}: ModalProps) {
  const modalRef = useRef<HTMLDivElement>(null);

  // Handle click outside the modal content
  const handleBackdropClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (preventBackdropClose) return;
    // Check if the click was on the backdrop and not on the modal content
    // Also check if the click target is not part of a Select menu
    if (
      modalRef.current &&
      !modalRef.current.contains(e.target as Node) &&
      !(e.target as HTMLElement).closest('.select__menu') &&
      window.getSelection()?.toString().length === 0 // Ensure no text is selected
    ) {
      onClose();
    }
  };

  // Handle Esc key press
  useEffect(() => {
    const handleEscKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        // Don't close if a select menu is open
        const selectMenu = document.querySelector('.select__menu');
        if (!selectMenu) {
          onClose();
        }
      }
    };

    // Add event listener for Escape key
    document.addEventListener('keydown', handleEscKey);

    // Add overflow-hidden to body to prevent scrolling background
    document.body.style.overflow = 'hidden';

    // Clean up
    return () => {
      document.removeEventListener('keydown', handleEscKey);
      // Restore body scrolling when modal closes
      document.body.style.overflow = '';
    };
  }, [onClose]);

  return (
    <div
      className="fixed inset-0 bg-black/30 dark:bg-white/20 transition-colors animate-[fadein_200ms_ease-in_forwards] flex items-center justify-center p-4 z-[9999]"
      onClick={handleBackdropClick}
      style={{ isolation: 'isolate' }} /* Creates a new stacking context */
    >
      <Card
        ref={modalRef}
        className="relative w-[500px] max-w-full bg-background-default rounded-xl my-10 max-h-[90vh] flex flex-col shadow-xl z-[10000]"
      >
        <div className="p-8 max-h-[calc(90vh-180px)] overflow-y-auto">{children}</div>
        {footer && (
          <div className="border-t border-borderSubtle bg-background-default w-full rounded-b-xl overflow-hidden">
            {footer}
          </div>
        )}
      </Card>
    </div>
  );
}
