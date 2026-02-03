"use client";
import { useEffect } from "react";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import { Agent } from "@/types/fleet-hub";
import List from "./list";
import OpenTelemetrySvg from "@/components/svg/opentelemetry";
import { usePostHog } from "posthog-js/react";
import { CLIENT_EVENTS } from "@/constants/events";

export default function Opamp() {
	const { fireRequest, data, isLoading, isFetched } = useFetchWrapper<Agent[]>();
	const posthog = usePostHog();

	useEffect(() => {
		fireRequest({
			requestType: "GET",
			url: `/api/fleet-hub`,
			responseDataKey: "data",
			successCb: (response) => {
				if (response?.data) {
					posthog?.capture(CLIENT_EVENTS.FLEET_HUB_VIEWED, {
						count: response?.data?.length || 0,
					});
				}
			},
		});
	}, []);

	return (
		<div className="flex flex-col w-full gap-3">
			{/* Feature Description Section */}
			<div className="border dark:border-stone-800 rounded-lg p-6">
				<div className="flex items-start space-x-4">
					<div className="flex-shrink-0">
						<div className="w-10 h-10 bg-stone-200 dark:bg-stone-700 rounded-full p-1.5 flex items-center justify-center">
							<OpenTelemetrySvg className="fill-stone-600 dark:fill-stone-300" />
						</div>
					</div>
					<div className="flex-1">
						<h2 className="text-xl font-semibold text-stone-600 dark:text-stone-300 mb-2">
							Fleet Hub
						</h2>
						<p className="text-stone-600 dark:text-stone-300 mb-4">
							Fleet hub is a feature that uses OpAMP (Open Agent Management Protocol) to provide centralized management and monitor OpenTelemetry collectors across your infrastructure. 
							Monitor collector health, view detailed configurations, and manage collector settings from a unified dashboard.
						</p>
					</div>
				</div>
			</div>

			{/* Collector List */}
			<List agents={data || []} isLoading={!data || isLoading} isFetched={isFetched} />
		</div>
	);
}