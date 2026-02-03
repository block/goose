import {
	dateTruncGroupingLogic,
	getFilterWhereConditionForGPU,
} from "@/helpers/server/platform";
import {
	dataCollector,
	OTEL_GPUS_TABLE_NAME,
	GPUMetricParams,
} from "../common";

export async function getMemoryParamsPerTime(params: GPUMetricParams) {
	const keys = [
		"memory.available",
		"memory.total",
		"memory.used",
		"memory.free",
	];

	const { start, end } = params.timeLimit;
	const dateTrunc = dateTruncGroupingLogic(end as Date, start as Date);

	const query = `
		SELECT
			${keys
				.map(
					(type) =>
						`MAX(${type.replaceAll(".", "_")}) AS ${type.replaceAll(".", "_")},`
				)
				.join(" ")}
			request_time
		FROM (
			SELECT
				${keys
					.map(
						(type) =>
							`SUM(IF(MetricName = 'gpu.${type}', Value, 0)) AS ${type.replaceAll(
								".",
								"_"
							)},`
					)
					.join(" ")}
				formatDateTime(DATE_TRUNC('${dateTrunc}', TimeUnix), '%Y/%m/%d %R') AS request_time
			FROM
					${OTEL_GPUS_TABLE_NAME}
			WHERE ${getFilterWhereConditionForGPU(params)}
			GROUP BY
				request_time,
				MetricName
			ORDER BY
				request_time
		)
		GROUP BY 
			request_time 
		ORDER BY 
			request_time;
  `;

	return dataCollector({ query });
}

export async function getAverageMemoryUsage(params: GPUMetricParams) {
	const query = `
			SELECT
				ROUND(AVG(Value), 2) as memory_used
			FROM
					${OTEL_GPUS_TABLE_NAME}
			WHERE ${getFilterWhereConditionForGPU(
				params
			)} AND MetricName = 'gpu.memory.used'
			GROUP BY
				MetricName
  `;

	return dataCollector({ query });
}
