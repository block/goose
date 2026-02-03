export type TIME_RANGES = "24H" | "7D" | "1M" | "3M" | "CUSTOM";
export type REFRESH_RATES = "Never" | "30s" | "1m" | "5m" | "15m";

export type FilterSorting = {
	type: string;
	direction: "asc" | "desc";
};

export interface FilterType {
	timeLimit: {
		start?: Date;
		end?: Date;
		type: TIME_RANGES;
	};
	limit: number;
	offset: number;
	selectedConfig: Partial<FilterConfig>;
	sorting: FilterSorting;
	refreshRate: REFRESH_RATES;
}

export interface FilterConfig {
	providers: string[];
	maxCost: number;
	models: string[];
	totalRows: number;
	traceTypes: string[];
	applicationNames: string[];
	environments: string[];
}

export type FilterStore = {
	details: FilterType;
	config?: FilterConfig;
	updateFilter: (key: string, value: any, extraParams?: any) => void;
	updateConfig: (config: FilterConfig) => void;
};
