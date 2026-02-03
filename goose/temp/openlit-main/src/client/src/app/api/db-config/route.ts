import { getDBConfigByUser, upsertDBConfig } from "@/lib/db-config";
import asaw from "@/utils/asaw";
import { DatabaseConfig } from "@prisma/client";

export async function GET() {
	const [err, res]: any = await asaw(getDBConfigByUser());
	if (err)
		return Response.json(err, {
			status: 400,
		});

	return Response.json(res);
}

export async function POST(request: Request) {
	const formData = await request.json();
	const id = formData.id;

	const dbConfig: Partial<DatabaseConfig> = {
		name: formData.name,
		environment: formData.environment,
		username: formData.username,
		password: formData.password,
		host: formData.host,
		port: formData.port,
		database: formData.database,
		query: formData.query,
	};

	const [err, res]: any = await asaw(upsertDBConfig(dbConfig, id));

	if (err)
		return Response.json(err, {
			status: 400,
		});

	return Response.json(res);
}
