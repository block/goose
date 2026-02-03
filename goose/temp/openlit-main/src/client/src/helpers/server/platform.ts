import { ValueOf } from "../../types/util";
import { MetricParams } from "@/lib/platform/common";
import {
	addDays,
	addMonths,
	differenceInDays,
	differenceInYears,
} from "date-fns";
import { getTraceMappingKeyFullPath } from "../server/trace";
import { FilterWhereConditionType } from "@/types/platform";

export const validateMetricsRequestType = {
	// Request
	REQUEST_PER_TIME: "REQUEST_PER_TIME",
	TOTAL_REQUESTS: "TOTAL_REQUESTS",
	AVERAGE_REQUEST_DURATION: "AVERAGE_REQUEST_DURATION",
	GET_ALL: "GET_ALL",

	// LLM
	// Cost
	TOTAL_COST: "TOTAL_COST",
	AVERAGE_REQUEST_COST: "AVERAGE_REQUEST_COST",
	COST_BY_APPLICATION: "COST_BY_APPLICATION",
	COST_BY_ENVIRONMENT: "COST_BY_ENVIRONMENT",
	// Model
	TOP_MODELS: "TOP_MODELS",
	MODEL_PER_TIME: "MODEL_PER_TIME",
	// Category
	GENERATION_BY_CATEGORY: "GENERATION_BY_CATEGORY",
	// Token
	AVERAGE_REQUEST_TOKEN: "AVERAGE_REQUEST_TOKEN",
	TOKENS_PER_TIME: "TOKENS_PER_TIME",
	// Endpoint
	GENERATION_BY_ENDPOINT: "GENERATION_BY_ENDPOINT",

	// GPU
	// memory
	AVERAGE_MEMORY_USAGE: "AVERAGE_MEMORY_USAGE",
	MEMORY_PER_TIME: "MEMORY_PER_TIME",
	// temperature
	AVERAGE_TEMPERATURE: "AVERAGE_TEMPERATURE",
	TEMPERATURE_PER_TIME: "TEMPERATURE_PER_TIME",
	// power
	AVERAGE_POWER_DRAW: "AVERAGE_POWER_DRAW",
	POWER_PER_TIME: "POWER_PER_TIME",
	// fanspeed
	FANSPEED_PER_TIME: "FANSPEED_PER_TIME",
	// utilization
	AVERAGE_UTILIZATION: "AVERAGE_UTILIZATION",
	UTILIZATION_PER_TIME: "UTILIZATION_PER_TIME",

	// Vectordb
	GENERATION_BY_OPERATION: "GENERATION_BY_OPERATION",
	// system
	GENERATION_BY_SYSTEM: "GENERATION_BY_SYSTEM",
	// environment
	GENERATION_BY_ENVIRONMENT: "GENERATION_BY_ENVIRONMENT",
	// application
	GENERATION_BY_APPLICATION: "GENERATION_BY_APPLICATION",
	// evaluation
	GET_TOTAL_EVALUATION_DETECTED: "GET_TOTAL_EVALUATION_DETECTED",
};

export const validateMetricsRequest = (
	params: Record<string, any>,
	type: ValueOf<typeof validateMetricsRequestType>
): { success: boolean; err?: string } => {
	switch (type) {
		// Request
		case validateMetricsRequestType.REQUEST_PER_TIME:
		case validateMetricsRequestType.GET_ALL:

		// Cost
		case validateMetricsRequestType.TOTAL_COST:
		case validateMetricsRequestType.AVERAGE_REQUEST_COST:
		case validateMetricsRequestType.COST_BY_APPLICATION:
		case validateMetricsRequestType.COST_BY_ENVIRONMENT:

		// Models
		case validateMetricsRequestType.MODEL_PER_TIME:
		// Category
		case validateMetricsRequestType.GENERATION_BY_CATEGORY:
		// Token
		case validateMetricsRequestType.TOKENS_PER_TIME:
		// Endpoint
		case validateMetricsRequestType.GENERATION_BY_ENDPOINT:

		// GPU
		// memory
		case validateMetricsRequestType.AVERAGE_MEMORY_USAGE:
		case validateMetricsRequestType.MEMORY_PER_TIME:
		// temperature
		case validateMetricsRequestType.AVERAGE_TEMPERATURE:
		case validateMetricsRequestType.TEMPERATURE_PER_TIME:
		// Power
		case validateMetricsRequestType.AVERAGE_POWER_DRAW:
		case validateMetricsRequestType.POWER_PER_TIME:
		// Fan speed
		case validateMetricsRequestType.FANSPEED_PER_TIME:

		// Vectordb
		case validateMetricsRequestType.GENERATION_BY_OPERATION:
		case validateMetricsRequestType.GENERATION_BY_SYSTEM:
		case validateMetricsRequestType.GENERATION_BY_ENVIRONMENT:
		case validateMetricsRequestType.GENERATION_BY_APPLICATION:

		// Evaluation
		case validateMetricsRequestType.GET_TOTAL_EVALUATION_DETECTED:
			if (!params.timeLimit?.start || !params.timeLimit?.end) {
				return {
					success: false,
					err: "Start date or End date missing!",
				};
			}
			break;
		// Model
		case validateMetricsRequestType.TOP_MODELS:
			if (!params.timeLimit?.start || !params.timeLimit?.end) {
				return {
					success: false,
					err: "Start date or End date missing!",
				};
			}
			if (!params.top) {
				return {
					success: false,
					err: "Number of top models missing!",
				};
			}
			break;
		// Token
		case validateMetricsRequestType.AVERAGE_REQUEST_TOKEN:
			if (!params.timeLimit?.start || !params.timeLimit?.end) {
				return {
					success: false,
					err: "Start date or End date missing!",
				};
			}
			if (!params.type) {
				return {
					success: false,
					err: "Type of token for the request is missing!",
				};
			}
			break;

		// Requests
		case validateMetricsRequestType.AVERAGE_REQUEST_DURATION:
		case validateMetricsRequestType.TOTAL_REQUESTS:
			if (!params.timeLimit?.start || !params.timeLimit?.end) {
				return {
					success: false,
					err: "Start date or End date missing!",
				};
			}
			if (!params.operationType) {
				return {
					success: false,
					err: "Operation type for the request is missing!",
				};
			}
			break;
		default:
			break;
	}

	return { success: true };
};

export const getFilterWhereCondition = (
	filter: FilterWhereConditionType,
	filterSelectedConfig?: boolean
) => {
	const whereArray: string[] = [];
	try {
		const { start, end } = filter.timeLimit || {};
		if (start && end) {
			whereArray.push(
				`Timestamp >= parseDateTimeBestEffort('${start}') AND Timestamp <= parseDateTimeBestEffort('${end}')`
			);
		}

		if (filterSelectedConfig && filter.selectedConfig) {
			if (filter.selectedConfig.models?.length) {
				whereArray.push(
					`SpanAttributes['${getTraceMappingKeyFullPath(
						"model"
					)}'] IN (${filter.selectedConfig.models
						.map((model) => `'${model}'`)
						.join(", ")})`
				);
			}

			if (filter.selectedConfig.providers?.length) {
				whereArray.push(
					`(SpanAttributes['${getTraceMappingKeyFullPath(
						"system"
					)}'] IN (${filter.selectedConfig.providers
						.map((provider) => `'${provider}'`)
						.join(", ")}) OR SpanAttributes['${getTraceMappingKeyFullPath(
						"provider"
					)}'] IN (${filter.selectedConfig.providers
						.map((provider) => `'${provider}'`)
						.join(", ")}))`
				);
			}

			if (filter.selectedConfig.traceTypes?.length) {
				whereArray.push(
					`SpanAttributes['${getTraceMappingKeyFullPath(
						"type"
					)}'] IN (${filter.selectedConfig.traceTypes
						.map((type) => `'${type}'`)
						.join(", ")})`
				);
			}

			if (filter.selectedConfig.maxCost) {
				whereArray.push(
					`toFloat64OrZero(SpanAttributes['${getTraceMappingKeyFullPath(
						"cost"
					)}']) BETWEEN 0 AND ${filter.selectedConfig.maxCost.toFixed(10)}`
				);
			}

			if (filter.selectedConfig.applicationNames?.length) {
				whereArray.push(
					`ResourceAttributes['${getTraceMappingKeyFullPath(
						"applicationName"
					)}'] IN (${filter.selectedConfig.applicationNames
						.map((applicationName) => `'${applicationName}'`)
						.join(", ")})`
				);
			}

			if (filter.selectedConfig.environments?.length) {
				whereArray.push(
					`ResourceAttributes['${getTraceMappingKeyFullPath(
						"environment"
					)}'] IN (${filter.selectedConfig.environments
						.map((environment) => `'${environment}'`)
						.join(", ")})`
				);
			}
		}

		if (filter.notOrEmpty && filter.notOrEmpty?.length > 0) {
			const whereOrArray: string[] = [];
			filter.notOrEmpty.map(({ key }: { key: string }) => {
				whereOrArray.push(`notEmpty(${key})`);
			});

			whereArray.push(`( ${whereOrArray.join(" OR ")} )`);
		}

		if (filter.notEmpty && filter.notEmpty?.length > 0) {
			filter.notEmpty.map(({ key }: { key: string }) => {
				whereArray.push(`notEmpty(${key})`);
			});
		}

		const {
			statusCode = ["STATUS_CODE_OK", "STATUS_CODE_UNSET", "Ok", "Unset"],
		} = filter;
		whereArray.push(
			`StatusCode IN (${statusCode.map((type) => `'${type}'`).join(", ")})`
		);

		if (filter.operationType) {
			if (filter.operationType === "vectordb") {
				whereArray.push(
					`SpanAttributes['${getTraceMappingKeyFullPath("type")}'] = 'vectordb'`
				);
			} else {
				whereArray.push(
					`SpanAttributes['${getTraceMappingKeyFullPath(
						"type"
					)}'] != 'vectordb'`
				);
			}
		}
	} catch {}
	return whereArray.join(" AND ");
};

export const getFilterPreviousParams = (filter: MetricParams) => {
	try {
		const previousParams: MetricParams = JSON.parse(JSON.stringify(filter));
		const { start, end, type } = filter.timeLimit || {};
		switch (type) {
			case "24H":
				previousParams.timeLimit.start = addDays(
					start as Date,
					-1
				).toISOString();
				previousParams.timeLimit.end = addDays(end as Date, -1).toISOString();
				break;
			case "7D":
				previousParams.timeLimit.start = addDays(
					start as Date,
					-7
				).toISOString();
				previousParams.timeLimit.end = addDays(end as Date, -1).toISOString();
				break;
			case "1M":
				previousParams.timeLimit.start = addMonths(
					start as Date,
					-1
				).toISOString();
				previousParams.timeLimit.end = addMonths(end as Date, -1).toISOString();
				break;
			case "3M":
				previousParams.timeLimit.start = addMonths(
					start as Date,
					-3
				).toISOString();
				previousParams.timeLimit.end = addMonths(end as Date, -3).toISOString();
				break;
			case "CUSTOM":
				const differenceDays = differenceInDays(end as Date, start as Date);
				previousParams.timeLimit.start = addDays(
					start as Date,
					-differenceDays
				).toISOString();
				previousParams.timeLimit.end = addDays(
					end as Date,
					-differenceDays
				).toISOString();
				break;
			default:
		}

		return previousParams;
	} catch {
		return filter;
	}
};

export const getFilterWhereConditionForGPU = (
	filter: FilterWhereConditionType
) => {
	const whereArray: string[] = [];
	try {
		const { start, end } = filter.timeLimit || {};
		if (start && end) {
			whereArray.push(
				`TimeUnix >= parseDateTimeBestEffort('${start}') AND TimeUnix <= parseDateTimeBestEffort('${end}')`
			);
		}
	} catch {}
	return whereArray.join(" AND ");
};

export const dateTruncGroupingLogic = (end: Date, start: Date) => {
	let dateTrunc = "day";
	if (differenceInYears(end as Date, start as Date) >= 1) {
		dateTrunc = "month";
	} else if (differenceInDays(end as Date, start as Date) <= 1) {
		dateTrunc = "hour";
	}

	return dateTrunc;
};
