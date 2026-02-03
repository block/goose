import { SERVER_EVENTS } from "@/constants/events";
import getMessage from "@/constants/messages";
import { SecretGetFiltersWithApiKey } from "@/types/vault";
import { getSecretsFromDatabaseId } from "@/lib/platform/vault";
import PostHogServer from "@/lib/posthog";
import asaw from "@/utils/asaw";

export async function POST(request: Request) {
	const startTimestamp = Date.now();
	const authorizationHeader = request.headers.get("Authorization") || "";
	let apiKey: string = "";
	if (authorizationHeader.startsWith("Bearer ")) {
		apiKey = authorizationHeader.replace(/^Bearer /, "");
	} else {
		return Response.json({
			err: getMessage().NO_API_KEY,
			res: null,
		});
	}

	const formData = await request.json();

	const filters: SecretGetFiltersWithApiKey = {
		apiKey,
		key: formData.key,
		tags: formData.tags,
	};

	const [err, data]: any = await asaw(getSecretsFromDatabaseId(filters));
	PostHogServer.fireEvent({
		event: err
			? SERVER_EVENTS.VAULT_SECRET_SDK_FETCH_FAILURE
			: SERVER_EVENTS.VAULT_SECRET_SDK_FETCH_SUCCESS,
		properties: {
			downloadSource: formData.source,
		},
		startTimestamp,
	});

	return Response.json({
		err,
		res: data,
	});
}

export async function OPTIONS() {
	return new Response(null, {
		status: 200,
		headers: {
			"Access-Control-Allow-Origin": "*",
			"Access-Control-Allow-Methods": "POST, OPTIONS",
			"Access-Control-Allow-Headers": "Content-Type, Authorization",
		},
	});
}
