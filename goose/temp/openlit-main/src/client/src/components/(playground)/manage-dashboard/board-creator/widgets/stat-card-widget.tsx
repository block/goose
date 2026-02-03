import React from "react";
import type { StatCardWidget } from "../types";
import { TrendingDown } from "lucide-react";
import { TrendingUp } from "lucide-react";
import { isNil } from "lodash";

interface StatCardProps {
	widget: StatCardWidget;
	data?: any;
}

const StatCardWidget: React.FC<StatCardProps> = ({ widget, data }) => {
	let value = "";
	let trend: number = 0;

	try {
		value = (widget.properties.value || "")
			.split(".")
			.reduce((acc: any, curr: string) => acc?.[curr], data);
		value = (value || 0).toString();
		trend = (widget.properties.trend || "")
			.split(".")
			.reduce((acc: any, curr: string) => acc?.[curr], data);
		trend = isNil(trend) ? trend : parseFloat(trend?.toString() || "0");
	} catch (error) {
		console.error(error);
	}

	return (
		<div className="flex flex-col justify-center items-center h-full">
			<div className={`text-3xl font-bold`} style={{ color: widget.properties.color }}>
				{widget.properties.prefix}
				{value}
				{widget.properties.suffix}
			</div>
			{!isNil(trend) && (
				<div
					className={`flex items-center gap-1 text-sm mt-2 ${trend > 0
						? "text-green-500"
						: "text-red-500"
						}`}
				>
					{widget.properties.trendPrefix}
					{Math.abs(trend)}
					{widget.properties.trendSuffix}
					{trend > 0 ? <TrendingUp className="w-4 h-4" /> : <TrendingDown className="w-4 h-4" />}
				</div>
			)}
		</div>
	);
};

export default StatCardWidget;
