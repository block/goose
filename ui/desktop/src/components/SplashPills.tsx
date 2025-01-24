import React from 'react';

function SplashPill({ content, append, className = '' }) {
  return (
    <div
      className={`px-4 py-2 text-sm text-center text-textSubtle dark:text-textStandard cursor-pointer border border-borderSubtle hover:bg-bgSubtle rounded-full transition-all duration-150 ${className}`}
      onClick={async () => {
        const message = {
          content,
          role: 'user',
        };
        await append(message);
      }}
    >
      <div className="line-clamp-2">{content}</div>
    </div>
  );
}

export default function SplashPills({ append }) {
  return (
    <div className="flex flex-wrap gap-2 animate-[fadein_500ms_ease-in_forwards]">
      <SplashPill
        content="What can you do?"
        append={append}
        className="bg-black dark:bg-white dark:hover:bg-gray-200 text-white dark:!text-black border-none hover:bg-slate"
      />
      <SplashPill content="Demo writing and reading files" append={append} />
      <SplashPill content="Make a snake game in a new folder" append={append} />
      <SplashPill content="List files in my current directory" append={append} />
      <SplashPill content="Take a screenshot and summarize" append={append} />
    </div>
  );
}
