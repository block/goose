import { Card } from './ui/card';
import { gsap } from 'gsap';
import { Greeting } from './common/Greeting';

// Register GSAP plugins
gsap.registerPlugin();

interface SplashProps {
  append: (text: string) => void;
  activities: string[] | null;
  title?: string;
}

export default function Splash({ append, activities, title }: SplashProps) {
  const pills = activities || [];

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
      {title && (
        <div className="flex items-center px-4 py-2 mb-4">
          <span className="w-2 h-2 rounded-full bg-blockTeal mr-2" />
          <span className="text-sm">
            <span className="text-text-muted">Agent</span>{' '}
            <span className="text-text-default">{title}</span>
          </span>
        </div>
      )}

      {/* Compact greeting section */}
      <div className="flex flex-col px-6 mb-0">
        <Greeting className="text-text-prominent text-4xl font-light mb-2" />
      </div>
      <div className="flex flex-col">
        {messagePill && (
          <div className="mb-4 p-3 rounded-lg border animate-[fadein_500ms_ease-in_forwards]">
            {messagePill.replace(/^message:/i, '').trim()}
          </div>
        )}

        <div className="flex flex-wrap gap-3 animate-[fadein_500ms_ease-in_forwards]">
          {remainingPills.map((content, index) => (
            <Card
              key={index}
              onClick={() => append(content)}
              title={content.length > 100 ? content : undefined}
              className="cursor-pointer px-4 py-2 w-[200px]"
            >
              {content.length > 100 ? content.slice(0, 100) + '...' : content}
            </Card>
          ))}
        </div>
      </div>
    </div>
  );
}
