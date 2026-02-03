import { getAllAgents } from "@/lib/platform/fleet-hub";

export async function GET() {
	const res: any = await getAllAgents();
	return Response.json(res);
}