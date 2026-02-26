import { useState } from 'react';
import { PageHeader } from '@/components/molecules/design-system/page-header';
import { TabBar } from '@/components/molecules/design-system/tab-bar';
import DatasetsTab from './DatasetsTab';
import EvalOverviewTab from './EvalOverviewTab';
import EvalRunner from './EvalRunner';
import RoutingInspector from './RoutingInspector';
import RunHistoryTab from './RunHistoryTab';
import TopicsTab from './TopicsTab';

const TAB_GROUPS = [
  {
    tabs: [
      { id: 'overview', label: 'Overview' },
      { id: 'datasets', label: 'Datasets' },
      { id: 'runs', label: 'Run History' },
      { id: 'topics', label: 'Topics' },
      { id: 'inspector', label: 'Routing Inspector' },
      { id: 'eval-runner', label: 'Eval Runner' },
    ],
  },
];

const COMPONENTS: Record<string, React.FC> = {
  overview: EvalOverviewTab,
  datasets: DatasetsTab,
  runs: RunHistoryTab,
  topics: TopicsTab,
  inspector: RoutingInspector,
  'eval-runner': EvalRunner,
};

export default function EvaluateView() {
  const [activeTab, setActiveTab] = useState('overview');
  const ActiveComponent = COMPONENTS[activeTab];

  return (
    <div className="h-full flex flex-col overflow-hidden">
      <div className="flex-shrink-0 px-6 pt-4 pb-0">
        <PageHeader title="Evaluate" />
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
