import { deleteDBConfig } from "@/lib/db-config";
import asaw from "@/utils/asaw";

export async function DELETE(_: Request, context: any) {
	const { id } = context.params;
	const [err, res] = await asaw(deleteDBConfig(id));
	if (err)
		return Response.json(err, {
			status: 400,
		});
	return Response.json(res);
}
