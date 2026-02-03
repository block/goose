import { SERVER_EVENTS } from "@/constants/events";
import { createWidget, getWidgets } from "@/lib/platform/manage-dashboard/widget";
import PostHogServer from "@/lib/posthog";
import { Widget } from "@/types/manage-dashboard";
import { NextRequest } from "next/server";

export async function POST(request: NextRequest) {
	const widget: Widget = await request.json();
	const startTimestamp = Date.now();
	const res = await createWidget(widget);
	PostHogServer.fireEvent({
		event: res.err ? SERVER_EVENTS.DASHBOARD_WIDGET_CREATE_FAILURE : SERVER_EVENTS.DASHBOARD_WIDGET_CREATE_SUCCESS,
		startTimestamp,
	});
	return Response.json(res);
}

export async function GET() {
	const startTimestamp = Date.now();
	const res = await getWidgets();
	PostHogServer.fireEvent({
		event: res.err ? SERVER_EVENTS.DASHBOARD_WIDGET_LIST_FAILURE : SERVER_EVENTS.DASHBOARD_WIDGET_LIST_SUCCESS,
		startTimestamp,
	});
	return Response.json(res);
}
