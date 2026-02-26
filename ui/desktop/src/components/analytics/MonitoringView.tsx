import { useState } from 'react';
import { PageHeader } from '../molecules/design-system/PageHeader';
import { TabBar } from '../molecules/design-system/TabBar';
import AnalyticsDashboard from './AnalyticsDashboard';
import LiveMonitoringTab from './LiveMonitoringTab';
import ResponseQualityTab from './ResponseQualityTab';
import ToolAnalyticsTab from './ToolAnalyticsTab';

const TAB_GROUPS = [
  {
    tabs: [
      { id: 'dashboard', label: 'Dashboard' },
      { id: 'tools', label: 'Tool Analytics' },
      { id: 'live', label: 'Live' },
      { id: 'quality', label: 'Quality' },
    ],
  },
];

const COMPONENTS: Record<string, React.FC> = {
  dashboard: AnalyticsDashboard,
  tools: ToolAnalyticsTab,
  live: LiveMonitoringTab,
  quality: ResponseQualityTab,
};

export default function MonitoringView() {
  const [activeTab, setActiveTab] = useState('dashboard');
  const ActiveComponent = COMPONENTS[activeTab];

  return (
    <div className="h-full flex flex-col overflow-hidden">
      <div className="flex-shrink-0 px-6 pt-4 pb-0">
        <PageHeader title="Monitoring" />
        <TabBar
          groups={TAB_GROUPS}
          activeTab={activeTab}
          onTabChange={setActiveTab}
          variant="underline"
          className="mt-4"
        />
      </div>
      <div className="flex-1 overflow-y-auto px-6 py-4">
        {ActiveComponent && <ActiveComponent />}
      </div>
    </div>
  );
}
