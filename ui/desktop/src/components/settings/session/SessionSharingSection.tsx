import React, { useState, useEffect } from 'react';
import { Input } from '../../ui/input';

export default function SessionSharingSection() {
  const [sessionSharingConfig, setSessionSharingConfig] = useState({
    enabled: false,
    baseUrl: '',
  });

  // Load session sharing config from localStorage
  useEffect(() => {
    const savedSessionConfig = localStorage.getItem('session_sharing_config');
    if (savedSessionConfig) {
      try {
        const config = JSON.parse(savedSessionConfig);
        setSessionSharingConfig(config);
      } catch (error) {
        console.error('Error parsing session sharing config:', error);
      }
    }
  }, []);

  return (
    <>
      <div className="flex justify-between items-center mb-6 border-b border-borderSubtle px-8">
        <h2 className="text-xl font-semibold text-textStandard">Session Sharing</h2>
      </div>

      <div className="px-8">
        <p className="text-sm text-textStandard mb-4">
          You can enable session sharing to share your sessions with others. You'll then need to
          enter the base URL for the session sharing API endpoint. Anyone with access to the same
          API and sharing session enabled will be able to see your sessions.
        </p>

        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <label className="text-sm font-medium text-textStandard cursor-pointer">
              Enable Session Sharing
            </label>
            <button
              onClick={() => {
                setSessionSharingConfig((prev) => {
                  const updated = {
                    ...prev,
                    enabled: !prev.enabled,
                  };
                  // Save to localStorage
                  localStorage.setItem('session_sharing_config', JSON.stringify(updated));
                  return updated;
                });
              }}
              className={`relative inline-flex h-6 w-11 items-center rounded-full ${
                sessionSharingConfig.enabled ? 'bg-indigo-500' : 'bg-bgProminent'
              } transition-colors duration-200 ease-in-out focus:outline-none`}
            >
              <span
                className={`inline-block h-5 w-5 transform rounded-full bg-white shadow ${
                  sessionSharingConfig.enabled ? 'translate-x-[22px]' : 'translate-x-[2px]'
                } transition-transform duration-200 ease-in-out`}
              />
            </button>
          </div>

          {sessionSharingConfig.enabled && (
            <div className="space-y-2">
              <label
                htmlFor="session-sharing-url"
                className="text-sm font-medium text-textStandard"
              >
                Base URL
              </label>
              <Input
                id="session-sharing-url"
                type="url"
                placeholder="https://example.com/api"
                value={sessionSharingConfig.baseUrl}
                onChange={(e) => {
                  const newBaseUrl = e.target.value;
                  setSessionSharingConfig((prev) => {
                    const updated = {
                      ...prev,
                      baseUrl: newBaseUrl,
                    };
                    // Save to localStorage
                    localStorage.setItem('session_sharing_config', JSON.stringify(updated));
                    return updated;
                  });
                }}
              />
            </div>
          )}
        </div>
      </div>
    </>
  );
}
