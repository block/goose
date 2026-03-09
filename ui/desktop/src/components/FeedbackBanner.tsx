import { useState } from 'react';

interface FeedbackBannerProps {
  onRate: (rating: 1 | 2 | 3 | 4) => void;
  onDismiss: () => void;
}

const ratingOptions: Array<{ rating: 1 | 2 | 3 | 4; emoji: string; label: string }> = [
  { rating: 1, emoji: '\u{1F623}', label: 'Frustrating' },
  { rating: 2, emoji: '\u{1F615}', label: 'Poor' },
  { rating: 3, emoji: '\u{1F642}', label: 'Good' },
  { rating: 4, emoji: '\u{1F929}', label: 'Great' },
];

export default function FeedbackBanner({ onRate, onDismiss }: FeedbackBannerProps) {
  const [submitted, setSubmitted] = useState(false);

  const handleRate = (rating: 1 | 2 | 3 | 4) => {
    setSubmitted(true);
    setTimeout(() => onRate(rating), 1000);
  };

  if (submitted) {
    return (
      <div className="flex items-center justify-center py-2 text-xs text-text-secondary animate-[fadein_300ms_ease-in_forwards]">
        Thanks for the feedback!
      </div>
    );
  }

  return (
    <div className="flex items-center justify-center gap-3 py-2 animate-[fadein_300ms_ease-in_forwards]">
      <span className="text-xs text-text-secondary">How&apos;s Goose doing?</span>
      <div className="flex items-center gap-1">
        {ratingOptions.map(({ rating, emoji, label }) => (
          <button
            key={rating}
            onClick={() => handleRate(rating)}
            className="p-1 rounded hover:bg-background-secondary transition-colors cursor-pointer"
            title={label}
            aria-label={label}
          >
            <span className="text-base">{emoji}</span>
          </button>
        ))}
      </div>
      <button
        onClick={onDismiss}
        className="ml-1 p-0.5 rounded text-text-secondary hover:text-text-primary hover:bg-background-secondary transition-colors cursor-pointer"
        title="Dismiss"
        aria-label="Dismiss feedback prompt"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <line x1="18" y1="6" x2="6" y2="18" />
          <line x1="6" y1="6" x2="18" y2="18" />
        </svg>
      </button>
    </div>
  );
}
