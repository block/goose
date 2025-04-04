import React, { useMemo, useState, useEffect, useRef } from 'react';
import { Buffer } from 'buffer';
import Copy from '../icons/Copy';
import { Card } from './card';
import { Gooseling, loadGooseling } from '../../api-client';

interface DeepLinkModalProps {
  botConfig: Gooseling;
  onClose: () => void;
  onOpen: () => void;
}

// Function to generate a deep link from a gooseling
export function generateDeepLink(gooseling: Gooseling): string {
  const configBase64 = Buffer.from(JSON.stringify(gooseling)).toString('base64');
  return `goose://gooseling?config=${configBase64}`;
}

export function DeepLinkModal({ botConfig: initialConfig, onClose, onOpen }: DeepLinkModalProps) {
  // Create editable state for the gooseling
  const [gooseling, setGooseling] = useState<Gooseling>(initialConfig);
  const [title, setTitle] = useState(initialConfig.title || '');
  const [description, setDescription] = useState(initialConfig.description || '');
  const [instructions, setInstructions] = useState(initialConfig.instructions || '');
  const [activities, setActivities] = useState<string[]>(initialConfig.activities || []);
  const [activityInput, setActivityInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Generate the deep link using the current gooseling
  const deepLink = useMemo(() => {
    const currentConfig = {
      ...gooseling,
      title,
      description,
      instructions,
      activities,
    };
    return generateDeepLink(currentConfig);
  }, [gooseling, title, description, instructions, activities]);

  // Update the gooseling when fields change
  useEffect(() => {
    setGooseling({
      ...gooseling,
      title,
      description,
      instructions,
      activities,
    });
  }, [title, description, instructions, activities]);

  // Handle Esc key press
  useEffect(() => {
    const handleEscKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };

    document.addEventListener('keydown', handleEscKey);
    return () => document.removeEventListener('keydown', handleEscKey);
  }, [onClose]);

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

  // Handle opening the gooseling
  const handleOpenGooseling = async () => {
    setIsLoading(true);
    setError(null);

    try {
      // Get current provider and model from appConfig
      const provider = window.appConfig.get('provider');
      const model = window.appConfig.get('model');

      // Load the gooseling using the API
      const currentConfig = {
        ...gooseling,
        title,
        description,
        instructions,
        activities,
      };

      const response = await loadGooseling({
        gooseling: currentConfig,
        provider,
        model,
      });

      // Create a new chat window with the loaded gooseling
      window.electron.createChatWindow(
        undefined,
        undefined,
        response.version,
        undefined,
        currentConfig
      );

      onOpen();
    } catch (err) {
      setError(err.message);
      window.electron.logInfo('Failed to load gooseling:');
      window.electron.logInfo(err.message);
    } finally {
      setIsLoading(false);
    }
  };

  // Reference for the modal content
  const modalRef = useRef<HTMLDivElement>(null);

  // Handle click outside the modal
  const handleBackdropClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (modalRef.current && !modalRef.current.contains(e.target as Node)) {
      onClose();
    }
  };

  return (
    <div
      className="fixed inset-0 bg-black/20 dark:bg-white/20 backdrop-blur-sm transition-colors flex items-center justify-center p-4 z-50"
      onClick={handleBackdropClick}
    >
      <Card
        ref={modalRef}
        className="relative w-[700px] max-w-full bg-bgApp rounded-xl my-10 max-h-[90vh] flex flex-col shadow-lg"
      >
        <div className="p-8 overflow-y-auto" style={{ maxHeight: 'calc(90vh - 32px)' }}>
          <div className="flex flex-col">
            <h2 className="text-2xl font-bold mb-4 text-textStandard">Gooseling Created!</h2>
            <p className="mb-4 text-textStandard">
              Your gooseling has been created successfully. You can share or open it below:
            </p>

            {/* Name and Description Fields */}
            <div className="mb-4">
              <label className="block font-medium mb-1 text-textStandard">Name:</label>
              <input
                type="text"
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                className="w-full p-2 border border-borderSubtle rounded-md bg-transparent text-textStandard focus:border-borderStandard hover:border-borderStandard"
                placeholder="Enter gooseling name..."
              />
            </div>

            <div className="mb-6">
              <label className="block font-medium mb-1 text-textStandard">Description:</label>
              <textarea
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                className="w-full p-2 border border-borderSubtle rounded-md bg-transparent text-textStandard focus:border-borderStandard hover:border-borderStandard"
                placeholder="Enter gooseling description..."
                rows={2}
              />
            </div>

            {/* Sharable Gooseling Section */}
            <div className="mb-6">
              <label className="block font-medium mb-1 text-textStandard">
                Sharable Gooseling:
              </label>
              <div className="flex items-center">
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
                  className="p-2 bg-blue-500 text-white rounded-r-md hover:bg-blue-600 flex items-center justify-center min-w-[100px]"
                >
                  <Copy className="w-5 h-5 mr-1" />
                  Copy
                </button>
              </div>
            </div>

            {/* Action Buttons */}
            <div className="flex mb-6">
              <button
                onClick={handleOpenGooseling}
                disabled={isLoading}
                className={`px-5 py-2.5 bg-green-500 text-white rounded-md hover:bg-green-600 flex-1 mr-2 ${
                  isLoading ? 'opacity-50 cursor-not-allowed' : ''
                }`}
              >
                {isLoading ? 'Opening...' : 'Open Gooseling'}
              </button>
              <button
                onClick={onClose}
                className="px-5 py-2.5 bg-gray-500 text-white rounded-md hover:bg-gray-600 flex-1"
              >
                Close
              </button>
            </div>

            {error && (
              <div className="mb-4 p-3 bg-red-100 border border-red-400 text-red-700 rounded">
                {error}
              </div>
            )}

            <h3 className="text-lg font-medium mb-3 text-textStandard">Edit Instructions:</h3>
            <div className="mb-4">
              <div className="border border-borderSubtle rounded-md bg-transparent max-h-[120px] overflow-y-auto">
                <textarea
                  id="instructions"
                  value={instructions}
                  onChange={(e) => setInstructions(e.target.value)}
                  className="w-full p-3 bg-transparent text-textStandard focus:outline-none"
                  placeholder="Instructions for the gooseling..."
                />
              </div>
            </div>

            {/* Activities Section */}
            <div className="mb-4">
              <label className="block font-medium mb-1 text-textStandard">Activities:</label>
              <div className="border border-borderSubtle rounded-md bg-transparent max-h-[120px] overflow-y-auto mb-2">
                <ul className="divide-y divide-borderSubtle">
                  {activities.map((activity, index) => (
                    <li key={index} className="flex items-center">
                      <span className="flex-1 p-2 text-textStandard">{activity}</span>
                      <button
                        onClick={() => handleRemoveActivity(index)}
                        className="p-1 bg-red-500 text-white rounded-md hover:bg-red-600 m-1"
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
                  className="flex-1 p-2 border border-borderSubtle rounded-l-md bg-transparent text-textStandard focus:border-borderStandard hover:border-borderStandard"
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
          </div>
        </div>
      </Card>
    </div>
  );
}
