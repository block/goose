import { dataCollector } from "@/lib/platform/common";
import getMessage from "@/constants/messages";
import prisma from "@/lib/prisma";
import asaw from "@/utils/asaw";
import { getDBConfigByUser, getDBConfigById } from "@/lib/db-config";
import {
	OPENLIT_OPENGROUND_TABLE_NAME,
	OPENLIT_OPENGROUND_PROVIDERS_TABLE_NAME,
	OPENLIT_OPENGROUND_CONFIG_TABLE_NAME,
} from "@/lib/platform/openground/table-details";

export default async function CreateOpengroundMigration(databaseConfigId?: string) {
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
					clickhouseMigrationId: "create-openground-table",
				},
			},
		})
	);

	if (migrationExist?.id) {
		return { migrationExist: true };
	}

	// Create main OpenGround table
	const opengroundTableQuery = `
		CREATE TABLE IF NOT EXISTS ${OPENLIT_OPENGROUND_TABLE_NAME} (
			id UUID DEFAULT generateUUIDv4(),
			prompt String,                          -- User prompt
			prompt_source String DEFAULT 'custom',  -- 'custom' or 'prompt-hub'
			prompt_hub_id Nullable(UUID),          -- FK to prompt hub if used
			prompt_hub_version Nullable(String),   -- Version if from hub
			prompt_variables String DEFAULT '{}',   -- JSON: variable substitutions

			created_by_user_id String,             -- User who created
			database_config_id String,             -- Multi-tenancy
			created_at DateTime DEFAULT now(),

			-- Computed statistics
			total_providers UInt8,                 -- Number of providers evaluated
			min_cost Float64,                      -- Minimum cost across providers
			min_cost_provider String DEFAULT '',   -- Provider with min cost
			min_response_time Float64,             -- Minimum response time
			min_response_time_provider String DEFAULT '', -- Provider with min time
			min_completion_tokens UInt32,          -- Minimum tokens
			min_completion_tokens_provider String DEFAULT '', -- Provider with min tokens
			errors Array(String),                  -- Array of error messages

			-- Indexes for fast queries
			INDEX user_index (created_by_user_id) TYPE bloom_filter GRANULARITY 1,
			INDEX id_index (id) TYPE bloom_filter GRANULARITY 1,
			INDEX created_at_index (created_at) TYPE minmax GRANULARITY 1,
			INDEX prompt_source_index (prompt_source) TYPE bloom_filter GRANULARITY 1
		) ENGINE = MergeTree()
		ORDER BY (database_config_id, created_at, created_by_user_id);
	`;

	// Create provider results table (1:N relationship)
	const providerResultsQuery = `
		CREATE TABLE IF NOT EXISTS ${OPENLIT_OPENGROUND_PROVIDERS_TABLE_NAME} (
			id UUID DEFAULT generateUUIDv4(),
			openground_id UUID,                    -- FK to openground table

			-- Provider info
			provider String,                        -- 'openai', 'anthropic', etc.
			model String,                           -- Model name

			-- Request configuration
			config String DEFAULT '{}',             -- JSON: temperature, maxTokens, etc.

			-- Response data
			response String DEFAULT '',             -- LLM response text
			error String DEFAULT '',                -- Error message if failed

			-- Metrics
			cost Float64 DEFAULT 0,                 -- Cost in USD
			prompt_tokens UInt32 DEFAULT 0,         -- Prompt tokens used
			completion_tokens UInt32 DEFAULT 0,     -- Completion tokens used
			total_tokens UInt32 DEFAULT 0,          -- Total tokens
			response_time Float64 DEFAULT 0,        -- Response time in seconds
			finish_reason String DEFAULT '',        -- 'stop', 'length', etc.

			-- Full provider response
			provider_response String DEFAULT '{}',  -- JSON: full API response

			created_at DateTime DEFAULT now(),

			-- Indexes
			INDEX id_index (id) TYPE bloom_filter GRANULARITY 1,
			INDEX provider_index (provider) TYPE bloom_filter GRANULARITY 1,
			INDEX model_index (model) TYPE bloom_filter GRANULARITY 1
		) ENGINE = MergeTree()
		ORDER BY (openground_id, provider, created_at);
	`;

	// Create configuration table for provider settings
	const configTableQuery = `
		CREATE TABLE IF NOT EXISTS ${OPENLIT_OPENGROUND_CONFIG_TABLE_NAME} (
			id UUID DEFAULT generateUUIDv4(),
			user_id String,                        -- User who owns this config
			database_config_id String,             -- Multi-tenancy
			provider String,                        -- 'openai', 'anthropic', etc.
			vault_id String,                        -- FK to vault secret (API key)
			model_id Nullable(String),             -- Default model (optional)
			is_active Boolean DEFAULT true,        -- Whether config is active
			created_at DateTime DEFAULT now(),
			updated_at DateTime DEFAULT now(),

			-- Indexes for fast queries
			INDEX id_index (id) TYPE bloom_filter GRANULARITY 1,
			INDEX provider_index (provider) TYPE bloom_filter GRANULARITY 1,
			INDEX active_index (is_active) TYPE bloom_filter GRANULARITY 1
		) ENGINE = MergeTree()
		ORDER BY (user_id, database_config_id, provider);
	`;

	const queries = [opengroundTableQuery, providerResultsQuery, configTableQuery];

	const queryResponses = await Promise.all(
		queries.map(async (query) => await dataCollector({ query }, "exec", dbConfig.id))
	);

	const errors = queryResponses.filter((res) => res.err);
	if (errors.length > 0) {
		console.error("Migration errors:", errors);
		return { err: getMessage().OPENGROUND_MIGRATION_FAILED };
	}

	// Record migration success
	await prisma.clickhouseMigrations.create({
		data: {
			databaseConfigId: dbConfig.id,
			clickhouseMigrationId: "create-openground-table",
		},
	});

	// Automatically migrate existing Prisma data to ClickHouse
	console.log("ClickHouse tables created. Checking for existing data to migrate...");
	try {
		// Import the migration function dynamically to avoid circular dependencies
		const { migrateOpengroundDataToClickhouse } = await import(
			"@/lib/platform/openground-clickhouse/migrate-data"
		);

		const migrationResult = await migrateOpengroundDataToClickhouse(dbConfig.id);

		if (migrationResult.data) {
			console.log("Data migration completed:", migrationResult.data);
		} else if (migrationResult.err) {
			console.warn("Data migration warning:", migrationResult.err);
		}
	} catch (migrationError) {
		console.warn("Could not auto-migrate data:", migrationError);
		console.log("You can manually trigger migration via POST /api/openground/migrate");
	}

	return { data: "Migration successful" };
}
