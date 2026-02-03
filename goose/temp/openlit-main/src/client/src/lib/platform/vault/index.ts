import getMessage from "@/constants/messages";
import {
	SecretGetFilters,
	SecretGetFiltersWithApiKey,
	SecretInput,
} from "@/types/vault";
import { normalizeSecretDataForSDK, verifySecretInput } from "@/helpers/server/vault";
import { getCurrentUser } from "@/lib/session";
import { throwIfError } from "@/utils/error";
import Sanitizer from "@/utils/sanitizer";
import { OPENLIT_VAULT_TABLE_NAME } from "./table-details";
import { dataCollector } from "../common";
import { jsonStringify } from "@/utils/json";
import { getAPIKeyInfo } from "../api-keys";

export async function getSecretByName({ key }: { key: string }) {
	const query = `
		SELECT * FROM ${OPENLIT_VAULT_TABLE_NAME} WHERE key='${Sanitizer.sanitizeValue(
		key
	)}';
  `;

	const { data }: { data?: any } = await dataCollector({ query });
	return data?.[0];
}

export async function checkNameValidity({ key }: { key: string }) {
	const data = await getSecretByName({ key });
	return { isValid: !data?.id };
}

export async function upsertSecret(secretInputParams: Partial<SecretInput>) {
	const user = await getCurrentUser();

	throwIfError(!user, getMessage().UNAUTHORIZED_USER);

	const secretInput = Sanitizer.sanitizeObject(secretInputParams);

	const verifiedSecretObj = verifySecretInput(secretInput);
	throwIfError(!verifiedSecretObj.success, verifiedSecretObj.err!);

	if (!secretInputParams.id) {
		const { isValid } = await checkNameValidity({ key: secretInput.key || "" });
		throwIfError(!isValid, getMessage().SECRET_NAME_TAKEN);
	}

	if (secretInputParams.id) {
		const updateValues = [
			`updated_by = '${user!.email}'`,
			secretInput.key && "key = '" + secretInput.key + "'",
			secretInput.value && "value = '" + secretInput.value + "'",
			secretInput.tags && "tags = '" + jsonStringify(secretInput.tags) + "'",
		];
		const updateQuery = `
      ALTER TABLE ${OPENLIT_VAULT_TABLE_NAME}
      UPDATE 
        ${updateValues.filter((e) => e).join(" , ")}
      WHERE id = '${secretInput.id}'`;
		const { err, data } = await dataCollector(
			{
				query: updateQuery,
			},
			"exec"
		);
		throwIfError(
			!!(err || !(data as { query_id: unknown })?.query_id),
			typeof err?.toString === "function"
				? err.toString()
				: (err as string) || getMessage().SECRET_NOT_SAVED
		);

		return getMessage().SECRET_SAVED;
	} else {
		const { err } = await dataCollector(
			{
				table: OPENLIT_VAULT_TABLE_NAME,
				values: [
					{
						key: secretInput.key,
						value: secretInput.value,
						tags: secretInput.tags,
						created_by: user!.email,
					},
				],
			},
			"insert"
		);

		throwIfError(
			!!err,
			typeof err?.toString === "function" ? err.toString() : (err as string)
		);
	}

	return {
		data: {},
		message: getMessage().SECRET_SAVED,
	};
}

export async function deleteSecret(secretIdParam: string) {
	const user = await getCurrentUser();

	throwIfError(!user, getMessage().UNAUTHORIZED_USER);
	const secretId = Sanitizer.sanitizeValue(secretIdParam);

	const { err } = await dataCollector(
		{
			query: `DELETE FROM ${OPENLIT_VAULT_TABLE_NAME} WHERE id = '${secretId}';`,
		},
		"exec"
	);

	if (err) {
		return [getMessage().SECRET_NOT_DELETED];
	}

	return [undefined, getMessage().SECRET_DELETED];
}

export async function getSecrets(
	filters: SecretGetFilters,
	{ selectValue }: { selectValue?: boolean } = {}
) {
	if (!filters.databaseConfigId) {
		const user = await getCurrentUser();
		throwIfError(!user, getMessage().UNAUTHORIZED_USER);
	}

	const filteredConditions: string[] = [
		filters.key ? "v.key = '" + filters.key + "'" : "",
		filters.tags && filters.tags.length > 0
			? `hasAny(JSONExtractArrayRaw(v.tags), ['"` +
			  filters.tags.join(`"' , '"`) +
			  `"'])`
			: "",
	].filter((e) => !!e);

	const query = `SELECT * ${!!selectValue ? "" : "EXCEPT value"}
		FROM
			${OPENLIT_VAULT_TABLE_NAME} v
		${
			filteredConditions.length > 0
				? `WHERE ${filteredConditions.join(" AND ")}`
				: ""
		} 
		ORDER BY
			v.created_at DESC;
	`;

	return await dataCollector({ query }, "query", filters.databaseConfigId);
}

export async function getSecretsFromDatabaseId(
	filters: SecretGetFiltersWithApiKey
) {
	const [err, apiInfo] = await getAPIKeyInfo({
		apiKey: filters.apiKey,
	});

	throwIfError(
		!!(err || !apiInfo?.databaseConfigId),
		err || getMessage().NO_API_KEY
	);

	const { err: secretErr, data: secretData } = await getSecrets(
		{
			...filters,
			databaseConfigId: apiInfo.databaseConfigId,
		},
		{ selectValue: true }
	);

	throwIfError(
		!!(secretErr || !secretData),
		(secretErr as any) || getMessage().NO_PROMPT
	);

	return normalizeSecretDataForSDK(secretData as any[]);
}

export async function getSecretById(
	id: string,
	databaseConfigId?: string,
	excludeVaultValue: boolean = true
) {
	const query = `SELECT * ${
		!!excludeVaultValue ? "EXCEPT value" : ""
	} FROM ${OPENLIT_VAULT_TABLE_NAME} v WHERE v.id = '${id}';`;

	return await dataCollector({ query }, "query", databaseConfigId);
}
