import { SERVER_EVENTS } from "@/constants/events";
import { deleteFolder, getFolderById } from "@/lib/platform/manage-dashboard/folder";
import PostHogServer from "@/lib/posthog";
import { NextRequest } from "next/server";

export async function DELETE(
	_: NextRequest,
	{ params: { id } }: { params: { id: string } }
) {
	const startTimestamp = Date.now();
	const res = await deleteFolder(id);
	PostHogServer.fireEvent({
		event: res.err ? SERVER_EVENTS.FOLDER_DELETE_FAILURE : SERVER_EVENTS.FOLDER_DELETE_SUCCESS,
		startTimestamp,
	});
	return Response.json(res);
}

export async function GET(
	_: NextRequest,
	{ params: { id } }: { params: { id: string } }
) {
	const res = await getFolderById(id);
	return Response.json(res);
}
