import { PropertyPath } from '../_internal/PropertyPath.mjs';

declare function property<T, R>(path: PropertyPath): (obj: T) => R;

export { property };
