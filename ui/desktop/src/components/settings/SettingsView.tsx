import { ScrollArea } from '../ui/scroll-area';
import { View, ViewOptions } from '../../utils/navigationUtils';
import ModelsSection from './models/ModelsSection';
import SessionSharingSection from './sessions/SessionSharingSection';
import AppSettingsSection from './app/AppSettingsSection';
import ConfigSettings from './config/ConfigSettings';
import { ExtensionConfig } from '../../api';
import { MainPanelLayout } from '../Layout/MainPanelLayout';
import { useState, useEffect } from 'react';
import ChatSettingsSection from './chat/ChatSettingsSection';
import { CONFIGURATION_ENABLED } from '../../updates';
import SettingsTileNavigation from './SettingsTileNavigation';

export type SettingsViewOptions = {
  deepLinkConfig?: ExtensionConfig;
  showEnvVars?: boolean;
  section?: string;
};

export default function SettingsView({
  onClose,
  setView,
  viewOptions,
}: {
  onClose: () => void;
  setView: (view: View, viewOptions?: ViewOptions) => void;
  viewOptions: SettingsViewOptions;
}) {
  const [activeTab, setActiveTab] = useState('models');

  // Determine initial tab based on section prop
  useEffect(() => {
    if (viewOptions.section) {
      // Map section names to tab values
      const sectionToTab: Record<string, string> = {
        update: 'app',
        models: 'models',
        modes: 'chat',
        sharing: 'sharing',
        styles: 'chat',
        tools: 'chat',
        app: 'app',
        chat: 'chat',
      };

      const targetTab = sectionToTab[viewOptions.section];
      if (targetTab) {
        setActiveTab(targetTab);
      }
    }
  }, [viewOptions.section]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onClose();
      }
    };

    document.addEventListener('keydown', handleKeyDown);

    return () => {
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [onClose]);

  return (
    <>
      <MainPanelLayout removeTopPadding={true}>
        <div className="flex-1 flex flex-col min-h-0">
          {/* Tile Navigation - Full bleed like TopNavigation */}
          <div className="bg-background-muted overflow-hidden relative z-50">
            <div className="pb-0.5 overflow-y-auto">
              <SettingsTileNavigation 
                activeSection={activeTab} 
                onSectionChange={setActiveTab} 
              />
            </div>
          </div>

          {/* Spacer where header was */}
          <div className="h-[50px]"></div>

          {/* Content Area */}
          <div className="flex-1 min-h-0 relative">
            <ScrollArea className="h-full w-full">
              <div className="px-8 pb-8">
                {activeTab === 'models' && (
                  <div className="focus-visible:outline-none focus-visible:ring-0">
                    <ModelsSection setView={setView} />
                  </div>
                )}

                {activeTab === 'chat' && (
                  <div className="focus-visible:outline-none focus-visible:ring-0">
                    <ChatSettingsSection />
                  </div>
                )}

                {activeTab === 'sharing' && (
                  <div className="focus-visible:outline-none focus-visible:ring-0">
                    <SessionSharingSection />
                  </div>
                )}

                {activeTab === 'app' && (
                  <div className="focus-visible:outline-none focus-visible:ring-0">
                    <div className="space-y-8">
                      {CONFIGURATION_ENABLED && <ConfigSettings />}
                      <AppSettingsSection scrollToSection={viewOptions.section} />
                    </div>
                  </div>
                )}
              </div>
            </ScrollArea>
          </div>
        </div>
      </MainPanelLayout>
    </>
  );
}
