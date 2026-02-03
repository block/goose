export default function asaw<T>(p: Promise<T>): Promise<(T | null)[] | [any]> {
	return p
		.then((result: T) => [null, result])
		.catch((error: any) => [error.toString()]);
}
