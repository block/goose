"use client";

import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ProviderResult } from "@/lib/platform/openground-clickhouse";
import { ChevronDownIcon, ChevronUpIcon } from "lucide-react";
import { useState } from "react";
import { JsonViewer } from "@textea/json-viewer";
import getMessage from "@/constants/messages";

interface ProviderResultCardProps {
	result: ProviderResult;
	index: number;
}

export default function ProviderResultCard({ result, index }: ProviderResultCardProps) {
	const [showJson, setShowJson] = useState(false);

	return (
		<Card
			className={`border-2 ${
				result.error
					? "border-red-200 dark:border-red-800"
					: "border-stone-200 dark:border-stone-800"
			}`}
		>
			{/* Header */}
			<div className="p-4 border-b border-stone-200 dark:border-stone-800 bg-stone-50 dark:bg-stone-900">
				<div className="flex items-center justify-between">
					<div className="flex items-center gap-3">
						<div className="w-10 h-10 rounded-lg bg-stone-200 dark:bg-stone-700 flex items-center justify-center text-lg font-bold">
							{result.provider.charAt(0).toUpperCase()}
						</div>
						<div>
							<p className="font-semibold text-stone-900 dark:text-stone-100">
								{result.provider}/{result.model}
							</p>
							<p className="text-xs text-stone-500 dark:text-stone-400">
								{result.responseTime.toFixed(2)}s •{" "}
								{result.totalTokens.toLocaleString()} tokens •
								${result.cost.toFixed(6)}
							</p>
						</div>
					</div>
					{result.error ? (
						<Badge variant="destructive">{getMessage().ERROR}</Badge>
					) : (
						<Badge
							variant="outline"
							className="bg-green-50 text-green-700 dark:bg-green-950/20 dark:text-green-300 border-green-200 dark:border-green-800"
						>
							{getMessage().SUCCESS}
						</Badge>
					)}
				</div>
			</div>

			{/* Response/Error Content */}
			<div className="p-4">
				{result.error ? (
					<div className="text-red-600 dark:text-red-400 text-sm font-mono whitespace-pre-wrap">
						{result.error}
					</div>
				) : (
					<div className="prose dark:prose-invert max-w-none">
						<p className="text-stone-700 dark:text-stone-300 whitespace-pre-wrap">
							{result.response}
						</p>
					</div>
				)}
			</div>

			{/* Collapsible JSON Section */}
			<div className="border-t border-stone-200 dark:border-stone-800">
				<Button
					variant="ghost"
					onClick={() => setShowJson(!showJson)}
					className="w-full justify-between rounded-none hover:bg-stone-50 dark:hover:bg-stone-900"
				>
					<span className="text-xs font-medium text-stone-600 dark:text-stone-400">
						{showJson ? getMessage().HIDE : getMessage().SHOW} {getMessage().OPENGROUND_RAW_RESPONSE_DATA}
					</span>
					{showJson ? (
						<ChevronUpIcon className="h-4 w-4" />
					) : (
						<ChevronDownIcon className="h-4 w-4" />
					)}
				</Button>

				{showJson && (
					<div className="p-4 bg-stone-50 dark:bg-stone-900 border-t border-stone-200 dark:border-stone-800">
						<JsonViewer
							value={result}
							className="rounded-lg"
							theme="dark"
							enableClipboard
							displayDataTypes={false}
							displaySize={false}
							defaultInspectDepth={1}
						/>
					</div>
				)}
			</div>
		</Card>
	);
}
