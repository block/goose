import { SERVER_EVENTS } from "@/constants/events";
import {
	deleteWidget,
	getWidgetById,
	updateWidget,
} from "@/lib/platform/manage-dashboard/widget";
import PostHogServer from "@/lib/posthog";
import { Widget } from "@/types/manage-dashboard";
import { NextRequest } from "next/server";

// Delete widget
export async function DELETE(
	_: NextRequest,
	{ params: { id } }: { params: { id: string } }
) {
	const startTimestamp = Date.now();
	const res = await deleteWidget(id);
	PostHogServer.fireEvent({
		event: res.err ? SERVER_EVENTS.DASHBOARD_WIDGET_DELETE_FAILURE : SERVER_EVENTS.DASHBOARD_WIDGET_DELETE_SUCCESS,
		startTimestamp,
	});
	return Response.json(res);
}

// Get widget by id
export async function GET(
	_: NextRequest,
	{ params: { id } }: { params: { id: string } }
) {
	const startTimestamp = Date.now();
	const res = await getWidgetById(id);
	PostHogServer.fireEvent({
		event: res.err ? SERVER_EVENTS.DASHBOARD_WIDGET_GET_FAILURE : SERVER_EVENTS.DASHBOARD_WIDGET_GET_SUCCESS,
		startTimestamp,
	});
	return Response.json(res);
}

// Update widget
export async function PUT(request: NextRequest) {
	const startTimestamp = Date.now();
	const widget: Widget = await request.json();

	const res = await updateWidget(widget);
	PostHogServer.fireEvent({
		event: res.err ? SERVER_EVENTS.DASHBOARD_WIDGET_UPDATE_FAILURE : SERVER_EVENTS.DASHBOARD_WIDGET_UPDATE_SUCCESS,
		startTimestamp,
	});
	return Response.json(res);
}
