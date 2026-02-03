import { SERVER_EVENTS } from "@/constants/events";
import {
	getBoardLayout,
	updateBoardLayout,
} from "@/lib/platform/manage-dashboard/board";
import PostHogServer from "@/lib/posthog";
import { NextRequest } from "next/server";

// Get board layout
export async function GET(
	_: NextRequest,
	{ params: { id } }: { params: { id: string } }
) {
	const startTimestamp = Date.now();
	const res = await getBoardLayout(id);
	PostHogServer.fireEvent({
		event: res.err ? SERVER_EVENTS.DASHBOARD_LAYOUT_GET_FAILURE : SERVER_EVENTS.DASHBOARD_LAYOUT_GET_SUCCESS,
		startTimestamp,
	});
	return Response.json(res);
}

// Update board layout
export async function PUT(
	request: NextRequest,
	{ params }: { params: { id: string } }
) {
	const startTimestamp = Date.now();
	const layoutConfig = await request.json();
	const boardId = params.id;

	const res = await updateBoardLayout(boardId, layoutConfig);
	PostHogServer.fireEvent({
		event: res.err ? SERVER_EVENTS.DASHBOARD_LAYOUT_UPDATE_FAILURE : SERVER_EVENTS.DASHBOARD_LAYOUT_UPDATE_SUCCESS,
		startTimestamp,
	});
	return Response.json(res);
}
