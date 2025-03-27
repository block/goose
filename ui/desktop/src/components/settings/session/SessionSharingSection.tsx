import React, { useState, useEffect } from 'react';
import { Input } from '../../ui/input';
import { Check } from 'lucide-react';

export default function SessionSharingSection() {
  const [sessionSharingConfig, setSessionSharingConfig] = useState({
    enabled: false,
    baseUrl: '',
  });
  const [urlError, setUrlError] = useState('');
  // Show a checkmark temporarily when the user’s input is valid
  const [urlSaved, setUrlSaved] = useState(false);

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

  // Helper to check if the user’s input is a valid URL
  function isValidUrl(value: string): boolean {
    if (!value) return false;
    try {
      new URL(value);
      return true;
    } catch {
      return false;
    }
  }

  // Handle toggling "Enable Session Sharing"
  const handleEnableToggle = () => {
    setSessionSharingConfig((prev) => {
      const updated = { ...prev, enabled: !prev.enabled };
      localStorage.setItem('session_sharing_config', JSON.stringify(updated));
      return updated;
    });
  };

  // Handle changes to the base URL field
  const handleBaseUrlChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newBaseUrl = e.target.value;
    setSessionSharingConfig((prev) => ({
      ...prev,
      baseUrl: newBaseUrl,
    }));

    if (isValidUrl(newBaseUrl)) {
      setUrlError('');
      const updated = { ...sessionSharingConfig, baseUrl: newBaseUrl };
      localStorage.setItem('session_sharing_config', JSON.stringify(updated));

      // Show the checkmark temporarily
      setUrlSaved(true);
      setTimeout(() => {
        setUrlSaved(false);
      }, 2000);
    } else {
      setUrlError('Invalid URL format. Please enter a valid URL (e.g. https://example.com/api).');
    }
  };

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
          {/* Enable Session Sharing toggle */}
          <div className="flex items-center justify-between">
            <label className="text-sm font-medium text-textStandard cursor-pointer">
              Enable Session Sharing
            </label>
            <button
              onClick={handleEnableToggle}
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

          {/* Base URL field (only visible if enabled) */}
          {sessionSharingConfig.enabled && (
            <div className="space-y-2 relative">
              <div className="flex items-center space-x-2">
                <label
                  htmlFor="session-sharing-url"
                  className="text-sm font-medium text-textStandard"
                >
                  Base URL
                </label>
                {urlSaved && <Check className="w-5 h-5 text-green-500" />}
              </div>
              <div className="flex items-center">
                <Input
                  id="session-sharing-url"
                  type="url"
                  placeholder="https://example.com/api"
                  value={sessionSharingConfig.baseUrl}
                  onChange={handleBaseUrlChange}
                />
              </div>
              {urlError && <p className="text-red-500 text-sm">{urlError}</p>}
            </div>
          )}
        </div>
      </div>
    </>
  );
}
