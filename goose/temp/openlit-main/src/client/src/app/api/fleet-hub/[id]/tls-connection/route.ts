import { updateTlsConnection } from "@/lib/platform/fleet-hub";

export async function POST(request: Request, context: any) {
	const { id } = context.params;
	const { tlsMin } = await request.json();
	const res = await updateTlsConnection(id, tlsMin);
	return Response.json(res);
}