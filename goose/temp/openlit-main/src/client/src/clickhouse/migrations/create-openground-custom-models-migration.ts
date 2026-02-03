import { dataCollector } from "@/lib/platform/common";
import getMessage from "@/constants/messages";
import prisma from "@/lib/prisma";
import asaw from "@/utils/asaw";
import { getDBConfigById, getDBConfigByUser } from "@/lib/db-config";

export const OPENLIT_OPENGROUND_CUSTOM_MODELS_TABLE_NAME = "openlit_openground_custom_models";

export default async function CreateOpengroundCustomModelsMigration(databaseConfigId?: string) {
	const [, dbConfig] = await asaw(
		databaseConfigId
			? getDBConfigById({ id: databaseConfigId })
			: getDBConfigByUser(true)
	);

	if (!dbConfig?.id) return { err: getMessage().DATABASE_CONFIG_NOT_FOUND };

	// Check if migration already ran
	const [, migrationExist] = await asaw(
		prisma.clickhouseMigrations.findFirst({
			where: {
				AND: {
					databaseConfigId: dbConfig.id,
					clickhouseMigrationId: "create-openground-custom-models-table",
				},
			},
		})
	);

	if (migrationExist?.id) {
		return { migrationExist: true };
	}

	// Create custom models table
	const customModelsTableQuery = `
		CREATE TABLE IF NOT EXISTS ${OPENLIT_OPENGROUND_CUSTOM_MODELS_TABLE_NAME} (
			id UUID DEFAULT generateUUIDv4(),
			provider String,                        -- Provider ID (e.g., 'openai', 'anthropic')
			model_id String,                        -- Model identifier
			display_name String,                    -- Human-readable name
			context_window UInt32 DEFAULT 4096,     -- Context window size
			input_price_per_m_token Float64 DEFAULT 0,  -- Input price per million tokens
			output_price_per_m_token Float64 DEFAULT 0, -- Output price per million tokens
			capabilities Array(String) DEFAULT [],  -- Array of capabilities

			created_by_user_id String,              -- User who created
			database_config_id String,              -- Multi-tenancy
			created_at DateTime DEFAULT now(),
			updated_at DateTime DEFAULT now()
		) ENGINE = MergeTree()
		PRIMARY KEY (database_config_id, created_by_user_id, provider, id)
		ORDER BY (database_config_id, created_by_user_id, provider, id, created_at);
	`;

	const { err } = await dataCollector(
		{ query: customModelsTableQuery },
		"exec",
		dbConfig.id
	);

	if (err) {
		console.error("Migration error:", err);
		return { err: getMessage().OPERATION_FAILED };
	}

	// Record migration success
	await prisma.clickhouseMigrations.create({
		data: {
			databaseConfigId: dbConfig.id,
			clickhouseMigrationId: "create-openground-custom-models-table",
		},
	});

	return { data: "Migration successful" };
}
