import { useState } from "react";
import AnalyticsDashboard from "./AnalyticsDashboard";
import EvalOverviewTab from "./EvalOverviewTab";
import DatasetsTab from "./DatasetsTab";
import RunHistoryTab from "./RunHistoryTab";
import TopicsTab from "./TopicsTab";
import ToolAnalyticsTab from "./ToolAnalyticsTab";
import RoutingInspector from "./RoutingInspector";
import EvalRunner from "./EvalRunner";
import AgentCatalog from "./AgentCatalog";

type ViewGroup = "observe" | "evaluate" | "configure";

interface TabDef {
  id: string;
  label: string;
  group: ViewGroup;
}

const TABS: TabDef[] = [
  // Observe
  { id: "dashboard", label: "Dashboard", group: "observe" },
  { id: "tools", label: "Tool Analytics", group: "observe" },
  // Evaluate
  { id: "overview", label: "Overview", group: "evaluate" },
  { id: "datasets", label: "Datasets", group: "evaluate" },
  { id: "runs", label: "Run History", group: "evaluate" },
  { id: "topics", label: "Topics", group: "evaluate" },
  // Configure
  { id: "inspector", label: "Routing Inspector", group: "configure" },
  { id: "eval-runner", label: "Eval Runner", group: "configure" },
  { id: "catalog", label: "Agent Catalog", group: "configure" },
];

const GROUP_LABELS: Record<ViewGroup, string> = {
  observe: "Observe",
  evaluate: "Evaluate",
  configure: "Configure",
};

const COMPONENTS: Record<string, React.FC> = {
  dashboard: AnalyticsDashboard,
  tools: ToolAnalyticsTab,
  overview: EvalOverviewTab,
  datasets: DatasetsTab,
  runs: RunHistoryTab,
  topics: TopicsTab,
  inspector: RoutingInspector,
  "eval-runner": EvalRunner,
  catalog: AgentCatalog,
};

export default function AnalyticsView() {
  const [activeTab, setActiveTab] = useState("dashboard");
  const ActiveComponent = COMPONENTS[activeTab];

  return (
    <div className="h-full flex flex-col overflow-hidden">
      {/* Header */}
      <div className="flex-shrink-0 px-6 pt-4 pb-0">
        <h1 className="text-xl font-semibold text-textProminent mb-4">Analytics</h1>

        {/* Tab groups */}
        <div className="flex items-center gap-6 border-b border-borderSubtle">
          {(["observe", "evaluate", "configure"] as const).map((group) => (
            <div key={group} className="flex items-center gap-0.5">
              <span className="text-[10px] uppercase tracking-wider text-textSubtle mr-2 select-none">
                {GROUP_LABELS[group]}
              </span>
              {TABS.filter((t) => t.group === group).map((tab) => (
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
          ))}
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto px-6 py-4">
        {ActiveComponent && <ActiveComponent />}
      </div>
    </div>
  );
}
