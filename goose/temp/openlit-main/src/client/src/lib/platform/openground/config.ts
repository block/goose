import { dataCollector } from "@/lib/platform/common";
import getMessage from "@/constants/messages";
import Sanitizer from "@/utils/sanitizer";
import { OPENLIT_VAULT_TABLE_NAME } from "@/lib/platform/vault/table-details";
import { OPENLIT_OPENGROUND_CONFIG_TABLE_NAME } from "./table-details";

export interface OpenGroundConfigData {
	id: string;
	userId: string;
	databaseConfigId: string;
	provider: string;
	vaultId: string;
	modelId?: string;
	isActive: boolean;
	createdAt: Date;
	updatedAt: Date;
}

export interface OpenGroundConfigWithSecret extends OpenGroundConfigData {
	apiKey: string;
	vaultKey: string;
}

/**
 * Get all OpenGround configurations for a user
 */
export async function getOpenGroundConfigs(
	userId: string,
	databaseConfigId: string
): Promise<{ data?: OpenGroundConfigData[]; err?: string }> {
	try {
		const query = `
      SELECT
        id,
        user_id as userId,
        database_config_id as databaseConfigId,
        provider,
        vault_id as vaultId,
        model_id as modelId,
        is_active as isActive,
        created_at as createdAt,
        updated_at as updatedAt
      FROM ${OPENLIT_OPENGROUND_CONFIG_TABLE_NAME}
      WHERE user_id = '${Sanitizer.sanitizeValue(userId)}'
      AND database_config_id = '${Sanitizer.sanitizeValue(databaseConfigId)}'
      ORDER BY created_at DESC
    `;

		const { data, err } = await dataCollector(
			{ query },
			"query",
			databaseConfigId
		);

		if (err) {
			console.error("Error fetching OpenGround configs:", err);
			return { err: getMessage().OPERATION_FAILED };
		}

		return { data: data as OpenGroundConfigData[] };
	} catch (error: any) {
		console.error("Error fetching OpenGround configs:", error);
		return { err: getMessage().OPERATION_FAILED };
	}
}

/**
 * Get a specific OpenGround config with API key from Vault
 */
export async function getOpenGroundConfigWithSecret(
	provider: string,
	userId: string,
	databaseConfigId: string
): Promise<{ data?: OpenGroundConfigWithSecret; err?: string }> {
	try {
		// Get the config from ClickHouse
		const configQuery = `
      SELECT
        id,
        user_id as userId,
        database_config_id as databaseConfigId,
        provider,
        vault_id as vaultId,
        model_id as modelId,
        is_active as isActive,
        created_at as createdAt,
        updated_at as updatedAt
      FROM ${OPENLIT_OPENGROUND_CONFIG_TABLE_NAME}
      WHERE provider = '${Sanitizer.sanitizeValue(provider)}'
      AND user_id = '${Sanitizer.sanitizeValue(userId)}'
      AND database_config_id = '${Sanitizer.sanitizeValue(databaseConfigId)}'
      AND is_active = true
      LIMIT 1
    `;

		const { data: configData, err: configErr } = await dataCollector(
			{ query: configQuery },
			"query",
			databaseConfigId
		);

		if (configErr || !(configData as any[])?.length) {
			return { err: `No active configuration found for provider: ${provider}` };
		}

		const config = (configData as any[])[0];

		// Fetch the API key from Vault (ClickHouse)
		const vaultQuery = `
      SELECT key, value
      FROM ${OPENLIT_VAULT_TABLE_NAME}
      WHERE id = '${Sanitizer.sanitizeValue(config.vaultId)}'
    `;

		const { data: vaultData, err: vaultErr } = await dataCollector(
			{ query: vaultQuery },
			"query",
			databaseConfigId
		);

		if (vaultErr || !(vaultData as any[])?.length) {
			return {
				err: `API key not found in Vault for provider: ${provider}`,
			};
		}

		const vaultRecord = (vaultData as any[])[0];

		return {
			data: {
				...config,
				apiKey: vaultRecord.value,
				vaultKey: vaultRecord.key,
			},
		};
	} catch (error: any) {
		console.error("Error fetching OpenGround config with secret:", error);
		return { err: getMessage().OPERATION_FAILED };
	}
}

/**
 * Create or update an OpenGround configuration
 */
export async function upsertOpenGroundConfig(data: {
	provider: string;
	vaultId: string;
	modelId?: string;
	userId: string;
	databaseConfigId: string;
	isActive?: boolean;
}): Promise<{ data?: OpenGroundConfigData; err?: string }> {
	try {
		// Check if config exists
		const checkQuery = `
      SELECT id
      FROM ${OPENLIT_OPENGROUND_CONFIG_TABLE_NAME}
      WHERE user_id = '${Sanitizer.sanitizeValue(data.userId)}'
      AND database_config_id = '${Sanitizer.sanitizeValue(data.databaseConfigId)}'
      AND provider = '${Sanitizer.sanitizeValue(data.provider)}'
      LIMIT 1
    `;

		const { data: existingData } = await dataCollector(
			{ query: checkQuery },
			"query",
			data.databaseConfigId
		);

		const exists = (existingData as any[])?.length > 0;

		if (exists) {
			// Update existing config
			const configId = (existingData as any[])[0].id;
			const updateQuery = `
        ALTER TABLE ${OPENLIT_OPENGROUND_CONFIG_TABLE_NAME}
        UPDATE
          vault_id = '${Sanitizer.sanitizeValue(data.vaultId)}',
          model_id = ${data.modelId ? `'${Sanitizer.sanitizeValue(data.modelId)}'` : "NULL"},
          is_active = ${data.isActive ?? true},
          updated_at = now()
        WHERE id = '${Sanitizer.sanitizeValue(configId)}'
      `;

			const { err: updateErr } = await dataCollector(
				{ query: updateQuery },
				"exec",
				data.databaseConfigId
			);

			if (updateErr) {
				console.error("Error updating OpenGround config:", updateErr);
				return { err: getMessage().OPERATION_FAILED };
			}

			// Fetch and return the updated config
			return getOpenGroundConfigById(configId, data.databaseConfigId);
		} else {
			// Insert new config
			const { err: insertErr } = await dataCollector(
				{
					table: OPENLIT_OPENGROUND_CONFIG_TABLE_NAME,
					values: [
						{
							user_id: data.userId,
							database_config_id: data.databaseConfigId,
							provider: data.provider,
							vault_id: data.vaultId,
							model_id: data.modelId || null,
							is_active: data.isActive ?? true,
						},
					],
				},
				"insert",
				data.databaseConfigId
			);

			if (insertErr) {
				console.error("Error inserting OpenGround config:", insertErr);
				return { err: getMessage().OPERATION_FAILED };
			}

			// Get the inserted config
			const { data: lastInsert } = await dataCollector(
				{
					query: `
            SELECT
              id,
              user_id as userId,
              database_config_id as databaseConfigId,
              provider,
              vault_id as vaultId,
              model_id as modelId,
              is_active as isActive,
              created_at as createdAt,
              updated_at as updatedAt
            FROM ${OPENLIT_OPENGROUND_CONFIG_TABLE_NAME}
            WHERE user_id = '${Sanitizer.sanitizeValue(data.userId)}'
            AND provider = '${Sanitizer.sanitizeValue(data.provider)}'
            ORDER BY created_at DESC
            LIMIT 1
          `,
				},
				"query",
				data.databaseConfigId
			);

			const config = (lastInsert as any[])?.[0];
			return { data: config };
		}
	} catch (error: any) {
		console.error("Error upserting OpenGround config:", error);
		return { err: getMessage().OPERATION_FAILED };
	}
}

/**
 * Get config by ID
 */
async function getOpenGroundConfigById(
	configId: string,
	databaseConfigId: string
): Promise<{ data?: OpenGroundConfigData; err?: string }> {
	const query = `
    SELECT
      id,
      user_id as userId,
      database_config_id as databaseConfigId,
      provider,
      vault_id as vaultId,
      model_id as modelId,
      is_active as isActive,
      created_at as createdAt,
      updated_at as updatedAt
    FROM ${OPENLIT_OPENGROUND_CONFIG_TABLE_NAME}
    WHERE id = '${Sanitizer.sanitizeValue(configId)}'
    LIMIT 1
  `;

	const { data, err } = await dataCollector(
		{ query },
		"query",
		databaseConfigId
	);

	if (err || !(data as any[])?.length) {
		return { err: "Config not found" };
	}

	return { data: (data as any[])[0] };
}

/**
 * Delete an OpenGround configuration
 */
export async function deleteOpenGroundConfig(
	configId: string,
	userId: string,
	databaseConfigId: string
): Promise<{ data?: string; err?: string }> {
	try {
		const query = `
      ALTER TABLE ${OPENLIT_OPENGROUND_CONFIG_TABLE_NAME}
      DELETE WHERE id = '${Sanitizer.sanitizeValue(configId)}'
      AND user_id = '${Sanitizer.sanitizeValue(userId)}'
    `;

		const { err } = await dataCollector(
			{ query },
			"exec",
			databaseConfigId
		);

		if (err) {
			console.error("Error deleting OpenGround config:", err);
			return { err: getMessage().OPERATION_FAILED };
		}

		return { data: "Configuration deleted successfully" };
	} catch (error: any) {
		console.error("Error deleting OpenGround config:", error);
		return { err: getMessage().OPERATION_FAILED };
	}
}

/**
 * Toggle active status of a configuration
 */
export async function toggleOpenGroundConfigStatus(
	configId: string,
	userId: string,
	databaseConfigId: string,
	isActive: boolean
): Promise<{ data?: OpenGroundConfigData; err?: string }> {
	try {
		const query = `
      ALTER TABLE ${OPENLIT_OPENGROUND_CONFIG_TABLE_NAME}
      UPDATE
        is_active = ${isActive},
        updated_at = now()
      WHERE id = '${Sanitizer.sanitizeValue(configId)}'
      AND user_id = '${Sanitizer.sanitizeValue(userId)}'
    `;

		const { err } = await dataCollector({ query }, "exec", databaseConfigId);

		if (err) {
			console.error("Error toggling OpenGround config status:", err);
			return { err: getMessage().OPERATION_FAILED };
		}

		// Fetch and return the updated config
		return getOpenGroundConfigById(configId, databaseConfigId);
	} catch (error: any) {
		console.error("Error toggling OpenGround config status:", error);
		return { err: getMessage().OPERATION_FAILED };
	}
}

/**
 * Get all active providers for a user (providers with active configurations)
 */
export async function getActiveProviders(
	userId: string,
	databaseConfigId: string
): Promise<{ data?: string[]; err?: string }> {
	try {
		const query = `
      SELECT DISTINCT provider
      FROM ${OPENLIT_OPENGROUND_CONFIG_TABLE_NAME}
      WHERE user_id = '${Sanitizer.sanitizeValue(userId)}'
      AND database_config_id = '${Sanitizer.sanitizeValue(databaseConfigId)}'
      AND is_active = true
    `;

		const { data, err } = await dataCollector(
			{ query },
			"query",
			databaseConfigId
		);

		if (err) {
			console.error("Error fetching active providers:", err);
			return { err: getMessage().OPERATION_FAILED };
		}

		const providers = (data as any[])?.map((row) => row.provider) || [];
		return { data: providers };
	} catch (error: any) {
		console.error("Error fetching active providers:", error);
		return { err: getMessage().OPERATION_FAILED };
	}
}
