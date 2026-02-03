import PieChartCard from "@/components/(playground)/pie-chart-card";
import StatCard from "@/components/(playground)/stat-card";
import { TraceMapping } from "@/constants/traces";
import { integerParser } from "@/helpers/client/trace";
import { RadioTower, Timer } from "lucide-react";

function NumberStats() {
	return (
		<div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
			<div className="grid gap-4 grid-cols-1">
				<StatCard
					dataKey="total_requests"
					extraParams={{
						operationType: "vectordb",
					}}
					heading="Total requests"
					icon={RadioTower}
					url="/api/metrics/request/total"
				/>
				<StatCard
					dataKey="average_duration"
					extraParams={{
						operationType: "vectordb",
					}}
					heading="Avg Request Duration"
					icon={Timer}
					textSuffix="s"
					parser={(value: any) =>
						integerParser(`${value}`, TraceMapping.requestDuration.offset)
					}
					url="/api/metrics/request/duration/average"
				/>
			</div>
			<PieChartCard
				categoryKey="count"
				heading="Generation by system"
				indexKey="system"
				url="/api/metrics/vector/system"
			/>
			<PieChartCard
				categoryKey="count"
				heading="Generation by application"
				indexKey="applicationName"
				url="/api/metrics/vector/application"
			/>
			<PieChartCard
				categoryKey="count"
				heading="Generation by Environment"
				indexKey="environment"
				url="/api/metrics/vector/environment"
			/>
		</div>
	);
}

export default NumberStats;
