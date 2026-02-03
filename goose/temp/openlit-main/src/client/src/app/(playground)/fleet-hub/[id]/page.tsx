"use client";
import { Agent } from "@/types/fleet-hub";
import AgentDetail from "./agent";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import { useCallback, useEffect } from "react";
import { useParams } from "next/navigation";
import Loader from "@/components/common/loader";
import { usePostHog } from "posthog-js/react";
import { CLIENT_EVENTS } from "@/constants/events";

export default function AgentPage() {
	const { fireRequest, data, isLoading } = useFetchWrapper<Agent>();
	const params = useParams();
	const posthog = usePostHog();


	const fetchAgentInfo = useCallback(async () => {
		fireRequest({
			requestType: "GET",
			url: `/api/fleet-hub/${params.id}`,
			responseDataKey: "data",
			successCb: (response) => {
				if (response?.data) {
					posthog?.capture(CLIENT_EVENTS.FLEET_HUB_AGENT_VIEWED, {
						agentId: params.id,
					});
				}
			},
		});
	}, [params.id]);

	useEffect(() => {
		fetchAgentInfo();
	}, [fetchAgentInfo]);

	if (isLoading && !data) return (
		<div className="flex items-center justify-center w-full h-full">
			<Loader />
		</div>
	);

	if (!isLoading && !data) {
		return null;
	}

	return <AgentDetail agent={data!} fetchAgentInfo={fetchAgentInfo} />;
}