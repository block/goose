import { consoleLog } from "./log";

export function jsonStringify(json: any) {
	try {
		const jsonS = JSON.stringify(json);
		return jsonS;
	} catch (e) {
		consoleLog(e, json);
		return "";
	}
}

export function jsonParse(str: string) {
	try {
		const jsonP = JSON.parse(str);
		return jsonP;
	} catch (e) {
		consoleLog(e);
		return undefined;
	}
}
