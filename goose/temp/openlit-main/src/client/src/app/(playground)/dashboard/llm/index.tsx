import DataCharts from "./data-charts";
import NumberStats from "./number-stats";
import RequestsPerTime from "./requests-per-time";
import TokenCharts from "./token-charts";

export default function LLMDashboard() {
	return (
		<>
			<NumberStats />
			<div className="flex flex-col gap-4">
				<RequestsPerTime />
				<DataCharts />
				<TokenCharts />
			</div>
		</>
	);
}
