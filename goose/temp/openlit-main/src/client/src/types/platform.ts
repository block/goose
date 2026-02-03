export type OPERATION_TYPE = "llm" | "vectordb";

export type FilterWhereConditionType = {
	timeLimit: {
		start: Date | string;
		end: Date | string;
		type: string;
	};
	offset?: number;
	limit?: number;
	selectedConfig?: Partial<{
		providers: string[];
		maxCost: number;
		models: string[];
		traceTypes: string[];
		applicationNames: string[];
		environments: string[];
	}>;
	notOrEmpty?: { key: string }[];
	notEmpty?: { key: string }[];
	statusCode?: string[];
	operationType?: OPERATION_TYPE;
};
