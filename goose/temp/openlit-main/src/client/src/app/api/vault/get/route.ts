import { SecretGetFilters } from "@/types/vault";
import { getSecrets } from "@/lib/platform/vault";

export async function POST(request: Request) {
	const formData = await request.json();

	const filters: SecretGetFilters = {
		key: formData.key,
		tags: formData.tags,
	};

	const { err, data }: any = await getSecrets(filters);
	if (err) {
		return Response.json(err, {
			status: 400,
		});
	}

	return Response.json(data);
}
