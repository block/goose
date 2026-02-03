import migrations from "@/clickhouse/migrations";
import { dataCollector } from "../common";
import asaw from "@/utils/asaw";

export async function pingClickhouse() {
	const pingResponse = await dataCollector({}, "ping");
	if (pingResponse.data) {
		const [err] = await asaw(runClickhouseMigrations());
		return {
			err,
			data: !err,
		};
	}

	return pingResponse;
}

export async function runClickhouseMigrations() {
	try {
		await migrations();
	} catch (error) {
		console.error("Error running migrations:", error);
	}
}
