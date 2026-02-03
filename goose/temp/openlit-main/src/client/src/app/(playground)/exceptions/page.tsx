"use client";
import { useCallback, useEffect } from "react";
import {
	RequestProvider,
	useRequest,
} from "@/components/(playground)/request/request-context";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import { toast } from "sonner";
import { getFilterDetails } from "@/selectors/filter";
import { useRootStore } from "@/store";
import { getPingStatus } from "@/selectors/database-config";
import { omit } from "lodash";
import RequestDetails from "@/components/(playground)/request/request-details";
import { normalizeTrace } from "@/helpers/client/trace";
import DataTable from "@/components/data-table/table";
import { columns } from "@/components/(playground)/exceptions/columns";
import { getVisibilityColumnsOfPage } from "@/selectors/page";
import TracesFilter from "@/components/(playground)/filter/traces-filter";
import ExceptionsGettingStarted from "@/components/(playground)/getting-started/tracing";

function ExceptionPage() {
	const filter = useRootStore(getFilterDetails);
	const [, updateRequest] = useRequest();

	const onClick = (item: any) => {
		!isLoading && updateRequest(item);
	};
	const pingStatus = useRootStore(getPingStatus);
	const { data, fireRequest, isFetched, isLoading } = useFetchWrapper();
	const { data: existData, fireRequest: fireExistRequest, isFetched: isFetchedExist, isLoading: isLoadingExist } = useFetchWrapper();
	useEffect(() => {
		fireExistRequest({
			requestType: "POST",
			url: "/api/metrics/request/exist",
		});
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, []);

	const visibilityColumns = useRootStore((state) =>
		getVisibilityColumnsOfPage(state, "exception")
	);

	const fetchData = useCallback(async () => {
		fireRequest({
			body: JSON.stringify(omit(filter, ["selectedConfig"])),
			requestType: "POST",
			url: "/api/metrics/exception",
			failureCb: (err?: string) => {
				toast.error(err || `Cannot connect to server!`, {
					id: "request-page",
				});
			},
		});
	}, [filter, fireRequest]);

	useEffect(() => {
		if (
			filter.timeLimit.start &&
			filter.timeLimit.end &&
			pingStatus === "success"
		)
			fetchData();
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, [filter, pingStatus]);

	const normalizedData = ((data as any)?.records || []).map(normalizeTrace);


	// Show getting started when there's no trace data AND initial fetch is complete
	if (existData === false && !isLoadingExist && isFetchedExist) {
		return (
			<div className="flex flex-col w-full h-full overflow-auto">
				<ExceptionsGettingStarted />
			</div>
		);
	}

	return (
		<>
			<TracesFilter
				total={(data as any)?.total}
				includeOnlySorting={["Timestamp"]}
				pageName={"exception"}
				columns={columns}
			/>
			<DataTable
				columns={columns}
				data={normalizedData}
				isFetched={isFetched || pingStatus !== "pending"}
				isLoading={isLoading || pingStatus === "pending"}
				visibilityColumns={visibilityColumns}
				onClick={onClick}
			/>
			<RequestDetails />
		</>
	);
}

export default function Page() {
	return (
		<RequestProvider>
			<ExceptionPage />
		</RequestProvider>
	);
}
