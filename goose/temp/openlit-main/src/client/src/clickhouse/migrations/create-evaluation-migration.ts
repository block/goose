import { OPENLIT_EVALUATION_TABLE_NAME } from "@/lib/platform/evaluation/table-details";
import migrationHelper from "./migration-helper";

const MIGRATION_ID = "create-evaluation-table-2";

export default async function CreateEvaluationMigration(
	databaseConfigId?: string
) {
	const queries = [
		`
      CREATE TABLE IF NOT EXISTS ${OPENLIT_EVALUATION_TABLE_NAME} (
          id UUID DEFAULT generateUUIDv4(),  -- Unique ID for each evaluation
          span_id String,
          created_at DateTime DEFAULT now(),
          meta Map(LowCardinality(String), String),

          -- Fixed metadata structure
          evaluationData Nested(
              evaluation LowCardinality(String),
              classification LowCardinality(String),
              explanation String,
              verdict LowCardinality(String)
          ),

          -- Dynamic evaluation scores
          scores Map(LowCardinality(String), Float32)  
      ) ENGINE = MergeTree()
      ORDER BY (span_id, created_at);
    `,
	];

	return migrationHelper({
		clickhouseMigrationId: MIGRATION_ID,
		databaseConfigId,
		queries,
	});
}
