import { ProviderRegistry } from "./provider-registry";
import { getCustomModelsForProvider, getCustomModelsGroupedByProvider } from "./custom-models-service";
import { ProviderMetadata } from "@/types/openground";

/**
 * Get all available providers with custom models merged in
 */
export async function getAllProvidersWithCustomModels(
	userId: string,
	databaseConfigId: string
): Promise<{ data?: ProviderMetadata[]; err?: string }> {
	try {
		// Get static providers
		const providers = await ProviderRegistry.getAvailableProviders();

		// Get all custom models grouped by provider
		const { data: customModelsByProvider, err } = await getCustomModelsGroupedByProvider(
			userId,
			databaseConfigId
		);

		if (err) {
			return { err };
		}

		// Merge custom models into each provider
		const providersWithCustomModels = providers.map((provider) => {
			const customModels = customModelsByProvider?.[provider.providerId] || [];

			// Transform custom models to ModelMetadata format
			const customModelsFormatted = customModels.map((model) => ({
				id: model.model_id, // Use model_id as id for consistency
				displayName: model.displayName,
				contextWindow: model.contextWindow,
				inputPricePerMToken: model.inputPricePerMToken,
				outputPricePerMToken: model.outputPricePerMToken,
				capabilities: model.capabilities,
			}));

			return {
				...provider,
				supportedModels: [
					...provider.supportedModels,
					...customModelsFormatted,
				],
			};
		});

		return { data: providersWithCustomModels };
	} catch (error) {
		console.error("Error getting providers with custom models:", error);
		return { err: "Failed to load providers" };
	}
}

/**
 * Get a specific provider by ID with custom models merged in
 */
export async function getProviderByIdWithCustomModels(
	providerId: string,
	userId: string,
	databaseConfigId: string
): Promise<{ data?: ProviderMetadata; err?: string }> {
	try {
		// Get static provider
		const provider = await ProviderRegistry.getProviderById(providerId);
		if (!provider) {
			return { err: "Provider not found" };
		}

		// Get custom models for this provider
		const { data: customModels, err } = await getCustomModelsForProvider(
			userId,
			databaseConfigId,
			providerId
		);

		if (err) {
			return { err };
		}

		// Merge custom models with static models
		return {
			data: {
				...provider,
				supportedModels: [
					...provider.supportedModels,
					...(customModels || []),
				],
			},
		};
	} catch (error) {
		console.error("Error getting provider by ID with custom models:", error);
		return { err: "Failed to load provider" };
	}
}

/**
 * Search providers by query with custom models merged in
 */
export async function searchProvidersWithCustomModels(
	query: string,
	userId: string,
	databaseConfigId: string
): Promise<{ data?: ProviderMetadata[]; err?: string }> {
	try {
		// Search static providers
		const providers = await ProviderRegistry.searchProviders(query);

		// Get all custom models grouped by provider
		const { data: customModelsByProvider, err } = await getCustomModelsGroupedByProvider(
			userId,
			databaseConfigId
		);

		if (err) {
			return { err };
		}

		// Merge custom models into each provider
		const providersWithCustomModels = providers.map((provider) => {
			const customModels = customModelsByProvider?.[provider.providerId] || [];

			// Transform custom models to ModelMetadata format
			const customModelsFormatted = customModels.map((model) => ({
				id: model.model_id,
				displayName: model.displayName,
				contextWindow: model.contextWindow,
				inputPricePerMToken: model.inputPricePerMToken,
				outputPricePerMToken: model.outputPricePerMToken,
				capabilities: model.capabilities,
			}));

			return {
				...provider,
				supportedModels: [
					...provider.supportedModels,
					...customModelsFormatted,
				],
			};
		});

		return { data: providersWithCustomModels };
	} catch (error) {
		console.error("Error searching providers with custom models:", error);
		return { err: "Failed to search providers" };
	}
}
