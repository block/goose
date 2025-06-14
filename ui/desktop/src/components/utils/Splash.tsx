import { useTextAnimator } from '@/hooks/use-text-animator';
import { Card } from '../ui/card';
import { useState, useEffect } from 'react';
import { gsap } from 'gsap';

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

  const [timeOfDay, setTimeOfDay] = useState<'morning' | 'afternoon' | 'night'>('morning');

  // Update time of day, current time, and date based on current time
  useEffect(() => {
    const updateTime = () => {
      const now = new Date();
      const hour = now.getHours();

      // Update time of day
      if (hour >= 5 && hour < 12) {
        setTimeOfDay('morning');
      } else if (hour >= 12 && hour < 18) {
        setTimeOfDay('afternoon');
      } else {
        setTimeOfDay('night');
      }
    };

    updateTime();
    const interval = setInterval(updateTime, 1000);
    return () => clearInterval(interval);
  }, []);

  const getGreeting = () => {
    switch (timeOfDay) {
      case 'morning':
        return {
          prefix: 'Morning.',
          message: " Let's get things done.",
        };
      case 'afternoon':
        return {
          prefix: 'Afternoon.',
          message: ' Keep the momentum going.',
        };
      case 'night':
        return {
          prefix: 'Evening.',
          message: ' Time to wrap things up.',
        };
    }
  };

  const greeting = getGreeting();
  const [isContentReady, setIsContentReady] = useState(false);
  const greetingMessageRef = useTextAnimator({ text: greeting.message });

  // Set content ready after initial render
  useEffect(() => {
    setIsContentReady(true);
  }, []);

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
            {/* <div className="relative text-textStandard mb-12">
              <div className="w-min animate-[flyin_2s_var(--spring-easing)_forwards]">
                <GooseLogo />
              </div>
            </div> */}

            <div className="flex flex-col mt-6 mb-4 animate-[appear_300ms_ease-in_forwards] origin-left">
              <h1 className="text-text-prominent text-4xl font-light min-h-[4rem]">
                <span>{greeting.prefix}</span>
                <div className="text-text-muted inline" ref={greetingMessageRef}>
                  {greeting.message}
                </div>
              </h1>
            </div>

            <div className="flex flex-col">
              {messagePill && (
                <div className="mb-6 p-4 bg-bgSubtle rounded-lg border border-borderStandard animate-[fadein_500ms_ease-in_forwards]">
                  {messagePill.replace(/^message:/i, '').trim()}
                </div>
              )}

              <div className="flex flex-wrap gap-4 animate-[fadein_500ms_ease-in_forwards]">
                {remainingPills.map((content, index) => (
                  <Card
                    key={index}
                    onClick={() => append(content)}
                    title={content.length > 100 ? content : undefined}
                    className="cursor-pointer px-6 w-[256px]"
                  >
                    {content.length > 100 ? content.slice(0, 100) + '...' : content}
                  </Card>
                ))}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
