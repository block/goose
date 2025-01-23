import React from 'react';
import { Card } from './ui/card';

interface ErrorScreenProps {
  error: string;
  onReload: () => void;
}

const ErrorScreen: React.FC<ErrorScreenProps> = ({ error, onReload }) => {
  return (
    <div className="chat-content flex flex-col w-screen h-screen bg-window-gradient dark:bg-dark-window-gradient  items-center justify-center p-[10px]">
      <div className="titlebar-drag-region" />
      <div className="relative block h-[20px] w-screen" />
      <Card className="flex flex-col flex-1 h-[calc(100vh-95px)] w-full bg-card-gradient dark:bg-dark-card-gradient mt-0 border-none shadow-xl rounded-2xl relative">
        <div className="flex flex-col items-center justify-center h-full p-4">
          <div className="text-red-700 dark:text-red-300 bg-red-400/50 p-3 rounded-lg mb-2">
            {'Honk! Goose experienced a fatal error'}
          </div>
          <div
            className="p-4 text-center text-splash-pills-text whitespace-nowrap cursor-pointer bg-prev-goose-gradient dark:bg-dark-prev-goose-gradient text-prev-goose-text dark:text-prev-goose-text-dark rounded-[14px] inline-block hover:scale-[1.02] transition-all duration-150"
            onClick={onReload}
          >
            Reload
          </div>
        </div>
      </Card>
    </div>
  );
};

export default ErrorScreen;
