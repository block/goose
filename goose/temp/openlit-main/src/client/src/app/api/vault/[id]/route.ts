import { deleteSecret } from "@/lib/platform/vault";

export async function DELETE(_: Request, context: any) {
	const { id } = context.params;
	const [err, res] = await deleteSecret(id);
	if (err) {
		return Response.json(err, {
			status: 400,
		});
	}

	return Response.json(res);
}
