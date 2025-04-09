import React from 'react';
import SplashPills from './SplashPills';
import GooseLogo from './GooseLogo';

interface SplashProps {
  append: (text: string) => void;
  activities: string[] | null;
  title?: string;
}

export default function Splash({ append, activities, title }: SplashProps) {
  return (
    <div className="flex flex-col h-full">
      {title && (
        <div className="flex items-center px-4 py-2">
          <span className="w-2 h-2 rounded-full bg-[#FF69B4] mr-2" />
          <span className="text-sm">
            <span className="text-gray-600">Agent</span> {title}
          </span>
        </div>
      )}
      <div className="flex flex-col items-center justify-center flex-1">
        <div className="h-full flex flex-col pb-12">
          <div className="p-8">
            <div className="relative text-textStandard mb-12">
              <div className="w-min animate-[flyin_2s_var(--spring-easing)_forwards]">
                <GooseLogo />
              </div>
            </div>

            <div className="flex">
              <SplashPills append={append} activities={activities} />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
