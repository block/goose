import { setCurrentDBConfig } from "@/lib/db-config";
import asaw from "@/utils/asaw";

export async function POST(_: Request, context: any) {
	const { id } = context.params;
	const [err, res] = await asaw(setCurrentDBConfig(id));
	if (err)
		return Response.json(err, {
			status: 400,
		});
	return Response.json(res);
}
