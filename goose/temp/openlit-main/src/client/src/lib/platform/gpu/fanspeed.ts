import {
	dateTruncGroupingLogic,
	getFilterWhereConditionForGPU,
} from "@/helpers/server/platform";
import {
	dataCollector,
	OTEL_GPUS_TABLE_NAME,
	GPUMetricParams,
} from "../common";

export async function getFanspeedParamsPerTime(params: GPUMetricParams) {
	const { start, end } = params.timeLimit;
	const dateTrunc = dateTruncGroupingLogic(end as Date, start as Date);

	const query = `
		SELECT
				ROUND(AVG(Value), 2) AS fan_speed,
				formatDateTime(DATE_TRUNC('${dateTrunc}', TimeUnix), '%Y/%m/%d %R') AS request_time
			FROM
					${OTEL_GPUS_TABLE_NAME}
			WHERE ${getFilterWhereConditionForGPU(params)} AND MetricName = 'gpu.fan_speed'
			GROUP BY
				request_time,
				MetricName
			ORDER BY
				request_time;
  `;

	return dataCollector({ query });
}
