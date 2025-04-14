import React from 'react';

function truncateText(text: string, maxLength: number = 100): string {
  if (text.length <= maxLength) return text;
  return text.slice(0, maxLength) + '...';
}

function SplashPill({ content, append, className = '', longForm = '' }) {
  const displayText = truncateText(content);

  return (
    <div
      className={`px-4 py-2 text-sm text-center text-textStandard cursor-pointer border border-borderSubtle hover:bg-bgSubtle rounded-full transition-all duration-150 ${className}`}
      onClick={async () => {
        // Always use the full text (longForm or original content) when clicked
        await append(longForm || content);
      }}
      title={content.length > 100 ? content : undefined} // Show full text on hover if truncated
    >
      <div className="whitespace-normal">{displayText}</div>
    </div>
  );
}

export default function SplashPills({ append, activities = null }) {
  // If custom activities are provided, use those instead of the default ones
  const pills = activities || [
    'What can you do?',
    'Demo writing and reading files',
    'Make a snake game in a new folder',
    'List files in my current directory',
    'Take a screenshot and summarize',
  ];

  return (
    <div className="flex flex-wrap gap-2 animate-[fadein_500ms_ease-in_forwards]">
      {pills.map((content, index) => (
        <SplashPill key={index} content={content} append={append} />
      ))}
    </div>
  );
}
