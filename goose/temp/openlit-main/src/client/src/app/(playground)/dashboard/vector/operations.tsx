import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { getChartColors } from "@/constants/chart-colors";
import { getFilterParamsForDashboard } from "@/helpers/client/filter";
import { getPingStatus } from "@/selectors/database-config";
import { getFilterDetails } from "@/selectors/filter";
import { useRootStore } from "@/store";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import { useCallback, useEffect } from "react";
import IntermediateState from "@/components/(playground)/intermediate-state";
import {
	Bar,
	BarChart,
	LabelList,
	ResponsiveContainer,
	XAxis,
	YAxis,
} from "recharts";

export default function Operations() {
	const filter = useRootStore(getFilterDetails);
	const pingStatus = useRootStore(getPingStatus);
	const { data, fireRequest, isFetched, isLoading } = useFetchWrapper();

	const fetchData = useCallback(async () => {
		fireRequest({
			body: JSON.stringify(getFilterParamsForDashboard(filter)),
			requestType: "POST",
			url: "/api/metrics/vector/operation",
			responseDataKey: "data",
		});
	}, [filter]);

	useEffect(() => {
		if (
			filter.timeLimit.start &&
			filter.timeLimit.end &&
			pingStatus === "success"
		)
			fetchData();
	}, [filter, fetchData, pingStatus]);

	const colors = getChartColors((data as any[])?.length || 0);

	const updatedData = ((data as any[]) || []).map((item, index) => ({
		name: item.operation,
		value: item.count,
		color: colors[index],
	}));

	return (
		<Card className="col-span-1">
			<CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
				<CardTitle className="text-sm font-medium text-stone-950 dark:text-white">
					Generation by operation
				</CardTitle>
			</CardHeader>
			<CardContent>
				<ResponsiveContainer className="!h-60" width="100%" height="100%">
					{isLoading || !isFetched || pingStatus === "pending" ? (
						<IntermediateState type="loading" classNames="h-40" />
					) : updatedData.length === 0 ? (
						<IntermediateState type="nodata" classNames="h-40" />
					) : (
						<BarChart
							data={updatedData}
							layout="vertical"
							margin={{ right: 140 }}
							width={500}
							height={300}
						>
							<YAxis dataKey="name" type="category" axisLine={false} hide />
							<XAxis dataKey="value" type="number" hide />
							<Bar
								dataKey="value"
								layout="vertical"
								className="fill-primary"
								radius={[0, 20, 20, 0]}
							>
								<LabelList
									dataKey="name"
									position="insideLeft"
									offset={8}
									className="fill-stone-100"
									fontSize={12}
								/>
								<LabelList
									dataKey="value"
									position="right"
									offset={8}
									className="fill-stone-800 dark:fill-stone-100"
									fontSize={12}
								/>
							</Bar>
						</BarChart>
					)}
				</ResponsiveContainer>
			</CardContent>
		</Card>
	);
}
