import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import { memo, useCallback, useEffect } from "react";
import { isNil, round } from "lodash";
import { getFilterDetails } from "@/selectors/filter";
import { useRootStore } from "@/store";
import { getPingStatus } from "@/selectors/database-config";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { LucideIcon, TrendingDown, TrendingUp } from "lucide-react";
import IntermediateState from "./intermediate-state";
import { getFilterParamsForDashboard } from "@/helpers/client/filter";

type StatCardProps = {
	heading?: string;
	isLoading?: boolean;
	loadingClass?: string;
	textClass?: string;
	dataKey: string;
	extraParams?: Record<any, any>;
	roundTo?: number;
	textPrefix?: string;
	textSuffix?: string;
	url: string;
	icon?: LucideIcon;
	parser?: (value: any) => any;
};

const StatCard = memo(
	({
		dataKey,
		extraParams = {},
		heading,
		roundTo = 4,
		textClass = "",
		textPrefix = "",
		textSuffix = "",
		loadingClass = "h-9 w-12",
		url,
		icon: IconComponent,
		parser,
	}: StatCardProps) => {
		const filter = useRootStore(getFilterDetails);
		const pingStatus = useRootStore(getPingStatus);
		const { data, isFetched, isLoading, fireRequest } = useFetchWrapper();

		const fetchData = useCallback(async () => {
			fireRequest({
				body: JSON.stringify(
					getFilterParamsForDashboard({
						...filter,
						...extraParams,
					})
				),
				requestType: "POST",
				url,
				responseDataKey: "data[0]",
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

		const isLoadingData = isLoading || !isFetched || pingStatus === "pending";

		const currentData = (data as Record<any, any>)?.[dataKey] || 0;
		const doesPreviousDataKeyExist = !isNil(
			(data as Record<any, any>)?.[`previous_${dataKey}`]
		);
		const previousData =
			(data as Record<any, any>)?.[`previous_${dataKey}`] || 0;
		const changePercent = doesPreviousDataKeyExist
			? round(((currentData - previousData) / (previousData || 1)) * 100, 2)
			: 0;

		const value = (data as Record<any, any>)?.[dataKey] || 0;

		return (
			<Card className="relative overflow-hidden">
				<CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
					<CardTitle className="text-sm font-medium text-stone-950 dark:text-white">
						{heading}
					</CardTitle>
					{IconComponent && (
						<span className="absolute -right-2 -top-2 p-3 bg-stone-200 dark:bg-stone-800 rounded-bl-3xl">
							<IconComponent className="h-6 w-6 text-stone-500 dark:text-stone-100" />
						</span>
					)}
				</CardHeader>
				<CardContent>
					{isLoadingData ? (
						<IntermediateState type="loading">
							<Skeleton className={`h-4 w-full rounded-xl ${loadingClass}`} />
						</IntermediateState>
					) : (
						<div
							className={`font-semibold text-primary ${
								textClass.match(/text-(xs|sm|base|lg|xl|[2-9]xl)/)
									? ""
									: "text-3xl"
							} ${textClass}`}
						>
							{`${textPrefix}${round(
								typeof parser === "function" ? parser(value) : value,
								roundTo
							)}${textSuffix}`}
						</div>
					)}
					<span
						className={`flex items-center text-xs text-muted-foreground ${
							changePercent > 0 ? "text-success" : "text-error"
						} ${doesPreviousDataKeyExist ? "" : "opacity-0"}`}
					>
						{changePercent > 0 ? (
							<TrendingUp className="w-4 mr-2" />
						) : (
							<TrendingDown className="w-4 mr-2" />
						)}
						{changePercent}%
					</span>
				</CardContent>
			</Card>
		);
	}
);

StatCard.displayName = "StatCard";

export default StatCard;
