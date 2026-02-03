import { getTraceMappingKeyFullPath } from "@/helpers/server/trace";
import { MetricParams, OTEL_TRACES_TABLE_NAME, dataCollector } from "../common";
import { getFilterWhereCondition } from "@/helpers/server/platform";

export async function getResultGenerationByApplication(params: MetricParams) {
	const key = `ResourceAttributes['${getTraceMappingKeyFullPath(
		"applicationName"
	)}']`;
	const query = `SELECT 
			DISTINCT ${key} as applicationName, 
      CAST(COUNT(*) AS INTEGER) AS count
		FROM ${OTEL_TRACES_TABLE_NAME} 
		WHERE ${getFilterWhereCondition({
			...params,
			notEmpty: [{ key: key }],
			operationType: "vectordb",
		})}
		GROUP BY applicationName`;

	return dataCollector({ query });
}
