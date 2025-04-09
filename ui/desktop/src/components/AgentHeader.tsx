import React from 'react';

interface AgentHeaderProps {
  title: string;
  profileInfo?: string;
  onChangeProfile?: () => void;
}

export function AgentHeader({ title, profileInfo, onChangeProfile }: AgentHeaderProps) {
  return (
    <div className="flex items-center justify-between px-4 py-2 border-b border-gray-200">
      <div className="flex items-center">
        <span className="w-2 h-2 rounded-full bg-[#FF69B4] mr-2" />
        <span className="text-sm">
          <span className="text-gray-600">Agent</span> {title}
        </span>
      </div>
      {profileInfo && (
        <div className="flex items-center text-sm">
          <span className="text-gray-600">{profileInfo}</span>
          {onChangeProfile && (
            <button onClick={onChangeProfile} className="ml-2 text-blue-600 hover:underline">
              change profile
            </button>
          )}
        </div>
      )}
    </div>
  );
}
