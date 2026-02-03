"use client";
import Widget from "@/components/(playground)/manage-dashboard/widget";
import { usePostHog } from "posthog-js/react";
import { CLIENT_EVENTS } from "@/constants/events";
import { useEffect } from "react";

export default function WidgetPage() {
	const posthog = usePostHog();

	useEffect(() => {
		posthog?.capture(CLIENT_EVENTS.LIST_WIDGETS_PAGE_VISITED);
	}, []);
	return <Widget />;
}
