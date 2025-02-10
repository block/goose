import React, { useState, useEffect } from 'react';
import { ScrollArea } from '../ui/scroll-area';
import { toast } from 'react-toastify';
import { Settings as SettingsType } from './types';
import {
  FullExtensionConfig,
  addExtension,
  removeExtension,
  BUILT_IN_EXTENSIONS,
} from '../../extensions';
import { ConfigureExtensionModal } from './extensions/ConfigureExtensionModal';
import { ManualExtensionModal } from './extensions/ManualExtensionModal';
import { ConfigureBuiltInExtensionModal } from './extensions/ConfigureBuiltInExtensionModal';
import BackButton from '../ui/BackButton';
import { RecentModelsRadio } from './models/RecentModels';
import { ExtensionItem } from './extensions/ExtensionItem';
import type { View } from '../../ChatWindow';

const EXTENSIONS_DESCRIPTION =
  'The Model Context Protocol (MCP) is a system that allows AI models to securely connect with local or remote resources using standard server setups. It works like a client-server setup and expands AI capabilities using three main components: Prompts, Resources, and Tools.';

const EXTENSIONS_SITE_LINK = 'https://block.github.io/goose/v1/extensions/';

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
  // @ts-expect-error "we actually do always have all the properties required for builtins, but tsc cannot tell for some reason"
  extensions: BUILT_IN_EXTENSIONS,
};

// We'll accept two props:
// onClose: to go back to chat
// setView: to switch to moreModels, configureProviders, etc.
export default function Settings({
  onClose,
  setView,
}: {
  onClose: () => void;
  setView: (view: View) => void;
}) {
  // We'll read query params from window.location instead of react-router's useLocation
  const [searchParams] = useState(() => new URLSearchParams(window.location.search));

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

  // Listen for settings updates from extension storage
  useEffect(() => {
    const handleSettingsUpdate = (_: any) => {
      const saved = localStorage.getItem('user_settings');
      if (saved) {
        let currentSettings = JSON.parse(saved);
        setSettings(currentSettings);
      }
    };

    window.electron.on('settings-updated', handleSettingsUpdate);
    return () => {
      window.electron.off('settings-updated', handleSettingsUpdate);
    };
  }, []);

  // Handle URL parameters for auto-opening extension configuration
  useEffect(() => {
    const extensionId = searchParams.get('extensionId');
    const showEnvVars = searchParams.get('showEnvVars');

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
    // We only run this once on load
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settings.extensions]);

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

    let response: Response;

    if (newEnabled) {
      response = await addExtension(extension);
    } else {
      response = await removeExtension(extension.name);
    }

    if (!response.ok) {
      setSettings(originalSettings);
    }
  };

  const handleExtensionRemove = async () => {
    if (!extensionBeingConfigured) return;

    const response = await removeExtension(extensionBeingConfigured.name, true);

    if (response.ok) {
      toast.success(`Successfully removed ${extensionBeingConfigured.name} extension`);

      // Remove from localstorage
      setSettings((prev) => ({
        ...prev,
        extensions: prev.extensions.filter((ext) => ext.id !== extensionBeingConfigured.id),
      }));
      setExtensionBeingConfigured(null);
    }
  };

  const handleExtensionConfigSubmit = () => {
    setExtensionBeingConfigured(null);
  };

  const isBuiltIn = (extensionId: string) => {
    return BUILT_IN_EXTENSIONS.some((builtIn) => builtIn.id === extensionId);
  };

  return (
    <div className="h-screen w-full">
      <div className="relative flex items-center h-[36px] w-full bg-bgSubtle"></div>

      <ScrollArea className="h-full w-full">
        <div className="flex flex-col pb-24">
          <div className="px-8 pt-6 pb-4">
            <BackButton
              onClick={() => {
                // Instead of navigate('/chat/1', { replace: true });
                onClose();
              }}
            />
            <h1 className="text-3xl font-medium text-textStandard mt-1">Settings</h1>
          </div>

          {/* Content Area */}
          <div className="flex-1 py-8 pt-[20px]">
            <div className="space-y-8">
              <section id="models">
                <div className="flex justify-between items-center mb-6 border-b border-borderSubtle px-8">
                  <h2 className="text-xl font-medium text-textStandard">Models</h2>
                  <button
                    onClick={() => {
                      // Instead of navigate('/settings/more-models'):
                      setView('moreModels');
                    }}
                    className="text-indigo-500 hover:text-indigo-600 text-sm"
                  >
                    Browse
                  </button>
                </div>
                <div className="px-8">
                  <RecentModelsRadio />
                </div>
              </section>

              <section id="extensions">
                <div className="flex justify-between items-center mb-6 border-b border-borderSubtle px-8">
                  <h2 className="text-xl font-semibold text-textStandard">Extensions</h2>
                  <div className="flex gap-4">
                    <button
                      onClick={() => setIsManualModalOpen(true)}
                      className="text-indigo-500 hover:text-indigo-600 text-sm"
                      title="Add Manually"
                    >
                      Add
                    </button>

                    <a
                      href={EXTENSIONS_SITE_LINK}
                      target="_blank"
                      className="text-indigo-500 hover:text-indigo-600 text-sm"
                      rel="noreferrer"
                    >
                      Browse
                    </a>
                  </div>
                </div>

                <div className="px-8">
                  <p className="text-sm text-textStandard mb-4">{EXTENSIONS_DESCRIPTION}</p>

                  {settings.extensions.length === 0 ? (
                    <p className="text-textSubtle text-center py-4">No Extensions Added</p>
                  ) : (
                    settings.extensions.map((ext) => (
                      <ExtensionItem
                        key={ext.id}
                        {...ext}
                        canConfigure={true}
                        onToggle={handleExtensionToggle}
                        onConfigure={(extension) => setExtensionBeingConfigured(extension)}
                      />
                    ))
                  )}
                </div>
              </section>
            </div>
          </div>
        </div>
      </ScrollArea>

      {extensionBeingConfigured && isBuiltIn(extensionBeingConfigured.id) ? (
        <ConfigureBuiltInExtensionModal
          isOpen={!!extensionBeingConfigured && isBuiltIn(extensionBeingConfigured.id)}
          onClose={() => {
            setExtensionBeingConfigured(null);
          }}
          extension={extensionBeingConfigured}
          onSubmit={handleExtensionConfigSubmit}
        />
      ) : (
        <ConfigureExtensionModal
          isOpen={!!extensionBeingConfigured}
          onClose={() => {
            setExtensionBeingConfigured(null);
          }}
          extension={extensionBeingConfigured}
          onSubmit={handleExtensionConfigSubmit}
          onRemove={handleExtensionRemove}
        />
      )}

      <ManualExtensionModal
        isOpen={isManualModalOpen}
        onClose={() => setIsManualModalOpen(false)}
        onSubmit={async (extension) => {
          const response = await addExtension(extension);

          if (response.ok) {
            setSettings((prev) => ({
              ...prev,
              extensions: [...prev.extensions, extension],
            }));
            setIsManualModalOpen(false);
          } else {
            // TODO - Anything for the UI state beyond validation?
          }
        }}
      />
    </div>
  );
}
