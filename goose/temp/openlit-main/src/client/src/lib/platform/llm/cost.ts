import {
	getFilterPreviousParams,
	getFilterWhereCondition,
} from "@/helpers/server/platform";
import { MetricParams, dataCollector, OTEL_TRACES_TABLE_NAME } from "../common";
import { getTraceMappingKeyFullPath } from "@/helpers/server/trace";

export async function getTotalCost(params: MetricParams) {
	const keyPath = `SpanAttributes['${getTraceMappingKeyFullPath("cost")}']`;
	const currentWhereParams = { ...params, notEmpty: [{ key: keyPath }] };
	const previousWhereParams = getFilterPreviousParams(currentWhereParams);

	const commonQuery = (parameters: any) => `
		SELECT
			sum(toFloat64OrZero(${keyPath})) AS total_usage_cost,
			'${params.timeLimit.start}' as start_date
		FROM ${OTEL_TRACES_TABLE_NAME} 
		WHERE ${getFilterWhereCondition({ ...parameters, operationType: "llm" })}
	`;

	const query = `
		SELECT
			CAST(current_data.total_usage_cost AS FLOAT) AS total_usage_cost,
			CAST(previous_day.total_usage_cost AS FLOAT) AS previous_total_usage_cost
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

	return dataCollector({ query });
}

export async function getAverageCost(params: MetricParams) {
	const keyPath = `SpanAttributes['${getTraceMappingKeyFullPath("cost")}']`;

	const currentWhereParams = { ...params, notEmpty: [{ key: keyPath }] };
	const previousWhereParams = getFilterPreviousParams(currentWhereParams);

	const commonQuery = (parameters: any) => `
		SELECT
			avg(toFloat64OrZero(${keyPath})) AS average_usage_cost,
			'${params.timeLimit.start}' as start_date
		FROM ${OTEL_TRACES_TABLE_NAME} 
		WHERE ${getFilterWhereCondition({ ...parameters, operationType: "llm" })}
	`;

	const query = `
		SELECT
			CAST(current_data.average_usage_cost AS FLOAT) AS average_usage_cost,
			CAST(previous_day.average_usage_cost AS FLOAT) AS previous_average_usage_cost
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

	return dataCollector({ query });
}

export async function getCostByApplication(params: MetricParams) {
	const keyPathApplicationName = `ResourceAttributes['${getTraceMappingKeyFullPath(
		"applicationName"
	)}']`;
	const keyPathCost = `SpanAttributes['${getTraceMappingKeyFullPath("cost")}']`;
	const query = `SELECT
		DISTINCT ${keyPathApplicationName} As applicationName,
			SUM(toFloat64OrZero(${keyPathCost})) AS cost
		FROM ${OTEL_TRACES_TABLE_NAME}
		WHERE ${getFilterWhereCondition({
			...params,
			notEmpty: [{ key: keyPathApplicationName }, { key: keyPathCost }],
			operationType: "llm",
		})}
		GROUP BY applicationName;`;

	return dataCollector({ query });
}

export async function getCostByEnvironment(params: MetricParams) {
	const keyPathEnvironment = `ResourceAttributes['${getTraceMappingKeyFullPath(
		"environment"
	)}']`;
	const keyPathCost = `SpanAttributes['${getTraceMappingKeyFullPath("cost")}']`;
	const query = `SELECT 
			DISTINCT ${keyPathEnvironment} as environment, 
			SUM(toFloat64OrZero(${keyPathCost})) AS cost
		FROM ${OTEL_TRACES_TABLE_NAME} 
		WHERE ${getFilterWhereCondition({
			...params,
			notEmpty: [{ key: keyPathEnvironment }, { key: keyPathCost }],
			operationType: "llm",
		})}
		GROUP BY environment`;

	return dataCollector({ query });
}
