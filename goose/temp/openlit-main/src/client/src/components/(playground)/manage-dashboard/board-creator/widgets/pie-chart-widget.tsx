import type React from "react";
import { PieChart, Pie, ResponsiveContainer, Sector } from "recharts";
import type { PieChartWidget } from "../types";
import { useMemo, useState } from "react";

interface PieChartProps {
	widget: PieChartWidget;
	data?: any[];
}

const renderActiveShape = (props: any) => {
	const {
		cx,
		cy,
		innerRadius,
		outerRadius,
		startAngle,
		endAngle,
		payload,
		percent,
		fill,
	} = props;

	return (
		<g>
			<text
				x={cx}
				y={cy - 10}
				dy={8}
				textAnchor="middle"
				fill={fill}
			>
				{payload.name}
			</text>
			<Sector
				cx={cx}
				cy={cy}
				innerRadius={innerRadius}
				outerRadius={outerRadius}
				startAngle={startAngle}
				endAngle={endAngle}
				fill={fill}
			/>
			<Sector
				cx={cx}
				cy={cy}
				startAngle={startAngle}
				endAngle={endAngle}
				innerRadius={outerRadius + 6}
				outerRadius={outerRadius + 10}
				fill={fill}
			/>
			<text
				x={cx}
				y={cy + 10}
				dy={8}
				textAnchor="middle"
				fill={fill}
			>
				{`(${(percent * 100).toFixed(2)}%)`}
			</text>
		</g>
	);
};

const PieChartWidgetComponent: React.FC<PieChartProps> = ({ widget, data }) => {
	const [activeIndex, setActiveIndex] = useState<number>(0);
	const onPieEnter = (_: any, index: number) => {
		setActiveIndex(index);
	};
	
	const { labelPath, valuePath } = widget.properties;

	const updatedData = useMemo(
		() =>
			(data as { [key: string]: any }[] || []).map((item) => ({
				name: item[labelPath],
				value: item[valuePath],
			})),
		[data]
	);

	return (
		<div className="flex flex-col h-full">
			<ResponsiveContainer width="100%" height="100%">
				<PieChart width={100} height={100}>
					<Pie
						activeIndex={activeIndex}
						activeShape={renderActiveShape}
						data={updatedData}
						cx="50%"
						cy="50%"
						innerRadius={60}
						outerRadius={80}
						className="fill-stone-800 dark:fill-stone-600"
						fill={widget.properties.color}
						dataKey={"value"}
						onMouseEnter={onPieEnter}
					/>
				</PieChart>
			</ResponsiveContainer>
		</div>
	);
};

export default PieChartWidgetComponent;
