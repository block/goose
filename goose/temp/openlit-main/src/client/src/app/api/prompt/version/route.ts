import { PromptUpdate } from "@/constants/prompts";
import { upsertPromptVersion } from "@/lib/platform/prompt/version";
import asaw from "@/utils/asaw";

export async function POST(request: Request) {
	const formData = await request.json();

	const promptInput: PromptUpdate = {
		versionId: formData.versionId,
		promptId: formData.promptId,
		prompt: formData.prompt,
		version: formData.version,
		status: formData.status,
		tags: formData.tags,
		metaProperties: formData.metaProperties,
	};

	const [err, res]: any = await asaw(upsertPromptVersion(promptInput));

	if (err) {
		return Response.json(err, {
			status: 400,
		});
	}

	return Response.json(res);
}
