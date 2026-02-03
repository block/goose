import React from "react";
import {
	AreaChart,
	Area,
	XAxis,
	YAxis,
	CartesianGrid,
	Tooltip,
	ResponsiveContainer,
} from "recharts";
import type { AreaChartWidget } from "../types";

interface AreaChartProps {
	widget: AreaChartWidget;
	data?: any[];
}

const AreaChartWidgetComponent: React.FC<AreaChartProps> = ({
	widget,
	data,
}) => {
	return (
		<div className="flex flex-col h-full">
			<ResponsiveContainer width="100%" height="100%">

				<AreaChart
					width={500}
					height={400}
					data={data || []}
					margin={{
						top: 10,
						right: 30,
						left: 0,
						bottom: 0,
					}}
				>
					<CartesianGrid strokeDasharray="3 3" stroke="currentColor" strokeOpacity={0.15} />
					<XAxis
						dataKey={widget.properties.xAxis}
						className="text-xs stroke-stone-300"
						stroke="currentColor"
					/>
					<YAxis
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
					<defs>
						{widget.properties.yAxes?.map((yAxis, index) => (
							<linearGradient
								key={yAxis.key}
								id={yAxis.key}
								x1="0"
								y1="0"
								x2="0"
								y2="1"
							>
								<stop
									offset="5%"
									stopColor={yAxis.color}
									stopOpacity={0.8}
								/>
								<stop
									offset="95%"
									stopColor={yAxis.color}
									stopOpacity={0.1}
								/>
							</linearGradient>
						))}
					</defs>
					{widget.properties.yAxes?.map((yAxis, index) => (
						<Area
							key={yAxis.key}
							type="natural"
							dataKey={yAxis.key}
							stackId={widget.properties.stackId}
							stroke={yAxis.color}
							name={yAxis.key}
							fill={`url(#${yAxis.key})`}
							fillOpacity={0.4}
						/>
					))}
				</AreaChart>
			</ResponsiveContainer>
		</div>
	);
};

export default AreaChartWidgetComponent;
