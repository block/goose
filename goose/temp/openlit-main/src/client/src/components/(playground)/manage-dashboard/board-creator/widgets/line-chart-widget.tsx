import type React from "react";
import {
	LineChart,
	Line,
	XAxis,
	YAxis,
	CartesianGrid,
	ResponsiveContainer,
	Tooltip,
} from "recharts";
import type { LineChartWidget } from "../types";

interface LineChartProps {
	widget: LineChartWidget;
	data?: any[];
}

const LineChartWidgetComponent: React.FC<LineChartProps> = ({
	widget,
	data,
}) => {
	const updatedData = data?.map((item) => ({
		...item,
		[widget.properties.xAxis]: item[widget.properties.xAxis],
		[widget.properties.yAxis]: parseFloat(item[widget.properties.yAxis]),
	})) || [];

	return (
		<div className="flex flex-col h-full relative">
			<ResponsiveContainer width="100%" height="100%">
				<LineChart
					data={updatedData}
					margin={{
						top: 5,
						right: 30,
						left: 20,
						bottom: 5,
					}}
				>
					<CartesianGrid strokeDasharray="3 3" stroke="currentColor" strokeOpacity={0.15} />
					<XAxis
						dataKey={widget.properties.xAxis}
						className="text-xs stroke-stone-300"
						stroke="currentColor"
					/>
					<YAxis
						dataKey={widget.properties.yAxis}
						className="text-xs stroke-stone-300"
						stroke="currentColor"
						domain={[0, "dataMax + 15"]}
					/>
					<Tooltip
						cursor={{ fill: 'transparent' }}
						labelClassName="text-xs text-stone-900 dark:text-stone-300"
						wrapperClassName="bg-stone-200 dark:bg-stone-800 rounded-md"
						contentStyle={{
							backgroundColor: '', border: 'none', boxShadow: 'none',
							fontWeight: 'bold'
						}}
					/>
					<Line
						type="monotone"
						dataKey={widget.properties.yAxis}
						stroke={widget.properties.color}
						strokeWidth={3}
					/>
				</LineChart>
			</ResponsiveContainer>
		</div>
	);
};

export default LineChartWidgetComponent;
