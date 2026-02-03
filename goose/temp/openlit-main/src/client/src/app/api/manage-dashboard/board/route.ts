import { SERVER_EVENTS } from "@/constants/events";
import {
	createBoard,
	getBoards,
	updateBoard,
} from "@/lib/platform/manage-dashboard/board";
import PostHogServer from "@/lib/posthog";
import { Board } from "@/types/manage-dashboard";
import { NextRequest } from "next/server";

export async function POST(request: NextRequest) {
	const startTimestamp = Date.now();
	const board: Board = await request.json();

	const res = await createBoard(board);
	PostHogServer.fireEvent({
		event: res.err ? SERVER_EVENTS.DASHBOARD_CREATE_FAILURE : SERVER_EVENTS.DASHBOARD_CREATE_SUCCESS,
		startTimestamp,
	});
	return Response.json(res);
}

export async function PUT(request: NextRequest) {
	const startTimestamp = Date.now();
	const board: Board & { updateParent?: boolean } = await request.json();
	const res = await updateBoard(board);
	PostHogServer.fireEvent({
		event: res.err ? SERVER_EVENTS.DASHBOARD_UPDATE_FAILURE : SERVER_EVENTS.DASHBOARD_UPDATE_SUCCESS,
		startTimestamp,
	});
	return Response.json(res);
}

export async function GET(request: NextRequest) {
	const startTimestamp = Date.now();
	const isHome = request.nextUrl.searchParams.get("home") === "true";
	const res = await getBoards(isHome);
	PostHogServer.fireEvent({
		event: res.err ? SERVER_EVENTS.DASHBOARD_GET_FAILURE : SERVER_EVENTS.DASHBOARD_GET_SUCCESS,
		startTimestamp,
		properties: {
			isHome,
			totalBoards: (res.data as Board[])?.length || 0,
		},
	});
	return Response.json(res);
}
