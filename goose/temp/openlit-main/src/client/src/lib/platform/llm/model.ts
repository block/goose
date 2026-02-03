import { MetricParams, dataCollector, OTEL_TRACES_TABLE_NAME } from "../common";
import { getTraceMappingKeyFullPath } from "@/helpers/server/trace";
import {
	dateTruncGroupingLogic,
	getFilterWhereCondition,
} from "@/helpers/server/platform";

export type ModelMetricParams = MetricParams & {
	top: number;
};

export async function getTopModels(params: ModelMetricParams) {
	const keyPath = `SpanAttributes['${getTraceMappingKeyFullPath("model")}']`;
	const query = `SELECT
			${keyPath} AS model,
			COUNT(${keyPath}) AS model_count,
			COUNT(*) AS total
		FROM
			${OTEL_TRACES_TABLE_NAME}
		WHERE ${getFilterWhereCondition({
			...params,
			notEmpty: [{ key: keyPath }],
			operationType: "llm",
		})}
		GROUP BY
			model
		ORDER BY
			model_count DESC
		LIMIT ${params.top};
	`;

	return dataCollector({ query });
}

export async function getModelsPerTime(params: MetricParams) {
	const { start, end } = params.timeLimit;
	const dateTrunc = dateTruncGroupingLogic(end as Date, start as Date);

	const keyPath = `SpanAttributes['${getTraceMappingKeyFullPath("model")}']`;
	const query = `
		SELECT 
			ARRAY_AGG(model) as models,
			ARRAY_AGG(model_count) as model_counts,
			CAST(SUM(model_count) AS INTEGER) as total_model_count,
			request_time as request_time
		FROM
			(SELECT
					${keyPath} as model,
					CAST(COUNT(*) AS INTEGER) AS model_count,
					formatDateTime(DATE_TRUNC('${dateTrunc}', Timestamp), '%Y/%m/%d') AS request_time
				FROM
					${OTEL_TRACES_TABLE_NAME}
				WHERE ${getFilterWhereCondition({
					...params,
					notEmpty: [{ key: keyPath }],
					operationType: "llm",
				})}
				GROUP BY
					model, request_time
				ORDER BY
					request_time)
		GROUP BY 
			request_time
		ORDER BY
			request_time;
	`;

	return dataCollector({ query });
}
