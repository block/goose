import { merge, set } from "lodash";
import { addDays, addMonths, addWeeks } from "date-fns";
import { lens } from "@dhmk/zustand-lens";
import {
	FilterConfig,
	FilterSorting,
	FilterStore,
	FilterType,
	REFRESH_RATES,
	TIME_RANGES,
} from "@/types/store/filter";

export const REFRESH_RATE_TYPE: Record<REFRESH_RATES, string> = {
	Never: "Never",
	"30s": "30s",
	"1m": "1m",
	"5m": "5m",
	"15m": "15m",
};

export const TIME_RANGE_TYPE: Record<TIME_RANGES, string> = {
	"24H": "24H",
	"7D": "7D",
	"1M": "1M",
	"3M": "3M",
	CUSTOM: "CUSTOM",
};

export const DEFAULT_TIME_RANGE = "24H";

const DEFAULT_LIMIT = 25;

const DEFAULT_SORTING: FilterSorting = {
	type: "Timestamp",
	direction: "desc",
};

export function getTimeLimitObject(
	value: string,
	keyPrefix: string,
	extraParams?: any
): unknown {
	let object = {};
	if (value === TIME_RANGE_TYPE["24H"]) {
		const currentDate = new Date();
		set(object, `${keyPrefix}start`, addDays(currentDate, -1));
		set(object, `${keyPrefix}end`, currentDate);
	} else if (value === TIME_RANGE_TYPE["7D"]) {
		const currentDate = new Date();
		set(object, `${keyPrefix}start`, addWeeks(currentDate, -1));
		set(object, `${keyPrefix}end`, currentDate);
	} else if (value === TIME_RANGE_TYPE["1M"]) {
		const currentDate = new Date();
		set(object, `${keyPrefix}start`, addMonths(currentDate, -1));
		set(object, `${keyPrefix}end`, currentDate);
	} else if (value === TIME_RANGE_TYPE["3M"]) {
		const currentDate = new Date();
		set(object, `${keyPrefix}start`, addMonths(currentDate, -3));
		set(object, `${keyPrefix}end`, currentDate);
	} else if (value === TIME_RANGE_TYPE["CUSTOM"]) {
		const start = extraParams?.start;
		const end = extraParams?.end;
		if (start && end) {
			set(object, `${keyPrefix}start`, start);
			set(object, `${keyPrefix}end`, end);
		}
	}
	return object;
}

const INITIAL_FILTER_DETAILS: FilterType = {
	timeLimit: {
		type: DEFAULT_TIME_RANGE,
		...(getTimeLimitObject(DEFAULT_TIME_RANGE, "") as {
			end: Date;
			start: Date;
		}),
	},
	limit: DEFAULT_LIMIT,
	offset: 0,
	selectedConfig: {},
	sorting: DEFAULT_SORTING,
	refreshRate: "1m",
};

export const filterStoreSlice: FilterStore = lens((setStore, getStore) => ({
	details: INITIAL_FILTER_DETAILS,
	updateFilter: (key: string, value: any, extraParams?: any) => {
		let object: Partial<
			FilterType & {
				end: Date;
				start: Date;
			}
		> = {};
		let resetConfig = false;
		switch (key) {
			case "timeLimit.type":
				object = getTimeLimitObject(value, "timeLimit.", extraParams) as {
					end: Date;
					start: Date;
				};
				resetConfig = true;
				break;
			case "limit":
			case "selectedConfig":
			case "sorting":
				set(object, "offset", 0);
				break;
			case "offset":
				// Its already handled in the set(object, key, value); line
				break;
			case "page-change":
				set(object, "offset", 0);
				set(object, "limit", DEFAULT_LIMIT);
				set(object, "sorting", DEFAULT_SORTING);
				break;
			default:
				break;
		}

		if (key !== "page-change") {
			set(object, key, value);
		}

		setStore({
			details: {
				...merge(getStore().details, object),
				selectedConfig:
					resetConfig || extraParams?.clearFilter
						? {}
						: object.selectedConfig
						? object.selectedConfig
						: getStore().details.selectedConfig,
			},
			config: resetConfig ? undefined : getStore().config,
		});
	},
	updateConfig: (config: FilterConfig) => {
		setStore({ config });
	},
}));
