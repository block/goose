import { getAgentByInstanceId } from "@/lib/platform/fleet-hub";

export async function GET(_: Request, context: any) {
	const { id } = context.params;
	const res = await getAgentByInstanceId(id);
	return Response.json(res);
}
