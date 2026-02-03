import { SUPPORTED_EVALUATION_OPERATIONS, TraceMapping } from "@/constants/traces";
import {
	TraceMappingKeyType,
	TransformedTraceRow,
	TraceRow,
	TraceHeirarchySpan,
} from "@/types/trace";
import { objectKeys } from "@/utils/object";
import { format } from "date-fns";
import { find, get, round } from "lodash";

export const integerParser = (value: string, offset?: number) =>
	parseInt((value || "0") as string, 10) * (offset || 1);

export const floatParser = (value: string, offset?: number) =>
	parseFloat((value || "0") as string) * (offset || 1);

export const getNormalizedTraceAttribute = (
	traceKey: TraceMappingKeyType,
	traceValue: unknown
) => {
	if (traceValue) {
		if (TraceMapping[traceKey].type === "integer") {
			return integerParser(traceValue as string, TraceMapping[traceKey].offset);
		} else if (TraceMapping[traceKey].type === "float") {
			return floatParser(
				(traceValue || "0") as string,
				TraceMapping[traceKey].offset
			).toFixed(10);
		} else if (TraceMapping[traceKey].type === "round") {
			return round(traceValue as number, TraceMapping[traceKey].offset).toFixed(
				10
			);
		} else if (TraceMapping[traceKey].type === "date") {
			const date = new Date(
				`${traceValue}${(traceValue as string).endsWith("Z") ? "" : "Z"}`
			);
			return format(date, "MMM do, y  HH:mm:ss a");
		} else {
			return traceValue;
		}
	} else {
		return TraceMapping[traceKey].defaultValue;
	}
};

export const normalizeTrace = (item: TraceRow): TransformedTraceRow => {
	return objectKeys(TraceMapping).reduce(
		(acc: TransformedTraceRow, traceKey: TraceMappingKeyType) => {
			let value: unknown;
			if (TraceMapping[traceKey].isRoot) {
				value = get(item, TraceMapping[traceKey].path);
			} else {
				value = get(item.SpanAttributes, getTraceMappingKeyFullPath(traceKey));
			}

			acc[traceKey] = getNormalizedTraceAttribute(traceKey, value);
			return acc;
		},
		{} as TransformedTraceRow
	);
};

export const getTraceMappingKeyFullPath = (
	key: TraceMappingKeyType,
	shouldReturnArray: boolean = false
) => {
	if (!TraceMapping[key].prefix) {
		if (!shouldReturnArray) {
			if (typeof TraceMapping[key].path !== "string") {
				return (TraceMapping[key].path as string[]).join(".");
			}
		}
		return TraceMapping[key].path;
	}
	if (shouldReturnArray) {
		let returnArr: string[] = [];
		if (typeof TraceMapping[key].prefix === "string") {
			returnArr = returnArr.concat([TraceMapping[key].prefix as string]);
		} else {
			returnArr = returnArr.concat(TraceMapping[key].prefix as string[]);
		}

		if (typeof TraceMapping[key].path === "string") {
			returnArr = returnArr.concat([TraceMapping[key].path as string]);
		} else {
			returnArr = returnArr.concat(TraceMapping[key].path as string[]);
		}
		return returnArr;
	}

	let returnString = "";
	if (typeof TraceMapping[key].prefix === "string") {
		returnString = TraceMapping[key].prefix as string;
	} else {
		returnString = (TraceMapping[key].prefix as string[]).join(".");
	}

	if (typeof TraceMapping[key].path === "string") {
		returnString = [returnString, TraceMapping[key].path as string].join(".");
	} else {
		returnString = [
			returnString,
			(TraceMapping[key].path as string[]).join("."),
		].join(".");
	}

	return returnString;
};

export const CODE_ITEM_DISPLAY_KEYS: TraceMappingKeyType[] = [
	"prompt",
	"revisedPrompt",
	"response",
	/* Vector */
	"statement",
	"whereDocument",
	"filter",
	/* Framework */
	"retrievalSource",
	/* Exception */
	"statusMessage",
];

export function findSpanInHierarchyLodash(
	hierarchy: TraceHeirarchySpan,
	targetSpanId: string
): TraceHeirarchySpan | undefined {
	// Check current span
	if (hierarchy.SpanId === targetSpanId) {
		return hierarchy;
	}

	// Search in children if they exist
	if (hierarchy.children?.length) {
		return find(hierarchy.children, (child) => {
			return findSpanInHierarchyLodash(child, targetSpanId);
		}) as TraceHeirarchySpan | undefined;
	}

	return undefined;
}

export function getExtraTabsContentTypes(trace: TransformedTraceRow) {
	const defaultTabs = [];

	if (SUPPORTED_EVALUATION_OPERATIONS.includes(trace.type)) {
		defaultTabs.push("Evaluation");
	}

	return defaultTabs;
}
