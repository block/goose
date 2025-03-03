import React from 'react';
import { Card } from './ui/card';

interface ModalProps {
  children: React.ReactNode;
}

/**
 * A reusable modal component that renders content with a semi-transparent backdrop and blur effect.
 */
export default function Modal({ children }: ModalProps) {
  return (
    <div className="fixed inset-0 bg-black/20 dark:bg-white/20 backdrop-blur-sm transition-colors animate-[fadein_200ms_ease-in_forwards]">
      <Card className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[500px] bg-bgApp rounded-xl overflow-hidden shadow-none p-6">
        <div className="space-y-6">{children}</div>
      </Card>
    </div>
  );
}
