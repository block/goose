import React, { useState, useEffect } from 'react';
import { ScrollArea } from '../ui/scroll-area';
import { useNavigate, useLocation } from 'react-router-dom';
import { Plus } from 'lucide-react';
import { Settings as SettingsType } from './types';
import { FullExtensionConfig, replaceWithShims, extendGoosed } from '../../extensions';
import { ConfigureExtensionModal } from './extensions/ConfigureExtensionModal';
import { ManualExtensionModal } from './extensions/ManualExtensionModal';
import { showToast } from '../ui/toast';
import BackButton from '../ui/BackButton';
import { RecentModelsRadio } from './models/RecentModels';
import { ExtensionItem } from './extensions/ExtensionItem';
import { getApiUrl, getSecretKey } from '../../config';

const EXTENSIONS_DESCRIPTION =
  'The Model Context Protocol (MCP) is a system that allows AI models to securely connect with local or remote resources using standard server setups. It works like a client-server setup and expands AI capabilities using three main components: Prompts, Resources, and Tools.';

const DEFAULT_SETTINGS: SettingsType = {
  models: [
    {
      id: 'gpt4',
      name: 'GPT 4.0',
      description: 'Standard config',
      enabled: false,
    },
    {
      id: 'gpt4lite',
      name: 'GPT 4.0 lite',
      description: 'Standard config',
      enabled: false,
    },
    {
      id: 'claude',
      name: 'Claude',
      description: 'Standard config',
      enabled: true,
    },
  ],
  extensions: [],
};

const BUILT_IN_EXTENSIONS = [
  {
    id: 'jetbrains',
    name: 'Jetbrains',
    type: 'stdio',
    cmd: 'goosed',
    args: ['mcp', 'jetbrains'],
    description: 'Integration with any Jetbrains IDE',
    enabled: false,
    env_keys: [],
  },
  {
    id: 'nondeveloper',
    name: 'Non-Developer assistant',
    type: 'stdio',
    cmd: 'goosed',
    args: ['mcp', 'nondeveloper'],
    description: "General assisant tools that don't require you to be a developer or engineer.",
    enabled: false,
    env_keys: [],
  },
  {
    id: 'memory',
    name: 'Memory',
    type: 'stdio',
    cmd: 'goosed',
    args: ['mcp', 'memory'],
    description: 'Teach goose your preferences as you go.',
    enabled: false,
    env_keys: [],
  },
];

export default function Settings() {
  const navigate = useNavigate();
  const location = useLocation();

  const [settings, setSettings] = React.useState<SettingsType>(() => {
    const saved = localStorage.getItem('user_settings');
    window.electron.logInfo('Settings: ' + saved);
    let currentSettings = saved ? JSON.parse(saved) : DEFAULT_SETTINGS;

    // Ensure built-in extensions are included if not already present
    BUILT_IN_EXTENSIONS.forEach((builtIn) => {
      const exists = currentSettings.extensions.some(
        (ext: FullExtensionConfig) => ext.id === builtIn.id
      );
      if (!exists) {
        currentSettings.extensions.push(builtIn);
      }
    });

    return currentSettings;
  });

  const [extensionBeingConfigured, setExtensionBeingConfigured] =
    useState<FullExtensionConfig | null>(null);
  const [isManualModalOpen, setIsManualModalOpen] = useState(false);

  // Persist settings changes
  React.useEffect(() => {
    localStorage.setItem('user_settings', JSON.stringify(settings));
  }, [settings]);

  // Handle URL parameters for auto-opening extension configuration
  useEffect(() => {
    const params = new URLSearchParams(location.search);
    const extensionId = params.get('extensionId');
    const showEnvVars = params.get('showEnvVars');

    if (extensionId && showEnvVars === 'true') {
      // Find the extension in settings
      const extension = settings.extensions.find((ext) => ext.id === extensionId);
      if (extension) {
        // Auto-open the configuration modal
        setExtensionBeingConfigured(extension);
        // Scroll to extensions section
        const element = document.getElementById('extensions');
        if (element) {
          element.scrollIntoView({ behavior: 'smooth' });
        }
      }
    }
  }, [location.search, settings.extensions]);

  const handleExtensionToggle = async (extensionId: string) => {
    // Find the extension to get its current state
    const extension = settings.extensions.find((ext) => ext.id === extensionId);

    if (!extension) return;

    const newEnabled = !extension.enabled;

    const originalSettings = settings;

    // Optimistically update local component state
    setSettings((prev) => ({
      ...prev,
      extensions: prev.extensions.map((ext) =>
        ext.id === extensionId ? { ...ext, enabled: newEnabled } : ext
      ),
    }));

    try {
      const endpoint = newEnabled ? '/extensions/add' : '/extensions/remove';

      // Full config for adding - only "name" as a string for removing
      const body = newEnabled
        ? {
            type: extension.type,
            ...(extension.type === 'stdio' && {
              cmd: await replaceWithShims(extension.cmd),
              args: extension.args || [],
            }),
            ...(extension.type === 'sse' && {
              uri: extension.uri,
            }),
            ...(extension.type === 'builtin' && {
              name: extension.name,
            }),
            env_keys: extension.env_keys,
          }
        : extension.name;

      const response = await fetch(getApiUrl(endpoint), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': getSecretKey(),
        },
        body: JSON.stringify(body),
      });

      if (!response.ok) {
        throw new Error(`Failed to ${newEnabled ? 'enable' : 'disable'} extension`);
      }

      showToast(`Successfully ${newEnabled ? 'enabled' : 'disabled'} extension`, 'success');
    } catch (error) {
      setSettings(originalSettings);
      showToast(`Error ${newEnabled ? 'enabling' : 'disabling'} extension`, 'error');
      console.error('Error toggling extension:', error);
    }
  };

  const handleExtensionRemove = async () => {
    if (!extensionBeingConfigured) return;

    try {
      // First disable the extension if it's enabled
      if (extensionBeingConfigured.enabled) {
        const response = await fetch(getApiUrl('/extensions/remove'), {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'X-Secret-Key': getSecretKey(),
          },
          body: JSON.stringify(extensionBeingConfigured.name),
        });

        if (!response.ok) {
          throw new Error('Failed to remove extension from backend');
        }
      }

      // Then remove it from the local settings
      setSettings((prev) => ({
        ...prev,
        extensions: prev.extensions.filter((ext) => ext.id !== extensionBeingConfigured.id),
      }));

      showToast(`Successfully removed ${extensionBeingConfigured.name} extension`, 'success');
      setExtensionBeingConfigured(null);
      navigate('/settings', { replace: true });
    } catch (error) {
      console.error('Error removing extension:', error);
      showToast('Failed to remove extension', 'error');
    }
  };

  const handleNavClick = (section: string, e: React.MouseEvent) => {
    e.preventDefault();
    const scrollArea = document.querySelector('[data-radix-scroll-area-viewport]');
    const element = document.getElementById(section.toLowerCase());

    if (scrollArea && element) {
      const topPos = element.offsetTop;
      scrollArea.scrollTo({
        top: topPos,
        behavior: 'smooth',
      });
    }
  };

  const handleExtensionConfigSubmit = () => {
    setExtensionBeingConfigured(null);
    // Clear the URL parameters after configuration
    navigate('/settings', { replace: true });
  };

  return (
    <div className="h-screen w-full pt-[36px]">
      <div className="h-full w-full bg-white dark:bg-gray-800 overflow-hidden p-2 pt-0">
        <ScrollArea className="h-full w-full">
          <div className="flex min-h-full">
            {/* Left Navigation */}
            <div className="w-48 border-gray-100 dark:border-gray-700 px-2 pt-2">
              <div className="sticky top-8">
                <BackButton
                  onClick={() => {
                    navigate('/chat/1', { replace: true });
                  }}
                  className="mb-4"
                />
                <div className="space-y-2">
                  {['Models', 'Extensions'].map((section) => (
                    <button
                      key={section}
                      onClick={(e) => handleNavClick(section, e)}
                      className="block w-full text-left px-3 py-2 rounded-lg transition-colors
                                                  text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800"
                    >
                      {section}
                    </button>
                  ))}
                </div>
              </div>
            </div>

            {/* Content Area */}
            <div className="flex-1 px-16 py-8 pt-[20px]">
              <div className="space-y-12">
                <section id="models">
                  <div className="flex justify-between items-center mb-4">
                    <h2 className="text-xl font-semibold text-textStandard">Models</h2>
                    <button
                      onClick={() => navigate('/settings/more-models')}
                      className="text-indigo-500 hover:text-indigo-600 font-medium"
                    >
                      More Models
                    </button>
                  </div>
                  <RecentModelsRadio />
                </section>

                <section id="extensions">
                  <div className="flex justify-between items-center mb-4">
                    <h2 className="text-xl font-semibold text-textStandard">Extensions</h2>
                    <div className="flex gap-4">
                      <button
                        onClick={() => setIsManualModalOpen(true)}
                        className="text-indigo-500 hover:text-indigo-600 font-medium"
                        title="Add Manually"
                      >
                        <Plus className="h-4 w-4" />
                      </button>{' '}
                      |
                      <button
                        onClick={() =>
                          window.electron.openInChrome(
                            'https://silver-disco-nvm6v4e.pages.github.io/'
                          )
                        }
                        className="text-indigo-500 hover:text-indigo-600 font-medium"
                      >
                        Browse Extensions
                      </button>
                    </div>
                  </div>
                  <p className="text-sm text-textStandard mb-4">{EXTENSIONS_DESCRIPTION}</p>
                  {settings.extensions.length === 0 ? (
                    <p className="text-gray-500 dark:text-gray-400 text-center py-4">
                      No Extensions Added
                    </p>
                  ) : (
                    settings.extensions.map((ext) => (
                      <ExtensionItem
                        key={ext.id}
                        {...ext}
                        onToggle={handleExtensionToggle}
                        onConfigure={(extension) => setExtensionBeingConfigured(extension)}
                      />
                    ))
                  )}
                </section>
              </div>
            </div>
          </div>
        </ScrollArea>
      </div>

      <ConfigureExtensionModal
        isOpen={!!extensionBeingConfigured}
        onClose={() => {
          setExtensionBeingConfigured(null);
          // Clear URL parameters when closing manually
          navigate('/settings', { replace: true });
        }}
        extension={extensionBeingConfigured}
        onSubmit={handleExtensionConfigSubmit}
        onRemove={handleExtensionRemove}
      />

      <ManualExtensionModal
        isOpen={isManualModalOpen}
        onClose={() => setIsManualModalOpen(false)}
        onSubmit={async (extension) => {
          // Create config for extendGoosed
          const config = {
            type: extension.type,
            ...(extension.type === 'stdio' && {
              cmd: await replaceWithShims(extension.cmd),
              args: extension.args || [],
            }),
            ...(extension.type === 'sse' && {
              uri: extension.uri,
            }),
            ...(extension.type === 'builtin' && {
              name: extension.name,
            }),
            env_keys: extension.env_keys,
          };

          try {
            const success = await extendGoosed(config);
            if (success) {
              setSettings((prev) => ({
                ...prev,
                extensions: [...prev.extensions, extension],
              }));
              setIsManualModalOpen(false);
              showToast('Extension added successfully', 'success');
            } else {
              throw new Error('Failed to add extension');
            }
          } catch (error) {
            console.error('Error adding extension:', error);
            showToast('Error adding extension', 'error');
          }
        }}
      />
    </div>
  );
}
