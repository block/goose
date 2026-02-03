import { getRequestExist } from "@/lib/platform/request";

export async function POST() {
	const res = await getRequestExist();
	const { data } = res;
	if ((data as any[])?.[0]?.total_requests > 0) {
		return Response.json(true);
	}

	return Response.json(false);
}
