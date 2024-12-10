import React from "react"

function SplashPill({ content, append }) {
  return (
    <div
      className="px-16 py-8 text-14 text-center dark:text-splash-pills-text-dark text-splash-pills-text whitespace-nowrap cursor-pointer bg-splash-pills dark:bg-splash-pills-dark  hover:bg-splash-pills/90 hover:scale-[1.02] rounded-lg inline-block transition-all duration-150"
      onClick={async () => {
        const message = {
          content,
          role: "user",
        };
        await append(message);
      }}
    >
      {content}
    </div>
  )
}

export default function SplashPills({ append }) {
  return (
    <div className="grid grid-cols-2 gap-4 mb-[8px]">
      <SplashPill content="Demo writing and reading files" append={append} />
      <SplashPill content="Make a snake game in a new folder" append={append} />
      <SplashPill content="List files in my current directory" append={append} />
      <SplashPill content="Take a screenshot and summarize" append={append} />
    </div>
  )
}
