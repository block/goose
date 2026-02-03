-- CreateTable
CREATE TABLE "openground" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "requestMeta" TEXT NOT NULL,
    "responseMeta" TEXT NOT NULL,
    "stats" TEXT NOT NULL,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "user_id" TEXT NOT NULL,
    "database_config_id" TEXT NOT NULL,
    CONSTRAINT "openground_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "users" ("id") ON DELETE RESTRICT ON UPDATE CASCADE,
    CONSTRAINT "openground_database_config_id_fkey" FOREIGN KEY ("database_config_id") REFERENCES "databaseconfig" ("id") ON DELETE RESTRICT ON UPDATE CASCADE
);
