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
      !(e.target as HTMLElement).closest('.select__menu')
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

    // Add event listener
    document.addEventListener('keydown', handleEscKey);

    // Clean up
    return () => {
      document.removeEventListener('keydown', handleEscKey);
    };
  }, [onClose]);

  return (
    <div
      className="fixed inset-0 bg-black/20 dark:bg-white/20 backdrop-blur-sm transition-colors animate-[fadein_200ms_ease-in_forwards] flex items-center justify-center p-4"
      onClick={handleBackdropClick}
    >
      <Card
        ref={modalRef}
        className="relative w-[500px] max-w-full bg-bgApp rounded-xl my-10 max-h-[90vh] flex flex-col"
      >
        <div className="p-8 max-h-[calc(90vh-180px)] overflow-y-auto">{children}</div>
        {footer && (
          <div className="border-t border-borderSubtle bg-bgApp w-full rounded-b-xl overflow-hidden">
            {footer}
          </div>
        )}
      </Card>
    </div>
  );
}
