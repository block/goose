import { useState } from 'react';
import { Button } from '../ui/button';
import PrivacyInfoModal from './PrivacyInfoModal';

interface OnboardingSuccessProps {
  providerName: string;
  onFinish: (telemetryEnabled: boolean) => void;
}

export default function OnboardingSuccess({ providerName, onFinish }: OnboardingSuccessProps) {
  const [showPrivacyInfo, setShowPrivacyInfo] = useState(false);
  const [telemetryOptIn, setTelemetryOptIn] = useState(true);

  return (
    <div className="h-screen w-full bg-background-default overflow-hidden">
      <div className="h-full overflow-y-auto">
        <div className="flex flex-col items-center justify-center h-full p-4">
          <div className="max-w-md w-full mx-auto text-center">
            <div className="mb-6">
              <div className="inline-flex items-center justify-center w-12 h-12 rounded-full bg-green-500/10 mb-4">
                <svg
                  className="w-6 h-6 text-green-500"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M5 13l4 4L19 7"
                  />
                </svg>
              </div>
              <h2 className="text-xl font-light text-text-default mb-1">
                Connected to {providerName}
              </h2>
              <p className="text-text-muted text-sm">You're all set to start using Goose.</p>
            </div>

            <div className="w-full p-4 bg-transparent border rounded-xl text-left mb-6">
              <h3 className="font-medium text-text-default text-sm mb-1">Privacy</h3>
              <p className="text-text-muted text-sm">
                Anonymous usage data helps improve goose. We never collect your conversations, code,
                or personal data.{' '}
                <button
                  onClick={() => setShowPrivacyInfo(true)}
                  className="text-blue-600 dark:text-blue-400 hover:underline"
                >
                  Learn more
                </button>
              </p>
              <label className="mt-3 flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={telemetryOptIn}
                  onChange={(e) => setTelemetryOptIn(e.target.checked)}
                  className="rounded"
                />
                <span className="text-text-muted text-sm">Share anonymous usage data</span>
              </label>
            </div>

            <Button onClick={() => onFinish(telemetryOptIn)} className="w-full">
              Get Started
            </Button>
          </div>
        </div>
      </div>

      <PrivacyInfoModal isOpen={showPrivacyInfo} onClose={() => setShowPrivacyInfo(false)} />
    </div>
  );
}
