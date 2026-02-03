import type React from "react";
import {
	BarChart,
	Bar,
	XAxis,
	YAxis,
	CartesianGrid,
	ResponsiveContainer,
	Tooltip,
} from "recharts";
import type { BarChartWidget } from "../types";

interface BarChartProps {
	widget: BarChartWidget;
	data?: any[];
}

const BarChartWidgetComponent: React.FC<BarChartProps> = ({ widget, data }) => {
	const updatedData = data?.map((item) => {
		const yValue = parseFloat(item[widget.properties.yAxis]);
		return {
			...item,
			[widget.properties.xAxis]: item[widget.properties.xAxis],
			[widget.properties.yAxis]: isNaN(yValue) ? 0 : yValue,
		};
	}) || [];

	// return null;
	return (
		<div className="flex flex-col h-full relative">
			<ResponsiveContainer width="100%" height="100%">
				<BarChart
					data={updatedData || []}
					margin={{
						top: 20,
						right: 10,
						left: 0,
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
						domain={[0, "dataMax + 15"]}
						className="text-xs stroke-stone-300"
						stroke="currentColor"
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
					<Bar dataKey={widget.properties.yAxis} fill={widget.properties.color} />
				</BarChart>
			</ResponsiveContainer>
		</div>
	);
};

export default BarChartWidgetComponent;
