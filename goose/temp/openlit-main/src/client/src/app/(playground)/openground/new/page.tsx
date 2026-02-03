"use client";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useRootStore } from "@/store";
import { PlayIcon, RotateCcwIcon, Loader2Icon } from "lucide-react";
import { toast } from "sonner";
import OpengroundHeader from "@/components/(playground)/openground/header";
import PromptSourceToggle from "@/components/(playground)/openground/prompt-source-toggle";
import DynamicProviderGrid from "@/components/(playground)/openground/dynamic-provider-grid";
import ProviderSettingsPanel from "@/components/(playground)/openground/provider-settings-panel";
import MetricsOverview from "@/components/(playground)/openground/metrics-overview";
import PerformanceWaterfall from "@/components/(playground)/openground/performance-waterfall";
import CostBreakdown from "@/components/(playground)/openground/cost-breakdown";
import ProviderResultCard from "@/components/(playground)/openground/provider-result-card";
import { ProviderResult } from "@/lib/platform/openground-clickhouse";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import getMessage from "@/constants/messages";

export default function OpengroundNew() {
	const promptSource = useRootStore((state) => state.openground.promptSource);
	const selectedProvidersNew = useRootStore((state) => state.openground.selectedProvidersNew);
	const evaluatedResponse = useRootStore((state) => state.openground.evaluatedResponse);
	const setEvaluatedLoading = useRootStore((state) => state.openground.setEvaluatedLoading);
	const setEvaluatedData = useRootStore((state) => state.openground.setEvaluatedData);
	const reset = useRootStore((state) => state.openground.reset);

	const { fireRequest: fireEvaluateRequest } = useFetchWrapper();

	const handleEvaluate = () => {
		// Validation
		if (selectedProvidersNew.length === 0) {
			toast.error(getMessage().OPENGROUND_SELECT_PROVIDER_ERROR);
			return;
		}

		const prompt =
			promptSource.type === "custom"
				? promptSource.content
				: promptSource.content;

		if (!prompt?.trim()) {
			toast.error(getMessage().OPENGROUND_ENTER_PROMPT_ERROR);
			return;
		}

		// Check variables
		const variableRegex = /\{\{([^}]+)\}\}/g;
		const variables: string[] = [];
		let match;
		while ((match = variableRegex.exec(prompt)) !== null) {
			variables.push(match[1].trim());
		}

		const missingVars = variables.filter(
			(v) => !promptSource.variables?.[v]?.trim()
		);
		if (missingVars.length > 0) {
			toast.error(`${getMessage().OPENGROUND_FILL_VARIABLES_ERROR}: ${missingVars.join(", ")}`);
			return;
		}

		setEvaluatedLoading(true);

		fireEvaluateRequest({
			requestType: "POST",
			url: "/api/openground",
			body: JSON.stringify({
				promptSource,
				providers: selectedProvidersNew.map((p) => ({
					provider: p.provider,
					model: p.model,
					config: p.config,
				})),
			}),
			successCb: (data: any) => {
				setEvaluatedData(data);
				toast.success(getMessage().OPENGROUND_EVALUATION_SUCCESS, {
					id: "evaluation",
				});
			},
			failureCb: (err?: string) => {
				toast.error(err || getMessage().OPENGROUND_EVALUATION_FAILED, {
					id: "evaluation",
				});
				setEvaluatedLoading(false);
			},
		});
	};

	const handleReset = () => {
		reset();
		toast.success(getMessage().OPENGROUND_RESET_SUCCESS);
	};

	const canEvaluate =
		!evaluatedResponse.isLoading &&
		selectedProvidersNew.length > 0 &&
		(promptSource.type === "custom"
			? promptSource.content?.trim()
			: promptSource.promptId);

	// Type guard for new format
	const providerResults = (evaluatedResponse.data && Array.isArray(evaluatedResponse.data)
		&& evaluatedResponse.data.length > 0
		&& 'provider' in evaluatedResponse.data[0])
		? evaluatedResponse.data as ProviderResult[]
		: null;

	return (
		<div className="flex flex-col w-full h-full gap-6 overflow-auto">
			<OpengroundHeader validateResponse={false} />

			{/* Main Content */}
			<div className="space-y-6">
				{/* Step 1: Prompt Configuration */}
				<PromptSourceToggle />

				{/* Step 2: Provider Selection */}
				<DynamicProviderGrid />

				{/* Step 3: Provider Settings */}
				<ProviderSettingsPanel />

				{/* Action Buttons */}
				<Card className="p-4 border-stone-200 dark:border-stone-800">
					<div className="flex items-center justify-between">
						<div className="text-sm text-stone-600 dark:text-stone-400">
							{selectedProvidersNew.length === 0 ? (
								getMessage().OPENGROUND_SELECT_PROVIDERS_BEGIN
							) : evaluatedResponse.data ? (
								<span className="text-green-600 dark:text-green-400 font-medium">
									✓ {getMessage().OPENGROUND_EVALUATION_COMPLETE}
								</span>
							) : (
								`${getMessage().OPENGROUND_READY_TO_EVALUATE} ${selectedProvidersNew.length} provider${
									selectedProvidersNew.length > 1 ? "s" : ""
								}`
							)}
						</div>
						<div className="flex gap-2">
							{evaluatedResponse.data && (
								<Button
									variant="outline"
									onClick={handleReset}
									className="gap-2"
								>
									<RotateCcwIcon className="h-4 w-4" />
									{getMessage().RESET}
								</Button>
							)}
							<Button
								onClick={handleEvaluate}
								disabled={!canEvaluate || evaluatedResponse.isLoading}
								className="gap-2"
							>
								{evaluatedResponse.isLoading ? (
									<>
										<Loader2Icon className="h-4 w-4 animate-spin" />
										{getMessage().OPENGROUND_EVALUATING}
									</>
								) : (
									<>
										<PlayIcon className="h-4 w-4" />
										{getMessage().OPENGROUND_EVALUATE_PROVIDERS}
									</>
								)}
							</Button>
						</div>
					</div>
				</Card>

				{/* Results Section */}
				{providerResults && (
					<div className="space-y-6">
						{/* Metrics Overview */}
						<MetricsOverview data={providerResults} />

						{/* Performance & Cost Analysis */}
						<div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
							<PerformanceWaterfall data={providerResults} />
							<CostBreakdown data={providerResults} />
						</div>

						{/* Detailed Results */}
						<Card className="border-stone-200 dark:border-stone-800">
							<CardHeader>
								<CardTitle className="text-lg">{getMessage().OPENGROUND_PROVIDER_RESPONSES}</CardTitle>
							</CardHeader>
							<CardContent className="space-y-4">
								{providerResults.map((result, index) => (
									<ProviderResultCard key={index} result={result} index={index} />
								))}
							</CardContent>
						</Card>
					</div>
				)}

				{/* Loading State */}
				{evaluatedResponse.isLoading && (
					<Card className="p-12 border-stone-200 dark:border-stone-800">
						<div className="flex flex-col items-center justify-center gap-4">
							<Loader2Icon className="h-12 w-12 animate-spin text-blue-500" />
							<div className="text-center">
								<p className="text-lg font-medium text-stone-900 dark:text-stone-100">
									{getMessage().OPENGROUND_EVALUATING_PROVIDERS}
								</p>
								<p className="text-sm text-stone-500 dark:text-stone-400 mt-1">
									{getMessage().OPENGROUND_MAY_TAKE_FEW_SECONDS}
								</p>
							</div>
						</div>
					</Card>
				)}
			</div>
		</div>
	);
}
