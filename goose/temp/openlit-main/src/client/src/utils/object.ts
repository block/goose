export function objectKeys<T extends object>(object: T): Array<keyof T> {
	return Object.keys(object) as Array<keyof T>;
}

export function objectEntries<T extends object>(
	object: T
): Array<[keyof T, T[keyof T]]> {
	return Object.entries(object) as Array<[keyof T, T[keyof T]]>;
}