import { ScrollArea } from '../ui/scroll-area';
import BackButton from '../ui/BackButton';
import type { View, ViewOptions } from '../../App';
import ExtensionsSection from './extensions/ExtensionsSection';
import ModelsSection from './models/ModelsSection';
import { ModeSection } from './mode/ModeSection';
import { ToolSelectionStrategySection } from './tool_selection_strategy/ToolSelectionStrategySection';
import SessionSharingSection from './sessions/SessionSharingSection';
import { ResponseStylesSection } from './response_styles/ResponseStylesSection';
import AppSettingsSection from './app/AppSettingsSection';
import SchedulerSection from './scheduler/SchedulerSection';
import DictationSection from './dictation/DictationSection';
import { ExtensionConfig } from '../../api';
import MoreMenuLayout from '../more_menu/MoreMenuLayout';

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
  return (
    <div className="h-screen w-full animate-[fadein_200ms_ease-in_forwards]">
      <MoreMenuLayout showMenu={false} />

      <ScrollArea className="h-full w-full">
        <div className="flex flex-col pb-24">
          <div className="px-8 pt-6 pb-4">
            <BackButton onClick={() => onClose()} />
            <h1 className="text-3xl font-medium text-textStandard mt-1">Settings</h1>
          </div>

          {/* Content Area */}
          <div className="flex-1 pt-[20px]">
            <div className="space-y-8">
              {/* Models Section */}
              <ModelsSection setView={setView} />
              {/* Extensions Section */}
              <ExtensionsSection
                deepLinkConfig={viewOptions.deepLinkConfig}
                showEnvVars={viewOptions.showEnvVars}
              />
              {/* Scheduler Section */}
              <SchedulerSection />
              {/* Goose Modes */}
              <ModeSection setView={setView} />
              {/*Session sharing*/}
              <SessionSharingSection />
              {/* Response Styles */}
              <ResponseStylesSection />
              {/* Voice Dictation */}
              <DictationSection />
              {/* Tool Selection Strategy */}
              <ToolSelectionStrategySection setView={setView} />
              {/* App Settings */}
              <AppSettingsSection scrollToSection={viewOptions.section} />
            </div>
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}
