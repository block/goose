import { getTraceMappingKeyFullPath } from "@/helpers/server/trace";
import { MetricParams, dataCollector, OTEL_TRACES_TABLE_NAME } from "../common";
import { getFilterWhereCondition } from "@/helpers/server/platform";

export async function getResultGenerationByOperation(params: MetricParams) {
	const keyPath = `SpanAttributes['${getTraceMappingKeyFullPath("operation")}']`;
	const query = `SELECT 
    ${keyPath} AS operation,
    CAST(COUNT(*) AS INTEGER) AS count
  FROM ${OTEL_TRACES_TABLE_NAME}
  WHERE ${getFilterWhereCondition({
		...params,
		notEmpty: [{ key: keyPath }],
		operationType: "vectordb",
	})}
  GROUP BY operation;`;

	return dataCollector({ query });
}
