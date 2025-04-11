import React from 'react';

interface AgentHeaderProps {
  title: string;
  profileInfo?: string;
  onChangeProfile?: () => void;
}

export function AgentHeader({ title, profileInfo, onChangeProfile }: AgentHeaderProps) {
  return (
    <div className="flex items-center justify-between px-4 py-2 border-b border-borderSubtle">
      <div className="flex items-center">
        <span className="w-2 h-2 rounded-full bg-blockTeal mr-2" />
        <span className="text-sm">
          <span className="text-textSubtle">Agent</span>{' '}
          <span className="text-textStandard">{title}</span>
        </span>
      </div>
      {profileInfo && (
        <div className="flex items-center text-sm">
          <span className="text-textSubtle">{profileInfo}</span>
          {onChangeProfile && (
            <button onClick={onChangeProfile} className="ml-2 text-blockTeal hover:underline">
              change profile
            </button>
          )}
        </div>
      )}
    </div>
  );
}
