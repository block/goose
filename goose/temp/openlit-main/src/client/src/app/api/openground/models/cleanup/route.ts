import { NextRequest, NextResponse } from "next/server";
import { getServerSession } from "next-auth";
import { authOptions } from "@/app/auth";
import { dataCollector } from "@/lib/platform/common";
import { getDBConfigByUser } from "@/lib/db-config";
import getMessage from "@/constants/messages";
import { OPENLIT_OPENGROUND_CUSTOM_MODELS_TABLE_NAME } from "@/lib/platform/openground/table-details";
import asaw from "@/utils/asaw";
import Sanitizer from "@/utils/sanitizer";

// Extend the session type to include id
interface SessionWithId {
	user?: {
		id?: string;
		name?: string | null;
		email?: string | null;
		image?: string | null;
	};
}

// POST: Clean up models with invalid UUIDs
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

	// First, get all models to see which ones have invalid UUIDs
	const selectQuery = `
		SELECT toString(id) as id, model_id, display_name
		FROM ${OPENLIT_OPENGROUND_CUSTOM_MODELS_TABLE_NAME}
		WHERE created_by_user_id = '${session.user.id}'
		  AND database_config_id = '${dbConfig.id}'
	`;

	const { data: allModels, err: selectErr } = await dataCollector(
		{ query: selectQuery },
		"query",
		dbConfig.id
	);

	if (selectErr) {
		console.error("Error fetching models:", selectErr);
		return NextResponse.json(
			{ error: selectErr || getMessage().OPERATION_FAILED },
			{ status: 500 }
		);
	}

	// Identify invalid UUIDs
	const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
	const invalidModels = (allModels as any[] || []).filter(
		(model: any) => !model.id || !uuidRegex.test(model.id)
	);

	if (invalidModels.length === 0) {
		return NextResponse.json({
			message: "No invalid models found",
			cleaned: 0,
		});
	}

	// Use separate parameters to prevent log injection
	console.log('Found', invalidModels.length, 'models with invalid UUIDs:', invalidModels);

	// Sanitize user inputs to prevent SQL injection
	const sanitizedUserId = Sanitizer.sanitizeValue(session.user.id);
	const sanitizedDbConfigId = Sanitizer.sanitizeValue(dbConfig.id);

	// Delete all rows with invalid UUIDs using a simpler approach
	// We'll delete all and rely on the user to recreate valid ones
	const deleteQuery = `
		ALTER TABLE ${OPENLIT_OPENGROUND_CUSTOM_MODELS_TABLE_NAME}
		DELETE WHERE created_by_user_id = '${sanitizedUserId}'
		  AND database_config_id = '${sanitizedDbConfigId}'
		  AND NOT match(toString(id), '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$')
	`;

	console.log("Cleanup query:", deleteQuery);

	const { err: deleteErr } = await dataCollector(
		{ query: deleteQuery },
		"exec",
		dbConfig.id
	);

	if (deleteErr) {
		console.error("Error cleaning up models:", deleteErr);
		return NextResponse.json(
			{ error: deleteErr || getMessage().OPERATION_FAILED },
			{ status: 500 }
		);
	}

	return NextResponse.json({
		message: `Successfully cleaned up ${invalidModels.length} models with invalid UUIDs`,
		cleaned: invalidModels.length,
		invalidModels: invalidModels.map((m: any) => ({
			id: m.id,
			model_id: m.model_id,
			name: m.display_name,
		})),
	});
}
