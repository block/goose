"use client";
import Filter from "@/components/(playground)/filter";
import { useEffect, useMemo, useState } from "react";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import { useSearchParams } from "next/navigation";
import Dashboard, { DashboardConfig } from "../../../components/(playground)/manage-dashboard/board-creator";
import { useRootStore } from "@/store";
import { getFilterDetails } from "@/selectors/filter";
import { getFilterParamsForDashboard } from "@/helpers/client/filter";
import Loader from "@/components/common/loader";
import { usePageHeader } from "@/selectors/page";
import { usePostHog } from "posthog-js/react";
import { CLIENT_EVENTS } from "@/constants/events";
import { toast } from "sonner";
import BoardList from "./board-list";
import DescriptionTooltip from "@/components/common/description-tooltip";
import { BookText } from "lucide-react";

export default function DashboardPage() {
	const filter = useRootStore(getFilterDetails);
	const { fireRequest, isLoading } = useFetchWrapper();
	const { fireRequest: fireRunQuery } = useFetchWrapper();
	const [initialConfig, setInitialConfig] = useState<
		DashboardConfig | undefined
	>();
	const searchParams = useSearchParams();
	const dashboardId = searchParams.get("dashboardId");
	const { setHeader } = usePageHeader();
	const posthog = usePostHog();

	useEffect(() => {
		const fetchBoardLayout = async () => {
			try {
				setInitialConfig(undefined);
				const { response, error } = await fireRequest({
					requestType: "GET",
					url: `/api/manage-dashboard/board/${dashboardId ? dashboardId : "main"}/layout`,
				});

				if (error) {
					throw new Error(error);
				}

				if (response?.data) {
					setInitialConfig(response.data);
					setHeader({
						title: response.data?.title,
						description: response.data?.description,
						breadcrumbs: [],
					});
					posthog?.capture(CLIENT_EVENTS.DASHBOARD_VIEWED, {
						id: response.data.id,
						home: true,
						mainDashboard: response.data.isMainDashboard,
					});
				}
			} catch (error) {
				toast.error("Failed to fetch board layout", {
					id: "dashboard-page",
				});
				posthog?.capture(CLIENT_EVENTS.DASHBOARD_VIEW_FAILURE, {
					error: error?.toString(),
					home: true,
					mainDashboard: !dashboardId,
				});
			}
		};

		fetchBoardLayout();
	}, [dashboardId, fireRequest]);

	const runFilters = useMemo(() => {
		return getFilterParamsForDashboard({
			...filter,
		});
	}, [filter]);

	const runQuery = async (
		widgetId: string,
		params: Record<string, unknown>
	) => {
		const data = await fireRunQuery({
			requestType: "POST",
			url: "/api/manage-dashboard/query/run",
			// Add params of the dashboard & add a check for edit mode to send query or not through body
			body: JSON.stringify({
				widgetId,
				filter: runFilters,
				...params,
			}),
		});

		return data.response;
	};

	return (
		<>
			<div className="flex w-full items-center justify-between mb-4 gap-4">
				{initialConfig?.description && (
					<DescriptionTooltip description={initialConfig.description} className="ml-2 h-4 w-4" icon={<BookText className="text-stone-500 cursor-pointer" />} />
				)}
				<Filter />
				<BoardList dashboardId={dashboardId} />
			</div>
			{
				isLoading ? (
					<div className="flex items-center justify-center h-full w-full">
						<Loader />
					</div>
				) : null
			}
			{initialConfig ? (
				<Dashboard
					className="h-100 overflow-y-auto"
					initialConfig={initialConfig}
					readonly
					runQuery={runQuery}
					runFilters={runFilters}
				/>
			) : null}
		</>
	);
}
