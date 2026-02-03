import { evaluate } from "@/lib/platform/openground/evaluate";
import { getOpengroundEvaluations } from "@/lib/platform/openground-clickhouse";
import { getCurrentUser } from "@/lib/session";
import { getDBConfigByUser } from "@/lib/db-config";
import asaw from "@/utils/asaw";
import getMessage from "@/constants/messages";

export async function GET() {
	const user = await getCurrentUser();
	if (!user) {
		return Response.json({ error: getMessage().UNAUTHORIZED_USER }, { status: 401 });
	}

	const [, dbConfig] = await asaw(getDBConfigByUser(true));
	if (!dbConfig) {
		return Response.json(
			{ error: getMessage().DATABASE_CONFIG_NOT_FOUND },
			{ status: 404 }
		);
	}

	const { data, err } = await getOpengroundEvaluations(user.id, dbConfig.id, {
		page: 1,
		limit: 100,
	});

	if (err) {
		return Response.json({ error: err }, { status: 500 });
	}

	return Response.json(data);
}

export async function POST(request: Request) {
	const user = await getCurrentUser();
	if (!user) {
		return Response.json({ error: getMessage().UNAUTHORIZED_USER }, { status: 401 });
	}

	const [, dbConfig] = await asaw(getDBConfigByUser(true));
	if (!dbConfig) {
		return Response.json(
			{ error: getMessage().DATABASE_CONFIG_NOT_FOUND },
			{ status: 404 }
		);
	}

	const body = await request.json();
	const { promptSource, providers } = body;

	if (!promptSource || !providers || !Array.isArray(providers)) {
		return Response.json(
			{ error: "Invalid request body: promptSource and providers are required" },
			{ status: 400 }
		);
	}

	const { data, err } = await evaluate({
		promptSource,
		providers,
		userId: user.id,
		databaseConfigId: dbConfig.id,
	});

	if (err) {
		return Response.json({ error: err }, { status: 500 });
	}

	return Response.json(data);
}
