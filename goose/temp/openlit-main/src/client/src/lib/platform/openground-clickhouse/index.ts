import { dataCollector } from "@/lib/platform/common";
import getMessage from "@/constants/messages";
import Sanitizer from "@/utils/sanitizer";
import {
	OPENLIT_OPENGROUND_TABLE_NAME,
	OPENLIT_OPENGROUND_PROVIDERS_TABLE_NAME,
} from "@/lib/platform/openground/table-details";

export interface OpengroundRecord {
	id: string;
	prompt: string;
	promptSource: "custom" | "prompt-hub";
	promptHubId?: string;
	promptHubVersion?: string;
	promptVariables?: Record<string, string>;
	createdByUserId: string;
	databaseConfigId: string;
	createdAt: Date;
	totalProviders: number;
	minCost: number;
	minCostProvider: string;
	minResponseTime: number;
	minResponseTimeProvider: string;
	minCompletionTokens: number;
	minCompletionTokensProvider: string;
	errors: string[];
	providers?: ProviderResult[];
}

export interface ProviderResult {
	id?: string;
	opengroundId?: string;
	provider: string;
	model: string;
	config: Record<string, any>;
	response: string;
	error: string;
	cost: number;
	promptTokens: number;
	completionTokens: number;
	totalTokens: number;
	responseTime: number;
	finishReason: string;
	providerResponse: Record<string, any>;
	createdAt?: Date;
}

// INSERT OpenGround evaluation
export async function createOpengroundEvaluation(
	data: {
		prompt: string;
		promptSource: "custom" | "prompt-hub";
		promptHubId?: string;
		promptHubVersion?: string;
		promptVariables?: Record<string, string>;
		providers: ProviderResult[];
	},
	userId: string,
	databaseConfigId: string
) {
	// Compute statistics
	const successfulProviders = data.providers.filter((p) => !p.error);

	// Calculate stats even if all providers failed (use defaults)
	const stats = {
		total_providers: data.providers.length,
		min_cost: successfulProviders.length > 0
			? Math.min(...successfulProviders.map((p) => p.cost))
			: 0,
		min_cost_provider:
			successfulProviders.find(
				(p) => p.cost === Math.min(...successfulProviders.map((p) => p.cost))
			)?.provider || "",
		min_response_time: successfulProviders.length > 0
			? Math.min(...successfulProviders.map((p) => p.responseTime))
			: 0,
		min_response_time_provider:
			successfulProviders.find(
				(p) =>
					p.responseTime ===
					Math.min(...successfulProviders.map((p) => p.responseTime))
			)?.provider || "",
		min_completion_tokens: successfulProviders.length > 0
			? Math.min(
				...successfulProviders.map((p) => p.completionTokens)
			)
			: 0,
		min_completion_tokens_provider:
			successfulProviders.find(
				(p) =>
					p.completionTokens ===
					Math.min(...successfulProviders.map((p) => p.completionTokens))
			)?.provider || "",
		errors: data.providers.filter((p) => p.error).map((p) => p.error),
	};

	// Insert main record
	const { err, data: insertData } = await dataCollector(
		{
			table: OPENLIT_OPENGROUND_TABLE_NAME,
			values: [
				{
					prompt: data.prompt,
					prompt_source: data.promptSource,
					prompt_hub_id: data.promptHubId || null,
					prompt_hub_version: data.promptHubVersion || null,
					prompt_variables: JSON.stringify(data.promptVariables || {}),
					created_by_user_id: userId,
					database_config_id: databaseConfigId,
					...stats,
				},
			],
		},
		"insert",
		databaseConfigId
	);

	if (err || !(insertData as { query_id: string }).query_id) {
		console.error("Error inserting OpenGround record:", err);
		return { err: getMessage().OPENGROUND_CREATE_FAILED };
	}

	// Get the inserted ID (last insert)
	const { data: lastInsert } = await dataCollector(
		{
			query: `SELECT id FROM ${OPENLIT_OPENGROUND_TABLE_NAME}
              WHERE created_by_user_id = '${Sanitizer.sanitizeValue(userId)}'
              ORDER BY created_at DESC LIMIT 1`,
		},
		"query",
		databaseConfigId
	);

	const opengroundId = (lastInsert as any[])?.[0]?.id;

	if (!opengroundId) {
		return { err: "Failed to retrieve inserted record ID" };
	}

	// Insert provider results
	const providerValues = data.providers.map((p) => ({
		openground_id: opengroundId,
		provider: p.provider,
		model: p.model,
		config: JSON.stringify(p.config),
		response: p.response,
		error: p.error || "",
		cost: p.cost,
		prompt_tokens: p.promptTokens,
		completion_tokens: p.completionTokens,
		total_tokens: p.totalTokens,
		response_time: p.responseTime,
		finish_reason: p.finishReason,
		provider_response: JSON.stringify(p.providerResponse),
	}));

	const { err: providerErr } = await dataCollector(
		{
			table: OPENLIT_OPENGROUND_PROVIDERS_TABLE_NAME,
			values: providerValues,
		},
		"insert",
		databaseConfigId
	);

	if (providerErr) {
		console.error("Error inserting provider results:", providerErr);
		return { err: getMessage().OPENGROUND_CREATE_FAILED };
	}

	return { data: { id: opengroundId } };
}

// GET all evaluations for a user
export async function getOpengroundEvaluations(
	userId: string,
	databaseConfigId: string,
	options?: {
		page?: number;
		limit?: number;
		startDate?: Date;
		endDate?: Date;
	}
) {
	const page = options?.page || 1;
	const limit = options?.limit || 20;
	const offset = (page - 1) * limit;

	const whereConditions = [
		`created_by_user_id = '${Sanitizer.sanitizeValue(userId)}'`,
		`database_config_id = '${Sanitizer.sanitizeValue(databaseConfigId)}'`,
	];

	if (options?.startDate) {
		whereConditions.push(
			`created_at >= parseDateTimeBestEffort('${options.startDate.toISOString()}')`
		);
	}
	if (options?.endDate) {
		whereConditions.push(
			`created_at <= parseDateTimeBestEffort('${options.endDate.toISOString()}')`
		);
	}

	const whereClause = `WHERE ${whereConditions.join(" AND ")}`;

	const query = `
    SELECT
      id,
      prompt,
      prompt_source as promptSource,
      prompt_hub_id as promptHubId,
      prompt_hub_version as promptHubVersion,
      prompt_variables as promptVariables,
      created_by_user_id as createdByUserId,
      database_config_id as databaseConfigId,
      created_at as createdAt,
      total_providers as totalProviders,
      min_cost as minCost,
      min_cost_provider as minCostProvider,
      min_response_time as minResponseTime,
      min_response_time_provider as minResponseTimeProvider,
      min_completion_tokens as minCompletionTokens,
      min_completion_tokens_provider as minCompletionTokensProvider,
      errors
    FROM ${OPENLIT_OPENGROUND_TABLE_NAME}
    ${whereClause}
    ORDER BY created_at DESC
    LIMIT ${limit}
    OFFSET ${offset}
  `;

	const { data, err } = await dataCollector(
		{ query },
		"query",
		databaseConfigId
	);

	if (err) {
		console.error("Error fetching OpenGround evaluations:", err);
		return { err: getMessage().OPENGROUND_FETCH_FAILED };
	}

	// Parse JSON fields
	const records = (data as any[])?.map((record) => ({
		...record,
		promptVariables: JSON.parse(record.promptVariables || "{}"),
	}));

	return { data: records };
}

// GET single evaluation with providers
export async function getOpengroundEvaluationById(
	id: string,
	databaseConfigId: string
) {
	const sanitizedId = Sanitizer.sanitizeValue(id);

	// Get main record
	const mainQuery = `
    SELECT
      id,
      prompt,
      prompt_source as promptSource,
      prompt_hub_id as promptHubId,
      prompt_hub_version as promptHubVersion,
      prompt_variables as promptVariables,
      created_by_user_id as createdByUserId,
      database_config_id as databaseConfigId,
      created_at as createdAt,
      total_providers as totalProviders,
      min_cost as minCost,
      min_cost_provider as minCostProvider,
      min_response_time as minResponseTime,
      min_response_time_provider as minResponseTimeProvider,
      min_completion_tokens as minCompletionTokens,
      min_completion_tokens_provider as minCompletionTokensProvider,
      errors
    FROM ${OPENLIT_OPENGROUND_TABLE_NAME}
    WHERE id = '${sanitizedId}'
  `;

	const { data: mainData, err: mainErr } = await dataCollector(
		{ query: mainQuery },
		"query",
		databaseConfigId
	);

	if (mainErr || !(mainData as any[])?.length) {
		console.error("Error fetching OpenGround evaluation:", mainErr);
		return { err: getMessage().OPENGROUND_FETCH_FAILED };
	}

	const record = (mainData as any[])[0];

	// Get provider results
	const providerQuery = `
    SELECT
      id,
      openground_id as opengroundId,
      provider,
      model,
      config,
      response,
      error,
      cost,
      prompt_tokens as promptTokens,
      completion_tokens as completionTokens,
      total_tokens as totalTokens,
      response_time as responseTime,
      finish_reason as finishReason,
      provider_response as providerResponse,
      created_at as createdAt
    FROM ${OPENLIT_OPENGROUND_PROVIDERS_TABLE_NAME}
    WHERE openground_id = '${sanitizedId}'
    ORDER BY created_at ASC
  `;

	const { data: providerData, err: providerErr } = await dataCollector(
		{ query: providerQuery },
		"query",
		databaseConfigId
	);

	if (providerErr) {
		console.error("Error fetching provider results:", providerErr);
		return { err: getMessage().OPENGROUND_FETCH_FAILED };
	}

	// Parse JSON fields
	const providers = (providerData as any[])?.map((p) => ({
		...p,
		config: JSON.parse(p.config || "{}"),
		providerResponse: JSON.parse(p.providerResponse || "{}"),
	}));

	return {
		data: {
			...record,
			promptVariables: JSON.parse(record.promptVariables || "{}"),
			providers,
		},
	};
}

// DELETE evaluation
export async function deleteOpengroundEvaluation(
	id: string,
	databaseConfigId: string
) {
	const sanitizedId = Sanitizer.sanitizeValue(id);

	const queries = [
		`DELETE FROM ${OPENLIT_OPENGROUND_PROVIDERS_TABLE_NAME} WHERE openground_id = '${sanitizedId}'`,
		`DELETE FROM ${OPENLIT_OPENGROUND_TABLE_NAME} WHERE id = '${sanitizedId}'`,
	];

	const queryResponses = await Promise.all(
		queries.map(
			async (query) => await dataCollector({ query }, "exec", databaseConfigId)
		)
	);

	const errors = queryResponses.filter((res) => res.err);
	if (errors.length > 0) {
		console.error("Error deleting OpenGround evaluation:", errors);
		return { err: getMessage().OPENGROUND_DELETE_FAILED };
	}

	return { data: "Deleted successfully" };
}
