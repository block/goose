import { SERVER_EVENTS } from "@/constants/events";
import { autoEvaluate } from "@/lib/platform/evaluation";
import PostHogServer from "@/lib/posthog";
import { NextRequest } from "next/server";

export async function POST(request: NextRequest) {
	const startTimestamp = Date.now();
	const formData = await request.json();
	const result = await autoEvaluate(formData);
	PostHogServer.fireEvent({
		event: result.success
			? SERVER_EVENTS.EVALUATION_AUTO_SUCCESS
			: SERVER_EVENTS.EVALUATION_AUTO_FAILURE,
		startTimestamp,
	});
	return Response.json(result);
}
