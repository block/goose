export type FetchWrapperProps = {
	body?: string;
	failureCb?: (s?: string) => void;
	url: string;
	requestType: "GET" | "POST" | "DELETE" | "PUT" | "PATCH";
	responseDataKey?: string;
	successCb?: (res?: any) => void;
};
