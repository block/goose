import React from 'react';
import SplashPills from './SplashPills';
import GooseLogo from './GooseLogo';

export default function Splash({ append, activities = null }) {
  return (
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
  );
}
