import { updateAgentConfig } from "@/lib/platform/fleet-hub";

export async function POST(request: Request, context: any) {
	const { id } = context.params;
	const { config } = await request.json();
	const res = await updateAgentConfig(id, config);

	// Check if there was an error from the OpAMP server
	if (res.err) {
		return Response.json(res.err,
			{ status: res.status || 500 }
		);
	}

	return Response.json(res);
}