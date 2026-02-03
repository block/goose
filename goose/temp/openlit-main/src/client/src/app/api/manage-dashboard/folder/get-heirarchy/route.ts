import { SERVER_EVENTS } from "@/constants/events";
import { getHeirarchy } from "@/lib/platform/manage-dashboard/heirarchy";
import PostHogServer from "@/lib/posthog";

export async function GET() {
	const startTimestamp = Date.now();
	const res = await getHeirarchy();
	PostHogServer.fireEvent({
		event: res.err ? SERVER_EVENTS.FOLDER_GET_HEIRARCHY_FAILURE : SERVER_EVENTS.FOLDER_GET_HEIRARCHY_SUCCESS,
		startTimestamp,
	});
	return Response.json(res);
}
