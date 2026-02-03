import { AISdkAdapter } from "./ai-sdk-adapter";
import { getOpenGroundConfigWithSecret } from "./config";
import { ProviderRegistry } from "./provider-registry";
import { createOpengroundEvaluation } from "@/lib/platform/openground-clickhouse";
import type { ProviderResult } from "@/lib/platform/openground-clickhouse";
import { getSpecificPrompt } from "@/lib/platform/prompt";

export interface PromptSource {
	type: "custom" | "prompt-hub";
	content?: string; // For custom prompts
	promptId?: string; // For Prompt Hub
	version?: number; // For Prompt Hub
	variables?: Record<string, string>; // Variable substitutions
}

export interface ProviderEvaluationRequest {
	provider: string;
	model: string;
	config: {
		temperature?: number;
		maxTokens?: number;
		topP?: number;
	};
}

export interface EvaluateParams {
	promptSource: PromptSource;
	providers: ProviderEvaluationRequest[];
	userId: string;
	databaseConfigId: string;
}

/**
 * Resolve the final prompt from the prompt source
 */
async function resolvePrompt(
	promptSource: PromptSource,
	databaseConfigId: string
): Promise<{ prompt: string; err?: string }> {
	let prompt = "";

	// If content is already provided (from frontend), use it directly
	if (promptSource.content) {
		prompt = promptSource.content;
	}
	// Otherwise, fetch from Prompt Hub if needed
	else if (promptSource.type === "prompt-hub" && promptSource.promptId) {
		const { data, err } = await getSpecificPrompt(
			{
				id: promptSource.promptId,
				version: promptSource.version?.toString(),
			},
			databaseConfigId
		);

		if (err || !data || !(data as any[])?.length) {
			return { prompt: "", err: "Failed to fetch prompt from Prompt Hub" };
		}

		prompt = (data as any[])[0]?.prompt;
	} else {
		return { prompt: "", err: "Invalid prompt source" };
	}

	// Substitute variables
	if (promptSource.variables) {
		Object.entries(promptSource.variables).forEach(([key, value]) => {
			// Escape special regex characters to prevent RegExp injection
			const escapedKey = key.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
			const regex = new RegExp(`\\{\\{\\s*${escapedKey}\\s*\\}\\}`, "g");
			prompt = prompt.replace(regex, value);
		});
	}

	return { prompt };
}

/**
 * Calculate cost based on token usage and provider pricing
 */
async function calculateCost(
	provider: string,
	model: string,
	promptTokens: number,
	completionTokens: number
): Promise<number> {
	// Get pricing from registry
	const providerMetadata = await ProviderRegistry.getProviderById(provider);
	if (!providerMetadata) {
		// Use separate parameter to prevent log injection
		console.warn('Provider metadata not found for:', provider);
		return 0;
	}

	const modelMetadata = providerMetadata.supportedModels.find(
		(m) => m.id === model
	);
	if (!modelMetadata) {
		// Use separate parameters to prevent log injection
		console.warn('Model metadata not found for:', provider, '/', model);
		return 0;
	}

	const inputCost = (promptTokens / 1_000_000) * modelMetadata.inputPricePerMToken;
	const outputCost =
		(completionTokens / 1_000_000) * modelMetadata.outputPricePerMToken;

	return inputCost + outputCost;
}

/**
 * Evaluate a single provider
 */
async function evaluateProvider(
	provider: string,
	model: string,
	prompt: string,
	config: Record<string, any>,
	userId: string,
	databaseConfigId: string
): Promise<ProviderResult> {
	const startTime = Date.now();

	try {
		// Get API key from Vault via config
		const { data: configWithSecret, err: configErr } =
			await getOpenGroundConfigWithSecret(provider, userId, databaseConfigId);

		if (configErr || !configWithSecret) {
			return {
				provider,
				model,
				config,
				response: "",
				error: configErr || "Provider not configured",
				cost: 0,
				promptTokens: 0,
				completionTokens: 0,
				totalTokens: 0,
				responseTime: (Date.now() - startTime) / 1000,
				finishReason: "",
				providerResponse: {},
			};
		}

		// Call AI SDK
		const result = await AISdkAdapter.generateCompletion({
			provider,
			model,
			apiKey: configWithSecret.apiKey,
			systemPrompt: prompt,
			temperature: config.temperature,
			maxTokens: config.maxTokens,
			topP: config.topP,
		});

		const responseTime = (Date.now() - startTime) / 1000;

		// Calculate cost
		const cost = await calculateCost(
			provider,
			model,
			result.usage.promptTokens,
			result.usage.completionTokens
		);

		return {
			provider,
			model,
			config,
			response: result.text,
			error: "",
			cost,
			promptTokens: result.usage.promptTokens,
			completionTokens: result.usage.completionTokens,
			totalTokens: result.usage.totalTokens,
			responseTime,
			finishReason: result.finishReason,
			providerResponse: result,
		};
	} catch (error: any) {
		const responseTime = (Date.now() - startTime) / 1000;
		// Use separate parameters to prevent log injection
		console.error('Error evaluating provider/model:', provider, '/', model, error);

		return {
			provider,
			model,
			config,
			response: "",
			error: error.message || String(error),
			cost: 0,
			promptTokens: 0,
			completionTokens: 0,
			totalTokens: 0,
			responseTime,
			finishReason: "error",
			providerResponse: { error: error.message },
		};
	}
}

/**
 * Evaluate multiple providers in parallel
 */
export async function evaluate(
	params: EvaluateParams
): Promise<{ data?: ProviderResult[]; err?: string }> {
	try {
		// Resolve prompt
		const { prompt, err: promptErr } = await resolvePrompt(
			params.promptSource,
			params.databaseConfigId
		);

		if (promptErr) {
			return { err: promptErr };
		}

		// Evaluate all providers in parallel
		const providerResults = await Promise.all(
			params.providers.map((p) =>
				evaluateProvider(
					p.provider,
					p.model,
					prompt,
					p.config,
					params.userId,
					params.databaseConfigId
				)
			)
		);

		// Save to ClickHouse
		const { err: saveErr } = await createOpengroundEvaluation(
			{
				prompt,
				promptSource: params.promptSource.type,
				promptHubId: params.promptSource.promptId,
				promptHubVersion: params.promptSource.version?.toString(),
				promptVariables: params.promptSource.variables,
				providers: providerResults,
			},
			params.userId,
			params.databaseConfigId
		);

		if (saveErr) {
			console.error("Error saving OpenGround evaluation:", saveErr);
			// Continue even if save fails - return results to user
		}

		return { data: providerResults };
	} catch (error: any) {
		console.error("Error in evaluate:", error);
		return { err: error.message || "Evaluation failed" };
	}
}
