"use client";
import ConfirmationModal from "@/components/common/confirmation-modal";
import { Badge } from "@/components/ui/badge";
import { getPingStatus } from "@/selectors/database-config";
import { useRootStore } from "@/store";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import copy from "copy-to-clipboard";
import { format } from "date-fns";
import { CopyIcon, TrashIcon } from "lucide-react";
import { useCallback, useEffect } from "react";
import { toast } from "sonner";
import Generate from "./generate";
import { ApiKey } from "@/types/api-key";
import { Columns } from "@/components/data-table/columns";
import DataTable from "@/components/data-table/table";

const columns: Columns<string, ApiKey> = {
	name: {
		header: () => "Name",
		cell: ({ row }) => row.name,
	},
	apiKey: {
		header: () => "Api Key",
		cell: ({ row }) => {
			return (
				<Badge variant="outline" className="rounded-md">
					{row.apiKey?.replace(/(openlit-.{4}).*(.{6})$/, "$1...$2")}
				</Badge>
			);
		},
	},
	createdBy: {
		header: () => "Created By",
		cell: ({ row }) => {
			return row.createdByUser.email;
		},
	},
	createdAt: {
		header: () => "Created At",
		cell: ({ row }) => {
			return format(row.createdAt, "MMM do, y");
		},
	},
	actions: {
		header: () => "Actions",
		cell: ({ row, extraFunctions }) => {
			const copyAPIKey = () => {
				copy(row.apiKey);
				toast.success("Copied!", {
					id: "api-key",
				});
			};
			return (
				<div className="flex gap-4 justify-center">
					<ConfirmationModal
						handleYes={extraFunctions?.handleYes}
						title="Are you sure you want to delete?"
						subtitle="Deleting API keys might result in breaking application if they are getting used. Please confirm before deleting it."
						params={{
							id: row.id,
						}}
					>
						<TrashIcon className="w-4 cursor-pointer" />
					</ConfirmationModal>
					<CopyIcon
						className="w-4 cursor-pointer"
						onClick={copyAPIKey}
					/>
				</div>
			);
		},
	},
}

export default function ManageKeys() {
	const { data, fireRequest, isFetched, isLoading } = useFetchWrapper<ApiKey[]>();
	const { fireRequest: fireDeleteRequest } = useFetchWrapper();
	const pingStatus = useRootStore(getPingStatus);
	const fetchData = useCallback(async () => {
		fireRequest({
			requestType: "GET",
			url: `/api/api-key`,
			failureCb: (err?: string) => {
				toast.error(err || `Cannot connect to server!`, {
					id: "api-key",
				});
			},
		});
	}, []);

	useEffect(() => {
		if (pingStatus !== "pending") {
			fetchData();
		}
	}, [pingStatus]);

	const handleYes = useCallback(
		async ({ id }: { id: string }) => {
			fireDeleteRequest({
				requestType: "DELETE",
				url: `/api/api-key/${id}`,
				successCb: () => {
					fetchData();
				},
				failureCb: (err?: string) => {
					toast.error(err || `Cannot connect to server!`, {
						id: "api-key",
					});
				},
			});
		},
		[fetchData]
	);

	return (
		<div className="flex flex-col grow w-full gap-6 overflow-hidden">
			<Generate refresh={fetchData} />
			<DataTable
				columns={columns}
				data={data || []}
				isFetched={isFetched || pingStatus !== "pending"}
				isLoading={isLoading || pingStatus === "pending"}
				visibilityColumns={{
					name: true,
					apiKey: true,
					createdBy: true,
					createdAt: true,
					actions: true,
				}}
				extraFunctions={{
					handleYes,
				}}
			/>
		</div>
	);
}
