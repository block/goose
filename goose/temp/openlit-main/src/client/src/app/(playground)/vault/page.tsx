"use client";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import { useCallback, useEffect } from "react";
import { toast } from "sonner";
import { format } from "date-fns";
import { useRootStore } from "@/store";
import { getPingStatus } from "@/selectors/database-config";
import { EditIcon, TrashIcon } from "lucide-react";
import ConfirmationModal from "@/components/common/confirmation-modal";
import VaultHeader from "@/components/(playground)/vault/header";
import SecretForm from "@/components/(playground)/vault/form";
import { Secret } from "@/types/vault";
import { Columns } from "@/components/data-table/columns";
import DataTable from "@/components/data-table/table";
import SecretsGettingStarted from "@/components/(playground)/getting-started/secrets";

const columns: Columns<string, Secret> = {
	key: {
		header: () => "Key",
		cell: ({ row }) => row.key,
	},
	createdBy: {
		header: () => "Created By",
		cell: ({ row }) => row.created_by,
	},
	updatedAt: {
		header: () => "Last Updated On",
		cell: ({ row }) => {
			return row.updated_at ? format(row.updated_at, "MMM do, y") : "-";
		},
	},
	actions: {
		header: () => "Actions",
		cell: ({ row, extraFunctions }) => {
			return (
				<div className="flex justify-start items-center gap-4">
					<SecretForm
						secretData={row}
						successCallback={extraFunctions.successCallback}
					>
						<EditIcon className="w-4 cursor-pointer" />
					</SecretForm>
					<ConfirmationModal
						handleYes={extraFunctions?.handleDelete}
						title="Are you sure you want to delete this secret?"
						subtitle="Deleting secrets might result in breaking application if they are getting used. Please confirm before deleting it."
						params={{
							id: row.id,
						}}
					>
						<TrashIcon className="w-4 cursor-pointer" />
					</ConfirmationModal>
				</div>
			);
		},
	},
}

export default function Vault() {
	const { data, fireRequest, isFetched, isLoading } = useFetchWrapper<Secret[]>();
	const { fireRequest: fireDeleteRequest, isLoading: isDeleting } =
		useFetchWrapper();
	const pingStatus = useRootStore(getPingStatus);
	const fetchData = useCallback(async () => {
		fireRequest({
			requestType: "POST",
			url: `/api/vault/get`,
			body: JSON.stringify(""),
			failureCb: (err?: string) => {
				toast.error(err || `Cannot connect to server!`, {
					id: "vault",
				});
			},
		});
	}, []);

	const deleteSecret = useCallback(
		async ({ id }: { id: string }) => {
			fireDeleteRequest({
				requestType: "DELETE",
				url: `/api/vault/${id}`,
				successCb: (data: any) => {
					toast.success(data, {
						id: "vault",
					});
					fetchData();
				},
				failureCb: (err?: string) => {
					toast.error(err || `Cannot connect to server!`, {
						id: "vault",
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
				<SecretsGettingStarted successCallback={fetchData} />
			</div>
		);
	}

	return (
		<div className="flex flex-col w-full h-full gap-4">
			<VaultHeader successCallback={fetchData} />
			<DataTable
				columns={columns}
				data={data || []}
				isFetched={isFetched || pingStatus === "failure"}
				isLoading={isLoading || isDeleting}
				visibilityColumns={{
					key: true,
					createdBy: true,
					updatedAt: true,
					actions: true
				}}
				extraFunctions={{
					handleDelete: deleteSecret,
					successCallback: fetchData,
				}}
			/>
		</div>
	);
}
