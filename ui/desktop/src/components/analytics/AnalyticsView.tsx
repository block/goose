import { useState } from "react";
import AnalyticsDashboard from "./AnalyticsDashboard";
import DatasetsTab from "./DatasetsTab";
import RunHistoryTab from "./RunHistoryTab";
import TopicsTab from "./TopicsTab";
import RoutingInspector from "./RoutingInspector";
import EvalRunner from "./EvalRunner";
import AgentCatalog from "./AgentCatalog";
import EvalOverviewTab from "./EvalOverviewTab";

type ViewId = "dashboard" | "evaluate" | "configure";
type SubViewId =
  | "overview"
  | "datasets"
  | "runs"
  | "topics"
  | "inspector"
  | "eval-runner"
  | "catalog"
  | "usage";

interface SubView {
  id: SubViewId;
  label: string;
}

const EVALUATE_VIEWS: SubView[] = [
  { id: "overview", label: "Overview" },
  { id: "datasets", label: "Datasets" },
  { id: "runs", label: "Run History" },
  { id: "topics", label: "Topics" },
];

const CONFIGURE_VIEWS: SubView[] = [
  { id: "inspector", label: "Routing Inspector" },
  { id: "eval-runner", label: "Eval Runner" },
  { id: "catalog", label: "Agent Catalog" },
];

export default function AnalyticsView() {
  const [activeView, setActiveView] = useState<ViewId>("dashboard");
  const [evalSubView, setEvalSubView] = useState<SubViewId>("overview");
  const [configSubView, setConfigSubView] = useState<SubViewId>("inspector");

  const viewLabels: Record<ViewId, { label: string; icon: string }> = {
    dashboard: { label: "Dashboard", icon: "📊" },
    evaluate: { label: "Evaluate", icon: "🧪" },
    configure: { label: "Configure", icon: "⚙️" },
  };

  return (
    <div className="flex flex-col h-full overflow-hidden bg-background">
      {/* Top Navigation Bar */}
      <div className="px-6 pt-4 pb-0 border-b border-borderSubtle">
        {/* Title + primary nav */}
        <div className="flex items-center justify-between mb-3">
          <h1 className="text-lg font-semibold text-textStandard">
            Analytics
          </h1>
        </div>

        {/* Primary view tabs */}
        <div className="flex items-center gap-0.5">
          {(Object.entries(viewLabels) as [ViewId, { label: string; icon: string }][]).map(
            ([id, { label, icon }]) => (
              <button
                key={id}
                onClick={() => setActiveView(id)}
                className={`px-4 py-2.5 text-sm font-medium transition-all relative rounded-t-md ${
                  activeView === id
                    ? "text-textStandard bg-surfaceHover"
                    : "text-textSubtle hover:text-textStandard hover:bg-surfaceHover/50"
                }`}
              >
                <span className="mr-1.5">{icon}</span>
                {label}
                {activeView === id && (
                  <div className="absolute bottom-0 left-2 right-2 h-0.5 bg-accent rounded-t" />
                )}
              </button>
            )
          )}
        </div>
      </div>

      {/* Sub-navigation for Evaluate and Configure */}
      {activeView === "evaluate" && (
        <div className="px-6 py-1.5 bg-surfaceHover/30 border-b border-borderSubtle">
          <div className="flex items-center gap-1">
            {EVALUATE_VIEWS.map((sv) => (
              <button
                key={sv.id}
                onClick={() => setEvalSubView(sv.id)}
                className={`px-3 py-1.5 text-xs font-medium rounded-md transition-all ${
                  evalSubView === sv.id
                    ? "text-textStandard bg-surfaceHover"
                    : "text-textSubtle hover:text-textStandard"
                }`}
              >
                {sv.label}
              </button>
            ))}
          </div>
        </div>
      )}

      {activeView === "configure" && (
        <div className="px-6 py-1.5 bg-surfaceHover/30 border-b border-borderSubtle">
          <div className="flex items-center gap-1">
            {CONFIGURE_VIEWS.map((sv) => (
              <button
                key={sv.id}
                onClick={() => setConfigSubView(sv.id)}
                className={`px-3 py-1.5 text-xs font-medium rounded-md transition-all ${
                  configSubView === sv.id
                    ? "text-textStandard bg-surfaceHover"
                    : "text-textSubtle hover:text-textStandard"
                }`}
              >
                {sv.label}
              </button>
            ))}
          </div>
        </div>
      )}

      {/* Content area */}
      <div className="flex-1 overflow-y-auto">
        {activeView === "dashboard" && <AnalyticsDashboard />}

        {activeView === "evaluate" && (
          <>
            {evalSubView === "overview" && <EvalOverviewTab />}
            {evalSubView === "datasets" && <DatasetsTab />}
            {evalSubView === "runs" && <RunHistoryTab />}
            {evalSubView === "topics" && <TopicsTab />}
          </>
        )}

        {activeView === "configure" && (
          <>
            {configSubView === "inspector" && <RoutingInspector />}
            {configSubView === "eval-runner" && <EvalRunner />}
            {configSubView === "catalog" && <AgentCatalog />}
          </>
        )}
      </div>
    </div>
  );
}
