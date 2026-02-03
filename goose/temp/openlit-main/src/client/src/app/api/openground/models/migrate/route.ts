import { NextRequest, NextResponse } from "next/server";
import { getServerSession } from "next-auth";
import { authOptions } from "@/app/auth";
import { getDBConfigByUser } from "@/lib/db-config";
import getMessage from "@/constants/messages";
import asaw from "@/utils/asaw";
import CreateOpengroundCustomModelsMigration from "@/clickhouse/migrations/create-openground-custom-models-migration";

// Extend the session type to include id
interface SessionWithId {
	user?: {
		id?: string;
		name?: string | null;
		email?: string | null;
		image?: string | null;
	};
}

// POST: Manually trigger the custom models table migration
export async function POST(request: NextRequest) {
	const session = (await getServerSession(authOptions)) as SessionWithId;

	if (!session?.user?.id) {
		return NextResponse.json(
			{ error: getMessage().UNAUTHORIZED_USER },
			{ status: 401 }
		);
	}

	const [, dbConfig] = await asaw(getDBConfigByUser(true));
	if (!dbConfig?.id) {
		return NextResponse.json(
			{ error: getMessage().DATABASE_CONFIG_NOT_FOUND },
			{ status: 500 }
		);
	}

	// Run the migration
	const result = await CreateOpengroundCustomModelsMigration(dbConfig.id);

	if (result.err) {
		console.error("Migration error:", result.err);
		return NextResponse.json(
			{ error: result.err },
			{ status: 500 }
		);
	}

	if (result.migrationExist) {
		return NextResponse.json({
			message: "Migration already exists",
			alreadyRan: true,
		});
	}

	return NextResponse.json({
		message: "Migration completed successfully",
		success: true,
		data: result.data,
	});
}
