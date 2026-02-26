import { useMemo, useState } from 'react';
import { useLocation } from 'react-router-dom';
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
  topics: TopicsTab,
  inspector: RoutingInspector,
  'eval-runner': EvalRunner,
};

type EvaluateLocationState = {
  tab?: string;
  runId?: string;
};

function getEvaluateState(state: unknown): EvaluateLocationState {
  if (!state || typeof state !== 'object') {
    return {};
  }

  const maybe = state as Partial<EvaluateLocationState>;
  return {
    tab: typeof maybe.tab === 'string' ? maybe.tab : undefined,
    runId: typeof maybe.runId === 'string' ? maybe.runId : undefined,
  };
}

export default function EvaluateView() {
  const location = useLocation();
  const evalState = useMemo(() => getEvaluateState(location.state), [location.state]);
  const [activeTab, setActiveTab] = useState(evalState.tab || 'overview');
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
        {activeTab === 'runs' ? (
          <RunHistoryTab initialRunId={evalState.runId} />
        ) : (
          ActiveComponent && <ActiveComponent />
        )}
      </div>
    </div>
  );
}
