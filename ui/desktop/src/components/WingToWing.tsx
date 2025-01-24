import React from 'react';
import { Bird } from '../components/ui/icons';

export enum Working {
  Idle = 'Idle',
  Working = 'Working',
}

interface WingToWingProps {
  onExpand: () => void;
  progressMessage: string;
  working: Working;
}

const WingToWing: React.FC<WingToWingProps> = ({ onExpand, progressMessage, working }) => {
  return (
    <div
      onClick={onExpand}
      className="flex items-center w-full h-28 bg-gradient-to-r from-gray-100 via-gray-200 to-gray-300 shadow-md rounded-lg p-4 cursor-pointer hover:shadow-lg transition-all duration-200"
    >
      {working === Working.Working && (
        <div className="w-10 h-10 mr-4 flex-shrink-0">
          <Bird />
        </div>
      )}

      {/* Status Text */}
      <div className="flex flex-col text-left">
        <span className="text-sm text-gray-600 font-medium">{progressMessage}</span>
      </div>
    </div>
  );
};

export default WingToWing;
