import { useCallback, useEffect } from "react";
import {
	LineChart,
	Line,
	XAxis,
	YAxis,
	CartesianGrid,
	Tooltip,
	ResponsiveContainer,
} from "recharts";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import { useRootStore } from "@/store";
import { getFilterDetails } from "@/selectors/filter";
import { getPingStatus } from "@/selectors/database-config";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { COLORS } from "../../../../../styles/colors";
import IntermediateState from "@/components/(playground)/intermediate-state";
import { toast } from "sonner";
import { getFilterParamsForDashboard } from "@/helpers/client/filter";

export default function RequestsPerTime() {
	const filter = useRootStore(getFilterDetails);
	const pingStatus = useRootStore(getPingStatus);
	const { data, fireRequest, isFetched, isLoading } = useFetchWrapper();
	const fetchData = useCallback(async () => {
		fireRequest({
			body: JSON.stringify(getFilterParamsForDashboard(filter)),
			requestType: "POST",
			url: "/api/metrics/request/time",
			responseDataKey: "data",
			failureCb: (err?: string) => {
				toast.error(err || `Cannot connect to server!`, {
					id: "dashboard-page",
				});
			},
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

	const updatedData = (data as any[]) || [];

	return (
		<Card>
			<CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
				<CardTitle className="text-sm font-medium text-stone-950 dark:text-stone-100">
					Requests per time
				</CardTitle>
			</CardHeader>
			<CardContent>
				<ResponsiveContainer className="h-40" width="100%" height="100%">
					{isLoading || !isFetched || pingStatus === "pending" ? (
						<IntermediateState type="loading" classNames="h-40" />
					) : updatedData.length === 0 ? (
						<IntermediateState type="nodata" classNames="h-40" />
					) : (
						<LineChart
							data={updatedData}
							margin={{
								top: 5,
								right: 30,
								left: 20,
								bottom: 5,
							}}
						>
							<CartesianGrid strokeDasharray="3 3" />
							<XAxis
								dataKey="request_time"
								className="text-xs stroke-stone-300"
								stroke="currentColor"
							/>
							<YAxis
								className="text-xs stroke-stone-300"
								stroke="currentColor"
								domain={[0, "dataMax + 15"]}
							/>
							<Tooltip labelClassName="dark:text-stone-700" />
							<Line
								type="monotone"
								dataKey="total"
								stroke={`${COLORS.primary}`}
								activeDot={{ r: 4 }}
							/>
						</LineChart>
					)}
				</ResponsiveContainer>
			</CardContent>
		</Card>
	);
}
