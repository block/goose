import React, { useState } from 'react';
import { Check, Copy } from '../icons';

interface CopyButtonProps {
  text: string;
  className?: string;
  iconClassName?: string;
  lightIcon?: boolean;
}

export default function CopyButton({
  text,
  className = 'absolute bottom-2 right-2 p-1.5 rounded-lg bg-gray-700/50 text-gray-300 opacity-0 group-hover:opacity-100 transition-opacity duration-200 hover:bg-gray-600/50 hover:text-gray-100',
  iconClassName = 'h-4 w-4',
  lightIcon = false,
}: CopyButtonProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000); // Reset after 2 seconds
    } catch (err) {
      console.error('Failed to copy text: ', err);
    }
  };

  const Icon = copied ? Check : Copy;

  return (
    <button onClick={handleCopy} className={className} title="Copy text">
      <Icon className={`${iconClassName} ${lightIcon ? 'text-white' : ''}`} />
    </button>
  );
}
