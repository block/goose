import React from 'react';

function SplashPill({ content, append }) {
  return (
    <div
      className="px-16 py-8 text-14 text-center text-black/60 dark:text-white/60 
                 cursor-pointer bg-black/5 dark:bg-white/5 
                 hover:bg-black/10 dark:hover:bg-white/10 
                 rounded-[1000px] inline-block transition-all duration-150"
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
    <div className="grid grid-cols-2 gap-4 mb-[8px] max-w-full">
      <SplashPill content="Demo writing and reading files" append={append} />
      <SplashPill content="Make a snake game in a new folder" append={append} />
      <SplashPill content="List files in my current directory" append={append} />
      <SplashPill content="Take a screenshot and summarize" append={append} />
    </div>
  );
}
