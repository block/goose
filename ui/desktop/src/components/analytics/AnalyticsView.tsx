import { useState } from 'react';
import RoutingInspector from './RoutingInspector';
import EvalRunner from './EvalRunner';
import AgentCatalog from './AgentCatalog';

type Tab = 'inspector' | 'eval' | 'catalog';

const TABS: { key: Tab; label: string }[] = [
  { key: 'inspector', label: 'Routing Inspector' },
  { key: 'eval', label: 'Eval Runner' },
  { key: 'catalog', label: 'Agent Catalog' },
];

export default function AnalyticsView() {
  const [activeTab, setActiveTab] = useState<Tab>('inspector');

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="px-6 pt-6 pb-0">
        <h1 className="text-xl font-semibold text-gray-100 mb-4">Routing Analytics</h1>

        {/* Tabs */}
        <div className="flex border-b border-gray-700">
          {TABS.map((tab) => (
            <button
              key={tab.key}
              onClick={() => setActiveTab(tab.key)}
              className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors ${
                activeTab === tab.key
                  ? 'border-blue-500 text-blue-400'
                  : 'border-transparent text-gray-500 hover:text-gray-300'
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>
      </div>

      {/* Tab content */}
      <div className="flex-1 overflow-y-auto px-6 py-4">
        {activeTab === 'inspector' && <RoutingInspector />}
        {activeTab === 'eval' && <EvalRunner />}
        {activeTab === 'catalog' && <AgentCatalog />}
      </div>
    </div>
  );
}
