import { ProviderMetadata, ModelMetadata, ConfigField } from "@/types/openground";

// Re-export types for convenience
export type { ProviderMetadata, ModelMetadata, ConfigField };

// Provider metadata registry
const PROVIDER_METADATA: Record<string, ProviderMetadata> = {
	openai: {
		providerId: "openai",
		displayName: "OpenAI",
		description: "GPT models from OpenAI",
		requiresVault: true,
		supportedModels: [
			{
				id: "gpt-4o",
				displayName: "GPT-4o",
				contextWindow: 128000,
				inputPricePerMToken: 2.5,
				outputPricePerMToken: 10.0,
				capabilities: ["function-calling", "vision", "streaming"],
			},
			{
				id: "gpt-4o-mini",
				displayName: "GPT-4o Mini",
				contextWindow: 128000,
				inputPricePerMToken: 0.15,
				outputPricePerMToken: 0.6,
				capabilities: ["function-calling", "vision", "streaming"],
			},
			{
				id: "gpt-4-turbo",
				displayName: "GPT-4 Turbo",
				contextWindow: 128000,
				inputPricePerMToken: 10.0,
				outputPricePerMToken: 30.0,
				capabilities: ["function-calling", "vision", "streaming"],
			},
			{
				id: "gpt-3.5-turbo",
				displayName: "GPT-3.5 Turbo",
				contextWindow: 16385,
				inputPricePerMToken: 0.5,
				outputPricePerMToken: 1.5,
				capabilities: ["function-calling", "streaming"],
			},
		],
		configSchema: {
			temperature: {
				min: 0,
				max: 2,
				step: 0.1,
				default: 1,
				description: "Sampling temperature (0 = deterministic, 2 = very random)",
			},
			maxTokens: {
				min: 1,
				max: 16000,
				step: 1,
				default: 1000,
				description: "Maximum tokens to generate",
			},
			topP: {
				min: 0,
				max: 1,
				step: 0.1,
				default: 1,
				description: "Nucleus sampling threshold",
			},
		},
	},
	anthropic: {
		providerId: "anthropic",
		displayName: "Anthropic",
		description: "Claude models from Anthropic",
		requiresVault: true,
		supportedModels: [
			{
				id: "claude-3-5-sonnet-20240620",
				displayName: "Claude 3.5 Sonnet",
				contextWindow: 200000,
				inputPricePerMToken: 3.0,
				outputPricePerMToken: 15.0,
				capabilities: ["function-calling", "vision", "streaming"],
			},
			{
				id: "claude-3-haiku-20240307",
				displayName: "Claude 3 Haiku",
				contextWindow: 200000,
				inputPricePerMToken: 0.25,
				outputPricePerMToken: 1.25,
				capabilities: ["function-calling", "vision", "streaming"],
			},
			{
				id: "claude-3-sonnet-20240229",
				displayName: "Claude 3 Sonnet",
				contextWindow: 200000,
				inputPricePerMToken: 3.0,
				outputPricePerMToken: 15.0,
				capabilities: ["function-calling", "vision", "streaming"],
			},
		],
		configSchema: {
			temperature: {
				min: 0,
				max: 1,
				step: 0.1,
				default: 1,
				description: "Sampling temperature",
			},
			maxTokens: {
				min: 1,
				max: 8096,
				step: 1,
				default: 1000,
				description: "Maximum tokens to generate",
			},
			topP: {
				min: 0,
				max: 1,
				step: 0.1,
				default: 1,
				description: "Nucleus sampling threshold",
			},
		},
	},
	google: {
		providerId: "google",
		displayName: "Google AI",
		description: "Gemini models from Google",
		requiresVault: true,
		supportedModels: [
			{
				id: "gemini-1.5-pro",
				displayName: "Gemini 1.5 Pro",
				contextWindow: 2000000,
				inputPricePerMToken: 1.25,
				outputPricePerMToken: 5.0,
				capabilities: ["function-calling", "vision", "streaming"],
			},
			{
				id: "gemini-1.5-flash",
				displayName: "Gemini 1.5 Flash",
				contextWindow: 1000000,
				inputPricePerMToken: 0.075,
				outputPricePerMToken: 0.3,
				capabilities: ["function-calling", "vision", "streaming"],
			},
			{
				id: "gemini-1.0-pro",
				displayName: "Gemini 1.0 Pro",
				contextWindow: 32768,
				inputPricePerMToken: 0.5,
				outputPricePerMToken: 1.5,
				capabilities: ["function-calling", "streaming"],
			},
		],
		configSchema: {
			temperature: {
				min: 0,
				max: 2,
				step: 0.1,
				default: 1,
				description: "Sampling temperature",
			},
			maxTokens: {
				min: 1,
				max: 8192,
				step: 1,
				default: 1000,
				description: "Maximum tokens to generate",
			},
			topP: {
				min: 0,
				max: 1,
				step: 0.1,
				default: 0.95,
				description: "Nucleus sampling threshold",
			},
		},
	},
	mistral: {
		providerId: "mistral",
		displayName: "Mistral AI",
		description: "Mistral and Mixtral models",
		requiresVault: true,
		supportedModels: [
			{
				id: "mistral-large-latest",
				displayName: "Mistral Large",
				contextWindow: 128000,
				inputPricePerMToken: 2.0,
				outputPricePerMToken: 6.0,
				capabilities: ["function-calling", "streaming"],
			},
			{
				id: "mistral-medium-latest",
				displayName: "Mistral Medium",
				contextWindow: 32000,
				inputPricePerMToken: 2.7,
				outputPricePerMToken: 8.1,
				capabilities: ["streaming"],
			},
			{
				id: "mistral-small-latest",
				displayName: "Mistral Small",
				contextWindow: 32000,
				inputPricePerMToken: 0.2,
				outputPricePerMToken: 0.6,
				capabilities: ["streaming"],
			},
		],
		configSchema: {
			temperature: {
				min: 0,
				max: 1,
				step: 0.1,
				default: 0.7,
				description: "Sampling temperature",
			},
			maxTokens: {
				min: 1,
				max: 8192,
				step: 1,
				default: 1000,
				description: "Maximum tokens to generate",
			},
			topP: {
				min: 0,
				max: 1,
				step: 0.1,
				default: 1,
				description: "Nucleus sampling threshold",
			},
		},
	},
	groq: {
		providerId: "groq",
		displayName: "Groq",
		description: "Ultra-fast inference for open source models",
		requiresVault: true,
		supportedModels: [
			{
				id: "llama-3.3-70b-versatile",
				displayName: "Llama 3.3 70B",
				contextWindow: 128000,
				inputPricePerMToken: 0.59,
				outputPricePerMToken: 0.79,
				capabilities: ["streaming"],
			},
			{
				id: "llama-3.1-8b-instant",
				displayName: "Llama 3.1 8B Instant",
				contextWindow: 128000,
				inputPricePerMToken: 0.05,
				outputPricePerMToken: 0.08,
				capabilities: ["streaming"],
			},
			{
				id: "mixtral-8x7b-32768",
				displayName: "Mixtral 8x7B",
				contextWindow: 32768,
				inputPricePerMToken: 0.24,
				outputPricePerMToken: 0.24,
				capabilities: ["streaming"],
			},
			{
				id: "gemma2-9b-it",
				displayName: "Gemma 2 9B",
				contextWindow: 8192,
				inputPricePerMToken: 0.2,
				outputPricePerMToken: 0.2,
				capabilities: ["streaming"],
			},
		],
		configSchema: {
			temperature: {
				min: 0,
				max: 2,
				step: 0.1,
				default: 1,
				description: "Sampling temperature",
			},
			maxTokens: {
				min: 1,
				max: 32768,
				step: 1,
				default: 1000,
				description: "Maximum tokens to generate",
			},
			topP: {
				min: 0,
				max: 1,
				step: 0.1,
				default: 1,
				description: "Nucleus sampling threshold",
			},
		},
	},
	perplexity: {
		providerId: "perplexity",
		displayName: "Perplexity",
		description: "Models with online search capabilities",
		requiresVault: true,
		supportedModels: [
			{
				id: "sonar-pro",
				displayName: "Sonar Pro",
				contextWindow: 127072,
				inputPricePerMToken: 3.0,
				outputPricePerMToken: 15.0,
				capabilities: ["streaming", "search"],
			},
			{
				id: "sonar",
				displayName: "Sonar",
				contextWindow: 127072,
				inputPricePerMToken: 1.0,
				outputPricePerMToken: 1.0,
				capabilities: ["streaming", "search"],
			},
			{
				id: "sonar-reasoning",
				displayName: "Sonar Reasoning",
				contextWindow: 127072,
				inputPricePerMToken: 5.0,
				outputPricePerMToken: 5.0,
				capabilities: ["streaming", "search", "reasoning"],
			},
		],
		configSchema: {
			temperature: {
				min: 0,
				max: 2,
				step: 0.1,
				default: 0.2,
				description: "Sampling temperature",
			},
			maxTokens: {
				min: 1,
				max: 4096,
				step: 1,
				default: 1000,
				description: "Maximum tokens to generate",
			},
			topP: {
				min: 0,
				max: 1,
				step: 0.1,
				default: 0.9,
				description: "Nucleus sampling threshold",
			},
		},
	},
	azure: {
		providerId: "azure",
		displayName: "Azure OpenAI",
		description: "OpenAI models via Azure",
		requiresVault: true,
		supportedModels: [
			{
				id: "gpt-4o",
				displayName: "GPT-4o",
				contextWindow: 128000,
				inputPricePerMToken: 2.5,
				outputPricePerMToken: 10.0,
				capabilities: ["function-calling", "vision", "streaming"],
			},
			{
				id: "gpt-4o-mini",
				displayName: "GPT-4o Mini",
				contextWindow: 128000,
				inputPricePerMToken: 0.15,
				outputPricePerMToken: 0.6,
				capabilities: ["function-calling", "vision", "streaming"],
			},
			{
				id: "gpt-4-turbo",
				displayName: "GPT-4 Turbo",
				contextWindow: 128000,
				inputPricePerMToken: 10.0,
				outputPricePerMToken: 30.0,
				capabilities: ["function-calling", "vision", "streaming"],
			},
			{
				id: "gpt-35-turbo",
				displayName: "GPT-3.5 Turbo",
				contextWindow: 16385,
				inputPricePerMToken: 0.5,
				outputPricePerMToken: 1.5,
				capabilities: ["function-calling", "streaming"],
			},
		],
		configSchema: {
			temperature: {
				min: 0,
				max: 2,
				step: 0.1,
				default: 1,
				description: "Sampling temperature",
			},
			maxTokens: {
				min: 1,
				max: 16000,
				step: 1,
				default: 1000,
				description: "Maximum tokens to generate",
			},
			topP: {
				min: 0,
				max: 1,
				step: 0.1,
				default: 1,
				description: "Nucleus sampling threshold",
			},
		},
	},
	cohere: {
		providerId: "cohere",
		displayName: "Cohere",
		description: "Command models with RAG capabilities",
		requiresVault: true,
		supportedModels: [
			{
				id: "command-r-plus",
				displayName: "Command R+",
				contextWindow: 128000,
				inputPricePerMToken: 3.0,
				outputPricePerMToken: 15.0,
				capabilities: ["function-calling", "streaming", "rag"],
			},
			{
				id: "command-r",
				displayName: "Command R",
				contextWindow: 128000,
				inputPricePerMToken: 0.5,
				outputPricePerMToken: 1.5,
				capabilities: ["function-calling", "streaming", "rag"],
			},
			{
				id: "command",
				displayName: "Command",
				contextWindow: 4096,
				inputPricePerMToken: 1.0,
				outputPricePerMToken: 2.0,
				capabilities: ["streaming"],
			},
			{
				id: "command-light",
				displayName: "Command Light",
				contextWindow: 4096,
				inputPricePerMToken: 0.3,
				outputPricePerMToken: 0.6,
				capabilities: ["streaming"],
			},
		],
		configSchema: {
			temperature: {
				min: 0,
				max: 5,
				step: 0.1,
				default: 0.3,
				description: "Sampling temperature",
			},
			maxTokens: {
				min: 1,
				max: 4096,
				step: 1,
				default: 1000,
				description: "Maximum tokens to generate",
			},
			topP: {
				min: 0,
				max: 0.99,
				step: 0.01,
				default: 0.75,
				description: "Nucleus sampling threshold",
			},
		},
	},
	together: {
		providerId: "together",
		displayName: "Together AI",
		description: "Fast inference for open source models",
		requiresVault: true,
		supportedModels: [
			{
				id: "meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo",
				displayName: "Llama 3.1 70B Turbo",
				contextWindow: 131072,
				inputPricePerMToken: 0.88,
				outputPricePerMToken: 0.88,
				capabilities: ["streaming"],
			},
			{
				id: "meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo",
				displayName: "Llama 3.1 8B Turbo",
				contextWindow: 131072,
				inputPricePerMToken: 0.18,
				outputPricePerMToken: 0.18,
				capabilities: ["streaming"],
			},
			{
				id: "mistralai/Mixtral-8x7B-Instruct-v0.1",
				displayName: "Mixtral 8x7B",
				contextWindow: 32768,
				inputPricePerMToken: 0.6,
				outputPricePerMToken: 0.6,
				capabilities: ["streaming"],
			},
			{
				id: "Qwen/Qwen2.5-72B-Instruct-Turbo",
				displayName: "Qwen 2.5 72B Turbo",
				contextWindow: 32768,
				inputPricePerMToken: 1.2,
				outputPricePerMToken: 1.2,
				capabilities: ["streaming"],
			},
		],
		configSchema: {
			temperature: {
				min: 0,
				max: 1,
				step: 0.1,
				default: 0.7,
				description: "Sampling temperature",
			},
			maxTokens: {
				min: 1,
				max: 8192,
				step: 1,
				default: 1000,
				description: "Maximum tokens to generate",
			},
			topP: {
				min: 0,
				max: 1,
				step: 0.1,
				default: 0.7,
				description: "Nucleus sampling threshold",
			},
		},
	},
	fireworks: {
		providerId: "fireworks",
		displayName: "Fireworks AI",
		description: "Production-ready LLM inference",
		requiresVault: true,
		supportedModels: [
			{
				id: "accounts/fireworks/models/llama-v3p1-70b-instruct",
				displayName: "Llama 3.1 70B",
				contextWindow: 131072,
				inputPricePerMToken: 0.9,
				outputPricePerMToken: 0.9,
				capabilities: ["streaming"],
			},
			{
				id: "accounts/fireworks/models/llama-v3p1-8b-instruct",
				displayName: "Llama 3.1 8B",
				contextWindow: 131072,
				inputPricePerMToken: 0.2,
				outputPricePerMToken: 0.2,
				capabilities: ["streaming"],
			},
			{
				id: "accounts/fireworks/models/mixtral-8x7b-instruct",
				displayName: "Mixtral 8x7B",
				contextWindow: 32768,
				inputPricePerMToken: 0.5,
				outputPricePerMToken: 0.5,
				capabilities: ["streaming"],
			},
			{
				id: "accounts/fireworks/models/qwen2p5-72b-instruct",
				displayName: "Qwen 2.5 72B",
				contextWindow: 32768,
				inputPricePerMToken: 0.9,
				outputPricePerMToken: 0.9,
				capabilities: ["streaming"],
			},
		],
		configSchema: {
			temperature: {
				min: 0,
				max: 2,
				step: 0.1,
				default: 1,
				description: "Sampling temperature",
			},
			maxTokens: {
				min: 1,
				max: 16384,
				step: 1,
				default: 1000,
				description: "Maximum tokens to generate",
			},
			topP: {
				min: 0,
				max: 1,
				step: 0.1,
				default: 1,
				description: "Nucleus sampling threshold",
			},
		},
	},
	deepseek: {
		providerId: "deepseek",
		displayName: "DeepSeek",
		description: "Advanced reasoning and coding models",
		requiresVault: true,
		supportedModels: [
			{
				id: "deepseek-chat",
				displayName: "DeepSeek Chat",
				contextWindow: 64000,
				inputPricePerMToken: 0.27,
				outputPricePerMToken: 1.1,
				capabilities: ["streaming", "reasoning"],
			},
			{
				id: "deepseek-coder",
				displayName: "DeepSeek Coder",
				contextWindow: 16000,
				inputPricePerMToken: 0.27,
				outputPricePerMToken: 1.1,
				capabilities: ["streaming", "coding"],
			},
		],
		configSchema: {
			temperature: {
				min: 0,
				max: 2,
				step: 0.1,
				default: 1,
				description: "Sampling temperature",
			},
			maxTokens: {
				min: 1,
				max: 4096,
				step: 1,
				default: 1000,
				description: "Maximum tokens to generate",
			},
			topP: {
				min: 0,
				max: 1,
				step: 0.1,
				default: 1,
				description: "Nucleus sampling threshold",
			},
		},
	},
	xai: {
		providerId: "xai",
		displayName: "xAI",
		description: "Grok models with real-time knowledge",
		requiresVault: true,
		supportedModels: [
			{
				id: "grok-beta",
				displayName: "Grok Beta",
				contextWindow: 131072,
				inputPricePerMToken: 5.0,
				outputPricePerMToken: 15.0,
				capabilities: ["streaming", "real-time"],
			},
			{
				id: "grok-vision-beta",
				displayName: "Grok Vision Beta",
				contextWindow: 8192,
				inputPricePerMToken: 5.0,
				outputPricePerMToken: 15.0,
				capabilities: ["streaming", "vision", "real-time"],
			},
		],
		configSchema: {
			temperature: {
				min: 0,
				max: 2,
				step: 0.1,
				default: 0,
				description: "Sampling temperature",
			},
			maxTokens: {
				min: 1,
				max: 131072,
				step: 1,
				default: 1000,
				description: "Maximum tokens to generate",
			},
			topP: {
				min: 0,
				max: 1,
				step: 0.1,
				default: 1,
				description: "Nucleus sampling threshold",
			},
		},
	},
	huggingface: {
		providerId: "huggingface",
		displayName: "Hugging Face",
		description: "Open source models via Inference API",
		requiresVault: true,
		supportedModels: [
			{
				id: "meta-llama/Meta-Llama-3-70B-Instruct",
				displayName: "Llama 3 70B",
				contextWindow: 8192,
				inputPricePerMToken: 0.0,
				outputPricePerMToken: 0.0,
				capabilities: ["streaming"],
			},
			{
				id: "mistralai/Mistral-7B-Instruct-v0.3",
				displayName: "Mistral 7B",
				contextWindow: 32768,
				inputPricePerMToken: 0.0,
				outputPricePerMToken: 0.0,
				capabilities: ["streaming"],
			},
			{
				id: "google/gemma-2-9b-it",
				displayName: "Gemma 2 9B",
				contextWindow: 8192,
				inputPricePerMToken: 0.0,
				outputPricePerMToken: 0.0,
				capabilities: ["streaming"],
			},
		],
		configSchema: {
			temperature: {
				min: 0,
				max: 2,
				step: 0.1,
				default: 0.7,
				description: "Sampling temperature",
			},
			maxTokens: {
				min: 1,
				max: 2048,
				step: 1,
				default: 1000,
				description: "Maximum tokens to generate",
			},
			topP: {
				min: 0,
				max: 1,
				step: 0.1,
				default: 0.95,
				description: "Nucleus sampling threshold",
			},
		},
	},
	replicate: {
		providerId: "replicate",
		displayName: "Replicate",
		description: "Run open source models in the cloud",
		requiresVault: true,
		supportedModels: [
			{
				id: "meta/meta-llama-3-70b-instruct",
				displayName: "Llama 3 70B",
				contextWindow: 8192,
				inputPricePerMToken: 0.65,
				outputPricePerMToken: 2.75,
				capabilities: ["streaming"],
			},
			{
				id: "meta/meta-llama-3-8b-instruct",
				displayName: "Llama 3 8B",
				contextWindow: 8192,
				inputPricePerMToken: 0.05,
				outputPricePerMToken: 0.25,
				capabilities: ["streaming"],
			},
			{
				id: "mistralai/mixtral-8x7b-instruct-v0.1",
				displayName: "Mixtral 8x7B",
				contextWindow: 32768,
				inputPricePerMToken: 0.3,
				outputPricePerMToken: 1.0,
				capabilities: ["streaming"],
			},
		],
		configSchema: {
			temperature: {
				min: 0,
				max: 5,
				step: 0.01,
				default: 0.75,
				description: "Sampling temperature",
			},
			maxTokens: {
				min: 1,
				max: 4096,
				step: 1,
				default: 1000,
				description: "Maximum tokens to generate",
			},
			topP: {
				min: 0,
				max: 1,
				step: 0.01,
				default: 0.9,
				description: "Nucleus sampling threshold",
			},
		},
	},
};

export class ProviderRegistry {
	/**
	 * Get all available providers
	 */
	static async getAvailableProviders(): Promise<ProviderMetadata[]> {
		return Object.values(PROVIDER_METADATA);
	}

	/**
	 * Get provider by ID
	 */
	static async getProviderById(
		providerId: string
	): Promise<ProviderMetadata | null> {
		return PROVIDER_METADATA[providerId] || null;
	}

	/**
	 * Get provider by ID with custom models included
	 */
	static async getProviderByIdWithCustomModels(
		providerId: string,
		userId: string,
		databaseConfigId: string
	): Promise<ProviderMetadata | null> {
		const provider = PROVIDER_METADATA[providerId];
		if (!provider) return null;

		// Load custom models from database
		const customModels = await this.getCustomModels(
			providerId,
			userId,
			databaseConfigId
		);

		// Merge custom models with static models
		return {
			...provider,
			supportedModels: [...provider.supportedModels, ...customModels],
		};
	}

	/**
	 * Get all supported models across all providers
	 */
	static async getAllModels(): Promise<
		Array<ModelMetadata & { providerId: string; providerName: string }>
	> {
		const models: Array<
			ModelMetadata & { providerId: string; providerName: string }
		> = [];

		for (const provider of Object.values(PROVIDER_METADATA)) {
			for (const model of provider.supportedModels) {
				models.push({
					...model,
					providerId: provider.providerId,
					providerName: provider.displayName,
				});
			}
		}

		return models;
	}

	/**
	 * Search providers by name or description
	 */
	static async searchProviders(query: string): Promise<ProviderMetadata[]> {
		const lowerQuery = query.toLowerCase();
		return Object.values(PROVIDER_METADATA).filter(
			(provider) =>
				provider.displayName.toLowerCase().includes(lowerQuery) ||
				provider.description?.toLowerCase().includes(lowerQuery)
		);
	}

	/**
	 * Get models for a specific provider (static models only)
	 */
	static async getProviderModels(providerId: string): Promise<ModelMetadata[]> {
		const provider = PROVIDER_METADATA[providerId];
		return provider?.supportedModels || [];
	}

	/**
	 * Get models for a specific provider including custom models
	 */
	static async getProviderModelsWithCustom(
		providerId: string,
		userId: string,
		databaseConfigId: string
	): Promise<ModelMetadata[]> {
		const provider = PROVIDER_METADATA[providerId];
		if (!provider) return [];

		// Load custom models from database
		const customModels = await this.getCustomModels(
			providerId,
			userId,
			databaseConfigId
		);

		// Merge static and custom models
		return [...provider.supportedModels, ...customModels];
	}

	/**
	 * Load custom models from database for a specific provider
	 */
	private static async getCustomModels(
		providerId: string,
		userId: string,
		databaseConfigId: string
	): Promise<ModelMetadata[]> {
		try {
			// Import dynamically to avoid circular dependencies
			const { dataCollector } = await import("@/lib/platform/common");
			const { OPENLIT_OPENGROUND_CUSTOM_MODELS_TABLE_NAME } = await import(
				"@/lib/platform/openground/table-details"
			);
			const Sanitizer = (await import("@/utils/sanitizer")).default;

			const query = `
				SELECT
					model_id as id,
					display_name as displayName,
					context_window as contextWindow,
					input_price_per_m_token as inputPricePerMToken,
					output_price_per_m_token as outputPricePerMToken,
					capabilities
				FROM ${OPENLIT_OPENGROUND_CUSTOM_MODELS_TABLE_NAME}
				WHERE created_by_user_id = '${Sanitizer.sanitizeValue(userId)}'
				  AND database_config_id = '${Sanitizer.sanitizeValue(databaseConfigId)}'
				  AND provider = '${Sanitizer.sanitizeValue(providerId)}'
				ORDER BY created_at DESC
			`;

			const { data } = await dataCollector(
				{ query },
				"query",
				databaseConfigId
			);

			return (data as ModelMetadata[]) || [];
		} catch (error) {
			console.error("Error loading custom models:", error);
			return [];
		}
	}
}
