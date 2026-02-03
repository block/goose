import { getTraceMappingKeyFullPath } from "@/helpers/server/trace";
import { MetricParams, dataCollector, OTEL_TRACES_TABLE_NAME } from "../common";
import { getFilterWhereCondition } from "@/helpers/server/platform";

export async function getResultGenerationBySystem(params: MetricParams) {
	const keyPath = `SpanAttributes['${getTraceMappingKeyFullPath("system")}']`;
	const query = `SELECT 
    ${keyPath} AS system,
    CAST(COUNT(*) AS INTEGER) AS count
  FROM ${OTEL_TRACES_TABLE_NAME}
  WHERE ${getFilterWhereCondition({
		...params,
		notEmpty: [{ key: keyPath }],
		operationType: "vectordb",
	})}
  GROUP BY system;`;

	return dataCollector({ query });
}
