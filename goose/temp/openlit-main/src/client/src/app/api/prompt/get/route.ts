import { getPrompts } from "@/lib/platform/prompt";

export async function POST() {
	const { err, data }: any = await getPrompts();
	if (err) {
		return Response.json(err, {
			status: 400,
		});
	}

	return Response.json(data);
}
