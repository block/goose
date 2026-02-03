import { NextRequest, NextResponse } from "next/server";
import { getCurrentUser } from "@/lib/session";
import { getDBConfigByUser } from "@/lib/db-config";
import asaw from "@/utils/asaw";
import {
	migrateOpengroundDataToClickhouse,
	checkMigrationNeeded,
} from "@/lib/platform/openground-clickhouse/migrate-data";
import getMessage from "@/constants/messages";

/**
 * POST /api/openground/migrate
 * Migrates existing Prisma OpenGround data to ClickHouse
 */
export async function POST(request: NextRequest) {
	try {
		const user = await getCurrentUser();
		if (!user) {
			return NextResponse.json(
				{ error: getMessage().UNAUTHORIZED_USER },
				{ status: 401 }
			);
		}

		const [, dbConfig] = await asaw(getDBConfigByUser(true));
		if (!dbConfig) {
			return NextResponse.json(
				{ error: getMessage().DATABASE_CONFIG_NOT_FOUND },
				{ status: 404 }
			);
		}

		// Run the migration
		const result = await migrateOpengroundDataToClickhouse(dbConfig.id);

		if (result.err) {
			return NextResponse.json(
				{ error: result.err, details: result.details },
				{ status: 500 }
			);
		}

		return NextResponse.json({
			message: "Migration completed successfully",
			result: result.data,
		});
	} catch (error: any) {
		console.error("Migration API error:", error);
		return NextResponse.json(
			{
				error: getMessage().OPENGROUND_DATA_MIGRATION_FAILED,
				details: error.message || String(error),
			},
			{ status: 500 }
		);
	}
}

/**
 * GET /api/openground/migrate
 * Checks if migration is needed
 */
export async function GET(request: NextRequest) {
	try {
		const user = await getCurrentUser();
		if (!user) {
			return NextResponse.json(
				{ error: getMessage().UNAUTHORIZED_USER },
				{ status: 401 }
			);
		}

		const [, dbConfig] = await asaw(getDBConfigByUser(true));
		if (!dbConfig) {
			return NextResponse.json(
				{ error: getMessage().DATABASE_CONFIG_NOT_FOUND },
				{ status: 404 }
			);
		}

		const needsMigration = await checkMigrationNeeded(dbConfig.id);

		return NextResponse.json({
			needsMigration,
			databaseConfigId: dbConfig.id,
		});
	} catch (error: any) {
		console.error("Migration check API error:", error);
		return NextResponse.json(
			{
				error: "Failed to check migration status",
				details: error.message || String(error),
			},
			{ status: 500 }
		);
	}
}
