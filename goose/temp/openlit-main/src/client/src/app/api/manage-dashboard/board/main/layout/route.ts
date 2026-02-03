import { SERVER_EVENTS } from "@/constants/events";
import { getMainDashboard } from "@/lib/platform/manage-dashboard/board";
import PostHogServer from "@/lib/posthog";
import { NextRequest } from "next/server";

export async function GET(_: NextRequest) {
	const startTimestamp = Date.now();
	const res = await getMainDashboard(true);
	PostHogServer.fireEvent({
		event: res.err ? SERVER_EVENTS.DASHBOARD_GET_MAIN_FAILURE : SERVER_EVENTS.DASHBOARD_GET_MAIN_SUCCESS,
		startTimestamp,
	});
	return Response.json(res);
}
