"use client";

import { useEffect, useMemo, useState } from "react";
import Dashboard from "@/components/(playground)/manage-dashboard/board-creator";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import { useParams } from "next/navigation";
import { DashboardConfig, Widget } from "@/components/(playground)/manage-dashboard/board-creator/types";
import getMessage from "@/constants/messages";
import Loader from "@/components/common/loader";
import Filter from "@/components/(playground)/filter";
import { getFilterParamsForDashboard } from "@/helpers/client/filter";
import { useFilters } from "@/selectors/filter";
import { useDynamicBreadcrumbs } from "@/utils/hooks/useBreadcrumbs";
import { usePostHog } from "posthog-js/react";
import { Board } from "@/types/manage-dashboard";
import { CLIENT_EVENTS } from "@/constants/events";

export default function DashboardPage() {
	const { details: filter } = useFilters();
	const { fireRequest, error, isLoading } = useFetchWrapper();
	const { fireRequest: fireRunQuery } = useFetchWrapper();
	const { fireRequest: fireRequestWidget } = useFetchWrapper();
	const { fireRequest: fireSaveLayout } = useFetchWrapper();
	const params = useParams();
	const boardId = params.id as string;
	const [initialConfig, setInitialConfig] = useState<
		DashboardConfig | undefined
	>();
	const [dashboardDetails, setDashboardDetails] = useState<{ title?: string; description?: string }>({});
	useDynamicBreadcrumbs(dashboardDetails, [dashboardDetails]);
	const posthog = usePostHog();

	const handleHeaderUpdates = (details: Partial<Board>) => {
		setDashboardDetails({
			title: details.title || "",
			description: details.description || "",
		});
	};

	const fetchBoardLayout = async () => {
		try {
			const { response, error } = await fireRequest({
				requestType: "GET",
				url: `/api/manage-dashboard/board/${boardId}/layout`,
			});

			if (error) {
				throw new Error(error);
			}

			if (response?.data) {
				setInitialConfig(response.data);
				handleHeaderUpdates(response.data);
			}
		} catch (error) {
			console.error("Failed to fetch board layout:", error);
		}
	};

	useEffect(() => {
		fetchBoardLayout();
	}, [boardId, fireRequest]);

	useEffect(() => {
		if (initialConfig) {
			posthog?.capture(CLIENT_EVENTS.DASHBOARD_VIEWED, {
				title: initialConfig?.title,
				widgets: initialConfig?.widgets?.length || 0,
			});
		}
	}, [initialConfig, posthog]);

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
			body: JSON.stringify({ widgetId, filter: runFilters, ...params }),
		});

		return data.response;
	};

	const handleSave = async (config: DashboardConfig) => {
		try {
			await fireSaveLayout({
				requestType: "PUT",
				url: `/api/manage-dashboard/board/${boardId}/layout`,
				body: JSON.stringify(config),
				failureCb: (error) => {
					throw new Error(error);
				},
			});
		} catch (error) {
			console.error("Failed to save dashboard layout:", error);
			throw error;
		}
	};

	const handleWidgetCrud = async (updates: Partial<Widget>) => {
		try {
			if (updates.id) {
				const { response, error } = await fireRequestWidget({
					requestType: "PUT",
					url: `/api/manage-dashboard/widget/${updates.id}`,
					body: JSON.stringify(updates),
				});

				if (error) {
					throw new Error(error);
				}

				return response.data;
			} else {
				const { response, error } = await fireRequestWidget({
					requestType: "POST",
					url: `/api/manage-dashboard/widget`,
					body: JSON.stringify(updates),
				});

				if (error) {
					throw new Error(error);
				}

				return response.data;
			}
		} catch (error) {
			console.error("Failed to update widget:", error);
			throw error;
		}
	};

	const fetchExistingWidgets = async () => {
		const { response, error } = await fireRequestWidget({
			requestType: "GET",
			url: "/api/manage-dashboard/widget",
		});

		if (error) {
			throw new Error(error);
		}

		return response.data;
	};

	return (
		<div className="flex flex-col w-full justify-between h-full">
			{
				!isLoading && (error as Error) && (
					<div className="flex flex-col items-center w-full justify-center h-full">
						<p className="text-xl text-red-500">
							{getMessage().ERROR_OCCURED}
						</p>
						<p className="text-sm text-stone-500 dark:text-stone-400">
							{getMessage().ERROR_OCCURED_DESCRIPTION}
						</p>
					</div>
				)
			}
			{
				isLoading && (
					<div className="flex flex-col items-center w-full justify-center h-full">
						<Loader />
					</div>
				)
			}
			{!isLoading && initialConfig && (
				<Dashboard
					className="h-100 overflow-y-auto"
					initialConfig={initialConfig}
					runQuery={runQuery}
					onSave={handleSave}
					handleWidgetCrud={handleWidgetCrud}
					fetchExistingWidgets={fetchExistingWidgets}
					runFilters={runFilters}
					headerComponent={(
						<Filter />
					)}
					handleBoardUpdates={handleHeaderUpdates}
				/>
			)}
		</div>
	);
}
