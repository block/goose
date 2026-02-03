import { SERVER_EVENTS } from "@/constants/events";
import { importBoardLayout } from "@/lib/platform/manage-dashboard/board";
import PostHogServer from "@/lib/posthog";
import { NextRequest } from "next/server";

export async function POST(request: NextRequest) {
  const startTimestamp = Date.now();
  const data = await request.json();
  const { err, data: boardData } = await importBoardLayout(data);
  PostHogServer.fireEvent({
    event: err ? SERVER_EVENTS.DASHBOARD_IMPORT_FAILURE : SERVER_EVENTS.DASHBOARD_IMPORT_SUCCESS,
    startTimestamp,
  });

  if (err) {
    return new Response(JSON.stringify({ error: err }), {
      status: 400,
      headers: { "Content-Type": "application/json" }
    });
  }

  return new Response(JSON.stringify({ data: boardData }), {
    status: 200,
    headers: { "Content-Type": "application/json" }
  });
}
