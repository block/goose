import { shareDBConfig } from "@/lib/db-config";
import asaw from "@/utils/asaw";

export async function POST(request: Request) {
	const formData = await request.json();
	const shareArray = formData.shareArray;
	const id = formData.id;
	const [err, res] = await asaw(shareDBConfig({ id, shareArray }));
	if (err)
		return Response.json(err, {
			status: 400,
		});
	return Response.json(res);
}
