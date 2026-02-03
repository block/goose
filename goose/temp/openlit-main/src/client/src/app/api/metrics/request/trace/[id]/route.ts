import { getRequestViaTraceId } from "@/lib/platform/request";

export async function GET(_: Request, context: any) {
	const { id } = context.params || {};

	if (!id)
		return Response.json("No parent span id provided", {
			status: 400,
		});

	const res: any = await getRequestViaTraceId(id);
	return Response.json(res);
}
