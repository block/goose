import { getFilterWhereCondition } from "@/helpers/server/platform";
import { MetricParams, dataCollector, OTEL_TRACES_TABLE_NAME } from "../common";
import { getTraceMappingKeyFullPath } from "@/helpers/server/trace";

export async function getResultGenerationByEndpoint(params: MetricParams) {
	const keyPath = `SpanAttributes['${getTraceMappingKeyFullPath("provider")}']`;
	const query = `
    SELECT 
      ${keyPath} AS provider,
      CAST(COUNT(*) AS INTEGER) AS count
    FROM
        ${OTEL_TRACES_TABLE_NAME}
    WHERE ${getFilterWhereCondition({
			...params,
			notEmpty: [{ key: keyPath }],
			operationType: "llm",
		})}
    GROUP BY provider;
  `;

	return dataCollector({ query });
}
