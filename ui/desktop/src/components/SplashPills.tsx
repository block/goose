import MarkdownContent from './MarkdownContent';

function truncateText(text: string, maxLength: number = 100): string {
  if (text.length <= maxLength) return text;
  return text.slice(0, maxLength) + '...';
}

interface SplashPillProps {
  content: string;
  append: (text: string) => void;
  className?: string;
}

function SplashPill({ content, append, className = '' }: SplashPillProps) {
  const displayText = truncateText(content);

  return (
    <div
      className={`px-4 py-2 text-sm text-center text-textStandard cursor-pointer border border-borderSubtle hover:bg-bgSubtle rounded-full transition-all duration-150 ${className}`}
      onClick={async () => {
        // Always use the full text (longForm or original content) when clicked
        await append(content);
      }}
      title={content.length > 100 ? content : undefined} // Show full text on hover if truncated
    >
      <div className="whitespace-normal">{displayText}</div>
    </div>
  );
}

interface ContextBlockProps {
  content: string;
}

function ContextBlock({ content }: ContextBlockProps) {
  // Remove the "message:" prefix and trim whitespace
  const displayText = content.replace(/^message:/i, '').trim();

  return (
    <div className="mb-6 p-4 bg-bgSubtle rounded-lg border border-borderStandard animate-[fadein_500ms_ease-in_forwards]">
      <MarkdownContent content={displayText} />
    </div>
  );
}

interface SplashPillsProps {
  append: (text: string) => void;
  activities: string[] | null;
}

export default function SplashPills({ append, activities = null }: SplashPillsProps) {
  // If custom activities are provided, use those instead of the default ones
  const defaultPills = [
    'What can you do?',
    'Demo writing and reading files',
    'Make a snake game in a new folder',
    'List files in my current directory',
    'Take a screenshot and summarize',
  ];

  const pills = activities || defaultPills;

  // Find any pill that starts with "message:"
  const messagePillIndex = pills.findIndex((pill) => pill.toLowerCase().startsWith('message:'));

  // Extract the message pill and the remaining pills
  const messagePill = messagePillIndex >= 0 ? pills[messagePillIndex] : null;
  const remainingPills =
    messagePillIndex >= 0
      ? [...pills.slice(0, messagePillIndex), ...pills.slice(messagePillIndex + 1)]
      : pills;

  return (
    <div className="flex flex-col">
      {messagePill && <ContextBlock content={messagePill} />}

      <div className="flex flex-wrap gap-2 animate-[fadein_500ms_ease-in_forwards]">
        {remainingPills.map((content, index) => (
          <SplashPill key={index} content={content} append={append} />
        ))}
      </div>
    </div>
  );
}
