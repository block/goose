-- CreateTable
CREATE TABLE "APIKeys" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "name" TEXT NOT NULL DEFAULT 'default',
    "apiKey" TEXT NOT NULL,
    "database_config_id" TEXT NOT NULL,
    "isDeleted" BOOLEAN NOT NULL DEFAULT false,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "createdByUserId" TEXT NOT NULL,
    "deletedAt" DATETIME,
    "deletedByUserId" TEXT,
    CONSTRAINT "APIKeys_database_config_id_fkey" FOREIGN KEY ("database_config_id") REFERENCES "databaseconfig" ("id") ON DELETE RESTRICT ON UPDATE CASCADE,
    CONSTRAINT "APIKeys_createdByUserId_fkey" FOREIGN KEY ("createdByUserId") REFERENCES "users" ("id") ON DELETE RESTRICT ON UPDATE CASCADE,
    CONSTRAINT "APIKeys_deletedByUserId_fkey" FOREIGN KEY ("deletedByUserId") REFERENCES "users" ("id") ON DELETE SET NULL ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "ClickhouseMigrations" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "databaseConfigId" TEXT NOT NULL,
    "clickhouseMigrationId" TEXT NOT NULL
);

-- CreateIndex
CREATE UNIQUE INDEX "APIKeys_apiKey_key" ON "APIKeys"("apiKey");
