import React, { useState, useEffect } from 'react';
import { Gooseling } from '../api-client';
import { Card } from './ui/card';
import Copy from './icons/Copy';
import { Buffer } from 'buffer';

interface GooselingEditorProps {
  gooseling: Gooseling;
  setView: (view: string) => void;
}

// Function to generate a deep link from a gooseling
function generateDeepLink(gooseling: Gooseling): string {
  const configBase64 = Buffer.from(JSON.stringify(gooseling)).toString('base64');
  return `goose://gooseling?config=${configBase64}`;
}

export default function GooselingEditor({
  gooseling: initialGooseling,
  setView,
}: GooselingEditorProps) {
  const [gooseling, setGooseling] = useState<Gooseling>(initialGooseling);
  const [title, setTitle] = useState(initialGooseling.title || '');
  const [description, setDescription] = useState(initialGooseling.description || '');
  const [instructions, setInstructions] = useState(initialGooseling.instructions || '');
  const [activities, setActivities] = useState<string[]>(initialGooseling.activities || []);
  const [activityInput, setActivityInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Update gooseling when fields change
  useEffect(() => {
    setGooseling({
      ...gooseling,
      title,
      description,
      instructions,
      activities,
    });
  }, [title, description, instructions, activities]);

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

  // Generate deep link
  const deepLink = generateDeepLink(gooseling);

  return (
    <div className="flex flex-col w-full h-screen bg-bgApp p-8">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold text-textStandard">Edit Gooseling</h1>
        <button
          onClick={() => window.close()}
          className="px-4 py-2 bg-gray-500 text-white rounded-md hover:bg-gray-600"
        >
          Close
        </button>
      </div>

      <Card className="flex-1 p-6 overflow-y-auto">
        {/* Name Field */}
        <div className="mb-6">
          <label className="block font-medium mb-2 text-textStandard">Name:</label>
          <input
            type="text"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            className="w-full p-3 border border-borderSubtle rounded-md bg-transparent text-textStandard focus:border-borderStandard hover:border-borderStandard"
            placeholder="Enter gooseling name..."
          />
        </div>

        {/* Description Field */}
        <div className="mb-6">
          <label className="block font-medium mb-2 text-textStandard">Description:</label>
          <textarea
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            className="w-full p-3 border border-borderSubtle rounded-md bg-transparent text-textStandard focus:border-borderStandard hover:border-borderStandard"
            placeholder="Enter gooseling description..."
            rows={3}
          />
        </div>

        {/* Instructions Field */}
        <div className="mb-6">
          <label className="block font-medium mb-2 text-textStandard">Instructions:</label>
          <textarea
            value={instructions}
            onChange={(e) => setInstructions(e.target.value)}
            className="w-full p-3 border border-borderSubtle rounded-md bg-transparent text-textStandard focus:border-borderStandard hover:border-borderStandard"
            placeholder="Enter instructions for the gooseling..."
            rows={6}
          />
        </div>

        {/* Activities Section */}
        <div className="mb-6">
          <label className="block font-medium mb-2 text-textStandard">Activities:</label>
          <div className="border border-borderSubtle rounded-md bg-transparent mb-3 max-h-[200px] overflow-y-auto">
            <ul className="divide-y divide-borderSubtle">
              {activities.map((activity, index) => (
                <li key={index} className="flex items-center p-2">
                  <span className="flex-1 text-textStandard">{activity}</span>
                  <button
                    onClick={() => handleRemoveActivity(index)}
                    className="p-1 bg-red-500 text-white rounded-md hover:bg-red-600 ml-2"
                  >
                    ✕
                  </button>
                </li>
              ))}
            </ul>
          </div>
          <div className="flex">
            <input
              type="text"
              value={activityInput}
              onChange={(e) => setActivityInput(e.target.value)}
              onKeyPress={(e) => e.key === 'Enter' && handleAddActivity()}
              className="flex-1 p-3 border border-borderSubtle rounded-l-md bg-transparent text-textStandard focus:border-borderStandard hover:border-borderStandard"
              placeholder="Add new activity..."
            />
            <button
              onClick={handleAddActivity}
              className="px-4 bg-green-500 text-white rounded-r-md hover:bg-green-600"
            >
              Add
            </button>
          </div>
        </div>

        {/* Deep Link Section */}
        <div className="mb-6">
          <label className="block font-medium mb-2 text-textStandard">Share Link:</label>
          <div className="flex">
            <input
              type="text"
              value={deepLink}
              readOnly
              className="flex-1 p-3 border border-borderSubtle rounded-l-md bg-transparent text-textStandard"
            />
            <button
              onClick={() => {
                navigator.clipboard.writeText(deepLink);
                window.electron.logInfo('Deep link copied to clipboard');
              }}
              className="px-4 bg-blue-500 text-white rounded-r-md hover:bg-blue-600 flex items-center"
            >
              <Copy className="w-5 h-5 mr-2" />
              Copy
            </button>
          </div>
        </div>

        {/* Action Buttons */}
        <div className="flex justify-end space-x-4">
          <button
            onClick={async () => {
              setIsLoading(true);
              setError(null);
              try {
                // Get current provider and model from appConfig
                const provider = window.appConfig.get('GOOSE_PROVIDER');
                const model = window.appConfig.get('GOOSE_MODEL');

                // Load the gooseling using the API
                window.electron.logInfo('Loading gooseling with config:', gooseling);

                // Create a new chat window
                window.electron.createChatWindow(
                  undefined, // query
                  undefined, // dir
                  undefined, // version
                  undefined, // resumeSessionId
                  gooseling, // gooseling config
                  undefined // viewType - not gooselingEditor this time
                );
              } catch (err) {
                console.error('Failed to load gooseling:', err);
                setError(err.message || 'Failed to load gooseling');
              } finally {
                setIsLoading(false);
              }
            }}
            disabled={isLoading}
            className={`px-6 py-3 bg-green-500 text-white rounded-md hover:bg-green-600 ${
              isLoading ? 'opacity-50 cursor-not-allowed' : ''
            }`}
          >
            Open Gooseling
          </button>
        </div>

        {error && (
          <div className="mt-4 p-3 bg-red-100 border border-red-400 text-red-700 rounded">
            {error}
          </div>
        )}
      </Card>
    </div>
  );
}
