import StatCard from "@/components/(playground)/stat-card";
import { MemoryStick, PowerIcon, Thermometer, UtilityPole } from "lucide-react";

function NumberStats() {
	return (
		<div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
			<StatCard
				dataKey="utilization"
				heading="Avg utilization percentage"
				textSuffix="%"
				icon={UtilityPole}
				url="/api/metrics/gpu/utilization/average"
			/>
			<StatCard
				dataKey="temperature"
				heading="Avg temperature"
				icon={Thermometer}
				textSuffix="°C"
				url="/api/metrics/gpu/temperature/average"
			/>
			<StatCard
				dataKey="power_draw"
				heading="Avg power draw"
				icon={PowerIcon}
				textSuffix="W"
				url="/api/metrics/gpu/power/average"
			/>
			<StatCard
				dataKey="memory_used"
				heading="Average memory used"
				icon={MemoryStick}
				textSuffix="MB"
				url="/api/metrics/gpu/memory/average"
			/>
		</div>
	);
}

export default NumberStats;
