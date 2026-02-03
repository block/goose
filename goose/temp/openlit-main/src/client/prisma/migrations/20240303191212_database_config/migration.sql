-- CreateTable
CREATE TABLE "databaseconfig" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "name" TEXT NOT NULL,
    "environment" TEXT NOT NULL DEFAULT 'production',
    "username" TEXT NOT NULL DEFAULT 'admin',
    "password" TEXT,
    "host" TEXT NOT NULL DEFAULT '127.0.0.1',
    "port" TEXT NOT NULL DEFAULT '8123',
    "database" TEXT NOT NULL DEFAULT 'openlit',
    "query" TEXT,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" DATETIME NOT NULL,
    "user_id" TEXT NOT NULL,
    CONSTRAINT "databaseconfig_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "users" ("id") ON DELETE RESTRICT ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "databaseconfiguser" (
    "databaseConfigId" TEXT NOT NULL,
    "user_id" TEXT NOT NULL,
    "isCurrent" BOOLEAN NOT NULL DEFAULT false,
    CONSTRAINT "databaseconfiguser_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "users" ("id") ON DELETE RESTRICT ON UPDATE CASCADE,
    CONSTRAINT "databaseconfiguser_databaseConfigId_fkey" FOREIGN KEY ("databaseConfigId") REFERENCES "databaseconfig" ("id") ON DELETE RESTRICT ON UPDATE CASCADE
);

-- CreateIndex
CREATE UNIQUE INDEX "databaseconfig_name_key" ON "databaseconfig"("name");

-- CreateIndex
CREATE UNIQUE INDEX "databaseconfiguser_databaseConfigId_user_id_key" ON "databaseconfiguser"("databaseConfigId", "user_id");
