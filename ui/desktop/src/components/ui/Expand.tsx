import { ChevronUp } from 'lucide-react';

export default function Expand({ size, isExpanded }: { size: number; isExpanded: boolean }) {
  return (
    <ChevronUp
      className={`shrink-0 w-${size} h-${size} text-textPlaceholder transition-all origin-center ${isExpanded ? 'rotate-180' : 'rotate-90'}`}
    />
  );
}
