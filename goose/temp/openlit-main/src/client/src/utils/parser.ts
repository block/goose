export const parseQueryStringToObject = (
	queryParamsStr: string
): Record<string, string> => {
	const queryObject: Record<string, string> = {};
	if (queryParamsStr) {
		queryParamsStr.split("&").forEach((param) => {
			const [key, value] = param.split("=");
			queryObject[key] = value;
		});
	}

	return queryObject;
};

export const constructURL = (hostname: string, port: string) =>
	`${hostname.match(/^https?\:\/\//) ? hostname : "http://" + hostname}${
		port ? ":" + port : ""
	}`;
