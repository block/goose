import { PromptDownloadInput, PromptUpdate } from "@/constants/prompts";
import { dataCollector } from "../common";
import {
	OPENLIT_PROMPT_VERSIONS_TABLE_NAME,
	OPENLIT_PROMPT_VERSION_DOWNLOADS_TABLE_NAME,
} from "./table-details";
import { consoleLog } from "@/utils/log";
import { getCurrentUser } from "@/lib/session";
import { jsonStringify } from "@/utils/json";
import getMessage from "@/constants/messages";
import Sanitizer from "@/utils/sanitizer";
import { throwIfError } from "@/utils/error";

export async function upsertPromptVersion(promptInputParams: PromptUpdate) {
	const user = await getCurrentUser();

	throwIfError(!user, getMessage().UNAUTHORIZED_USER);

	const promptInput = Sanitizer.sanitizeObject(promptInputParams);

	let versionErr;
	let versionData;

	if (promptInput.versionId) {
		const updateValues = [
			`updated_by = '${user!.email}'`,
			promptInput.version && "version = '" + promptInput.version + "'",
			promptInput.status && "status = '" + promptInput.status + "'",
			promptInput.prompt && "prompt = '" + promptInput.prompt + "'",
			promptInput.tags && "tags = '" + jsonStringify(promptInput.tags) + "'",
			promptInput.metaProperties &&
				"meta_properties = '" + jsonStringify(promptInput.metaProperties) + "'",
		];
		const updateQuery = `
      ALTER TABLE ${OPENLIT_PROMPT_VERSIONS_TABLE_NAME}
      UPDATE 
        ${updateValues.filter((e) => e).join(" , ")}
      WHERE version_id = '${promptInput.versionId}'`;
		const { err, data } = await dataCollector(
			{
				query: updateQuery,
			},
			"exec"
		);

		versionErr = err;
		versionData = data;
	} else {
		const { err, data } = await dataCollector(
			{
				table: OPENLIT_PROMPT_VERSIONS_TABLE_NAME,
				values: [
					{
						prompt_id: promptInput.promptId,
						updated_by: user!.email,
						version: promptInput.version,
						status: promptInput.status,
						prompt: promptInput.prompt,
						tags: jsonStringify(promptInput.tags),
						meta_properties: jsonStringify(promptInput.metaProperties),
					},
				],
			},
			"insert"
		);
		versionErr = err;
		versionData = data;
	}

	throwIfError(
		!!(versionErr || !(versionData as { query_id: unknown })?.query_id),
		typeof versionErr?.toString === "function"
			? versionErr.toString()
			: (versionErr as string) || getMessage().VERSION_NOT_SAVED
	);

	return getMessage().VERSION_SAVED;
}

export async function updateDownloadDetails(
	promptDownloadInputParams: PromptDownloadInput,
	dbConfigId?: string
) {
	const promptDownloadInput = Sanitizer.sanitizeObject(
		promptDownloadInputParams
	);

	const { err, data } = await dataCollector(
		{
			table: OPENLIT_PROMPT_VERSION_DOWNLOADS_TABLE_NAME,
			values: [
				{
					prompt_id: promptDownloadInput.promptId,
					version_id: promptDownloadInput.versionId,
					meta_properties: jsonStringify(promptDownloadInput.metaProperties),
					download_source: promptDownloadInput.downloadSource,
				},
			],
		},
		"insert",
		dbConfigId
	);

	if (err || !(data as any)?.query_id)
		consoleLog(err || getMessage().DOWNLOAD_INFO_NOT_SAVED);
	return true;
}
