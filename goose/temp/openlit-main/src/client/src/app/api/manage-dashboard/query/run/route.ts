// import { SERVER_EVENTS } from "@/constants/events";
import { runWidgetQuery } from "@/lib/platform/manage-dashboard/widget";
// import PostHogServer from "@/lib/posthog";
import { NextRequest } from "next/server";

export async function POST(request: NextRequest) {
	const {
		widgetId,
		userQuery,
		filter,
	} = await request.json();
	const startTimestamp = Date.now();
	const res = await runWidgetQuery(widgetId, {
		userQuery,
		filter,
	});
	// Commenting out PostHogServer.fireEvent to avoid sending events to PostHog
	// PostHogServer.fireEvent({
	// 	event: res.err ? SERVER_EVENTS.DASHBOARD_QUERY_RUN_FAILURE : SERVER_EVENTS.DASHBOARD_QUERY_RUN_SUCCESS,
	// 	startTimestamp,
	// });
	return Response.json(res);
}
