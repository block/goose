"use client";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
	TrendingDownIcon,
	ZapIcon,
	CoinsIcon,
	CheckCircle2Icon,
} from "lucide-react";
import { ProviderResult } from "@/lib/platform/openground-clickhouse";
import getMessage from "@/constants/messages";

interface MetricsOverviewProps {
	data: ProviderResult[];
}

export default function MetricsOverview({ data }: MetricsOverviewProps) {
	const successfulProviders = data.filter((p) => !p.error);
	const totalProviders = data.length;
	const successRate = Math.round((successfulProviders.length / totalProviders) * 100);

	// Find fastest response
	const fastest = successfulProviders.reduce((min, p) =>
		p.responseTime < min.responseTime ? p : min
		, successfulProviders[0] || { responseTime: 0, provider: "", model: "" });

	// Find lowest cost
	const cheapest = successfulProviders.reduce((min, p) =>
		p.cost < min.cost ? p : min
		, successfulProviders[0] || { cost: 0, provider: "", model: "" });

	// Find most token efficient
	const mostEfficient = successfulProviders.reduce((min, p) =>
		p.completionTokens < min.completionTokens ? p : min
		, successfulProviders[0] || { completionTokens: 0, provider: "", model: "" });

	const stats = [
		{
			title: getMessage().OPENGROUND_FASTEST_RESPONSE,
			value: `${fastest.responseTime.toFixed(2)}s`,
			subtitle: fastest.provider ? `${fastest.provider}/${fastest.model}` : '',
			icon: ZapIcon,
			iconColor: "text-yellow-500",
			bgColor: "bg-yellow-50 dark:bg-yellow-950/20",
		},
		{
			title: getMessage().OPENGROUND_LOWEST_COST,
			value: `$${cheapest.cost.toFixed(4)}`,
			subtitle: cheapest.provider ? `${cheapest.provider}/${cheapest.model}` : '',
			icon: CoinsIcon,
			iconColor: "text-green-500",
			bgColor: "bg-green-50 dark:bg-green-950/20",
		},
		{
			title: getMessage().OPENGROUND_MOST_EFFICIENT,
			value: `${mostEfficient.completionTokens} tokens`,
			subtitle: mostEfficient.provider ? `${mostEfficient.provider}/${mostEfficient.model}` : '',
			icon: TrendingDownIcon,
			iconColor: "text-blue-500",
			bgColor: "bg-blue-50 dark:bg-blue-950/20",
		},
		{
			title: getMessage().OPENGROUND_SUCCESS_RATE,
			value: `${successRate}%`,
			subtitle: `${successfulProviders.length}/${totalProviders} ${getMessage().PROVIDERS}`,
			icon: CheckCircle2Icon,
			iconColor:
				successRate === 100
					? "text-green-500"
					: successRate >= 50
						? "text-yellow-500"
						: "text-red-500",
			bgColor:
				successRate === 100
					? "bg-green-50 dark:bg-green-950/20"
					: successRate >= 50
						? "bg-yellow-50 dark:bg-yellow-950/20"
						: "bg-red-50 dark:bg-red-950/20",
		},
	];

	return (
		<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
			{stats.map((stat) => {
				const Icon = stat.icon;
				return (
					<Card
						key={stat.title}
					>
						<CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
							<CardTitle className="text-sm font-medium text-stone-600 dark:text-stone-400">
								{stat.title}
							</CardTitle>
							<div className={`p-2 rounded-lg ${stat.bgColor}`}>
								<Icon className={`h-4 w-4 ${stat.iconColor}`} />
							</div>
						</CardHeader>
						<CardContent>
							<div className="text-2xl font-bold text-stone-900 dark:text-stone-100">
								{stat.value}
							</div>
							<p className="text-xs text-stone-500 dark:text-stone-400 mt-1 truncate">
								{stat.subtitle}
							</p>
						</CardContent>
					</Card>
				);
			})}
		</div>
	);
}
