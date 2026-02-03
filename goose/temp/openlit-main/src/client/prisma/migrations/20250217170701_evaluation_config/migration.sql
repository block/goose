-- CreateTable
CREATE TABLE "EvaluationConfigs" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "database_config_id" TEXT NOT NULL,
    "provider" TEXT NOT NULL,
    "model" TEXT NOT NULL,
    "vaultId" TEXT NOT NULL,
    "auto" BOOLEAN NOT NULL DEFAULT false,
    "recurringTime" TEXT NOT NULL,
    "meta" TEXT NOT NULL
);

-- CreateIndex
CREATE UNIQUE INDEX "EvaluationConfigs_database_config_id_key" ON "EvaluationConfigs"("database_config_id");
