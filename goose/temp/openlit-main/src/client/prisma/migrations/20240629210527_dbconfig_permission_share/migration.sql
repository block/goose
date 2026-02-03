-- CreateTable
CREATE TABLE "DatabaseConfigInvitedUser" (
    "databaseConfigId" TEXT NOT NULL,
    "email" TEXT NOT NULL,
    "canEdit" BOOLEAN NOT NULL DEFAULT false,
    "canShare" BOOLEAN NOT NULL DEFAULT false,
    "canDelete" BOOLEAN NOT NULL DEFAULT false,
    CONSTRAINT "DatabaseConfigInvitedUser_databaseConfigId_fkey" FOREIGN KEY ("databaseConfigId") REFERENCES "databaseconfig" ("id") ON DELETE RESTRICT ON UPDATE CASCADE
);

-- RedefineTables
PRAGMA defer_foreign_keys=ON;
PRAGMA foreign_keys=OFF;
CREATE TABLE "new_databaseconfiguser" (
    "databaseConfigId" TEXT NOT NULL,
    "user_id" TEXT NOT NULL,
    "isCurrent" BOOLEAN NOT NULL DEFAULT false,
    "canEdit" BOOLEAN NOT NULL DEFAULT false,
    "canShare" BOOLEAN NOT NULL DEFAULT false,
    "canDelete" BOOLEAN NOT NULL DEFAULT false,
    "createdAt" DATETIME DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" DATETIME,
    CONSTRAINT "databaseconfiguser_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "users" ("id") ON DELETE RESTRICT ON UPDATE CASCADE,
    CONSTRAINT "databaseconfiguser_databaseConfigId_fkey" FOREIGN KEY ("databaseConfigId") REFERENCES "databaseconfig" ("id") ON DELETE RESTRICT ON UPDATE CASCADE
);
INSERT INTO "new_databaseconfiguser" ("databaseConfigId", "isCurrent", "user_id") SELECT "databaseConfigId", "isCurrent", "user_id" FROM "databaseconfiguser";
DROP TABLE "databaseconfiguser";
ALTER TABLE "new_databaseconfiguser" RENAME TO "databaseconfiguser";
CREATE UNIQUE INDEX "databaseconfiguser_databaseConfigId_user_id_key" ON "databaseconfiguser"("databaseConfigId", "user_id");
PRAGMA foreign_keys=ON;
PRAGMA defer_foreign_keys=OFF;

-- CreateIndex
CREATE UNIQUE INDEX "DatabaseConfigInvitedUser_databaseConfigId_email_key" ON "DatabaseConfigInvitedUser"("databaseConfigId", "email");


-- This migration is for updating the permissions to true for the user who created the db config
BEGIN TRANSACTION;

-- Update canEdit, canShare, and canDelete fields for users who created the DatabaseConfig
UPDATE DatabaseConfigUser
SET 
    canEdit = TRUE,
    canShare = TRUE,
    canDelete = TRUE
WHERE 
    DatabaseConfigUser.user_id IN (SELECT DatabaseConfig.user_id FROM DatabaseConfig WHERE DatabaseConfig.id = DatabaseConfigUser.databaseConfigId);

COMMIT;