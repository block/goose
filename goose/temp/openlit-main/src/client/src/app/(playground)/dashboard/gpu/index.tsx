import GPUMetric from "./gpu-metric";
import NumberStats from "./number-stats";

export default function GPUDashboard() {
	return (
		<>
			<NumberStats />
			<div className="grid-cols-2 grid gap-4">
				<GPUMetric
					chartKeys={["utilization", "enc_utilization", "dec_utilization"]}
					url="/api/metrics/gpu/utilization/time"
					title="Avg Utilization Percentage (%)"
				/>

				<GPUMetric
					chartKeys={["temperature"]}
					url="/api/metrics/gpu/temperature/time"
					title="Avg Temperature (°C)"
				/>
				<GPUMetric
					chartKeys={["memory_total", "memory_used", "memory_free"]}
					url="/api/metrics/gpu/memory/time"
					title="Memory (MB)"
				/>

				<GPUMetric
					chartKeys={["power.limit", "power.draw"]}
					url="/api/metrics/gpu/power/time"
					title="Power (Watt)"
				/>
				<GPUMetric
					chartKeys={["fan_speed"]}
					url="/api/metrics/gpu/fanspeed/time"
					title="Fan speed (0-100)"
				/>
			</div>
		</>
	);
}
