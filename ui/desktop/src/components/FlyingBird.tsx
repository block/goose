import { Avocado } from './icons/Avocado';

interface FlyingBirdProps {
  className?: string;
  cycleInterval?: number;
}

/** Streaming indicator — avocado mark with a light pulse (legacy name kept for call sites). */
export default function FlyingBird({ className = '' }: FlyingBirdProps) {
  return (
    <div className={`text-text-primary ${className}`}>
      <Avocado className="w-4 h-4 animate-pulse" />
    </div>
  );
}
