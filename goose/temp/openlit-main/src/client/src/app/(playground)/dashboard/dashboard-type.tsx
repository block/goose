import { useRootStore } from "@/store";
import { getDashboardType, setPageData } from "@/selectors/page";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { DASHBOARD_TYPE_OBJECT } from "@/types/store/page";
import LLMDashboard from "./llm";
import GPUDashboard from "./gpu";
import VectorDashboard from "./vector";
import { objectKeys } from "@/utils/object";

const DashboardLabels: any = {
	llm: "LLM",
	vector: "VECTOR DB",
	gpu: "GPU",
};

export function DashboardTypeFilter() {
	const dashboardType = useRootStore(getDashboardType);
	const updateDashboardType = useRootStore(setPageData);
	const changeDashboardType = (value: string) => {
		updateDashboardType("dashboard", "type", value);
	};

	return (
		<Tabs defaultValue={dashboardType} onValueChange={changeDashboardType}>
			<TabsList className="p-0 h-[30px]">
				{objectKeys(DASHBOARD_TYPE_OBJECT).map((key) => (
					<TabsTrigger key={key} value={key} className="py-1.5 text-xs">
						{DashboardLabels[key as any]}
					</TabsTrigger>
				))}
			</TabsList>
		</Tabs>
	);
}

export function DashboardTypeGraphContainer() {
	const dashboardType = useRootStore(getDashboardType);

	return (
		<div className="flex flex-col grow w-full h-full rounded overflow-y-auto gap-4">
			{dashboardType === DASHBOARD_TYPE_OBJECT.llm ? (
				<LLMDashboard />
			) : dashboardType === DASHBOARD_TYPE_OBJECT.vector ? (
				<VectorDashboard />
			) : dashboardType === DASHBOARD_TYPE_OBJECT.gpu ? (
				<GPUDashboard />
			) : null}
		</div>
	);
}
