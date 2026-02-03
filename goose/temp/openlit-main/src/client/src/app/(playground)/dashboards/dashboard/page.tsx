"use client";
import Board from "@/components/(playground)/manage-dashboard/board";
import { usePostHog } from "posthog-js/react";
import { CLIENT_EVENTS } from "@/constants/events";
import { useEffect } from "react";

export default function BoardPage() {
	const posthog = usePostHog();

	useEffect(() => {
		posthog?.capture(CLIENT_EVENTS.LIST_DASHBOARDS_PAGE_VISITED);
	}, []);

	return <Board />;
}
