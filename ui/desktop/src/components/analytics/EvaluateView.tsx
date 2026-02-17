import { useState } from "react";
import EvalOverviewTab from "./EvalOverviewTab";
import DatasetsTab from "./DatasetsTab";
import RunHistoryTab from "./RunHistoryTab";
import TopicsTab from "./TopicsTab";
import RoutingInspector from "./RoutingInspector";
import EvalRunner from "./EvalRunner";

interface TabDef {
  id: string;
  label: string;
}

const TABS: TabDef[] = [
  { id: "overview", label: "Overview" },
  { id: "datasets", label: "Datasets" },
  { id: "runs", label: "Run History" },
  { id: "topics", label: "Topics" },
  { id: "inspector", label: "Routing Inspector" },
  { id: "eval-runner", label: "Eval Runner" },
];

const COMPONENTS: Record<string, React.FC> = {
  overview: EvalOverviewTab,
  datasets: DatasetsTab,
  runs: RunHistoryTab,
  topics: TopicsTab,
  inspector: RoutingInspector,
  "eval-runner": EvalRunner,
};

export default function EvaluateView() {
  const [activeTab, setActiveTab] = useState("overview");
  const ActiveComponent = COMPONENTS[activeTab];

  return (
    <div className="h-full flex flex-col overflow-hidden">
      <div className="flex-shrink-0 px-6 pt-4 pb-0">
        <h1 className="text-xl font-semibold text-textProminent mb-4">Evaluate</h1>
        <div className="flex items-center gap-0.5 border-b border-borderSubtle">
          {TABS.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`px-3 py-2 text-sm font-medium transition-colors relative ${
                activeTab === tab.id
                  ? "text-textProminent"
                  : "text-textSubtle hover:text-textStandard"
              }`}
            >
              {tab.label}
              {activeTab === tab.id && (
                <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-indigo-500 rounded-full" />
              )}
            </button>
          ))}
        </div>
      </div>
      <div className="flex-1 overflow-y-auto px-6 py-4">
        {ActiveComponent && <ActiveComponent />}
      </div>
    </div>
  );
}
