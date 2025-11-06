import { ScrollArea } from '../ui/scroll-area';
import { Tabs, TabsContent } from '../ui/tabs';
import { View, ViewOptions } from '../../utils/navigationUtils';
import ModelsSection from './models/ModelsSection';
import SessionSharingSection from './sessions/SessionSharingSection';
import AppSettingsSection from './app/AppSettingsSection';
import ConfigSettings from './config/ConfigSettings';
import { ExtensionConfig } from '../../api';
import { MainPanelLayout } from '../Layout/MainPanelLayout';
import { Bot, Share2, Monitor, MessageSquare } from 'lucide-react';
import { useState, useEffect } from 'react';
import ChatSettingsSection from './chat/ChatSettingsSection';
import { CONFIGURATION_ENABLED } from '../../updates';

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

  const settingsSections = [
    {
      id: 'models',
      label: 'Models',
      icon: Bot,
      description: 'Configure AI models',
    },
    {
      id: 'chat',
      label: 'Chat',
      icon: MessageSquare,
      description: 'Chat preferences',
    },
    {
      id: 'sharing',
      label: 'Session',
      icon: Share2,
      description: 'Session sharing',
    },
    {
      id: 'app',
      label: 'App',
      icon: Monitor,
      description: 'Application settings',
    },
  ];

  return (
    <>
      <MainPanelLayout removeTopPadding={true} backgroundColor="bg-background-muted">
        <div className="flex-1 flex flex-col min-h-0">
          <Tabs value={activeTab} onValueChange={setActiveTab} className="h-full flex flex-col">
            {/* Settings Tiles Navigation */}
            <div className="bg-background-muted px-0.5 pt-0.5 pb-0.5">
              <div className="grid grid-cols-2 md:grid-cols-4 gap-0.5">
                {settingsSections.map((section, index) => {
                  const IconComponent = section.icon;
                  const isActive = activeTab === section.id;
                  
                  return (
                    <button
                      key={section.id}
                      onClick={() => setActiveTab(section.id)}
                      className={`
                        relative flex flex-col items-start justify-between
                        bg-background-default rounded-2xl
                        px-4 py-4 min-h-[140px]
                        transition-all duration-200
                        hover:bg-background-medium
                        no-drag
                        animate-in slide-in-from-top-4 fade-in
                        ${isActive ? 'bg-background-medium' : ''}
                      `}
                      style={{
                        animationDelay: `${index * 50}ms`,
                        animationDuration: '400ms',
                        animationFillMode: 'backwards',
                      }}
                      data-testid={`settings-${section.id}-tab`}
                    >
                      {/* Icon and Label at bottom */}
                      <div className="mt-auto w-full">
                        <IconComponent className="w-5 h-5 mb-2 text-text-default" />
                        <h2 className="text-xl font-light text-left text-text-default">{section.label}</h2>
                        <p className="text-xs text-text-muted mt-1 text-left">{section.description}</p>
                      </div>
                    </button>
                  );
                })}
              </div>
            </div>

            {/* Content Area */}
            <div className="flex-1 min-h-0 px-0.5 pb-0.5">
              <ScrollArea className="h-full bg-background-default rounded-2xl px-6 py-6">
                <TabsContent
                  value="models"
                  className="mt-0 focus-visible:outline-none focus-visible:ring-0"
                >
                  <ModelsSection setView={setView} />
                </TabsContent>

                <TabsContent
                  value="chat"
                  className="mt-0 focus-visible:outline-none focus-visible:ring-0"
                >
                  <ChatSettingsSection />
                </TabsContent>

                <TabsContent
                  value="sharing"
                  className="mt-0 focus-visible:outline-none focus-visible:ring-0"
                >
                  <SessionSharingSection />
                </TabsContent>

                <TabsContent
                  value="app"
                  className="mt-0 focus-visible:outline-none focus-visible:ring-0"
                >
                  <div className="space-y-8">
                    {CONFIGURATION_ENABLED && <ConfigSettings />}
                    <AppSettingsSection scrollToSection={viewOptions.section} />
                  </div>
                </TabsContent>
              </ScrollArea>
            </div>
          </Tabs>
        </div>
      </MainPanelLayout>
    </>
  );
}
