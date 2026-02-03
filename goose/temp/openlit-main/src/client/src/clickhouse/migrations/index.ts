import CreateEvaluationMigration from "./create-evaluation-migration";
import CreateCronLogMigration from "./create-cron-log-migration";
import CreatePromptMigration from "./create-prompt-migration";
import CreateVaultMigration from "./create-vault-migration";
import CreateCustomDashboardsMigration from "./create-custom-dashboards-migration";
import CreateOpengroundMigration from "./create-openground-migration";
import CreateOpengroundCustomModelsMigration from "./create-openground-custom-models-migration";

export default async function migrations(databaseConfigId?: string) {
	return Promise.all([
		CreatePromptMigration(databaseConfigId),
		CreateVaultMigration(databaseConfigId),
		CreateEvaluationMigration(databaseConfigId),
		CreateCronLogMigration(databaseConfigId),
		CreateCustomDashboardsMigration(databaseConfigId),
		CreateOpengroundMigration(databaseConfigId),
		CreateOpengroundCustomModelsMigration(databaseConfigId),
	]);
}
