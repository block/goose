import { OPENLIT_CRON_LOG_TABLE_NAME } from "@/lib/platform/cron-log/table-details";
import migrationHelper from "./migration-helper";

const MIGRATION_ID = "create-cron-log-table";

export default async function CreateCronLogMigration(
	databaseConfigId?: string
) {
	const queries = [
		`
      CREATE TABLE IF NOT EXISTS ${OPENLIT_CRON_LOG_TABLE_NAME}
      (
          id UUID DEFAULT generateUUIDv4(),  -- Unique identifier for each cron job run
          cron_id String,  -- Unique identifier for cron job config id
          cron_type String,  -- Type of cron job
          run_status String,  -- Status of the cron execution
          meta Map(LowCardinality(String), String),  -- JSON to store additional metadata
          error_stacktrace Map(LowCardinality(String), String),  -- Stacktrace in case of failure
          started_at DateTime DEFAULT now(),  -- Start time of execution
          finished_at DateTime,  -- End time of execution
          duration Float64  -- Execution duration in seconds
      ) 
      ENGINE = MergeTree()
      ORDER BY (id, cron_id, started_at);
    `,
	];

	return migrationHelper({
		clickhouseMigrationId: MIGRATION_ID,
		databaseConfigId,
		queries,
	});
}
