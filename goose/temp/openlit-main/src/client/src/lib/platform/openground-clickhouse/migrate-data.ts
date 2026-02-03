import prisma from "@/lib/prisma";
import { dataCollector } from "@/lib/platform/common";
import getMessage from "@/constants/messages";
import Sanitizer from "@/utils/sanitizer";
import {
	OPENLIT_OPENGROUND_TABLE_NAME,
	OPENLIT_OPENGROUND_PROVIDERS_TABLE_NAME,
} from "@/lib/platform/openground/table-details";

interface PrismaOpenGroundRecord {
	id: string;
	requestMeta: string; // JSON
	responseMeta: string; // JSON
	stats: string; // JSON
	createdAt: Date;
	createdByUserId: string;
	databaseConfigId: string;
}

interface RequestMeta {
	prompt: string;
	selectedProviders: {
		provider: string;
		config: any;
	}[];
}

interface ResponseMeta {
	evaluationData?: {
		cost: number;
		responseTime: number;
		completionTokens: number;
		promptTokens: number;
		totalTokens?: number;
		model: string;
		prompt: string;
		response: string;
		finishReason?: string;
	};
	[key: string]: any;
}

interface Stats {
	prompt: string;
	errors: string[];
	totalProviders: number;
	minCostProvider?: string;
	minCost?: number;
	minResponseTimeProvider?: string;
	minResponseTime?: number;
	minCompletionTokensProvider?: string;
	minCompletionTokens?: number;
}

/**
 * Migrate all existing Prisma OpenGround data to ClickHouse
 * This should be run once after the ClickHouse tables are created
 */
export async function migrateOpengroundDataToClickhouse(
	databaseConfigId: string
) {
	console.log("Starting OpenGround data migration to ClickHouse...");

	try {
		// Fetch all OpenGround records from Prisma
		const prismaRecords: PrismaOpenGroundRecord[] =
			await prisma.openGround.findMany({
				where: {
					databaseConfigId,
				},
				orderBy: {
					createdAt: "asc",
				},
			});

		if (prismaRecords.length === 0) {
			console.log("No OpenGround records to migrate.");
			return { data: "No records to migrate" };
		}

		console.log('Found', prismaRecords.length, 'records to migrate.');

		let successCount = 0;
		let errorCount = 0;
		const errors: string[] = [];

		// Process each record
		for (const record of prismaRecords) {
			try {
				// Parse JSON strings
				const requestMeta: RequestMeta = JSON.parse(record.requestMeta);
				const responseMeta: [string | null, ResponseMeta | null][] = JSON.parse(
					record.responseMeta
				);
				const stats: Stats = JSON.parse(record.stats);

				// Escape single quotes in string values for SQL
				const escapeString = (str: string) => str.replace(/'/g, "''");
				const promptEscaped = escapeString(requestMeta.prompt);
				const createdAt = record.createdAt.toISOString().replace("T", " ").slice(0, 19);

				// Format errors array for ClickHouse
				const errorsArray = stats.errors || [];
				const errorsClickhouse = errorsArray.length > 0
					? `[${errorsArray.map(err => `'${escapeString(err)}'`).join(', ')}]`
					: '[]';

				// Insert main record using raw query for better control
				const insertQuery = `
					INSERT INTO ${OPENLIT_OPENGROUND_TABLE_NAME} (
						prompt,
						prompt_source,
						prompt_hub_id,
						prompt_hub_version,
						prompt_variables,
						created_by_user_id,
						database_config_id,
						created_at,
						total_providers,
						min_cost,
						min_cost_provider,
						min_response_time,
						min_response_time_provider,
						min_completion_tokens,
						min_completion_tokens_provider,
						errors
					) VALUES (
				'${promptEscaped}',
				'custom',
				NULL,
				NULL,
				'{}',
				'${escapeString(record.createdByUserId)}',
				'${escapeString(databaseConfigId)}',
				parseDateTimeBestEffort('${createdAt}'),
						${stats.totalProviders || responseMeta.length},
						${stats.minCost || 0},
						'${escapeString(stats.minCostProvider || "")}',
						${stats.minResponseTime || 0},
						'${escapeString(stats.minResponseTimeProvider || "")}',
						${stats.minCompletionTokens || 0},
						'${escapeString(stats.minCompletionTokensProvider || "")}',
						${errorsClickhouse}
					)
				`;

				const { err: mainErr } = await dataCollector(
					{
						query: insertQuery,
					},
					"exec",
					databaseConfigId
				);

			if (mainErr) {
				// Use separate parameters to prevent log injection
				console.error('Error migrating record:', record.id, mainErr);
				errorCount++;
				errors.push(`Record ${record.id}: ${mainErr}`);
				continue;
			}

				// Get the inserted record ID
				const { data: insertedData, err: fetchErr } = await dataCollector(
					{
						query: `SELECT id FROM ${OPENLIT_OPENGROUND_TABLE_NAME}
								WHERE created_by_user_id = '${record.createdByUserId}'
								AND database_config_id = '${databaseConfigId}'
								AND created_at = parseDateTimeBestEffort('${record.createdAt.toISOString()}')
								ORDER BY created_at DESC
								LIMIT 1`,
					},
					"query",
					databaseConfigId
				);

			if (fetchErr || !(insertedData as any[])?.[0]?.id) {
				// Use separate parameters to prevent log injection
				console.error('Error fetching inserted ID for record:', record.id, fetchErr);
				errorCount++;
				errors.push(`Record ${record.id}: Could not fetch inserted ID`);
				continue;
			}

				const insertedId = (insertedData as any[])[0].id;

				// Insert provider results using raw queries
				for (let index = 0; index < requestMeta.selectedProviders.length; index++) {
					const provider = requestMeta.selectedProviders[index];
					const [error, response] = responseMeta[index] || [null, null];
					const evaluationData = response?.evaluationData;

					const providerName = escapeString(provider.provider);
					const model = escapeString(evaluationData?.model || provider.config?.model || "unknown");
					const config = JSON.stringify(provider.config || {}).replace(/'/g, "''");
					const responseText = escapeString(evaluationData?.response || "");
					const errorText = escapeString(error || "");
					const cost = evaluationData?.cost || 0;
					const promptTokens = evaluationData?.promptTokens || 0;
					const completionTokens = evaluationData?.completionTokens || 0;
					const totalTokens = evaluationData?.totalTokens || (promptTokens + completionTokens);
					const responseTime = evaluationData?.responseTime || 0;
					const finishReason = escapeString(evaluationData?.finishReason || "");
					const providerResponse = JSON.stringify(response || {}).replace(/'/g, "''");

					const providerInsertQuery = `
						INSERT INTO ${OPENLIT_OPENGROUND_PROVIDERS_TABLE_NAME} (
							openground_id,
							provider,
							model,
							config,
							response,
							error,
							cost,
							prompt_tokens,
							completion_tokens,
							total_tokens,
							response_time,
							finish_reason,
							provider_response,
							created_at
					) VALUES (
						'${escapeString(insertedId)}',
						'${providerName}',
						'${model}',
						'${config}',
						'${responseText}',
						'${errorText}',
							${cost},
							${promptTokens},
							${completionTokens},
							${totalTokens},
							${responseTime},
							'${finishReason}',
							'${providerResponse}',
							parseDateTimeBestEffort('${createdAt}')
						)
					`;

					const { err: providerErr } = await dataCollector(
						{
							query: providerInsertQuery,
						},
						"exec",
						databaseConfigId
					);

					if (providerErr) {
						console.error(
							`Error migrating provider result ${index} for ${record.id}:`,
							providerErr
						);
						// Don't increment errorCount or continue - just log and move to next provider
					}
				}

			successCount++;
			// Use separate parameters to prevent log injection
			console.log(
				'✓ Migrated record', record.id, `(${successCount}/${prismaRecords.length})`
			);
		} catch (recordError: any) {
			// Use separate parameters to prevent log injection
			console.error('Error processing record:', record.id, recordError);
			errorCount++;
			errors.push(
				`Record ${record.id}: ${recordError.message || String(recordError)}`
			);
		}
		}

	console.log("\nMigration complete!");
	console.log('Success:', `${successCount}/${prismaRecords.length}`);
	console.log('Errors:', `${errorCount}/${prismaRecords.length}`);

	if (errors.length > 0) {
		console.error("\nErrors encountered:");
		// Use separate parameters to prevent log injection
		errors.forEach((err) => console.error('  -', err));
	}

		return {
			data: {
				total: prismaRecords.length,
				success: successCount,
				errors: errorCount,
				errorDetails: errors,
			},
		};
	} catch (error: any) {
		console.error("Migration failed:", error);
		return {
			err: getMessage().OPENGROUND_DATA_MIGRATION_FAILED,
			details: error.message || String(error),
		};
	}
}

/**
 * Check if migration is needed (if there are Prisma records but no ClickHouse records)
 */
export async function checkMigrationNeeded(
	databaseConfigId: string
): Promise<boolean> {
	try {
		// Check Prisma count
		const prismaCount = await prisma.openGround.count({
			where: {
				databaseConfigId,
			},
		});

		if (prismaCount === 0) {
			return false; // No data to migrate
		}

	// Check ClickHouse count
	// Sanitize databaseConfigId to prevent SQL injection
	const sanitizedDbConfigId = Sanitizer.sanitizeValue(databaseConfigId);
	const { data: clickhouseData } = await dataCollector(
		{
			query: `SELECT COUNT(*) as count FROM ${OPENLIT_OPENGROUND_TABLE_NAME} WHERE database_config_id = '${sanitizedDbConfigId}'`,
		},
		"query",
		databaseConfigId
	);

		const clickhouseCount = (clickhouseData as any[])?.[0]?.count || 0;

		// Migration needed if Prisma has records but ClickHouse is empty or has fewer records
		return prismaCount > clickhouseCount;
	} catch (error) {
		console.error("Error checking migration status:", error);
		return false;
	}
}
