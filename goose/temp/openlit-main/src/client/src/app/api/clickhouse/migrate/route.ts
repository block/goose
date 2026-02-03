import { runClickhouseMigrations } from "@/lib/platform/clickhouse/helpers";
import asaw from "@/utils/asaw";

export async function POST() {
	const [err, res] = await asaw(runClickhouseMigrations());
	if (err)
		return Response.json(err, {
			status: 400,
		});

	return Response.json(res);
}
