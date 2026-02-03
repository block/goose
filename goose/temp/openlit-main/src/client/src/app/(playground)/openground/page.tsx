"use client";

import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import Link from "next/link";
import { useCallback, useEffect } from "react";
import { toast } from "sonner";
import { Component, EyeIcon, MonitorPlay, PlayIcon } from "lucide-react";
import OpengroundHeader from "@/components/(playground)/openground/header";
import { Columns } from "@/components/data-table/columns";
import DataTable from "@/components/data-table/table";
import OpengroundGettingStarted from "@/components/(playground)/getting-started/openground";
import { OpengroundRecord } from "@/lib/platform/openground-clickhouse";
import { format } from "date-fns";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useRootStore } from "@/store";
import { useRouter } from "next/navigation";
import getMessage from "@/constants/messages";
import FeatureHero from "@/components/(playground)/getting-started/feature-hero";

const columns: Columns<string, OpengroundRecord> = {
	prompt: {
		header: () => "Prompt",
		cell: ({ row }) => {
			return (
				<div className="max-w-md">
					<p className="truncate text-sm font-medium text-stone-900 dark:text-stone-100">
						{row.promptSource === "prompt-hub" && (
							<Component className="w-3 h-3 inline mr-2" />
						)}
						{row.prompt}
					</p>
				</div>
			);
		},
	},
	providers: {
		header: () => "Providers",
		cell: ({ row }) => {
			return (
				<div className="flex gap-1 items-center ">
					<span className="text-green-500">{row.totalProviders - (row.errors?.length || 0)}</span>
					<div className="bg-stone-200 dark:bg-stone-800 h-3 w-0.5 shrink-0" />
					<span className="text-red-500">{(row.errors?.length || 0)}</span>
				</div>
			);
		},
	},
	bestCost: {
		header: () => "Best Cost",
		cell: ({ row }) => {
			return row.minCostProvider !== undefined ? (
				<div className="flex flex-col">
					<span className="text-sm font-medium text-green-600 dark:text-green-400">
						${row.minCost.toFixed(6)}
					</span>
					<span className="text-xs text-stone-500 dark:text-stone-400">
						{row.minCostProvider}
					</span>
				</div>
			) : (
				<span className="text-xs text-stone-400">-</span>
			);
		},
	},
	bestSpeed: {
		header: () => "Best Speed",
		cell: ({ row }) => {
			return row.minResponseTimeProvider !== undefined ? (
				<div className="flex flex-col">
					<span className="text-sm font-medium text-blue-600 dark:text-blue-400">
						{row.minResponseTime.toFixed(2)}s
					</span>
					<span className="text-xs text-stone-500 dark:text-stone-400">
						{row.minResponseTimeProvider}
					</span>
				</div>
			) : (
				<span className="text-xs text-stone-400">-</span>
			);
		},
	},
	bestEfficiency: {
		header: () => getMessage().BEST_EFFICIENCY,
		cell: ({ row }) => {
			return row.minCompletionTokensProvider !== undefined ? (
				<div className="flex flex-col">
					<span className="text-sm font-medium text-purple-600 dark:text-purple-400">
						{row.minCompletionTokens} tokens
					</span>
					<span className="text-xs text-stone-500 dark:text-stone-400">
						{row.minCompletionTokensProvider}
					</span>
				</div>
			) : (
				<span className="text-xs text-stone-400">-</span>
			);
		},
	},
	createdAt: {
		header: () => "Created",
		cell: ({ row }) => {
			return (
				<span className="text-sm text-stone-600 dark:text-stone-400">
					{format(new Date(row.createdAt), "MMM d, y HH:mm")}
				</span>
			);
		},
	},
	actions: {
		header: () => "Actions",
		cell: ({ row, extraFunctions }) => {
			return (
				<div className="flex items-center gap-2">
					<Link href={`/openground/${row.id}`}>
						<Button variant="ghost" size="icon" className="h-8 w-8">
							<EyeIcon className="h-4 w-4" />
						</Button>
					</Link>
					<Button
						variant="ghost"
						size="icon"
						className="h-8 w-8"
						onClick={() => extraFunctions?.handleRerun(row)}
					>
						<PlayIcon className="h-4 w-4" />
					</Button>
				</div>
			);
		},
	},
}

export default function Openground() {
	const router = useRouter();
	const { data, fireRequest, isFetched, isLoading } = useFetchWrapper<OpengroundRecord[]>();
	const setPromptSource = useRootStore((state) => state.openground.setPromptSource);
	const reset = useRootStore((state) => state.openground.reset);

	const fetchData = useCallback(async () => {
		fireRequest({
			requestType: "GET",
			url: `/api/openground`,
			failureCb: (err?: string) => {
				toast.error(err || getMessage().CANNOT_CONNECT_TO_SERVER, {
					id: "openground",
				});
			},
		});
	}, []);

	const handleRerun = useCallback((record: OpengroundRecord) => {
		// Reset store and load the evaluation data
		reset();

		// Set prompt source
		setPromptSource({
			type: record.promptSource,
			content: record.prompt,
			promptId: record.promptHubId,
			version: record.promptHubVersion ? parseInt(record.promptHubVersion) : undefined,
			variables: record.promptVariables,
		});

		// Navigate to new evaluation page
		toast.success(getMessage().OPENGROUND_EVALUATION_LOADED, {
			id: "openground-rerun",
		});
		router.push("/openground/new");
	}, [reset, setPromptSource, router]);

	useEffect(() => {
		fetchData();
	}, []);

	if (!data?.length && !isLoading && isFetched) {
		return (
			<div className="flex flex-col items-center mx-auto p-8 overflow-auto">
				<OpengroundGettingStarted />
			</div>
		);
	}

	return (
		<div className="flex flex-col w-full h-full gap-4">
			<OpengroundHeader validateResponse={false} />
			<FeatureHero 
				iconComponent={<MonitorPlay />}
				title={getMessage().FEATURE_OPENGROUND}
				description={getMessage().GET_STARTED_WITH_OPENGROUND_DESCRIPTION}
			/>

			<DataTable
				columns={columns}
				data={data || []}
				isFetched={isFetched}
				isLoading={isLoading}
				visibilityColumns={{
					prompt: true,
					providers: true,
					bestCost: true,
					bestSpeed: true,
					bestEfficiency: true,
					createdAt: true,
					actions: true
				}}
				extraFunctions={{
					handleRerun,
				}}
			/>
		</div>
	);
}
