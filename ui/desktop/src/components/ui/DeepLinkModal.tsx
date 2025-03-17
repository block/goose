import React, { useMemo, useState, useEffect } from 'react';
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

export function DeepLinkModal({
  botConfig: initialBotConfig,
  onClose,
  onOpen,
}: DeepLinkModalProps) {
  // Create editable state for the bot config
  const [botConfig, setBotConfig] = useState(initialBotConfig);
  const [instructions, setInstructions] = useState(initialBotConfig.instructions || '');
  const [activities, setActivities] = useState<string[]>(initialBotConfig.activities || []);
  const [activityInput, setActivityInput] = useState('');

  // Generate the deep link using the current bot config
  const deepLink = useMemo(() => {
    const currentConfig = {
      ...botConfig,
      instructions,
      activities,
    };
    return generateDeepLink(currentConfig);
  }, [botConfig, instructions, activities]);

  // Update the bot config when instructions or activities change
  useEffect(() => {
    setBotConfig({
      ...botConfig,
      instructions,
      activities,
    });
  }, [instructions, activities]);

  // Handle adding a new activity
  const handleAddActivity = () => {
    if (activityInput.trim()) {
      setActivities([...activities, activityInput.trim()]);
      setActivityInput('');
    }
  };

  // Handle removing an activity
  const handleRemoveActivity = (index: number) => {
    const newActivities = [...activities];
    newActivities.splice(index, 1);
    setActivities(newActivities);
  };

  return (
    <div className="fixed inset-0 flex items-center justify-center bg-black bg-opacity-50 z-50">
      <div className="bg-bgApp p-6 rounded-lg shadow-lg max-w-2xl w-full max-h-[90vh] overflow-y-auto">
        <h2 className="text-xl font-bold mb-4">Agent Created!</h2>
        <p className="mb-4">
          Your agent has been created successfully. You can review and edit the details below:
        </p>

        {/* Instructions Section */}
        <div className="mb-4">
          <label htmlFor="instructions" className="block font-medium mb-1">
            Instructions:
          </label>
          <textarea
            id="instructions"
            value={instructions}
            onChange={(e) => setInstructions(e.target.value)}
            className="w-full p-2 border border-borderSubtle rounded-md bg-bgSubtle text-textStandard min-h-[100px]"
            placeholder="Instructions for the agent..."
          />
        </div>

        {/* Activities Section */}
        <div className="mb-4">
          <label className="block font-medium mb-1">Activities:</label>
          <ul className="mb-2 space-y-1">
            {activities.map((activity, index) => (
              <li key={index} className="flex items-center">
                <span className="flex-1 p-2 bg-bgSubtle rounded-l-md">{activity}</span>
                <button
                  onClick={() => handleRemoveActivity(index)}
                  className="p-2 bg-red-500 text-white rounded-r-md hover:bg-red-600"
                >
                  ✕
                </button>
              </li>
            ))}
          </ul>
          <div className="flex">
            <input
              type="text"
              value={activityInput}
              onChange={(e) => setActivityInput(e.target.value)}
              onKeyPress={(e) => e.key === 'Enter' && handleAddActivity()}
              className="flex-1 p-2 border border-borderSubtle rounded-l-md bg-bgSubtle text-textStandard"
              placeholder="Add new activity..."
            />
            <button
              onClick={handleAddActivity}
              className="p-2 bg-green-500 text-white rounded-r-md hover:bg-green-600"
            >
              +
            </button>
          </div>
        </div>

        {/* Deep Link Section */}
        <div className="mb-4">
          <label className="block font-medium mb-1">Deep Link:</label>
          <div className="flex items-center">
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
        </div>

        {/* Action Buttons */}
        <div className="flex justify-end">
          <button
            onClick={onClose}
            className="px-4 py-2 bg-gray-500 text-white rounded-md hover:bg-gray-600 mr-2"
          >
            Close
          </button>
          <button
            onClick={() => {
              // Open the deep link with the current bot config
              const currentConfig = {
                ...botConfig,
                instructions,
                activities,
              };
              window.electron.createChatWindow(
                undefined,
                undefined,
                undefined,
                undefined,
                currentConfig
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
