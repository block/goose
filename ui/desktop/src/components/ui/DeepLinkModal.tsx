import React, { useMemo } from 'react';
import { Buffer } from 'buffer';

interface DeepLinkModalProps {
  botConfig: any;
  onClose: () => void;
  onOpen: () => void;
}

// Function to generate a deep link from a bot config
export function generateDeepLink(botConfig: any): string {
  const configBase64 = Buffer.from(JSON.stringify(botConfig)).toString('base64');
  return `goose://bot?config=${configBase64}`;
}

export function DeepLinkModal({ botConfig, onClose, onOpen }: DeepLinkModalProps) {
  // Generate the deep link using the provided bot config
  const deepLink = useMemo(() => generateDeepLink(botConfig), [botConfig]);

  return (
    <div className="fixed inset-0 flex items-center justify-center bg-black bg-opacity-50 z-50">
      <div className="bg-bgApp p-6 rounded-lg shadow-lg max-w-md w-full">
        <h2 className="text-xl font-bold mb-4">Agent Created!</h2>
        <p className="mb-4">
          Your agent has been created successfully. Use the link below to access it:
        </p>
        <div className="flex items-center mb-4">
          <input
            type="text"
            value={deepLink}
            readOnly
            className="flex-1 p-2 border border-borderSubtle rounded-l-md bg-bgSubtle text-textStandard"
          />
          <button
            onClick={() => {
              navigator.clipboard.writeText(deepLink);
              window.electron.logInfo('Deep link copied to clipboard');
            }}
            className="p-2 bg-blue-500 text-white rounded-r-md hover:bg-blue-600"
          >
            Copy
          </button>
        </div>
        <div className="flex justify-end">
          <button
            onClick={onClose}
            className="px-4 py-2 bg-gray-500 text-white rounded-md hover:bg-gray-600 mr-2"
          >
            Close
          </button>
          <button
            onClick={() => {
              // Open the deep link
              window.electron.createChatWindow(
                undefined,
                undefined,
                undefined,
                undefined,
                botConfig
              );
              onOpen();
            }}
            className="px-4 py-2 bg-green-500 text-white rounded-md hover:bg-green-600"
          >
            Open Agent
          </button>
        </div>
      </div>
    </div>
  );
}
