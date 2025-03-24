import React from 'react';

function SplashPill({ content, append, className = '', longForm = '' }) {
  return (
    <div
      className={`px-4 py-2 text-sm text-center text-textSubtle dark:text-textStandard cursor-pointer border border-borderSubtle hover:bg-bgSubtle rounded-full transition-all duration-150 ${className}`}
      onClick={async () => {
        // Use the longForm text if provided, otherwise use the content
        await append(longForm || content);
      }}
    >
      <div className="line-clamp-2">{content}</div>
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
