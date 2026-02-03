import { memo, useCallback, useEffect, useMemo, useState } from "react";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import { useRootStore } from "@/store";
import { getFilterDetails } from "@/selectors/filter";
import { getPingStatus } from "@/selectors/database-config";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { PieChart, Pie, Sector, ResponsiveContainer } from "recharts";
import { COLORS } from "../../../styles/colors";
import IntermediateState from "./intermediate-state";
import { Skeleton } from "../ui/skeleton";
import { getFilterParamsForDashboard } from "@/helpers/client/filter";

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
	} = props;

	return (
		<g>
			<text
				x={cx}
				y={cy - 10}
				dy={8}
				textAnchor="middle"
				fill={`${COLORS.primary}`}
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
				fill={`${COLORS.primary}`}
			/>
			<Sector
				cx={cx}
				cy={cy}
				startAngle={startAngle}
				endAngle={endAngle}
				innerRadius={outerRadius + 6}
				outerRadius={outerRadius + 10}
				fill={`${COLORS.primary}`}
			/>
			<text
				x={cx}
				y={cy + 10}
				dy={8}
				textAnchor="middle"
				fill={`${COLORS.primary}`}
			>
				{`(${(percent * 100).toFixed(2)}%)`}
			</text>
		</g>
	);
};

const PieChartRenderer = ({
	data,
	indexKey,
	categoryKey,
}: {
	data: any;
	indexKey: string;
	categoryKey: string;
}) => {
	const [activeIndex, setActiveIndex] = useState<number>(0);
	const onPieEnter = (_: any, index: number) => {
		setActiveIndex(index);
	};

	const updatedData = useMemo(
		() =>
			(data as any[]).map((item) => ({
				name: item[indexKey],
				value: item[categoryKey],
			})),
		[data]
	);
	return (
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
					fill={"currentColor"}
					dataKey={"value"}
					onMouseEnter={onPieEnter}
				/>
			</PieChart>
		</ResponsiveContainer>
	);
};

type PieChartCardProps = {
	categoryKey: string;
	extraParams?: Record<any, any>;
	heading: string;
	indexKey: string;
	url: string;
};

const PieChartCard = memo(
	({ categoryKey, extraParams, heading, indexKey, url }: PieChartCardProps) => {
		const filter = useRootStore(getFilterDetails);
		const pingStatus = useRootStore(getPingStatus);
		const { data, fireRequest, isFetched, isLoading } = useFetchWrapper();

		const fetchData = useCallback(async () => {
			fireRequest({
				body: JSON.stringify({
					...getFilterParamsForDashboard(filter),
					...extraParams,
				}),
				requestType: "POST",
				url,
				responseDataKey: "data",
			});
		}, [filter, url]);

		useEffect(() => {
			if (
				filter.timeLimit.start &&
				filter.timeLimit.end &&
				pingStatus === "success"
			)
				fetchData();
		}, [filter, fetchData, pingStatus]);

		const updatedData = data as any[];

		return (
			<Card>
				<CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
					<CardTitle className="text-sm font-medium text-stone-950 dark:text-stone-100">
						{heading}
					</CardTitle>
				</CardHeader>
				<CardContent className="h-60">
					{isLoading || !isFetched || pingStatus === "pending" ? (
						<IntermediateState type="loading">
							<div className="flex w-full h-full items-center justify-center">
								<Skeleton className={`h-9 w-1/3 rounded`} />
							</div>
						</IntermediateState>
					) : updatedData?.length > 0 ? (
						<PieChartRenderer
							data={updatedData}
							categoryKey={categoryKey}
							indexKey={indexKey}
						/>
					) : (
						<IntermediateState type="nodata" />
					)}
				</CardContent>
			</Card>
		);
	}
);

PieChartCard.displayName = "PieChartCard";

export default PieChartCard;
