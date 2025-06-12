import { Button } from '../ui/button';
import GooseLogo from '../brand/GooseLogo';

interface SplashProps {
  append: (text: string) => void;
  activities: string[] | null;
  title?: string;
}

export default function Splash({ append, activities, title }: SplashProps) {
  // Default activities if none provided
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
    <div className="flex flex-col h-full">
      {title && (
        <div className="flex items-center px-4 py-2">
          <span className="w-2 h-2 rounded-full bg-blockTeal mr-2" />
          <span className="text-sm">
            <span className="text-textSubtle">Agent</span>{' '}
            <span className="text-textStandard">{title}</span>
          </span>
        </div>
      )}
      <div className="flex flex-col flex-1">
        <div className="h-full flex flex-col pb-12">
          <div className="p-8">
            <div className="relative text-textStandard mb-12">
              <div className="w-min animate-[flyin_2s_var(--spring-easing)_forwards]">
                <GooseLogo />
              </div>
            </div>

            <div className="flex flex-col">
              {messagePill && (
                <div className="mb-6 p-4 bg-bgSubtle rounded-lg border border-borderStandard animate-[fadein_500ms_ease-in_forwards]">
                  {messagePill.replace(/^message:/i, '').trim()}
                </div>
              )}

              <div className="flex flex-wrap gap-4 animate-[fadein_500ms_ease-in_forwards]">
                {remainingPills.map((content, index) => (
                  <Button
                    key={index}
                    variant="outline"
                    onClick={() => append(content)}
                    title={content.length > 100 ? content : undefined}
                  >
                    {content.length > 100 ? content.slice(0, 100) + '...' : content}
                  </Button>
                ))}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
