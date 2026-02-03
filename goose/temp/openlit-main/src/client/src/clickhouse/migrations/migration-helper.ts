import getMessage from "@/constants/messages";
import { getDBConfigById, getDBConfigByUser } from "@/lib/db-config";
import { dataCollector } from "@/lib/platform/common";
import prisma from "@/lib/prisma";
import asaw from "@/utils/asaw";
import { consoleLog } from "@/utils/log";

export default async function migrationHelper({
	clickhouseMigrationId,
	queries,
	databaseConfigId,
}: {
	clickhouseMigrationId: string;
	queries: string[];
	databaseConfigId?: string;
}) {
	let err, dbConfig;
	if (databaseConfigId) {
		[err, dbConfig] = await asaw(getDBConfigById({ id: databaseConfigId }));
	} else {
		[err, dbConfig] = await asaw(getDBConfigByUser(true));
	}

	if (err || !dbConfig?.id) throw err || getMessage().DATABASE_CONFIG_NOT_FOUND;

	const [, migrationExist] = await asaw(
		prisma.clickhouseMigrations.findFirst({
			where: {
				AND: {
					databaseConfigId: dbConfig.id as string,
					clickhouseMigrationId,
				},
			},
		})
	);

	if (migrationExist?.id) {
		return { migrationExist: true, queriesRun: false };
	}

	const queriesRun = await Promise.all(
		queries.map(async (query) => {
			const { err } = await dataCollector({ query });
			if (err) {
				console.log(
					`********* Migration Error : ${clickhouseMigrationId} *********`
				);
				consoleLog(err);
				console.log(
					`********* Migration Error : ${clickhouseMigrationId} *********`
				);
			}

			return { err };
		})
	);

	if (queriesRun.filter(({ err }) => !err).length === queries.length) {
		await asaw(
			prisma.clickhouseMigrations.create({
				data: {
					databaseConfigId: dbConfig.id,
					clickhouseMigrationId,
				},
			})
		);

		return { migrationExist: false, queriesRun: true };
	}

	return { migrationExist: false, queriesRun: false };
}
