import sqlString from "sqlstring";
import { objectKeys } from "./object";
import { isArray, isNil, isObject } from "lodash";

export default class Sanitizer {
	static options = { xss: true, noSql: true, sql: true, level: 5 };
	static sanitizeObject<T extends object>(obj: T): T {
		const v = objectKeys(obj).reduce((acc: Partial<T>, key) => {
			let value: T[keyof T] = obj[key];
			if (isArray(value)) {
				value = value.map((val) => {
					if (isObject(val)) {
						return Sanitizer.sanitizeObject(val);
					} else {
						return Sanitizer.sanitizeValue(val);
					}
				}) as T[keyof T];
			} else if (isObject(value)) {
				value = Sanitizer.sanitizeObject(value);
			} else {
				value = Sanitizer.sanitizeValue(value) as T[keyof T];
			}
			acc[key] = value;
			return acc;
		}, {}) as T;
		return v;
	}
	static sanitizeValue<T>(value: T): T {
		if (!isNil(value)) {
			if (typeof value === "boolean") {
				return value as T;
			}
			if (typeof value === "string") {
				return sqlString.escape(value).slice(1, -1) as T; // Remove outer quotes and escape the string
			} else {
				return sqlString.escape(value) as T; // For other types (numbers, booleans), use default escape
			}
		}

		return value;
	}
}
