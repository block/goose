import { Bot, FileText, Keyboard, MessageSquare, Monitor, Share2, Shield } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import type { ExtensionConfig } from '../../api';
import { CONFIGURATION_ENABLED } from '../../updates';
import { trackSettingsTabViewed } from '../../utils/analytics';
import type { View, ViewOptions } from '../../utils/navigationUtils';
import { PageShell } from '../Layout/PageShell';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../ui/atoms/tabs';
import AppSettingsSection from './app/AppSettingsSection';
import ExternalBackendSection from './app/ExternalBackendSection';
import AuthSection from './auth/AuthSection';
import ChatSettingsSection from './chat/ChatSettingsSection';
import ConfigSettings from './config/ConfigSettings';
import KeyboardShortcutsSection from './keyboard/KeyboardShortcutsSection';
import ModelsSection from './models/ModelsSection';
import PromptsSettingsSection from './PromptsSettingsSection';
import SessionSharingSection from './sessions/SessionSharingSection';

export type SettingsViewOptions = {
  deepLinkConfig?: ExtensionConfig;
  showEnvVars?: boolean;
  section?: string;
};

const SECTION_TO_TAB: Record<string, string> = {
  update: 'app',
  models: 'models',
  modes: 'chat',
  sharing: 'sharing',
  styles: 'chat',
  tools: 'chat',
  auth: 'auth',
  app: 'app',
  chat: 'chat',
  prompts: 'prompts',
  keyboard: 'keyboard',
};

const TAB_CONTENT_CLASS = 'mt-0 focus-visible:outline-none focus-visible:ring-0';

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
  const hasTrackedInitialTab = useRef(false);

  const handleTabChange = (tab: string) => {
    setActiveTab(tab);
    trackSettingsTabViewed(tab);
  };

  useEffect(() => {
    if (viewOptions.section) {
      const targetTab = SECTION_TO_TAB[viewOptions.section];
      if (targetTab) {
        setActiveTab(targetTab);
      }
    }
  }, [viewOptions.section]);

  useEffect(() => {
    if (!hasTrackedInitialTab.current) {
      trackSettingsTabViewed(activeTab);
      hasTrackedInitialTab.current = true;
    }
  }, [activeTab]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onClose();
      }
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [onClose]);

  const tabBar = (
    <TabsList className="w-full justify-start mb-2">
      <TabsTrigger value="models" className="flex gap-2" data-testid="settings-models-tab">
        <Bot className="h-4 w-4" />
        Models
      </TabsTrigger>
      <TabsTrigger value="chat" className="flex gap-2" data-testid="settings-chat-tab">
        <MessageSquare className="h-4 w-4" />
        Chat
      </TabsTrigger>
      <TabsTrigger value="sharing" className="flex gap-2" data-testid="settings-sharing-tab">
        <Share2 className="h-4 w-4" />
        Session
      </TabsTrigger>
      <TabsTrigger value="prompts" className="flex gap-2" data-testid="settings-prompts-tab">
        <FileText className="h-4 w-4" />
        Prompts
      </TabsTrigger>
      <TabsTrigger value="keyboard" className="flex gap-2" data-testid="settings-keyboard-tab">
        <Keyboard className="h-4 w-4" />
        Keyboard
      </TabsTrigger>
      <TabsTrigger value="auth" className="flex gap-2" data-testid="settings-auth-tab">
        <Shield className="h-4 w-4" />
        Auth
      </TabsTrigger>
      <TabsTrigger value="app" className="flex gap-2" data-testid="settings-app-tab">
        <Monitor className="h-4 w-4" />
        App
      </TabsTrigger>
    </TabsList>
  );

  return (
    <Tabs value={activeTab} onValueChange={handleTabChange} className="h-full flex flex-col">
      <PageShell
        title="Settings"
        subtitle="Configure your Goose experience"
        stickyHeader
        headerExtra={tabBar}
      >
        <TabsContent value="models" className={TAB_CONTENT_CLASS}>
          <ModelsSection setView={setView} />
        </TabsContent>

        <TabsContent value="chat" className={TAB_CONTENT_CLASS}>
          <ChatSettingsSection />
        </TabsContent>

        <TabsContent value="sharing" className={TAB_CONTENT_CLASS}>
          <div className="space-y-6">
            <SessionSharingSection />
            <ExternalBackendSection />
          </div>
        </TabsContent>

        <TabsContent value="prompts" className={TAB_CONTENT_CLASS}>
          <PromptsSettingsSection />
        </TabsContent>

        <TabsContent value="keyboard" className={TAB_CONTENT_CLASS}>
          <KeyboardShortcutsSection />
        </TabsContent>

        <TabsContent value="auth" className={TAB_CONTENT_CLASS}>
          <AuthSection />
        </TabsContent>

        <TabsContent value="app" className={TAB_CONTENT_CLASS}>
          <div className="space-y-6">
            {CONFIGURATION_ENABLED && <ConfigSettings />}
            <AppSettingsSection scrollToSection={viewOptions.section} />
          </div>
        </TabsContent>
      </PageShell>
    </Tabs>
  );
}
