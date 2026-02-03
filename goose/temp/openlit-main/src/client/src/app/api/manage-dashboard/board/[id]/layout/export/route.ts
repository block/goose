import { SERVER_EVENTS } from "@/constants/events";
import { getBoardLayout } from "@/lib/platform/manage-dashboard/board";
import PostHogServer from "@/lib/posthog";
import { NextRequest } from "next/server";

// Export board layout
export async function GET(
	_: NextRequest,
	{ params: { id } }: { params: { id: string } }
) {
	const startTimestamp = Date.now();
	const res = await getBoardLayout(id);
	PostHogServer.fireEvent({
		event: res.err ? SERVER_EVENTS.DASHBOARD_LAYOUT_EXPORT_FAILURE : SERVER_EVENTS.DASHBOARD_LAYOUT_EXPORT_SUCCESS,
		startTimestamp,
	});
	if (res.err) {
		return new Response(JSON.stringify({ error: res.err }), {
			status: 404,
			headers: { "Content-Type": "application/json" },
		});
	}
	const json = JSON.stringify(res.data, null, 2);
	return new Response(json, {
		status: 200,
		headers: {
			"Content-Type": "application/json",
			"Content-Disposition": `attachment; filename=openlit-dashboard-${res.data!.title || res.data!.id}-layout.json`,
		},
	});
}
