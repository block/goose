import {
	dateTruncGroupingLogic,
	getFilterWhereConditionForGPU,
} from "@/helpers/server/platform";
import {
	GPUMetricParams,
	OTEL_GPUS_TABLE_NAME,
	dataCollector,
} from "../common";

export async function getAverageUtilization(params: GPUMetricParams) {
	const query = `
			SELECT
				ROUND(AVG(Value), 2) as utilization
			FROM
					${OTEL_GPUS_TABLE_NAME}
			WHERE ${getFilterWhereConditionForGPU(
				params
			)} AND MetricName = 'gpu.utilization'
			GROUP BY
				MetricName
  `;

	return dataCollector({ query });
}

export async function getUtilizationParamsPerTime(params: GPUMetricParams) {
	const keys = ["utilization", "enc.utilization", "dec.utilization"];
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
							`ROUND(AVG(IF(MetricName = 'gpu.${type}', Value, 0)), 2) AS ${type.replaceAll(
								".",
								"_"
							)},`
					)
					.join(" ")}
				formatDateTime(DATE_TRUNC('${dateTrunc}', TimeUnix), '%Y/%m/%d %R') AS request_time
			FROM
					${OTEL_GPUS_TABLE_NAME}
			WHERE ${getFilterWhereConditionForGPU({
				...params,
			})}
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
