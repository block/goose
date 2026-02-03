import { MetricParams, dataCollector, OTEL_TRACES_TABLE_NAME } from "../common";
import { getTraceMappingKeyFullPath } from "@/helpers/server/trace";
import {
	dateTruncGroupingLogic,
	getFilterPreviousParams,
	getFilterWhereCondition,
} from "@/helpers/server/platform";

export type TOKEN_TYPE = "total" | "prompt" | "completion";

export type TokenParams = MetricParams & {
	type: TOKEN_TYPE;
};

export async function getAverageTokensPerRequest(params: TokenParams) {
	const keyPath = `SpanAttributes['${getTraceMappingKeyFullPath(
		params.type === "total"
			? "totalTokens"
			: params.type === "prompt"
			? "promptTokens"
			: "completionTokens"
	)}']`;

	const currentWhereParams = { ...params, notEmpty: [{ key: keyPath }] };

	let query;

	const commonQuery = (parameters: any) => `SELECT
			AVG(toInt32OrZero(${keyPath})) AS total_tokens
			${params.type === "total" ? `, '${params.timeLimit.start}' as start_date` : ""}
			FROM ${OTEL_TRACES_TABLE_NAME} 
			WHERE ${getFilterWhereCondition({ ...parameters, operationType: "llm" })}`;

	if (params.type === "total") {
		const previousWhereParams = getFilterPreviousParams(currentWhereParams);
		query = `
			SELECT
				CAST(current_data.total_tokens AS FLOAT) AS total_tokens,
				CAST(previous_day.total_tokens AS FLOAT) AS previous_total_tokens
			FROM
				(
					${commonQuery(currentWhereParams)}
				) as current_data
				JOIN
				(
					${commonQuery(previousWhereParams)}
				) as previous_day
			ON
				current_data.start_date = previous_day.start_date;
		`;
	} else {
		query = commonQuery(currentWhereParams);
	}

	return dataCollector({ query });
}

export async function getTokensPerTime(params: MetricParams) {
	const { start, end } = params.timeLimit;
	const dateTrunc = dateTruncGroupingLogic(end as Date, start as Date);

	const keyPaths: { key: string }[] = [
		{
			key: `SpanAttributes['${getTraceMappingKeyFullPath("totalTokens")}']`,
		},
		{
			key: `SpanAttributes['${getTraceMappingKeyFullPath("promptTokens")}']`,
		},
		{
			key: `SpanAttributes['${getTraceMappingKeyFullPath(
				"completionTokens"
			)}']`,
		},
	];

	const query = `SELECT
		SUM(toInt64OrZero(${keyPaths[0].key})) AS totaltokens,
		SUM(toInt64OrZero(${keyPaths[1].key})) AS prompttokens,
		SUM(toInt64OrZero(${keyPaths[2].key})) AS completiontokens,
		formatDateTime(DATE_TRUNC('${dateTrunc}', Timestamp), '%Y/%m/%d %R') AS request_time
		FROM ${OTEL_TRACES_TABLE_NAME} 
		WHERE ${getFilterWhereCondition({
			...params,
			notEmpty: keyPaths,
			operationType: "llm",
		})}
		GROUP BY request_time
		ORDER BY request_time`;

	return dataCollector({ query });
}
