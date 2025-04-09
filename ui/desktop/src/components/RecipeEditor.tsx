import React, { useState, useEffect } from 'react';
import { Recipe } from '../recipe';
import { Buffer } from 'buffer';
import { type View } from '../App';
import { ExtensionItem } from './settings/extensions/ExtensionItem';
import { FullExtensionConfig } from '../extensions';
import { ChevronRight } from './icons/ChevronRight';
import Back from './icons/Back';
import { Bars } from './icons/Bars';
import { Geese } from './icons/Geese';
import Copy from './icons/Copy';

interface RecipeEditorProps {
  config?: Recipe;
  onClose: () => void;
  onSave?: (config: Recipe) => void;
  setView: (view: View, viewOptions?: Record<string, any>) => void;
}

// Function to generate a deep link from a recipe
function generateDeepLink(recipe: Recipe): string {
  const configBase64 = Buffer.from(JSON.stringify(recipe)).toString('base64');
  return `goose://bot?config=${configBase64}`;
}

export default function RecipeEditor({ config, onClose, onSave, setView }: RecipeEditorProps) {
  // State management
  const [botConfig, setBotConfig] = useState<Recipe | undefined>(config);
  const [title, setTitle] = useState(config?.title || '');
  const [description, setDescription] = useState(config?.description || '');
  const [instructions, setInstructions] = useState(config?.instructions || '');
  const [activities, setActivities] = useState<string[]>(config?.activities || []);
  const [availableExtensions, setAvailableExtensions] = useState<FullExtensionConfig[]>([]);
  const [selectedExtensions, setSelectedExtensions] = useState<string[]>(
    config?.extensions?.map((e) => e.id) || []
  );
  const [newActivity, setNewActivity] = useState('');

  // Section visibility state
  const [activeSection, setActiveSection] = useState<
    'none' | 'activities' | 'instructions' | 'extensions'
  >('none');

  // Load extensions
  useEffect(() => {
    const loadExtensions = () => {
      const userSettingsStr = localStorage.getItem('user_settings');
      if (userSettingsStr) {
        const userSettings = JSON.parse(userSettingsStr);
        setAvailableExtensions(userSettings.extensions || []);
      }
    };
    loadExtensions();
  }, []);

  const handleExtensionToggle = (id: string) => {
    console.log('Toggling extension:', id);
    setSelectedExtensions((prev) => {
      const isSelected = prev.includes(id);
      const newState = isSelected ? prev.filter((extId) => extId !== id) : [...prev, id];
      return newState;
    });
  };

  const handleAddActivity = () => {
    if (newActivity.trim()) {
      setActivities((prev) => [...prev, newActivity.trim()]);
      setNewActivity('');
    }
  };

  const handleRemoveActivity = (activity: string) => {
    setActivities((prev) => prev.filter((a) => a !== activity));
  };

  const getCurrentConfig = (): Recipe => ({
    ...botConfig,
    title,
    description,
    instructions,
    activities,
    extensions: selectedExtensions
      .map((id) => {
        const extension = availableExtensions.find((e) => e.id === id);
        if (!extension) return null;

        // Create a clean copy of the extension
        const cleanExtension = { ...extension, enabled: true };

        // If the extension has env_vars, preserve keys but clear values
        if (cleanExtension.env_keys) {
          cleanExtension.env_keys = Object.keys(cleanExtension.env_keys).reduce(
            (acc, key) => {
              acc[key] = '';
              return acc;
            },
            {} as Record<string, string>
          );
        }

        return cleanExtension;
      })
      .filter(Boolean) as FullExtensionConfig[],
  });

  const deeplink = generateDeepLink(getCurrentConfig());

  // Render expanded section content
  const renderSectionContent = () => {
    switch (activeSection) {
      case 'activities':
        return (
          <div className="p-6 pt-10">
            <button onClick={() => setActiveSection('none')} className="mb-6">
              <Back className="w-6 h-6" />
            </button>
            <div className="py-2">
              <Bars className="w-6 h-6 text-gray-500" />
            </div>
            <div className="mb-8 mt-6">
              <h2 className="text-2xl font-medium mb-2">Activities</h2>
              <p className="text-gray-600">
                The top-line prompts and activities that will display within your goose home page.
              </p>
            </div>
            <div className="space-y-4">
              <div className="flex flex-wrap gap-3">
                {activities.map((activity, index) => (
                  <div
                    key={index}
                    className="inline-flex items-center bg-white border-2 border-gray-300 rounded-full px-4 py-2 text-sm"
                  >
                    <span>{activity}</span>
                    <button
                      onClick={() => handleRemoveActivity(activity)}
                      className="ml-2 text-gray-900 hover:text-gray-600 transition-colors"
                    >
                      ×
                    </button>
                  </div>
                ))}
              </div>
              <div className="flex gap-3 mt-6">
                <input
                  type="text"
                  value={newActivity}
                  onChange={(e) => setNewActivity(e.target.value)}
                  onKeyPress={(e) => e.key === 'Enter' && handleAddActivity()}
                  className="flex-1 px-4 py-3 bg-gray-50 rounded-xl placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-black"
                  placeholder="Add new activity..."
                />
                <button
                  onClick={handleAddActivity}
                  className="px-5 py-3 bg-black text-white rounded-xl hover:bg-gray-900 transition-colors"
                >
                  Add activity
                </button>
              </div>
            </div>
          </div>
        );

      case 'instructions':
        return (
          <div className="p-6 pt-10">
            <button onClick={() => setActiveSection('none')} className="mb-6">
              <Back className="w-6 h-6" />
            </button>
            <div className="py-2">
              <Bars className="w-6 h-6 text-gray-500" />
            </div>
            <div className="mb-8 mt-6">
              <h2 className="text-2xl font-medium mb-2">Instructions</h2>
              <p className="text-gray-600">
                Hidden instructions that will be passed to the provider to help direct and add
                context to your responses.
              </p>
            </div>
            <textarea
              value={instructions}
              onChange={(e) => setInstructions(e.target.value)}
              className="w-full h-96 p-4 bg-gray-50 rounded-xl resize-none focus:outline-none focus:ring-2 focus:ring-black"
              placeholder="Enter instructions..."
            />
          </div>
        );

      case 'extensions':
        return (
          <div className="p-6 pt-10">
            <button onClick={() => setActiveSection('none')} className="mb-6">
              <Back className="w-6 h-6" />
            </button>
            <div className="py-2">
              <Bars className="w-6 h-6 text-gray-500" />
            </div>
            <div className="mb-8 mt-6">
              <h2 className="text-2xl font-medium mb-2">Extensions</h2>
              <p className="text-gray-600">
                Choose which extensions will be available to your agent.
              </p>
            </div>
            <div className="grid grid-cols-2 gap-4">
              {availableExtensions.map((extension) => (
                <button
                  key={extension.id}
                  className="p-4 border border-gray-200 rounded-lg flex justify-between items-center w-full text-left hover:bg-gray-50"
                  onClick={() => handleExtensionToggle(extension.id)}
                >
                  <div>
                    <h3 className="font-medium">{extension.name || 'File viewer'}</h3>
                    <p className="text-sm text-gray-600">Standard config</p>
                  </div>
                  <div className="relative inline-block w-10 align-middle select-none">
                    <div
                      className={`w-10 h-6 rounded-full transition-colors duration-200 ease-in-out ${
                        selectedExtensions.includes(extension.id) ? 'bg-black' : 'bg-gray-300'
                      }`}
                    >
                      <div
                        className={`w-6 h-6 rounded-full bg-white border-2 transform transition-transform duration-200 ease-in-out ${
                          selectedExtensions.includes(extension.id)
                            ? 'translate-x-4 border-black'
                            : 'translate-x-0 border-gray-300'
                        }`}
                      />
                    </div>
                  </div>
                </button>
              ))}
            </div>
          </div>
        );

      default:
        return (
          <div className="space-y-4 py-4">
            <div>
              <h2 className="text-lg font-medium mb-2">Agent</h2>
              <input
                type="text"
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                className="w-full p-3 border border-gray-200 rounded-lg"
                placeholder="Agent Name"
              />
            </div>

            <div>
              <input
                type="text"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                className="w-full p-3 border border-gray-200 rounded-lg"
                placeholder="Description"
              />
            </div>

            {/* Section buttons */}
            <button
              onClick={() => setActiveSection('activities')}
              className="w-full flex items-start justify-between p-4 border border-gray-200 rounded-lg"
            >
              <div className="text-left">
                <h3 className="font-medium">Activities</h3>
                <p className="text-gray-500 text-sm">
                  Starting activities present in the home panel on a fresh goose session
                </p>
              </div>
              <ChevronRight className="w-5 h-5 mt-1" />
            </button>

            <button
              onClick={() => setActiveSection('instructions')}
              className="w-full flex items-start justify-between p-4 border border-gray-200 rounded-lg"
            >
              <div className="text-left">
                <h3 className="font-medium">Instructions</h3>
                <p className="text-gray-500 text-sm">
                  Starting activities present in the home panel on a fresh goose session
                </p>
              </div>
              <ChevronRight className="w-5 h-5 mt-1" />
            </button>

            <button
              onClick={() => setActiveSection('extensions')}
              className="w-full flex items-start justify-between p-4 border border-gray-200 rounded-lg"
            >
              <div className="text-left">
                <h3 className="font-medium">Extensions</h3>
                <p className="text-gray-500 text-sm">
                  Starting activities present in the home panel on a fresh goose session
                </p>
              </div>
              <ChevronRight className="w-5 h-5 mt-1" />
            </button>

            {/* Deep Link Display */}
            <div className="w-full p-4 bg-gray-50 rounded-lg flex items-center justify-between">
              <code className="text-sm text-gray-600 truncate">{deeplink}</code>
              <button onClick={() => navigator.clipboard.writeText(deeplink)} className="ml-2">
                <Copy className="w-5 h-5" />
              </button>
            </div>

            {/* Action Buttons */}
            <div className="flex flex-col space-y-2 pt-4">
              <button
                onClick={() => {
                  const updatedConfig = getCurrentConfig();
                  window.electron.createChatWindow(
                    undefined,
                    undefined,
                    undefined,
                    undefined,
                    updatedConfig,
                    undefined
                  );
                }}
                className="w-full p-3 bg-black text-white rounded-lg hover:bg-gray-900"
              >
                Open agent
              </button>
              <button
                onClick={() => window.electron.closeWindow()}
                className="w-full p-3 text-gray-600 rounded-lg hover:bg-gray-100"
              >
                Cancel
              </button>
            </div>
          </div>
        );
    }
  };

  return (
    <div className="flex flex-col w-full h-screen bg-white max-w-3xl mx-auto">
      {activeSection === 'none' && (
        <div className="flex flex-col items-center mb-6 px-6 pt-10">
          <div className="w-12 h-12 bg-white rounded-full flex items-center justify-center mb-4">
            <Geese className="w-5 h-5" />
          </div>
          <h1 className="text-2xl font-medium text-center">Create custom agent</h1>
          <p className="text-gray-500 text-center mt-2 text-sm">
            Your custom agent can be shared with others
          </p>
        </div>
      )}
      <div className="flex-1 overflow-y-auto px-6">{renderSectionContent()}</div>
    </div>
  );
}
