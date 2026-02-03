"use client";

import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import Link from "next/link";
import { useCallback, useEffect } from "react";
import { toast } from "sonner";
import { format } from "date-fns";
import { useRootStore } from "@/store";
import { getPingStatus } from "@/selectors/database-config";
import { BookOpenText, TrashIcon } from "lucide-react";
import ConfirmationModal from "@/components/common/confirmation-modal";
import DataTable from "@/components/data-table/table";
import { Columns } from "@/components/data-table/columns";
import { PromptList } from "@/types/prompt";
import PromptsGettingStarted from "@/components/(playground)/getting-started/prompts";
import PromptHubHeader from "@/components/(playground)/prompt-hub/header";

const columns: Columns<string, PromptList> = {
	name: {
		header: () => "Name",
		cell: ({ row }) => row.name,
	},
	createdBy: {
		header: () => "Created By",
		cell: ({ row }) => row.createdBy,
	},
	latestVersion: {
		header: () => "Latest Version",
		cell: ({ row }) => {
			const latestVersion = row.latestVersion;
			return latestVersion || "draft";
		},
	},
	downloads: {
		header: () => "Downloads",
		cell: ({ row }) => {
			const totalDownloads = row.totalDownloads;
			const latestVersion = row.latestVersion;
			return latestVersion ? totalDownloads : "-";
		},
	},
	lastReleasedOn: {
		header: () => "Last Released On",
		cell: ({ row }) => {
			const latestVersionDate = row.latestVersionDate;
			const latestVersion = row.latestVersion;
			return latestVersion ? format(latestVersionDate, "MMM do, y") : "-";
		},
	},
	actions: {
		header: () => "Actions",
		cell: ({ row, extraFunctions }) => {
			return (
				<div className="flex justify-start items-center gap-4">
					<Link
						href={`/prompt-hub/${row.promptId}`}
						className="inline-block "
					>
						<BookOpenText className="w-4" />
					</Link>
					<ConfirmationModal
						handleYes={extraFunctions?.handleDelete}
						title="Are you sure you want to delete this prompt?"
						subtitle="Deleting prompts might result in breaking application if they are getting used. Please confirm before deleting it."
						params={{
							id: row.promptId,
						}}
					>
						<TrashIcon className="w-4 cursor-pointer" />
					</ConfirmationModal>
				</div>
			);
		},
	},
}

export default function PromptHub() {
	const { data, fireRequest, isFetched, isLoading } = useFetchWrapper<PromptList[]>();
	const { fireRequest: fireDeleteRequest, isLoading: isDeleting } =
		useFetchWrapper();
	const pingStatus = useRootStore(getPingStatus);
	const fetchData = useCallback(async () => {
		fireRequest({
			requestType: "POST",
			url: `/api/prompt/get`,
			failureCb: (err?: string) => {
				toast.error(err || `Cannot connect to server!`, {
					id: "prompt-hub",
				});
			},
		});
	}, []);

	const deletePrompt = useCallback(
		async ({ id }: { id: string }) => {
			fireDeleteRequest({
				requestType: "DELETE",
				url: `/api/prompt/${id}`,
				successCb: (data: any) => {
					toast.success(data, {
						id: "prompt-hub",
					});
					fetchData();
				},
				failureCb: (err?: string) => {
					toast.error(err || `Cannot connect to server!`, {
						id: "prompt-hub",
					});
				},
			});
		},
		[fetchData]
	);

	useEffect(() => {
		if (pingStatus !== "pending") {
			fetchData();
		}
	}, [pingStatus]);

	if (!data?.length && !isLoading && isFetched) {
		return (
			<div className="flex flex-col items-center mx-auto p-8 overflow-auto">
				<PromptsGettingStarted />
			</div>
		);
	}

	return (
		<div className="flex flex-col w-full h-full gap-4">
			<PromptHubHeader createNew />
			<DataTable
				columns={columns}
				data={data || []}
				isFetched={isFetched || pingStatus === "failure"}
				isLoading={isLoading || isDeleting}
				visibilityColumns={{
					name: true,
					createdBy: true,
					latestVersion: true,
					downloads: true,
					lastReleasedOn: true,
					actions: true
				}}
				extraFunctions={{
					handleDelete: deletePrompt,
				}}
			/>
		</div>
	);
}
