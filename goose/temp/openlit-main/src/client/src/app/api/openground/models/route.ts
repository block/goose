import { NextRequest, NextResponse } from "next/server";
import { getServerSession } from "next-auth";
import { authOptions } from "@/app/auth";
import { dataCollector } from "@/lib/platform/common";
import { getDBConfigByUser } from "@/lib/db-config";
import getMessage from "@/constants/messages";
import Sanitizer from "@/utils/sanitizer";
import { OPENLIT_OPENGROUND_CUSTOM_MODELS_TABLE_NAME } from "@/lib/platform/openground/table-details";
import asaw from "@/utils/asaw";

// Extend the session type to include id
interface SessionWithId {
	user?: {
		id?: string;
		name?: string | null;
		email?: string | null;
		image?: string | null;
	};
}

// GET: List all custom models for a provider (or all providers if no provider specified)
export async function GET(request: NextRequest) {
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

	const searchParams = request.nextUrl.searchParams;
	const provider = searchParams.get("provider");

	// If no provider specified, return all custom models grouped by provider
	if (!provider) {
		const query = `
			SELECT
				toString(id) as customId,
				model_id as id,
				provider,
				display_name as displayName,
				context_window as contextWindow,
				input_price_per_m_token as inputPricePerMToken,
				output_price_per_m_token as outputPricePerMToken,
				capabilities
			FROM ${OPENLIT_OPENGROUND_CUSTOM_MODELS_TABLE_NAME}
			WHERE created_by_user_id = '${Sanitizer.sanitizeValue(session.user.id)}'
			  AND database_config_id = '${Sanitizer.sanitizeValue(dbConfig.id)}'
			ORDER BY provider, created_at DESC
		`;

		const { data, err } = await dataCollector(
			{ query },
			"query",
			dbConfig.id
		);

		if (err) {
			return NextResponse.json(
				{ error: getMessage().OPERATION_FAILED },
				{ status: 500 }
			);
		}

		// Filter out invalid UUIDs and group by provider
		const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
		const grouped: Record<string, any[]> = {};
		const invalidIds: string[] = [];

		(data as any[] || []).forEach((model: any) => {
			// Check if UUID is valid
			if (!model.customId || !uuidRegex.test(model.customId)) {
				invalidIds.push(model.customId || 'null');
				return; // Skip invalid models
			}

			if (!grouped[model.provider]) {
				grouped[model.provider] = [];
			}
			grouped[model.provider].push(model);
		});

		// Log invalid IDs for cleanup
		if (invalidIds.length > 0) {
			// Use separate parameters to prevent log injection
			console.warn('Found', invalidIds.length, 'models with invalid UUIDs:', invalidIds);
			console.warn('These models will not be shown in the UI. Clean them up by calling: DELETE FROM', OPENLIT_OPENGROUND_CUSTOM_MODELS_TABLE_NAME, 'WHERE id NOT IN (SELECT id FROM', OPENLIT_OPENGROUND_CUSTOM_MODELS_TABLE_NAME, 'WHERE match(toString(id), \'^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$\'))');
		}

		console.log("GET /api/openground/models - Returning grouped models:", JSON.stringify(grouped, null, 2));

		return NextResponse.json(grouped);
	}

	// Get models for specific provider
	const query = `
		SELECT
			toString(id) as customId,
			provider,
			model_id as id,
			display_name as displayName,
			context_window as contextWindow,
			input_price_per_m_token as inputPricePerMToken,
			output_price_per_m_token as outputPricePerMToken,
			capabilities
		FROM ${OPENLIT_OPENGROUND_CUSTOM_MODELS_TABLE_NAME}
		WHERE created_by_user_id = '${Sanitizer.sanitizeValue(session.user.id)}'
		  AND database_config_id = '${Sanitizer.sanitizeValue(dbConfig.id)}'
		  AND provider = '${Sanitizer.sanitizeValue(provider)}'
		ORDER BY created_at DESC
	`;

	const { data, err } = await dataCollector(
		{ query },
		"query",
		dbConfig.id
	);

	if (err) {
		return NextResponse.json(
			{ error: getMessage().OPERATION_FAILED },
			{ status: 500 }
		);
	}

	// Filter out invalid UUIDs
	const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
	const validModels = (data as any[] || []).filter((model: any) => {
		const isValid = model.customId && uuidRegex.test(model.customId);
		if (!isValid) {
			console.warn(`Filtering out model with invalid UUID:`, model.customId);
		}
		return isValid;
	});

	return NextResponse.json(validModels);
}

// POST: Create or update a custom model
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

	const body = await request.json();
	const { provider, model, customId } = body;

	console.log("POST /api/openground/models - Body:", JSON.stringify(body, null, 2));
	console.log("CustomId type:", typeof customId, "value:", customId);

	if (!provider || !model?.id || !model?.displayName) {
		console.error("Validation failed - provider:", provider, "model.id:", model?.id, "model.displayName:", model?.displayName);
		return NextResponse.json(
			{ error: "Provider, model ID, and display name are required" },
			{ status: 400 }
		);
	}

	// Check if updating existing model
	if (customId) {
		console.log("Updating custom model with ID:", customId);

		// Validate UUID format
		const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
		if (!uuidRegex.test(customId)) {
			console.error("Invalid UUID format:", customId);
			return NextResponse.json(
				{ error: `Invalid UUID format: ${customId}` },
				{ status: 400 }
			);
		}

		// Use ALTER TABLE UPDATE for ClickHouse
		const capabilitiesArray = (model.capabilities || [])
			.map((c: string) => `'${Sanitizer.sanitizeValue(c)}'`)
			.join(", ");

		const updateQuery = `
			ALTER TABLE ${OPENLIT_OPENGROUND_CUSTOM_MODELS_TABLE_NAME}
			UPDATE
				display_name = '${Sanitizer.sanitizeValue(model.displayName)}',
				context_window = ${model.contextWindow || 4096},
				input_price_per_m_token = ${model.inputPricePerMToken || 0},
				output_price_per_m_token = ${model.outputPricePerMToken || 0},
				capabilities = [${capabilitiesArray}],
				updated_at = now()
			WHERE id = toUUID('${Sanitizer.sanitizeValue(customId)}')
		`;

		console.log("Update query:", updateQuery);

		const { err } = await dataCollector(
			{ query: updateQuery },
			"exec",
			dbConfig.id
		);

		if (err) {
			console.error("Error updating model:", err);
			return NextResponse.json(
				{ error: err || getMessage().OPERATION_FAILED },
				{ status: 500 }
			);
		}

		return NextResponse.json({ success: true, updated: true });
	}

	// Insert new model - Use dataCollector's insert type to let ClickHouse handle UUID generation
	const { err } = await dataCollector(
		{
			table: OPENLIT_OPENGROUND_CUSTOM_MODELS_TABLE_NAME,
			values: [
				{
					provider: provider,
					model_id: model.id,
					display_name: model.displayName,
					context_window: model.contextWindow || 4096,
					input_price_per_m_token: model.inputPricePerMToken || 0,
					output_price_per_m_token: model.outputPricePerMToken || 0,
					capabilities: model.capabilities || [],
					created_by_user_id: session.user.id,
					database_config_id: dbConfig.id,
				},
			],
		},
		"insert",
		dbConfig.id
	);

	if (err) {
		console.error("Error inserting custom model:", err);
		return NextResponse.json(
			{ error: err || getMessage().OPERATION_FAILED },
			{ status: 500 }
		);
	}

	// Fetch the newly created model to return it
	const selectQuery = `
		SELECT
			toString(id) as customId,
			provider,
			model_id as id,
			display_name as displayName,
			context_window as contextWindow,
			input_price_per_m_token as inputPricePerMToken,
			output_price_per_m_token as outputPricePerMToken,
			capabilities
		FROM ${OPENLIT_OPENGROUND_CUSTOM_MODELS_TABLE_NAME}
		WHERE created_by_user_id = '${Sanitizer.sanitizeValue(session.user.id)}'
		  AND database_config_id = '${Sanitizer.sanitizeValue(dbConfig.id)}'
		  AND provider = '${Sanitizer.sanitizeValue(provider)}'
		  AND model_id = '${Sanitizer.sanitizeValue(model.id)}'
		ORDER BY created_at DESC
		LIMIT 1
	`;

	const { data: newModel } = await dataCollector(
		{ query: selectQuery },
		"query",
		dbConfig.id
	);

	console.log("Newly created model:", JSON.stringify(newModel, null, 2));

	return NextResponse.json({
		success: true,
		created: true,
		model: (newModel as any[])?.[0]
	});
}

// DELETE: Remove a custom model
export async function DELETE(request: NextRequest) {
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

	const searchParams = request.nextUrl.searchParams;
	const id = searchParams.get("id");

	console.log("DELETE - id parameter:", id, "type:", typeof id);

	if (!id) {
		return NextResponse.json(
			{ error: "Model ID parameter is required" },
			{ status: 400 }
		);
	}

	// Validate UUID format
	const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
	if (!uuidRegex.test(id)) {
		console.error("Invalid UUID format:", id);
		return NextResponse.json(
			{ error: `Invalid UUID format: ${id}` },
			{ status: 400 }
		);
	}

	const deleteQuery = `
		DELETE FROM ${OPENLIT_OPENGROUND_CUSTOM_MODELS_TABLE_NAME}
		WHERE id = toUUID('${Sanitizer.sanitizeValue(id)}')
		  AND created_by_user_id = '${Sanitizer.sanitizeValue(session.user.id)}'
		  AND database_config_id = '${Sanitizer.sanitizeValue(dbConfig.id)}'
	`;

	console.log("Delete query:", deleteQuery);

	const { err } = await dataCollector(
		{ query: deleteQuery },
		"exec",
		dbConfig.id
	);

	if (err) {
		console.error("Delete error:", err);
		return NextResponse.json(
			{ error: err || getMessage().OPERATION_FAILED },
			{ status: 500 }
		);
	}

	return NextResponse.json({ success: true, deleted: true });
}
