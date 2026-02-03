-- RedefineTables
PRAGMA foreign_keys=OFF;
CREATE TABLE "new_databaseconfig" (
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
INSERT INTO "new_databaseconfig" ("createdAt", "database", "environment", "host", "id", "name", "password", "port", "query", "updatedAt", "user_id", "username") SELECT "createdAt", "database", "environment", "host", "id", "name", "password", "port", "query", "updatedAt", "user_id", "username" FROM "databaseconfig";
DROP TABLE "databaseconfig";
ALTER TABLE "new_databaseconfig" RENAME TO "databaseconfig";
CREATE UNIQUE INDEX "databaseconfig_name_key" ON "databaseconfig"("name");
PRAGMA foreign_key_check;
PRAGMA foreign_keys=ON;
