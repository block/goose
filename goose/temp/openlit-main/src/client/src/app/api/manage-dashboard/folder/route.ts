import { SERVER_EVENTS } from "@/constants/events";
import {
	createFolder,
	getFolders,
	updateFolder,
} from "@/lib/platform/manage-dashboard/folder";
import PostHogServer from "@/lib/posthog";
import { Folder } from "@/types/manage-dashboard";
import { NextRequest } from "next/server";

export async function POST(request: NextRequest) {
	const startTimestamp = Date.now();
	const folder: Folder = await request.json();

	const res = await createFolder(folder);
	PostHogServer.fireEvent({
		event: res.err ? SERVER_EVENTS.FOLDER_CREATE_FAILURE : SERVER_EVENTS.FOLDER_CREATE_SUCCESS,
		startTimestamp,
	});
	return Response.json(res);
}

export async function PUT(request: NextRequest) {
	const startTimestamp = Date.now();
	const folder: Folder = await request.json();

	const res = await updateFolder(folder);
	PostHogServer.fireEvent({
		event: res.err ? SERVER_EVENTS.FOLDER_UPDATE_FAILURE : SERVER_EVENTS.FOLDER_UPDATE_SUCCESS,
		startTimestamp,
	});
	return Response.json(res);
}

export async function GET() {
	const startTimestamp = Date.now();
	const res = await getFolders();
	PostHogServer.fireEvent({
		event: res.err ? SERVER_EVENTS.FOLDER_GET_FAILURE : SERVER_EVENTS.FOLDER_GET_SUCCESS,
		startTimestamp,
		properties: {
			totalFolders: (res.data as Folder[])?.length || 0,
		},
	});
	return Response.json(res);
}
