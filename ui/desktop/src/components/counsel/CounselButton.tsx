import { useState } from 'react';
import { CounselModal } from './CounselModal';

interface CounselButtonProps {
  className?: string;
}

export function CounselButton({ className = '' }: CounselButtonProps) {
  const [isModalOpen, setIsModalOpen] = useState(false);

  return (
    <>
      <button
        onClick={() => setIsModalOpen(true)}
        className={`flex items-center justify-center gap-2 px-4 py-2 rounded-lg bg-accent-primary hover:bg-accent-primary-hover text-white font-medium transition-colors ${className}`}
        title="Get opinions from the Counsel of 9"
      >
        <span className="text-lg">🎭</span>
        <span>Counsel of 9</span>
      </button>

      {isModalOpen && <CounselModal isOpen={isModalOpen} onClose={() => setIsModalOpen(false)} />}
    </>
  );
}
