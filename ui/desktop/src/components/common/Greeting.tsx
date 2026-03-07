import { useState } from 'react';
import { useTextAnimator } from '../../hooks/use-text-animator';
import { useWhiteLabel } from '../../whitelabel/WhiteLabelContext';

interface GreetingProps {
  className?: string;
  forceRefresh?: boolean;
}

export function Greeting({
  className = 'mt-1 text-4xl font-light animate-in fade-in duration-300',
  forceRefresh = false,
}: GreetingProps) {
  const { getRandomGreeting, branding } = useWhiteLabel();

  const greeting = useState(() => ({
    message: getRandomGreeting(),
  }))[0];

  const messageRef = useTextAnimator({ text: greeting.message });

  return (
    <div key={forceRefresh ? Date.now() : undefined}>
      <h1 className={className}>
        <span ref={messageRef}>{greeting.message}</span>
      </h1>
      {branding.tagline && (
        <p className="mt-1 text-sm text-text-secondary animate-in fade-in duration-500">
          {branding.tagline}
        </p>
      )}
    </div>
  );
}
