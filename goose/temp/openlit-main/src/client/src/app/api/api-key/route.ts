import { generateAPIKey, getAllAPIKeys } from "@/lib/platform/api-keys";
import asaw from "@/utils/asaw";

export async function GET() {
	const res: any = await getAllAPIKeys();
	return Response.json(res);
}

export async function POST(request: Request) {
	const formData = await request.json();
	const name = formData.name;

	const [err, res]: any = await asaw(generateAPIKey(name));

	if (err) {
		return Response.json(err, {
			status: 400,
		});
	}

	return Response.json(res);
}
