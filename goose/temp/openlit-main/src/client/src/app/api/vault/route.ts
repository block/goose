import { SecretInput } from "@/types/vault";
import { upsertSecret } from "@/lib/platform/vault";
import asaw from "@/utils/asaw";

export async function POST(request: Request) {
	const formData = await request.json();

	const promptInput: Partial<SecretInput> = {
		key: formData.key as string,
		value: formData.value as string,
		tags: formData.tags as string[],
	};

	const [err, res]: any = await asaw(upsertSecret(promptInput));

	if (err) {
		return Response.json(err, {
			status: 400,
		});
	}

	return Response.json(res);
}

export async function PUT(request: Request) {
	const formData = await request.json();

	const secretInput: Partial<SecretInput> = {
		id: formData.id,
		key: formData.key as string,
		value: formData.value as string,
		tags: formData.tags as string[],
	};

	const [err, res]: any = await asaw(upsertSecret(secretInput));

	if (err) {
		return Response.json(err, {
			status: 400,
		});
	}

	return Response.json(res);
}
