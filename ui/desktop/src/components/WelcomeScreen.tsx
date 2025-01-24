import React from 'react';
import { Card } from './ui/card';
import { Bird } from './ui/icons';

interface WelcomeScreenProps {
  className?: string;
  onDismiss: () => void;
}

export function WelcomeScreen({ className, onDismiss }: WelcomeScreenProps) {
  return (
    <Card
      className={`flex flex-col items-center justify-center p-8 space-y-6 bg-bgApp w-full h-full ${className}`}
    >
      <div className="w-16 h-16">
        <Bird />
      </div>
      <div className="text-center space-y-4">
        <h2 className="text-2xl font-semibold text-gray-800 dark:text-white/70">
          Welcome to Goose 1.0 <b>beta</b>! 🎉
        </h2>
        <div className="whitespace-pre-wrap text-gray-600 dark:text-white/50">
          Goose is your AI-powered agent.
          <br />
          <br />
          <b>
            {' '}
            Warning: During the beta, your chats are not saved - closing the window <br />
            or closing the app will lose your history. <br />
          </b>
          <br />
          <br />
          Try ⌘+N for a new window, or ⌘+O to work on a specific directory.
        </div>

        <button
          onClick={onDismiss}
          className="mt-6 px-4 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600 transition-colors"
        >
          Get Started
        </button>
      </div>
    </Card>
  );
}
