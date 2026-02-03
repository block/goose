"use client";

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { ProviderResult } from "@/lib/platform/openground-clickhouse";
import { DollarSignIcon } from "lucide-react";
import getMessage from "@/constants/messages";

interface CostBreakdownProps {
	data: ProviderResult[];
}

export default function CostBreakdown({ data }: CostBreakdownProps) {
	const successfulData = data.filter((p) => !p.error);
	const totalCost = successfulData.reduce((sum, p) => sum + p.cost, 0);
	const cheapest = successfulData.reduce((min, p) =>
		p.cost < min.cost ? p : min
		, successfulData[0] || { cost: 0 });

	return (
		<Card>
			<CardHeader>
				<div className="flex items-center justify-between">
					<div>
						<CardTitle className="text-lg flex items-center gap-2">
							<DollarSignIcon className="h-5 w-5" />
							{getMessage().OPENGROUND_COST_BREAKDOWN}
						</CardTitle>
						<CardDescription>
							{getMessage().OPENGROUND_COST_BREAKDOWN_DESCRIPTION}
						</CardDescription>
					</div>
				</div>
			</CardHeader>
			<CardContent className="space-y-3">
				{/* Provider Cost Cards */}
				<div className="space-y-2">
					{successfulData.map((result, index) => {
						const isCheapest = result.cost === cheapest.cost;
						return (
							<div
								key={index}
								className={`border p-4 ${isCheapest
									? "border-green-300 dark:border-green-700 bg-green-50 dark:bg-green-950/20"
									: "border-stone-200 dark:border-stone-700"
									}`}
							>
								<div className="flex items-start justify-between gap-4">
									{/* Provider Info */}
									<div className="flex-1 min-w-0">
										<div className="flex items-center gap-2 mb-2">
											<h4 className="font-semibold text-sm text-stone-900 dark:text-stone-100 truncate">
												{result.provider}
											</h4>
											{isCheapest && (
												<Badge
													variant="outline"
													className="text-xs bg-green-100 text-green-700 dark:bg-green-950/40 dark:text-green-300 border-green-200 dark:border-green-800"
												>
													{getMessage().CHEAPEST}
												</Badge>
											)}
										</div>
										<p className="text-xs text-stone-500 dark:text-stone-400 truncate">
											{result.model}
										</p>
									</div>

									{/* Cost */}
									<div className="text-right">
										<p className="text-2xl font-bold text-stone-900 dark:text-stone-100 font-mono">
											${result.cost.toFixed(6)}
										</p>
										<p className="text-xs text-stone-500 dark:text-stone-400">
											{result.responseTime.toFixed(2)}s
										</p>
									</div>
								</div>

								{/* Token Details */}
								<div className="grid grid-cols-3 gap-3 mt-3 pt-3 border-t border-stone-200 dark:border-stone-700">
									<div>
										<p className="text-xs text-stone-500 dark:text-stone-400 mb-0.5">
											{getMessage().PROMPT}
										</p>
										<p className="text-sm font-semibold text-stone-900 dark:text-stone-100 font-mono">
											{result.promptTokens.toLocaleString()}
										</p>
									</div>
									<div>
										<p className="text-xs text-stone-500 dark:text-stone-400 mb-0.5">
											{getMessage().COMPLETION}
										</p>
										<p className="text-sm font-semibold text-stone-900 dark:text-stone-100 font-mono">
											{result.completionTokens.toLocaleString()}
										</p>
									</div>
									<div>
										<p className="text-xs text-stone-500 dark:text-stone-400 mb-0.5">
											{getMessage().TOTAL}
										</p>
										<p className="text-sm font-semibold text-stone-900 dark:text-stone-100 font-mono">
											{result.totalTokens.toLocaleString()}
										</p>
									</div>
								</div>
							</div>
						);
					})}
				</div>

				{/* Total Cost Card */}
				<div className="border-t p-4 border-b">
					<div className="flex items-center justify-between">
						<span className="text-sm font-semibold text-stone-700 dark:text-stone-300">
							{getMessage().TOTAL} {getMessage().COST}
						</span>
						<span className="text-2xl font-bold text-stone-900 dark:text-stone-100 font-mono">
							${totalCost.toFixed(6)}
						</span>
					</div>
				</div>

				{/* Summary Stats */}
				<div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
					<div className="flex flex-col p-3">
						<p className="text-xs text-stone-500 dark:text-stone-400 mb-1">
							{getMessage().AVERAGE} {getMessage().COST}
						</p>
						<p className="text-lg font-bold text-stone-900 dark:text-stone-100 font-mono">
							${(totalCost / successfulData.length || 0).toFixed(6)}
						</p>
					</div>
					<div className="flex flex-col p-3">
						<p className="text-xs text-stone-500 dark:text-stone-400 mb-1">
							{getMessage().TOTAL} {getMessage().TOKENS}
						</p>
						<p className="text-lg font-bold text-stone-900 dark:text-stone-100 font-mono">
							{successfulData
								.reduce((sum, p) => sum + p.totalTokens, 0)
								.toLocaleString()}
						</p>
					</div>
					<div className="flex flex-col p-3">
						<p className="text-xs text-stone-500 dark:text-stone-400 mb-1">
							{getMessage().AVERAGE} {getMessage().COST} / 1K {getMessage().TOKENS}
						</p>
						<p className="text-lg font-bold text-stone-900 dark:text-stone-100 font-mono">
							${(
								totalCost /
								successfulData.reduce((sum, p) => sum + p.totalTokens, 0) *
								1000 || 0
							).toFixed(4)}
						</p>
					</div>
				</div>
			</CardContent>
		</Card>
	);
}
