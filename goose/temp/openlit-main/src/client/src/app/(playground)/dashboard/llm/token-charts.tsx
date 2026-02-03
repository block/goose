import StatCard from "@/components/(playground)/stat-card";
import {
	AreaChart,
	Area,
	Bar,
	BarChart,
	Cell,
	XAxis,
	YAxis,
	LabelList,
	CartesianGrid,
	Tooltip,
	ResponsiveContainer,
} from "recharts";
import { useCallback, useEffect, useState } from "react";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import { getFilterDetails } from "@/selectors/filter";
import { useRootStore } from "@/store";
import { getPingStatus } from "@/selectors/database-config";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { ScrollArea, ScrollBar } from "@/components/ui/scroll-area";
import { COLORS } from "../../../../../styles/colors";
import IntermediateState from "@/components/(playground)/intermediate-state";
import { getFilterParamsForDashboard } from "@/helpers/client/filter";

function TopModels() {
	const filter = useRootStore(getFilterDetails);
	const pingStatus = useRootStore(getPingStatus);
	const { data, fireRequest, isFetched, isLoading } = useFetchWrapper();

	const fetchData = useCallback(async () => {
		fireRequest({
			body: JSON.stringify(getFilterParamsForDashboard(filter)),
			requestType: "POST",
			url: "/api/metrics/llm/model/top",
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

	const updatedData = ((data as any[]) || []).map((item, index) => ({
		name: item.model,
		value: item.model_count,
		target: item.total,
	}));

	return (
		<Card className="col-span-1">
			<CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
				<CardTitle className="text-sm font-medium text-stone-950 dark:text-white">
					Top models
				</CardTitle>
			</CardHeader>
			<CardContent>
				<ResponsiveContainer width="100%" height="100%">
					{isLoading || !isFetched || pingStatus === "pending" ? (
						<IntermediateState type="loading" classNames="h-40" />
					) : updatedData.length === 0 ? (
						<IntermediateState type="nodata" classNames="h-40" />
					) : (
						<BarChart
							width={500}
							height={300}
							data={updatedData}
							margin={{
								top: 5,
								right: 10,
								left: 0,
								bottom: 5,
							}}
							layout="vertical"
						>
							<YAxis dataKey="name" type="category" axisLine={false} hide />
							<XAxis dataKey="value" type="number" hide />
							<Bar
								dataKey="value"
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

function ModelsPerTime() {
	const [activeIndex, setActiveIndex] = useState<number>(0);
	const filter = useRootStore(getFilterDetails);
	const pingStatus = useRootStore(getPingStatus);
	const { data, fireRequest, isFetched, isLoading } = useFetchWrapper();
	const handleClick = (_: any, index: number) => {
		setActiveIndex(index);
	};
	const fetchData = useCallback(async () => {
		fireRequest({
			body: JSON.stringify(getFilterParamsForDashboard(filter)),
			requestType: "POST",
			url: "/api/metrics/llm/model/time",
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

	const updatedData = (data as any[]) || [];

	const activeItem = updatedData?.[activeIndex];

	return (
		<Card className="col-span-2">
			<CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
				<CardTitle className="text-sm font-medium text-stone-950 dark:text-white">
					Models per time
				</CardTitle>
			</CardHeader>
			<CardContent>
				<ResponsiveContainer className="h-40" width="100%" height="100%">
					{isLoading || !isFetched || pingStatus === "pending" ? (
						<IntermediateState type="loading" classNames="h-40" />
					) : updatedData.length === 0 ? (
						<IntermediateState type="nodata" classNames="h-40" />
					) : (
						<BarChart
							width={500}
							height={300}
							data={updatedData}
							margin={{
								top: 20,
								right: 10,
								left: 0,
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
								domain={[0, "dataMax + 15"]}
								className="text-xs stroke-stone-300"
								stroke="currentColor"
							/>
							<Bar
								dataKey="total_model_count"
								fill="#8884d8"
								label={{ position: "top" }}
								onClick={handleClick}
							>
								{(data as any[]).map((_, index) => (
									<Cell
										cursor="pointer"
										fill={index === activeIndex ? COLORS.primary : "#8884d8"}
										key={`cell-${index}`}
									/>
								))}
							</Bar>
						</BarChart>
					)}
				</ResponsiveContainer>
				{isLoading || !isFetched || pingStatus === "pending" ? null : (
					<ScrollArea className="w-full whitespace-nowrap">
						<div className="flex w-max space-x-4 p-2">
							{activeItem?.models?.map((model: string, index: number) => (
								<Badge key={`${model}-${index}`}>
									{model} ( {activeItem.model_counts?.[index] || 0} )
								</Badge>
							))}
						</div>
						<ScrollBar orientation="horizontal" />
					</ScrollArea>
				)}
			</CardContent>
		</Card>
	);
}

function TokensPerTime() {
	const filter = useRootStore(getFilterDetails);
	const pingStatus = useRootStore(getPingStatus);
	const { data, fireRequest, isFetched, isLoading } = useFetchWrapper();
	const fetchData = useCallback(async () => {
		fireRequest({
			body: JSON.stringify(getFilterParamsForDashboard(filter)),
			requestType: "POST",
			url: "/api/metrics/llm/token/time",
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

	const updatedDataWithType = ((data || []) as any[]) || [];

	const COLORS = {
		promptokens: "hsl(12 76% 61%)",
		completiontokens: "hsl(243 51.9 68.2%)",
	};

	return (
		<Card className="w-full flex flex-col col-span-3">
			<CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
				<CardTitle className="text-sm font-medium text-stone-950 dark:text-white">
					Tokens usage
				</CardTitle>
			</CardHeader>
			<CardContent className="grow">
				{isLoading || !isFetched || pingStatus === "pending" ? (
					<IntermediateState type="loading" />
				) : updatedDataWithType.length === 0 ? (
					<IntermediateState type="nodata" />
				) : (
					<ResponsiveContainer width="100%" height="100%">
						<AreaChart
							width={500}
							height={400}
							data={updatedDataWithType}
							margin={{
								top: 10,
								right: 30,
								left: 0,
								bottom: 0,
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
							<defs>
								<linearGradient
									id="completionTokens"
									x1="0"
									y1="0"
									x2="0"
									y2="1"
								>
									<stop
										offset="5%"
										stopColor={COLORS.completiontokens}
										stopOpacity={0.8}
									/>
									<stop
										offset="95%"
										stopColor={COLORS.completiontokens}
										stopOpacity={0.1}
									/>
								</linearGradient>
								<linearGradient id="promptTokens" x1="0" y1="0" x2="0" y2="1">
									<stop
										offset="5%"
										stopColor={COLORS.promptokens}
										stopOpacity={0.8}
									/>
									<stop
										offset="95%"
										stopColor={COLORS.promptokens}
										stopOpacity={0.1}
									/>
								</linearGradient>
							</defs>
							<Area
								type="natural"
								dataKey="completiontokens"
								stackId="0"
								stroke={COLORS.completiontokens}
								fill="url(#completionTokens)"
								fillOpacity={0.4}
							/>
							<Area
								type="natural"
								dataKey="prompttokens"
								stackId="0"
								stroke={COLORS.promptokens}
								fill="url(#promptTokens)"
								fillOpacity={0.4}
							/>
						</AreaChart>
					</ResponsiveContainer>
				)}
			</CardContent>
		</Card>
	);
}

function TokenCharts() {
	return (
		<div className="flex flex-col w-full gap-4">
			<div className="grid w-full gap-4 grid-cols-4">
				<div className="flex flex-col gap-4 col-span-1">
					<StatCard
						dataKey="total_tokens"
						extraParams={{ type: "prompt" }}
						heading="Avg prompt tokens / request"
						loadingClass="h-8 w-12"
						textClass="text-2xl"
						url="/api/metrics/llm/token/request/average"
					/>
					<StatCard
						dataKey="total_tokens"
						extraParams={{ type: "completion" }}
						heading="Avg completion tokens / request"
						loadingClass="h-8 w-12"
						textClass="text-2xl"
						url="/api/metrics/llm/token/request/average"
					/>
				</div>
				<TokensPerTime />
			</div>
			<div className="grid gap-4 grid-cols-3">
				<TopModels />
				<ModelsPerTime />
			</div>
		</div>
	);
}

export default TokenCharts;
