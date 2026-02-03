"use client";
import OpengroundHeader from "@/components/(playground)/openground/header";
import { OpengroundRecord } from "@/lib/platform/openground-clickhouse";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import { useCallback, useEffect } from "react";
import { toast } from "sonner";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { format } from "date-fns";
import MetricsOverview from "@/components/(playground)/openground/metrics-overview";
import PerformanceWaterfall from "@/components/(playground)/openground/performance-waterfall";
import CostBreakdown from "@/components/(playground)/openground/cost-breakdown";
import ProviderResultCard from "@/components/(playground)/openground/provider-result-card";
import Link from "next/link";
import { Component } from "lucide-react";
import getMessage from "@/constants/messages";

export default function OpengroundRequest({
	params,
}: {
	params: { id: string };
}) {
	const { data, fireRequest, isFetched, isLoading } = useFetchWrapper<OpengroundRecord>();

	const fetchData = useCallback(async () => {
		fireRequest({
			requestType: "GET",
			url: `/api/openground/${params.id}`,
			failureCb: (err?: string) => {
				toast.error(err || getMessage().CANNOT_CONNECT_TO_SERVER, {
					id: "openground",
				});
			},
		});
	}, []);

	useEffect(() => {
		fetchData();
	}, []);

	if (isLoading || !isFetched)
		return (
			<div className="flex w-full h-full text-stone-600 dark:text-stone-400 items-center justify-center">
				{getMessage().LOADING}
			</div>
		);

	if (!data)
		return (
			<div className="flex w-full h-full text-error items-center justify-center">
				{getMessage().NO_DATA_FOUND}
			</div>
		);

	return (
		<div className="flex flex-col w-full h-full gap-6 overflow-auto">
			<OpengroundHeader validateResponse={false} />

			{/* Evaluation Info */}
			<Card>
				<CardHeader>
					<CardTitle className="text-lg">{getMessage().OPENGROUND_RUN_DETAILS}</CardTitle>
				</CardHeader>
				<CardContent className="space-y-4 p-0">
					<div className="flex flex-col">
						<div className="px-6 pb-6 pt-0">
							<p className="text-sm font-medium text-stone-500 dark:text-stone-400 mb-1">
								{getMessage().PROMPT}
							</p>
							<p className="text-sm text-stone-900 dark:text-stone-100">
								{data.prompt}
							</p>
						</div>
						<div className="border-t grid grid-cols-4 border-stone-100 dark:border-stone-800">
							<div className="flex items-center gap-4 pt-2 pb-4 px-6 shadow-sm">
								<p className="text-sm font-medium text-stone-500 dark:text-stone-400">
									{getMessage().PROVIDERS}
								</p>
								<p className="flex gap-1 items-center text-sm font-semibold text-stone-900 dark:text-stone-100">
									<span className="text-green-500">{data.totalProviders - (data.errors?.length || 0)}</span>
									<div className="bg-stone-200 dark:bg-stone-800 h-3 w-0.5 shrink-0" />
									<span className="text-red-500">{(data.errors?.length || 0)}</span>
								</p>
							</div>
							<div className="flex items-center gap-4 shadow-sm pt-2 pb-4 px-6">
								<p className="text-sm font-medium text-stone-500 dark:text-stone-400">
									{getMessage().CREATED_AT}
								</p>
								<p className="text-sm text-stone-900 dark:text-stone-100">
									{format(new Date(data.createdAt), "MMM d, y HH:mm")}
								</p>
							</div>
							{data.promptVariables && Object.keys(data.promptVariables).length > 0 && (
								<div className="flex items-center gap-4 shadow-sm pt-2 pb-4  px-6">
									<p className="text-sm font-medium text-stone-500 dark:text-stone-400">
										{getMessage().VARIABLES}
									</p>
									<span className="text-xs">
										{Object.keys(data.promptVariables).length}
									</span>
								</div>
							)}
							{
								data.promptSource === "prompt-hub" && data.promptHubId ? (
									<div className="flex items-center gap-4 shadow-sm pt-2 pb-4  px-6">
										<p className="text-sm font-medium text-stone-500 dark:text-stone-400">
											{getMessage().PROMPT_HUB}
										</p>
										<Link href={`/prompt-hub/${data.promptHubId}`}>
											<Component className="w-4 h-4" />
										</Link>
									</div>
								) : null
							}
						</div>
					</div>
				</CardContent>
			</Card>

			{/* Provider Results */}
			{data.providers && data.providers.length > 0 && (
				<div className="space-y-6">
					{/* Metrics Overview */}
					<MetricsOverview data={data.providers} />

					{/* Performance & Cost Analysis */}
					<div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
						<PerformanceWaterfall data={data.providers} />
						<CostBreakdown data={data.providers} />
					</div>

					{/* Detailed Results */}
					<Card className="border-stone-200 dark:border-stone-800">
						<CardHeader>
							<CardTitle className="text-lg">{getMessage().OPENGROUND_PROVIDER_RESPONSE}</CardTitle>
						</CardHeader>
						<CardContent className="space-y-4">
							{data.providers.map((result, index) => (
								<ProviderResultCard key={index} result={result} index={index} />
							))}
						</CardContent>
					</Card>
				</div>
			)}
		</div>
	);
}
