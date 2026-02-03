"use client";

import DashboardExplorer from "@/components/(playground)/manage-dashboard/explorer";
import { usePostHog } from "posthog-js/react";
import { CLIENT_EVENTS } from "@/constants/events";
import { useEffect } from "react";

export default function DashboardPage() {
	const posthog = usePostHog();

	useEffect(() => {
		posthog?.capture(CLIENT_EVENTS.EXPLORE_DASHBOARDS_PAGE_VISITED);
	}, []);
	return <DashboardExplorer />;
}
