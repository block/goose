import React from 'react';
import { featureFlags, type FeatureFlags } from '../featureFlags';
import { Card } from './ui/card';
import { Input } from './ui/input';
import Box from './ui/Box';

export default function FeatureFlagsWindow() {
  const [flags, setFlags] = React.useState<FeatureFlags>(featureFlags.getFlags());

  const handleFlagChange = (key: keyof FeatureFlags, value: any) => {
    featureFlags.updateFlag(key, value);
    setFlags({ ...featureFlags.getFlags() });
  };

  return (
    <div className="h-screen flex flex-col">
      {/* Draggable title bar */}
      <div className="h-12 w-full draggable" style={{ WebkitAppRegion: 'drag' }} />
      
      <div className="flex-1 container max-w-2xl mx-auto py-4">
        <div className="space-y-4">
          <div className="flex items-center mb-2">
            <Box size={16} />
            <div className="ml-2">
              <h1 className="text-lg font-semibold leading-none">Feature Flags</h1>
              <p className="text-xs text-gray-500 mt-1">
                Configure experimental features and settings
              </p>
            </div>
          </div>
          
          <Card className="p-4">
            <div className="space-y-4">
              {Object.entries(flags).map(([key, value]) => (
                <div key={key} className="flex flex-col space-y-1">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center">
                      <Box size={12} />
                      <span className="ml-2 text-xs font-medium">
                        {key.split(/(?=[A-Z])/).join(" ")}
                      </span>
                    </div>
                    {typeof value === 'boolean' ? (
                      <button
                        onClick={() => handleFlagChange(key as keyof FeatureFlags, !value)}
                        className={`px-3 py-1 rounded text-xs font-medium ${
                          value 
                            ? 'bg-green-100 text-green-800 hover:bg-green-200' 
                            : 'bg-gray-100 text-gray-800 hover:bg-gray-200'
                        }`}
                      >
                        {value ? 'Enabled' : 'Disabled'}
                      </button>
                    ) : (
                      <Input
                        value={value}
                        onChange={(e) => handleFlagChange(key as keyof FeatureFlags, e.target.value)}
                        className="max-w-[300px] h-7 text-xs"
                      />
                    )}
                  </div>
                  <p className="text-xs text-gray-500 ml-6">
                    {getFeatureFlagDescription(key)}
                  </p>
                </div>
              ))}
            </div>
          </Card>

          <Card className="p-3 bg-gray-50">
            <div className="flex items-center">
              <Box size={12} />
              <p className="text-xs text-gray-500 ml-2">
                ⚡️ Tip: Open a new chat window to see your changes take effect
              </p>
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
}

function getFeatureFlagDescription(key: string): string {
  const descriptions: Record<string, string> = {
    whatCanGooseDoText: "Customize the splash screen button text",
    expandedToolsByDefault: "Show tool outputs expanded by default instead of collapsed"
  };
  return descriptions[key] || "No description available";
}