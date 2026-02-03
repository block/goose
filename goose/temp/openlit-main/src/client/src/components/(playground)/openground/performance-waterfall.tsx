"use client";

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { ProviderResult } from "@/lib/platform/openground-clickhouse";
import { TimerIcon } from "lucide-react";
import getMessage from "@/constants/messages";

interface PerformanceWaterfallProps {
	data: ProviderResult[];
}

export default function PerformanceWaterfall({ data }: PerformanceWaterfallProps) {
	const maxTime = Math.max(...data.map((p) => p.responseTime));

	// Sort by response time (fastest first)
	const sortedData = [...data].sort((a, b) => a.responseTime - b.responseTime);

	const getBarColor = (index: number, hasError: boolean) => {
		if (hasError) return "bg-red-500/80";
		if (index === 0) return "bg-green-500/80"; // Fastest
		if (index === 1) return "bg-blue-500/80"; // Second fastest
		return "bg-yellow-500/80"; // Others
	};

	const getTextColor = (index: number, hasError: boolean) => {
		if (hasError) return "text-red-700 dark:text-red-300";
		if (index === 0) return "text-green-700 dark:text-green-300";
		if (index === 1) return "text-blue-700 dark:text-blue-300";
		return "text-yellow-700 dark:text-yellow-300";
	};

	return (
		<Card>
			<CardHeader>
				<CardTitle className="text-lg flex items-center gap-2">
					<TimerIcon className="h-5 w-5" />
					{getMessage().OPENGROUND_RESPONSE_TIME_COMPARISON}
				</CardTitle>
				<CardDescription>
					{getMessage().OPENGROUND_RESPONSE_TIME_COMPARISON_DESCRIPTION}
				</CardDescription>
			</CardHeader>
			<CardContent>
				<div className="space-y-3">
					{sortedData.map((result, index) => {
						const hasError = !!result.error;
						const percentage = hasError ? 0 : (result.responseTime / maxTime) * 100;

						return (
							<div key={index} className="space-y-1">
								<div className="flex items-center justify-between text-sm">
									<div className="flex items-center gap-2">
										<span className="font-medium text-stone-700 dark:text-stone-300">
											{result.provider}/{result.model}
										</span>
										{index === 0 && !hasError && (
											<Badge variant="outline" className="text-xs bg-green-50 text-green-700 dark:bg-green-950/20 dark:text-green-300 border-green-200 dark:border-green-800">
												{getMessage().FASTEST}
											</Badge>
										)}
									</div>
									<span className={`text-sm font-mono ${getTextColor(index, hasError)}`}>
										{hasError ? getMessage().ERROR : `${result.responseTime.toFixed(2)}s`}
									</span>
								</div>
								<div className="relative h-8 bg-stone-100 dark:bg-stone-900 rounded-md overflow-hidden">
									<div
										className={`h-full ${getBarColor(index, hasError)} transition-all duration-500 flex items-center px-3`}
										style={{ width: `${Math.max(percentage, 2)}%` }}
									>
										{!hasError && percentage > 20 && (
											<span className="text-xs font-medium text-white">
												{result.responseTime.toFixed(2)}s
											</span>
										)}
									</div>
									{hasError && (
										<div className="absolute inset-0 flex items-center px-3">
											<span className="text-xs text-red-600 dark:text-red-400">
												{result.error}
											</span>
										</div>
									)}
								</div>
							</div>
						);
					})}
				</div>
				<div className="mt-4 pt-4 border-t border-stone-200 dark:border-stone-800">
					<div className="grid grid-cols-3 gap-4 text-center">
						<div>
							<p className="text-2xl font-bold text-green-600 dark:text-green-400">
								{sortedData.find((p) => !p.error)?.responseTime.toFixed(2) || "—"}s
							</p>
							<p className="text-xs text-stone-500 dark:text-stone-400">{getMessage().FASTEST}</p>
						</div>
						<div>
							<p className="text-2xl font-bold text-stone-700 dark:text-stone-300">
								{(data.filter((p) => !p.error).reduce((sum, p) => sum + p.responseTime, 0) /
									data.filter((p) => !p.error).length || 0).toFixed(2)}s
							</p>
							<p className="text-xs text-stone-500 dark:text-stone-400">{getMessage().AVERAGE}</p>
						</div>
						<div>
							<p className="text-2xl font-bold text-red-600 dark:text-red-400">
								{sortedData[sortedData.length - 1]?.error ? "—" : sortedData[sortedData.length - 1]?.responseTime.toFixed(2)}s
							</p>
							<p className="text-xs text-stone-500 dark:text-stone-400">{getMessage().SLOWEST}</p>
						</div>
					</div>
				</div>
			</CardContent>
		</Card>
	);
}
