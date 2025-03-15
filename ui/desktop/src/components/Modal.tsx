import React from 'react';
import { Card } from './ui/card';

interface ModalProps {
  children: React.ReactNode;
  footer?: React.ReactNode; // Optional footer
}

/**
 * A reusable modal component that renders content with a semi-transparent backdrop and blur effect.
 */
export default function Modal({ children, footer }: ModalProps) {
  return (
    <div className="fixed inset-0 bg-black/20 dark:bg-white/20 backdrop-blur-sm transition-colors animate-[fadein_200ms_ease-in_forwards] flex items-center justify-center p-4">
      <Card className="relative w-[500px] max-w-full bg-bgApp rounded-xl shadow-none my-10 overflow-hidden max-h-[90vh] flex flex-col">
        <div className="p-6 overflow-y-auto max-h-[calc(90vh-180px)]">{children}</div>
        {footer && (
          <div className="border-t border-borderSubtle bg-bgApp w-full mt-auto">{footer}</div>
        )}
      </Card>
    </div>
  );
}
