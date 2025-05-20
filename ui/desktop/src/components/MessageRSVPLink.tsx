import React from 'react';
import { Eye } from './icons';

interface MessageRSVPLinkProps {
  text: string;
  onRSVP: (text: string) => void;
}

export default function MessageRSVPLink({ text, onRSVP }: MessageRSVPLinkProps) {
  return (
    <button
      onClick={() => onRSVP(text)}
      className="flex items-center gap-1 text-xs text-textSubtle hover:cursor-pointer hover:text-textProminent transition-all duration-200 opacity-0 group-hover:opacity-100 -translate-y-4 group-hover:translate-y-0 ml-2"
    >
      <Eye className="h-3 w-3" />
      <span>RSVP</span>
    </button>
  );
}
