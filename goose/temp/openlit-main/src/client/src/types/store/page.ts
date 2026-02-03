import { ValueOf } from "@/types/util";

export const DASHBOARD_TYPE_OBJECT: Record<"llm" | "vector" | "gpu", string> = {
	llm: "llm",
	vector: "vector",
	gpu: "gpu",
};

export type DASHBOARD_TYPE = ValueOf<typeof DASHBOARD_TYPE_OBJECT>;

export type REQUEST_VISIBILITY_COLUMNS = Record<
	"id" | "time" | "requestDuration" | "spanName" | "serviceName",
	boolean
>;

export type PAGE = "dashboard" | "request" | "exception" | "fleethub";

export type PageHeader = {
	title: string;
	description?: string;
	breadcrumbs: {
		title: string;
		href: string;
	}[];
}

export type PageStore = {
	dashboard: {
		type: DASHBOARD_TYPE;
	};
	request: {
		visibilityColumns: Record<string, boolean>;
	};
	exception: {
		visibilityColumns: Record<string, boolean>;
	};
	fleethub: {
		visibilityColumns: Record<string, boolean>;
	};
	setData: (p: PAGE, keyPath: string, value: unknown) => void;
	header: PageHeader,
	setHeader: (header: PageHeader) => void;
};